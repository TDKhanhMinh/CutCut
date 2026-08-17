use tauri::AppHandle;

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
