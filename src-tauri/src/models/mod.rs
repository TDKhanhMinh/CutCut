use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectSettings {
    pub resolution: (u32, u32),
    pub fps: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub version: String,
    pub source_media_path: Option<String>,
    pub settings: ProjectSettings,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranscriptSegment {
    pub id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transcript {
    pub id: String,
    pub segments: Vec<TranscriptSegment>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ActionSource {
    Local,
    AI,
    User,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ActionType {
    Cut,
    Mute,
    Zoom(f32), // zoom level
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EditAction {
    pub id: String,
    pub action_type: ActionType,
    pub start_ms: u64,
    pub end_ms: u64,
    pub source: ActionSource,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EditPlan {
    pub actions: Vec<EditAction>,
}
