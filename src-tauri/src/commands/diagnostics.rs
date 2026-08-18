use tauri::command;
use crate::models::hardware::RuntimeProfile;
use crate::services::hardware_detection::HardwareDetectionService;

#[command]
pub fn get_runtime_profile() -> RuntimeProfile {
    HardwareDetectionService::detect_profile()
}
