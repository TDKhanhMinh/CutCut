export interface SilenceSettings {
  thresholdDb: number;
  minDurationMs: number;
}

export interface SilenceInterval {
  id: string;
  startMs: number;
  endMs: number;
  durationMs: number;
}
