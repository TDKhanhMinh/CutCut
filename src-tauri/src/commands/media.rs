use sha2::{Digest, Sha256};
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
    let args = build_render_args(&project, input_path, output_path, None, None)?;
    spawn_render_job(app, args, duration_us).await
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
    let project = validated_project(project)?;
    let media = project.media.first().ok_or("No media in project")?;
    let duration_ms = positive_duration_us(media.metadata.duration_sec)? / 1_000;
    if end_ms > duration_ms {
        return Err("Preview range exceeds media duration".into());
    }
    let input_path = path_to_string(&canonical_existing_path(&media.path, "Input")?)?;
    let signature = generate_preview_signature(&project, start_ms, end_ms);
    let cache_dir = std::env::temp_dir().join("cutcut_preview_cache");
    std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    let output_path = cache_dir.join(format!("preview_{signature}.mp4"));
    let output_path_str = path_to_string(&output_path)?;
    if output_path.exists() {
        return Ok(PreviewResponse {
            job_id: None,
            cached_path: Some(output_path_str),
        });
    }
    let duration_ms = end_ms - start_ms;
    let args = build_render_args(
        &project,
        input_path,
        output_path_str,
        Some(start_ms),
        Some(duration_ms),
    )?;
    let job_id = spawn_render_job(app, args, duration_ms.saturating_mul(1_000)).await?;
    Ok(PreviewResponse {
        job_id: Some(job_id),
        cached_path: None,
    })
}

#[derive(serde::Serialize)]
pub struct PreviewResponse {
    pub job_id: Option<String>,
    pub cached_path: Option<String>,
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

fn build_render_args(
    project: &crate::models::project::Project,
    input_path: String,
    output_path: String,
    seek_start_ms: Option<u64>,
    seek_duration_ms: Option<u64>,
) -> Result<Vec<String>, String> {
    let mut cut_exprs = Vec::new();
    for action in &project.edit_plan.actions {
        if action.enabled && action.action_type == EditActionType::Cut {
            cut_exprs.push(format!(
                "between(t,{:.3},{:.3})",
                action.start_ms as f64 / 1000.0,
                action.end_ms as f64 / 1000.0
            ));
        }
    }
    let mut vf_filters = vec!["scale=-2:720".to_string()];
    let mut af_filters = Vec::new();
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
            let ass = crate::services::subtitle_generator::SubtitleGenerator::generate_ass_content(
                &project.caption_cues,
                style,
                &project.edit_plan,
                1280,
                720,
            );
            let ass_path =
                crate::services::subtitle_generator::SubtitleGenerator::write_temp_ass_file(&ass)?;
            let mut path = path_to_string(&ass_path)?.replace('\\', "/");
            if let Some(position) = path.find(':') {
                path.insert(position, '\\');
            }
            vf_filters.push(format!("ass='{path}'"));
        }
    }
    let mut args = vec!["-y".into()];
    if let Some(start_ms) = seek_start_ms {
        args.extend(["-ss".into(), format!("{:.3}", start_ms as f64 / 1000.0)]);
    }
    args.extend(["-i".into(), input_path]);
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
        "-progress".into(),
        "pipe:1".into(),
        output_path,
    ]);
    Ok(args)
}

async fn spawn_render_job(
    app: AppHandle,
    args: Vec<String>,
    duration_us: u64,
) -> Result<String, String> {
    let job_id = uuid::Uuid::new_v4().to_string();
    crate::services::media_job::spawn_ffmpeg_job(app, job_id.clone(), args, Some(duration_us))
        .await
        .map_err(|e| e.to_string())?;
    Ok(job_id)
}

fn generate_preview_signature(
    project: &crate::models::project::Project,
    start_ms: u64,
    end_ms: u64,
) -> String {
    let mut hasher = Sha256::new();
    for value in [
        serde_json::to_string(&project.edit_plan).unwrap_or_default(),
        serde_json::to_string(&project.captions).unwrap_or_default(),
        serde_json::to_string(&project.caption_cues).unwrap_or_default(),
        serde_json::to_string(&project.settings).unwrap_or_default(),
    ] {
        hasher.update(value.as_bytes());
    }
    hasher.update(start_ms.to_be_bytes());
    hasher.update(end_ms.to_be_bytes());
    hex::encode(hasher.finalize())
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
    use super::validate_export_paths;
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
}
