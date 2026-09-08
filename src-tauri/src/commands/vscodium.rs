use std::process::Command;

/// Launches the user's local VSCodium (or VSCode) install pointed at
/// local_path. Tries `codium` on PATH first (Linux/macOS common case,
/// Windows if the user installed the "Add to PATH" option), falling back
/// to an explicitly-configured executable path (set once via the frontend's
/// "locate VSCodium" file picker flow, persisted in AppConfig.vscodium_path)
/// if PATH lookup fails.
#[tauri::command]
pub fn launch_vscodium(local_path: String, vscodium_path: Option<String>) -> Result<(), String> {
    let exe = vscodium_path.unwrap_or_else(|| "codium".to_string());
    Command::new(&exe)
        .arg(&local_path)
        .spawn()
        .map_err(|e| format!("Failed to launch VSCodium ({exe}): {e}. If VSCodium isn't on your PATH, set its executable location in Settings."))?;
    Ok(())
}
