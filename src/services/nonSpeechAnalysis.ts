import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { SilenceConfig, SilenceDetectionResult } from "@/types/silence";
import type { FusionConfig, FusionResult } from "@/types/fusion";
import type { VadAnalysisResult, VadConfig } from "@/types/vad";
import type { MediaJobEvent } from "@/services/media";

const DEFAULT_VAD_CONFIG: VadConfig = {
  min_speech_duration_ms: 250,
  min_silence_duration_ms: 100,
  speech_pad_ms: 30,
  threshold: 0.5,
};

const DEFAULT_FUSION_CONFIG: FusionConfig = {
  lead_in_padding_ms: 0,
  lead_out_padding_ms: 0,
  min_candidate_duration_ms: 250,
};

function newJobId(): string {
  return crypto.randomUUID();
}

async function runMediaJob<T>(jobId: string, start: () => Promise<unknown>): Promise<T> {
  let resolveResult!: (value: T) => void;
  let rejectResult!: (reason?: unknown) => void;
  const resultPromise = new Promise<T>((resolve, reject) => {
    resolveResult = resolve;
    rejectResult = reject;
  });
  const unlisten = await listen<MediaJobEvent>("media-job", ({ payload }) => {
    if (payload.jobId !== jobId) return;

    if (payload.state === "completed") {
      if (payload.result === undefined || payload.result === null) {
        rejectResult(new Error(payload.message ?? "Media analysis completed without a result."));
        return;
      }
      resolveResult(payload.result as T);
    } else if (payload.state === "failed" || payload.state === "cancelled") {
      rejectResult(new Error(payload.error ?? payload.message ?? "Media analysis failed."));
    }
  });

  try {
    await start();
    return await resultPromise;
  } finally {
    unlisten();
  }
}

/**
 * Runs the local-only silence and VAD providers, then fuses their typed
 * results. The UI never fabricates candidates; suggestions are derived from
 * this persisted-source analysis result.
 */
export async function analyzeNonSpeech({
  sourcePath,
  durationMs,
  silenceConfig,
  vadConfig = DEFAULT_VAD_CONFIG,
  fusionConfig = DEFAULT_FUSION_CONFIG,
}: {
  sourcePath: string;
  durationMs: number;
  silenceConfig: SilenceConfig;
  vadConfig?: VadConfig;
  fusionConfig?: FusionConfig;
}): Promise<FusionResult> {
  if (!sourcePath || durationMs <= 0) {
    throw new Error("A readable media source and positive duration are required.");
  }

  const silenceJobId = newJobId();
  const vadJobId = newJobId();

  const [silence, vad] = await Promise.all([
    runMediaJob<SilenceDetectionResult>(silenceJobId, () =>
      invoke("start_silence_detection", {
        jobId: silenceJobId,
        path: sourcePath,
        settings: silenceConfig.settings,
        durationMs,
      }),
    ),
    runMediaJob<VadAnalysisResult>(vadJobId, () =>
      invoke("start_vad_analysis", {
        sourcePath,
        jobId: vadJobId,
        durationMs,
        config: vadConfig,
      }),
    ),
  ]);

  return invoke<FusionResult>("fuse_non_speech_intervals", {
    durationMs,
    silence: silence.intervals,
    vad,
    config: fusionConfig,
  });
}
