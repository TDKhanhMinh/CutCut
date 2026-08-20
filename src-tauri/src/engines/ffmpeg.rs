use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error types — typed errors for process spawn failures
// ---------------------------------------------------------------------------

#[derive(Error, Debug, Serialize, Deserialize, Clone)]
pub enum MediaEngineError {
    #[error("Sidecar binary not found: {binary}. Ensure FFmpeg is bundled correctly.")]
    BinaryNotFound { binary: String },

    #[error("Process exited with code {code}: {stderr}")]
    ProcessFailed { code: i32, stderr: String },

    #[error("Failed to spawn process: {message}")]
    SpawnFailed { message: String },

    #[error("Process output is not valid UTF-8")]
    InvalidUtf8,

    #[error("Invalid media input: {message}")]
    InvalidInput { message: String },
}

// ---------------------------------------------------------------------------
// Version info — structured result from ffmpeg/ffprobe -version
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaBinaryInfo {
    /// Name of the binary (e.g. "ffmpeg" or "ffprobe")
    pub name: String,
    /// Full first line of -version output
    pub version_line: String,
    /// Whether the binary was found and executed successfully
    pub available: bool,
}

// ---------------------------------------------------------------------------
// FfmpegService — resolve and spawn FFmpeg/FFprobe sidecars
// ---------------------------------------------------------------------------

/// Sidecar names must match the keys in `tauri.conf.json > bundle > externalBin`
const FFMPEG_SIDECAR: &str = "ffmpeg";
const FFPROBE_SIDECAR: &str = "ffprobe";

/// Run `ffmpeg -version` through the sidecar and return structured info.
pub async fn get_ffmpeg_version(app: &AppHandle) -> Result<MediaBinaryInfo, MediaEngineError> {
    run_version_check(app, FFMPEG_SIDECAR, "ffmpeg").await
}

/// Run `ffprobe -version` through the sidecar and return structured info.
pub async fn get_ffprobe_version(app: &AppHandle) -> Result<MediaBinaryInfo, MediaEngineError> {
    run_version_check(app, FFPROBE_SIDECAR, "ffprobe").await
}

/// Internal helper: spawn a sidecar with `-version` and parse the first line.
async fn run_version_check(
    app: &AppHandle,
    sidecar_name: &str,
    display_name: &str,
) -> Result<MediaBinaryInfo, MediaEngineError> {
    let cmd = app
        .shell()
        .sidecar(sidecar_name)
        .map_err(|e| MediaEngineError::BinaryNotFound {
            binary: format!("{}: {}", display_name, e),
        })?
        .args(["-version"]);

    let output = cmd
        .output()
        .await
        .map_err(|e| MediaEngineError::SpawnFailed {
            message: format!("{}: {}", display_name, e),
        })?;

    if !output.status.success() {
        return Err(MediaEngineError::ProcessFailed {
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8(output.stderr)
                .unwrap_or_else(|_| "<non-utf8 stderr>".to_string()),
        });
    }

    let stdout = String::from_utf8(output.stdout).map_err(|_| MediaEngineError::InvalidUtf8)?;

    let version_line = stdout.lines().next().unwrap_or("unknown").to_string();

    Ok(MediaBinaryInfo {
        name: display_name.to_string(),
        version_line,
        available: true,
    })
}

/// Run an arbitrary ffprobe command with given args and return stdout as String.
/// Arguments are passed as an array — never concatenated as a shell string.
pub async fn run_ffprobe(app: &AppHandle, args: Vec<String>) -> Result<String, MediaEngineError> {
    run_sidecar_command(app, FFPROBE_SIDECAR, "ffprobe", args).await
}

/// Run an arbitrary ffmpeg command with given args and return stdout as String.
/// Arguments are passed as an array — never concatenated as a shell string.
pub async fn run_ffmpeg(app: &AppHandle, args: Vec<String>) -> Result<String, MediaEngineError> {
    run_sidecar_command(app, FFMPEG_SIDECAR, "ffmpeg", args).await
}

