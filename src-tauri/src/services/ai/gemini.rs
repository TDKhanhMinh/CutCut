use super::AIProvider;
use crate::models::ai::{AIAnalysisRequest, AIAnalysisResponse, AIEditAction, AIProviderError};
use async_trait::async_trait;
use reqwest::{Client, StatusCode, Url};
use serde_json::{json, Value};
use std::time::Duration;

const MAX_SEGMENTS: usize = 256;
const MAX_SEGMENT_TEXT: usize = 2_000;
const MAX_TOTAL_TEXT: usize = 100_000;
const MAX_INSTRUCTIONS: usize = 4_000;
const MAX_MODEL_NAME: usize = 64;
const MAX_ACTION_REASON: usize = 500;

/// Gemini implementation of the provider-neutral `AIProvider` contract.
///
/// The API key and model are injected by the caller. No Gemini request/response
/// types escape this module and no project/media mutation is performed here.
pub struct GeminiProvider {
    client: Client,
    api_key: String,
    model: String,
}

impl GeminiProvider {
    pub fn new(api_key: String, model: String) -> Result<Self, AIProviderError> {
        if api_key.trim().is_empty() {
            return Err(AIProviderError::InvalidRequest("missing_api_key".into()));
        }
        validate_model_name(&model)?;
        let client = Client::builder()
            .timeout(Duration::from_secs(45))
            .build()
            .map_err(|error| AIProviderError::Network(error.to_string()))?;
        Ok(Self {
            client,
            api_key,
            model,
        })
    }

    fn endpoint(&self) -> Result<Url, AIProviderError> {
        Url::parse(&format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.model
        ))
        .map_err(|_| AIProviderError::InvalidRequest("invalid_model".into()))
    }

    fn prompt(request: &AIAnalysisRequest) -> String {
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
        format!(
            "Return only a JSON array of bounded edit actions.\n{}{}",
            transcript,
            request
                .instructions
                .as_deref()
                .map(|value| format!("\n\nInstructions: {value}"))
                .unwrap_or_default()
        )
    }

    fn request_body(request: &AIAnalysisRequest) -> Value {
        json!({
            "systemInstruction": {
                "parts": [{"text": "Use only input segment boundaries. Actions must be CUT, KEEP, or HIGHLIGHT with confidence 0..1."}]
            },
            "contents": [{"parts": [{"text": Self::prompt(request)}]}],
            "generationConfig": {
                "temperature": request.config.temperature.unwrap_or(0.1).clamp(0.0, 1.0),
                "maxOutputTokens": request.config.max_tokens.unwrap_or(4096).clamp(1, 8192),
                "responseMimeType": "application/json"
            }
        })
    }

    fn parse_response(
        body: &Value,
        request: &AIAnalysisRequest,
        model: &str,
    ) -> Result<AIAnalysisResponse, AIProviderError> {
        let raw_text = body
            .get("candidates")
            .and_then(Value::as_array)
            .and_then(|candidates| candidates.first())
            .and_then(|candidate| candidate.get("content"))
            .and_then(|content| content.get("parts"))
            .and_then(Value::as_array)
            .and_then(|parts| parts.first())
            .and_then(|part| part.get("text"))
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_output("missing candidate text"))?;
        let json_text = raw_text
            .trim()
            .strip_prefix("```json")
            .or_else(|| raw_text.trim().strip_prefix("```JSON"))
            .unwrap_or(raw_text.trim())
            .trim()
            .strip_suffix("```")
            .unwrap_or_else(|| raw_text.trim())
            .trim();
        let parsed: Value = serde_json::from_str(json_text)
            .map_err(|_| invalid_output("response is not valid JSON"))?;
        let list = parsed
            .as_array()
            .ok_or_else(|| invalid_output("response is not an action array"))?;
        if list.len() > MAX_SEGMENTS {
            return Err(invalid_output("too many actions"));
        }

        let actions = list
            .iter()
            .enumerate()
            .map(|(index, candidate)| parse_action(candidate, index, request))
            .collect::<Result<Vec<_>, _>>()?;
        let usage_tokens = body
            .get("usageMetadata")
            .and_then(|usage| usage.get("totalTokenCount"))
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok());
        let response = AIAnalysisResponse {
            actions,
            summary: Some("Gemini semantic analysis completed".into()),
            usage_tokens,
            provider: Some("gemini".into()),
            model: Some(model.to_string()),
            prompt_version: Some("gemini-semantic-v1".into()),
        };
        response.validate_against(request)?;
        Ok(response)
    }
}

#[async_trait]
impl AIProvider for GeminiProvider {
    async fn analyze_transcript(
        &self,
        request: &AIAnalysisRequest,
    ) -> Result<AIAnalysisResponse, AIProviderError> {
        validate_request(request)?;
        let response = self
            .client
            .post(self.endpoint()?)
            .header("x-goog-api-key", &self.api_key)
            .json(&Self::request_body(request))
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    AIProviderError::Timeout
                } else {
                    AIProviderError::Network(error.to_string())
                }
            })?;

        let status = response.status();
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(AIProviderError::AuthError);
        }
        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(AIProviderError::RateLimit);
        }
        if !status.is_success() {
            return Err(AIProviderError::Provider(format!(
                "upstream request failed (HTTP {})",
                status.as_u16()
            )));
        }

        let body: Value = response
            .json()
            .await
            .map_err(|error| invalid_output(&format!("response JSON parse failed: {error}")))?;
        Self::parse_response(&body, request, &self.model)
    }
}

