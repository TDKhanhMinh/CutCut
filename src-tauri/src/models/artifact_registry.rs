use super::artifact::ArtifactType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactStatus {
    Valid,
    Stale,
    Missing,
    Building,
    Failed,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRecord {
    pub id: String,
    pub artifact_type: ArtifactType,
    pub signature: String,
    pub relative_path: String,
    pub created_at: u64,
    pub status: ArtifactStatus,
    pub dependencies: Vec<String>,
}
