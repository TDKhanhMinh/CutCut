use anyhow::{Context, Result};
use std::path::Path;
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::ShellExt;
use std::fs;
use crate::models::whisper::WhisperResult;

pub struct WhisperService;

impl WhisperService {
    pub async fn transcribe(
        app: &AppHandle,
        audio_path: &str,
        model_path: &str,
        language: &str,
    ) -> Result<WhisperResult> {
        let audio_path_obj = Path::new(audio_path);
        if !audio_path_obj.exists() {
            anyhow::bail!("Audio file does not exist: {}", audio_path);
        }

        let model_path_obj = Path::new(model_path);
        if !model_path_obj.exists() {
            anyhow::bail!("Model file does not exist: {}", model_path);
        }
        
        let output_base_path = audio_path_obj.with_extension("whisper_out");
        let json_output_path = output_base_path.with_extension("whisper_out.json");

        let shell = app.shell();
        let command = shell.sidecar("whisper").context("Failed to create sidecar command. Is whisper configured in externalBin?")?;

        let args = vec![
            "-m".to_string(),
            model_path.to_string(),
            "-f".to_string(),
            audio_path.to_string(),
            "-l".to_string(),
            language.to_string(),
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
            .context("Failed to execute whisper sidecar")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Whisper execution failed: {}", stderr);
        }

        if !json_output_path.exists() {
            anyhow::bail!("Whisper completed but JSON output was not found at {:?}", json_output_path);
        }

        let json_content = fs::read_to_string(&json_output_path)
            .context("Failed to read whisper JSON output")?;

        let result: WhisperResult = serde_json::from_str(&json_content)
            .context("Failed to parse whisper JSON output")?;

        let _ = fs::remove_file(&json_output_path);

        Ok(result)
    }
}
