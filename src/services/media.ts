import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { MediaSourceMetadata } from "@/types/media";

export type MediaJobState = "started" | "progress" | "completed" | "failed" | "cancelled";

export interface MediaJobEvent {
  jobId: string;
  state: MediaJobState;
  progress?: number;
  message?: string;
  error?: string;
}

export const readMediaMetadata = (path: string) =>
  invoke<MediaSourceMetadata>("read_media_metadata", { path });

export const checkMediaExists = (path: string) => invoke<boolean>("check_media_exists", { path });

export const exportPrototypeVideo = (
  inputPath: string,
  outputPath: string,
  totalDurationSec: number,
) => invoke<string>("export_prototype_video", { inputPath, outputPath, totalDurationSec });

export const cancelMediaJob = (jobId: string) => invoke<void>("cancel_media_job", { jobId });

export const listenToMediaJobs = (handler: (event: MediaJobEvent) => void): Promise<UnlistenFn> =>
  listen<MediaJobEvent>("media-job", ({ payload }) => handler(payload));
