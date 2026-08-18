use crate::models::project::{Transcript, TranscriptSegment};
use crate::models::whisper::WhisperResult;
use anyhow::{Context, Result};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct TranscriptParser;

impl TranscriptParser {
    pub fn parse(
        whisper_result: WhisperResult,
        source_id: &str,
        model_id: &str,
        language: &str,
    ) -> Result<Transcript> {
        let mut segments = Vec::new();

        for seg in whisper_result.transcription {
            let start_ms = Self::parse_timestamp(&seg.timestamps.from).unwrap_or(0);
            let mut end_ms = Self::parse_timestamp(&seg.timestamps.to).unwrap_or(0);
            
            // Validate timestamps
            if end_ms < start_ms {
                end_ms = start_ms;
            }

            // Trim text and preserve Unicode
            let text = seg.text.trim().to_string();

            if text.is_empty() {
                continue;
            }

            let segment = TranscriptSegment {
                id: uuid::Uuid::new_v4().to_string(),
                text,
                original_text: None,
                start_ms,
                end_ms,
                speaker: None,
                is_filler: false,
                is_modified: false,
            };
            segments.push(segment);
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Ok(Transcript {
            id: uuid::Uuid::new_v4().to_string(),
            source_id: source_id.to_string(),
            model_id: model_id.to_string(),
            language: language.to_string(),
            generated_at: now,
            segments,
        })
    }

    /// Parse timestamp in format "HH:MM:SS,mmm" into milliseconds
    fn parse_timestamp(ts: &str) -> Result<u64> {
        let parts: Vec<&str> = ts.split(',').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid timestamp format: {}", ts);
        }
        
        let time_parts: Vec<&str> = parts[0].split(':').collect();
        if time_parts.len() != 3 {
            anyhow::bail!("Invalid time format: {}", parts[0]);
        }

        let hours: u64 = time_parts[0].parse()?;
        let minutes: u64 = time_parts[1].parse()?;
        let seconds: u64 = time_parts[2].parse()?;
        let millis: u64 = parts[1].parse()?;

        Ok((hours * 3600000) + (minutes * 60000) + (seconds * 1000) + millis)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::whisper::{WhisperTimestamp, WhisperOffset, WhisperSegment};

    #[test]
    fn test_parse_timestamp() {
        assert_eq!(TranscriptParser::parse_timestamp("00:00:01,000").unwrap(), 1000);
        assert_eq!(TranscriptParser::parse_timestamp("01:02:03,456").unwrap(), 3723456);
    }
    
    #[test]
    fn test_parse_whisper_result() {
        let result = WhisperResult {
            transcription: vec![
                WhisperSegment {
                    timestamps: WhisperTimestamp {
                        from: "00:00:01,000".to_string(),
                        to: "00:00:02,500".to_string(),
                    },
                    offsets: WhisperOffset { from: 0, to: 0 },
                    text: "   Xin chào, đây là tiếng Việt!   ".to_string(),
                }
            ]
        };

        let transcript = TranscriptParser::parse(result, "src1", "model1", "vi").unwrap();
        assert_eq!(transcript.segments.len(), 1);
        assert_eq!(transcript.segments[0].text, "Xin chào, đây là tiếng Việt!");
        assert_eq!(transcript.segments[0].start_ms, 1000);
        assert_eq!(transcript.segments[0].end_ms, 2500);
    }
}
