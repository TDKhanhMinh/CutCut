use crate::models::fusion::{FusionConfig, FusionResult};
use crate::models::silence::SilenceInterval;
use crate::models::vad::VadAnalysisResult;
use crate::services::fusion_service::FusionService;
use tauri::command;

#[command]
pub async fn fuse_non_speech_intervals(
    duration_ms: u64,
    silence: Vec<SilenceInterval>,
    vad: VadAnalysisResult,
    config: FusionConfig,
) -> Result<FusionResult, String> {
    if duration_ms == 0 {
        return Err("Media duration is 0".to_string());
    }

    let result = FusionService::fuse_intervals(duration_ms, &silence, &vad, &config);
    Ok(result)
}
