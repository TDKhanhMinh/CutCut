import { invoke } from "@tauri-apps/api/core";
import type { Transcript } from "@/types/project";
import type { FillerAnalysisResult } from "@/types/filler";

export const detectFillerCandidates = (
  sourceMediaId: string,
  transcript: Transcript,
  mediaDurationMs: number,
  paddingMs = 0,
) =>
  invoke<FillerAnalysisResult>("detect_filler_candidates", {
    sourceMediaId,
    transcript,
    mediaDurationMs,
    paddingMs,
  });
