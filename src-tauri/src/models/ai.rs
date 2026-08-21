use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AIAnalysisConfig {
    pub language: String,
    pub strict_mode: bool,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AIAnalysisRequest {
    pub segments: Vec<TranscriptSegment>,
    pub config: AIAnalysisConfig,
    pub instructions: Option<String>,
    #[serde(default)]
    pub source_media_id: Option<String>,
    #[serde(default)]
    pub request_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AIEditAction {
    pub id: String,
    pub source_media_id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub action: String, // "CUT", "KEEP", "HIGHLIGHT"
    pub reason: String,
    pub confidence: f32,
    pub taxonomy: String,
    pub source: String,
    pub segment_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AIAnalysisResponse {
    pub actions: Vec<AIEditAction>,
    pub summary: Option<String>,
    pub usage_tokens: Option<u32>, // for billing/audit
    pub provider: Option<String>,
    pub model: Option<String>,
    pub prompt_version: Option<String>,
}

#[derive(Debug, Error)]
pub enum AIProviderError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Rate limit exceeded. Please try again later.")]
    RateLimit,

    #[error("Timeout occurred while waiting for AI response")]
    Timeout,

    #[error("Invalid output format returned by AI: {0}")]
    InvalidOutput(String),

    #[error("Authentication error: invalid provider API key")]
    AuthError,

    #[error("Provider specific error: {0}")]
    Provider(String),
}

impl AIAnalysisResponse {
    pub fn validate_against(&self, request: &AIAnalysisRequest) -> Result<(), AIProviderError> {
        if self.actions.len() > 256 {
            return Err(AIProviderError::InvalidOutput("too many actions".into()));
        }
        for action in &self.actions {
            if action.id.trim().is_empty()
                || action.start_ms >= action.end_ms
                || !matches!(action.action.as_str(), "CUT" | "KEEP" | "HIGHLIGHT")
                || !(0.0..=1.0).contains(&action.confidence)
                || !action.confidence.is_finite()
                || (action.action == "CUT" && action.confidence < 0.8)
                || action.source != "ai"
                || !matches!(
                    action.taxonomy.as_str(),
                    "false_start"
                        | "repeated_take"
                        | "redundant_sentence"
                        | "important_statement"
                        | "none"
                )
            {
                return Err(AIProviderError::InvalidOutput(
                    "action violates canonical EditPlan bounds".into(),
                ));
            }
            let matching_segment = request.segments.iter().find(|segment| {
                segment.start_ms == action.start_ms && segment.end_ms == action.end_ms
            });
            let Some(segment) = matching_segment else {
                return Err(AIProviderError::InvalidOutput(
                    "action timestamp is not an input segment boundary".into(),
                ));
            };
            if request.source_media_id.as_deref() != Some(action.source_media_id.as_str())
                || action.segment_ids != [segment.id.clone()]
            {
                return Err(AIProviderError::InvalidOutput(
                    "action provenance does not match the input segment".into(),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn canonical_ai_contract_uses_millisecond_and_provenance_fields() {
        let value = serde_json::to_value(request()).unwrap();
        assert_eq!(value["segments"][0]["startMs"], 1_000);
        assert_eq!(value["segments"][0]["endMs"], 2_000);
        assert!(value["segments"][0].get("start").is_none());
        assert_eq!(value["requestId"], "request-1");
    }

    #[test]
    fn response_validation_rejects_timestamp_not_in_input() {
        let response = AIAnalysisResponse {
            actions: vec![AIEditAction {
                id: "a1".into(),
                source_media_id: "media-1".into(),
                start_ms: 0,
                end_ms: 1_000,
                action: "CUT".into(),
                reason: "test".into(),
                confidence: 0.9,
                taxonomy: "false_start".into(),
                source: "ai".into(),
                segment_ids: vec!["segment-1".into()],
            }],
            summary: None,
            usage_tokens: None,
            provider: Some("mock".into()),
            model: Some("test".into()),
            prompt_version: Some("semantic-v2".into()),
        };
        assert!(response.validate_against(&request()).is_err());
    }
}
