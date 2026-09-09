# nodepulse-devkit — Current State

## Role
Desktop companion app (NodePulse IDE) that lets a user open an opt-in
NodePulse folder in their locally-installed VSCodium/VSCode, edit it as a
normal local git working copy, and sync changes back to the node — via a
git bare repo the node itself hosts, reached exclusively through Core's
`git-transport` gRPC session proxy (never SSH/SFTP directly to a node).

Separate sub-project from `nodepulse-connect` (which joins the
Headscale/Tailscale mesh) — devkit never touches the mesh at all, it talks
to Core over plain HTTPS like a normal API client.

**IMPORTANT: this is its own git repo, not part of the `node-pulse`
monorepo's git history**, even though the folder lives inside the monorepo
checkout (same pattern as `nodepulse-connect/`) — see "Git Repo" below
before running any git command in this folder.

Full architecture rationale: `document/decision-log/2026-09-08-nodepulse-ide-git-bare-repo-transport.md`
(in the `node-pulse` monorepo, not here).

## Git Repo

- `nodepulse-devkit/` has its own `.git`, separate from the monorepo's.
- Remote: `git@github.com:aurigalugina/nodepulse-devkit-releases.git`
  (source code and releases live in the SAME repo — same naming pattern as
  `nodepulse-connect-releases`, not a separate downstream repo).
