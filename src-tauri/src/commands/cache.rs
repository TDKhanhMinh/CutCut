use crate::models::project::Project;
use crate::services::cache_cleanup::CacheCleanupService;
use crate::services::project_repository::{save_project, ProjectSaveCoordinator};
use serde::Serialize;
use std::path::Path;
use tauri::State;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheUsageResponse {
    pub reclaimable_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheClearResponse {
    pub project: Project,
    pub freed_bytes: u64,
}

fn project_root(project_path: &str) -> Result<&Path, String> {
    let path = Path::new(project_path);
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| "Project path has no parent directory".to_string())
}

#[tauri::command]
pub fn get_cache_usage(
    project_path: String,
    project: Project,
) -> Result<CacheUsageResponse, String> {
    let root = project_root(&project_path)?;
    Ok(CacheUsageResponse {
        reclaimable_bytes: CacheCleanupService::calculate_reclaimable_size(&project, root),
    })
}

#[tauri::command]
pub async fn clear_project_cache(
    project_path: String,
    mut project: Project,
    coordinator: State<'_, ProjectSaveCoordinator>,
) -> Result<CacheClearResponse, String> {
    let root = project_root(&project_path)?;
    let freed_bytes = CacheCleanupService::clear_recomputable_cache(&mut project, root)
        .map_err(|error| error.to_string())?;

    let _save_guard = coordinator.0.lock().await;
    save_project(&project_path, &project).map_err(|error| error.to_string())?;

    Ok(CacheClearResponse {
        project,
        freed_bytes,
    })
}
