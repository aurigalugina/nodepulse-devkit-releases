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

This means conflict detection (non-fast-forward push rejection), delta
transfer, history, and rollback (`git revert`/`git reset`) are all git's
own native behavior — devkit does not reimplement any of this. Devkit's own
git-related Tauri commands (`git.rs`) shell out to the real `git` CLI, they
don't reimplement git operations.

A repo may simultaneously have an `origin` remote (GitHub, from the Git
Tahap 3a pull-only deploy-key flow in `backend-core-node`) and a
`nodepulse` remote (this app) — they're entirely independent; devkit only
ever touches `nodepulse`.

## VSCodium Launch

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

## Storage Location: user-chosen per folder

No default project directory. Every time a folder is opened, devkit shows
a native OS folder-picker dialog (`@tauri-apps/plugin-dialog`'s `open()`
JS API, called directly from the frontend — no custom Tauri command
needed). This includes Windows UNC paths like `\\wsl$\<distro>\...`,
handled as an ordinary filesystem path with no special WSL code.

## Key Files

| File | Purpose |
|---|---|
| `src/App.svelte` | Root — startup gate, deep-link event listener, auth/pending-open/opened state routing |
| `src/lib/stores/authStore.svelte.js` | Config persist (url, username, token, token_expires_at, vscodium_path) — mirrors `nodepulse-connect`'s `authStore.svelte.js` pattern, adds `isAuthenticated` expiry check since devkit has no refresh flow |
| `src/lib/components/Login.svelte` | Host/username/password form → `login` Tauri command |
| `src/lib/components/OpenFolder.svelte` | Drives the picker → clone → launch-VSCodium sequence once a deep-link is queued |
| `src/lib/components/ProjectView.svelte` | Post-clone screen — "Open in VSCodium" (primary), "Push to Server" / "Pull Latest" (secondary, beginner-friendly path with change preview + non-fast-forward conflict recovery) |
| `src-tauri/src/commands/storage.rs` | `read_config`/`write_config`/`clear_auth_token` — config persisted to `%APPDATA%/NodePulse IDE/config.json` (Windows) or `~/.config/nodepulse-ide/config.json` (macOS/Linux) |
| `src-tauri/src/commands/nodepulse.rs` | `login` — calls the devkit-specific 90-day JWT endpoint |
| `src-tauri/src/commands/git.rs` | `git_clone`, `git_status_porcelain`, `git_commit_and_push`, `git_pull` — all shell out to the real `git` CLI, no custom protocol logic |
| `src-tauri/src/commands/vscodium.rs` | `launch_vscodium` — shells out to `codium` (or configured path) |
| `src-tauri/src/lib.rs` | Plugin registration, deep-link `on_open_url` handler forwarding to frontend as a `deep-link` event |
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
