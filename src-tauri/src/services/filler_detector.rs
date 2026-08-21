use crate::models::edit_plan::{EditAction, EditActionSource, EditActionType};
use crate::models::project::TranscriptSegment;
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

// ──────────────────────────────────────────────────────────────────────────────
// Filler Dictionary (V1 — Vietnamese)
// ──────────────────────────────────────────────────────────────────────────────

/// A filler entry with its canonical form and optional display label.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillerEntry {
    /// Normalised (NFC, lowercase) token to match against.
    pub token: String,
    /// Human-readable label shown in review UI.
    pub label: String,
}

/// The configurable dictionary powering the detector.
/// Extend by adding new `FillerEntry` values — no changes to detector core needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillerDictionary {
    pub version: String,
    pub entries: Vec<FillerEntry>,
}

impl Default for FillerDictionary {
    fn default() -> Self {
        let raw: &[(&str, &str)] = &[
            // Vietnamese filler syllables ─────────────────────────────────────
            ("ờ", "ờ"),
            ("ừ", "ừ"),
            ("ừm", "ừm"),
            ("à", "à"),
            ("ơ", "ơ"),
            ("ơm", "ơm"),
            ("uh", "uh"),
            ("um", "um"),
            ("uhm", "uhm"),
            ("ờm", "ờm"),
            ("ầm", "ầm"),
            ("này", "này"), // discourse-level — conservative
        ];

        Self {
            version: "1.0".to_string(),
            entries: raw
                .iter()
                .map(|(t, l)| FillerEntry {
                    token: normalize(t),
                    label: l.to_string(),
                })
                .collect(),
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Detector Output
// ──────────────────────────────────────────────────────────────────────────────

/// Precision level of the timing in the generated candidate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TimestampPrecision {
    /// Timestamps come from word-level alignment.
    WordLevel,
    /// Timestamps cover the whole segment — less precise.
    SegmentLevel,
}

/// A single filler candidate ready to be validated and turned into a CutSuggestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FillerCandidate {
    /// Unique deterministic ID: `filler_{segment_id}_{start_ms}_{end_ms}`.
    pub id: String,
    pub source_media_id: String,
    /// The matched filler token.
    pub matched_token: String,
    /// The segment text that contained the match.
    pub segment_text: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub precision: TimestampPrecision,
    /// True when the candidate needs human review before applying.
    pub review_required: bool,
}

impl FillerCandidate {
    /// Convert to a canonical `EditAction` ready for `EditPlan`.
    /// Confidence is set to `None` for segment-level timing (insufficient precision).
    pub fn to_edit_action(
        &self,
        created_at: u64,
        padding_ms: u64,
        media_duration_ms: u64,
    ) -> Option<EditAction> {
        // Apply pre/post padding clamped to [0, media_duration].
        let padded_start = self.start_ms.saturating_sub(padding_ms);
        let padded_end = (self.end_ms + padding_ms).min(media_duration_ms);

        // Start must still be strictly less than end.
        let (start_ms, end_ms) = if padded_start < padded_end {
            (padded_start, padded_end)
        } else {
            (
                self.start_ms.min(media_duration_ms),
                self.end_ms.min(media_duration_ms),
            )
        };

        if start_ms >= end_ms {
            return None;
        }

        let confidence = match self.precision {
            TimestampPrecision::WordLevel => Some(0.5),
            TimestampPrecision::SegmentLevel => None,
        };

        Some(EditAction {
            id: self.id.clone(),
            action_type: EditActionType::Cut,
            source_media_id: self.source_media_id.clone(),
            start_ms,
            end_ms,
            source: EditActionSource::Local,
            reason: format!("filler:{}", self.matched_token),
            confidence,
            // Disabled by default — user must review.
            enabled: false,
            is_manual_modified: None,
            created_at,
            updated_at: created_at,
            payload: None,
        })
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Detection Logic
// ──────────────────────────────────────────────────────────────────────────────

/// Detect filler words in the provided transcript segments.
///
/// Strategy:
/// - Tokenise segment text by whitespace and strip punctuation.
/// - Perform whole-token match against dictionary (no substring false-positives).
/// - Use segment-level timestamps (the only data available in V1 Whisper output).
/// - Mark all candidates as `review_required = true` since segment timing is coarse.
/// - Deduplicate: skip segments already marked `is_filler = true` upstream.
pub fn detect_fillers(
    source_media_id: &str,
    segments: &[TranscriptSegment],
    dictionary: &FillerDictionary,
) -> Vec<FillerCandidate> {
    let mut candidates = Vec::new();

    for segment in segments {
        if segment.start_ms >= segment.end_ms {
            continue;
        }
        // Skip segments already classified filler by upstream (Whisper / transcript parser).
        if segment.is_filler {
            continue;
        }

        let text = segment.original_text.as_deref().unwrap_or(&segment.text);
        let tokens = tokenise(text);

        for token in &tokens {
            if let Some(entry) = dictionary.entries.iter().find(|e| e.token == *token) {
                let id = format!(
                    "filler_{}_{}_{}",
                    segment.id, segment.start_ms, segment.end_ms
                );

                // Avoid generating duplicate candidates for same segment
                if candidates.iter().any(|c: &FillerCandidate| c.id == id) {
                    continue;
                }

                candidates.push(FillerCandidate {
                    id,
                    source_media_id: source_media_id.to_string(),
                    matched_token: entry.label.clone(),
                    segment_text: text.to_string(),
                    start_ms: segment.start_ms,
                    end_ms: segment.end_ms,
                    // V1 Whisper output only provides segment-level timestamps.
                    precision: TimestampPrecision::SegmentLevel,
                    // Always require human review — coarse timing.
                    review_required: true,
                });
            }
        }
    }

    candidates
}

// ──────────────────────────────────────────────────────────────────────────────
// String Utilities
// ──────────────────────────────────────────────────────────────────────────────

/// Normalize Unicode to NFC and lowercase at the detector boundary.
pub fn normalize(s: &str) -> String {
    s.nfc().collect::<String>().to_lowercase().nfc().collect()
}

/// Split text into whole tokens, strip punctuation, and normalise.
fn tokenise(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|word| normalize(word.trim_matches(|character: char| !character.is_alphanumeric())))
        .filter(|t| !t.is_empty())
        .collect()
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(id: &str, text: &str, start_ms: u64, end_ms: u64, is_filler: bool) -> TranscriptSegment {
        TranscriptSegment {
            id: id.to_string(),
            text: text.to_string(),
            original_text: None,
            start_ms,
            end_ms,
            speaker: None,
            is_filler,
            is_modified: false,
        }
    }

    #[test]
    fn test_standalone_filler_detected() {
        let dict = FillerDictionary::default();
        let segments = vec![seg("s1", "ờ thì mình sẽ bắt đầu", 0, 2000, false)];
        let candidates = detect_fillers("vid", &segments, &dict);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].matched_token, "ờ");
        assert_eq!(candidates[0].start_ms, 0);
        assert_eq!(candidates[0].end_ms, 2000);
        assert!(candidates[0].review_required);
        assert_eq!(candidates[0].precision, TimestampPrecision::SegmentLevel);
    }

