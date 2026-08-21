import { invoke } from "@tauri-apps/api/core";
import { useProjectStore } from "@/stores/useProjectStore";
import type { Transcript } from "@/types/transcript";

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
  }).then((transcript) => {
    const state = useProjectStore.getState();
    if (options.projectPath && state.projectPath === options.projectPath && state.activeProject) {
      state.updateProject((draft) => {
        draft.transcript = transcript;
        draft.updatedAt = Date.now();
      });
    }
    return transcript;
  });
