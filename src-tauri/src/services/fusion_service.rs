use crate::models::fusion::{
    Confidence, DetectorEvidence, FusionConfig, FusionResult, NonSpeechCandidate,
};
use crate::models::silence::SilenceInterval;
use crate::models::vad::VadAnalysisResult;
use std::collections::BTreeSet;

pub struct FusionService;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MsState {
    Speech,
    HighConfidence,
    MediumConfidence,
    LowConfidence,
    None,
}

impl FusionService {
    pub fn fuse_intervals(
        duration_ms: u64,
        silence: &[SilenceInterval],
        vad: &VadAnalysisResult,
        config: &FusionConfig,
    ) -> FusionResult {
        if duration_ms == 0 {
            return FusionResult {
                candidates: Vec::new(),
                config_used: config.clone(),
                analysis_version: "fusion-v2".to_string(),
            };
        }

        let mut boundaries = BTreeSet::from([0, duration_ms]);
        for interval in silence {
            add_bounds(
                &mut boundaries,
                interval.start_ms,
                interval.end_ms,
                duration_ms,
            );
        }
        for interval in &vad.non_speech_intervals {
            add_bounds(
                &mut boundaries,
                interval.start_ms,
                interval.end_ms,
                duration_ms,
            );
        }
        for interval in &vad.speech_intervals {
            add_bounds(
                &mut boundaries,
                interval.start_ms,
                interval.end_ms,
                duration_ms,
            );
        }
        let boundaries: Vec<u64> = boundaries.into_iter().collect();

        let mut candidates = Vec::new();
        for pair in boundaries.windows(2) {
            let start = pair[0];
            let end = pair[1];
            if start >= end {
                continue;
            }
            let midpoint = start + (end - start) / 2;
            let has_silence = silence
                .iter()
                .any(|item| contains(item.start_ms, item.end_ms, midpoint));
            let has_vad_non_speech = vad
                .non_speech_intervals
                .iter()
                .any(|item| contains(item.start_ms, item.end_ms, midpoint));
            let has_vad_speech = vad
                .speech_intervals
                .iter()
                .any(|item| contains(item.start_ms, item.end_ms, midpoint));

            let state = if has_silence && has_vad_non_speech {
                MsState::HighConfidence
            } else if !has_silence && has_vad_non_speech && !has_vad_speech {
                MsState::MediumConfidence
            } else if has_silence && has_vad_speech {
                MsState::LowConfidence
            } else if has_vad_speech {
                MsState::Speech
            } else {
                MsState::None
            };

            if !matches!(
                state,
                MsState::HighConfidence | MsState::MediumConfidence | MsState::LowConfidence
            ) {
                continue;
            }

            let (confidence, reason) = match state {
                MsState::HighConfidence => (Confidence::High, "silence"),
                MsState::MediumConfidence => (Confidence::Medium, "noise_only"),
                MsState::LowConfidence => (Confidence::Low, "uncertain"),
                _ => unreachable!(),
            };
            let padded_start = start.saturating_add(config.lead_in_padding_ms).min(end);
            let padded_end = end
                .saturating_sub(config.lead_out_padding_ms)
                .max(padded_start);
            if padded_end.saturating_sub(padded_start) < config.min_candidate_duration_ms {
                continue;
            }

            candidates.push(NonSpeechCandidate {
                start_ms: padded_start,
                end_ms: padded_end,
                reason: reason.to_string(),
                evidence: DetectorEvidence {
                    has_amplitude_silence: has_silence,
                    has_vad_non_speech,
                    original_silence_duration_ms: has_silence.then_some(end - start),
                    original_vad_non_speech_duration_ms: has_vad_non_speech.then_some(end - start),
                },
                confidence,
                recommended_padding_ms: config.lead_in_padding_ms + config.lead_out_padding_ms,
            });
        }

        let mut merged: Vec<NonSpeechCandidate> = Vec::new();
        for candidate in candidates {
            if let Some(previous) = merged.last_mut() {
                if previous.end_ms == candidate.start_ms
                    && previous.reason == candidate.reason
                    && previous.confidence == candidate.confidence
                {
                    previous.end_ms = candidate.end_ms;
                    previous.evidence.original_silence_duration_ms = Some(
                        previous
                            .evidence
                            .original_silence_duration_ms
                            .unwrap_or_default()
                            + candidate
                                .evidence
                                .original_silence_duration_ms
                                .unwrap_or_default(),
                    );
                    previous.evidence.original_vad_non_speech_duration_ms = Some(
                        previous
                            .evidence
                            .original_vad_non_speech_duration_ms
                            .unwrap_or_default()
                            + candidate
                                .evidence
                                .original_vad_non_speech_duration_ms
                                .unwrap_or_default(),
                    );
                    continue;
                }
            }
            merged.push(candidate);
        }

        FusionResult {
            candidates: merged,
            config_used: config.clone(),
            analysis_version: "fusion-v2".to_string(),
        }
    }
}

