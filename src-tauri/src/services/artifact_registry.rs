use crate::models::artifact::get_content_fingerprint;
use crate::models::artifact_registry::{ArtifactDiagnosticReason, ArtifactRecord, ArtifactStatus};
use crate::models::project::Project;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ArtifactRegistryService;

#[derive(Debug, thiserror::Error)]
pub enum ArtifactRegistryError {
    #[error("artifact path must be relative to the project root")]
    InvalidPath,
    #[error("artifact output does not exist: {0}")]
    MissingOutput(String),
    #[error("artifact integrity mismatch for {0}")]
    IntegrityMismatch(String),
    #[error("failed to fingerprint artifact: {0}")]
    Fingerprint(#[from] std::io::Error),
}

impl ArtifactRegistryService {
    /// Register metadata only. A producer should use `register_completed` after
    /// the output is fully written so partial/cancelled jobs cannot be valid.
    pub fn register(project: &mut Project, mut record: ArtifactRecord) {
        if record.status == ArtifactStatus::Valid && record.integrity.is_none() {
            record.status = ArtifactStatus::Building;
            record.diagnostic_reason = Some(ArtifactDiagnosticReason::RegistrationFailed);
        } else if record.status == ArtifactStatus::Valid {
            record.diagnostic_reason = None;
        }
        if let Some(existing) = project.artifacts.iter_mut().find(|a| a.id == record.id) {
            *existing = record;
        } else {
            project.artifacts.push(record);
        }
    }

    /// Atomically promote a completed, app-owned artifact to `Valid` and store
    /// its content fingerprint. The project metadata is mutated only after the
    /// output and integrity checks have succeeded.
    pub fn register_completed<P: AsRef<Path>>(
        project: &mut Project,
        mut record: ArtifactRecord,
        project_root: P,
    ) -> std::result::Result<(), ArtifactRegistryError> {
        let path = safe_artifact_path(project_root.as_ref(), &record.relative_path)
            .ok_or(ArtifactRegistryError::InvalidPath)?;
        if !path.is_file() {
            record.status = ArtifactStatus::Failed;
            record.diagnostic_reason = Some(ArtifactDiagnosticReason::RegistrationFailed);
            Self::register(project, record.clone());
            return Err(ArtifactRegistryError::MissingOutput(
                record.relative_path.clone(),
            ));
        }

        let actual_integrity = get_content_fingerprint(&path)?;
        if let Some(expected_integrity) = &record.integrity {
            if expected_integrity != &actual_integrity {
                record.status = ArtifactStatus::Failed;
                record.diagnostic_reason = Some(ArtifactDiagnosticReason::IntegrityMismatch);
                Self::register(project, record.clone());
                return Err(ArtifactRegistryError::IntegrityMismatch(
                    record.relative_path.clone(),
                ));
            }
        }

        record.integrity = Some(actual_integrity);
        record.status = ArtifactStatus::Valid;
        record.diagnostic_reason = None;
        Self::register(project, record);
        Ok(())
    }

    /// Resolve only a reusable artifact: signature and status must match, the
    /// path must remain inside the project root, and integrity is verified when
    /// the record has one. Missing/corrupt files are marked without crashing.
    pub fn resolve<P: AsRef<Path>>(
        project: &mut Project,
        expected_signature: &str,
        project_root: P,
    ) -> Option<ArtifactRecord> {
        let target_index = project
            .artifacts
            .iter()
            .position(|artifact| artifact.signature == expected_signature);

        let index = target_index?;

        let record = &mut project.artifacts[index];
        if record.status != ArtifactStatus::Valid {
            return None;
        }

        let Some(path) = safe_artifact_path(project_root.as_ref(), &record.relative_path) else {
            record.status = ArtifactStatus::Stale;
            record.diagnostic_reason = Some(ArtifactDiagnosticReason::InvalidPath);
            return None;
        };

        if !path.is_file() {
            record.status = ArtifactStatus::Missing;
            record.diagnostic_reason = Some(ArtifactDiagnosticReason::FileMissing);
            return None;
        }

        if let Some(expected_integrity) = &record.integrity {
            match get_content_fingerprint(&path) {
                Ok(actual_integrity) if actual_integrity == *expected_integrity => {}
                Ok(_) => {
                    record.status = ArtifactStatus::Stale;
                    record.diagnostic_reason = Some(ArtifactDiagnosticReason::IntegrityMismatch);
                    return None;
                }
                Err(_) => {
                    record.status = ArtifactStatus::Missing;
                    record.diagnostic_reason = Some(ArtifactDiagnosticReason::FileMissing);
                    return None;
                }
            }
        }

        Some(record.clone())
    }

