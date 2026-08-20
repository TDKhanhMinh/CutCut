use crate::models::project::{Transcript, TranscriptSegment};
use crate::models::whisper::{WhisperResult, WhisperSegment};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TranscriptParseError {
    #[error("{field} metadata cannot be empty")]
    EmptyMetadata { field: &'static str },

    #[error("invalid {field} timestamp `{value}`")]
    InvalidTimestamp { field: &'static str, value: String },

    #[error("segment {index} ends before it starts ({start_ms} > {end_ms})")]
    InvalidRange {
        index: usize,
        start_ms: u64,
        end_ms: u64,
    },

    #[error("segment {index} is out of order ({start_ms} < previous {previous_start_ms})")]
    OutOfOrder {
        index: usize,
        start_ms: u64,
        previous_start_ms: u64,
    },
}

pub struct TranscriptParser;

impl TranscriptParser {
    pub fn parse(
        whisper_result: WhisperResult,
        source_id: &str,
        model_id: &str,
        language: &str,
    ) -> Result<Transcript, TranscriptParseError> {
        let source_id = require_metadata(source_id, "source_id")?;
        let model_id = require_metadata(model_id, "model_id")?;
        let language = normalize_language(language)?;
        let mut segments = Vec::new();
        let mut previous_start_ms = None;

        for (index, raw_segment) in whisper_result.transcription.iter().enumerate() {
            let start_ms = Self::parse_timestamp(&raw_segment.timestamps.from, "start")?;
            let end_ms = Self::parse_timestamp(&raw_segment.timestamps.to, "end")?;

            if end_ms < start_ms {
                return Err(TranscriptParseError::InvalidRange {
                    index,
                    start_ms,
                    end_ms,
                });
            }
            if let Some(previous_start_ms) = previous_start_ms {
                if start_ms < previous_start_ms {
                    return Err(TranscriptParseError::OutOfOrder {
                        index,
                        start_ms,
                        previous_start_ms,
                    });
                }
            }
            previous_start_ms = Some(start_ms);

            let text = normalize_text(&raw_segment.text);
            if text.is_empty() {
                continue;
            }

            let id = stable_segment_id(source_id, model_id, &language, raw_segment, &text);
            segments.push(TranscriptSegment {
                id,
                text,
                // Whitespace normalization is parser-owned, not a user edit;
                // reserve original_text/is_modified for the editor audit flow.
                original_text: None,
                start_ms,
                end_ms,
                speaker: None,
                is_filler: false,
                is_modified: false,
            });
        }

        Ok(Transcript {
            id: stable_transcript_id(source_id, model_id, &language),
            source_id: source_id.to_string(),
            model_id: model_id.to_string(),
            language,
            generated_at: now_millis(),
            segments,
        })
    }

    /// Parse Whisper's `HH:MM:SS,mmm` timestamp into project milliseconds.
    /// A dot separator is accepted as well because some Whisper JSON fixtures
    /// use the ISO-like `HH:MM:SS.mmm` spelling.
    fn parse_timestamp(timestamp: &str, field: &'static str) -> Result<u64, TranscriptParseError> {
        let Some(separator) = timestamp.find([',', '.']) else {
            return Err(invalid_timestamp(field, timestamp));
        };
        let clock = &timestamp[..separator];
        let milliseconds = &timestamp[separator + 1..];
        let clock_parts: Vec<&str> = clock.split(':').collect();
        if clock_parts.len() != 3 || milliseconds.is_empty() || milliseconds.len() > 3 {
            return Err(invalid_timestamp(field, timestamp));
        }

        let hours = clock_parts[0]
            .parse::<u64>()
            .map_err(|_| invalid_timestamp(field, timestamp))?;
        let minutes = clock_parts[1]
            .parse::<u64>()
            .map_err(|_| invalid_timestamp(field, timestamp))?;
        let seconds = clock_parts[2]
            .parse::<u64>()
            .map_err(|_| invalid_timestamp(field, timestamp))?;
        let milliseconds = milliseconds
            .parse::<u64>()
            .map_err(|_| invalid_timestamp(field, timestamp))?;
        if minutes >= 60 || seconds >= 60 || milliseconds >= 1_000 {
            return Err(invalid_timestamp(field, timestamp));
        }

        let scale = 10_u64.pow(3 - (timestamp.len() - separator - 1) as u32);
        hours
            .checked_mul(3_600_000)
            .and_then(|value| value.checked_add(minutes * 60_000))
            .and_then(|value| value.checked_add(seconds * 1_000))
            .and_then(|value| value.checked_add(milliseconds * scale))
            .ok_or_else(|| invalid_timestamp(field, timestamp))
    }
}

fn require_metadata<'a>(
    value: &'a str,
    field: &'static str,
) -> Result<&'a str, TranscriptParseError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(TranscriptParseError::EmptyMetadata { field });
    }
    Ok(value)
}

