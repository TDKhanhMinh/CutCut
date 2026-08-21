use crate::models::ai::AIEditAction;
use crate::models::edit_plan::{EditAction, EditActionSource, EditActionType};
use crate::models::project::Project;
use uuid::Uuid;

pub struct AiMergerService;

impl AiMergerService {
    const OVERLAP_TOLERANCE_MS: u64 = 200;

    /// Merge validated AI proposals into the canonical, non-destructive plan.
    /// Unsupported proposal kinds are ignored because the canonical EditPlan
    /// deliberately has no highlight action type.
    pub fn merge_proposals(project: &mut Project, ai_actions: Vec<AIEditAction>) {
        for ai_action in ai_actions {
            let action_type = match ai_action.action.as_str() {
                "CUT" => EditActionType::Cut,
                "HIGHLIGHT" => EditActionType::Highlight,
                _ => continue,
            };

            let same_range = |existing: &EditAction| {
                existing.action_type == action_type
                    && existing.source_media_id == ai_action.source_media_id
                    && existing.start_ms.abs_diff(ai_action.start_ms)
                        <= Self::OVERLAP_TOLERANCE_MS
                    && existing.end_ms.abs_diff(ai_action.end_ms)
                        <= Self::OVERLAP_TOLERANCE_MS
            };

            if project.edit_plan.actions.iter().any(|existing| {
                same_range(existing)
                    && existing.source == EditActionSource::User
                    && (existing.is_manual_modified == Some(true) || !existing.enabled)
            }) {
                continue;
            }
            if project.edit_plan.actions.iter().any(same_range) {
                continue;
            }

            project.edit_plan.actions.push(EditAction {
                id: Uuid::new_v4().to_string(),
                action_type,
                source_media_id: ai_action.source_media_id,
                start_ms: ai_action.start_ms,
                end_ms: ai_action.end_ms,
                source: EditActionSource::Ai,
                reason: format!("{} ({})", ai_action.reason, ai_action.taxonomy),
                confidence: Some(ai_action.confidence),
                enabled: true,
                is_manual_modified: None,
                created_at: 0,
                updated_at: 0,
                payload: None,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::edit_plan::{EditAction, EditActionSource, EditActionType};

    fn ai(start_ms: u64, end_ms: u64) -> AIEditAction {
        AIEditAction {
            id: "ai-1".into(),
            source_media_id: "media-1".into(),
            start_ms,
            end_ms,
            action: "CUT".into(),
            reason: "false_start".into(),
            confidence: 0.9,
            taxonomy: "false_start".into(),
            source: "ai".into(),
            segment_ids: vec!["segment-1".into()],
        }
    }

    fn project_with(actions: Vec<EditAction>) -> Project {
        let mut project = Project::default();
        project.edit_plan.actions = actions;
        project
    }

    fn action(source: EditActionSource, enabled: bool) -> EditAction {
        EditAction {
            id: "existing".into(),
            action_type: EditActionType::Cut,
            source_media_id: "media-1".into(),
            start_ms: 1_000,
            end_ms: 2_000,
            source: source.clone(),
            reason: "existing".into(),
            confidence: None,
            enabled,
            is_manual_modified: (source == EditActionSource::User).then_some(true),
            created_at: 0,
            updated_at: 0,
            payload: None,
        }
    }

    #[test]
    fn merges_new_cut() {
        let mut project = project_with(vec![]);
        AiMergerService::merge_proposals(&mut project, vec![ai(1_000, 2_000)]);
        assert_eq!(project.edit_plan.actions.len(), 1);
        assert_eq!(project.edit_plan.actions[0].source, EditActionSource::Ai);
    }

    #[test]
    fn deduplicates_existing_cut_with_tolerance() {
        let mut project = project_with(vec![action(EditActionSource::Local, true)]);
        AiMergerService::merge_proposals(&mut project, vec![ai(1_050, 1_950)]);
        assert_eq!(project.edit_plan.actions.len(), 1);
    }

    #[test]
    fn preserves_user_rejection() {
        let mut project = project_with(vec![action(EditActionSource::User, false)]);
        AiMergerService::merge_proposals(&mut project, vec![ai(1_000, 2_000)]);
        assert_eq!(project.edit_plan.actions.len(), 1);
        assert!(!project.edit_plan.actions[0].enabled);
    }
}
