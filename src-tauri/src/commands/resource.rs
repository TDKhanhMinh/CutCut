use crate::models::resource::{ResourceItem, ResourceState};
use crate::services::resource_manager::{ResourceJobManager, ResourceManager};
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_models() -> Vec<ResourceItem> {
    ResourceManager::get_catalog()
}

#[tauri::command]
pub fn get_model_state(app: AppHandle, id: String) -> Result<ResourceState, String> {
    let item = ResourceManager::find_catalog_item(&id).map_err(|e| e.to_string())?;
    ResourceManager::get_resource_state(&app, &item).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    id: String,
    jobs: State<'_, ResourceJobManager>,
) -> Result<(), String> {
    ResourceManager::download_resource(app, id, jobs.inner().clone())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cancel_model_download(
    id: String,
    jobs: State<'_, ResourceJobManager>,
) -> Result<(), String> {
    if jobs.cancel(&id).await {
        Ok(())
    } else {
        Err(format!("No active download for `{id}`"))
    }
}

#[tauri::command]
pub async fn delete_model(
    app: AppHandle,
    id: String,
    jobs: State<'_, ResourceJobManager>,
) -> Result<(), String> {
    ResourceManager::delete_resource(&app, id, jobs.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_active_model(app: AppHandle) -> Result<Option<String>, String> {
    ResourceManager::get_active_model(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_active_model(app: AppHandle, id: String) -> Result<(), String> {
    ResourceManager::set_active_model(&app, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_resource_usage(app: AppHandle) -> Result<u64, String> {
    ResourceManager::get_resource_usage(&app).map_err(|e| e.to_string())
}
