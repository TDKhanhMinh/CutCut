import { invoke } from "@tauri-apps/api/core";
import type { Project } from "@/types/project";

export interface CacheUsageResponse {
  reclaimableBytes: number;
}

export interface CacheClearResponse {
  project: Project;
  freedBytes: number;
}

export const getCacheUsage = (projectPath: string, project: Project) =>
  invoke<CacheUsageResponse>("get_cache_usage", { projectPath, project });

export const clearProjectCache = (projectPath: string, project: Project) =>
  invoke<CacheClearResponse>("clear_project_cache", { projectPath, project });
