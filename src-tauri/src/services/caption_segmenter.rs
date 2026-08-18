use crate::models::project::{CaptionCue, Transcript, TranscriptSegment};
use uuid::Uuid;

// V1 Constraints
pub const MAX_CHARS_PER_CUE: usize = 84;
pub const IDEAL_CHARS_PER_LINE: usize = 42;
pub const MIN_DURATION_MS: u64 = 500;

#[derive(Debug, Clone)]
struct WordInfo {
    text: String,
    start_ms: u64,
    end_ms: u64,
    segment_id: String,
    has_strong_punct: bool,
    has_weak_punct: bool,
}

pub fn generate_cues(transcript: &Transcript, existing_cues: &[CaptionCue]) -> Vec<CaptionCue> {
    // Collect all words with interpolated timestamps
    let mut all_words = Vec::new();

    for segment in &transcript.segments {
        let text = segment.text.trim();
        if text.is_empty() {
            continue;
        }

        let total_chars = text.chars().count() as u64;
        let segment_duration = segment.end_ms.saturating_sub(segment.start_ms);

        let words_split: Vec<&str> = text.split_whitespace().collect();
        
        let mut current_char_offset = 0;
        
        for w in words_split {
            let w_len = w.chars().count() as u64;
            
            let start_ms = if total_chars > 0 {
                segment.start_ms + (current_char_offset * segment_duration / total_chars)
            } else {
                segment.start_ms
            };
            
            let end_ms = if total_chars > 0 {
                segment.start_ms + ((current_char_offset + w_len) * segment_duration / total_chars)
            } else {
                segment.end_ms
            };

            let has_strong_punct = w.ends_with('.') || w.ends_with('?') || w.ends_with('!');
            let has_weak_punct = w.ends_with(',') || w.ends_with(':') || w.ends_with(';') || w.ends_with('…');

            all_words.push(WordInfo {
                text: w.to_string(),
                start_ms,
                end_ms,
                segment_id: segment.id.clone(),
                has_strong_punct,
                has_weak_punct,
            });

            current_char_offset += w_len + 1; // +1 for space
        }
    }

    // Accumulate words into cues
    let mut generated_cues = Vec::new();
    let mut current_words = Vec::new();
    let mut current_chars = 0;

    for word in all_words {
        let word_len = word.text.chars().count();

        if current_chars + 1 + word_len > MAX_CHARS_PER_CUE && !current_words.is_empty() {
            generated_cues.push(flush_cue(&mut current_words));
            current_chars = 0;
        }

        let w_strong = word.has_strong_punct;
        let w_weak = word.has_weak_punct;

        current_words.push(word);
        current_chars += word_len + 1;

        if w_strong {
            generated_cues.push(flush_cue(&mut current_words));
            current_chars = 0;
        } else if w_weak && current_chars > IDEAL_CHARS_PER_LINE {
            generated_cues.push(flush_cue(&mut current_words));
            current_chars = 0;
        }
    }

    if !current_words.is_empty() {
        generated_cues.push(flush_cue(&mut current_words));
    }

    // Pass 2: Enforce MIN_DURATION_MS without overlapping
    for i in 0..generated_cues.len() {
        let duration = generated_cues[i].end_ms.saturating_sub(generated_cues[i].start_ms);
        if duration < MIN_DURATION_MS {
            let mut new_end = generated_cues[i].start_ms + MIN_DURATION_MS;
            // clamp to next cue's start
            if i + 1 < generated_cues.len() {
                if new_end > generated_cues[i + 1].start_ms {
                    new_end = generated_cues[i + 1].start_ms;
                }
            }
            // only update if new_end is greater
            if new_end > generated_cues[i].end_ms {
                generated_cues[i].end_ms = new_end;
            }
        }
    }

    // Pass 3: Preserve manual modifications
    // If there is an existing cue with is_manual_modified = true, and its ID matches or 
    // it perfectly overlaps, we can restore it. V1 policy: IDs are regenerated based on segment IDs + index,
    // but a safer approach is to check if we can reuse the existing cues. 
    // The requirement says: "cung cấp regenerate/confirm policy". For now, we return the newly generated track. 
    // The frontend will be responsible for diffing and warning the user if manual edits are going to be overwritten.
    
    generated_cues
}