fn normalize_language(language: &str) -> Result<String, TranscriptParseError> {
    let language = language.trim();
    if language.is_empty() {
        return Ok("auto".to_string());
    }
    Ok(language.to_ascii_lowercase())
}

fn normalize_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn stable_transcript_id(source_id: &str, model_id: &str, language: &str) -> String {
    format!(
        "transcript-{}",
        digest_hex(&[source_id, model_id, language])
    )
}

fn stable_segment_id(
    source_id: &str,
    model_id: &str,
    language: &str,
    raw_segment: &WhisperSegment,
    normalized_text: &str,
) -> String {
    let offset_from = raw_segment.offsets.from.to_string();
    let offset_to = raw_segment.offsets.to.to_string();
    format!(
        "segment-{}",
        digest_hex(&[
            source_id,
            model_id,
            language,
            &raw_segment.timestamps.from,
            &raw_segment.timestamps.to,
            &offset_from,
            &offset_to,
            normalized_text,
        ])
    )
}

fn digest_hex(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part.as_bytes());
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn invalid_timestamp(field: &'static str, value: &str) -> TranscriptParseError {
    TranscriptParseError::InvalidTimestamp {
        field,
        value: value.to_string(),
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::whisper::{WhisperOffset, WhisperSegment, WhisperTimestamp};

    fn segment(from: &str, to: &str, text: &str, offset_from: i64) -> WhisperSegment {
        WhisperSegment {
            timestamps: WhisperTimestamp {
                from: from.to_string(),
                to: to.to_string(),
            },
            offsets: WhisperOffset {
                from: offset_from,
                to: offset_from + 100,
            },
            text: text.to_string(),
        }
    }

    #[test]
    fn parses_timestamp_to_milliseconds() {
        assert_eq!(
            TranscriptParser::parse_timestamp("00:00:01,000", "start").unwrap(),
            1_000
        );
        assert_eq!(
            TranscriptParser::parse_timestamp("01:02:03.456", "start").unwrap(),
            3_723_456
        );
    }

    #[test]
    fn rejects_invalid_timestamp_and_range() {
        assert!(matches!(
            TranscriptParser::parse_timestamp("00:60:00,000", "start"),
            Err(TranscriptParseError::InvalidTimestamp { .. })
        ));
        let result = TranscriptParser::parse(
            WhisperResult {
                transcription: vec![segment("00:00:02,000", "00:00:01,000", "bad", 0)],
            },
            "source",
            "model",
            "vi",
        );
        assert!(matches!(
            result,
            Err(TranscriptParseError::InvalidRange { .. })
        ));
    }

    #[test]
    fn rejects_out_of_order_segments_but_allows_small_overlap() {
        let valid = TranscriptParser::parse(
            WhisperResult {
                transcription: vec![
                    segment("00:00:00,000", "00:00:02,000", "one", 0),
                    segment("00:00:01,900", "00:00:03,000", "two", 100),
                ],
            },
            "source",
            "model",
            "vi",
        );
        assert!(valid.is_ok());

        let invalid = TranscriptParser::parse(
            WhisperResult {
                transcription: vec![
                    segment("00:00:02,000", "00:00:03,000", "one", 0),
                    segment("00:00:01,000", "00:00:02,000", "two", 100),
                ],
            },
            "source",
            "model",
            "vi",
        );
        assert!(matches!(
            invalid,
            Err(TranscriptParseError::OutOfOrder { .. })
        ));
    }

    #[test]
    fn raw_whisper_fixture_preserves_unicode_and_punctuation() {
        let raw = r#"{
            "transcription": [
                {"timestamps":{"from":"00:00:00,000","to":"00:00:01,250"},"offsets":{"from":0,"to":1250},"text":"  Xin   chào, Việt Nam!  "},
                {"timestamps":{"from":"00:00:01,200","to":"00:00:02,000"},"offsets":{"from":1200,"to":2000},"text":"Ừm — tiếp tục nhé…"}
            ]
        }"#;
        let result: WhisperResult = serde_json::from_str(raw).unwrap();
        let transcript = TranscriptParser::parse(result, "src1", "model1", "VI").unwrap();
        assert_eq!(transcript.language, "vi");
        assert_eq!(transcript.segments[0].text, "Xin chào, Việt Nam!");
        assert_eq!(transcript.segments[1].text, "Ừm — tiếp tục nhé…");
        assert_eq!(transcript.segments[0].start_ms, 0);
        assert_eq!(transcript.segments[0].end_ms, 1_250);
    }

    #[test]
    fn segment_ids_are_stable_across_reparse() {
        let result = WhisperResult {
            transcription: vec![segment("00:00:01,000", "00:00:02,500", "Xin chào", 10)],
        };
        let first = TranscriptParser::parse(result, "src1", "model1", "vi").unwrap();
        let second = TranscriptParser::parse(
            WhisperResult {
                transcription: vec![segment("00:00:01,000", "00:00:02,500", "Xin chào", 10)],
            },
            "src1",
            "model1",
            "vi",
        )
        .unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(first.segments[0].id, second.segments[0].id);
    }
}
