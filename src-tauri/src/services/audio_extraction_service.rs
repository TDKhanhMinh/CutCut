use crate::engines::ffmpeg::{run_ffprobe, MediaEngineError};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AudioExtractionError {
    #[error("source media is not a readable file: {0}")]
    InvalidSource(PathBuf),

    #[error("source media does not contain an audio stream")]
    NoAudioStream,

    #[error("job id must be a UUID")]
    InvalidJobId,

    #[error("temporary audio path is outside the app-owned directory")]
    UnsafeTempPath,

    #[error("temporary audio output already exists")]
    TempOutputExists,

    #[error("temporary audio output was not created")]
    MissingOutput,

    #[error("temporary audio path is not valid Unicode")]
    NonUnicodePath,

    #[error("failed to resolve app data directory: {0}")]
    AppPath(String),

    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),

    #[error("FFmpeg/FFprobe error: {0}")]
    Engine(#[from] MediaEngineError),
}

pub struct AudioExtractionService;

impl AudioExtractionService {
    pub fn get_temp_audio_dir(app: &AppHandle) -> Result<PathBuf, AudioExtractionError> {
        let dir = app
            .path()
            .app_local_data_dir()
            .map_err(|error| AudioExtractionError::AppPath(error.to_string()))?
            .join("temp_audio");
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    pub fn cleanup_stale_audio(app: &AppHandle) -> Result<(), AudioExtractionError> {
        let dir = Self::get_temp_audio_dir(app)?;
        let root = fs::canonicalize(&dir)?;
        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.file_type().is_file()
                && path.extension().and_then(|extension| extension.to_str()) == Some("wav")
            {
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }

    pub async fn extract_audio_for_stt(
        app: AppHandle,
        source_path: String,
        requested_job_id: Option<String>,
        duration_us: Option<u64>,
    ) -> Result<String, AudioExtractionError> {
        let source = validate_source_path(&source_path)?;
        let job_id = normalize_job_id(requested_job_id.as_deref())?;
        let dir = Self::get_temp_audio_dir(&app)?;
        let root = fs::canonicalize(&dir)?;
        let temp_path = safe_temp_path(&root, &job_id)?;
        if temp_path.exists() {
            return Err(AudioExtractionError::TempOutputExists);
        }

        ensure_audio_stream(&app, &source).await?;

        let source_path = source
            .to_str()
            .ok_or(AudioExtractionError::NonUnicodePath)?
            .to_string();
        let temp_path_str = temp_path
            .to_str()
            .ok_or(AudioExtractionError::NonUnicodePath)?
            .to_string();
        let args = vec![
            "-hide_banner".to_string(),
            "-loglevel".to_string(),
            "error".to_string(),
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

        let result = crate::services::media_job::spawn_ffmpeg_job_and_wait(
            app.clone(),
            job_id,
            args,
            duration_us,
        )
        .await;

        if let Err(error) = result {
            let _ = remove_owned_temp_file(&root, &temp_path);
            return Err(error.into());
        }

        if !temp_path.is_file() {
            let _ = remove_owned_temp_file(&root, &temp_path);
            return Err(AudioExtractionError::MissingOutput);
        }

        Ok(temp_path_str)
    }

    pub fn cleanup_stt_audio(
        app: &AppHandle,
        temp_path: String,
    ) -> Result<(), AudioExtractionError> {
        let dir = Self::get_temp_audio_dir(app)?;
        let root = fs::canonicalize(&dir)?;
        let path = PathBuf::from(temp_path);
        if !path.exists() {
            return Ok(());
        }
        let canonical_path = fs::canonicalize(&path)?;
        remove_owned_temp_file(&root, &canonical_path)
    }
}

fn validate_source_path(source_path: &str) -> Result<PathBuf, AudioExtractionError> {
    let path = Path::new(source_path);
    if !path.is_file() {
        return Err(AudioExtractionError::InvalidSource(path.to_path_buf()));
    }
    Ok(fs::canonicalize(path)?)
}

fn normalize_job_id(requested: Option<&str>) -> Result<String, AudioExtractionError> {
    let Some(value) = requested.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(Uuid::new_v4().to_string());
    };
    Uuid::parse_str(value)
        .map(|uuid| uuid.to_string())
        .map_err(|_| AudioExtractionError::InvalidJobId)
}

fn safe_temp_path(root: &Path, job_id: &str) -> Result<PathBuf, AudioExtractionError> {
    let path = root.join(format!("{job_id}.wav"));
    if path.parent() != Some(root)
        || path.extension().and_then(|extension| extension.to_str()) != Some("wav")
    {
        return Err(AudioExtractionError::UnsafeTempPath);
    }
    Ok(path)
}

fn remove_owned_temp_file(root: &Path, path: &Path) -> Result<(), AudioExtractionError> {
    if path.parent() != Some(root)
        || path.extension().and_then(|extension| extension.to_str()) != Some("wav")
    {
        return Err(AudioExtractionError::UnsafeTempPath);
    }
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

async fn ensure_audio_stream(app: &AppHandle, source: &Path) -> Result<(), AudioExtractionError> {
    let source_path = source
        .to_str()
        .ok_or(AudioExtractionError::NonUnicodePath)?
        .to_string();
    let output = run_ffprobe(
        app,
        vec![
            "-v".to_string(),
            "error".to_string(),
            "-select_streams".to_string(),
            "a:0".to_string(),
            "-show_entries".to_string(),
            "stream=index".to_string(),
            "-of".to_string(),
            "csv=p=0".to_string(),
            source_path,
        ],
    )
    .await?;

    if !has_audio_stream(&output) {
        return Err(AudioExtractionError::NoAudioStream);
    }
    Ok(())
}

fn has_audio_stream(ffprobe_output: &str) -> bool {
    ffprobe_output.lines().any(|line| !line.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_job_id_is_generated_as_uuid() {
        let id = normalize_job_id(None).expect("generated id");
        assert!(Uuid::parse_str(&id).is_ok());
    }

    #[test]
    fn only_uuid_job_ids_are_accepted() {
        let valid = "7e1d0c6f-b7d5-4d4b-a08b-0b3d0e8d8c72";
        assert_eq!(normalize_job_id(Some(valid)).unwrap(), valid);
        for invalid in ["..\\outside", "-y", "job.wav", "{uuid}"] {
            assert!(matches!(
                normalize_job_id(Some(invalid)),
                Err(AudioExtractionError::InvalidJobId)
            ));
        }
    }

    #[test]
    fn temp_path_is_direct_child_of_app_owned_root() {
        let root = Path::new("cutcut-temp-audio");
        let path = safe_temp_path(root, "7e1d0c6f-b7d5-4d4b-a08b-0b3d0e8d8c72").unwrap();
        assert_eq!(path.parent(), Some(root));
        assert!(matches!(
            safe_temp_path(root, "../outside"),
            Err(AudioExtractionError::UnsafeTempPath)
        ));
    }

    #[test]
    fn cleanup_rejects_paths_outside_root() {
        let root = Path::new("cutcut-temp-audio");
        let outside = Path::new("cutcut-outside.wav");
        assert!(matches!(
            remove_owned_temp_file(root, outside),
            Err(AudioExtractionError::UnsafeTempPath)
        ));
    }

    #[test]
    fn empty_ffprobe_result_is_a_domain_no_audio_signal() {
        assert!(!has_audio_stream("\n\n"));
        assert!(has_audio_stream("1\n"));
    }
}
