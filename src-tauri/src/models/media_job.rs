use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum MediaJobState {
    Started,
    Progress,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MediaJobEvent {
    pub job_id: String,
    pub state: MediaJobState,
    pub progress: Option<f64>, // 0.0 to 1.0
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Default)]
pub struct FfmpegProgress {
    pub frame: Option<u64>,
    pub fps: Option<f64>,
    pub out_time_us: Option<u64>,
    pub speed: Option<f64>,
}
