use std::process::Command;

/// Builds the git remote URL devkit clones/pushes/pulls against — points at
/// Core's git-transport proxy endpoint (see
/// backend-core-node/internal/handler/http/git_transport.go), never
/// directly at a node. `folder` is the absolute path on the NODE's
/// filesystem (not the local clone destination) — base64url-encoded since
/// git remote URLs can't cleanly carry an arbitrary path with slashes as a
/// single segment.
fn build_remote_url(host: &str, node_id: &str, folder: &str) -> String {
    use base64::Engine;
    let folder_b64 = base64::engine::general_purpose::URL_SAFE.encode(folder.as_bytes());
    format!(
        "{}/api/v1/network/nodes/{}/git/{}/",
        host.trim_end_matches('/'),
        node_id,
        folder_b64
    )
}

fn run_git(args: &[&str], cwd: Option<&str>) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd.output().map_err(|e| format!("failed to run git: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Clones the node's bare repo (via Core's git-transport proxy) to
/// local_path, then configures the clone so VSCodium's own git integration
/// (Push/Pull/Fetch buttons, Source Control panel) works without ever
/// prompting for credentials — the JWT is attached via a persisted
/// `http.extraHeader` git config value, not a one-shot flag, and
/// user.name/user.email are set from the logged-in NodePulse identity so
/// commit history shows a real name instead of the OS username.
///
/// KNOWN TRADE-OFF (flagged in the implementation plan and decision log):
/// this embeds the 90-day JWT in the clone's local .git/config in
/// plaintext. Accepted for an internal-tool context — not a storage
/// location NodePulse's own database ever sees.
#[tauri::command]
pub async fn git_clone(
    host: String,
    node_id: String,
    folder: String,
    local_path: String,
    jwt_token: String,
    display_name: String,
    email: String,
) -> Result<(), String> {
    let remote_url = build_remote_url(&host, &node_id, &folder);

    // The auth header MUST be present during the clone itself, not set
    // afterward — `git clone` immediately makes an authenticated request to
    // the remote, and without a header at that point git falls back to its
    // normal interactive credential flow (on Windows: Git Credential
    // Manager), which has no matching stored credential and fails with a
    // generic "Authentication failed" error that gives no hint the actual
    // issue is a missing Bearer token, not a wrong username/password.
    // `git clone -c <key>=<value>` applies config values for the duration
    // of the clone command itself (unlike a bare `git config` afterward,
    // which only affects commands run after the clone already finished).
    let auth_scheme = "Authorization:";
    let bearer_kind = "Bearer";
    let auth_header = format!("{auth_scheme} {bearer_kind} {jwt_token}");
    let http_extra_header_config = format!("http.extraHeader={auth_header}");
    run_git(
        &["clone", "-c", &http_extra_header_config, &remote_url, &local_path],
        None,
    )?;

    // `git clone <url> <path>` always names the remote "origin" — rename it
    // to "nodepulse" so it matches what git_commit_and_push/git_pull (and
    // VSCodium's Source Control panel, which shows whatever remotes exist)
    // actually use. Without this rename, every push/pull from devkit failed
    // with "fatal: 'nodepulse' does not appear to be a git repository"
    // because that remote name never existed.
    run_git(&["remote", "rename", "origin", "nodepulse"], Some(&local_path))?;

    // Persist the auth header as a permanent (not clone-scoped) config value
    // so VSCodium's own Push/Pull/Fetch buttons keep working after this
    // command returns — the -c flag above only applied for the clone
    // command's own duration, it does not persist to .git/config on its own.
    run_git(&["config", "http.extraHeader", &auth_header], Some(&local_path))?;
    run_git(&["config", "user.name", &display_name], Some(&local_path))?;
    run_git(&["config", "user.email", &email], Some(&local_path))?;

    Ok(())
}

/// Returns `git status --porcelain=v1` output — used by the frontend to
/// build the "N files changed" preview before a "Push to Server" click
/// (Task 9), and to gate the "Checkout & Pull" action if there are
/// uncommitted changes.
#[tauri::command]
pub fn git_status_porcelain(local_path: String) -> Result<String, String> {
    run_git(&["status", "--porcelain=v1"], Some(&local_path))
}

/// Stages everything, commits (with a default message if none given), and
/// pushes to the `nodepulse` remote's current branch — the beginner-
/// friendly path from the design discussion, for users not yet comfortable
/// with git directly. On a non-fast-forward rejection, returns git's own
/// stderr message untouched so the frontend can show it and offer a
/// "Pull latest & retry" follow-up (git_pull below) rather than silently
/// retrying or attempting any custom conflict resolution.
#[tauri::command]
pub fn git_commit_and_push(local_path: String, commit_message: Option<String>) -> Result<String, String> {
    run_git(&["add", "-A"], Some(&local_path))?;

    let message = commit_message.unwrap_or_else(|| "Update via NodePulse IDE".to_string());
    // "nothing to commit" is not an error condition worth failing on — the
    // user may click Push with no changes; git exits non-zero for that, so
    // check for it explicitly rather than surfacing a confusing error.
    match run_git(&["commit", "-m", &message], Some(&local_path)) {
        Ok(out) => out,
        Err(e) if e.contains("nothing to commit") => {
            return Ok("Nothing to commit — already up to date.".to_string());
        }
        Err(e) => return Err(e),
    };

    let branch = run_git(&["rev-parse", "--abbrev-ref", "HEAD"], Some(&local_path))?
        .trim()
        .to_string();

    run_git(&["push", "nodepulse", &branch], Some(&local_path))
}

/// Pulls the current branch from the `nodepulse` remote — used both
/// standalone (branch-picker "Pull" button) and as the recovery action
/// after a rejected push.
#[tauri::command]
pub fn git_pull(local_path: String) -> Result<String, String> {
    let branch = run_git(&["rev-parse", "--abbrev-ref", "HEAD"], Some(&local_path))?
        .trim()
        .to_string();
    run_git(&["pull", "nodepulse", &branch], Some(&local_path))
}
