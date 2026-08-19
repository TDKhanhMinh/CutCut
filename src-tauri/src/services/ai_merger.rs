use uuid::Uuid;
use crate::models::project::Project;
use crate::models::edit_plan::{ActionPayload, ActionSource, EditAction};
use crate::models::ai::AIAnalysisAction;

pub struct AiMergerService;

impl AiMergerService {
    const OVERLAP_TOLERANCE_MS: u64 = 200; // 200ms tolerance

    pub fn merge_proposals(
        project: &mut Project,
        ai_actions: Vec<AIAnalysisAction>,
    ) {
        let plan = &mut project.edit_plan;
        
        for ai_action in ai_actions {
            // Only care about CUT and HIGHLIGHT
            if ai_action.action == "KEEP" {
                continue;
            }

            let start_ms = (ai_action.start * 1000.0) as u64;
            let end_ms = (ai_action.end * 1000.0) as u64;

            // Check against existing actions for duplicates / user rejections
            let mut is_duplicate = false;
            let mut user_rejected = false;

            for existing_action in &plan.actions {
                let (ex_start, ex_end) = match &existing_action.payload {
                    ActionPayload::Cut { start_ms, end_ms } => (*start_ms, *end_ms),
                    ActionPayload::Highlight { start_ms, end_ms } => (*start_ms, *end_ms),
                    _ => continue, // Ignore captions/zooms for this comparison
                };

                let same_type = match (&existing_action.payload, ai_action.action.as_str()) {
                    (ActionPayload::Cut { .. }, "CUT") => true,
                    (ActionPayload::Highlight { .. }, "HIGHLIGHT") => true,
                    _ => false,
                };

                if same_type {
                    let start_diff = (ex_start as i64 - start_ms as i64).abs() as u64;
                    let end_diff = (ex_end as i64 - end_ms as i64).abs() as u64;

                    if start_diff <= Self::OVERLAP_TOLERANCE_MS && end_diff <= Self::OVERLAP_TOLERANCE_MS {
                        is_duplicate = true;
                        if !existing_action.enabled {
                            user_rejected = true;
                        }
                        break;
                    }
                }
            }

            // Conflict Resolution Matrix:
            if user_rejected {
                // Rule 1: User previously rejected this cut. Ignore AI proposal entirely.
                continue;
            }

            if is_duplicate {
                // Rule 2: It's a duplicate of an enabled local action. We deduplicate by ignoring the new one.
                continue;
            }

            // Rule 3: It's a new valid action. Insert it.
            let payload = match ai_action.action.as_str() {
                "CUT" => ActionPayload::Cut { start_ms, end_ms },
                "HIGHLIGHT" => ActionPayload::Highlight { start_ms, end_ms },
                _ => continue,
            };

            let new_action = EditAction {
                id: Uuid::new_v4().to_string(),
                source_media_id: "default_media".to_string(), // In a real app, match with actual media
                payload,
                source: ActionSource::AiAgent,
                reason: format!("{} ({})", ai_action.reason, ai_action.taxonomy),
                confidence: Some(ai_action.confidence.to_string()),
                enabled: true, // Proposed action defaults to enabled
                created_at: 0,
                updated_at: 0,
            };

            plan.actions.push(new_action);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::Project;
    use crate::models::edit_plan::{ActionPayload, ActionSource, EditAction, EditPlan};
    use crate::models::ai::AIAnalysisAction;

    fn create_mock_project(existing_actions: Vec<EditAction>) -> Project {
        let mut proj = Project::default();
        proj.edit_plan.actions = existing_actions;
        proj
    }

    #[test]
    fn test_merge_new_action() {
        let mut proj = create_mock_project(vec![]);
        let ai_actions = vec![AIAnalysisAction {
            start: 1.0,
            end: 2.0,
            action: "CUT".to_string(),
            reason: "false_start".to_string(),
            confidence: 0.9,
            taxonomy: "false_start".to_string(),
        }];

        AiMergerService::merge_proposals(&mut proj, ai_actions);
        assert_eq!(proj.edit_plan.actions.len(), 1);
        assert_eq!(proj.edit_plan.actions[0].source, ActionSource::AiAgent);
    }

    #[test]
    fn test_merge_dedupe_local_action() {
        let mut proj = create_mock_project(vec![EditAction {
            id: "local-1".to_string(),
            source_media_id: "default".to_string(),
            payload: ActionPayload::Cut { start_ms: 1000, end_ms: 2000 },
            source: ActionSource::LocalDetector,
            reason: "silence".to_string(),
            confidence: None,
            enabled: true,
            created_at: 0,
            updated_at: 0,
        }]);

        let ai_actions = vec![AIAnalysisAction {
            start: 1.05,
            end: 1.95,
            action: "CUT".to_string(),
            reason: "redundant".to_string(),
            confidence: 0.95,
            taxonomy: "redundant_sentence".to_string(),
        }];

        AiMergerService::merge_proposals(&mut proj, ai_actions);
        assert_eq!(proj.edit_plan.actions.len(), 1);
        assert_eq!(proj.edit_plan.actions[0].id, "local-1");
    }

    #[test]
    fn test_merge_user_rejected_action() {
        let mut proj = create_mock_project(vec![EditAction {
            id: "local-2".to_string(),
            source_media_id: "default".to_string(),
            payload: ActionPayload::Cut { start_ms: 5000, end_ms: 6000 },
            source: ActionSource::UserManual,
            reason: "manual".to_string(),
            confidence: None,
            enabled: false,
            created_at: 0,
            updated_at: 0,
        }]);

        let ai_actions = vec![AIAnalysisAction {
            start: 5.0,
            end: 6.0,
            action: "CUT".to_string(),
            reason: "false_start".to_string(),
            confidence: 0.85,
            taxonomy: "false_start".to_string(),
        }];

        AiMergerService::merge_proposals(&mut proj, ai_actions);
        assert_eq!(proj.edit_plan.actions.len(), 1);
        assert_eq!(proj.edit_plan.actions[0].enabled, false);
    }
}
