use crate::models::edit_plan::EditAction;
use crate::models::project::Transcript;
use crate::services::filler_detector::{detect_fillers, FillerCandidate, FillerDictionary};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FillerAnalysisResult {
    pub dictionary_version: String,
    pub candidates: Vec<FillerCandidate>,
    pub actions: Vec<EditAction>,
}

/// Detect conservative filler candidates from the persisted transcript. The
/// returned actions are disabled and must be explicitly reviewed before they
/// can affect the EditPlan.
#[tauri::command]
pub fn detect_filler_candidates(
    source_media_id: String,
    transcript: Transcript,
    media_duration_ms: u64,
    padding_ms: Option<u64>,
) -> Result<FillerAnalysisResult, String> {
    if source_media_id.trim().is_empty() || media_duration_ms == 0 {
        return Err("A source media id and positive media duration are required".to_string());
    }

    let dictionary = FillerDictionary::default();
    let candidates = detect_fillers(&source_media_id, &transcript.segments, &dictionary);
    let created_at = now_ms();
    let actions = candidates
        .iter()
        .filter_map(|candidate| {
            candidate.to_edit_action(created_at, padding_ms.unwrap_or(0), media_duration_ms)
        })
        .collect();

    Ok(FillerAnalysisResult {
        dictionary_version: dictionary.version,
        candidates,
        actions,
    })
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
