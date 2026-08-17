use crate::models::project::Project;
use crate::services::project_migration::{load_project_from_json, MigrationError};
use std::fs;
use std::path::Path;
use tempfile::NamedTempFile;

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Migration error: {0}")]
    Migration(#[from] MigrationError),
    #[error("Cannot determine parent directory of path")]
    InvalidPath,
}

/// Save a project atomically to the given path.
pub fn save_project<P: AsRef<Path>>(path: P, project: &Project) -> Result<(), RepositoryError> {
    let path = path.as_ref();
    let parent = path.parent().ok_or(RepositoryError::InvalidPath)?;

    // Ensure the directory exists
    if !parent.exists() {
        fs::create_dir_all(parent)?;
    }

    // Create a temporary file in the same directory to ensure they are on the same filesystem
    // which makes the `persist` (rename) atomic.
    let temp_file = NamedTempFile::new_in(parent)?;
    
    // Write JSON to the temp file
    serde_json::to_writer_pretty(&temp_file, project)?;

    // Atomically persist to the target path
    temp_file.persist(path).map_err(|e| e.error)?;

    Ok(())
}

/// Load a project from a given path.
pub fn load_project<P: AsRef<Path>>(path: P) -> Result<Project, RepositoryError> {
    let path = path.as_ref();
    let content = fs::read_to_string(path)?;
    let project = load_project_from_json(&content)?;
    Ok(project)
}
