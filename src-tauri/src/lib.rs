mod commands;

use commands::git::{git_clone, git_commit_and_push, git_pull, git_status_porcelain};
use commands::nodepulse::login;
use commands::storage::{clear_auth_token, read_config, write_config};
use commands::vscodium::launch_vscodium;
use tauri::Emitter;
use tauri_plugin_deep_link::DeepLinkExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            // Register the nodepulse-ide:// scheme handler and forward any
            // deep-link URL (both the one that launched the app cold, and
            // any received while already running) to the frontend as a
            // "deep-link" event — App.svelte listens for this and parses
            // node/path/host query params to drive the open-folder flow.
            // See document/decision-log/2026-09-08-nodepulse-ide-git-bare-repo-transport.md
            // for why a custom URI scheme is the browser-to-desktop-app
            // handoff mechanism.
            let handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    let _ = handle.emit("deep-link", url.to_string());
                }
            });
            Ok(())
        })
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
