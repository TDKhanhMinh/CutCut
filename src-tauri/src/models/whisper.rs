use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperRuntimeInfo {
    pub available: bool,
    pub version: String,
    pub backend: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WhisperTimestamp {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WhisperOffset {
    pub from: i64,
    pub to: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WhisperSegment {
    pub timestamps: WhisperTimestamp,
    pub offsets: WhisperOffset,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WhisperResult {
    pub transcription: Vec<WhisperSegment>,
}
