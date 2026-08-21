use crate::services::auth_session::{AuthSession, AuthSessionStore};
use keyring::Entry;
use tauri::{AppHandle, Manager};

pub(crate) const SUPABASE_AUTH_KEY: &str = "supabase-auth-session";
pub(crate) const GEMINI_BYOK_KEY: &str = "gemini-byok";

fn secure_entry(key: &str) -> Result<Entry, String> {
    let service_key = match key {
        SUPABASE_AUTH_KEY => SUPABASE_AUTH_KEY,
        GEMINI_BYOK_KEY => GEMINI_BYOK_KEY,
        _ => return Err("Unsupported secure credential key".into()),
    };
    Entry::new("cutcut", service_key).map_err(|error| error.to_string())
}

fn public_session_key(key: &str) -> Result<(), String> {
    if key == SUPABASE_AUTH_KEY {
        Ok(())
    } else {
        Err("Unsupported public secure token key".into())
    }
}

pub(crate) fn read_secure_value(key: &str) -> Result<Option<String>, String> {
    let entry = secure_entry(key)?;
    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

pub(crate) fn write_secure_value(key: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > 16_384 || value.chars().any(char::is_whitespace) {
        return Err("Invalid secure credential".into());
    }
    secure_entry(key)?
        .set_password(value)
        .map_err(|error| error.to_string())
}

pub(crate) fn delete_secure_value(key: &str) -> Result<(), String> {
    match secure_entry(key)?.delete_credential() {
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

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

#[tauri::command]
pub fn set_secure_token(key: String, value: String) -> Result<(), String> {
    public_session_key(&key)?;
    write_secure_value(&key, &value)
}

#[tauri::command]
pub fn get_secure_token(key: String) -> Result<Option<String>, String> {
    public_session_key(&key)?;
    read_secure_value(&key)
}

#[tauri::command]
pub fn delete_secure_token(key: String) -> Result<(), String> {
    public_session_key(&key)?;
    delete_secure_value(&key)
}
