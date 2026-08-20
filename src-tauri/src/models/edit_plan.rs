use serde::{Deserialize, Serialize};

/// Canonical, non-destructive instruction consumed by preview and rendering.
/// Timestamps are always milliseconds.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditAction {
    pub id: String,
    #[serde(rename = "type")]
    pub action_type: EditActionType,
    pub source_media_id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub source: EditActionSource,
    pub reason: String,
    pub confidence: Option<f32>,
    pub enabled: bool,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EditActionType {
    Cut,
    Keep,
    Mute,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EditActionSource {
    Local,
    Ai,
    User,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditPlan {
    pub actions: Vec<EditAction>,
}

#[cfg(test)]
mod tests {
    use super::{EditAction, EditActionSource, EditActionType};

    #[test]
    fn serializes_the_canonical_action_type_field() {
        let action = EditAction {
            id: "action-1".to_string(),
            action_type: EditActionType::Cut,
            source_media_id: "media-1".to_string(),
            start_ms: 0,
            end_ms: 1_000,
            source: EditActionSource::User,
            reason: "test".to_string(),
            confidence: None,
            enabled: true,
            created_at: 1,
            updated_at: 1,
        };

        let json = serde_json::to_value(action).unwrap();
        assert_eq!(
            json.get("type").and_then(|value| value.as_str()),
            Some("cut")
        );
        assert!(json.get("actionType").is_none());
    }
}
