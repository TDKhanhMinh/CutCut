use crate::services::auth_session::{AuthSession, AuthSessionStore};
use tauri::{AppHandle, Manager};

#[tauri::command]
pub async fn set_auth_session(
    app: AppHandle,
    access_token: String,
    expires_at: Option<u64>,
    user_id: Option<String>,
) -> Result<(), String> {
    if access_token.len() < 32
        || access_token.len() > 16_384
        || access_token.chars().any(char::is_whitespace)
    {
        return Err("Invalid access token".into());
    }
    if let Some(id) = &user_id {
        uuid::Uuid::parse_str(id).map_err(|_| "Invalid user id".to_string())?;
    }
    app.state::<AuthSessionStore>()
        .set(AuthSession {
            access_token,
            expires_at,
            user_id,
        })
        .await;
    Ok(())
}

#[tauri::command]
pub async fn get_auth_session(app: AppHandle) -> Option<AuthSession> {
    app.state::<AuthSessionStore>().get().await
}

#[tauri::command]
pub async fn clear_auth_session(app: AppHandle) {
    app.state::<AuthSessionStore>().clear().await;
}
