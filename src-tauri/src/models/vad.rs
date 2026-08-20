use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadConfig {
    pub min_speech_duration_ms: u32,
    pub min_silence_duration_ms: u32,
    pub speech_pad_ms: u32,
    pub threshold: f32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            min_speech_duration_ms: 250,
            min_silence_duration_ms: 100,
            speech_pad_ms: 30,
            threshold: 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechInterval {
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonSpeechInterval {
    pub start_ms: u64,
    pub end_ms: u64,
    pub reason: String, // "noise_only", "silence", "uncertain"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadAnalysisResult {
    pub provider: String,
    pub version: String,
    pub speech_intervals: Vec<SpeechInterval>,
    pub non_speech_intervals: Vec<NonSpeechInterval>,
    pub config_used: VadConfig,
}