- The monorepo (`node-pulse/`) also tracks these files in its own history
  (dual-tracking, same as `nodepulse-connect/`) — when committing devkit
  changes, commit in **both** places: `cd nodepulse-devkit && git commit`
  (pushes to the releases repo) AND `cd .. && git add nodepulse-devkit/ &&
  git commit` (keeps the monorepo's copy in sync). Forgetting the monorepo
  commit doesn't break anything functionally, just leaves the monorepo's
  history stale for this folder.
- Signing keypair: `~/.tauri/nodepulse-devkit.key` (private, never
  committed anywhere) / `.key.pub`. **Distinct from `nodepulse-connect`'s
  keypair** — never reuse; a compromise of one app's key must not affect
  the other.

## Stack
- Tauri v2 + Svelte 5 (runes mode) + Tailwind v4
- `@tauri-apps/api` v2, `@tauri-apps/plugin-fs`, `@tauri-apps/plugin-dialog`,
  `@tauri-apps/plugin-deep-link`
- `@tauri-apps/plugin-updater` v2 — auto-update via GitHub Releases manifest
  (same mechanism as `nodepulse-connect`)
- No Tailscale/mesh dependencies of any kind — this is the key structural
  difference from `nodepulse-connect`'s build.

## Auth: 90-day JWT, no refresh-token flow

Devkit calls `POST /api/v1/auth/login/token/devkit` (backend-core-node),
a route distinct from the standard `/auth/login/token` used by web-panel
and `nodepulse-connect` — this keeps their 8h session token behavior
completely unaffected. Devkit's token is deliberately long-lived (90 days)
instead of using a refresh-token mechanism: refresh tokens solve
fast-revocation scenarios that don't matter much for a small internal
team, at the cost of meaningfully more implementation surface. See the
decision log for the full trade-off discussion.

The token is persisted two places:
1. Devkit's own config file (`read_config`/`write_config` Tauri commands,
   mirroring `nodepulse-connect`'s `storage.rs` pattern) — used for devkit's
   own API calls (login check, etc).
2. Each cloned project's local `.git/config`, via `git config
   http.extraHeader "Authorization: Bearer <jwt>"` — **not** a one-shot
   clone flag, a persisted config value, which is what lets VSCodium's own
   Push/Pull/Fetch buttons (Source Control panel) authenticate without
   ever prompting the user for credentials.

**Known trade-off, deliberate:** this means the 90-day JWT sits in
plaintext inside each cloned project's `.git/config` on the user's laptop.
Accepted for this internal-tool context — not a location NodePulse's own
database ever sees or stores. Flagged here so it's never mistaken for an
oversight.

## Git Sync Mechanism: node-hosted bare repo, not custom delta sync

Each opt-in folder gets a bare git repo lazily created on the node
(`<folder>.nodepulse-git/`, sibling directory, never inside the working
folder — so it never appears in NodePulse's Files tab listing). Devkit
clones/pushes/pulls this bare repo via a completely ordinary git HTTPS
remote — from git's (and VSCodium's) point of view, indistinguishable from
cloning off GitHub. The remote URL points at Core's `git-transport` proxy
endpoint, which forwards the raw git Smart HTTP protocol to the Runner over
a gRPC session; the Runner executes git's own `git-http-backend` binary
on-demand (never a daemon, never an open port on the node).

**The remote is named `nodepulse`, not the git-clone default `origin`.**
`git_clone` (in `git.rs`) explicitly runs `git remote rename origin
nodepulse` right after cloning — `git_commit_and_push`/`git_pull` (and
VSCodium's own Source Control panel push/pull buttons) both target the
`nodepulse` remote by name, so without this rename every push/pull failed
with `fatal: 'nodepulse' does not appear to be a git repository` (fixed
2026-09-09, v0.1.2 — the remote name mismatch existed since the initial
scaffold and was only caught during manual end-user testing, not during
implementation).

**Auth header must be applied via `git clone -c http.extraHeader=...`, not
a `git config` call after cloning.** The original implementation ran plain
`git clone <url> <path>` first, then set the auth header afterward — but
`git clone` itself immediately makes an authenticated request to the
remote, so without a header present at THAT point, git falls back to its
normal interactive credential prompt (Git Credential Manager on Windows),
finds no matching stored credential, and fails with a generic
"Authentication failed" error that gives no hint the real problem is a
missing Bearer token. Fixed 2026-09-09, v0.1.3, by passing `-c
http.extraHeader=<value>` directly to the `clone` subcommand (config values
passed via `-c` apply for that command's own duration); the header is then
also persisted permanently via a normal `git config` call afterward so
VSCodium's Push/Pull/Fetch keep working post-clone.

This means conflict detection (non-fast-forward push rejection), delta
transfer, history, and rollback (`git revert`/`git reset`) are all git's
own native behavior — devkit does not reimplement any of this. Devkit's own
git-related Tauri commands (`git.rs`) shell out to the real `git` CLI, they
don't reimplement git operations.

A repo may simultaneously have an `origin` remote (GitHub, from the Git
Tahap 3a pull-only deploy-key flow in `backend-core-node`) and a
`nodepulse` remote (this app) — they're entirely independent; devkit only
ever touches `nodepulse`.

## Startup Update Check + Copyable Error Reports (v0.1.4)

Two UX additions requested after the user hit repeated real-world bugs
during manual verification and had to keep re-checking Settings for
updates / re-typing error text by hand when reporting back:

- **`StartupCheck.svelte`** gates the login/deep-link UI behind a blocking
  update-check screen (same pattern `nodepulse-connect` already uses) —
  checks on every launch via `@tauri-apps/plugin-updater`, shows an
  "Update Now" button that downloads+installs+relaunches. No separate
  Settings page needed to catch a new release anymore.
- **`ErrorPanel.svelte`** is the one place every failure surface renders
  through — clone/launch failures in `OpenFolder.svelte`, push/pull/launch
  failures in `ProjectView.svelte`. Its "Copy" button builds one
  self-contained plain-text report (which step failed, node id, folder,
  raw git/OS error text) via `@tauri-apps/plugin-clipboard-manager`, so a
  bug report is one click instead of manually re-typing what scrolled by
  in a terminal window.

## `-c safe.directory=<cwd>` on every git command (fixed 2026-09-09, v0.1.4)

WSL folders accessed through Windows' `\\wsl.localhost\<distro>\...` UNC
path get flagged by git's "dubious ownership" safety check (designed to
stop a local user being tricked into operating on a repo owned by another,
possibly malicious, local account) — `git clone`/`status`/`push`/`pull`
all failed with `fatal: detected dubious ownership in repository at
'...'` the moment the working copy lived on a WSL-mounted path. `run_git()`
now always passes `-c safe.directory=<cwd>` (command-scoped, not
`--global`) so only the exact directory being operated on is whitelisted.



Devkit is a bridge, not an IDE — it shells out to the user's existing
VSCodium/VSCode install (`codium` on PATH by default, falls back to a
user-configured executable path stored in config). It never bundles an
editor.

## Handoff from web-panel: custom URI scheme

Web-panel's "Open in NodePulse-IDE" context menu item (folders only, gated
behind `node.gitToolsEnabled`, see `web-panel/CLAUDE.md`) navigates to
`nodepulse-ide://open?node=<id>&path=<folder>&host=<origin>`. Devkit
registers this scheme with the OS at install time via the deep-link plugin
(`tauri-plugin-deep-link`) — clicking the link launches (or focuses) devkit
and forwards the URL as a `deep-link` frontend event, which `App.svelte`
parses into `{ nodeId, path, host }` and queues until login completes if
needed.

**Known limitation (not yet built, not blocking):** no
`tauri-plugin-single-instance` — a second deep-link click while devkit is
already running may spawn a new app instance instead of routing to the
existing one. Flagged for a later pass if it proves to be a real annoyance
in practice.

**Important implementation detail (fixed 2026-09-09, v0.1.1):** deep-link
handling MUST happen via the plugin's frontend JS API
(`getCurrent()`/`onOpenUrl()` from `@tauri-apps/plugin-deep-link`), not via
a custom Rust-side event emitted from `.setup()`. The original
implementation emitted a `deep-link` event from `.setup()` before any
frontend `listen()` call could possibly be registered yet — the cold-start
launch URL (the ONLY case that matters on Windows/Linux, see below) was
silently dropped every time, and the app would land on the plain "Signed
in as..." screen with no folder ever opened. `getCurrent()` in `onMount`
fixes this by asking Rust what the launch URL was, on-demand, instead of
racing a one-shot event. Per the plugin's own docs, `onOpenUrl` (warm-app
case) is unsupported on Windows/Linux without the single-instance plugin —
the OS spawns a brand-new process per click there instead of reusing the
running one — which makes `getCurrent()` the primary mechanism on those
platforms, not a fallback.

