use tauri::command;
use crate::models::fusion::NonSpeechCandidate;
use crate::models::project::EditTimeline;
use crate::models::suggestion::CutSuggestion;
use crate::services::suggestion_service;

#[command]
pub async fn generate_cut_suggestions(
    source_media_id: String,
    candidates: Vec<NonSpeechCandidate>,
    analysis_version: String,
    existing_timeline: Option<EditTimeline>,
) -> Result<Vec<CutSuggestion>, String> {
    let suggestions = suggestion_service::generate_suggestions(
        &source_media_id,
        &candidates,
        &analysis_version,
        existing_timeline.as_ref(),
    );
    Ok(suggestions)
}
