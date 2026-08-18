use tauri::{AppHandle, Manager};

use crate::engines::ffmpeg::{self, MediaBinaryInfo, MediaEngineError};

/// Tauri command: Check FFmpeg and FFprobe versions.
/// Returns version info for both binaries.
#[tauri::command]
pub async fn check_media_engines(
    app: AppHandle,
) -> Result<Vec<MediaBinaryInfo>, MediaEngineError> {
    let ffmpeg = ffmpeg::get_ffmpeg_version(&app).await?;
    let ffprobe = ffmpeg::get_ffprobe_version(&app).await?;
    Ok(vec![ffmpeg, ffprobe])
}

/// Tauri command: Get FFmpeg version only.
#[tauri::command]
pub async fn get_ffmpeg_version(
    app: AppHandle,
) -> Result<MediaBinaryInfo, MediaEngineError> {
    ffmpeg::get_ffmpeg_version(&app).await
}

/// Tauri command: Get FFprobe version only.
#[tauri::command]
pub async fn get_ffprobe_version(
    app: AppHandle,
) -> Result<MediaBinaryInfo, MediaEngineError> {
    ffmpeg::get_ffprobe_version(&app).await
}

/// Tauri command: Cancel an ongoing media job.
#[tauri::command]
pub async fn cancel_media_job(app: AppHandle, job_id: String) -> Result<(), String> {
    let job_manager = app.state::<crate::services::media_job::JobManager>();
    job_manager.cancel_job(&job_id).await
}

/// Tauri command: Spawn a test FFmpeg job to verify progress events.
/// This generates a 5-second test video.
#[tauri::command]
pub async fn spawn_test_ffmpeg_job(app: AppHandle) -> Result<String, String> {
    let job_id = uuid::Uuid::new_v4().to_string();
    
    // Create a 5-second test video using lavfi
    let args = vec![
        "-y".to_string(),
        "-f".to_string(), "lavfi".to_string(),
        "-i".to_string(), "testsrc=duration=5:size=1280x720:rate=30".to_string(),
        // We will output to null to just test the process and progress
        "-f".to_string(), "null".to_string(), "-".to_string(),
    ];

    // Total duration is 5 seconds = 5,000,000 microseconds
    let total_duration_us = Some(5_000_000);

    crate::services::media_job::spawn_ffmpeg_job(app, job_id.clone(), args, total_duration_us)
        .await
        .map_err(|e| e.to_string())?;

    Ok(job_id)
}

/// Tauri command: Use FFprobe to read metadata of a local video file.
#[tauri::command]
pub async fn read_media_metadata(
    app: AppHandle,
    path: String,
) -> Result<crate::models::media_info::MediaSourceMetadata, MediaEngineError> {
    crate::engines::ffmpeg::read_media_metadata(&app, path).await
}

/// Tauri command: Export a video based on a Project's EditPlan and Captions
#[tauri::command]
pub async fn export_prototype_video(
    app: AppHandle,
    project: crate::models::project::Project,
    output_path: String,
) -> Result<String, String> {
    let job_id = uuid::Uuid::new_v4().to_string();
    
    // We assume the first media is the primary video. 
    // In V1, there is only one source media.
    let media = project.media.first().ok_or("No media in project")?;
    let input_path = media.path.clone();

    // 1. Build Cut Filter Strings
    let mut cut_exprs = Vec::new();
    for action in &project.edit_plan.actions {
        if action.enabled {
            if let crate::models::edit_plan::ActionPayload::Cut { start_ms, end_ms } = action.payload {
                let start_s = start_ms as f64 / 1000.0;
                let end_s = end_ms as f64 / 1000.0;
                cut_exprs.push(format!("between(t,{},{})", start_s, end_s));
            }
        }
    }

    let select_expr = if cut_exprs.is_empty() {
        "".to_string()
    } else {
        format!("not({})", cut_exprs.join("+"))
    };

    // 2. Build Subtitle ASS File
    let mut vf_filters = vec!["scale=-2:720".to_string()];
    let mut af_filters = Vec::new();

    if !select_expr.is_empty() {
        vf_filters.push(format!("select='{}'", select_expr));
        vf_filters.push("setpts=N/FRAME_RATE/TB".to_string());
        
        af_filters.push(format!("aselect='{}'", select_expr));
        af_filters.push("asetpts=N/SR/TB".to_string());
    }

    // Generate ASS Subtitles
    if let Some(style) = &project.captions {
        let cues = &project.caption_cues;
        if !cues.is_empty() {
            // Assume 720p output height for caption generation. Width is auto (-2) but let's assume 1280 for 16:9 for ASS scaling
            // ASS scaling is relative, so 1280x720 is a good baseline.
            let ass_content = crate::services::subtitle_generator::SubtitleGenerator::generate_ass_content(
                cues,
                style,
                &project.edit_plan,
                1280,
                720,
            );
            
            let ass_path = crate::services::subtitle_generator::SubtitleGenerator::write_temp_ass_file(&ass_content)?;
            let mut path_str = ass_path.to_string_lossy().to_string().replace("\\", "/");
            // Escape colon for FFmpeg filter syntax (C:/ -> C\:/)
            if let Some(pos) = path_str.find(':') {
                path_str.insert(pos, '\\');
            }
            
            vf_filters.push(format!("ass='{}'", path_str));
        }
    }

    let mut args = vec![
        "-y".to_string(),
        "-i".to_string(),
        input_path,
    ];

    if !vf_filters.is_empty() {
        args.push("-vf".to_string());
        args.push(vf_filters.join(","));
    }

    if !af_filters.is_empty() {
        args.push("-af".to_string());
        args.push(af_filters.join(","));
    }

    // Encoding options
    args.extend_from_slice(&[
        "-c:v".to_string(),
        "libx264".to_string(),
        "-preset".to_string(),
        "fast".to_string(),
        "-crf".to_string(),
        "23".to_string(),
        "-c:a".to_string(),
        "aac".to_string(),
        "-progress".to_string(),
        "pipe:1".to_string(),
        output_path,
    ]);

    // Duration is used for progress calculation. We should use the *output* duration ideally, 
    // but source duration is fine for an approximate progress bar.
    let total_duration_us = (media.metadata.duration_sec * 1_000_000.0) as u64;

    crate::services::media_job::spawn_ffmpeg_job(app, job_id.clone(), args, Some(total_duration_us))
        .await
        .map_err(|e| e.to_string())?;

    Ok(job_id)
}

/// Tauri command: Check if a media file exists at the given path.
#[tauri::command]
pub async fn check_media_exists(path: String) -> Result<bool, String> {
    Ok(std::path::Path::new(&path).exists())
}

#[tauri::command]
pub async fn extract_audio_for_stt(
    app: AppHandle,
    source_path: String,
    job_id: String,
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

