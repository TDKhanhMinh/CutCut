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
  result?: unknown;
}

export type MediaCompatibilitySeverity = "warning" | "blocking";

export interface MediaCompatibilityWarning {
  code: "duration-mismatch" | "resolution-mismatch" | "orientation-mismatch" | "metadata-invalid";
  message: string;
  severity: MediaCompatibilitySeverity;
}

export interface MediaCompatibilityResult {
  warnings: MediaCompatibilityWarning[];
  requiresConfirmation: boolean;
}

const DURATION_ABSOLUTE_TOLERANCE_SEC = 0.25;
const DURATION_RELATIVE_TOLERANCE = 0.01;

/**
 * Compare a replacement media file with the saved source metadata before relink.
 * A relink with any warning requires an explicit user confirmation because all
 * transcript/edit timestamps remain anchored to the original source timeline.
 */
export function compareMediaMetadata(
  previous: MediaSourceMetadata,
  replacement: MediaSourceMetadata,
): MediaCompatibilityResult {
  const warnings: MediaCompatibilityWarning[] = [];

  if (
    !Number.isFinite(previous.durationSec) ||
    previous.durationSec <= 0 ||
    !Number.isFinite(replacement.durationSec) ||
    replacement.durationSec <= 0
  ) {
    warnings.push({
      code: "metadata-invalid",
      message: "Không thể xác nhận duration của media thay thế.",
      severity: "blocking",
    });
  } else {
    const durationDelta = Math.abs(previous.durationSec - replacement.durationSec);
    const allowedDelta = Math.max(
      DURATION_ABSOLUTE_TOLERANCE_SEC,
      previous.durationSec * DURATION_RELATIVE_TOLERANCE,
    );

    if (durationDelta > allowedDelta) {
      warnings.push({
        code: "duration-mismatch",
        message: `Duration thay đổi từ ${previous.durationSec.toFixed(2)}s thành ${replacement.durationSec.toFixed(2)}s.`,
        severity: "blocking",
      });
    }
  }

  if (previous.width !== replacement.width || previous.height !== replacement.height) {
    warnings.push({
      code: "resolution-mismatch",
      message: `Resolution thay đổi từ ${previous.width}×${previous.height} thành ${replacement.width}×${replacement.height}.`,
      severity: "warning",
    });
  }

  if (previous.rotation !== replacement.rotation) {
    warnings.push({
      code: "orientation-mismatch",
      message: `Orientation thay đổi từ ${previous.rotation}° thành ${replacement.rotation}°.`,
      severity: "warning",
    });
  }

  return {
    warnings,
    requiresConfirmation: warnings.length > 0,
  };
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

export const extractAudioForStt = (sourcePath: string, durationUs?: number, jobId?: string) =>
  invoke<string>("extract_audio_for_stt", {
    sourcePath,
    jobId: jobId ?? null,
    durationUs: durationUs ?? null,
  });

export const cleanupSttAudio = (tempPath: string) =>
  invoke<void>("cleanup_stt_audio", { tempPath });

export const listenToMediaJobs = (handler: (event: MediaJobEvent) => void): Promise<UnlistenFn> =>
  listen<MediaJobEvent>("media-job", ({ payload }) => handler(payload));
