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
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::media::check_media_engines,
            commands::media::get_ffmpeg_version,
            commands::media::get_ffprobe_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
