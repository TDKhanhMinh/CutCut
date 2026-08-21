use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Once;
use tauri::{AppHandle, Manager};

static INSTALL_HOOK: Once = Once::new();

pub fn install_panic_hook(app: &AppHandle) -> Result<(), String> {
    let crash_path = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("crash.log");
    if let Some(parent) = crash_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    INSTALL_HOOK.call_once(move || {
        let path: PathBuf = crash_path;
        std::panic::set_hook(Box::new(move |panic_info| {
            let location = panic_info
                .location()
                .map(|value| format!("{}:{}", value.file(), value.line()))
                .unwrap_or_else(|| "unknown".into());
            let payload = panic_info
                .payload()
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| {
                    panic_info
                        .payload()
                        .downcast_ref::<String>()
                        .map(String::as_str)
                })
                .unwrap_or("panic");
            // Only a static panic code/location is persisted. Never write
            // arbitrary payloads, paths, transcript text or credentials.
            let safe_payload = payload
                .chars()
                .filter(|character| {
                    character.is_ascii_alphanumeric() || *character == '_' || *character == '-'
                })
                .take(120)
                .collect::<String>();
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
                let _ = writeln!(file, "panic;location={location};code={safe_payload}");
            }
        }));
    });
    Ok(())
}
