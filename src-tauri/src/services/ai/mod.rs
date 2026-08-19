pub mod supabase;
pub mod mock;

use async_trait::async_trait;
use crate::models::ai::{AIAnalysisRequest, AIAnalysisResponse, AIProviderError};

#[async_trait]
pub trait AIProvider: Send + Sync {
    /// Analyzes a transcript and proposes edit actions.
    async fn analyze_transcript(
        &self,
        request: &AIAnalysisRequest,
    ) -> Result<AIAnalysisResponse, AIProviderError>;
}