/// Internal helper: spawn a sidecar with arbitrary args, capture output.
async fn run_sidecar_command(
    app: &AppHandle,
    sidecar_name: &str,
    display_name: &str,
    args: Vec<String>,
) -> Result<String, MediaEngineError> {
    let cmd = app
        .shell()
        .sidecar(sidecar_name)
        .map_err(|e| MediaEngineError::BinaryNotFound {
            binary: format!("{}: {}", display_name, e),
        })?
        .args(args);

    let output = cmd
        .output()
        .await
        .map_err(|e| MediaEngineError::SpawnFailed {
            message: format!("{}: {}", display_name, e),
        })?;

    if !output.status.success() {
        return Err(MediaEngineError::ProcessFailed {
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8(output.stderr)
                .unwrap_or_else(|_| "<non-utf8 stderr>".to_string()),
        });
    }

    String::from_utf8(output.stdout).map_err(|_| MediaEngineError::InvalidUtf8)
}

/// Helper function to parse frame rate from FFprobe string like "30000/1001" or "30"
fn parse_frame_rate(fps_str: &str) -> f64 {
    let parts: Vec<&str> = fps_str.split('/').collect();
    if parts.len() == 2 {
        let num: f64 = parts[0].parse().unwrap_or(0.0);
        let den: f64 = parts[1].parse().unwrap_or(1.0);
        if den > 0.0 {
            return num / den;
        }
    } else if let Ok(fps) = fps_str.parse::<f64>() {
        return fps;
    }
    0.0
}

/// Use FFprobe to read metadata and parse into MediaSourceMetadata
pub async fn read_media_metadata(
    app: &AppHandle,
    path: String,
) -> Result<crate::models::media_info::MediaSourceMetadata, MediaEngineError> {
    let args = vec![
        "-v".to_string(),
        "quiet".to_string(),
        "-print_format".to_string(),
        "json".to_string(),
        "-show_format".to_string(),
        "-show_streams".to_string(),
        path.clone(),
    ];

    let output = run_ffprobe(app, args).await?;
    let json: serde_json::Value =
        serde_json::from_str(&output).map_err(|e| MediaEngineError::SpawnFailed {
            message: format!("Failed to parse ffprobe JSON: {}", e),
        })?;

    let format = json
        .get("format")
        .ok_or_else(|| MediaEngineError::SpawnFailed {
            message: "No format section in ffprobe output".into(),
        })?;

    // Parse duration from format
    let duration_sec: f64 = format
        .get("duration")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);

    let streams = json
        .get("streams")
        .and_then(|v| v.as_array())
        .ok_or_else(|| MediaEngineError::SpawnFailed {
            message: "No streams in ffprobe output".into(),
        })?;

    // Find video stream
    let video_stream = streams
        .iter()
        .find(|s| s.get("codec_type").and_then(|v| v.as_str()) == Some("video"));

    // Find audio stream
    let audio_stream = streams
        .iter()
        .find(|s| s.get("codec_type").and_then(|v| v.as_str()) == Some("audio"));

    let audio_codec = audio_stream
        .and_then(|s| s.get("codec_name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let v = video_stream.ok_or_else(|| MediaEngineError::InvalidInput {
        message: "Selected file does not contain a video stream".into(),
    })?;

    let width = v.get("width").and_then(|w| w.as_u64()).unwrap_or(0) as u32;
    let height = v.get("height").and_then(|h| h.as_u64()).unwrap_or(0) as u32;
    let video_codec = v
        .get("codec_name")
        .and_then(|c| c.as_str())
        .unwrap_or("unknown")
        .to_string();
    let mut fps = 0.0;
    let mut rotation = 0;

    if let Some(r_frame_rate) = v.get("r_frame_rate").and_then(|f| f.as_str()) {
        fps = parse_frame_rate(r_frame_rate);
    }

    // Try to parse rotation from tags
    if let Some(tags) = v.get("tags") {
        if let Some(rot) = tags.get("rotate").and_then(|r| r.as_str()) {
            rotation = rot.parse().unwrap_or(0);
        }
    }

    if duration_sec <= 0.0 || width == 0 || height == 0 || fps <= 0.0 {
        return Err(MediaEngineError::InvalidInput {
            message: "Video metadata is incomplete or has zero duration".into(),
        });
    }

    Ok(crate::models::media_info::MediaSourceMetadata {
        path,
        duration_sec,
        fps,
        width,
        height,
        video_codec,
        audio_codec,
        rotation,
    })
}
