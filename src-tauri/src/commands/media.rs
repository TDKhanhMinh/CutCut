use serde_json::json;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

use crate::engines::ffmpeg::{self, MediaBinaryInfo, MediaEngineError};
use crate::models::edit_plan::EditActionType;
use crate::services::edit_validator::{validate_and_normalize, IssueLevel};

#[tauri::command]
pub async fn check_media_engines(app: AppHandle) -> Result<Vec<MediaBinaryInfo>, MediaEngineError> {
    Ok(vec![
        ffmpeg::get_ffmpeg_version(&app).await?,
        ffmpeg::get_ffprobe_version(&app).await?,
    ])
}

#[tauri::command]
pub async fn get_ffmpeg_version(app: AppHandle) -> Result<MediaBinaryInfo, MediaEngineError> {
    ffmpeg::get_ffmpeg_version(&app).await
}

#[tauri::command]
pub async fn get_ffprobe_version(app: AppHandle) -> Result<MediaBinaryInfo, MediaEngineError> {
    ffmpeg::get_ffprobe_version(&app).await
}

#[tauri::command]
pub async fn cancel_media_job(app: AppHandle, job_id: String) -> Result<(), String> {
    app.state::<crate::services::media_job::JobManager>()
        .cancel_job(&job_id)
        .await
}

#[tauri::command]
pub async fn spawn_test_ffmpeg_job(app: AppHandle) -> Result<String, String> {
    let job_id = uuid::Uuid::new_v4().to_string();
    let args = vec![
        "-y".into(),
        "-f".into(),
        "lavfi".into(),
        "-i".into(),
        "testsrc=duration=5:size=1280x720:rate=30".into(),
        "-f".into(),
        "null".into(),
        "-".into(),
    ];
    crate::services::media_job::spawn_ffmpeg_job(app, job_id.clone(), args, Some(5_000_000))
        .await
        .map_err(|e| e.to_string())?;
    Ok(job_id)
}

#[tauri::command]
pub async fn read_media_metadata(
    app: AppHandle,
    path: String,
) -> Result<crate::models::media_info::MediaSourceMetadata, MediaEngineError> {
    crate::engines::ffmpeg::read_media_metadata(&app, path).await
}

#[tauri::command]
pub async fn export_prototype_video(
    app: AppHandle,
    project: crate::models::project::Project,
    output_path: String,
) -> Result<String, String> {
    let project = validated_project(project)?;
    let media = project.media.first().ok_or("No media in project")?;
    let (input_path, output_path) = validate_export_paths(&media.path, &output_path)?;
    let duration_us = positive_duration_us(media.metadata.duration_sec)?;
    let prepared = build_render_args(&project, input_path, output_path, None, None)?;
    spawn_render_job(app, prepared, duration_us).await
}

