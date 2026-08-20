use crate::models::project::Project;
use crate::services::project_migration::{load_project_from_json, MigrationError};
use std::fs;
use std::path::Path;
use tempfile::NamedTempFile;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct ProjectSaveCoordinator(pub Mutex<()>);

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
    #[error("Project recovery failed. Primary error: {primary}; backup error: {backup}")]
    RecoveryFailed { primary: String, backup: String },
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
    let mut temp_file = NamedTempFile::new_in(parent)?;

    // Write and sync the complete new project before replacing the current one.
    serde_json::to_writer_pretty(temp_file.as_file_mut(), project)?;
    temp_file.as_file_mut().sync_all()?;

    // Keep one last-known-good copy for recovery if a later write is damaged.
    if path.exists() && existing_file_is_recoverable(path) {
        fs::copy(path, backup_path(path))?;
    }

    // Atomically persist to the target path
    temp_file.persist(path).map_err(|e| e.error)?;

    Ok(())
}

fn existing_file_is_recoverable(path: &Path) -> bool {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .is_some()
}

/// Load a project from a given path.
pub fn load_project<P: AsRef<Path>>(path: P) -> Result<Project, RepositoryError> {
    let path = path.as_ref();
    match load_project_file(path) {
        Ok(project) => Ok(project),
        Err(primary_error) => {
            let backup = backup_path(path);
            if !backup.exists() {
                return Err(primary_error);
            }

            load_project_file(&backup).map_err(|backup_error| RepositoryError::RecoveryFailed {
                primary: primary_error.to_string(),
                backup: backup_error.to_string(),
            })
        }
    }
}

fn load_project_file(path: &Path) -> Result<Project, RepositoryError> {
    let content = fs::read_to_string(path)?;
    Ok(load_project_from_json(&content)?)
}

fn backup_path(path: &Path) -> std::path::PathBuf {
    let mut backup = path.to_path_buf();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("cutcut");
    backup.set_extension(format!("{extension}.bak"));
    backup
}

#[cfg(test)]
mod tests {
    use super::{backup_path, load_project, save_project};
    use crate::models::project::Project;
    use std::fs;

    #[test]
    fn saves_and_reopens_a_project() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("project.cutcut");
        let project = Project::default();

        save_project(&path, &project).unwrap();
        let reopened = load_project(&path).unwrap();

        assert_eq!(reopened.id, project.id);
        assert_eq!(reopened.schema_version, project.schema_version);
    }

    #[test]
    fn recovers_from_last_known_good_backup() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("project.cutcut");
        let first = Project::default();
        let second = Project::default();

        save_project(&path, &first).unwrap();
        save_project(&path, &second).unwrap();
        fs::write(&path, "not valid json").unwrap();

        let recovered = load_project(&path).unwrap();

        assert_eq!(recovered.id, first.id);
        assert!(backup_path(&path).exists());

        save_project(&path, &recovered).unwrap();
        let backup = load_project(backup_path(&path)).unwrap();
        assert_eq!(backup.id, first.id);
    }
}
