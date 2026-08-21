use super::AIProvider;
use crate::models::ai::{AIAnalysisRequest, AIAnalysisResponse, AIEditAction, AIProviderError};
use async_trait::async_trait;

pub struct MockAIProvider;

impl MockAIProvider {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for MockAIProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AIProvider for MockAIProvider {
    async fn analyze_transcript(
        &self,
        request: &AIAnalysisRequest,
    ) -> Result<AIAnalysisResponse, AIProviderError> {
        let actions = request
            .source_media_id
            .as_ref()
            .map(|source_media_id| {
                request
                    .segments
                    .iter()
                    .map(|segment| AIEditAction {
                        id: format!("mock-{}", segment.id),
                        source_media_id: source_media_id.clone(),
                        start_ms: segment.start_ms,
                        end_ms: segment.end_ms,
                        action: "KEEP".into(),
                        reason: "Deterministic mock suggestion".into(),
                        confidence: 1.0,
                        taxonomy: "none".into(),
                        source: "ai".into(),
                        segment_ids: vec![segment.id.clone()],
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(AIAnalysisResponse {
            actions,
            summary: Some("Deterministic mock analysis completed.".into()),
            usage_tokens: Some(0),
            provider: Some("mock".into()),
            model: Some("deterministic".into()),
            prompt_version: Some("semantic-v2".into()),
        })
    }
}
