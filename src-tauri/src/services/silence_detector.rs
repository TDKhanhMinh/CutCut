use crate::models::media_job::{MediaJobEvent, MediaJobState};
use crate::models::silence::{
    SilenceDetectionMetadata, SilenceDetectionResult, SilenceInterval, SilenceSettings,
    SILENCE_DETECTOR_VERSION,
};
use crate::services::media_job::JobManager;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

const TAIL_POLICY_FLUSH_TO_DURATION: &str = "flushToSourceDuration";
const TAIL_POLICY_DROP_WITHOUT_DURATION: &str = "dropWithoutSourceDuration";

fn detection_metadata(
    settings: &SilenceSettings,
    source_duration_ms: Option<u64>,
) -> SilenceDetectionMetadata {
    SilenceDetectionMetadata {
        detector_version: SILENCE_DETECTOR_VERSION.to_string(),
        threshold_db: settings.threshold_db,
        min_duration_ms: settings.min_duration_ms,
        padding_ms: settings.padding_ms,
        tail_policy: if source_duration_ms.is_some() {
            TAIL_POLICY_FLUSH_TO_DURATION.to_string()
        } else {
            TAIL_POLICY_DROP_WITHOUT_DURATION.to_string()
        },
    }
}

fn build_interval(
    raw_start_ms: u64,
    raw_end_ms: u64,
    settings: &SilenceSettings,
    source_duration_ms: Option<u64>,
) -> Option<SilenceInterval> {
    if raw_end_ms <= raw_start_ms {
        return None;
    }

    let start_ms = raw_start_ms.saturating_sub(settings.padding_ms);
    let mut end_ms = raw_end_ms.saturating_add(settings.padding_ms);
    if let Some(duration_ms) = source_duration_ms {
        end_ms = end_ms.min(duration_ms);
    }
    if end_ms <= start_ms {
        return None;
    }

    let detection = detection_metadata(settings, source_duration_ms);
    Some(SilenceInterval {
        id: format!("silence-{start_ms}-{end_ms}"),
        start_ms,
        end_ms,
        duration_ms: end_ms - start_ms,
        detection,
        measured_level_db: None,
    })
}

pub fn parse_silence_line(
    line: &str,
    current_start: &mut Option<f64>,
    intervals: &mut Vec<SilenceInterval>,
    settings: &SilenceSettings,
    source_duration_ms: Option<u64>,
) {
    if !line.contains("silencedetect") {
        return;
    }

    if let Some(idx) = line.find("silence_start: ") {
        let text = &line[idx + "silence_start: ".len()..];
        let num_str = text.split_whitespace().next().unwrap_or(text);
        if let Ok(value) = num_str.parse::<f64>() {
            if value.is_finite() && value >= 0.0 && current_start.is_none() {
                *current_start = Some(value);
            }
        }
        return;
    }

    if let Some(idx) = line.find("silence_end: ") {
        let text = &line[idx + "silence_end: ".len()..];
        let num_str = text
            .split_whitespace()
            .next()
            .unwrap_or(text)
            .trim_end_matches('|');
        if let Ok(value) = num_str.parse::<f64>() {
            if !value.is_finite() || value < 0.0 {
                return;
            }
            if let Some(start) = current_start.take() {
                let start_ms = (start * 1000.0).round() as u64;
                let end_ms = (value * 1000.0).round() as u64;
                if let Some(interval) =
                    build_interval(start_ms, end_ms, settings, source_duration_ms)
                {
                    intervals.push(interval);
                }
            }
        }
    }
}

fn flush_tail_silence(
    current_start: &mut Option<f64>,
    intervals: &mut Vec<SilenceInterval>,
    settings: &SilenceSettings,
    source_duration_ms: Option<u64>,
) {
    let Some(start) = current_start.take() else {
        return;
    };
    let Some(duration_ms) = source_duration_ms else {
        // Without source duration an unbalanced start cannot be assigned a
        // safe end. Drop it explicitly instead of inventing a cut range.
        return;
    };

    let start_ms = (start * 1000.0).round() as u64;
    if let Some(interval) = build_interval(start_ms, duration_ms, settings, Some(duration_ms)) {
        intervals.push(interval);
    }
}

