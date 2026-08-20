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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactDiagnosticReason {
    DependencyChanged,
    FileMissing,
    IntegrityMismatch,
    InvalidPath,
    RegistrationFailed,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRecord {
    pub id: String,
    pub artifact_type: ArtifactType,
    pub signature: String,
    pub relative_path: String,
    pub created_at: u64,
    #[serde(default = "default_artifact_version")]
    pub artifact_version: u32,
    #[serde(default = "default_producer")]
    pub producer: String,
    pub status: ArtifactStatus,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub integrity: Option<String>,
    #[serde(default)]
    pub diagnostic_reason: Option<ArtifactDiagnosticReason>,
}

fn default_artifact_version() -> u32 {
    1
}

fn default_producer() -> String {
    "legacy".to_string()
}
