use crate::models::fusion::{NonSpeechCandidate, Confidence};
use crate::models::project::{EditTimeline, EditAction};
use crate::models::suggestion::CutSuggestion;

pub fn generate_suggestions(
    source_media_id: &str,
    candidates: &[NonSpeechCandidate],
    existing_timeline: Option<&EditTimeline>,
) -> Vec<CutSuggestion> {
    let mut suggestions = Vec::new();

    for candidate in candidates {
        // Deterministic ID based on source media and timestamps
        let id = format!("cut_{}_{}_{}", source_media_id, candidate.start_ms, candidate.end_ms);

        // Check for overlaps with "Keep" actions in the existing timeline
        let mut overlaps_with_keep = false;
        if let Some(timeline) = existing_timeline {
            for action in &timeline.actions {
                if let EditAction::Keep { start_ms, end_ms, .. } = action {
                    // Check overlap
                    if candidate.start_ms < *end_ms && candidate.end_ms > *start_ms {
                        overlaps_with_keep = true;
                        break;
                    }
                }
            }
        }

        // According to acceptance criteria:
        // - Do not auto-enable Uncertain/Speech-conflict (Low confidence).
        // - Safety fixtures (Keep actions) overlap should disable the cut.
        let is_enabled = if overlaps_with_keep {
            false
        } else {
            matches!(candidate.confidence, Confidence::High | Confidence::Medium)
        };

        suggestions.push(CutSuggestion {
            id: id.clone(),
            source_media_id: source_media_id.to_string(),
            action: EditAction::Cut {
                id,
                source_media_id: source_media_id.to_string(),
                start_ms: candidate.start_ms,
                end_ms: candidate.end_ms,
            },
            confidence: candidate.confidence.clone(),
            reason: candidate.reason.clone(),
            evidence: candidate.evidence.clone(),
            is_enabled,
        });
    }

    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::fusion::DetectorEvidence;

    fn dummy_evidence() -> DetectorEvidence {
        DetectorEvidence {
            has_amplitude_silence: true,
            has_vad_non_speech: true,
            original_silence_duration_ms: None,
            original_vad_non_speech_duration_ms: None,
        }
    }

    #[test]
    fn test_generate_suggestions_basic() {
        let candidates = vec![
            NonSpeechCandidate {
                start_ms: 0,
                end_ms: 1000,
                reason: "silence".to_string(),
                evidence: dummy_evidence(),
                confidence: Confidence::High,
                recommended_padding_ms: 0,
            },
            NonSpeechCandidate {
                start_ms: 2000,
                end_ms: 3000,
                reason: "uncertain".to_string(),
                evidence: dummy_evidence(),
                confidence: Confidence::Low,
                recommended_padding_ms: 0,
            },
        ];

        let suggestions = generate_suggestions("media_1", &candidates, None);
        assert_eq!(suggestions.len(), 2);

        // High confidence -> enabled
        assert!(suggestions[0].is_enabled);
        assert_eq!(suggestions[0].id, "cut_media_1_0_1000");

        // Low confidence -> disabled
        assert!(!suggestions[1].is_enabled);
        assert_eq!(suggestions[1].id, "cut_media_1_2000_3000");
    }

    #[test]
    fn test_overlap_with_keep_action() {
        let candidates = vec![
            NonSpeechCandidate {
                start_ms: 0,
                end_ms: 1000,
                reason: "silence".to_string(),
                evidence: dummy_evidence(),
                confidence: Confidence::High,
                recommended_padding_ms: 0,
            },
        ];

        let timeline = EditTimeline {
            actions: vec![
                EditAction::Keep {
                    id: "keep_1".to_string(),
                    source_media_id: "media_1".to_string(),
                    start_ms: 500,
                    end_ms: 1500,
                }
            ],
        };

        let suggestions = generate_suggestions("media_1", &candidates, Some(&timeline));
        assert_eq!(suggestions.len(), 1);

        // Should be disabled because of overlap with Keep action, despite High confidence
        assert!(!suggestions[0].is_enabled);
    }
}
