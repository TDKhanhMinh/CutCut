use super::AIProvider;
use crate::models::ai::{AIAnalysisRequest, AIAnalysisResponse, AIProviderError};
use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;

pub struct SupabaseAIProvider {
    client: Client,
    jwt_token: String,
    function_url: String,
}

impl SupabaseAIProvider {
    pub fn new(jwt_token: String, function_url: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(45))
                .build()
                .unwrap_or_default(),
            jwt_token,
            function_url,
        }
    }
}

#[async_trait]
impl AIProvider for SupabaseAIProvider {
    async fn analyze_transcript(
        &self,
        request: &AIAnalysisRequest,
    ) -> Result<AIAnalysisResponse, AIProviderError> {
        let response = self
            .client
            .post(&self.function_url)
            .header("Authorization", format!("Bearer {}", self.jwt_token))
            .json(request)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AIProviderError::Timeout
                } else {
                    AIProviderError::Network(e.to_string())
                }
            })?;

        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            if status == 429 {
                return Err(AIProviderError::RateLimit);
            }
            if status == 401 || status == 403 {
                return Err(AIProviderError::AuthError);
            }
            let err_body = response.text().await.unwrap_or_default();
            return Err(AIProviderError::Provider(format!(
                "HTTP {}: {}",
                status, err_body
            )));
        }

        let body: AIAnalysisResponse = response.json().await.map_err(|e| {
            AIProviderError::InvalidOutput(format!("Failed to parse Supabase JSON response: {}", e))
        })?;

        Ok(body)
    }
}