## Storage Location: user-chosen per folder

No default project directory. Every time a folder is opened, devkit shows
a native OS folder-picker dialog (`@tauri-apps/plugin-dialog`'s `open()`
JS API, called directly from the frontend — no custom Tauri command
needed). This includes Windows UNC paths like `\\wsl$\<distro>\...`,
handled as an ordinary filesystem path with no special WSL code.

## Key Files

| File | Purpose |
|---|---|
| `src/App.svelte` | Root — startup-update gate, deep-link event listener, auth/pending-open/opened state routing |
| `src/lib/components/StartupCheck.svelte` | Blocking update-check screen shown before login/deep-link handling (same pattern as `nodepulse-connect`'s `StartupCheck.svelte`) — "Update Now" downloads+installs+relaunches via `@tauri-apps/plugin-updater` |
| `src/lib/components/ErrorPanel.svelte` | Shared error display — title + contextual key/value pairs (step, node, folder) + raw error text + "Copy" button that builds one self-contained plain-text report via `@tauri-apps/plugin-clipboard-manager`. Used by `OpenFolder.svelte` (clone/launch failures) and `ProjectView.svelte` (push/pull/launch failures) so every failure surface in the app is copy-pasteable for bug reports |
| `src/lib/stores/authStore.svelte.js` | Config persist (url, username, token, token_expires_at, vscodium_path) — mirrors `nodepulse-connect`'s `authStore.svelte.js` pattern, adds `isAuthenticated` expiry check since devkit has no refresh flow |
| `src/lib/components/Login.svelte` | Host/username/password form → `login` Tauri command |
| `src/lib/components/OpenFolder.svelte` | Drives the picker → clone → launch-VSCodium sequence once a deep-link is queued |
| `src/lib/components/ProjectView.svelte` | Post-clone screen — "Open in VSCodium" (primary), "Push to Server" / "Pull Latest" (secondary, beginner-friendly path with change preview + non-fast-forward conflict recovery) |
| `src-tauri/src/commands/storage.rs` | `read_config`/`write_config`/`clear_auth_token` — config persisted to `%APPDATA%/NodePulse IDE/config.json` (Windows) or `~/.config/nodepulse-ide/config.json` (macOS/Linux) |
| `src-tauri/src/commands/nodepulse.rs` | `login` — calls the devkit-specific 90-day JWT endpoint |
| `src-tauri/src/commands/git.rs` | `git_clone`, `git_status_porcelain`, `git_commit_and_push`, `git_pull` — all shell out to the real `git` CLI, no custom protocol logic. `run_git()` always passes `-c safe.directory=<cwd>` (see "Git Sync Mechanism" below) |
| `src-tauri/src/commands/vscodium.rs` | `launch_vscodium` — shells out to `codium` (or configured path) |
| `src-tauri/src/lib.rs` | Plugin registration (updater, process, deep-link, clipboard-manager), deep-link `on_open_url` handler forwarding to frontend as a `deep-link` event |
| `src-tauri/tauri.conf.json` | identifier `id.ussi.nodepulse-devkit`, updater endpoint (`nodepulse-devkit-releases`), deep-link scheme (`nodepulse-ide`), resizable 640×520 window (not connect's fixed compact window — devkit needs to show file lists/previews) |
| `.github/workflows/build.yml` | CI — same pipeline shape as `nodepulse-connect`'s, minus all Tailscale bundling steps |

## Release Pipeline

Mirrors `nodepulse-connect`'s exactly (see its `CLAUDE.md` for the general
mechanism), with these differences:
- No Tailscale binary bundling steps at all (devkit never touches the mesh).
- Distinct signing keypair (`~/.tauri/nodepulse-devkit.key`).
- Publishes to `github.com/aurigalugina/nodepulse-devkit-releases` (its own
  repo, not a separate downstream repo — see "Git Repo" above) using the
  default `GITHUB_TOKEN` (no cross-repo PAT needed, since source and
  releases are the same repo).
- Update manifest: `https://github.com/aurigalugina/nodepulse-devkit-releases/releases/latest/download/latest.json`

**v0.1.0 released and CI-verified end-to-end (2026-09-08):** all 3 platform
builds green (Windows NSIS `.exe`, macOS `.dmg` + `.app.tar.gz`, Linux
`.AppImage`), `latest.json` manifest fetched live and confirmed
well-formed with valid per-platform signatures.

## DO NOT
- Jangan pakai httpOnly cookie — desktop app pakai Bearer token (same
  convention as `nodepulse-connect`)
- Jangan reuse `nodepulse-connect`'s signing keypair — always a distinct
  key per app
- Jangan tambah Tailscale/mesh dependency — devkit is deliberately
  mesh-free, talks to Core over plain HTTPS only
- Jangan implement custom git delta/conflict/rollback logic — always shell
  out to the real `git` CLI and let git's own behavior handle it
- Jangan lupa commit di DUA tempat (repo sendiri + monorepo) — lihat "Git
  Repo" di atas

## Known Gaps / Deferred

- **Manual end-to-end verification against a real dev node** (actual `git
  clone`/`push` through the live Core↔Runner gRPC pipeline) has not yet
  been performed — this is the single highest-risk piece of the whole
  Tahap 4 feature (see the decision log's Risk Acknowledgement section).
  Do this before trusting the feature in production.
- No `tauri-plugin-single-instance` (see "Handoff" above).
- No deploy-key revocation/rotation UI (shared gap with Git Tahap 3a).
- Push to GitHub (Git Tahap 3b) — entirely separate feature, not built.
- No dedicated download page in web-panel — the "Open in NodePulse-IDE"
  toast links directly to the GitHub Releases URL as a fallback (no UI
  precedent existed for `nodepulse-connect` either).
