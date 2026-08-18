use tauri::{command, AppHandle, Manager, Emitter};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;
use crate::models::vad::{VadConfig, VadAnalysisResult};
use crate::services::vad_detector::VadDetectionService;
use crate::services::audio_extraction_service::AudioExtractionService;
use crate::services::media_job::JobManager;
use crate::models::media_job::{MediaJobEvent, MediaJobState};

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

    // Extract audio for VAD
    let duration_us = duration_ms * 1000;
    let audio_path = AudioExtractionService::extract_audio_for_stt(
        app.clone(),
        source_path,
        job_id.clone(),
        Some(duration_us),
    ).await.map_err(|e| format!("Failed to extract audio: {}", e))?;

    let bin_path = VadDetectionService::get_vad_binary_path(&app)
        .map_err(|e| format!("VAD bin error: {}", e))?;
    let model_path = VadDetectionService::get_vad_model_path(&app)
        .map_err(|e| format!("VAD model error: {}", e))?;

    let args = vec![
        "-f".to_string(),
        audio_path.clone(),
        "-vm".to_string(),
        model_path.to_string_lossy().to_string(),
        "-vt".to_string(),
        format!("{:.2}", config.threshold),
        "-vspd".to_string(),
        config.min_speech_duration_ms.to_string(),
        "-vsd".to_string(),
        config.min_silence_duration_ms.to_string(),
        "-vp".to_string(),
        config.speech_pad_ms.to_string(),
        "-np".to_string(),
    ];

    let cmd = app
        .shell()
        .command(bin_path.to_string_lossy().to_string())
        .args(args);

    let (mut rx, child) = cmd.spawn().map_err(|e| format!("Spawn error: {}", e))?;

    let job_manager = app.state::<JobManager>();
    job_manager.add_job(job_id.clone(), child).await;

    let _ = app.emit(
        "media-job",
        MediaJobEvent {
            job_id: job_id.clone(),
            state: MediaJobState::Started,
            progress: Some(0.0),
            message: None,
            error: None,
        },
    );

    let app_clone = app.clone();
    let job_id_clone = job_id.clone();
    let audio_path_clone = audio_path.clone();

    tokio::spawn(async move {
        let mut intervals = Vec::new();
        let mut full_output = String::new();

        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line_bytes) | CommandEvent::Stderr(line_bytes) => {
                    let line = String::from_utf8_lossy(&line_bytes);
                    full_output.push_str(&line);
                    full_output.push('\n');
                }
                CommandEvent::Terminated(payload) => {
                    let job_manager = app_clone.state::<JobManager>();
                    job_manager.remove_job(&job_id_clone).await;

                    // Ensure cleanup happens regardless of success/failure
                    let _ = AudioExtractionService::cleanup_stt_audio(&app_clone, audio_path_clone.clone());

                    if payload.code == Some(0) {
                        VadDetectionService::parse_vad_output(&full_output, &mut intervals);
                        let non_speech = VadDetectionService::invert_speech_intervals(&intervals, duration_ms);

                        let result = VadAnalysisResult {
                            provider: "whisper.cpp/silero-vad".to_string(),
                            version: "v5.1.2".to_string(),
                            speech_intervals: intervals.clone(),
                            non_speech_intervals: non_speech,
                            config_used: config.clone(),
                        };

                        let result_json = serde_json::to_string(&result).unwrap_or_default();

                        let _ = app_clone.emit(
                            "media-job",
                            MediaJobEvent {
                                job_id: job_id_clone.clone(),
                                state: MediaJobState::Completed,
                                progress: Some(1.0),
                                message: Some(result_json),
                                error: None,
                            },
                        );
                    } else if payload.code == Some(255) || payload.code.is_none() {
                        let _ = app_clone.emit(
                            "media-job",
                            MediaJobEvent {
                                job_id: job_id_clone.clone(),
                                state: MediaJobState::Cancelled,
                                progress: None,
                                message: Some("Job cancelled by user".to_string()),
                                error: None,
                            },
                        );
                    } else {
                        let _ = app_clone.emit(
                            "media-job",
                            MediaJobEvent {
                                job_id: job_id_clone.clone(),
                                state: MediaJobState::Failed,
                                progress: None,
                                message: None,
                                error: Some(format!("Process exited with code {:?}", payload.code)),
                            },
                        );
                    }
                }
                _ => {}
            }
        }
    });

    Ok(())
}
