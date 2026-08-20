use crate::models::artifact::ArtifactType;
use crate::models::artifact_registry::{ArtifactDiagnosticReason, ArtifactStatus};
use crate::models::project::Project;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheRetentionClass {
    UserOwned,
    Persistent,
    Recomputable,
    Temporary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheRetentionPolicy {
    pub class: CacheRetentionClass,
    /// `None` means retention is cost-driven and not age-evicted automatically.
    pub ttl_days: Option<u32>,
}

/// Generated artifacts are disposable; Project JSON, source media, and user
/// assets are deliberately outside this registry and are never cleanup targets.
pub fn get_retention_policy(artifact_type: &ArtifactType) -> CacheRetentionPolicy {
    match artifact_type {
        ArtifactType::Transcript => CacheRetentionPolicy {
            class: CacheRetentionClass::Recomputable,
            ttl_days: Some(30),
        },
        ArtifactType::ExtractedAudio => CacheRetentionPolicy {
            class: CacheRetentionClass::Temporary,
            ttl_days: Some(1),
        },
        ArtifactType::MediaMetadata
        | ArtifactType::SilenceAnalysis
        | ArtifactType::LocalAnalysis
        | ArtifactType::AiAnalysis
        | ArtifactType::Caption
        | ArtifactType::Preview => CacheRetentionPolicy {
            class: CacheRetentionClass::Recomputable,
            ttl_days: Some(14),
        },
    }
}

pub fn get_retention_class(artifact_type: &ArtifactType) -> CacheRetentionClass {
    get_retention_policy(artifact_type).class
}

pub struct CacheCleanupService;

impl CacheCleanupService {
    /// Tính tổng dung lượng có thể giải phóng chỉ trong app-owned artifacts root.
    pub fn calculate_reclaimable_size<P: AsRef<Path>>(project: &Project, project_root: P) -> u64 {
        let Ok(Some(artifacts_dir)) = managed_artifacts_dir(project_root.as_ref()) else {
            return 0;
        };
        let mut total_size = 0;

        for artifact in &project.artifacts {
            if get_retention_class(&artifact.artifact_type) != CacheRetentionClass::Recomputable
                && get_retention_class(&artifact.artifact_type) != CacheRetentionClass::Temporary
            {
                continue;
            }
            let Some(path) = safe_managed_file(&artifacts_dir, &artifact.relative_path) else {
                continue;
            };
            if let Ok(metadata) = fs::metadata(path) {
                if metadata.is_file() {
                    total_size += metadata.len();
                }
            }
        }
        total_size
    }

    /// Xóa các artifact disposable và mark registry missing. Invalid/traversal
    /// paths are rejected and never passed to `remove_file`.
    pub fn clear_recomputable_cache<P: AsRef<Path>>(
        project: &mut Project,
        project_root: P,
    ) -> Result<u64> {
        let Some(artifacts_dir) = managed_artifacts_dir(project_root.as_ref())? else {
            return Ok(0);
        };
        let mut freed_size = 0;

        for artifact in &mut project.artifacts {
            let class = get_retention_class(&artifact.artifact_type);
            if class != CacheRetentionClass::Recomputable && class != CacheRetentionClass::Temporary
            {
                continue;
            }

            let Some(path) = safe_managed_file(&artifacts_dir, &artifact.relative_path) else {
                artifact.status = ArtifactStatus::Stale;
                artifact.diagnostic_reason = Some(ArtifactDiagnosticReason::InvalidPath);
                continue;
            };

            if !path.exists() {
                artifact.status = ArtifactStatus::Missing;
                artifact.diagnostic_reason = Some(ArtifactDiagnosticReason::FileMissing);
                continue;
            }

            if let Ok(metadata) = fs::metadata(&path) {
                if metadata.is_file() && fs::remove_file(&path).is_ok() {
                    freed_size += metadata.len();
                    artifact.status = ArtifactStatus::Missing;
                    artifact.diagnostic_reason = Some(ArtifactDiagnosticReason::FileMissing);
                }
            }
        }
        Ok(freed_size)
    }

    /// Dọn dẹp orphan chỉ bên trong `.cutcut/artifacts`; symlink và path ngoài
    /// managed root bị bỏ qua để tránh đụng user-owned data.
    pub fn cleanup_orphans<P: AsRef<Path>>(project: &Project, project_root: P) -> Result<u64> {
        let Some(artifacts_dir) = managed_artifacts_dir(project_root.as_ref())? else {
            return Ok(0);
        };

        let mut valid_paths = Vec::new();
        for artifact in &project.artifacts {
            if let Some(path) = safe_managed_file(&artifacts_dir, &artifact.relative_path) {
                if let Ok(canonical) = fs::canonicalize(path) {
                    valid_paths.push(canonical);
                }
            }
        }

        let mut freed_size = 0;
        for entry in fs::read_dir(&artifacts_dir)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if !file_type.is_file() || file_type.is_symlink() {
                continue;
            }

            let path = entry.path();
            let canonical_path = fs::canonicalize(&path)?;
            if !canonical_path.starts_with(&artifacts_dir) || valid_paths.contains(&canonical_path)
            {
                continue;
            }

            let metadata = fs::metadata(&path)?;
            if fs::remove_file(path).is_ok() {
                freed_size += metadata.len();
            }
        }

        Ok(freed_size)
    }
}

