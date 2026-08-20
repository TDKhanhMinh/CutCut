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
    /// Optional typed operation result. `message` stays reserved for a short
    /// human-readable status/diagnostic string.
    pub result: Option<serde_json::Value>,
}

#[derive(Debug, Default)]
pub struct FfmpegProgress {
    pub frame: Option<u64>,
    pub fps: Option<f64>,
    pub out_time_us: Option<u64>,
    pub speed: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::MediaJobState;

    #[test]
    fn serializes_event_states_in_frontend_contract_case() {
        assert_eq!(
            serde_json::to_string(&MediaJobState::Completed).unwrap(),
            "\"completed\""
        );
        assert_eq!(
            serde_json::to_string(&MediaJobState::Cancelled).unwrap(),
            "\"cancelled\""
        );
    }
}
