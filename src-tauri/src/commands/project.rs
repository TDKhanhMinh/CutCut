use crate::models::project::Project;
use crate::services::edit_validator::{validate_and_normalize, IssueLevel};
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
    let (normalized_plan, issues) =
        validate_and_normalize(project.edit_plan.clone(), &project.media);
    if let Some(error) = issues.iter().find(|issue| issue.level == IssueLevel::Error) {
        return Err(format!("Project edit plan is invalid: {}", error.message));
    }
    let mut project = project;
    project.edit_plan = normalized_plan;
    let _save_guard = coordinator.0.lock().await;
    save_project(&path, &project).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_project_from_disk(path: String) -> Result<ProjectResponse, String> {
    let mut project = load_project(&path).map_err(|e| e.to_string())?;
    let (normalized_plan, issues) =
        validate_and_normalize(project.edit_plan.clone(), &project.media);
    if let Some(error) = issues.iter().find(|issue| issue.level == IssueLevel::Error) {
        return Err(format!("Project edit plan is invalid: {}", error.message));
    }
    project.edit_plan = normalized_plan;
    Ok(ProjectResponse { project, path })
}
