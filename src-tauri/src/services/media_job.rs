use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use tokio::sync::Mutex;

use crate::engines::ffmpeg::MediaEngineError;
use crate::models::media_job::{MediaJobEvent, MediaJobState};

pub enum JobChild {
    Tauri(CommandChild),
    Tokio(Box<tokio::process::Child>),
}

impl JobChild {
    pub async fn kill(self) -> Result<(), String> {
        match self {
            JobChild::Tauri(c) => c.kill().map_err(|e| e.to_string()),
            JobChild::Tokio(mut c) => c.kill().await.map_err(|e| e.to_string()),
        }
    }
}

#[derive(Default, Clone)]
pub struct JobManager {
    jobs: Arc<Mutex<HashMap<String, JobChild>>>,
    cancelled: Arc<Mutex<HashSet<String>>>,
}

impl JobManager {
    pub async fn add_job(&self, job_id: String, child: JobChild) {
        let mut jobs = self.jobs.lock().await;
        jobs.insert(job_id, child);
    }

    pub async fn remove_job(&self, job_id: &str) -> Option<JobChild> {
        let mut jobs = self.jobs.lock().await;
        jobs.remove(job_id)
    }

    pub async fn cancel_job(&self, job_id: &str) -> Result<(), String> {
        self.cancelled.lock().await.insert(job_id.to_string());
        let mut jobs = self.jobs.lock().await;
        if let Some(child) = jobs.remove(job_id) {
            // Note: killing a child process might leave temp files.
            // Further cleanup can be added here if needed.
            child.kill().await?;
            Ok(())
        } else {
            drop(jobs);
            self.cancelled.lock().await.remove(job_id);
            Err(format!("Job {} not found or already completed", job_id))
        }
    }

    pub async fn take_cancelled(&self, job_id: &str) -> bool {
        self.cancelled.lock().await.remove(job_id)
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
    job_manager
        .add_job(job_id.clone(), JobChild::Tauri(child))
        .await;

    // Emit started event
    let _ = app.emit(
        "media-job",
        MediaJobEvent {
            job_id: job_id.clone(),
            state: MediaJobState::Started,
            progress: Some(0.0),
            message: None,
            error: None,
            result: None,
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
                                    result: None,
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
                    let was_cancelled = job_manager.take_cancelled(&job_id_clone).await;
                    job_manager.remove_job(&job_id_clone).await;

                    if was_cancelled {
                        let _ = app_clone.emit(
                            "media-job",
                            MediaJobEvent {
                                job_id: job_id_clone.clone(),
                                state: MediaJobState::Cancelled,
                                progress: None,
                                message: Some("Job cancelled by user".to_string()),
                                error: None,
                                result: None,
                            },
                        );
                    } else if payload.code == Some(0) {
                        let _ = app_clone.emit(
                            "media-job",
                            MediaJobEvent {
                                job_id: job_id_clone.clone(),
                                state: MediaJobState::Completed,
                                progress: Some(1.0),
                                message: None,
                                error: None,
                                result: None,
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
                                result: None,
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
                                result: None,
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

/// Spawn FFmpeg and wait until the process reaches a terminal state.
///
/// The regular `spawn_ffmpeg_job` command is intentionally fire-and-forget for
/// UI jobs such as export. Audio extraction is different: its caller must not
/// receive a path before FFmpeg has finished writing the WAV file. This helper
/// keeps the same progress/cancellation event contract while awaiting the
/// terminal event.
pub async fn spawn_ffmpeg_job_and_wait(
    app: AppHandle,
    job_id: String,
    args: Vec<String>,
    total_duration_us: Option<u64>,
) -> Result<(), MediaEngineError> {
    let mut final_args = args;
    final_args.push("-progress".to_string());
    final_args.push("pipe:1".to_string());

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

    let job_manager = app.state::<JobManager>();
    job_manager
        .add_job(job_id.clone(), JobChild::Tauri(child))
        .await;
    let _ = app.emit(
        "media-job",
        MediaJobEvent {
            job_id: job_id.clone(),
            state: MediaJobState::Started,
            progress: Some(0.0),
            message: None,
            error: None,
            result: None,
        },
    );

    let mut last_progress = 0.0;
    let mut last_stderr = String::new();
    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(line) => {
                let line_str = String::from_utf8_lossy(&line);
                if let Some(progress) = parse_progress_line(&line_str, total_duration_us) {
                    if progress - last_progress >= 0.01 || progress == 1.0 {
                        last_progress = progress;
                        let _ = app.emit(
                            "media-job",
                            MediaJobEvent {
                                job_id: job_id.clone(),
                                state: MediaJobState::Progress,
                                progress: Some(progress),
                                message: None,
                                error: None,
                                result: None,
                            },
                        );
                    }
                }
            }
            CommandEvent::Stderr(line) => {
                last_stderr = String::from_utf8_lossy(&line).trim().to_string();
            }
            CommandEvent::Terminated(payload) => {
                let was_cancelled = job_manager.take_cancelled(&job_id).await;
                job_manager.remove_job(&job_id).await;
                if was_cancelled {
                    let _ = app.emit(
                        "media-job",
                        MediaJobEvent {
                            job_id,
                            state: MediaJobState::Cancelled,
                            progress: None,
                            message: Some("Job cancelled by user".to_string()),
                            error: None,
                            result: None,
                        },
                    );
                    return Err(MediaEngineError::Cancelled);
                }

                if payload.code == Some(0) {
                    let _ = app.emit(
                        "media-job",
                        MediaJobEvent {
                            job_id,
                            state: MediaJobState::Completed,
                            progress: Some(1.0),
                            message: None,
                            error: None,
                            result: None,
                        },
                    );
                    return Ok(());
                }

                if payload.code == Some(255) || payload.code.is_none() {
                    let _ = app.emit(
                        "media-job",
                        MediaJobEvent {
                            job_id,
                            state: MediaJobState::Cancelled,
                            progress: None,
                            message: Some("Job cancelled by user".to_string()),
                            error: None,
                            result: None,
                        },
                    );
                    return Err(MediaEngineError::Cancelled);
                }

                let code = payload.code.unwrap_or(-1);
                let error = if last_stderr.is_empty() {
                    format!("Process exited with code {code}")
                } else {
                    format!("Process exited with code {code}: {last_stderr}")
                };
                let _ = app.emit(
                    "media-job",
                    MediaJobEvent {
                        job_id,
                        state: MediaJobState::Failed,
                        progress: None,
                        message: None,
                        error: Some(error.clone()),
                        result: None,
                    },
                );
                return Err(MediaEngineError::ProcessFailed {
                    code,
                    stderr: error,
                });
            }
            _ => {}
        }
    }

    job_manager.remove_job(&job_id).await;
    Err(MediaEngineError::ProcessFailed {
        code: -1,
        stderr: "FFmpeg ended without a termination event".to_string(),
    })
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
