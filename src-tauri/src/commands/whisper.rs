use crate::models::project::Transcript;
use crate::models::whisper::WhisperRuntimeInfo;
use crate::services::audio_extraction_service::AudioExtractionService;
use crate::services::project_repository::{persist_transcript, ProjectSaveCoordinator};
use crate::services::resource_manager::{ResourceJobManager, ResourceManager};
use crate::services::transcript_parser::TranscriptParser;
use crate::services::whisper_service::WhisperService;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn check_whisper_runtime(app: AppHandle) -> Result<WhisperRuntimeInfo, String> {
    WhisperService::check_runtime(&app)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn transcribe_audio(
    app: AppHandle,
    source_id: String,
    audio_path: String,
    model_id: String,
    language: String,
    project_path: Option<String>,
    replace_existing: Option<bool>,
    force_replace_modified: Option<bool>,
    jobs: State<'_, ResourceJobManager>,
    coordinator: State<'_, ProjectSaveCoordinator>,
) -> Result<Transcript, String> {
    jobs.acquire_model(&model_id).await;

    let result = async {
        let model_path = ResourceManager::resolve_model_path(&app, &model_id)
            .map_err(|error| error.to_string())?;
        let whisper_result =
            WhisperService::transcribe(&app, &audio_path, &model_path.to_string_lossy(), &language)
                .await
                .map_err(|e| e.to_string())?;

        let transcript = TranscriptParser::parse(whisper_result, &source_id, &model_id, &language)
            .map_err(|e| e.to_string())?;

        if let Some(project_path) = project_path.as_deref() {
            let _save_guard = coordinator.0.lock().await;
            persist_transcript(
                project_path,
                transcript.clone(),
                replace_existing.unwrap_or(false),
                force_replace_modified.unwrap_or(false),
            )
            .map_err(|error| error.to_string())?;
        }

        Ok(transcript)
    }
    .await;

    // Extracted STT WAVs are app-owned and must not survive the transcription
    // terminal state. External audio fixtures are left untouched because the
    // cleanup service verifies canonical containment before deleting anything.
    let _ = AudioExtractionService::cleanup_stt_audio(&app, audio_path);
    jobs.release_model(&model_id).await;
    result
}
