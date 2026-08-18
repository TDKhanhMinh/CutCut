use tauri::{AppHandle, Manager};

use crate::engines::ffmpeg::{self, MediaBinaryInfo, MediaEngineError};

/// Tauri command: Check FFmpeg and FFprobe versions.
/// Returns version info for both binaries.
#[tauri::command]
pub async fn check_media_engines(
    app: AppHandle,
) -> Result<Vec<MediaBinaryInfo>, MediaEngineError> {
    let ffmpeg = ffmpeg::get_ffmpeg_version(&app).await?;
    let ffprobe = ffmpeg::get_ffprobe_version(&app).await?;
    Ok(vec![ffmpeg, ffprobe])
}

/// Tauri command: Get FFmpeg version only.
#[tauri::command]
pub async fn get_ffmpeg_version(
    app: AppHandle,
) -> Result<MediaBinaryInfo, MediaEngineError> {
    ffmpeg::get_ffmpeg_version(&app).await
}

/// Tauri command: Get FFprobe version only.
#[tauri::command]
pub async fn get_ffprobe_version(
    app: AppHandle,
) -> Result<MediaBinaryInfo, MediaEngineError> {
    ffmpeg::get_ffprobe_version(&app).await
}

/// Tauri command: Cancel an ongoing media job.
#[tauri::command]
pub async fn cancel_media_job(app: AppHandle, job_id: String) -> Result<(), String> {
    let job_manager = app.state::<crate::services::media_job::JobManager>();
    job_manager.cancel_job(&job_id).await
}

/// Tauri command: Spawn a test FFmpeg job to verify progress events.
/// This generates a 5-second test video.
#[tauri::command]
pub async fn spawn_test_ffmpeg_job(app: AppHandle) -> Result<String, String> {
    let job_id = uuid::Uuid::new_v4().to_string();
    
    // Create a 5-second test video using lavfi
    let args = vec![
        "-y".to_string(),
        "-f".to_string(), "lavfi".to_string(),
        "-i".to_string(), "testsrc=duration=5:size=1280x720:rate=30".to_string(),
        // We will output to null to just test the process and progress
        "-f".to_string(), "null".to_string(), "-".to_string(),
    ];

    // Total duration is 5 seconds = 5,000,000 microseconds
    let total_duration_us = Some(5_000_000);

    crate::services::media_job::spawn_ffmpeg_job(app, job_id.clone(), args, total_duration_us)
        .await
        .map_err(|e| e.to_string())?;

    Ok(job_id)
}

/// Tauri command: Use FFprobe to read metadata of a local video file.
#[tauri::command]
pub async fn read_media_metadata(
    app: AppHandle,
    path: String,
) -> Result<crate::models::media_info::MediaSourceMetadata, MediaEngineError> {
    crate::engines::ffmpeg::read_media_metadata(&app, path).await
}

/// Tauri command: Export a prototype MP4 video using FFmpeg.
#[tauri::command]
pub async fn export_prototype_video(
    app: AppHandle,
    input_path: String,
    output_path: String,
    total_duration_sec: f64,
) -> Result<String, String> {
    let job_id = uuid::Uuid::new_v4().to_string();
    
    // Scale to max 720p height, preserving aspect ratio. 
    // -2 means calculate width automatically to maintain aspect ratio and ensure it's divisible by 2.
    let args = vec![
        "-y".to_string(), // overwrite
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
        "-progress".to_string(),
        "pipe:1".to_string(),
        output_path,
    ];

    let total_duration_us = (total_duration_sec * 1_000_000.0) as u64;

    crate::services::media_job::spawn_ffmpeg_job(app, job_id.clone(), args, Some(total_duration_us))
        .await
        .map_err(|e| e.to_string())?;

    Ok(job_id)
}

/// Tauri command: Check if a media file exists at the given path.
#[tauri::command]
pub async fn check_media_exists(path: String) -> Result<bool, String> {
    Ok(std::path::Path::new(&path).exists())
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

