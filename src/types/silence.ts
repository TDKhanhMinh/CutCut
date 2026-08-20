export interface SilenceSettings {
  thresholdDb: number;
  minDurationMs: number;
  paddingMs: number;
}

export type SilenceTailPolicy = "flushToSourceDuration" | "dropWithoutSourceDuration";

export interface SilenceDetectionMetadata {
  detectorVersion: string;
  thresholdDb: number;
  minDurationMs: number;
  paddingMs: number;
  tailPolicy: SilenceTailPolicy;
}

export interface SilenceInterval {
  id: string;
  startMs: number;
  endMs: number;
  durationMs: number;
  detection: SilenceDetectionMetadata;
  measuredLevelDb: number | null;
}

export interface SilenceDetectionResult {
  sourceDurationMs: number | null;
  detection: SilenceDetectionMetadata;
  intervals: SilenceInterval[];
}
