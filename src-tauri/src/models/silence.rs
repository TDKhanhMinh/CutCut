use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SilencePreset {
    Conservative,
    Balanced,
    Aggressive,
    Custom,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SilenceConfig {
    pub preset: SilencePreset,
    pub settings: SilenceSettings,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SilenceSettings {
    pub threshold_db: i32,
    pub min_duration_ms: u64,
    #[serde(default)]
    pub padding_ms: u64,
}

pub const SILENCE_DETECTOR_VERSION: &str = "ffmpeg-silencedetect-v1";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SilenceDetectionMetadata {
    pub detector_version: String,
    pub threshold_db: i32,
    pub min_duration_ms: u64,
    pub padding_ms: u64,
    pub tail_policy: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SilenceInterval {
    pub id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub duration_ms: u64,
    pub detection: SilenceDetectionMetadata,
    pub measured_level_db: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SilenceDetectionResult {
    pub source_duration_ms: Option<u64>,
    pub detection: SilenceDetectionMetadata,
    pub intervals: Vec<SilenceInterval>,
}
