use crate::models::edit_plan::{EditActionType, EditPlan};
use crate::models::project::MediaSource;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
    pub level: IssueLevel,
    pub message: String,
    pub action_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum IssueLevel {
    Error,
    Warning,
}

/// Validate the canonical flat EditPlan contract and merge only overlapping,
/// enabled cut actions on the same source media.
pub fn validate_and_normalize(
    plan: EditPlan,
    media: &[MediaSource],
) -> (EditPlan, Vec<ValidationIssue>) {
    let mut issues = Vec::new();
    let media_durations: HashMap<String, u64> = media
        .iter()
        .map(|item| {
            (
                item.id.clone(),
                (item.metadata.duration_sec * 1000.0) as u64,
            )
        })
        .collect();
    let mut seen_ids = HashSet::new();
    let mut valid_actions = Vec::new();

    for action in plan.actions {
        let mut valid = true;
        if !seen_ids.insert(action.id.clone()) {
            issues.push(issue(
                IssueLevel::Error,
                "Duplicate action ID",
                Some(action.id.clone()),
            ));
            valid = false;
        }
        let Some(duration_ms) = media_durations.get(&action.source_media_id).copied() else {
            issues.push(issue(
                IssueLevel::Error,
                &format!("Unknown source_media_id: {}", action.source_media_id),
                Some(action.id.clone()),
            ));
            valid = false;
            if valid {
                valid_actions.push(action);
            }
            continue;
        };

        if action.start_ms >= action.end_ms {
            issues.push(issue(
                IssueLevel::Error,
                "start_ms must be strictly less than end_ms",
                Some(action.id.clone()),
            ));
            valid = false;
        }
        if action.end_ms > duration_ms {
            issues.push(issue(
                IssueLevel::Error,
                &format!(
                    "Action end_ms ({}) exceeds media duration ({duration_ms})",
                    action.end_ms
                ),
                Some(action.id.clone()),
            ));
            valid = false;
        }
        if valid {
            valid_actions.push(action);
        }
    }

    valid_actions.sort_by_key(|action| (action.source_media_id.clone(), action.start_ms));
    let mut normalized: Vec<crate::models::edit_plan::EditAction> =
        Vec::with_capacity(valid_actions.len());
    let mut cut_ranges: Vec<(String, u64, u64)> = Vec::new();
    for action in valid_actions {
        if action.action_type == EditActionType::Cut && action.enabled {
            if let Some(previous) = normalized.last_mut() {
                if previous.action_type == EditActionType::Cut
                    && previous.enabled
                    && previous.source_media_id == action.source_media_id
                    && action.start_ms <= previous.end_ms
                {
                    let old_end = previous.end_ms;
                    previous.end_ms = previous.end_ms.max(action.end_ms);
                    cut_ranges.push((
                        previous.source_media_id.clone(),
                        previous.start_ms,
                        previous.end_ms,
                    ));
                    issues.push(issue(
                        IssueLevel::Warning,
                        &format!(
                            "Merged overlapping cuts: {}->{} and {}->{}",
                            previous.start_ms, old_end, action.start_ms, action.end_ms
                        ),
                        Some(action.id),
                    ));
                    continue;
                }
            }
            cut_ranges.push((
                action.source_media_id.clone(),
                action.start_ms,
                action.end_ms,
            ));
        }
        normalized.push(action);
    }

    for action in &mut normalized {
        if action.action_type == EditActionType::Cut || !action.enabled {
            continue;
        }
        if cut_ranges.iter().any(|(media_id, start, end)| {
            media_id == &action.source_media_id
                && action.start_ms >= *start
                && action.end_ms <= *end
        }) {
            action.enabled = false;
            issues.push(issue(
                IssueLevel::Warning,
                "Action overlaps completely with enabled Cut, disabling action.",
                Some(action.id.clone()),
            ));
        }
    }

    (
        EditPlan {
            actions: normalized,
        },
        issues,
    )
}

fn issue(level: IssueLevel, message: &str, action_id: Option<String>) -> ValidationIssue {
    ValidationIssue {
        level,
        message: message.to_string(),
        action_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::edit_plan::{EditAction, EditActionSource};
    use crate::models::media_info::MediaSourceMetadata;

    fn media() -> Vec<MediaSource> {
        vec![MediaSource {
            id: "vid_1".to_string(),
            path: "vid_1.mp4".to_string(),
            metadata: MediaSourceMetadata {
                path: "vid_1.mp4".to_string(),
                duration_sec: 10.0,
                fps: 30.0,
                width: 1920,
                height: 1080,
                video_codec: "h264".to_string(),
                audio_codec: None,
                rotation: 0,
            },
        }]
    }

    fn cut(id: &str, start_ms: u64, end_ms: u64) -> EditAction {
        EditAction {
            id: id.to_string(),
            action_type: EditActionType::Cut,
            source_media_id: "vid_1".to_string(),
            start_ms,
            end_ms,
            source: EditActionSource::Local,
            reason: "silence".to_string(),
            confidence: None,
            enabled: true,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn rejects_out_of_bounds_actions() {
        let (plan, issues) = validate_and_normalize(
            EditPlan {
                actions: vec![cut("1", 9_000, 12_000)],
            },
            &media(),
        );
        assert!(plan.actions.is_empty());
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("exceeds media duration"));
    }

    #[test]
    fn merges_overlapping_cuts() {
        let (plan, issues) = validate_and_normalize(
            EditPlan {
                actions: vec![cut("1", 1_000, 3_000), cut("2", 2_000, 4_000)],
            },
            &media(),
        );
        assert_eq!(plan.actions.len(), 1);
        assert_eq!(plan.actions[0].start_ms, 1_000);
        assert_eq!(plan.actions[0].end_ms, 4_000);
        assert_eq!(issues.len(), 1);
    }
}
