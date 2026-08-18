use anyhow::{Context, Result};
use regex::Regex;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Emitter};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;
use crate::models::vad::{NonSpeechInterval, SpeechInterval, VadAnalysisResult, VadConfig};
use crate::services::resource_manager::ResourceManager;
use crate::models::resource::ResourceState;
use crate::services::media_job::JobManager;
use crate::models::media_job::{MediaJobEvent, MediaJobState};
use crate::services::audio_extraction_service::AudioExtractionService;

pub struct VadDetectionService;

impl VadDetectionService {
    pub fn get_vad_binary_path(app: &AppHandle) -> Result<PathBuf> {
        let scratch_dir = app.path().app_local_data_dir()?.join("scratch").join("whisper-bin").join("Release");
        let bin_path = scratch_dir.join("whisper-vad-speech-segments.exe");
        if bin_path.exists() {
            return Ok(bin_path);
        }
        anyhow::bail!("VAD binary not found at {:?}", bin_path);
    }

    pub fn get_vad_model_path(app: &AppHandle) -> Result<PathBuf> {
        let models_dir = ResourceManager::get_models_dir(app)?;
        let bin_path = models_dir.join("silero-vad-v5.bin");
        
        let catalog = ResourceManager::get_catalog();
        if let Some(item) = catalog.iter().find(|i| i.id == "silero-vad-v5") {
            if let Ok(ResourceState::Installed) = ResourceManager::get_resource_state(app, item) {
                return Ok(bin_path);
            }
        }
        
        anyhow::bail!("VAD model not installed. Please download it first.");
    }

    pub fn parse_vad_output(output: &str, intervals: &mut Vec<SpeechInterval>) {
        // Example: Speech segment 0: start = 13.00, end = 57.00
        if let Ok(re) = Regex::new(r"start = ([\d.]+),\s*end = ([\d.]+)") {
            for caps in re.captures_iter(output) {
                let start_cs: f64 = caps[1].parse().unwrap_or(0.0);
                let end_cs: f64 = caps[2].parse().unwrap_or(0.0);
                
                let start_ms = (start_cs * 10.0).round() as u64;
                let end_ms = (end_cs * 10.0).round() as u64;

                intervals.push(SpeechInterval { start_ms, end_ms });
            }
        }
    }

    pub fn invert_speech_intervals(speech: &[SpeechInterval], duration_ms: u64) -> Vec<NonSpeechInterval> {
        let mut non_speech = Vec::new();
        let mut last_end = 0;

        for s in speech {
            if s.start_ms > last_end {
                non_speech.push(NonSpeechInterval {
                    start_ms: last_end,
                    end_ms: s.start_ms,
                    reason: "non_speech".to_string(),
                });
            }
            last_end = s.end_ms;
        }

        if duration_ms > last_end {
            non_speech.push(NonSpeechInterval {
                start_ms: last_end,
                end_ms: duration_ms,
                reason: "non_speech".to_string(),
            });
        }

        non_speech
    }
}
