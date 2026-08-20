use crate::models::project::{Project, CURRENT_SCHEMA_VERSION};
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

    if schema_version < CURRENT_SCHEMA_VERSION {
        // Here we will do sequential migrations when we have V2, V3.
        // e.g., if schema_version == 1 { raw_val = migrate_1_to_2(raw_val); schema_version = 2; }
    }

    // Now it should match CURRENT_SCHEMA_VERSION
    let project: Project = serde_json::from_value(raw_val)?;

    Ok(project)
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
        // We will create a minimal V1 schema
        let project = Project::default();
        let json = serde_json::to_string(&project).unwrap();

        let loaded = load_project_from_json(&json).unwrap();
        assert_eq!(loaded.id, project.id);
        assert_eq!(loaded.schema_version, 1);
    }
}
