use crate::models::artifact_registry::{ArtifactRecord, ArtifactStatus};
use crate::models::project::Project;
use std::path::Path;

pub struct ArtifactRegistryService;

impl ArtifactRegistryService {
    /// Đăng ký một artifact mới hoặc cập nhật một artifact cũ nếu trùng ID.
    pub fn register(project: &mut Project, record: ArtifactRecord) {
        if let Some(existing) = project.artifacts.iter_mut().find(|a| a.id == record.id) {
            *existing = record;
        } else {
            project.artifacts.push(record);
        }
    }

    /// Lấy ra Artifact còn Valid và có file thực sự trên ổ cứng.
    pub fn resolve<P: AsRef<Path>>(
        project: &mut Project,
        expected_signature: &str,
        project_root: P,
    ) -> Option<ArtifactRecord> {
        let mut target_index = None;
        for (i, artifact) in project.artifacts.iter().enumerate() {
            if artifact.signature == expected_signature {
                target_index = Some(i);
                break;
            }
        }

        if let Some(idx) = target_index {
            let artifact = &project.artifacts[idx];
            if artifact.status == ArtifactStatus::Valid {
                let absolute_path = project_root.as_ref().join(&artifact.relative_path);
                if absolute_path.exists() {
                    return Some(artifact.clone());
                } else {
                    // File vật lý đã bị xóa ngoài app
                    project.artifacts[idx].status = ArtifactStatus::Missing;
                }
            }
        }

        None
    }

    /// Invalidate các artifacts dựa vào thay đổi signature của dependency.
    pub fn invalidate(project: &mut Project, trigger_signature: &str) {
        for artifact in project.artifacts.iter_mut() {
            if artifact
                .dependencies
                .contains(&trigger_signature.to_string())
            {
                artifact.status = ArtifactStatus::Stale;
            }
        }
    }

    /// Xóa record ra khỏi registry (thường dùng khi dọn dẹp bộ nhớ đệm thủ công).
    pub fn remove(project: &mut Project, artifact_id: &str) {
        project.artifacts.retain(|a| a.id != artifact_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::artifact::ArtifactType;
    use crate::models::project::Project;
    use tempfile::NamedTempFile;

    #[test]
    fn test_artifact_registry() {
        let mut project = Project::default();
        let temp_file = NamedTempFile::new().unwrap();

        let record = ArtifactRecord {
            id: "art_1".to_string(),
            artifact_type: ArtifactType::Transcript,
            signature: "sig_abc".to_string(),
            relative_path: temp_file.path().to_string_lossy().to_string(),
            created_at: 1000,
            status: ArtifactStatus::Valid,
            dependencies: vec!["dep_xyz".to_string()],
        };

        // Test Register
        ArtifactRegistryService::register(&mut project, record.clone());
        assert_eq!(project.artifacts.len(), 1);

        // Test Resolve (File exists)
        // Here we just pass an empty path because relative_path is already absolute in this test
        let resolved = ArtifactRegistryService::resolve(&mut project, "sig_abc", Path::new(""));
        assert!(resolved.is_some());

        // Test Invalidate
        ArtifactRegistryService::invalidate(&mut project, "dep_xyz");
        assert_eq!(project.artifacts[0].status, ArtifactStatus::Stale);

        // Test Resolve (Stale)
        let resolved_stale =
            ArtifactRegistryService::resolve(&mut project, "sig_abc", Path::new(""));
        assert!(resolved_stale.is_none());

        // Reset to Valid, test Missing file
        project.artifacts[0].status = ArtifactStatus::Valid;
        drop(temp_file); // Delete the file
        let resolved_missing =
            ArtifactRegistryService::resolve(&mut project, "sig_abc", Path::new(""));
        assert!(resolved_missing.is_none());
        assert_eq!(project.artifacts[0].status, ArtifactStatus::Missing);

        // Test Remove
        ArtifactRegistryService::remove(&mut project, "art_1");
        assert_eq!(project.artifacts.len(), 0);
    }
}
