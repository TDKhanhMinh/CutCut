import { invoke } from "@tauri-apps/api/core";
import type { Project } from "@/types/project";

export interface LoadedProject {
  project: Project;
  path: string;
}

export const createProject = () => invoke<Project>("create_project");

export const loadProjectFromDisk = (path: string) =>
  invoke<LoadedProject>("load_project_from_disk", { path });

export const saveProjectToDisk = (path: string, project: Project) =>
  invoke<void>("save_project_to_disk", { path, project });
