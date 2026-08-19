use async_trait::async_trait;
use crate::models::ai::{AIAnalysisRequest, AIAnalysisResponse, AIAnalysisAction, AIProviderError};
use super::AIProvider;

pub struct MockAIProvider;

impl MockAIProvider {
    pub fn new() -> Self {
        Self {}
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
                AIAnalysisAction {
                    start: 1.5,
                    end: 3.0,
                    action: "CUT".to_string(),
                    reason: "Mock false start detection".to_string(),
                    confidence: 0.85,
                    taxonomy: "false_start".to_string(),
                },
                AIAnalysisAction {
                    start: 5.0,
                    end: 8.5,
                    action: "HIGHLIGHT".to_string(),
                    reason: "Mock important statement".to_string(),
                    confidence: 0.95,
                    taxonomy: "important_statement".to_string(),
                },
            ],
            summary: Some("Mock analysis completed successfully.".to_string()),
            usage_tokens: Some(42),
        })
    }
}
