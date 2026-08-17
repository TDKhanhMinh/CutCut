use crate::models::project::Project;
use crate::services::project_repository::{load_project, save_project};
use serde::{Deserialize, Serialize};

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
pub async fn save_project_to_disk(path: String, project: Project) -> Result<(), String> {
    save_project(&path, &project).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_project_from_disk(path: String) -> Result<ProjectResponse, String> {
    let project = load_project(&path).map_err(|e| e.to_string())?;
    Ok(ProjectResponse {
        project,
        path,
    })
}
