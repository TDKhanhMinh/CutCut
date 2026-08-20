use crate::models::project::Project;
use crate::services::project_repository::{load_project, save_project, ProjectSaveCoordinator};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize)]
pub struct ProjectResponse {
    pub project: Project,
    pub path: String,
}

#[tauri::command]
pub async fn create_project() -> Result<Project, String> {
    Ok(Project::default())
}

#[tauri::command]
pub async fn save_project_to_disk(
    path: String,
    project: Project,
    coordinator: State<'_, ProjectSaveCoordinator>,
) -> Result<(), String> {
    let _save_guard = coordinator.0.lock().await;
    save_project(&path, &project).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_project_from_disk(path: String) -> Result<ProjectResponse, String> {
    let project = load_project(&path).map_err(|e| e.to_string())?;
    Ok(ProjectResponse { project, path })
}
