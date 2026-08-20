use super::artifact_registry::ArtifactRecord;
use super::edit_plan::EditPlan;
use super::media_info::MediaSourceMetadata;
use serde::{Deserialize, Serialize};

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Canonical portable editing state. Source media is referenced by path only;
/// the project file never contains media bytes or credentials.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub schema_version: u32,
    pub created_at: u64,
    pub updated_at: u64,
    pub media: Vec<MediaSource>,
    pub transcript: Option<Transcript>,
    pub edit_plan: EditPlan,
    pub captions: Option<CaptionSettings>,
    pub settings: OutputSettings,
    pub artifacts: Vec<ArtifactRecord>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MediaSource {
    pub id: String,
    pub path: String,
    pub metadata: MediaSourceMetadata,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    pub id: String,
    pub source_id: String,
    pub model_id: String,
    pub language: String,
    pub generated_at: u64,
    pub segments: Vec<TranscriptSegment>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub id: String,
    pub text: String,
    pub original_text: Option<String>,
    pub start_ms: u64,
    pub end_ms: u64,
    pub speaker: Option<String>,
    pub is_filler: bool,
    #[serde(default)]
    pub is_modified: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CaptionSettings {
    pub style: String,
    pub font_size: u32,
    pub primary_color: String,
    pub stroke_color: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OutputSettings {
    pub aspect_ratio: String,
    pub target_resolution: u32,
    pub fps: f64,
}

impl Default for Project {
    fn default() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            schema_version: CURRENT_SCHEMA_VERSION,
            created_at: now,
            updated_at: now,
            media: Vec::new(),
            transcript: None,
            edit_plan: EditPlan::default(),
            captions: None,
            settings: OutputSettings {
                aspect_ratio: "16:9".to_string(),
                target_resolution: 1080,
                fps: 30.0,
            },
            artifacts: Vec::new(),
        }
    }
}
