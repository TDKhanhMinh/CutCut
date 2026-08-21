use crate::models::media_job::{MediaJobEvent, MediaJobState};
use crate::models::vad::VadConfig;
use crate::services::audio_extraction_service::AudioExtractionService;
use crate::services::media_job::JobManager;
use crate::services::vad_detector::VadDetectionService;
use tauri::{command, AppHandle, Emitter, Manager};

#[command]
pub async fn start_vad_analysis(
    app: AppHandle,
    source_path: String,
    job_id: String,
    duration_ms: u64,
    config: VadConfig,
) -> Result<(), String> {
    if duration_ms == 0 {
        return Err("Media duration is 0 or unknown".to_string());
    }

    let audio_path = AudioExtractionService::extract_audio_for_stt(
        app.clone(),
        source_path,
        Some(job_id.clone()),
        Some(duration_ms.saturating_mul(1_000)),
    )
    .await
    .map_err(|e| format!("Failed to extract audio: {e}"))?;

    let job_manager = app.state::<JobManager>().inner().clone();
    job_manager.register_cooperative(job_id.clone()).await;
    let _ = app.emit(
        "media-job",
        MediaJobEvent {
            job_id: job_id.clone(),
            state: MediaJobState::Started,
            progress: Some(0.0),
            message: Some("Local VAD analysis started".to_string()),
            error: None,
            result: None,
        },
    );

    let app_clone = app.clone();
    let job_id_clone = job_id.clone();
    let audio_path_clone = audio_path.clone();
    tokio::spawn(async move {
        let _ = app_clone.emit(
            "media-job",
            MediaJobEvent {
                job_id: job_id_clone.clone(),
                state: MediaJobState::Progress,
                progress: Some(0.25),
                message: Some("Analyzing local PCM frames".to_string()),
                error: None,
                result: None,
            },
        );

        let path_for_worker = std::path::PathBuf::from(audio_path_clone.clone());
        let config_for_worker = config.clone();
        let analysis = tokio::task::spawn_blocking(move || {
            VadDetectionService::analyze_wav(&path_for_worker, duration_ms, config_for_worker)
        })
        .await;

        let job_manager = app_clone.state::<JobManager>().inner().clone();
        let cancelled = job_manager.is_cancelled(&job_id_clone).await;
        let _ = AudioExtractionService::cleanup_stt_audio(&app_clone, audio_path_clone);

        if cancelled {
            job_manager.finish_cooperative(&job_id_clone).await;
            let _ = app_clone.emit(
                "media-job",
                MediaJobEvent {
                    job_id: job_id_clone,
                    state: MediaJobState::Cancelled,
                    progress: None,
                    message: Some("Local VAD analysis cancelled".to_string()),
                    error: None,
                    result: None,
                },
            );
            return;
        }

        match analysis {
            Ok(Ok(result)) => {
                let typed_result = serde_json::to_value(result).ok();
                let _ = app_clone.emit(
                    "media-job",
                    MediaJobEvent {
                        job_id: job_id_clone.clone(),
                        state: MediaJobState::Completed,
                        progress: Some(1.0),
                        message: Some("Local VAD analysis completed".to_string()),
                        error: None,
                        result: typed_result,
                    },
                );
            }
            Ok(Err(error)) => {
                let _ = app_clone.emit(
                    "media-job",
                    MediaJobEvent {
                        job_id: job_id_clone.clone(),
                        state: MediaJobState::Failed,
                        progress: None,
                        message: None,
                        error: Some(error.to_string()),
                        result: None,
                    },
                );
            }
            Err(error) => {
                let _ = app_clone.emit(
                    "media-job",
                    MediaJobEvent {
                        job_id: job_id_clone.clone(),
                        state: MediaJobState::Failed,
                        progress: None,
                        message: None,
                        error: Some(format!("VAD worker failed: {error}")),
                        result: None,
                    },
                );
            }
        }
        job_manager.finish_cooperative(&job_id_clone).await;
    });

    Ok(())
}
