use crate::commands::auth::{read_secure_value, GEMINI_BYOK_KEY};
use crate::models::ai::{AIAnalysisRequest, AIAnalysisResponse, AIEditAction};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::time::Duration;

const GEMINI_MODEL: &str = "gemini-1.5-flash";
const MAX_SEGMENTS: usize = 256;
const MAX_SEGMENT_TEXT: usize = 2_000;
const MAX_TOTAL_TEXT: usize = 100_000;
const MAX_INSTRUCTIONS: usize = 4_000;

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

fn validate_request(request: &AIAnalysisRequest) -> Result<(), String> {
    if request.segments.is_empty() || request.segments.len() > MAX_SEGMENTS {
        return Err("invalid_segments".into());
    }
    if request
        .source_media_id
        .as_deref()
        .is_none_or(|value| value.is_empty() || value.len() > 128)
    {
        return Err("invalid_source_media_id".into());
    }
    if request
        .instructions
        .as_deref()
        .is_some_and(|value| value.len() > MAX_INSTRUCTIONS)
    {
        return Err("instructions_too_long".into());
    }
    let mut total_chars = 0usize;
    for segment in &request.segments {
        if segment.id.is_empty()
            || segment.id.len() > 128
            || segment.start_ms >= segment.end_ms
            || segment.text.len() > MAX_SEGMENT_TEXT
        {
            return Err("invalid_segment".into());
        }
        total_chars += segment.text.len();
    }
    if total_chars > MAX_TOTAL_TEXT {
        return Err("transcript_too_large".into());
    }
    Ok(())
}

fn provider_actions(
    value: &Value,
    request: &AIAnalysisRequest,
) -> Result<Vec<AIEditAction>, String> {
    let actions = value
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|candidates| candidates.first())
        .and_then(|candidate| candidate.get("content"))
        .and_then(|content| content.get("parts"))
        .and_then(Value::as_array)
        .and_then(|parts| parts.first())
        .and_then(|part| part.get("text"))
        .and_then(Value::as_str)
        .ok_or_else(|| "invalid_provider_output".to_string())?;
    let parsed: Value = serde_json::from_str(actions).map_err(|_| "invalid_provider_output")?;
    let list = parsed.as_array().ok_or("invalid_provider_output")?;
    if list.len() > 256 {
        return Err("invalid_provider_output".into());
    }

    list.iter()
        .enumerate()
        .map(|(index, candidate)| {
            let start_ms = candidate
                .get("startMs")
                .and_then(Value::as_u64)
                .ok_or("invalid_provider_output")?;
            let end_ms = candidate
                .get("endMs")
                .and_then(Value::as_u64)
                .ok_or("invalid_provider_output")?;
            let action = candidate
                .get("action")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_uppercase();
            let confidence = candidate
                .get("confidence")
                .and_then(Value::as_f64)
                .ok_or("invalid_provider_output")? as f32;
            let segment = request
                .segments
                .iter()
                .find(|segment| segment.start_ms == start_ms && segment.end_ms == end_ms)
                .ok_or("invalid_provider_output")?;
            if !matches!(action.as_str(), "CUT" | "KEEP" | "HIGHLIGHT")
                || !(0.0..=1.0).contains(&confidence)
                || (action == "CUT" && confidence < 0.8)
            {
                return Err("invalid_provider_output".into());
            }
            Ok(AIEditAction {
                id: candidate
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or(&format!("byok-{}", index + 1))
                    .to_string(),
                source_media_id: request.source_media_id.clone().unwrap_or_default(),
                start_ms,
                end_ms,
                action,
                reason: candidate
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .chars()
                    .take(500)
                    .collect(),
                confidence,
                taxonomy: candidate
                    .get("taxonomy")
                    .and_then(Value::as_str)
                    .unwrap_or("none")
                    .to_string(),
                source: "ai".into(),
                segment_ids: vec![segment.id.clone()],
            })
        })
        .collect()
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

#[tauri::command]
pub async fn call_gemini_direct(request: AIAnalysisRequest) -> Result<AIAnalysisResponse, String> {
    validate_request(&request)?;
    let key = api_key()?;
    let model = GEMINI_MODEL;
    let transcript = request
        .segments
        .iter()
        .map(|segment| {
            format!(
                "[{}-{}] {}: {}",
                segment.start_ms, segment.end_ms, segment.id, segment.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let prompt = format!(
        "Return only a JSON array of bounded edit actions.\n{}{}",
        transcript,
        request
            .instructions
            .as_deref()
            .map(|value| format!("\n\nInstructions: {value}"))
            .unwrap_or_default()
    );
    let client = Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|_| "provider_unavailable")?;
    let endpoint = reqwest::Url::parse(&format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"
    ))
    .map_err(|_| "provider_unavailable")?;
    let response = client
        .post(endpoint)
        .header("x-goog-api-key", key)
        .json(&json!({
            "systemInstruction": {"parts": [{"text": "Use only input segment boundaries. Actions must be CUT, KEEP, or HIGHLIGHT with confidence 0..1."}]},
            "contents": [{"parts": [{"text": prompt}]}],
            "generationConfig": {"temperature": 0.1, "maxOutputTokens": request.config.max_tokens.unwrap_or(4096).min(8192), "responseMimeType": "application/json"}
        }))
        .send()
        .await
        .map_err(|_| "provider_unavailable")?;
    let status = response.status();
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Err("invalid_key".into());
    }
    if status == StatusCode::TOO_MANY_REQUESTS {
        return Err("rate_limited".into());
    }
    if !status.is_success() {
        return Err("provider_unavailable".into());
    }
    let body: Value = response
        .json()
        .await
        .map_err(|_| "invalid_provider_output")?;
    let actions = provider_actions(&body, &request)?;
    let usage_tokens = body
        .get("usageMetadata")
        .and_then(|usage| usage.get("totalTokenCount"))
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    let result = AIAnalysisResponse {
        actions,
        summary: Some("BYOK semantic analysis completed".into()),
        usage_tokens,
        provider: Some("gemini-byok".into()),
        model: Some(model.into()),
        prompt_version: Some("byok-semantic-v1".into()),
    };
    result
        .validate_against(&request)
        .map_err(|_| String::from("invalid_provider_output"))?;
    Ok(result)
}
