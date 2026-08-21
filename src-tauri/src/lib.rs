pub mod commands;
pub mod engines;
pub mod models;
pub mod services;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(services::media_job::JobManager::default())
        .manage(services::auth_session::AuthSessionStore::default())
        .manage(services::resource_manager::ResourceJobManager::default())
        .manage(services::hardware_detection::RuntimeProfileCache::default())
        .manage(services::project_repository::ProjectSaveCoordinator::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::cache::get_cache_usage,
            commands::cache::clear_project_cache,
            commands::media::check_media_engines,
            commands::media::get_ffmpeg_version,
            commands::media::get_ffprobe_version,
            commands::media::cancel_media_job,
            commands::media::spawn_test_ffmpeg_job,
            commands::media::read_media_metadata,
            commands::media::export_prototype_video,
            commands::media::preview_prototype_video,
            commands::media::finalize_preview_artifact,
            commands::auth::set_auth_session,
            commands::auth::get_auth_session,
            commands::auth::clear_auth_session,
            commands::project::create_project,
            commands::project::validate_edit_plan,
            commands::project::save_project_to_disk,
            commands::project::load_project_from_disk,
            commands::media::check_media_exists,
            commands::media::extract_audio_for_stt,
            commands::media::cleanup_stt_audio,
            commands::caption::generate_caption_cues,
            commands::filler::detect_filler_candidates,
            commands::whisper::transcribe_audio,
            commands::resource::get_models,
            commands::resource::get_model_state,
            commands::resource::download_model,
            commands::resource::cancel_model_download,
            commands::resource::delete_model,
            commands::resource::get_active_model,
            commands::resource::set_active_model,
            commands::resource::get_resource_usage,
            commands::whisper::check_whisper_runtime,
            commands::diagnostics::get_runtime_profile,
            commands::diagnostics::resolve_runtime_preset,
            commands::diagnostics::get_runtime_preset_preference,
            commands::diagnostics::set_runtime_preset_preference,
            services::silence_detector::start_silence_detection,
            commands::vad::start_vad_analysis,
            commands::fusion::fuse_non_speech_intervals,
            commands::suggestion::generate_cut_suggestions,
            commands::auth::set_secure_token,
            commands::auth::get_secure_token,
            commands::auth::delete_secure_token,
            commands::device::get_or_create_installation_id,
        ])
        .setup(|app| {
            let _ = crate::services::audio_extraction_service::AudioExtractionService::cleanup_stale_audio(app.handle());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
