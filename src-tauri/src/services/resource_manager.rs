use crate::models::hardware::RuntimeProfile;
use crate::models::resource::{
    ResourceCompatibility, ResourceItem, ResourceManifest, ResourceState, ResourceType,
};
use anyhow::{bail, Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

#[derive(Default, Clone)]
pub struct ResourceJobManager {
    active: Arc<Mutex<HashSet<String>>>,
    cancelled: Arc<Mutex<HashSet<String>>>,
    in_use: Arc<Mutex<HashMap<String, usize>>>,
}

impl ResourceJobManager {
    pub async fn begin(&self, id: &str) -> bool {
        let mut active = self.active.lock().await;
        if !active.insert(id.to_string()) {
            return false;
        }
        self.cancelled.lock().await.remove(id);
        true
    }

    pub async fn finish(&self, id: &str) {
        self.active.lock().await.remove(id);
        self.cancelled.lock().await.remove(id);
    }

    pub async fn cancel(&self, id: &str) -> bool {
        let is_active = self.active.lock().await.contains(id);
        if is_active {
            self.cancelled.lock().await.insert(id.to_string());
        }
        is_active
    }

    pub async fn is_cancelled(&self, id: &str) -> bool {
        self.cancelled.lock().await.contains(id)
    }

    pub async fn is_active(&self, id: &str) -> bool {
        self.active.lock().await.contains(id)
    }

    pub async fn acquire_model(&self, id: &str) {
        let mut in_use = self.in_use.lock().await;
        *in_use.entry(id.to_string()).or_default() += 1;
    }

    pub async fn release_model(&self, id: &str) {
        let mut in_use = self.in_use.lock().await;
        if let Some(count) = in_use.get_mut(id) {
            *count -= 1;
            if *count == 0 {
                in_use.remove(id);
            }
        }
    }

    pub async fn is_model_in_use(&self, id: &str) -> bool {
        self.in_use.lock().await.contains_key(id)
    }
}

pub struct ResourceManager;

impl ResourceManager {
    pub fn get_catalog() -> Vec<ResourceItem> {
        vec![
            catalog_item(
                "ggml-tiny",
                "Fast (Tiny)",
                77_691_713,
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
                "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
                2_000,
            ),
            catalog_item(
                "ggml-base",
                "Balanced (Base)",
                147_951_465,
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
                "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
                4_000,
            ),
            catalog_item(
                "ggml-small",
                "Accurate (Small)",
                487_601_967,
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
                "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
                8_000,
            ),
            vad_catalog_item(),
        ]
    }

    pub fn get_models_dir(app: &AppHandle) -> Result<PathBuf> {
        let dir = app
            .path()
            .app_local_data_dir()
            .context("Failed to get local data dir")?
            .join("models");
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        Ok(dir)
    }

    pub fn find_catalog_item(id: &str) -> Result<ResourceItem> {
        Self::get_catalog()
            .into_iter()
            .find(|item| item.id == id)
            .with_context(|| format!("Resource `{id}` is not in the catalog"))
    }

    pub fn get_resource_state(app: &AppHandle, item: &ResourceItem) -> Result<ResourceState> {
        let profile =
            crate::services::hardware_detection::HardwareDetectionService::detect_profile();
        if let Some(reason) = compatibility_error(&item.compatibility, &profile) {
            return Ok(ResourceState::Incompatible { reason });
        }

        let (bin_path, manifest_path) = resource_paths(&Self::get_models_dir(app)?, &item.id);
        if !bin_path.is_file() || !manifest_path.is_file() {
            return Ok(ResourceState::NotInstalled);
        }

        let metadata = fs::metadata(&bin_path)?;
        if metadata.len() != item.size_bytes {
            return Ok(ResourceState::Corrupted {
                reason: "Installed size does not match the catalog.".to_string(),
            });
        }

        let manifest: ResourceManifest = match fs::read_to_string(&manifest_path)
            .ok()
            .and_then(|value| serde_json::from_str(&value).ok())
        {
            Some(manifest) => manifest,
            None => {
                return Ok(ResourceState::Corrupted {
                    reason: "Manifest is missing or invalid JSON.".to_string(),
                })
            }
        };

        if manifest.id != item.id
            || manifest.resource_type != item.resource_type
            || manifest.version != item.version
            || manifest.checksum != item.checksum
            || manifest.size_bytes != item.size_bytes
            || manifest.compatibility != item.compatibility
        {
            return Ok(ResourceState::Corrupted {
                reason: "Manifest does not match the catalog.".to_string(),
            });
        }

        let actual_checksum = sha256_file(&bin_path)?;
        if actual_checksum != item.checksum {
            return Ok(ResourceState::Corrupted {
                reason: "SHA-256 checksum does not match the catalog.".to_string(),
            });
        }

        Ok(ResourceState::Installed)
    }

    pub async fn download_resource(
        app: AppHandle,
        id: String,
        jobs: ResourceJobManager,
    ) -> Result<()> {
        let item = Self::find_catalog_item(&id)?;
        if !jobs.begin(&id).await {
            bail!("A download for `{id}` is already active");
        }

        let models_dir = Self::get_models_dir(&app)?;
        let (bin_path, manifest_path) = resource_paths(&models_dir, &item.id);
        let tmp_path = models_dir.join(format!("{}.tmp", item.id));
        let result =
            Self::download_inner(&app, &item, &tmp_path, &bin_path, &manifest_path, &jobs).await;
        jobs.finish(&id).await;

        if let Err(error) = &result {
            let _ = fs::remove_file(&tmp_path);
            if error.to_string() != "Download cancelled" {
                let _ = app.emit(
                    "resource-download-finished",
                    serde_json::json!({
                        "id": item.id,
                        "status": "failed",
                        "reason": error.to_string()
                    }),
                );
            }
        }
        result
    }

    async fn download_inner(
        app: &AppHandle,
        item: &ResourceItem,
        tmp_path: &Path,
        bin_path: &Path,
        manifest_path: &Path,
        jobs: &ResourceJobManager,
    ) -> Result<()> {
        let client = Client::new();
        let response = client.get(&item.url).send().await?;
        if !response.status().is_success() {
            bail!("Failed to download {}: {}", item.id, response.status());
        }

        let total_size = response.content_length().unwrap_or(item.size_bytes);
        let mut downloaded = 0_u64;
        let mut stream = response.bytes_stream();
        let mut output = File::create(tmp_path).await?;
        let mut hasher = Sha256::new();

        while let Some(chunk) = stream.next().await {
            if jobs.is_cancelled(&item.id).await {
                let _ = fs::remove_file(tmp_path);
                let _ = app.emit(
                    "resource-download-finished",
                    serde_json::json!({"id": item.id, "status": "cancelled"}),
                );
                bail!("Download cancelled");
            }

            let chunk = chunk?;
            output.write_all(&chunk).await?;
            hasher.update(&chunk);
            downloaded += chunk.len() as u64;
            let progress = if total_size > 0 {
                (downloaded as f64 / total_size as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let _ = app.emit(
                "resource-download-progress",
                serde_json::json!({
                    "id": item.id,
                    "progress": progress,
                    "downloaded": downloaded,
                    "total": total_size
                }),
            );
        }
        output.sync_all().await?;

        if jobs.is_cancelled(&item.id).await {
            let _ = fs::remove_file(tmp_path);
            let _ = app.emit(
                "resource-download-finished",
                serde_json::json!({"id": item.id, "status": "cancelled"}),
            );
            bail!("Download cancelled");
        }

        let checksum = format_digest(hasher.finalize());
        if checksum != item.checksum {
            bail!("Checksum mismatch for {}", item.id);
        }
        if downloaded != item.size_bytes {
            bail!(
                "Downloaded size mismatch for {}: expected {}, received {}",
                item.id,
                item.size_bytes,
                downloaded
            );
        }

        // The old file is removed only after the new bytes and checksum pass.
        // The manifest is written after rename, so a crash cannot advertise a
        // half-valid model as installed.
        let _ = fs::remove_file(bin_path);
        fs::rename(tmp_path, bin_path)?;
        write_manifest_atomic(
            manifest_path,
            &ResourceManifest {
                id: item.id.clone(),
                resource_type: item.resource_type.clone(),
                version: item.version.clone(),
                checksum: item.checksum.clone(),
                size_bytes: item.size_bytes,
                compatibility: item.compatibility.clone(),
                installed_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            },
        )?;

        let _ = app.emit(
            "resource-download-finished",
            serde_json::json!({"id": item.id, "status": "installed"}),
        );
        Ok(())
    }

    pub async fn delete_resource(
        app: &AppHandle,
        id: String,
        jobs: &ResourceJobManager,
    ) -> Result<()> {
        let item = Self::find_catalog_item(&id)?;
        if jobs.is_active(&id).await {
            bail!("Cannot delete `{id}` while its download is active");
        }
        if jobs.is_model_in_use(&id).await {
            bail!("Cannot delete `{id}` while it is used by a transcription job");
        }
        if Self::get_active_model(app)?.as_deref() == Some(item.id.as_str()) {
            bail!("Select another Whisper model before deleting the active model");
        }

        let models_dir = Self::get_models_dir(app)?;
        let (bin_path, manifest_path) = resource_paths(&models_dir, &item.id);
        let tmp_path = models_dir.join(format!("{}.tmp", item.id));
        let _ = fs::remove_file(bin_path);
        let _ = fs::remove_file(manifest_path);
        let _ = fs::remove_file(tmp_path);
        Ok(())
    }

    pub fn get_resource_usage(app: &AppHandle) -> Result<u64> {
        let models_dir = Self::get_models_dir(app)?;
        let mut total = 0;
        for item in Self::get_catalog() {
            let (bin_path, _) = resource_paths(&models_dir, &item.id);
            if let Ok(metadata) = fs::metadata(bin_path) {
                total += metadata.len();
            }
        }
        Ok(total)
    }

    pub fn get_active_model(app: &AppHandle) -> Result<Option<String>> {
        let path = Self::get_models_dir(app)?.join("active_whisper.txt");
        if !path.is_file() {
            return Ok(None);
        }
        let id = fs::read_to_string(path)?.trim().to_string();
        if id.is_empty() {
            return Ok(None);
        }
        let item = match Self::find_catalog_item(&id) {
            Ok(item) => item,
            Err(_) => return Ok(None),
        };
        if Self::get_resource_state(app, &item)? == ResourceState::Installed {
            Ok(Some(id))
        } else {
            Ok(None)
        }
    }

    pub fn set_active_model(app: &AppHandle, id: String) -> Result<()> {
        if id.is_empty() {
            let path = Self::get_models_dir(app)?.join("active_whisper.txt");
            fs::write(path, "")?;
            return Ok(());
        }
        let item = Self::find_catalog_item(&id)?;
        if Self::get_resource_state(app, &item)? != ResourceState::Installed {
            bail!("Model `{id}` is not installed, compatible, and checksum-valid");
        }
        fs::write(Self::get_models_dir(app)?.join("active_whisper.txt"), id)?;
        Ok(())
    }

    pub fn resolve_model_path(app: &AppHandle, id: &str) -> Result<PathBuf> {
        let item = Self::find_catalog_item(id)?;
        if Self::get_resource_state(app, &item)? != ResourceState::Installed {
            bail!("Model `{id}` is not installed, compatible, and checksum-valid");
        }
        let (path, _) = resource_paths(&Self::get_models_dir(app)?, &item.id);
        Ok(path)
    }
}

fn catalog_item(
    id: &str,
    name: &str,
    size_bytes: u64,
    url: &str,
    checksum: &str,
    min_memory_mb: u64,
) -> ResourceItem {
    ResourceItem {
        id: id.to_string(),
        resource_type: ResourceType::WhisperModel,
        name: name.to_string(),
        version: "whisper.cpp-1.9.2".to_string(),
        size_bytes,
        url: url.to_string(),
        checksum: checksum.to_string(),
        compatibility: ResourceCompatibility {
            min_memory_mb,
            requires_avx2: false,
            supported_backends: vec!["CPU_BASIC".to_string(), "CPU_AVX2".to_string()],
            runtime_version: "1.9.2".to_string(),
        },
    }
}

fn vad_catalog_item() -> ResourceItem {
    ResourceItem {
        id: "silero-vad-v5".to_string(),
        resource_type: ResourceType::VadModel,
        name: "Silero VAD v5.1.2".to_string(),
        version: "silero-vad-5.1.2".to_string(),
        size_bytes: 885_098,
        url: "https://huggingface.co/ggml-org/whisper-vad/resolve/main/ggml-silero-v5.1.2.bin"
            .to_string(),
        checksum: "29940d98d42b91fbd05ce489f3ecf7c72f0a42f027e4875919a28fb4c04ea2cf".to_string(),
        compatibility: ResourceCompatibility {
            min_memory_mb: 512,
            requires_avx2: false,
            supported_backends: vec!["CPU_BASIC".to_string(), "CPU_AVX2".to_string()],
            runtime_version: "5.1.2".to_string(),
        },
    }
}

fn compatibility_error(
    compatibility: &ResourceCompatibility,
    profile: &RuntimeProfile,
) -> Option<String> {
    if !profile.runtime_available {
        return Some(
            "Whisper runtime is unavailable; repair the packaged runtime before using a model."
                .to_string(),
        );
    }
    if !profile
        .runtime_backends
        .iter()
        .any(|backend| backend == &profile.supported_acceleration || backend == "CPU_BASIC")
    {
        return Some("No supported packaged Whisper backend is available.".to_string());
    }
    if profile.total_memory_mb < compatibility.min_memory_mb {
        return Some(format!(
            "Requires at least {} MB RAM; detected {} MB.",
            compatibility.min_memory_mb, profile.total_memory_mb
        ));
    }
    if compatibility.requires_avx2 && !profile.has_avx2 {
        return Some("This resource requires CPU AVX2 support.".to_string());
    }
    if !compatibility
        .supported_backends
        .iter()
        .any(|backend| backend == &profile.supported_acceleration || backend == "CPU_BASIC")
    {
        return Some(format!(
            "Runtime backend {} is not supported by this resource.",
            profile.supported_acceleration
        ));
    }
    None
}

fn resource_paths(models_dir: &Path, id: &str) -> (PathBuf, PathBuf) {
    (
        models_dir.join(format!("{id}.bin")),
        models_dir.join(format!("{id}.manifest.json")),
    )
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    Ok(format_digest(Sha256::digest(bytes)))
}

fn format_digest(digest: impl AsRef<[u8]>) -> String {
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn write_manifest_atomic(path: &Path, manifest: &ResourceManifest) -> Result<()> {
    let temp_path = path.with_file_name(format!(
        "{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("resource.manifest.json")
    ));
    let serialized = serde_json::to_vec_pretty(manifest)?;
    fs::write(&temp_path, serialized)?;
    fs::rename(temp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_ids_match_runtime_preset_contract() {
        let ids: Vec<_> = ResourceManager::get_catalog()
            .into_iter()
            .map(|item| item.id)
            .collect();
        assert_eq!(
            ids,
            vec!["ggml-tiny", "ggml-base", "ggml-small", "silero-vad-v5"]
        );
    }

    #[test]
    fn compatibility_accepts_cpu_and_rejects_insufficient_memory() {
        let item = ResourceManager::get_catalog().remove(0);
        let compatible = RuntimeProfile {
            cpu_name: "test".to_string(),
            cpu_logical_cores: 4,
            total_memory_mb: 4_000,
            has_avx2: false,
            has_avx512: false,
            has_gpu: false,
            gpu_names: vec![],
            supported_acceleration: "CPU_BASIC".to_string(),
            runtime_available: true,
            runtime_version: None,
            runtime_backends: vec!["CPU_BASIC".to_string(), "CPU_AVX2".to_string()],
            recommended_model_ids: vec!["ggml-tiny".to_string(), "ggml-base".to_string()],
            fallback_reason: None,
        };
        assert!(compatibility_error(&item.compatibility, &compatible).is_none());

        let mut no_packaged_cpu = compatible.clone();
        no_packaged_cpu.runtime_backends = vec!["CUDA".to_string()];
        assert!(compatibility_error(&item.compatibility, &no_packaged_cpu).is_some());

        let mut too_little = compatible;
        too_little.total_memory_mb = 512;
        assert!(compatibility_error(&item.compatibility, &too_little).is_some());
    }

    #[tokio::test]
    async fn resource_job_manager_supports_cancel_and_cleanup() {
        let jobs = ResourceJobManager::default();
        assert!(jobs.begin("ggml-tiny").await);
        assert!(!jobs.begin("ggml-tiny").await);
        assert!(jobs.cancel("ggml-tiny").await);
        assert!(jobs.is_cancelled("ggml-tiny").await);
        jobs.finish("ggml-tiny").await;
        assert!(!jobs.is_active("ggml-tiny").await);
        assert!(!jobs.is_cancelled("ggml-tiny").await);
        jobs.acquire_model("ggml-tiny").await;
        jobs.acquire_model("ggml-tiny").await;
        assert!(jobs.is_model_in_use("ggml-tiny").await);
        jobs.release_model("ggml-tiny").await;
        assert!(jobs.is_model_in_use("ggml-tiny").await);
        jobs.release_model("ggml-tiny").await;
        assert!(!jobs.is_model_in_use("ggml-tiny").await);
    }

    #[test]
    fn sha256_file_hashes_bytes_not_only_manifest_metadata() {
        let temp_dir = tempfile::tempdir().expect("temp directory");
        let path = temp_dir.path().join("model.bin");
        fs::write(&path, b"model bytes").expect("write fixture");

        assert_eq!(
            sha256_file(&path).expect("hash fixture"),
            "9cb7487000bc86ac36ce83c4acfabe8878552be99572a6770f65ab1d048a5c48"
        );
    }
}
