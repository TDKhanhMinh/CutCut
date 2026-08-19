use serde::{Deserialize, Serialize};
use crate::models::edit_plan::{EditPlan, EditAction, ActionPayload};
use crate::models::project::MediaSource;
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

pub fn validate_and_normalize(plan: EditPlan, media: &[MediaSource]) -> (EditPlan, Vec<ValidationIssue>) {
    let mut issues = Vec::new();
    let mut normalized_actions = Vec::new();

    // Map of media durations
    let media_durations: HashMap<String, u64> = media
        .iter()
        .map(|m| (m.id.clone(), (m.metadata.duration_sec * 1000.0) as u64))
        .collect();

    let mut seen_ids = HashSet::new();

    let EditPlan { version, actions, generation_metadata } = plan;

    for action in actions {
        let mut is_valid = true;

        // Rule 1: Duplicate ID
        if !seen_ids.insert(action.id.clone()) {
            issues.push(ValidationIssue {
                level: IssueLevel::Error,
                message: "Duplicate action ID".to_string(),
                action_id: Some(action.id.clone()),
            });
            is_valid = false;
        }

        // Rule 2: Missing source_media_id
        if !media_durations.contains_key(&action.source_media_id) {
            issues.push(ValidationIssue {
                level: IssueLevel::Error,
                message: format!("Unknown source_media_id: {}", action.source_media_id),
                action_id: Some(action.id.clone()),
            });
            is_valid = false;
        }

        let media_duration = media_durations.get(&action.source_media_id).cloned().unwrap_or(0);

        // Rule 3: Valid timestamps and bounds
        let (start_ms, end_ms) = match &action.payload {
            ActionPayload::Cut { start_ms, end_ms } => (*start_ms, *end_ms),
            ActionPayload::Highlight { start_ms, end_ms } => (*start_ms, *end_ms),
            ActionPayload::Zoom { start_ms, end_ms, scale, .. } => {
                if *scale <= 0.0 {
                    issues.push(ValidationIssue {
                        level: IssueLevel::Error,
                        message: "Zoom scale must be strictly positive".to_string(),
                        action_id: Some(action.id.clone()),
                    });
                    is_valid = false;
                }
                (*start_ms, *end_ms)
            }
            ActionPayload::Caption { start_ms, end_ms, .. } => (*start_ms, *end_ms),
        };

        if start_ms >= end_ms {
            issues.push(ValidationIssue {
                level: IssueLevel::Error,
                message: "start_ms must be strictly less than end_ms".to_string(),
                action_id: Some(action.id.clone()),
            });
            is_valid = false;
        }

        // Only enforce bounds if media exists (so media_duration > 0)
        if media_durations.contains_key(&action.source_media_id) && end_ms > media_duration {
            issues.push(ValidationIssue {
                level: IssueLevel::Error,
                message: format!("Action end_ms ({}) exceeds media duration ({})", end_ms, media_duration),
                action_id: Some(action.id.clone()),
            });
            is_valid = false;
        }

        if is_valid {
            normalized_actions.push(action);
        }
    }

    // Now, Overlap Normalization Logic
    // For simplicity, we just sort the actions by start_ms
    normalized_actions.sort_by_key(|a| match a.payload {
        ActionPayload::Cut { start_ms, .. } => start_ms,
        ActionPayload::Zoom { start_ms, .. } => start_ms,
        ActionPayload::Highlight { start_ms, .. } => start_ms,
        ActionPayload::Caption { start_ms, .. } => start_ms,
    });

    let mut final_actions: Vec<EditAction> = Vec::new();
    let mut cut_intervals: Vec<(u64, u64)> = Vec::new();

    // Pass 1: Normalize cuts
    for action in normalized_actions.drain(..) {
        if let ActionPayload::Cut { start_ms, end_ms } = action.payload {
            if action.enabled {
                if let Some(last_cut) = final_actions.last_mut() {
                    if let ActionPayload::Cut { start_ms: last_start, end_ms: last_end } = last_cut.payload {
                        if last_cut.source_media_id == action.source_media_id && last_cut.enabled {
                            if start_ms <= last_end {
                                // Overlap found! Merge them.
                                let new_end = last_end.max(end_ms);
                                issues.push(ValidationIssue {
                                    level: IssueLevel::Warning,
                                    message: format!("Merged overlapping cuts: {}->{} and {}->{}", last_start, last_end, start_ms, end_ms),
                                    action_id: Some(action.id.clone()),
                                });
                                last_cut.payload = ActionPayload::Cut { start_ms: last_start, end_ms: new_end };
                                
                                // Update tracking
                                if let Some(last_interval) = cut_intervals.last_mut() {
                                    last_interval.1 = new_end;
                                }
                                continue;
                            }
                        }
                    }
                }
                cut_intervals.push((start_ms, end_ms));
            }
        }
        final_actions.push(action);
    }

    // Pass 2: Check Zooms and Captions against active Cuts
    for action in final_actions.iter_mut() {
        let (start_ms, end_ms, is_cut) = match &action.payload {
            ActionPayload::Cut { .. } => (0, 0, true),
            ActionPayload::Zoom { start_ms, end_ms, .. } => (*start_ms, *end_ms, false),
            ActionPayload::Highlight { start_ms, end_ms } => (*start_ms, *end_ms, false),
            ActionPayload::Caption { start_ms, end_ms, .. } => (*start_ms, *end_ms, false),
        };

        if is_cut || !action.enabled {
            continue;
        }

        // Check if [start_ms, end_ms] is completely inside any Cut interval
        for (cut_start, cut_end) in &cut_intervals {
            if start_ms >= *cut_start && end_ms <= *cut_end {
                action.enabled = false;
                issues.push(ValidationIssue {
                    level: IssueLevel::Warning,
                    message: "Action overlaps completely with enabled Cut, disabling action.".to_string(),
                    action_id: Some(action.id.clone()),
                });
                break;
            }
        }
    }

    let result_plan = EditPlan {
        version,
        actions: final_actions,
        generation_metadata,
    };

    (result_plan, issues)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::edit_plan::ActionSource;
    use crate::models::media_info::MediaSourceMetadata;

    fn dummy_media() -> Vec<MediaSource> {
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

    fn dummy_cut(id: &str, start_ms: u64, end_ms: u64, enabled: bool) -> EditAction {
        EditAction {
            id: id.to_string(),
            source_media_id: "vid_1".to_string(),
            payload: ActionPayload::Cut { start_ms, end_ms },
            source: ActionSource::LocalDetector,
            reason: "silence".to_string(),
            confidence: None,
            enabled,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn test_valid_plan() {
        let plan = EditPlan {
            version: 1,
            actions: vec![dummy_cut("1", 0, 1000, true)],
            generation_metadata: None,
        };
        let (norm_plan, issues) = validate_and_normalize(plan, &dummy_media());
        assert!(issues.is_empty());
        assert_eq!(norm_plan.actions.len(), 1);
    }

    #[test]
    fn test_out_of_bounds() {
        let plan = EditPlan {
            version: 1,
            actions: vec![dummy_cut("1", 9000, 12000, true)], // duration is 10s (10000ms)
            generation_metadata: None,
        };
        let (norm_plan, issues) = validate_and_normalize(plan, &dummy_media());
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, IssueLevel::Error);
        assert!(issues[0].message.contains("exceeds media duration"));
        assert!(norm_plan.actions.is_empty());
    }

    #[test]
    fn test_overlap_cut_normalization() {
        let plan = EditPlan {
            version: 1,
            actions: vec![
                dummy_cut("1", 1000, 3000, true),
                dummy_cut("2", 2000, 4000, true),
            ],
            generation_metadata: None,
        };
        let (norm_plan, issues) = validate_and_normalize(plan, &dummy_media());
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, IssueLevel::Warning);
        assert!(issues[0].message.contains("Merged overlapping cuts"));
        
        assert_eq!(norm_plan.actions.len(), 1);
        if let ActionPayload::Cut { start_ms, end_ms } = norm_plan.actions[0].payload {
            assert_eq!(start_ms, 1000);
            assert_eq!(end_ms, 4000);
        } else {
            panic!("Expected Cut");
        }
    }

    #[test]
    fn test_zoom_inside_cut() {
        let mut zoom = dummy_cut("2", 1500, 2500, true);
        zoom.payload = ActionPayload::Zoom {
            start_ms: 1500,
            end_ms: 2500,
            scale: 1.5,
            anchor_x: 0.5,
            anchor_y: 0.5,
            easing: "linear".to_string(),
        };

        let plan = EditPlan {
            version: 1,
            actions: vec![
                dummy_cut("1", 1000, 3000, true),
                zoom,
            ],
            generation_metadata: None,
        };

        let (norm_plan, issues) = validate_and_normalize(plan, &dummy_media());
        // 1 warning for the overlap
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, IssueLevel::Warning);

        assert_eq!(norm_plan.actions.len(), 2);
        // Zoom should be disabled
        assert!(!norm_plan.actions[1].enabled);
    }
}
