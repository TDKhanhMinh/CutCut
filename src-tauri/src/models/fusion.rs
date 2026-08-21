use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Confidence {
    High,   // Silence + Non-speech
    Medium, // Non-speech but NOT Silence (e.g. background noise)
    Low,    // Silence but VAD says Speech (Uncertain/Speech protected)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorEvidence {
    pub has_amplitude_silence: bool,
    pub has_vad_non_speech: bool,
    pub original_silence_duration_ms: Option<u64>,
    pub original_vad_non_speech_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonSpeechCandidate {
    pub start_ms: u64,
    pub end_ms: u64,
    pub reason: String,
    pub evidence: DetectorEvidence,
    pub confidence: Confidence,
    pub recommended_padding_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionConfig {
    pub lead_in_padding_ms: u64,
    pub lead_out_padding_ms: u64,
    pub min_candidate_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionResult {
    pub candidates: Vec<NonSpeechCandidate>,
    pub config_used: FusionConfig,
    pub analysis_version: String,
}