#[tauri::command]
pub async fn preview_prototype_video(
    app: AppHandle,
    project: crate::models::project::Project,
    start_ms: u64,
    end_ms: u64,
) -> Result<PreviewResponse, String> {
    if start_ms >= end_ms {
        return Err("Preview range must have end_ms > start_ms".into());
    }
    let requested_duration_ms = end_ms - start_ms;
    if !(3_000..=5_000).contains(&requested_duration_ms) {
        return Err("Accurate Preview range must be between 3 and 5 seconds".into());
    }
    let project = validated_project(project)?;
    let media = project.media.first().ok_or("No media in project")?;
    let duration_ms = positive_duration_us(media.metadata.duration_sec)? / 1_000;
    if end_ms > duration_ms {
        return Err("Preview range exceeds media duration".into());
    }
    let input_path = path_to_string(&canonical_existing_path(&media.path, "Input")?)?;
    let (signature, source_fingerprint) =
        generate_preview_signature(&project, &input_path, start_ms, end_ms)?;
    let cache_root = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    let cache_dir = cache_root.join("cutcut_preview_cache");
    std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    let output_path = cache_dir.join(format!("preview_{signature}.mp4"));
    let output_path_str = path_to_string(&output_path)?;
    let relative_path = Path::new("cutcut_preview_cache").join(format!("preview_{signature}.mp4"));
    let mut project_for_registry = project.clone();
    if let Some(record) = crate::services::artifact_registry::ArtifactRegistryService::resolve(
        &mut project_for_registry,
        &signature,
        &cache_root,
    ) {
        return Ok(PreviewResponse {
            job_id: None,
            cached_path: Some(output_path_str),
            artifact: Some(record),
        });
    }
    let duration_ms = end_ms - start_ms;
    let prepared = build_render_args(
        &project,
        input_path,
        output_path_str.clone(),
        Some(start_ms),
        Some(duration_ms),
    )?;
    let job_id = spawn_render_job(app, prepared, duration_ms.saturating_mul(1_000)).await?;
    Ok(PreviewResponse {
        job_id: Some(job_id),
        // The deterministic target path is returned before the job finishes;
        // it is safe for the UI to attach only after a Completed event.
        cached_path: Some(output_path_str),
        artifact: Some(crate::models::artifact_registry::ArtifactRecord {
            id: format!("preview-{signature}"),
            artifact_type: crate::models::artifact::ArtifactType::Preview,
            signature,
            relative_path: relative_path.to_string_lossy().replace('\\', "/"),
            created_at: now_ms(),
            artifact_version: 2,
            producer: "ffmpeg-accurate-preview".into(),
            status: crate::models::artifact_registry::ArtifactStatus::Building,
            dependencies: vec![source_fingerprint],
            integrity: None,
            diagnostic_reason: None,
        }),
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewResponse {
    pub job_id: Option<String>,
    pub cached_path: Option<String>,
    pub artifact: Option<crate::models::artifact_registry::ArtifactRecord>,
}

#[tauri::command]
pub fn finalize_preview_artifact(
    app: AppHandle,
    mut project: crate::models::project::Project,
    artifact: crate::models::artifact_registry::ArtifactRecord,
) -> Result<crate::models::artifact_registry::ArtifactRecord, String> {
    let cache_root = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    crate::services::artifact_registry::ArtifactRegistryService::register_completed(
        &mut project,
        artifact.clone(),
        cache_root,
    )
    .map_err(|e| e.to_string())?;
    project
        .artifacts
        .into_iter()
        .find(|record| record.id == artifact.id)
        .ok_or_else(|| "Preview artifact was not registered".to_string())
}

fn validated_project(
    mut project: crate::models::project::Project,
) -> Result<crate::models::project::Project, String> {
    let (plan, issues) = validate_and_normalize(project.edit_plan.clone(), &project.media);
    if let Some(issue) = issues.iter().find(|issue| issue.level == IssueLevel::Error) {
        return Err(format!("Project edit plan is invalid: {}", issue.message));
    }
    project.edit_plan = plan;
    Ok(project)
}

struct PreparedRender {
    args: Vec<String>,
    cleanup_paths: Vec<PathBuf>,
    failure_cleanup_paths: Vec<PathBuf>,
}

fn render_dimensions(settings: &crate::models::project::OutputSettings) -> (u32, u32) {
    let resolution = settings.target_resolution.clamp(240, 8_192);
    let (width, height) = match settings.aspect_ratio.as_str() {
        "9:16" => ((resolution as f64 * 9.0 / 16.0).round() as u32, resolution),
        "1:1" => (resolution, resolution),
        _ => ((resolution as f64 * 16.0 / 9.0).round() as u32, resolution),
    };
    (width.max(2) & !1, height.max(2) & !1)
}

fn build_render_args(
    project: &crate::models::project::Project,
    input_path: String,
    output_path: String,
    seek_start_ms: Option<u64>,
    seek_duration_ms: Option<u64>,
) -> Result<PreparedRender, String> {
    let preview_window = seek_start_ms
        .zip(seek_duration_ms)
        .map(|(start, duration)| {
            let end = start.saturating_add(duration);
            (start, end)
        });
    let mut cut_exprs = Vec::new();
    for action in &project.edit_plan.actions {
        if action.enabled && action.action_type == EditActionType::Cut {
            let (cut_start_ms, cut_end_ms) =
                if let Some((window_start, window_end)) = preview_window {
                    let clipped_start = action.start_ms.max(window_start);
                    let clipped_end = action.end_ms.min(window_end);
                    if clipped_start >= clipped_end {
                        continue;
                    }
                    (
                        clipped_start.saturating_sub(window_start),
                        clipped_end.saturating_sub(window_start),
                    )
                } else {
                    (action.start_ms, action.end_ms)
                };
            cut_exprs.push(format!(
                "between(t,{:.3},{:.3})",
                cut_start_ms as f64 / 1000.0,
                cut_end_ms as f64 / 1000.0
            ));
        }
    }
    let (video_width, video_height) = render_dimensions(&project.settings);
    let mut vf_filters = vec![format!("scale={video_width}:{video_height}")];
    let mut af_filters = Vec::new();
    let mut cleanup_paths = Vec::new();
    if let Some(duration_ms) = seek_duration_ms {
        // `-t` limits demuxing/output, but does not guarantee that a filtered
        // stream ends at the requested range when select/aselect removes
        // timestamps. Trim both streams in the planner so preview duration is
        // deterministic before the final `-shortest` mux decision.
        let duration_seconds = duration_ms as f64 / 1000.0;
        vf_filters.push(format!("trim=start=0:end={duration_seconds:.3}"));
        af_filters.push(format!("atrim=start=0:end={duration_seconds:.3}"));
    }
    if !cut_exprs.is_empty() {
        let select_expr = format!("not({})", cut_exprs.join("+"));
        vf_filters.extend([
            format!("select='{select_expr}'"),
            "setpts=N/FRAME_RATE/TB".into(),
        ]);
        af_filters.extend([format!("aselect='{select_expr}'"), "asetpts=N/SR/TB".into()]);
    }
    if let Some(style) = &project.captions {
        if !project.caption_cues.is_empty() {
            let ass = match (seek_start_ms, seek_duration_ms) {
                (Some(start_ms), Some(duration_ms)) => crate::services::subtitle_generator::SubtitleGenerator::generate_ass_content_for_range(
                    &project.caption_cues,
                    style,
                    &project.edit_plan,
                    video_width,
                    video_height,
                    start_ms,
                    start_ms.saturating_add(duration_ms),
                ),
                _ => crate::services::subtitle_generator::SubtitleGenerator::generate_ass_content(
                    &project.caption_cues,
                    style,
                    &project.edit_plan,
                    video_width,
                    video_height,
                ),
            };
            let ass_path =
                crate::services::subtitle_generator::SubtitleGenerator::write_temp_ass_file(&ass)?;
            let mut path = path_to_string(&ass_path)?.replace('\\', "/");
            if let Some(position) = path.find(':') {
                path.insert(position, '\\');
            }
            path = path.replace('\'', "\\'");
            vf_filters.push(format!("ass='{path}'"));
            cleanup_paths.push(ass_path);
        }
    }
    let failure_cleanup_paths = seek_duration_ms
        .is_some()
        .then(|| PathBuf::from(&output_path))
        .into_iter()
        .collect();
    let mut args = vec!["-y".into()];
    args.extend(["-i".into(), input_path]);
    // Place seek after input for frame-accurate preview. Export has no seek.
    if let Some(start_ms) = seek_start_ms {
        args.extend(["-ss".into(), format!("{:.3}", start_ms as f64 / 1000.0)]);
    }
    if !vf_filters.is_empty() {
        args.extend(["-vf".into(), vf_filters.join(",")]);
    }
    if !af_filters.is_empty() {
        args.extend(["-af".into(), af_filters.join(",")]);
    }
    if let Some(duration_ms) = seek_duration_ms {
        args.extend(["-t".into(), format!("{:.3}", duration_ms as f64 / 1000.0)]);
    }
    args.extend([
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        if seek_duration_ms.is_some() {
            "ultrafast".into()
        } else {
            "fast".into()
        },
        "-crf".into(),
        if seek_duration_ms.is_some() {
            "28".into()
        } else {
            "23".into()
        },
        "-c:a".into(),
        "aac".into(),
    ]);
    if seek_duration_ms.is_some() {
        args.push("-shortest".into());
    }
    args.push(output_path);
    Ok(PreparedRender {
        args,
        cleanup_paths,
        failure_cleanup_paths,
    })
}

async fn spawn_render_job(
    app: AppHandle,
    prepared: PreparedRender,
    duration_us: u64,
) -> Result<String, String> {
    let job_id = uuid::Uuid::new_v4().to_string();
    crate::services::media_job::spawn_ffmpeg_job_with_cleanup_policy(
        app,
        job_id.clone(),
        prepared.args,
        Some(duration_us),
        prepared.cleanup_paths,
        prepared.failure_cleanup_paths,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(job_id)
}

fn generate_preview_signature(
    project: &crate::models::project::Project,
    input_path: &str,
    start_ms: u64,
    end_ms: u64,
) -> Result<(String, String), String> {
    let source_fingerprint = crate::models::artifact::get_content_fingerprint(input_path)
        .map_err(|e| format!("Unable to fingerprint preview source: {e}"))?;
    let descriptor = crate::models::artifact::ArtifactSignature::new(
        crate::models::artifact::ArtifactType::Preview,
        2,
        vec![source_fingerprint.clone()],
        json!({
            "plannerVersion": 2,
            "captionRendererVersion": 2,
            "rangeStartMs": start_ms,
            "rangeEndMs": end_ms,
            "editPlan": project.edit_plan,
            "captions": project.captions,
            "captionCues": project.caption_cues,
            "settings": project.settings,
            "mediaMetadata": project.media.first().map(|media| &media.metadata),
        }),
    );
    Ok((descriptor.signature, source_fingerprint))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn positive_duration_us(duration_sec: f64) -> Result<u64, String> {
    if !duration_sec.is_finite() || duration_sec <= 0.0 {
        return Err("Video duration must be a positive finite value".into());
    }
    Ok((duration_sec * 1_000_000.0) as u64)
}

#[tauri::command]
pub async fn check_media_exists(path: String) -> Result<bool, String> {
    Ok(Path::new(&path).exists())
}

#[tauri::command]
pub async fn extract_audio_for_stt(
    app: AppHandle,
    source_path: String,
    job_id: Option<String>,
    duration_us: Option<u64>,
) -> Result<String, String> {
    crate::services::audio_extraction_service::AudioExtractionService::extract_audio_for_stt(
        app,
        source_path,
        job_id,
        duration_us,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cleanup_stt_audio(app: AppHandle, temp_path: String) -> Result<(), String> {
    crate::services::audio_extraction_service::AudioExtractionService::cleanup_stt_audio(
        &app, temp_path,
    )
    .map_err(|e| e.to_string())
}

fn validate_export_paths(input_path: &str, output_path: &str) -> Result<(String, String), String> {
    let input = canonical_existing_path(input_path, "Input")?;
    if input.is_dir() {
        return Err("Input path must be a media file, not a directory".into());
    }
    let output = canonical_output_path(output_path)?;
    if input == output {
        return Err("Export output must be different from the source media path".into());
    }
    if !output
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("mp4"))
    {
        return Err("Export output must use the .mp4 extension".into());
    }
    Ok((path_to_string(&input)?, path_to_string(&output)?))
}

fn canonical_existing_path(path: &str, label: &str) -> Result<PathBuf, String> {
    let path = Path::new(path);
    if path.as_os_str().is_empty() || !path.is_absolute() {
        return Err(format!("{label} path must be absolute"));
    }
    path.canonicalize()
        .map_err(|e| format!("{label} path is unavailable: {e}"))
}

fn canonical_output_path(path: &str) -> Result<PathBuf, String> {
    let path = Path::new(path);
    if path.as_os_str().is_empty() || !path.is_absolute() {
        return Err("Output path must be absolute".into());
    }
    if path.exists() {
        return path
            .canonicalize()
            .map_err(|e| format!("Output path is unavailable: {e}"));
    }
    let parent = path
        .parent()
        .ok_or("Output path must have a parent directory")?;
    let file_name = path
        .file_name()
        .ok_or("Output path must include a file name")?;
    Ok(parent
        .canonicalize()
        .map_err(|e| format!("Output directory is unavailable: {e}"))?
        .join(file_name))
}

fn path_to_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| "Path must be valid Unicode".into())
}

#[cfg(test)]
mod tests {
    use super::{build_render_args, generate_preview_signature, validate_export_paths};
    use crate::models::edit_plan::{EditAction, EditActionSource, EditActionType};
    use crate::models::media_info::MediaSourceMetadata;
    use crate::models::project::MediaSource;
    use crate::models::project::Project;
    use std::fs;
    #[test]
    fn rejects_export_that_would_overwrite_the_source() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.mp4");
        fs::write(&source, "source media").unwrap();
        let error =
            validate_export_paths(source.to_str().unwrap(), source.to_str().unwrap()).unwrap_err();
        assert!(error.contains("different from the source"));
    }

    #[test]
    fn preview_signature_changes_for_range_or_source_content() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.mp4");
        fs::write(&source, b"source-a").unwrap();
        let mut project = Project::default();
        project.media.push(MediaSource {
            id: "media-1".into(),
            path: source.to_string_lossy().into_owned(),
            metadata: MediaSourceMetadata {
                path: source.to_string_lossy().into_owned(),
                duration_sec: 10.0,
                fps: 30.0,
                width: 1920,
                height: 1080,
                video_codec: "h264".into(),
                audio_codec: Some("aac".into()),
                rotation: 0,
            },
        });
        let first =
            generate_preview_signature(&project, source.to_str().unwrap(), 0, 3_000).unwrap();
        let second =
            generate_preview_signature(&project, source.to_str().unwrap(), 1_000, 4_000).unwrap();
        assert_ne!(first.0, second.0);
        fs::write(&source, b"source-b").unwrap();
        let third =
            generate_preview_signature(&project, source.to_str().unwrap(), 0, 3_000).unwrap();
        assert_ne!(first.0, third.0);
    }

    #[test]
    fn preview_cut_ranges_are_relative_to_the_seek_window() {
        let mut project = Project::default();
        project.edit_plan.actions.push(EditAction {
            id: "cut-5-6".into(),
            source_media_id: "media-1".into(),
            action_type: EditActionType::Cut,
            start_ms: 5_000,
            end_ms: 6_000,
            source: EditActionSource::User,
            reason: "fixture".into(),
            confidence: None,
            enabled: true,
            is_manual_modified: None,
            created_at: 0,
            updated_at: 0,
            payload: None,
        });

        let prepared = build_render_args(
            &project,
            "C:/source.mp4".into(),
            "C:/preview.mp4".into(),
            Some(4_000),
            Some(4_000),
        )
        .expect("preview planner should accept a bounded range");
        assert!(prepared
            .args
            .windows(2)
            .any(|pair| pair[0] == "-vf" && pair[1].contains("between(t,1.000,2.000)")));
        assert!(prepared
            .args
            .windows(2)
            .any(|pair| pair[0] == "-vf" && pair[1].contains("trim=start=0:end=4.000")));
        assert!(prepared.args.iter().any(|arg| arg == "-shortest"));
    }
}
