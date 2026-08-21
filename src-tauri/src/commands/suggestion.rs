use crate::models::edit_plan::EditPlan;
use crate::models::fusion::NonSpeechCandidate;
use crate::models::suggestion::CutSuggestion;
use crate::services::suggestion_service;
use tauri::command;

#[command]
pub async fn generate_cut_suggestions(
    source_media_id: String,
    candidates: Vec<NonSpeechCandidate>,
    analysis_version: String,
    existing_plan: Option<EditPlan>,
    media_duration_ms: u64,
) -> Result<Vec<CutSuggestion>, String> {
    let suggestions = suggestion_service::generate_suggestions(
        &source_media_id,
        &candidates,
        &analysis_version,
        existing_plan.as_ref(),
        media_duration_ms,
    );
    Ok(suggestions)
}
