use tauri::AppHandle;
use crate::models::project::Transcript;
use crate::services::whisper_service::WhisperService;
use crate::services::transcript_parser::TranscriptParser;

#[tauri::command]
pub async fn transcribe_audio(
    app: AppHandle,
    source_id: String,
    audio_path: String,
    model_path: String,
    language: String,
) -> Result<Transcript, String> {
    let whisper_result = WhisperService::transcribe(&app, &audio_path, &model_path, &language)
        .await
        .map_err(|e| e.to_string())?;

    let model_id = std::path::Path::new(&model_path)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    TranscriptParser::parse(whisper_result, &source_id, &model_id, &language)
        .map_err(|e| e.to_string())
}
