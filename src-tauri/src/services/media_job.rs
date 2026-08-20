use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use tokio::sync::Mutex;

use crate::engines::ffmpeg::MediaEngineError;
use crate::models::media_job::{MediaJobEvent, MediaJobState};

#[derive(Default, Clone)]
pub struct JobManager {
    jobs: Arc<Mutex<HashMap<String, CommandChild>>>,
}

impl JobManager {
    pub async fn add_job(&self, job_id: String, child: CommandChild) {
        let mut jobs = self.jobs.lock().await;
        jobs.insert(job_id, child);
    }

    pub async fn remove_job(&self, job_id: &str) -> Option<CommandChild> {
        let mut jobs = self.jobs.lock().await;
        jobs.remove(job_id)
    }

    pub async fn cancel_job(&self, job_id: &str) -> Result<(), String> {
        let mut jobs = self.jobs.lock().await;
        if let Some(child) = jobs.remove(job_id) {
            // Note: killing a child process might leave temp files.
            // Further cleanup can be added here if needed.
            child.kill().map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err(format!("Job {} not found or already completed", job_id))
        }
    }
}

/// Parse a single line from FFmpeg `-progress` output.
/// Example lines: "frame=123", "out_time_us=4100000", "progress=continue"
fn parse_progress_line(line: &str, total_duration_us: Option<u64>) -> Option<f64> {
    if line.starts_with("out_time_us=") {
        if let Some(value_str) = line.split('=').nth(1) {
            if let Ok(time_us) = value_str.trim().parse::<u64>() {
                if let Some(total) = total_duration_us {
                    if total > 0 {
                        let progress = (time_us as f64 / total as f64).clamp(0.0, 1.0);
                        return Some(progress);
                    }
                }
            }
        }
    }
    None
}

/// Spawn a long-running FFmpeg job and emit progress events.
pub async fn spawn_ffmpeg_job(
    app: AppHandle,
    job_id: String,
    args: Vec<String>,
    total_duration_us: Option<u64>,
) -> Result<(), MediaEngineError> {
    // We expect the arguments to NOT include `-progress pipe:1`
    // We will append it here to ensure we get machine-readable progress.
    let mut final_args = args;
    final_args.push("-progress".to_string());
    final_args.push("pipe:1".to_string());
    // Use `-v warning` or similar to reduce stderr spam if needed,
    // but the user's input `args` should handle the log level.

    let cmd = app
        .shell()
        .sidecar("ffmpeg")
        .map_err(|e| MediaEngineError::BinaryNotFound {
            binary: format!("ffmpeg: {}", e),
        })?
        .args(final_args);

    let (mut rx, child) = cmd.spawn().map_err(|e| MediaEngineError::SpawnFailed {
        message: format!("ffmpeg: {}", e),
    })?;

    // Store the child in JobManager
    let job_manager = app.state::<JobManager>();
    job_manager.add_job(job_id.clone(), child).await;

    // Emit started event
    let _ = app.emit(
        "media-job",
        MediaJobEvent {
            job_id: job_id.clone(),
            state: MediaJobState::Started,
            progress: Some(0.0),
            message: None,
            error: None,
        },
    );

    // Read events in a background task
    let app_clone = app.clone();
    let job_id_clone = job_id.clone();

    tokio::spawn(async move {
        let mut last_progress = 0.0;

        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    let line_str = String::from_utf8_lossy(&line);
                    if let Some(p) = parse_progress_line(&line_str, total_duration_us) {
                        // Only emit if progress increased by at least 1% to reduce IPC spam
                        if p - last_progress >= 0.01 || p == 1.0 {
                            last_progress = p;
                            let _ = app_clone.emit(
                                "media-job",
                                MediaJobEvent {
                                    job_id: job_id_clone.clone(),
                                    state: MediaJobState::Progress,
                                    progress: Some(p),
                                    message: None,
                                    error: None,
                                },
                            );
                        }
                    }
                }
                CommandEvent::Stderr(_line) => {
                    // For now, we don't spam stderr to the UI, but we could log it or
                    // capture the last few lines for error reporting.
                }
                CommandEvent::Terminated(payload) => {
                    // Remove from job manager
                    let job_manager = app_clone.state::<JobManager>();
                    job_manager.remove_job(&job_id_clone).await;

                    if payload.code == Some(0) {
                        let _ = app_clone.emit(
                            "media-job",
                            MediaJobEvent {
                                job_id: job_id_clone.clone(),
                                state: MediaJobState::Completed,
                                progress: Some(1.0),
                                message: None,
                                error: None,
                            },
                        );
                    } else if payload.code == Some(255) || payload.code.is_none() {
                        // In some OS/shells, kill/cancel produces 255 or None exit code
                        let _ = app_clone.emit(
                            "media-job",
                            MediaJobEvent {
                                job_id: job_id_clone.clone(),
                                state: MediaJobState::Cancelled,
                                progress: None,
                                message: Some("Job cancelled by user".to_string()),
                                error: None,
                            },
                        );
                    } else {
                        let _ = app_clone.emit(
                            "media-job",
                            MediaJobEvent {
                                job_id: job_id_clone.clone(),
                                state: MediaJobState::Failed,
                                progress: None,
                                message: None,
                                error: Some(format!("Process exited with code {:?}", payload.code)),
                            },
                        );
                    }
                }
                _ => {}
            }
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_progress_line() {
        // Valid out_time_us
        let res = parse_progress_line("out_time_us=5000000", Some(10_000_000));
        assert_eq!(res, Some(0.5));

        // Clamped to 1.0 if over duration
        let res = parse_progress_line("out_time_us=12000000", Some(10_000_000));
        assert_eq!(res, Some(1.0));

        // Invalid format
        let res = parse_progress_line("out_time_us=N/A", Some(10_000_000));
        assert_eq!(res, None);

        // Unknown key
        let res = parse_progress_line("frame=123", Some(10_000_000));
        assert_eq!(res, None);

        // Missing duration
        let res = parse_progress_line("out_time_us=5000000", None);
        assert_eq!(res, None);
    }
}
