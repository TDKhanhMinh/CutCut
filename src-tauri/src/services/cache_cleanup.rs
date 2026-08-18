use crate::models::project::Project;
use crate::models::artifact::ArtifactType;
use crate::models::artifact_registry::ArtifactStatus;
use std::path::Path;
use std::fs;
use anyhow::{Result, bail};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheRetentionClass {
    UserOwned,
    Persistent,
    Recomputable,
    Temporary,
}

pub fn get_retention_class(artifact_type: &ArtifactType) -> CacheRetentionClass {
    match artifact_type {
        ArtifactType::Transcript 
        | ArtifactType::SilenceAnalysis 
        | ArtifactType::Preview 
        | ArtifactType::Caption 
        | ArtifactType::ExtractedAudio => CacheRetentionClass::Recomputable,
    }
}

pub struct CacheCleanupService;

impl CacheCleanupService {
    /// Tính tổng dung lượng có thể giải phóng từ các Recomputable artifacts.
    pub fn calculate_reclaimable_size<P: AsRef<Path>>(project: &Project, project_root: P) -> u64 {
        let mut total_size = 0;
        let root = project_root.as_ref();
        
        for artifact in &project.artifacts {
            if get_retention_class(&artifact.artifact_type) == CacheRetentionClass::Recomputable {
                let abs_path = root.join(&artifact.relative_path);
                if abs_path.exists() {
                    if let Ok(metadata) = fs::metadata(&abs_path) {
                        total_size += metadata.len();
                    }
                }
            }
        }
        total_size
    }

    /// Xóa vật lý các Recomputable artifacts và cập nhật Registry thành Missing.
    pub fn clear_recomputable_cache<P: AsRef<Path>>(project: &mut Project, project_root: P) -> Result<u64> {
        let mut freed_size = 0;
        let root = project_root.as_ref();
        
        for artifact in &mut project.artifacts {
            if get_retention_class(&artifact.artifact_type) == CacheRetentionClass::Recomputable {
                let abs_path = root.join(&artifact.relative_path);
                if abs_path.exists() {
                    if let Ok(metadata) = fs::metadata(&abs_path) {
                        if fs::remove_file(&abs_path).is_ok() {
                            freed_size += metadata.len();
                            artifact.status = ArtifactStatus::Missing;
                        }
                    }
                }
            }
        }
        Ok(freed_size)
    }

    /// Dọn dẹp rác mồ côi (file không có trong Registry) nằm trong thư mục .cutcut/artifacts
    pub fn cleanup_orphans<P: AsRef<Path>>(project: &Project, project_root: P) -> Result<u64> {
        let root = project_root.as_ref();
        let artifacts_dir = root.join(".cutcut").join("artifacts");
        
        if !artifacts_dir.exists() {
            return Ok(0);
        }

        // Chống path traversal: xác nhận artifacts_dir phải nằm trong project_root
        let canon_root = match fs::canonicalize(root) {
            Ok(p) => p,
            Err(_) => bail!("Project root does not exist"),
        };
        let canon_artifacts_dir = match fs::canonicalize(&artifacts_dir) {
            Ok(p) => p,
            Err(_) => return Ok(0), // Thư mục không tồn tại thì thôi
        };

        if !canon_artifacts_dir.starts_with(&canon_root) {
            bail!("Security Violation: Artifacts directory is outside project root");
        }

        // Thu thập danh sách các path hợp lệ từ Registry
        let mut valid_paths = Vec::new();
        for artifact in &project.artifacts {
            let abs_path = root.join(&artifact.relative_path);
            if let Ok(canon_path) = fs::canonicalize(&abs_path) {
                valid_paths.push(canon_path);
            }
        }

        let mut freed_size = 0;
        if let Ok(entries) = fs::read_dir(&canon_artifacts_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(canon_path) = fs::canonicalize(&path) {
                        if !valid_paths.contains(&canon_path) {
                            if let Ok(metadata) = fs::metadata(&path) {
                                if fs::remove_file(&path).is_ok() {
                                    freed_size += metadata.len();
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(freed_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::artifact_registry::ArtifactRecord;
    use tempfile::tempdir;
    use std::io::Write;

    #[test]
    fn test_cache_cleanup_lifecycle() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        
        // Setup .cutcut/artifacts directory
        let artifacts_dir = root.join(".cutcut").join("artifacts");
        fs::create_dir_all(&artifacts_dir).unwrap();
        
        // Create 2 files: one is in registry (recomputable), one is orphan
        let registered_file = artifacts_dir.join("reg.bin");
        let orphan_file = artifacts_dir.join("orphan.bin");
        
        let mut f1 = fs::File::create(&registered_file).unwrap();
        f1.write_all(b"12345").unwrap(); // 5 bytes
        
        let mut f2 = fs::File::create(&orphan_file).unwrap();
        f2.write_all(b"1234567890").unwrap(); // 10 bytes

        let mut project = Project::default();
        let relative_path = Path::new(".cutcut").join("artifacts").join("reg.bin").to_string_lossy().to_string();
        
        project.artifacts.push(ArtifactRecord {
            id: "1".to_string(),
            artifact_type: ArtifactType::Transcript, // Recomputable
            signature: "sig1".to_string(),
            relative_path,
            created_at: 0,
            status: ArtifactStatus::Valid,
            dependencies: vec![],
        });

        // 1. Test calculate_reclaimable_size
        let reclaimable = CacheCleanupService::calculate_reclaimable_size(&project, root);
        assert_eq!(reclaimable, 5);

        // 2. Test cleanup_orphans (should delete orphan_file, leave registered_file)
        let orphan_freed = CacheCleanupService::cleanup_orphans(&project, root).unwrap();
        assert_eq!(orphan_freed, 10);
        assert!(!orphan_file.exists());
        assert!(registered_file.exists());

        // 3. Test clear_recomputable_cache (should delete registered_file and mark missing)
        let cleared = CacheCleanupService::clear_recomputable_cache(&mut project, root).unwrap();
        assert_eq!(cleared, 5);
        assert!(!registered_file.exists());
        assert_eq!(project.artifacts[0].status, ArtifactStatus::Missing);
    }
}
