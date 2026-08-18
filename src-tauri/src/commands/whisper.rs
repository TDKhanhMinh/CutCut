use tauri::AppHandle;
use crate::models::whisper::WhisperResult;
use crate::services::whisper_service::WhisperService;

#[tauri::command]
pub async fn transcribe_audio(
    app: AppHandle,
    audio_path: String,
    model_path: String,
    language: String,
) -> Result<WhisperResult, String> {
    WhisperService::transcribe(&app, &audio_path, &model_path, &language)
        .await
        .map_err(|e| e.to_string())
}
