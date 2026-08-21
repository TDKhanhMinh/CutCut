use crate::models::project::{Project, Transcript};
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
    #[error("Project already contains a transcript; explicit replace confirmation is required")]
    TranscriptReplaceRequiresConfirmation,
    #[error("Existing transcript or edit plan has manual changes; explicit force replacement is required")]
    TranscriptManualChangesRequireForce,
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

/// Persist a parsed transcript while making re-transcription destructive only
/// after explicit confirmation. Manual transcript edits and dependent edit
/// actions require a second force flag so they cannot be silently discarded.
pub fn persist_transcript<P: AsRef<Path>>(
    path: P,
    transcript: Transcript,
    replace_existing: bool,
    force_replace_modified: bool,
) -> Result<Project, RepositoryError> {
    let path = path.as_ref();
    let mut project = load_project(path)?;
    if project.transcript.is_some() {
        if !replace_existing {
            return Err(RepositoryError::TranscriptReplaceRequiresConfirmation);
        }
        let has_manual_transcript_edits = project
            .transcript
            .as_ref()
            .is_some_and(transcript_has_manual_edits);
        if (has_manual_transcript_edits || !project.edit_plan.actions.is_empty())
            && !force_replace_modified
        {
            return Err(RepositoryError::TranscriptManualChangesRequireForce);
        }
    }

    project.transcript = Some(transcript);
    project.updated_at = now_millis();
    save_project(path, &project)?;
    Ok(project)
}

fn transcript_has_manual_edits(transcript: &Transcript) -> bool {
    transcript.segments.iter().any(|segment| {
        segment.is_modified
            || segment
                .original_text
                .as_ref()
                .is_some_and(|original| original != &segment.text)
    })
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
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
    use super::{backup_path, load_project, persist_transcript, save_project, RepositoryError};
    use crate::models::edit_plan::{EditAction, EditActionSource, EditActionType};
    use crate::models::project::{
        CaptionCue, CaptionStyle, Project, Transcript, TranscriptSegment,
    };
    use crate::models::silence::{SilenceConfig, SilencePreset};
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
    fn edit_plan_decision_round_trips_through_project_save() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("project.cutcut");
        let mut project = Project::default();
        project.edit_plan.actions.push(EditAction {
            id: "silence-1".into(),
            action_type: EditActionType::Cut,
            source_media_id: "media-1".into(),
            start_ms: 1_000,
            end_ms: 2_000,
            source: EditActionSource::Local,
            reason: "silence".into(),
            confidence: Some(0.9),
            enabled: false,
            is_manual_modified: None,
            created_at: 1,
            updated_at: 2,
            payload: None,
        });

        save_project(&path, &project).unwrap();
        let reopened = load_project(&path).unwrap();

        assert_eq!(reopened.edit_plan.actions[0].id, "silence-1");
        assert!(!reopened.edit_plan.actions[0].enabled);
        assert_eq!(reopened.edit_plan.actions[0].updated_at, 2);
    }

    #[test]
    fn round_trips_detection_and_caption_contracts() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("project.cutcut");
        let project = Project {
            silence_settings: Some(SilenceConfig {
                preset: SilencePreset::Aggressive,
                ..SilenceConfig::default()
            }),
            caption_cues: vec![CaptionCue {
                id: "cue-1".to_string(),
                source_segment_ids: vec!["segment-1".to_string()],
                start_ms: 100,
                end_ms: 800,
                text: "Xin chào".to_string(),
                is_manual_modified: true,
            }],
            captions: Some(CaptionStyle {
                primary_color: "#00FF00".to_string(),
                ..CaptionStyle::get_default_16_9_preset()
            }),
            ..Project::default()
        };

        save_project(&path, &project).unwrap();
        let reopened = load_project(&path).unwrap();

        assert_eq!(
            reopened.silence_settings.unwrap().preset,
            SilencePreset::Aggressive
        );
        assert_eq!(reopened.caption_cues[0].text, "Xin chào");
        assert_eq!(reopened.captions.unwrap().primary_color, "#00FF00");
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

    fn transcript(text: &str, is_modified: bool) -> Transcript {
        Transcript {
            id: "transcript-1".to_string(),
            source_id: "source-1".to_string(),
            model_id: "ggml-tiny".to_string(),
            language: "vi".to_string(),
            generated_at: 1,
            segments: vec![TranscriptSegment {
                id: "segment-1".to_string(),
                text: text.to_string(),
                original_text: if is_modified {
                    Some("original".to_string())
                } else {
                    None
                },
                start_ms: 0,
                end_ms: 1_000,
                speaker: None,
                is_filler: false,
                is_modified,
            }],
        }
    }

    #[test]
    fn persists_initial_transcript_and_round_trips() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("project.cutcut");
        save_project(&path, &Project::default()).unwrap();

        let persisted = persist_transcript(&path, transcript("Xin chào", false), false, false)
            .expect("persist initial transcript");
        assert_eq!(persisted.transcript.unwrap().segments[0].text, "Xin chào");
        assert!(load_project(&path).unwrap().transcript.is_some());
    }

    #[test]
    fn replacement_requires_confirmation_and_protects_manual_edits() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("project.cutcut");
        let project = Project {
            transcript: Some(transcript("edited", true)),
            ..Project::default()
        };
        save_project(&path, &project).unwrap();

        assert!(matches!(
            persist_transcript(&path, transcript("new", false), false, false),
            Err(RepositoryError::TranscriptReplaceRequiresConfirmation)
        ));
        assert!(matches!(
            persist_transcript(&path, transcript("new", false), true, false),
            Err(RepositoryError::TranscriptManualChangesRequireForce)
        ));
        let replaced = persist_transcript(&path, transcript("new", false), true, true).unwrap();
        assert_eq!(replaced.transcript.unwrap().segments[0].text, "new");
    }
}
