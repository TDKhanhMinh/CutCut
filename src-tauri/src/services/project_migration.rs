use crate::models::project::{CaptionStyle, Project, CURRENT_SCHEMA_VERSION};
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("Failed to parse project JSON: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("Unsupported schema version: {0}. Please update the app.")]
    UnsupportedNewerVersion(u32),
    #[error("Missing schemaVersion field in JSON")]
    MissingSchemaVersion,
}

pub fn load_project_from_json(json_str: &str) -> Result<Project, MigrationError> {
    let raw_val: Value = serde_json::from_str(json_str)?;

    let schema_version = raw_val
        .get("schemaVersion")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .ok_or(MigrationError::MissingSchemaVersion)?;

    if schema_version > CURRENT_SCHEMA_VERSION {
        return Err(MigrationError::UnsupportedNewerVersion(schema_version));
    }

    let raw_val = if schema_version < CURRENT_SCHEMA_VERSION {
        migrate_to_current(raw_val, schema_version)
    } else {
        raw_val
    };

    // Now it should match CURRENT_SCHEMA_VERSION
    let project: Project = serde_json::from_value(raw_val)?;

    Ok(project)
}

fn migrate_to_current(mut value: Value, from_version: u32) -> Value {
    let Some(object) = value.as_object_mut() else {
        return value;
    };

    if from_version < 2 {
        object
            .entry("captionCues".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        object
            .entry("silenceSettings".to_string())
            .or_insert(Value::Null);

        if let Some(legacy) = object.get("captions").cloned() {
            if let Some(legacy_object) = legacy.as_object() {
                if legacy_object.get("fontSize").is_some() {
                    let style = CaptionStyle {
                        preset_id: legacy_object
                            .get("style")
                            .and_then(Value::as_str)
                            .unwrap_or("default_16_9")
                            .to_string(),
                        font_family: "Arial".to_string(),
                        font_weight: 700,
                        font_style: "normal".to_string(),
                        font_size_vh: 0.06,
                        position_x_vw: 0.5,
                        position_y_vh: 0.85,
                        alignment: "center".to_string(),
                        primary_color: legacy_object
                            .get("primaryColor")
                            .and_then(Value::as_str)
                            .unwrap_or("#FFFFFF")
                            .to_string(),
                        outline_color: legacy_object
                            .get("strokeColor")
                            .and_then(Value::as_str)
                            .map(ToString::to_string),
                        outline_width_vh: Some(0.005),
                        background_color: None,
                        background_opacity: None,
                    };
                    if let Ok(value) = serde_json::to_value(style) {
                        object.insert("captions".to_string(), value);
                    }
                }
            }
        }

        object.insert(
            "schemaVersion".to_string(),
            Value::Number(CURRENT_SCHEMA_VERSION.into()),
        );
    }

    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejects_newer_version() {
        let json = r#"{ "schemaVersion": 999, "id": "123" }"#;
        let result = load_project_from_json(json);
        assert!(matches!(
            result,
            Err(MigrationError::UnsupportedNewerVersion(999))
        ));
    }

    #[test]
    fn test_missing_schema_version() {
        let json = r#"{ "id": "123" }"#;
        let result = load_project_from_json(json);
        assert!(matches!(result, Err(MigrationError::MissingSchemaVersion)));
    }

    #[test]
    fn test_valid_v1_schema() {
        let project = Project::default();
        let mut value = serde_json::to_value(&project).unwrap();
        let object = value.as_object_mut().unwrap();
        object.insert("schemaVersion".to_string(), 1.into());
        object.remove("captionCues");
        object.remove("silenceSettings");
        object.insert(
            "captions".to_string(),
            serde_json::json!({
                "style": "default_16_9",
                "fontSize": 64,
                "primaryColor": "#00ff00",
                "strokeColor": "#111111"
            }),
        );
        let json = serde_json::to_string(&value).unwrap();

        let loaded = load_project_from_json(&json).unwrap();
        assert_eq!(loaded.id, project.id);
        assert_eq!(loaded.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(loaded.caption_cues.is_empty());
        assert!(loaded.silence_settings.is_none());
        let captions = loaded.captions.expect("legacy caption settings migrate");
        assert_eq!(captions.primary_color, "#00ff00");
        assert_eq!(captions.font_family, "Arial");
    }

    #[test]
    fn legacy_project_without_artifact_registry_loads_with_empty_registry() {
        let project = Project::default();
        let mut value = serde_json::to_value(&project).unwrap();
        value.as_object_mut().unwrap().remove("artifacts");

        let loaded = load_project_from_json(&serde_json::to_string(&value).unwrap()).unwrap();
        assert!(loaded.artifacts.is_empty());
    }
}
