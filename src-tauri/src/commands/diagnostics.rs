use crate::models::hardware::RuntimeProfile;
use crate::models::preset::{PresetResolution, PresetType};
use crate::models::resource::ResourceState;
use crate::services::hardware_detection::HardwareDetectionService;
use crate::services::resource_manager::ResourceManager;
use crate::services::runtime_preset::RuntimePresetService;
use tauri::{command, AppHandle};

#[command]
pub fn get_runtime_profile() -> RuntimeProfile {
    HardwareDetectionService::detect_profile()
}

#[command]
pub fn resolve_runtime_preset(
    app: AppHandle,
    preset: PresetType,
    user_override: Option<String>,
) -> Result<PresetResolution, String> {
    let profile = HardwareDetectionService::detect_profile();
    let catalog = ResourceManager::get_catalog();

    let mut installed_model_ids = Vec::new();
    for item in catalog {
        if let Ok(ResourceState::Installed) = ResourceManager::get_resource_state(&app, &item) {
            installed_model_ids.push(item.id.clone());
        }
    }

    Ok(RuntimePresetService::resolve_preset(
        preset,
        &profile,
        &installed_model_ids,
        user_override,
    ))
}