fn managed_artifacts_dir(project_root: &Path) -> Result<Option<PathBuf>> {
    let root = fs::canonicalize(project_root).context("project root does not exist")?;
    let artifacts_dir = root.join(".cutcut").join("artifacts");
    if !artifacts_dir.exists() {
        return Ok(None);
    }

    let canonical = fs::canonicalize(&artifacts_dir)?;
    if !canonical.starts_with(&root) {
        bail!("security violation: managed artifacts directory is outside project root");
    }
    Ok(Some(canonical))
}

fn safe_managed_file(artifacts_dir: &Path, relative_path: &str) -> Option<PathBuf> {
    let relative = Path::new(relative_path);
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return None;
    }

    // Registry paths are project-root-relative, so normalize the suffix after
    // `.cutcut/artifacts` and require it to land below that managed directory.
    let project_root = artifacts_dir.parent()?.parent()?;
    let candidate = project_root.join(relative);
    if !candidate.starts_with(artifacts_dir) {
        return None;
    }

    if candidate.exists() {
        let canonical = fs::canonicalize(&candidate).ok()?;
        return canonical.starts_with(artifacts_dir).then_some(canonical);
    }

    Some(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::artifact_registry::ArtifactRecord;
    use std::io::Write;
    use tempfile::tempdir;

    fn record(
        artifact_type: ArtifactType,
        relative_path: &str,
        status: ArtifactStatus,
    ) -> ArtifactRecord {
        ArtifactRecord {
            id: relative_path.to_string(),
            artifact_type,
            signature: "sig".to_string(),
            relative_path: relative_path.to_string(),
            created_at: 0,
            artifact_version: 1,
            producer: "test".to_string(),
            status,
            dependencies: vec![],
            integrity: None,
            diagnostic_reason: None,
        }
    }

    #[test]
    fn cleanup_lifecycle_reports_usage_removes_only_managed_cache_and_marks_missing() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let artifacts_dir = root.join(".cutcut").join("artifacts");
        fs::create_dir_all(&artifacts_dir).unwrap();

        let registered_file = artifacts_dir.join("transcript.json");
        let orphan_file = artifacts_dir.join("orphan.bin");
        fs::write(&registered_file, b"12345").unwrap();
        fs::write(&orphan_file, b"1234567890").unwrap();

        let relative_path = ".cutcut/artifacts/transcript.json";
        let mut project = Project::default();
        project.artifacts.push(record(
            ArtifactType::Transcript,
            relative_path,
            ArtifactStatus::Valid,
        ));

        assert_eq!(
            CacheCleanupService::calculate_reclaimable_size(&project, root),
            5
        );
        assert_eq!(
            CacheCleanupService::cleanup_orphans(&project, root).unwrap(),
            10
        );
        assert!(!orphan_file.exists());
        assert!(registered_file.exists());

        assert_eq!(
            CacheCleanupService::clear_recomputable_cache(&mut project, root).unwrap(),
            5
        );
        assert!(!registered_file.exists());
        assert_eq!(project.artifacts[0].status, ArtifactStatus::Missing);
    }

    #[test]
    fn traversal_and_user_source_are_never_deleted() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let artifacts_dir = root.join(".cutcut").join("artifacts");
        fs::create_dir_all(&artifacts_dir).unwrap();
        let source = root.join("source.mp4");
        fs::write(&source, b"source").unwrap();

        let mut project = Project::default();
        project.artifacts.push(record(
            ArtifactType::Preview,
            ".cutcut/artifacts/..\\source.mp4",
            ArtifactStatus::Valid,
        ));

        assert_eq!(
            CacheCleanupService::clear_recomputable_cache(&mut project, root).unwrap(),
            0
        );
        assert!(source.exists());
        assert_eq!(project.artifacts[0].status, ArtifactStatus::Stale);
        assert_eq!(
            project.artifacts[0].diagnostic_reason,
            Some(ArtifactDiagnosticReason::InvalidPath)
        );
    }

    #[test]
    fn policies_keep_expensive_transcript_longer_than_temporary_audio() {
        let transcript = get_retention_policy(&ArtifactType::Transcript);
        let audio = get_retention_policy(&ArtifactType::ExtractedAudio);
        assert_eq!(transcript.class, CacheRetentionClass::Recomputable);
        assert_eq!(transcript.ttl_days, Some(30));
        assert_eq!(audio.class, CacheRetentionClass::Temporary);
        assert_eq!(audio.ttl_days, Some(1));
    }

    #[test]
    fn orphan_cleanup_does_not_follow_symlink_entries() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let artifacts_dir = root.join(".cutcut").join("artifacts");
        fs::create_dir_all(&artifacts_dir).unwrap();
        let outside = root.join("user-logo.png");
        fs::File::create(&outside)
            .unwrap()
            .write_all(b"logo")
            .unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, artifacts_dir.join("logo-link")).unwrap();
        #[cfg(windows)]
        {
            // Symlink creation may require a developer-mode privilege on Windows;
            // the path-containment tests above still cover the deletion boundary.
            let _ = std::os::windows::fs::symlink_file(&outside, artifacts_dir.join("logo-link"));
        }

        let project = Project::default();
        let _ = CacheCleanupService::cleanup_orphans(&project, root).unwrap();
        assert!(outside.exists());
    }
}
