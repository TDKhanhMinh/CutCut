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
        _request: &AIAnalysisRequest,
    ) -> Result<AIAnalysisResponse, AIProviderError> {
        // Return a mock response
        Ok(AIAnalysisResponse {
            actions: vec![
                AIEditAction {
                    id: "mock-cut".into(),
                    source_media_id: "mock-media".into(),
                    start_ms: 0,
                    end_ms: 1_500,
                    action: "CUT".to_string(),
                    reason: "Silence at the beginning".to_string(),
                    confidence: 0.9,
                    taxonomy: "redundant_sentence".into(),
                    source: "ai".into(),
                    segment_ids: vec![],
                },
                AIEditAction {
                    id: "mock-keep".into(),
                    source_media_id: "mock-media".into(),
                    start_ms: 1_500,
                    end_ms: 5_000,
                    action: "KEEP".to_string(),
                    reason: "Important dialogue".to_string(),
                    confidence: 0.9,
                    taxonomy: "none".into(),
                    source: "ai".into(),
                    segment_ids: vec![],
                },
            ],
            summary: Some("Mock analysis completed successfully.".to_string()),
            usage_tokens: Some(42),
            provider: Some("mock".into()),
            model: Some("deterministic".into()),
            prompt_version: Some("semantic-v1".into()),
        })
    }
}
