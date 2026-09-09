mod commands;

use commands::git::{git_clone, git_commit_and_push, git_pull, git_status_porcelain};
use commands::nodepulse::login;
use commands::storage::{clear_auth_token, read_config, write_config};
use commands::vscodium::launch_vscodium;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        // NOTE: deep-link handling is done entirely on the frontend (App.svelte)
        // via the plugin's own JS API (getCurrent() for the cold-start launch
        // URL + onOpenUrl() for URLs received while already running) — NOT via
        // a custom Rust event emitted from .setup(), which raced the frontend's
        // listener registration and silently dropped the cold-start URL (the
        // app would launch, sign in, but never see the folder/node to open).
        // See document/decision-log/2026-09-08-nodepulse-ide-git-bare-repo-transport.md.
        .invoke_handler(tauri::generate_handler![
            // Storage
            read_config,
            write_config,
            clear_auth_token,
            // NodePulse auth
            login,
            // Git
            git_clone,
            git_status_porcelain,
            git_commit_and_push,
            git_pull,
            // VSCodium
            launch_vscodium,
        ])
        .build(tauri::generate_context!())
        .expect("error building NodePulse IDE")
        .run(|_app_handle, _event| {});
}