#[tauri::command]
pub async fn start_silence_detection(
    app: AppHandle,
    job_id: String,
    path: String,
    settings: SilenceSettings,
    duration_ms: Option<u64>,
) -> Result<(), String> {
    if settings.min_duration_ms == 0 {
        return Err("Minimum silence duration must be greater than zero".to_string());
    }

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
        .map_err(|e| format!("Binary error: {e}"))?
        .args(args);

    let (mut rx, child) = cmd.spawn().map_err(|e| format!("Spawn error: {e}"))?;

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
            result: None,
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
                    parse_silence_line(
                        &line,
                        &mut current_start,
                        &mut intervals,
                        &settings,
                        duration_ms,
                    );
                }
                CommandEvent::Terminated(payload) => {
                    let job_manager = app_clone.state::<JobManager>();
                    let was_cancelled = job_manager.take_cancelled(&job_id_clone).await;
                    job_manager.remove_job(&job_id_clone).await;

                    if was_cancelled || payload.code == Some(255) || payload.code.is_none() {
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
                        flush_tail_silence(
                            &mut current_start,
                            &mut intervals,
                            &settings,
                            duration_ms,
                        );
                        let detection = detection_metadata(&settings, duration_ms);
                        let result = SilenceDetectionResult {
                            source_duration_ms: duration_ms,
                            detection,
                            intervals: std::mem::take(&mut intervals),
                        };
                        let result = serde_json::to_value(result).unwrap_or_else(|_| {
                            serde_json::json!({
                                "sourceDurationMs": duration_ms,
                                "intervals": [],
                            })
                        });
                        let _ = app_clone.emit(
                            "media-job",
                            MediaJobEvent {
                                job_id: job_id_clone.clone(),
                                state: MediaJobState::Completed,
                                progress: Some(1.0),
                                message: Some("Silence detection completed".to_string()),
                                error: None,
                                result: Some(result),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(padding_ms: u64) -> SilenceSettings {
        SilenceSettings {
            threshold_db: -35,
            min_duration_ms: 500,
            padding_ms,
        }
    }

    #[test]
    fn parses_balanced_interval_with_metadata_and_padding() {
        let settings = settings(100);
        let mut current_start = None;
        let mut intervals = Vec::new();

        parse_silence_line(
            "[silencedetect @ 0x123] silence_start: 1.25",
            &mut current_start,
            &mut intervals,
            &settings,
            Some(3_000),
        );
        parse_silence_line(
            "[silencedetect @ 0x123] silence_end: 2.25 | silence_duration: 1",
            &mut current_start,
            &mut intervals,
            &settings,
            Some(3_000),
        );

        assert_eq!(intervals.len(), 1);
        assert_eq!(intervals[0].start_ms, 1_150);
        assert_eq!(intervals[0].end_ms, 2_350);
        assert_eq!(intervals[0].detection.padding_ms, 100);
        assert_eq!(intervals[0].id, "silence-1150-2350");
    }

    #[test]
    fn flushes_tail_to_known_duration_and_clamps_padding() {
        let settings = settings(200);
        let mut current_start = Some(2.5);
        let mut intervals = Vec::new();

        flush_tail_silence(&mut current_start, &mut intervals, &settings, Some(3_000));

        assert_eq!(intervals.len(), 1);
        assert_eq!(intervals[0].start_ms, 2_300);
        assert_eq!(intervals[0].end_ms, 3_000);
        assert_eq!(
            intervals[0].detection.tail_policy,
            TAIL_POLICY_FLUSH_TO_DURATION
        );
        assert!(current_start.is_none());
    }

    #[test]
    fn drops_unbalanced_tail_without_duration_instead_of_inventing_end() {
        let settings = settings(0);
        let mut current_start = Some(2.5);
        let mut intervals = Vec::new();

        flush_tail_silence(&mut current_start, &mut intervals, &settings, None);

        assert!(intervals.is_empty());
        assert!(current_start.is_none());
    }
}
