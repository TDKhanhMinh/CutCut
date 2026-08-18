use crate::engines::ffmpeg::MediaEngineError;
use crate::models::media_job::{MediaJobEvent, MediaJobState};
use crate::models::silence::{SilenceInterval, SilenceSettings};
use crate::services::media_job::JobManager;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

pub fn parse_silence_line(
    line: &str,
    current_start: &mut Option<f64>,
    intervals: &mut Vec<SilenceInterval>,
) {
    if !line.contains("silencedetect") {
        return;
    }
    if let Some(idx) = line.find("silence_start: ") {
        let text = &line[idx + "silence_start: ".len()..];
        let num_str = text.split_whitespace().next().unwrap_or(text);
        if let Ok(val) = num_str.parse::<f64>() {
            *current_start = Some(val);
        }
    } else if let Some(idx) = line.find("silence_end: ") {
        let text = &line[idx + "silence_end: ".len()..];
        let num_str = text
            .split_whitespace()
            .next()
            .unwrap_or(text)
            .trim_end_matches('|');
        if let Ok(val) = num_str.parse::<f64>() {
            if let Some(start) = *current_start {
                let start_ms = (start * 1000.0) as u64;
                let end_ms = (val * 1000.0) as u64;
                intervals.push(SilenceInterval {
                    id: uuid::Uuid::new_v4().to_string(),
                    start_ms,
                    end_ms,
                    duration_ms: end_ms.saturating_sub(start_ms),
                });
                *current_start = None;
            }
        }
    }
}

#[tauri::command]
pub async fn start_silence_detection(
    app: AppHandle,
    job_id: String,
    path: String,
    settings: SilenceSettings,
) -> Result<(), String> {
    let args = vec![
        "-i".to_string(),
        path,
        "-af".to_string(),
        format!(
            "silencedetect=noise={}dB:d={}",
            settings.threshold_db,
            settings.min_duration_ms as f64 / 1000.0
        ),
        "-f".to_string(),
        "null".to_string(),
        "-".to_string(),
    ];

    let cmd = app
        .shell()
        .sidecar("ffmpeg")
        .map_err(|e| format!("Binary error: {}", e))?
        .args(args);

    let (mut rx, child) = cmd.spawn().map_err(|e| format!("Spawn error: {}", e))?;

    let job_manager = app.state::<JobManager>();
    job_manager.add_job(job_id.clone(), child).await;

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

    let app_clone = app.clone();
    let job_id_clone = job_id.clone();

    tokio::spawn(async move {
        let mut current_start: Option<f64> = None;
        let mut intervals = Vec::new();

        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stderr(line_bytes) => {
                    let line = String::from_utf8_lossy(&line_bytes);
                    parse_silence_line(&line, &mut current_start, &mut intervals);
                }
                CommandEvent::Terminated(payload) => {
                    let job_manager = app_clone.state::<JobManager>();
                    job_manager.remove_job(&job_id_clone).await;

                    if payload.code == Some(0) {
                        // Trả kết quả JSON về UI thông qua field message của sự kiện Completed
                        let intervals_json = serde_json::to_string(&intervals).unwrap_or_default();
                        let _ = app_clone.emit(
                            "media-job",
                            MediaJobEvent {
                                job_id: job_id_clone.clone(),
                                state: MediaJobState::Completed,
                                progress: Some(1.0),
                                message: Some(intervals_json),
                                error: None,
                            },
                        );
                    } else if payload.code == Some(255) || payload.code.is_none() {
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
    fn test_parse_silence_line() {
        let mut current_start = None;
        let mut intervals = Vec::new();

        let line1 = "[silencedetect @ 0x123] silence_start: 1.25";
        parse_silence_line(line1, &mut current_start, &mut intervals);
        assert_eq!(current_start, Some(1.25));

        let line2 = "[silencedetect @ 0x123] silence_end: 2.25 | silence_duration: 1";
        parse_silence_line(line2, &mut current_start, &mut intervals);
        assert_eq!(current_start, None);
        assert_eq!(intervals.len(), 1);
        assert_eq!(intervals[0].start_ms, 1250);
        assert_eq!(intervals[0].end_ms, 2250);
        assert_eq!(intervals[0].duration_ms, 1000);
    }
}
