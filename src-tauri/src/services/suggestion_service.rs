use crate::models::fusion::{NonSpeechCandidate, Confidence};
use crate::models::edit_plan::{EditPlan, EditAction, ActionPayload, ActionSource};
use crate::models::suggestion::CutSuggestion;

pub fn generate_suggestions(
    source_media_id: &str,
    candidates: &[NonSpeechCandidate],
    analysis_version: &str,
    existing_plan: Option<&EditPlan>,
) -> Vec<CutSuggestion> {
    let mut suggestions = Vec::new();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    for candidate in candidates {
        // Deterministic ID based on source media and timestamps
        let id = format!("cut_{}_{}_{}", source_media_id, candidate.start_ms, candidate.end_ms);

        // Check for overlaps with "Keep" decisions in the existing plan
        // In the new schema, a user "Keep" is simply an action (e.g. Cut) with enabled = false
        // or a specific UserManual action. For simplicity, we just check if there's any action
        // at this time range that user modified.
        let mut user_overridden = false;
        if let Some(plan) = existing_plan {
            for action in &plan.actions {
                if action.source == ActionSource::UserManual {
                    if let ActionPayload::Cut { start_ms, end_ms } = action.payload {
                        // Check overlap
                        if candidate.start_ms < end_ms && candidate.end_ms > start_ms {
                            user_overridden = true;
                            break;
                        }
                    }
                }
            }
        }

        // According to acceptance criteria:
        // - Do not auto-enable Uncertain/Speech-conflict (Low confidence).
        // - Overlaps with user overrides disable the auto cut.
        let is_enabled = if user_overridden {
            false
        } else {
            matches!(candidate.confidence, Confidence::High | Confidence::Medium)
        };

        suggestions.push(CutSuggestion {
            action: EditAction {
                id: id.clone(),
                source_media_id: source_media_id.to_string(),
                payload: ActionPayload::Cut {
                    start_ms: candidate.start_ms,
                    end_ms: candidate.end_ms,
                },
                source: ActionSource::LocalDetector,
                reason: candidate.reason.clone(),
                confidence: Some(format!("{:?}", candidate.confidence)),
                enabled: is_enabled,
                is_manual_modified: None,
                created_at: now,
                updated_at: now,
            },
            evidence: candidate.evidence.clone(),
            source_version: analysis_version.to_string(),
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

        let suggestions = generate_suggestions("media_1", &candidates, "v1", None);
        assert_eq!(suggestions.len(), 2);

        // High confidence -> enabled
        assert!(suggestions[0].action.enabled);
        assert_eq!(suggestions[0].action.id, "cut_media_1_0_1000");
        assert_eq!(suggestions[0].source_version, "v1");

        // Low confidence -> disabled
        assert!(!suggestions[1].action.enabled);
        assert_eq!(suggestions[1].action.id, "cut_media_1_2000_3000");
    }

    #[test]
    fn test_overlap_with_user_action() {
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

        let plan = EditPlan {
            version: 1,
            actions: vec![
                EditAction {
                    id: "user_cut_1".to_string(),
                    source_media_id: "media_1".to_string(),
                    payload: ActionPayload::Cut { start_ms: 500, end_ms: 1500 },
                    source: ActionSource::UserManual,
                    reason: "manual".to_string(),
                    confidence: None,
                    enabled: false,
                    is_manual_modified: None,
                    created_at: 0,
                    updated_at: 0,
                }
            ],
            generation_metadata: None,
        };

        let suggestions = generate_suggestions("media_1", &candidates, "v1", Some(&plan));
        assert_eq!(suggestions.len(), 1);

        // Should be disabled because of overlap with User action, despite High confidence
        assert!(!suggestions[0].action.enabled);
    }
}
