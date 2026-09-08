use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Persisted devkit config — mirrors nodepulse-connect's AppConfig shape
/// (see nodepulse-connect/src-tauri/src/commands/storage.rs) but carries
/// devkit-specific fields instead of mesh-related ones. auth_token here is
/// the long-lived (90-day) JWT from Task 7 — see
/// document/decision-log/2026-09-08-nodepulse-ide-git-bare-repo-transport.md
/// for why this app deliberately has no refresh-token mechanism.
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct AppConfig {
    pub nodepulse_url: Option<String>,
    pub username: Option<String>,
    pub auth_token: Option<String>,
    pub token_expires_at: Option<String>, // RFC3339
    pub vscodium_path: Option<String>,    // set once if `codium` isn't on PATH
}

fn config_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA")
            .map_err(|_| "APPDATA environment variable not set".to_string())?;
        Ok(PathBuf::from(appdata).join("NodePulse IDE"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").map_err(|_| "HOME environment variable not set".to_string())?;
        Ok(PathBuf::from(home).join(".config").join("nodepulse-ide"))
    }
}

fn config_path() -> Result<PathBuf, String> {
    let dir = config_dir()?;
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config directory: {e}"))?;
    Ok(dir.join("config.json"))
}

/// Read persisted config. Returns default (empty) config if the file
/// doesn't exist yet — same convention as nodepulse-connect.
#[tauri::command]
pub fn read_config() -> Result<AppConfig, String> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let data = fs::read_to_string(&path).map_err(|e| format!("Failed to read config: {e}"))?;
    serde_json::from_str(&data).map_err(|e| format!("Failed to parse config: {e}"))
}

/// Write config atomically (write to .tmp then rename) — same convention as
/// nodepulse-connect.
#[tauri::command]
pub fn write_config(config: AppConfig) -> Result<(), String> {
    let path = config_path()?;
    let tmp_path = path.with_extension("tmp");

    let data = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {e}"))?;

    fs::write(&tmp_path, data).map_err(|e| format!("Failed to write config: {e}"))?;
    fs::rename(&tmp_path, &path).map_err(|e| format!("Failed to finalize config write: {e}"))?;

    Ok(())
}

/// Clear the stored auth token (on logout), preserving other fields.
#[tauri::command]
pub fn clear_auth_token() -> Result<(), String> {
    let mut config = read_config()?;
    config.auth_token = None;
    config.token_expires_at = None;
    write_config(config)
}
