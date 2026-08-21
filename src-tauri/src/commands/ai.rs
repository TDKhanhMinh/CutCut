use crate::commands::auth::{read_secure_value, GEMINI_BYOK_KEY};
use crate::models::ai::{AIAnalysisRequest, AIAnalysisResponse, AIProviderError};
use crate::services::ai::{gemini::GeminiProvider, AIProvider};
use reqwest::{Client, StatusCode};
use serde_json::json;
use std::time::Duration;

/// Default BYOK model. The provider still receives it through constructor
/// configuration so switching models does not change editor/business logic.
const GEMINI_MODEL: &str = "gemini-1.5-flash";

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiKeyStatus {
    pub configured: bool,
    pub masked_hint: Option<String>,
}

fn key_status(key: Option<String>) -> GeminiKeyStatus {
    let masked_hint = key.map(|value| {
        let chars: Vec<char> = value.chars().collect();
        if chars.len() <= 8 {
            "••••••••".to_string()
        } else {
            format!(
                "{}••••{}",
                chars.iter().take(4).collect::<String>(),
                chars.iter().skip(chars.len() - 4).collect::<String>()
            )
        }
    });
    GeminiKeyStatus {
        configured: masked_hint.is_some(),
        masked_hint,
    }
}

fn api_key() -> Result<String, String> {
    read_secure_value(GEMINI_BYOK_KEY)?.ok_or_else(|| "not_configured".into())
}

fn map_provider_error(error: AIProviderError) -> String {
    match error {
        AIProviderError::InvalidRequest(code) => code,
        AIProviderError::AuthError => "invalid_key".into(),
        AIProviderError::RateLimit => "rate_limited".into(),
        AIProviderError::Timeout => "provider_timeout".into(),
        AIProviderError::InvalidOutput(_) => "invalid_provider_output".into(),
        AIProviderError::Network(_) | AIProviderError::Provider(_) => "provider_unavailable".into(),
    }
}

#[tauri::command]
pub fn get_gemini_key_status() -> Result<GeminiKeyStatus, String> {
    Ok(key_status(read_secure_value(GEMINI_BYOK_KEY)?))
}

#[tauri::command]
pub fn set_gemini_api_key(api_key: String) -> Result<GeminiKeyStatus, String> {
    if api_key.trim().is_empty() || api_key.len() > 512 || api_key.chars().any(char::is_whitespace)
    {
        return Err("invalid_key".into());
    }
    crate::commands::auth::write_secure_value(GEMINI_BYOK_KEY, &api_key)?;
    Ok(key_status(Some(api_key)))
}

#[tauri::command]
pub fn delete_gemini_api_key() -> Result<(), String> {
    crate::commands::auth::delete_secure_value(GEMINI_BYOK_KEY)
}

#[tauri::command]
pub async fn test_gemini_key() -> Result<(), String> {
    let key = api_key()?;
    let endpoint = reqwest::Url::parse(&format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{GEMINI_MODEL}:generateContent"
    ))
    .map_err(|_| "provider_unavailable")?;
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| "provider_unavailable")?;
    let response = client
        .post(endpoint)
        .header("x-goog-api-key", key)
        .json(&json!({
            "contents": [{"parts": [{"text": "Reply with the single word OK."}]}],
            "generationConfig": {"maxOutputTokens": 4}
        }))
        .send()
        .await
        .map_err(|_| "provider_unavailable")?;
    match response.status() {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err("invalid_key".into()),
        StatusCode::TOO_MANY_REQUESTS => Err("rate_limited".into()),
        status if status.is_success() => Ok(()),
        _ => Err("provider_unavailable".into()),
    }
}

/// Compatibility command for the BYOK UI. Provider construction and Gemini
/// wire-format handling live in `services::ai::gemini`, not in the command or
/// editor business layer.
#[tauri::command]
pub async fn call_gemini_direct(request: AIAnalysisRequest) -> Result<AIAnalysisResponse, String> {
    let provider =
        GeminiProvider::new(api_key()?, GEMINI_MODEL.into()).map_err(map_provider_error)?;
    provider
        .analyze_transcript(&request)
        .await
        .map_err(map_provider_error)
}