fn add_bounds(bounds: &mut BTreeSet<u64>, start: u64, end: u64, duration_ms: u64) {
    bounds.insert(start.min(duration_ms));
    bounds.insert(end.min(duration_ms));
}

fn contains(start: u64, end: u64, value: u64) -> bool {
    start <= value && value < end
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::silence::{SilenceDetectionMetadata, SilenceInterval};
    use crate::models::vad::{NonSpeechInterval, SpeechInterval, VadConfig};

    fn silence(start_ms: u64, end_ms: u64) -> SilenceInterval {
        SilenceInterval {
            id: format!("silence-{start_ms}-{end_ms}"),
            start_ms,
            end_ms,
            duration_ms: end_ms - start_ms,
            detection: SilenceDetectionMetadata {
                detector_version: "test".into(),
                threshold_db: -35,
                min_duration_ms: 100,
                padding_ms: 0,
                tail_policy: "test".into(),
            },
            measured_level_db: None,
        }
    }

    fn vad(non_speech: Vec<NonSpeechInterval>, speech: Vec<SpeechInterval>) -> VadAnalysisResult {
        VadAnalysisResult {
            provider: "test".into(),
            version: "test".into(),
            speech_intervals: speech,
            non_speech_intervals: non_speech,
            config_used: VadConfig::default(),
        }
    }

    #[test]
    fn clean_silence_is_high_confidence() {
        let result = FusionService::fuse_intervals(
            1_000,
            &[silence(100, 300)],
            &vad(
                vec![NonSpeechInterval {
                    start_ms: 100,
                    end_ms: 300,
                    reason: "non_speech".into(),
                }],
                vec![
                    SpeechInterval {
                        start_ms: 0,
                        end_ms: 100,
                    },
                    SpeechInterval {
                        start_ms: 300,
                        end_ms: 1_000,
                    },
                ],
            ),
            &FusionConfig {
                lead_in_padding_ms: 0,
                lead_out_padding_ms: 0,
                min_candidate_duration_ms: 0,
            },
        );
        assert_eq!(result.analysis_version, "fusion-v2");
        assert!(matches!(result.candidates[0].confidence, Confidence::High));
    }

    #[test]
    fn speech_conflict_is_uncertain_and_never_high_confidence() {
        let result = FusionService::fuse_intervals(
            1_000,
            &[silence(100, 300)],
            &vad(
                vec![],
                vec![SpeechInterval {
                    start_ms: 100,
                    end_ms: 300,
                }],
            ),
            &FusionConfig {
                lead_in_padding_ms: 0,
                lead_out_padding_ms: 0,
                min_candidate_duration_ms: 0,
            },
        );
        assert!(matches!(result.candidates[0].confidence, Confidence::Low));
        assert_eq!(result.candidates[0].reason, "uncertain");
    }

    #[test]
    fn padding_shrinks_candidate_at_both_edges() {
        let result = FusionService::fuse_intervals(
            1_000,
            &[silence(100, 500)],
            &vad(
                vec![NonSpeechInterval {
                    start_ms: 100,
                    end_ms: 500,
                    reason: "non_speech".into(),
                }],
                vec![],
            ),
            &FusionConfig {
                lead_in_padding_ms: 50,
                lead_out_padding_ms: 75,
                min_candidate_duration_ms: 0,
            },
        );
        assert_eq!(
            (result.candidates[0].start_ms, result.candidates[0].end_ms),
            (150, 425)
        );
    }

    #[test]
    fn background_noise_without_speech_is_reviewable_medium_confidence() {
        let result = FusionService::fuse_intervals(
            1_000,
            &[],
            &vad(
                vec![NonSpeechInterval {
                    start_ms: 200,
                    end_ms: 700,
                    reason: "non_speech".into(),
                }],
                vec![],
            ),
            &FusionConfig {
                lead_in_padding_ms: 0,
                lead_out_padding_ms: 0,
                min_candidate_duration_ms: 100,
            },
        );

        assert_eq!(result.candidates.len(), 1);
        assert_eq!(result.candidates[0].reason, "noise_only");
        assert!(matches!(
            result.candidates[0].confidence,
            Confidence::Medium
        ));
        assert!(!result.candidates[0].evidence.has_amplitude_silence);
    }
}