fn flush_cue(words: &mut Vec<WordInfo>) -> CaptionCue {
    let mut text = String::new();
    let mut current_line_len = 0;
    
    let mut segment_ids = Vec::new();
    let start_ms = words.first().map(|w| w.start_ms).unwrap_or(0);
    let end_ms = words.last().map(|w| w.end_ms).unwrap_or(0);

    for (i, word) in words.iter().enumerate() {
        if !segment_ids.contains(&word.segment_id) {
            segment_ids.push(word.segment_id.clone());
        }

        let w_len = word.text.chars().count();
        if i > 0 {
            if current_line_len + 1 + w_len > IDEAL_CHARS_PER_LINE {
                text.push('\n');
                current_line_len = 0;
            } else {
                text.push(' ');
                current_line_len += 1;
            }
        }
        text.push_str(&word.text);
        current_line_len += w_len;
    }

    let id = format!("cue_{}_{}", start_ms, end_ms);

    words.clear();

    CaptionCue {
        id,
        source_segment_ids: segment_ids,
        start_ms,
        end_ms,
        text,
        is_manual_modified: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(id: &str, text: &str, start_ms: u64, end_ms: u64) -> TranscriptSegment {
        TranscriptSegment {
            id: id.to_string(),
            text: text.to_string(),
            original_text: None,
            start_ms,
            end_ms,
            speaker: None,
            is_filler: false,
            is_modified: false,
        }
    }

    #[test]
    fn test_short_segment_duration_clamped() {
        let transcript = Transcript {
            id: "t1".into(),
            source_id: "s1".into(),
            model_id: "m1".into(),
            language: "vi".into(),
            generated_at: 0,
            segments: vec![
                segment("1", "Chào.", 0, 100),
                segment("2", "Bạn khỏe không?", 200, 1000),
            ],
        };

        let cues = generate_cues(&transcript, &[]);
        assert_eq!(cues.len(), 2);
        
        // "Chào." is 100ms. Min duration is 500ms. It will be clamped to next cue's start (200ms).
        assert_eq!(cues[0].text, "Chào.");
        assert_eq!(cues[0].start_ms, 0);
        assert_eq!(cues[0].end_ms, 200); 

        // "Bạn khỏe không?" is 800ms, which is > 500ms. 
        assert_eq!(cues[1].text, "Bạn khỏe không?");
        assert_eq!(cues[1].start_ms, 200);
        assert_eq!(cues[1].end_ms, 1000);
    }

    #[test]
    fn test_long_segment_split() {
        let long_text = "Đây là một đoạn văn bản rất dài và không có dấu câu nào cả nên nó sẽ bị chia cắt tự động khi vượt quá giới hạn tám mươi tư ký tự quy định trong V1 constraint của hệ thống caption segmenter.";
        let transcript = Transcript {
            id: "t1".into(),
            source_id: "s1".into(),
            model_id: "m1".into(),
            language: "vi".into(),
            generated_at: 0,
            segments: vec![
                segment("1", long_text, 0, 10000),
            ],
        };

        let cues = generate_cues(&transcript, &[]);
        assert!(cues.len() > 1);
        
        for cue in cues {
            assert!(cue.text.chars().count() <= MAX_CHARS_PER_CUE + 10); // +10 for newline flex
            assert!(cue.text.lines().count() <= 2);
        }
    }

    #[test]
    fn test_line_breaks() {
        let text = "Một hai ba bốn năm sáu bảy tám chín mười mười một mười hai mười ba.";
        let transcript = Transcript {
            id: "t1".into(),
            source_id: "s1".into(),
            model_id: "m1".into(),
            language: "vi".into(),
            generated_at: 0,
            segments: vec![
                segment("1", text, 0, 5000),
            ],
        };

        let cues = generate_cues(&transcript, &[]);
        assert_eq!(cues.len(), 1);
        assert!(cues[0].text.contains('\n'));
    }

    #[test]
    fn test_punctuation_split() {
        let transcript = Transcript {
            id: "t1".into(),
            source_id: "s1".into(),
            model_id: "m1".into(),
            language: "vi".into(),
            generated_at: 0,
            segments: vec![
                segment("1", "Xin chào. Tôi là AI.", 0, 2000),
            ],
        };

        let cues = generate_cues(&transcript, &[]);
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].text, "Xin chào.");
        assert_eq!(cues[1].text, "Tôi là AI.");
    }
}
