pub mod gemini;
pub mod mock;
pub mod service;
pub mod supabase;

use crate::models::ai::{AIAnalysisRequest, AIAnalysisResponse, AIProviderError};
use async_trait::async_trait;

#[async_trait]
pub trait AIProvider: Send + Sync {
    /// Analyzes a transcript and proposes edit actions.
    async fn analyze_transcript(
        &self,
        request: &AIAnalysisRequest,
    ) -> Result<AIAnalysisResponse, AIProviderError>;
}
