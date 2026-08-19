use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AIAnalysisConfig {
    pub language: String,
    pub strict_mode: bool,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranscriptSegment {
    pub start: f32,
    pub end: f32,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AIAnalysisRequest {
    pub segments: Vec<TranscriptSegment>,
    pub config: AIAnalysisConfig,
    pub instructions: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AIAnalysisAction {
    pub start: f64,
    pub end: f64,
    pub action: String, // CUT, KEEP, HIGHLIGHT
    pub reason: String,
    pub confidence: f32,
    pub taxonomy: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AIAnalysisResponse {
    pub actions: Vec<AIAnalysisAction>,
    pub summary: Option<String>,
    pub usage_tokens: Option<u32>, // for billing/audit
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
