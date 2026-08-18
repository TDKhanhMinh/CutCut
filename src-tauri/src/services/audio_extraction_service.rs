use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub struct AudioExtractionService;

impl AudioExtractionService {
    pub fn get_temp_audio_dir(app: &AppHandle) -> Result<PathBuf> {
        let dir = app
            .path()
            .app_local_data_dir()
            .context("Failed to get local data dir")?
            .join("temp_audio");
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        Ok(dir)
    }

    pub fn cleanup_stale_audio(app: &AppHandle) -> Result<()> {
        let dir = Self::get_temp_audio_dir(app)?;
        if dir.exists() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() && path.extension().unwrap_or_default() == "wav" {
                    let _ = fs::remove_file(path);
                }
            }
        }
        Ok(())
    }

    pub async fn extract_audio_for_stt(
        app: AppHandle,
        source_path: String,
        job_id: String,
        duration_us: Option<u64>,
    ) -> Result<String> {
        let dir = Self::get_temp_audio_dir(&app)?;
        let temp_path = dir.join(format!("{}.wav", job_id));
        let temp_path_str = temp_path.to_string_lossy().to_string();

        let args = vec![
            "-y".to_string(),
            "-i".to_string(),
            source_path,
            "-vn".to_string(),
            "-ar".to_string(),
            "16000".to_string(),
            "-ac".to_string(),
            "1".to_string(),
            "-c:a".to_string(),
            "pcm_s16le".to_string(),
            temp_path_str.clone(),
        ];

        crate::services::media_job::spawn_ffmpeg_job(app, job_id, args, duration_us).await?;

        Ok(temp_path_str)
    }

    pub fn cleanup_stt_audio(app: &AppHandle, temp_path: String) -> Result<()> {
        let path = Path::new(&temp_path);
        let dir = Self::get_temp_audio_dir(app)?;
        
        // Safety check to ensure we only delete within temp_audio
        if path.starts_with(dir) && path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}