    #[test]
    fn test_um_variant_detected() {
        let dict = FillerDictionary::default();
        let segments = vec![seg("s1", "ừm hôm nay chúng ta", 500, 3000, false)];
        let candidates = detect_fillers("vid", &segments, &dict);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].matched_token, "ừm");
    }

    #[test]
    fn normalizes_combining_marks_to_nfc_before_matching() {
        let dict = FillerDictionary::default();
        let segments = vec![seg("s1", "u\u{031b}\u{0300}m hôm nay", 500, 3000, false)];
        let candidates = detect_fillers("vid", &segments, &dict);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].matched_token, "ừm");
    }

    #[test]
    fn test_valid_word_not_matched() {
        let dict = FillerDictionary::default();
        // "màu" contains "à" but is NOT a standalone filler
        let segments = vec![seg("s1", "cái màu này đẹp", 0, 2000, false)];
        let candidates = detect_fillers("vid", &segments, &dict);
        // "này" IS in the dictionary — should match once; "màu" should not
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].matched_token, "này");
    }

    #[test]
    fn test_is_filler_segment_skipped() {
        let dict = FillerDictionary::default();
        // Segment already marked is_filler by upstream parser — skip it
        let segments = vec![seg("s1", "ờ", 0, 500, true)];
        let candidates = detect_fillers("vid", &segments, &dict);
        assert!(candidates.is_empty());
    }

    #[test]
    fn test_no_duplicate_candidate_per_segment() {
        let dict = FillerDictionary::default();
        // Segment contains two filler tokens — only 1 candidate per segment (same id)
        let segments = vec![seg("s1", "ừ ờ bắt đầu", 0, 1500, false)];
        let candidates = detect_fillers("vid", &segments, &dict);
        assert_eq!(candidates.len(), 1);
    }

    #[test]
    fn test_candidate_range_within_segment() {
        let dict = FillerDictionary::default();
        let segments = vec![seg("s1", "ờ okay", 1000, 3000, false)];
        let candidates = detect_fillers("vid", &segments, &dict);
        assert_eq!(candidates.len(), 1);
        // Candidate range must not exceed segment range
        assert!(candidates[0].start_ms >= 1000);
        assert!(candidates[0].end_ms <= 3000);
    }

    #[test]
    fn test_to_edit_action_has_correct_fields() {
        let candidate = FillerCandidate {
            id: "filler_s1_0_2000".to_string(),
            source_media_id: "vid".to_string(),
            matched_token: "ờ".to_string(),
            segment_text: "ờ thì".to_string(),
            start_ms: 0,
            end_ms: 2000,
            precision: TimestampPrecision::SegmentLevel,
            review_required: true,
        };

        let action = candidate.to_edit_action(12345, 50, 60000).unwrap();
        assert_eq!(action.source, EditActionSource::Local);
        assert!(action.reason.contains("filler"));
        assert!(
            !action.enabled,
            "must be disabled by default for user review"
        );
        assert_eq!(action.confidence, None, "segment-level → no confidence");

        assert_eq!(action.action_type, EditActionType::Cut);
        assert_eq!(action.start_ms, 0);
        assert_eq!(action.end_ms, 2050);
    }
}
