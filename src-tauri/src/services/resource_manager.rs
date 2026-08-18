use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Emitter};
use reqwest::Client;
use futures_util::StreamExt;
use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use crate::models::resource::{ResourceItem, ResourceType, ResourceState, ResourceManifest};

pub struct ResourceManager;

impl ResourceManager {
    pub fn get_catalog() -> Vec<ResourceItem> {
        vec![
            ResourceItem {
                id: "whisper-tiny".to_string(),
                resource_type: ResourceType::WhisperModel,
                name: "Fast (Tiny)".to_string(),
                version: "v1.9.2".to_string(),
                size_bytes: 77691713,
                url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin".to_string(),
                checksum: "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21".to_string(),
                compatibility: None,
            },
            ResourceItem {
                id: "whisper-base".to_string(),
                resource_type: ResourceType::WhisperModel,
                name: "Balanced (Base)".to_string(),
                version: "v1.9.2".to_string(),
                size_bytes: 147951465,
                url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin".to_string(),
                checksum: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe".to_string(),
                compatibility: None,
            },
            ResourceItem {
                id: "whisper-small".to_string(),
                resource_type: ResourceType::WhisperModel,
                name: "Accurate (Small)".to_string(),
                version: "v1.9.2".to_string(),
                size_bytes: 487601967,
                url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin".to_string(),
                checksum: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b".to_string(),
                compatibility: None,
            }
        ]
    }

    pub fn get_models_dir(app: &AppHandle) -> Result<PathBuf> {
        let dir = app.path().app_local_data_dir().context("Failed to get local data dir")?.join("models");
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        Ok(dir)
    }

    pub fn get_resource_state(app: &AppHandle, item: &ResourceItem) -> Result<ResourceState> {
        let models_dir = Self::get_models_dir(app)?;
        let bin_path = models_dir.join(format!("{}.bin", item.id));
        let manifest_path = models_dir.join(format!("{}.manifest.json", item.id));

        if !bin_path.exists() || !manifest_path.exists() {
            return Ok(ResourceState::NotInstalled);
        }

        let metadata = fs::metadata(&bin_path)?;
        if metadata.len() != item.size_bytes {
            return Ok(ResourceState::Corrupted);
        }

        let manifest_str = fs::read_to_string(&manifest_path)?;
        let manifest: ResourceManifest = serde_json::from_str(&manifest_str)?;

        if manifest.checksum != item.checksum {
            return Ok(ResourceState::Corrupted);
        }

        Ok(ResourceState::Installed)
    }

    pub async fn download_resource(app: AppHandle, id: String) -> Result<()> {
        let catalog = Self::get_catalog();
        let item = catalog.into_iter().find(|i| i.id == id).context("Resource not found")?;

        let models_dir = Self::get_models_dir(&app)?;
        let tmp_path = models_dir.join(format!("{}.tmp", item.id));
        let bin_path = models_dir.join(format!("{}.bin", item.id));
        let manifest_path = models_dir.join(format!("{}.manifest.json", item.id));

        let client = Client::new();
        let res = client.get(&item.url).send().await?;
        if !res.status().is_success() {
            anyhow::bail!("Failed to download: {}", res.status());
        }

        let total_size = res.content_length().unwrap_or(item.size_bytes);
        let mut downloaded: u64 = 0;
        let mut stream = res.bytes_stream();

        let mut out_file = File::create(&tmp_path).await?;
        let mut hasher = Sha256::new();

        while let Some(chunk_res) = stream.next().await {
            let chunk = chunk_res?;
            out_file.write_all(&chunk).await?;
            hasher.update(&chunk);
            downloaded += chunk.len() as u64;

            let progress = downloaded as f64 / total_size as f64;
            let _ = app.emit("resource-download-progress", serde_json::json!({
                "id": item.id,
                "progress": progress,
                "downloaded": downloaded,
                "total": total_size
            }));
        }

        let hash_bytes = hasher.finalize();
        let hash_result = hash_bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        
        if hash_result != item.checksum {
            let _ = fs::remove_file(&tmp_path);
            anyhow::bail!("Checksum mismatch!");
        }

        fs::rename(&tmp_path, &bin_path)?;

        let manifest = ResourceManifest {
            id: item.id.clone(),
            checksum: item.checksum.clone(),
            size_bytes: item.size_bytes,
            installed_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };

        fs::write(&manifest_path, serde_json::to_string(&manifest)?)?;

        let _ = app.emit("resource-download-finished", serde_json::json!({ "id": item.id }));

        Ok(())
    }

    pub fn delete_resource(app: &AppHandle, id: String) -> Result<()> {
        let models_dir = Self::get_models_dir(app)?;
        let _ = fs::remove_file(models_dir.join(format!("{}.bin", id)));
        let _ = fs::remove_file(models_dir.join(format!("{}.manifest.json", id)));
        let _ = fs::remove_file(models_dir.join(format!("{}.tmp", id)));
        Ok(())
    }

    pub fn get_active_model(app: &AppHandle) -> Result<Option<String>> {
        let path = Self::get_models_dir(app)?.join("active_whisper.txt");
        if path.exists() {
            Ok(Some(fs::read_to_string(path)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_active_model(app: &AppHandle, id: String) -> Result<()> {
        let path = Self::get_models_dir(app)?.join("active_whisper.txt");
        fs::write(path, id)?;
        Ok(())
    }
}
