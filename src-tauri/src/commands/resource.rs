use tauri::AppHandle;
use crate::models::resource::{ResourceItem, ResourceState};
use crate::services::resource_manager::ResourceManager;

#[tauri::command]
pub fn get_models() -> Vec<ResourceItem> {
    ResourceManager::get_catalog()
}

#[tauri::command]
pub fn get_model_state(app: AppHandle, item: ResourceItem) -> Result<ResourceState, String> {
    ResourceManager::get_resource_state(&app, &item).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn download_model(app: AppHandle, id: String) -> Result<(), String> {
    ResourceManager::download_resource(app, id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_model(app: AppHandle, id: String) -> Result<(), String> {
    ResourceManager::delete_resource(&app, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_active_model(app: AppHandle) -> Result<Option<String>, String> {
    ResourceManager::get_active_model(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_active_model(app: AppHandle, id: String) -> Result<(), String> {
    ResourceManager::set_active_model(&app, id).map_err(|e| e.to_string())
}
