use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

use crate::engines::ffmpeg::{self, MediaBinaryInfo, MediaEngineError};

/// Tauri command: Check FFmpeg and FFprobe versions.
#[tauri::command]
pub async fn check_media_engines(app: AppHandle) -> Result<Vec<MediaBinaryInfo>, MediaEngineError> {
    let ffmpeg = ffmpeg::get_ffmpeg_version(&app).await?;
    let ffprobe = ffmpeg::get_ffprobe_version(&app).await?;
    Ok(vec![ffmpeg, ffprobe])
}

#[tauri::command]
pub async fn get_ffmpeg_version(app: AppHandle) -> Result<MediaBinaryInfo, MediaEngineError> {
    ffmpeg::get_ffmpeg_version(&app).await
}

#[tauri::command]
pub async fn get_ffprobe_version(app: AppHandle) -> Result<MediaBinaryInfo, MediaEngineError> {
    ffmpeg::get_ffprobe_version(&app).await
}

#[tauri::command]
pub async fn cancel_media_job(app: AppHandle, job_id: String) -> Result<(), String> {
    let job_manager = app.state::<crate::services::media_job::JobManager>();
    job_manager.cancel_job(&job_id).await
}

/// Tauri command: Spawn a test FFmpeg job to verify progress events.
#[tauri::command]
pub async fn spawn_test_ffmpeg_job(app: AppHandle) -> Result<String, String> {
    let job_id = uuid::Uuid::new_v4().to_string();
    let args = vec![
        "-y".to_string(),
        "-f".to_string(),
        "lavfi".to_string(),
        "-i".to_string(),
        "testsrc=duration=5:size=1280x720:rate=30".to_string(),
        "-f".to_string(),
        "null".to_string(),
        "-".to_string(),
    ];

    crate::services::media_job::spawn_ffmpeg_job(app, job_id.clone(), args, Some(5_000_000))
        .await
        .map_err(|e| e.to_string())?;

    Ok(job_id)
}

#[tauri::command]
pub async fn read_media_metadata(
    app: AppHandle,
    path: String,
) -> Result<crate::models::media_info::MediaSourceMetadata, MediaEngineError> {
    crate::engines::ffmpeg::read_media_metadata(&app, path).await
}

#[tauri::command]
pub async fn export_prototype_video(
    app: AppHandle,
    input_path: String,
    output_path: String,
    total_duration_sec: f64,
) -> Result<String, String> {
    let (input_path, output_path) = validate_export_paths(&input_path, &output_path)?;
    if !total_duration_sec.is_finite() || total_duration_sec <= 0.0 {
        return Err("Video duration must be a positive finite value".to_string());
    }

    let job_id = uuid::Uuid::new_v4().to_string();
    let args = vec![
        "-y".to_string(),
        "-i".to_string(),
        input_path,
        "-vf".to_string(),
        "scale=-2:720".to_string(),
        "-c:v".to_string(),
        "libx264".to_string(),
        "-preset".to_string(),
        "fast".to_string(),
        "-crf".to_string(),
        "23".to_string(),
        "-c:a".to_string(),
        "aac".to_string(),
        output_path,
    ];

    let total_duration_us = (total_duration_sec * 1_000_000.0) as u64;
    crate::services::media_job::spawn_ffmpeg_job(
        app,
        job_id.clone(),
        args,
        Some(total_duration_us),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(job_id)
}

#[tauri::command]
pub async fn check_media_exists(path: String) -> Result<bool, String> {
    Ok(Path::new(&path).exists())
}

#[tauri::command]
pub async fn extract_audio_for_stt(
    app: AppHandle,
    source_path: String,
    job_id: String,
    duration_us: Option<u64>,
) -> Result<String, String> {
    crate::services::audio_extraction_service::AudioExtractionService::extract_audio_for_stt(
        app,
        source_path,
        job_id,
        duration_us,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cleanup_stt_audio(app: AppHandle, temp_path: String) -> Result<(), String> {
    crate::services::audio_extraction_service::AudioExtractionService::cleanup_stt_audio(
        &app, temp_path,
    )
    .map_err(|e| e.to_string())
}

fn validate_export_paths(input_path: &str, output_path: &str) -> Result<(String, String), String> {
    let input = canonical_existing_path(input_path, "Input")?;
    if input.is_dir() {
        return Err("Input path must be a media file, not a directory".to_string());
    }

    let output = canonical_output_path(output_path)?;
    if input == output {
        return Err("Export output must be different from the source media path".to_string());
    }

    if !output
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("mp4"))
    {
        return Err("Export output must use the .mp4 extension".to_string());
    }

    Ok((path_to_string(&input)?, path_to_string(&output)?))
}

fn canonical_existing_path(path: &str, label: &str) -> Result<PathBuf, String> {
    let path = Path::new(path);
    if path.as_os_str().is_empty() || !path.is_absolute() {
        return Err(format!("{label} path must be absolute"));
    }
    path.canonicalize()
        .map_err(|error| format!("{label} path is unavailable: {error}"))
}

fn canonical_output_path(path: &str) -> Result<PathBuf, String> {
    let path = Path::new(path);
    if path.as_os_str().is_empty() || !path.is_absolute() {
        return Err("Output path must be absolute".to_string());
    }

    if path.exists() {
        return path
            .canonicalize()
            .map_err(|error| format!("Output path is unavailable: {error}"));
    }

    let parent = path
        .parent()
        .ok_or_else(|| "Output path must have a parent directory".to_string())?;
    let file_name = path
        .file_name()
        .ok_or_else(|| "Output path must include a file name".to_string())?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|error| format!("Output directory is unavailable: {error}"))?;

    Ok(canonical_parent.join(file_name))
}

fn path_to_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| "Path must be valid Unicode".to_string())
}

#[cfg(test)]
mod tests {
    use super::validate_export_paths;
    use std::fs;

    #[test]
    fn rejects_export_that_would_overwrite_the_source() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.mp4");
        fs::write(&source, "source media").unwrap();

        let error = validate_export_paths(source.to_str().unwrap(), source.to_str().unwrap())
            .expect_err("source and output must differ");

        assert!(error.contains("different from the source"));
    }
}
