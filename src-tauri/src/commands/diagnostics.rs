use crate::models::hardware::RuntimeProfile;
use crate::models::preset::{PresetResolution, PresetType};
use crate::models::resource::ResourceState;
use crate::services::hardware_detection::RuntimeProfileCache;
use crate::services::resource_manager::ResourceManager;
use crate::services::runtime_preset::{RuntimePresetPreference, RuntimePresetService};
use tauri::{command, AppHandle, State};

#[command]
pub async fn get_runtime_profile(
    app: AppHandle,
    cache: State<'_, RuntimeProfileCache>,
    refresh: Option<bool>,
) -> Result<RuntimeProfile, String> {
    Ok(cache.get_or_detect(&app, refresh.unwrap_or(false)).await)
}

#[command]
pub async fn resolve_runtime_preset(
    app: AppHandle,
    cache: State<'_, RuntimeProfileCache>,
    preset: PresetType,
    user_override: Option<String>,
) -> Result<PresetResolution, String> {
    let profile = cache.get_or_detect(&app, false).await;
    let catalog = ResourceManager::get_catalog();

    let mut installed_model_ids = Vec::new();
    if profile.runtime_available {
        for item in catalog {
            if let Ok(ResourceState::Installed) = ResourceManager::get_resource_state(&app, &item) {
                installed_model_ids.push(item.id.clone());
            }
        }
    }

    Ok(RuntimePresetService::resolve_preset(
        preset,
        &profile,
        &installed_model_ids,
        user_override,
    ))
}

#[command]
pub fn get_runtime_preset_preference(app: AppHandle) -> Result<RuntimePresetPreference, String> {
    RuntimePresetService::load_preference(&app)
}

#[command]
pub fn set_runtime_preset_preference(
    app: AppHandle,
    preset: PresetType,
    user_override_model: Option<String>,
) -> Result<RuntimePresetPreference, String> {
    RuntimePresetService::save_preference(
        &app,
        RuntimePresetPreference {
            schema_version: crate::services::runtime_preset::PRESET_CONFIG_VERSION,
            preset,
            user_override_model,
        },
    )
}
