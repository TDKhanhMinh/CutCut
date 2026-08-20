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
                    start: 0.0,
                    end: 1.5,
                    action: "CUT".to_string(),
                    reason: "Silence at the beginning".to_string(),
                },
                AIEditAction {
                    start: 1.5,
                    end: 5.0,
                    action: "KEEP".to_string(),
                    reason: "Important dialogue".to_string(),
                },
            ],
            summary: Some("Mock analysis completed successfully.".to_string()),
            usage_tokens: Some(42),
        })
    }
}
