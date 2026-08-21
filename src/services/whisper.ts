import { invoke } from "@tauri-apps/api/core";
import { useProjectStore } from "@/stores/useProjectStore";
import type { Transcript } from "@/types/transcript";
import { telemetry } from "@/services/telemetry";

export interface WhisperRuntimeInfo {
  available: boolean;
  version: string;
  backend: string;
}

export const checkWhisperRuntime = () => invoke<WhisperRuntimeInfo>("check_whisper_runtime");

export const transcribeAudio = (options: {
  sourceId: string;
  audioPath: string;
  modelId: string;
  language?: string;
  projectPath?: string;
  replaceExisting?: boolean;
  forceReplaceModified?: boolean;
}) =>
  invoke<Transcript>("transcribe_audio", {
    sourceId: options.sourceId,
    audioPath: options.audioPath,
    modelId: options.modelId,
    language: options.language ?? "auto",
    projectPath: options.projectPath ?? null,
    replaceExisting: options.replaceExisting ?? false,
    forceReplaceModified: options.forceReplaceModified ?? false,
  })
    .then((transcript) => {
      telemetry.track("stt_completed", { modelId: options.modelId });
      const state = useProjectStore.getState();
      if (options.projectPath && state.projectPath === options.projectPath && state.activeProject) {
        state.updateProject((draft) => {
          draft.transcript = transcript;
          draft.updatedAt = Date.now();
        });
      }
      return transcript;
    })
    .catch((error: unknown) => {
      telemetry.track("stt_failed", { errorCode: error instanceof Error ? error.name : "unknown" });
      throw error;
    });