fn validate_model_name(model: &str) -> Result<(), AIProviderError> {
    if model.is_empty()
        || model.len() > MAX_MODEL_NAME
        || !model.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
    {
        return Err(AIProviderError::InvalidRequest("invalid_model".into()));
    }
    Ok(())
}

fn validate_request(request: &AIAnalysisRequest) -> Result<(), AIProviderError> {
    if request.segments.is_empty() || request.segments.len() > MAX_SEGMENTS {
        return Err(AIProviderError::InvalidRequest("invalid_segments".into()));
    }
    if request
        .source_media_id
        .as_deref()
        .is_none_or(|value| value.is_empty() || value.len() > 128)
    {
        return Err(AIProviderError::InvalidRequest(
            "invalid_source_media_id".into(),
        ));
    }
    if request
        .instructions
        .as_deref()
        .is_some_and(|value| value.len() > MAX_INSTRUCTIONS)
    {
        return Err(AIProviderError::InvalidRequest(
            "instructions_too_long".into(),
        ));
    }
    let mut total_chars = 0usize;
    for segment in &request.segments {
        if segment.id.is_empty()
            || segment.id.len() > 128
            || segment.start_ms >= segment.end_ms
            || segment.text.len() > MAX_SEGMENT_TEXT
        {
            return Err(AIProviderError::InvalidRequest("invalid_segment".into()));
        }
        total_chars = total_chars.saturating_add(segment.text.len());
        if total_chars > MAX_TOTAL_TEXT {
            return Err(AIProviderError::InvalidRequest(
                "transcript_too_large".into(),
            ));
        }
    }
    Ok(())
}

fn parse_action(
    candidate: &Value,
    index: usize,
    request: &AIAnalysisRequest,
) -> Result<AIEditAction, AIProviderError> {
    let start_ms = candidate
        .get("startMs")
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid_output("missing startMs"))?;
    let end_ms = candidate
        .get("endMs")
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid_output("missing endMs"))?;
    let action = candidate
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_uppercase();
    let confidence = candidate
        .get("confidence")
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .map(|value| value as f32)
        .ok_or_else(|| invalid_output("missing confidence"))?;
    let segment = request
        .segments
        .iter()
        .find(|segment| segment.start_ms == start_ms && segment.end_ms == end_ms)
        .ok_or_else(|| invalid_output("action timestamp is not an input boundary"))?;
    if !matches!(action.as_str(), "CUT" | "KEEP" | "HIGHLIGHT")
        || !(0.0..=1.0).contains(&confidence)
        || (action == "CUT" && confidence < 0.8)
    {
        return Err(invalid_output("action violates provider bounds"));
    }

    Ok(AIEditAction {
        id: candidate
            .get("id")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&format!("gemini-{index}"))
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
            .take(MAX_ACTION_REASON)
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
}

fn invalid_output(message: &str) -> AIProviderError {
    AIProviderError::InvalidOutput(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ai::{AIAnalysisConfig, TranscriptSegment};

    fn request() -> AIAnalysisRequest {
        AIAnalysisRequest {
            segments: vec![TranscriptSegment {
                id: "segment-1".into(),
                start_ms: 1_000,
                end_ms: 2_000,
                text: "Xin chào".into(),
            }],
            config: AIAnalysisConfig {
                language: "vi".into(),
                strict_mode: true,
                max_tokens: Some(100),
                temperature: Some(0.1),
            },
            instructions: None,
            source_media_id: Some("media-1".into()),
            request_id: Some("request-1".into()),
        }
    }

    #[test]
    fn model_is_injected_and_validated() {
        assert!(GeminiProvider::new("key".into(), "gemini-2.5-flash".into()).is_ok());
        assert!(GeminiProvider::new("key".into(), "gemini/unsafe".into()).is_err());
    }

    #[test]
    fn provider_json_is_mapped_to_canonical_output() {
        let body = json!({
            "candidates": [{"content": {"parts": [{"text": "```json\n[{\"id\":\"a1\",\"startMs\":1000,\"endMs\":2000,\"action\":\"KEEP\",\"reason\":\"clear\",\"confidence\":0.9,\"taxonomy\":\"none\"}]\n```"}]}}],
            "usageMetadata": {"totalTokenCount": 12}
        });
        let result = GeminiProvider::parse_response(&body, &request(), "gemini-2.5-flash").unwrap();
        assert_eq!(result.actions[0].segment_ids, vec!["segment-1"]);
        assert_eq!(result.usage_tokens, Some(12));
        assert_eq!(result.model.as_deref(), Some("gemini-2.5-flash"));
    }

    #[test]
    fn invalid_provider_json_is_a_domain_error() {
        let body = json!({
            "candidates": [{"content": {"parts": [{"text": "not-json"}]}}]
        });
        assert!(matches!(
            GeminiProvider::parse_response(&body, &request(), "gemini-2.5-flash"),
            Err(AIProviderError::InvalidOutput(_))
        ));
    }
}
