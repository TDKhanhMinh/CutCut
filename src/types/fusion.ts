export type Confidence = "High" | "Medium" | "Low";

export interface DetectorEvidence {
    has_amplitude_silence: boolean;
    has_vad_non_speech: boolean;
    original_silence_duration_ms?: number;
    original_vad_non_speech_duration_ms?: number;
}

export interface NonSpeechCandidate {
    start_ms: number;
    end_ms: number;
    reason: string;
    evidence: DetectorEvidence;
    confidence: Confidence;
    recommended_padding_ms: number;
}

export interface FusionConfig {
    lead_in_padding_ms: number;
    lead_out_padding_ms: number;
    min_candidate_duration_ms: number;
}

export interface FusionResult {
    candidates: NonSpeechCandidate[];
    config_used: FusionConfig;
    analysis_version: string;
}
