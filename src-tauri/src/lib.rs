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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::media::check_media_engines,
            commands::media::get_ffmpeg_version,
            commands::media::get_ffprobe_version,
            commands::media::cancel_media_job,
            commands::media::spawn_test_ffmpeg_job,
            commands::media::read_media_metadata,
            commands::media::export_prototype_video,
            commands::project::create_project,
            commands::project::save_project_to_disk,
            commands::project::load_project_from_disk,
            commands::media::check_media_exists,
            commands::media::extract_audio_for_stt,
            commands::media::cleanup_stt_audio,
            commands::whisper::transcribe_audio,
            commands::resource::get_models,
            commands::resource::get_model_state,
            commands::resource::download_model,
            commands::resource::delete_model,
            commands::resource::get_active_model,
            commands::resource::set_active_model,
        ])
        .setup(|app| {
            let _ = crate::services::audio_extraction_service::AudioExtractionService::cleanup_stale_audio(app.handle());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
