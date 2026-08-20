use crate::models::whisper::{WhisperResult, WhisperRuntimeInfo};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WhisperError {
    #[error("audio input does not exist or is not a file: {0}")]
    InvalidAudio(PathBuf),
    #[error("model file does not exist or is not a file: {0}")]
    InvalidModel(PathBuf),
    #[error("language must be `auto` or a two-letter code")]
    InvalidLanguage,
    #[error("whisper sidecar is unavailable: {0}")]
    SidecarUnavailable(String),
    #[error("whisper runtime exited with code {code}: {stderr}")]
    RuntimeFailure { code: i32, stderr: String },
    #[error("whisper output was not created: {0}")]
    MissingOutput(PathBuf),
    #[error("failed to read whisper output: {0}")]
    OutputRead(#[from] std::io::Error),
    #[error("invalid whisper JSON output: {0}")]
    OutputParse(#[from] serde_json::Error),
}

pub struct WhisperService;

impl WhisperService {
    pub async fn check_runtime(
        app: &AppHandle,
    ) -> std::result::Result<WhisperRuntimeInfo, WhisperError> {
        let command = app
            .shell()
            .sidecar("whisper")
            .map_err(|error| WhisperError::SidecarUnavailable(error.to_string()))?;
        let output = command
            .args(["--version"])
            .output()
            .await
            .map_err(|error| WhisperError::SidecarUnavailable(error.to_string()))?;

        if !output.status.success() {
            return Err(WhisperError::RuntimeFailure {
                code: output.status.code().unwrap_or(-1),
                stderr: trim_diagnostic(&output.stderr),
            });
        }

        let banner = first_non_empty_line(&output.stdout)
            .or_else(|| first_non_empty_line(&output.stderr))
            .unwrap_or_else(|| "whisper.cpp runtime".to_string());
        Ok(WhisperRuntimeInfo {
            available: true,
            version: banner,
            backend: "cpu".to_string(),
        })
    }

    pub async fn transcribe(
        app: &AppHandle,
        audio_path: &str,
        model_path: &str,
        language: &str,
    ) -> std::result::Result<WhisperResult, WhisperError> {
        let audio_path_obj = Path::new(audio_path);
        if !audio_path_obj.is_file() {
            return Err(WhisperError::InvalidAudio(audio_path_obj.to_path_buf()));
        }

        let model_path_obj = Path::new(model_path);
        if !model_path_obj.is_file() {
            return Err(WhisperError::InvalidModel(model_path_obj.to_path_buf()));
        }

        let language = normalize_language(language)?;
        let output_base_path = audio_path_obj.with_extension("whisper_out");
        let json_output_path =
            PathBuf::from(format!("{}.json", output_base_path.to_string_lossy()));
        let _ = fs::remove_file(&json_output_path);

        let command = app
            .shell()
            .sidecar("whisper")
            .map_err(|error| WhisperError::SidecarUnavailable(error.to_string()))?;
        let args = [
            "-m".to_string(),
            model_path.to_string(),
            "-f".to_string(),
            audio_path.to_string(),
            "-l".to_string(),
            language,
            "-of".to_string(),
            output_base_path.to_string_lossy().to_string(),
            "-oj".to_string(),
            "-nt".to_string(),
            "-np".to_string(),
        ];

        let output = command
            .args(args)
            .output()
            .await
            .map_err(|error| WhisperError::SidecarUnavailable(error.to_string()))?;

        if !output.status.success() {
            let _ = fs::remove_file(&json_output_path);
            return Err(WhisperError::RuntimeFailure {
                code: output.status.code().unwrap_or(-1),
                stderr: trim_diagnostic(&output.stderr),
            });
        }

        if !json_output_path.is_file() {
            return Err(WhisperError::MissingOutput(json_output_path));
        }

        let json_content = fs::read_to_string(&json_output_path)?;
        let result = serde_json::from_str::<WhisperResult>(&json_content)?;
        let _ = fs::remove_file(&json_output_path);
        Ok(result)
    }
}

fn normalize_language(language: &str) -> std::result::Result<String, WhisperError> {
    let normalized = if language.trim().is_empty() {
        "auto"
    } else {
        language.trim()
    };

    if normalized != "auto"
        && (normalized.len() != 2 || !normalized.bytes().all(|byte| byte.is_ascii_alphabetic()))
    {
        return Err(WhisperError::InvalidLanguage);
    }
    Ok(normalized.to_ascii_lowercase())
}

fn trim_diagnostic(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        "no stderr output".to_string()
    } else {
        trimmed.chars().take(2000).collect()
    }
}

fn first_non_empty_line(bytes: &[u8]) -> Option<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_input_is_bounded_before_reaching_sidecar() {
        assert_eq!(normalize_language("").unwrap(), "auto");
        assert_eq!(normalize_language("VI").unwrap(), "vi");
        assert!(matches!(
            normalize_language("vi; --model evil"),
            Err(WhisperError::InvalidLanguage)
        ));
    }

    #[test]
    fn diagnostics_are_trimmed_and_bounded() {
        assert_eq!(trim_diagnostic(b"  runtime failed  "), "runtime failed");
        assert_eq!(trim_diagnostic(b"\n\n"), "no stderr output");
    }
}
