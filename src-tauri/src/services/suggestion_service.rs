use crate::models::edit_plan::{EditAction, EditActionSource, EditActionType, EditPlan};
use crate::models::fusion::{Confidence, NonSpeechCandidate};
use crate::models::suggestion::CutSuggestion;

pub fn generate_suggestions(
    source_media_id: &str,
    candidates: &[NonSpeechCandidate],
    analysis_version: &str,
    existing_plan: Option<&EditPlan>,
) -> Vec<CutSuggestion> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    candidates
        .iter()
        .map(|candidate| {
            let id = format!(
                "cut_{}_{}_{}",
                source_media_id, candidate.start_ms, candidate.end_ms
            );
            let user_overridden = existing_plan.is_some_and(|plan| {
                plan.actions.iter().any(|action| {
                    action.source == EditActionSource::User
                        && action.start_ms < candidate.end_ms
                        && action.end_ms > candidate.start_ms
                })
            });

            let enabled = !user_overridden
                && matches!(candidate.confidence, Confidence::High | Confidence::Medium);
            CutSuggestion {
                action: EditAction {
                    id,
                    action_type: EditActionType::Cut,
                    source_media_id: source_media_id.to_string(),
                    start_ms: candidate.start_ms,
                    end_ms: candidate.end_ms,
                    source: EditActionSource::Local,
                    reason: candidate.reason.clone(),
                    confidence: Some(match candidate.confidence {
                        Confidence::High => 1.0,
                        Confidence::Medium => 0.6,
                        Confidence::Low => 0.2,
                    }),
                    enabled,
                    created_at: now,
                    updated_at: now,
                },
                evidence: candidate.evidence.clone(),
                source_version: analysis_version.to_string(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::fusion::DetectorEvidence;

    fn evidence() -> DetectorEvidence {
        DetectorEvidence {
            has_amplitude_silence: true,
            has_vad_non_speech: true,
            original_silence_duration_ms: None,
            original_vad_non_speech_duration_ms: None,
        }
    }

    #[test]
    fn enables_only_conservative_confidence() {
        let candidates = vec![
            NonSpeechCandidate {
                start_ms: 0,
                end_ms: 1_000,
                reason: "silence".to_string(),
                evidence: evidence(),
                confidence: Confidence::High,
                recommended_padding_ms: 0,
            },
            NonSpeechCandidate {
                start_ms: 2_000,
                end_ms: 3_000,
                reason: "uncertain".to_string(),
                evidence: evidence(),
                confidence: Confidence::Low,
                recommended_padding_ms: 0,
            },
        ];
        let suggestions = generate_suggestions("media_1", &candidates, "v1", None);
        assert!(suggestions[0].action.enabled);
        assert!(!suggestions[1].action.enabled);
        assert_eq!(suggestions[0].action.start_ms, 0);
    }

    #[test]
    fn disables_candidate_overlapping_user_action() {
        let plan = EditPlan {
            actions: vec![EditAction {
                id: "user-1".to_string(),
                action_type: EditActionType::Keep,
                source_media_id: "media_1".to_string(),
                start_ms: 500,
                end_ms: 1_500,
                source: EditActionSource::User,
                reason: "manual".to_string(),
                confidence: None,
                enabled: true,
                created_at: 0,
                updated_at: 0,
            }],
        };
        let candidate = NonSpeechCandidate {
            start_ms: 0,
            end_ms: 1_000,
            reason: "silence".to_string(),
            evidence: evidence(),
            confidence: Confidence::High,
            recommended_padding_ms: 0,
        };
        let suggestions = generate_suggestions("media_1", &[candidate], "v1", Some(&plan));
        assert!(!suggestions[0].action.enabled);
    }
}
