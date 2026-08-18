use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::models::ai::{AIAnalysisRequest, AIAnalysisResponse, AIProviderError};
use super::AIProvider;
use std::time::Duration;

pub struct GeminiProvider {
    client: Client,
    api_key: String,
    model_name: String,
}

impl GeminiProvider {
    pub fn new(api_key: String, model_name: Option<String>) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            api_key,
            model_name: model_name.unwrap_or_else(|| "gemini-1.5-flash".to_string()),
        }
    }
}

// Minimal Gemini REST types
#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContentOutput>,
}

#[derive(Deserialize)]
struct GeminiContentOutput {
    parts: Option<Vec<GeminiPartOutput>>,
}

#[derive(Deserialize)]
struct GeminiPartOutput {
    text: Option<String>,
}

#[async_trait]
impl AIProvider for GeminiProvider {
    async fn analyze_transcript(
        &self,
        request: &AIAnalysisRequest,
    ) -> Result<AIAnalysisResponse, AIProviderError> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model_name, self.api_key
        );

        // Serialize transcript segments to text prompt
        let mut prompt = String::new();
        prompt.push_str("Analyze the following transcript and suggest edits (CUT/KEEP/HIGHLIGHT).\n");
        prompt.push_str("Return a JSON array of objects with fields: start (float), end (float), action (string), reason (string).\n\n");
        
        for segment in &request.segments {
            prompt.push_str(&format!("[{:.2} - {:.2}] {}\n", segment.start, segment.end, segment.text));
        }

        if let Some(instructions) = &request.instructions {
            prompt.push_str(&format!("\nUser Instructions: {}\n", instructions));
        }

        let gemini_req = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart { text: prompt }],
            }],
        };

        let response = self.client.post(&url)
            .json(&gemini_req)
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
            return Err(AIProviderError::Provider(format!("HTTP {}: {}", status, err_body)));
        }

        let body: GeminiResponse = response.json().await.map_err(|e| {
            AIProviderError::InvalidOutput(format!("Failed to parse JSON response: {}", e))
        })?;

        let text = body.candidates
            .and_then(|mut c| c.pop())
            .and_then(|c| c.content)
            .and_then(|mut content| content.parts.take())
            .and_then(|mut parts| parts.pop())
            .and_then(|p| p.text)
            .unwrap_or_default();

        // Strip markdown backticks if Gemini returned them
        let clean_text = text.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();

        let actions = serde_json::from_str(clean_text).map_err(|e| {
            AIProviderError::InvalidOutput(format!("Failed to parse Structured Actions: {}", e))
        })?;

        Ok(AIAnalysisResponse {
            actions,
            summary: Some("Gemini analysis completed".to_string()),
            usage_tokens: None, // Hard to extract easily from raw REST v1beta without usageMetadata
        })
    }
}
