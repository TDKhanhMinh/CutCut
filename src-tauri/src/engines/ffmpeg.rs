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

    let stdout =
        String::from_utf8(output.stdout).map_err(|_| MediaEngineError::InvalidUtf8)?;

    let version_line = stdout
        .lines()
        .next()
        .unwrap_or("unknown")
        .to_string();

    Ok(MediaBinaryInfo {
        name: display_name.to_string(),
        version_line,
        available: true,
    })
}

/// Run an arbitrary ffprobe command with given args and return stdout as String.
/// Arguments are passed as an array — never concatenated as a shell string.
pub async fn run_ffprobe(
    app: &AppHandle,
    args: Vec<String>,
) -> Result<String, MediaEngineError> {
    run_sidecar_command(app, FFPROBE_SIDECAR, "ffprobe", args).await
}

/// Run an arbitrary ffmpeg command with given args and return stdout as String.
/// Arguments are passed as an array — never concatenated as a shell string.
pub async fn run_ffmpeg(
    app: &AppHandle,
    args: Vec<String>,
) -> Result<String, MediaEngineError> {
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
