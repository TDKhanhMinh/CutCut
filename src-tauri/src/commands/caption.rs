use crate::models::project::{CaptionCue, Transcript};
use crate::services::caption_segmenter;
use tauri::command;

#[command]
pub async fn generate_caption_cues(
    transcript: Transcript,
    existing_cues: Vec<CaptionCue>,
) -> Result<Vec<CaptionCue>, String> {
    Ok(caption_segmenter::generate_cues(
        &transcript,
        &existing_cues,
    ))
}
