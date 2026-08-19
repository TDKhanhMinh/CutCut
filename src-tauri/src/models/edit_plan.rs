use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ActionSource {
    LocalDetector,
    AiAgent,
    UserManual,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ActionPayload {
    Cut {
        start_ms: u64,
        end_ms: u64,
    },
    Zoom {
        start_ms: u64,
        end_ms: u64,
        scale: f64,
        anchor_x: f64,
        anchor_y: f64,
        easing: String,
    },
    Highlight {
        start_ms: u64,
        end_ms: u64,
    },
    Caption {
        start_ms: u64,
        end_ms: u64,
        text: String,
        style_reference: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EditAction {
    pub id: String,
    pub source_media_id: String,
    pub payload: ActionPayload,
    pub source: ActionSource,
    pub reason: String,
    pub confidence: Option<String>,
    pub enabled: bool,
    pub is_manual_modified: Option<bool>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GenerationMetadata {
    pub analyzer_version: Option<String>,
    pub model_id: Option<String>,
    pub run_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EditPlan {
    pub version: u32,
    pub actions: Vec<EditAction>,
    pub generation_metadata: Option<GenerationMetadata>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_plan_serialization() {
        let plan = EditPlan {
            version: 1,
            actions: vec![
                EditAction {
                    id: "cut-1".to_string(),
                    source_media_id: "media-1".to_string(),
                    payload: ActionPayload::Cut { start_ms: 1000, end_ms: 2000 },
                    source: ActionSource::LocalDetector,
                    reason: "silence".to_string(),
                    confidence: Some("High".to_string()),
                    enabled: true,
                    is_manual_modified: None,
                    created_at: 0,
                    updated_at: 0,
                },
                EditAction {
                    id: "zoom-1".to_string(),
                    source_media_id: "media-1".to_string(),
                    payload: ActionPayload::Zoom { 
                        start_ms: 3000, end_ms: 5000, 
                        scale: 1.5, anchor_x: 0.5, anchor_y: 0.5, easing: "ease-in-out".to_string() 
                    },
                    source: ActionSource::AiAgent,
                    reason: "face_detected".to_string(),
                    confidence: None,
                    enabled: true,
                    is_manual_modified: None,
                    created_at: 0,
                    updated_at: 0,
                }
            ],
            generation_metadata: Some(GenerationMetadata {
                analyzer_version: Some("v1.0".to_string()),
                model_id: None,
                run_id: None,
            }),
        };

        let json = serde_json::to_string_pretty(&plan).unwrap();
        assert!(json.contains(r#""type": "cut""#));
        assert!(json.contains(r#""type": "zoom""#));
        assert!(json.contains(r#""source": "localDetector""#));

        let deserialized: EditPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.actions.len(), 2);
        
        match &deserialized.actions[0].payload {
            ActionPayload::Cut { start_ms, end_ms } => {
                assert_eq!(*start_ms, 1000);
                assert_eq!(*end_ms, 2000);
            },
            _ => panic!("Expected Cut payload"),
        }
    }
}
