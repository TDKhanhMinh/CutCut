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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
