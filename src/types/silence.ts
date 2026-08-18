export type SilencePreset = 'conservative' | 'balanced' | 'aggressive' | 'custom';

export interface SilenceConfig {
  preset: SilencePreset;
  settings: SilenceSettings;
}

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
