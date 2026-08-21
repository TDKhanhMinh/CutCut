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
