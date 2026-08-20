use crate::models::fusion::{
    Confidence, DetectorEvidence, FusionConfig, FusionResult, NonSpeechCandidate,
};
use crate::models::silence::SilenceInterval;
use crate::models::vad::VadAnalysisResult;
use std::cmp;

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
                candidates: vec![],
                config_used: config.clone(),
                analysis_version: "v1".to_string(),
            };
        }

        let mut is_silence = vec![false; duration_ms as usize];
        let mut is_vad_non_speech = vec![false; duration_ms as usize];
        let mut is_vad_speech = vec![false; duration_ms as usize];

        for s in silence {
            let start = cmp::min(s.start_ms, duration_ms) as usize;
            let end = cmp::min(s.end_ms, duration_ms) as usize;
            for value in is_silence.iter_mut().take(end).skip(start) {
                *value = true;
            }
        }

        for v in &vad.non_speech_intervals {
            let start = cmp::min(v.start_ms, duration_ms) as usize;
            let end = cmp::min(v.end_ms, duration_ms) as usize;
            for value in is_vad_non_speech.iter_mut().take(end).skip(start) {
                *value = true;
            }
        }

        for v in &vad.speech_intervals {
            let start = cmp::min(v.start_ms, duration_ms) as usize;
            let end = cmp::min(v.end_ms, duration_ms) as usize;
            for value in is_vad_speech.iter_mut().take(end).skip(start) {
                *value = true;
            }
        }

        let mut states = vec![MsState::None; duration_ms as usize];
        for i in 0..(duration_ms as usize) {
            let s = is_silence[i];
            let vns = is_vad_non_speech[i];
            let vs = is_vad_speech[i];

            states[i] = if s && vns {
                MsState::HighConfidence
            } else if !s && vns {
                MsState::MediumConfidence
            } else if s && vs {
                MsState::LowConfidence
            } else if !s && vs {
                MsState::Speech
            } else {
                MsState::None
            };
        }

        let mut candidates = Vec::new();
        let mut current_start = None;
        let mut current_state_type = MsState::None; // High/Medium grouped together, Low is separate

        let flush =
            |start: usize, end: usize, state_type: MsState, cands: &mut Vec<NonSpeechCandidate>| {
                if start >= end {
                    return;
                }
                let mut conf = Confidence::High;
                let mut reason = "silence".to_string();

                if state_type == MsState::LowConfidence {
                    conf = Confidence::Low;
                    reason = "uncertain".to_string();
                } else if state_type == MsState::MediumConfidence {
                    conf = Confidence::Medium;
                    reason = "noise_only".to_string();
                }

                // Calculate original durations loosely
                let mut sil_len = 0;
                let mut vad_ns_len = 0;
                for i in start..end {
                    if is_silence[i] {
                        sil_len += 1;
                    }
                    if is_vad_non_speech[i] {
                        vad_ns_len += 1;
                    }
                }

                // Apply padding
                let padded_start = (start as u64).saturating_add(config.lead_out_padding_ms);
                let padded_end = (end as u64).saturating_sub(config.lead_in_padding_ms);

                if padded_start < padded_end
                    && (padded_end - padded_start) >= config.min_candidate_duration_ms
                {
                    cands.push(NonSpeechCandidate {
                        start_ms: padded_start,
                        end_ms: padded_end,
                        reason,
                        evidence: DetectorEvidence {
                            has_amplitude_silence: sil_len > 0,
                            has_vad_non_speech: vad_ns_len > 0,
                            original_silence_duration_ms: if sil_len > 0 {
                                Some(sil_len)
                            } else {
                                None
                            },
                            original_vad_non_speech_duration_ms: if vad_ns_len > 0 {
                                Some(vad_ns_len)
                            } else {
                                None
                            },
                        },
                        confidence: conf,
                        recommended_padding_ms: config.lead_in_padding_ms
                            + config.lead_out_padding_ms,
                    });
                }
            };

        for (i, st) in states.iter().copied().enumerate() {
            let is_candidate = matches!(
                st,
                MsState::HighConfidence | MsState::MediumConfidence | MsState::LowConfidence
            );

            let group_type = st;

            if is_candidate {
                if current_start.is_none() {
                    current_start = Some(i);
                    current_state_type = group_type;
                } else {
                    let is_high_med = matches!(
                        current_state_type,
                        MsState::HighConfidence | MsState::MediumConfidence
                    );
                    let new_is_high_med = matches!(
                        group_type,
                        MsState::HighConfidence | MsState::MediumConfidence
                    );

                    if is_high_med && new_is_high_med {
                        if group_type == MsState::MediumConfidence {
                            current_state_type = MsState::MediumConfidence;
                        }
                    } else if current_state_type != group_type {
                        // Group boundary changed (e.g. from High/Medium to Low)
                        flush(
                            current_start.unwrap(),
                            i,
                            current_state_type,
                            &mut candidates,
                        );
                        current_start = Some(i);
                        current_state_type = group_type;
                    }
                }
            } else {
                if let Some(start) = current_start {
                    flush(start, i, current_state_type, &mut candidates);
                    current_start = None;
                    current_state_type = MsState::None;
                }
            }
        }

        if let Some(start) = current_start {
            flush(
                start,
                duration_ms as usize,
                current_state_type,
                &mut candidates,
            );
        }

        FusionResult {
            candidates,
            config_used: config.clone(),
            analysis_version: "v1".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::vad::NonSpeechInterval;
    use crate::models::vad::SpeechInterval;

    #[test]
    fn test_fusion_basic() {
        let duration = 1000;
        let silence = vec![SilenceInterval {
            start_ms: 100,
            end_ms: 300,
            duration_ms: 200,
            id: "".into(),
            detection: crate::models::silence::SilenceDetectionMetadata {
                detector_version: "test".into(),
                threshold_db: -35,
                min_duration_ms: 100,
                padding_ms: 0,
                tail_policy: "test".into(),
            },
            measured_level_db: None,
        }];
        let vad = VadAnalysisResult {
            provider: "".into(),
            version: "".into(),
            speech_intervals: vec![
                SpeechInterval {
                    start_ms: 0,
                    end_ms: 100,
                },
                SpeechInterval {
                    start_ms: 300,
                    end_ms: 1000,
                },
            ],
            non_speech_intervals: vec![NonSpeechInterval {
                start_ms: 100,
                end_ms: 300,
                reason: "".into(),
            }],
            config_used: crate::models::vad::VadConfig {
                threshold: 0.5,
                min_speech_duration_ms: 0,
                min_silence_duration_ms: 0,
                speech_pad_ms: 0,
            },
        };
        let config = FusionConfig {
            lead_in_padding_ms: 0,
            lead_out_padding_ms: 0,
            min_candidate_duration_ms: 0,
        };

        let result = FusionService::fuse_intervals(duration, &silence, &vad, &config);
        assert_eq!(result.candidates.len(), 1);
        assert_eq!(result.candidates[0].start_ms, 100);
        assert_eq!(result.candidates[0].end_ms, 300);
        assert!(matches!(result.candidates[0].confidence, Confidence::High));
    }
}
