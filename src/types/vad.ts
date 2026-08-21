export interface VadConfig {
  min_speech_duration_ms: number;
  min_silence_duration_ms: number;
  speech_pad_ms: number;
  threshold: number;
}

export interface SpeechInterval {
  start_ms: number;
  end_ms: number;
}

export interface NonSpeechInterval {
  start_ms: number;
  end_ms: number;
  reason: string; // "noise_only", "silence", "uncertain", "non_speech"
}

export interface VadAnalysisResult {
  provider: string;
  version: string;
  speech_intervals: SpeechInterval[];
  non_speech_intervals: NonSpeechInterval[];
  config_used: VadConfig;
}
