use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_at: String,
    pub role: Option<String>,
    pub must_change_password: Option<bool>,
}

/// Calls the devkit-specific 90-day login endpoint (see
/// backend-core-node/internal/handler/http/auth.go's LoginTokenDevkit) — NOT
/// the standard 8h /auth/login/token used by web-panel/connect. Distinct
/// endpoint deliberately keeps their default session length unaffected.
#[tauri::command]
pub async fn login(host: String, username: String, password: String) -> Result<LoginResponse, String> {
    let url = format!("{}/api/v1/auth/login/token/devkit", host.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&serde_json::json!({ "username": username, "password": password }))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Login failed ({status}): {body}"));
    }

    resp.json::<LoginResponse>()
        .await
        .map_err(|e| format!("Failed to parse login response: {e}"))
}
