use super::AIProvider;
use crate::models::ai::{AIAnalysisRequest, AIAnalysisResponse, AIProviderError};

/// Provider-neutral application service. UI and EditPlan orchestration depend
/// on this contract, never on Gemini/Supabase-specific request details.
pub struct AIAnalysisService<P> {
    provider: P,
}

impl<P> AIAnalysisService<P>
where
    P: AIProvider,
{
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    pub async fn analyze(
        &self,
        request: &AIAnalysisRequest,
    ) -> Result<AIAnalysisResponse, AIProviderError> {
        let response = self.provider.analyze_transcript(request).await?;
        response.validate_against(request)?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ai::{AIAnalysisConfig, TranscriptSegment};
    use crate::services::ai::mock::MockAIProvider;

    #[tokio::test]
    async fn provider_neutral_service_accepts_mock_output() {
        let request = AIAnalysisRequest {
            segments: vec![TranscriptSegment {
                id: "segment-1".into(),
                start_ms: 0,
                end_ms: 1_000,
                text: "Xin chào".into(),
            }],
            config: AIAnalysisConfig {
                language: "vi".into(),
                strict_mode: true,
                max_tokens: Some(256),
                temperature: Some(0.1),
            },
            instructions: None,
            source_media_id: Some("media-1".into()),
            request_id: Some("mock-request-1".into()),
        };

        let response = AIAnalysisService::new(MockAIProvider::new())
            .analyze(&request)
            .await
            .expect("mock provider output should satisfy canonical validation");

        assert_eq!(response.provider.as_deref(), Some("mock"));
        assert_eq!(response.actions[0].segment_ids, vec!["segment-1"]);
        assert_eq!(response.actions[0].source_media_id, "media-1");
    }
}
