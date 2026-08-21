use super::edit_plan::EditAction;
use super::fusion::DetectorEvidence;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CutSuggestion {
    pub action: EditAction,
    pub evidence: DetectorEvidence,
    pub source_version: String,
}