    /// Invalidate dependency descendants until the graph reaches a fixed point.
    /// The trigger may be a source signature or a completed artifact signature.
    pub fn invalidate(project: &mut Project, trigger_signature: &str) {
        let mut changed_signatures = HashSet::from([trigger_signature.to_string()]);

        loop {
            let mut discovered = false;
            for artifact in &mut project.artifacts {
                let depends_on_changed = artifact
                    .dependencies
                    .iter()
                    .any(|dependency| changed_signatures.contains(dependency));
                if !depends_on_changed {
                    continue;
                }

                if artifact.status != ArtifactStatus::Missing {
                    artifact.status = ArtifactStatus::Stale;
                    artifact.diagnostic_reason = Some(ArtifactDiagnosticReason::DependencyChanged);
                }
                discovered |= changed_signatures.insert(artifact.signature.clone());
            }

            if !discovered {
                break;
            }
        }
    }

    /// Remove only registry metadata. Physical cleanup is a separate, explicit
    /// cache operation and never touches user-owned source media.
    pub fn remove(project: &mut Project, artifact_id: &str) {
        project
            .artifacts
            .retain(|artifact| artifact.id != artifact_id);
    }
}

fn safe_artifact_path(project_root: &Path, relative_path: &str) -> Option<PathBuf> {
    let relative = Path::new(relative_path);
    if relative.is_absolute() {
        return None;
    }

    if relative.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    }) {
        return None;
    }

    let root = fs::canonicalize(project_root).ok()?;
    let candidate = root.join(relative);
    let mut existing_ancestor = candidate.as_path();
    while !existing_ancestor.exists() {
        existing_ancestor = existing_ancestor.parent()?;
    }

    let canonical_ancestor = fs::canonicalize(existing_ancestor).ok()?;
    if !canonical_ancestor.starts_with(&root) {
        return None;
    }

    if candidate.exists() {
        let canonical_candidate = fs::canonicalize(&candidate).ok()?;
        return canonical_candidate
            .starts_with(&root)
            .then_some(canonical_candidate);
    }

    Some(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::artifact::get_content_fingerprint;
    use crate::models::artifact::ArtifactType;
    use crate::models::artifact_registry::ArtifactDiagnosticReason;
    use crate::models::project::Project;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    fn record(
        id: &str,
        signature: &str,
        relative_path: &str,
        dependencies: &[&str],
    ) -> ArtifactRecord {
        ArtifactRecord {
            id: id.to_string(),
            artifact_type: ArtifactType::Transcript,
            signature: signature.to_string(),
            relative_path: relative_path.to_string(),
            created_at: 1000,
            artifact_version: 1,
            producer: "test".to_string(),
            status: ArtifactStatus::Valid,
            dependencies: dependencies.iter().map(|value| value.to_string()).collect(),
            integrity: None,
            diagnostic_reason: None,
        }
    }

    #[test]
    fn completed_registration_and_resolve_verify_integrity() {
        let directory = tempdir().unwrap();
        let artifact_path = directory.path().join(".cutcut/artifacts/transcript.json");
        fs::create_dir_all(artifact_path.parent().unwrap()).unwrap();
        fs::write(&artifact_path, b"transcript").unwrap();

        let relative = ".cutcut/artifacts/transcript.json";
        let mut project = Project::default();
        ArtifactRegistryService::register_completed(
            &mut project,
            record("transcript", "sig-transcript", relative, &["audio-v1"]),
            directory.path(),
        )
        .unwrap();

        assert!(
            ArtifactRegistryService::resolve(&mut project, "sig-transcript", directory.path())
                .is_some()
        );

        fs::write(&artifact_path, b"tampered").unwrap();
        assert!(
            ArtifactRegistryService::resolve(&mut project, "sig-transcript", directory.path())
                .is_none()
        );
        assert_eq!(
            project.artifacts[0].diagnostic_reason,
            Some(ArtifactDiagnosticReason::IntegrityMismatch)
        );
    }

    #[test]
    fn invalidation_cascades_through_transcript_and_preview() {
        let mut project = Project::default();
        project
            .artifacts
            .push(record("transcript", "sig-transcript", "t", &["audio-v1"]));
        project
            .artifacts
            .push(record("preview", "sig-preview", "p", &["sig-transcript"]));

        ArtifactRegistryService::invalidate(&mut project, "audio-v1");

        assert_eq!(project.artifacts[0].status, ArtifactStatus::Stale);
        assert_eq!(project.artifacts[1].status, ArtifactStatus::Stale);
        assert_eq!(
            project.artifacts[1].diagnostic_reason,
            Some(ArtifactDiagnosticReason::DependencyChanged)
        );
    }

    #[test]
    fn missing_file_is_marked_without_crashing_and_absolute_paths_are_rejected() {
        let directory = tempdir().unwrap();
        let mut project = Project::default();
        project.artifacts.push(record(
            "missing",
            "sig-missing",
            ".cutcut/artifacts/missing.json",
            &[],
        ));

        assert!(
            ArtifactRegistryService::resolve(&mut project, "sig-missing", directory.path())
                .is_none()
        );
        assert_eq!(project.artifacts[0].status, ArtifactStatus::Missing);
        assert_eq!(
            project.artifacts[0].diagnostic_reason,
            Some(ArtifactDiagnosticReason::FileMissing)
        );

        project.artifacts[0] = record("outside", "sig-outside", "C:/source.mp4", &[]);
        assert!(
            ArtifactRegistryService::resolve(&mut project, "sig-outside", directory.path())
                .is_none()
        );
        assert_eq!(
            project.artifacts[0].diagnostic_reason,
            Some(ArtifactDiagnosticReason::InvalidPath)
        );
    }

    #[test]
    fn legacy_artifact_fields_have_safe_defaults() {
        let raw = r#"{
            "id":"legacy",
            "artifactType":"transcript",
            "signature":"sig",
            "relativePath":".cutcut/artifacts/t.json",
            "createdAt":1,
            "status":"valid"
        }"#;
        let parsed: ArtifactRecord = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.artifact_version, 1);
        assert_eq!(parsed.producer, "legacy");
        assert!(parsed.dependencies.is_empty());
        assert!(parsed.integrity.is_none());
    }

    #[test]
    fn registry_record_round_trips_through_project_json() {
        let mut project = Project::default();
        let mut saved = record(
            "caption",
            "sig-caption",
            ".cutcut/artifacts/caption.ass",
            &["sig-transcript"],
        );
        saved.artifact_type = ArtifactType::Caption;
        saved.artifact_version = 3;
        saved.producer = "caption-renderer".to_string();
        saved.integrity = Some("sha256:abc".to_string());
        project.artifacts.push(saved.clone());

        let encoded = serde_json::to_string(&project).unwrap();
        let decoded: Project = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.artifacts, vec![saved]);
    }

    #[test]
    fn completed_registration_does_not_promote_missing_or_wrong_integrity() {
        let directory = tempdir().unwrap();
        let relative = ".cutcut/artifacts/audio.wav";
        let path = directory.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(b"audio").unwrap();

        let mut project = Project::default();
        let mut wrong = record("audio", "sig-audio", relative, &[]);
        wrong.integrity = Some("wrong".to_string());
        assert!(
            ArtifactRegistryService::register_completed(&mut project, wrong, directory.path())
                .is_err()
        );
        assert_eq!(project.artifacts[0].status, ArtifactStatus::Failed);

        let expected = get_content_fingerprint(&path).unwrap();
        let mut correct = record("audio", "sig-audio", relative, &[]);
        correct.integrity = Some(expected);
        ArtifactRegistryService::register_completed(&mut project, correct, directory.path())
            .unwrap();
        assert_eq!(project.artifacts[0].status, ArtifactStatus::Valid);
    }

    #[test]
    fn metadata_only_registration_cannot_mark_unverified_output_valid() {
        let mut project = Project::default();
        ArtifactRegistryService::register(
            &mut project,
            record("partial", "sig-partial", ".cutcut/artifacts/partial", &[]),
        );

        assert_eq!(project.artifacts[0].status, ArtifactStatus::Building);
        assert_eq!(
            project.artifacts[0].diagnostic_reason,
            Some(ArtifactDiagnosticReason::RegistrationFailed)
        );
    }
}
