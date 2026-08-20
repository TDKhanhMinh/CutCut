use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PresetType {
    Fast,
    Balanced,
    Accurate,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetResolution {
    pub preset: PresetType,
    pub target_model_id: String,
    pub target_backend: String,
    pub is_model_installed: bool,
    pub fallback_reason: Option<String>,
    pub tradeoff_description: String,
}
