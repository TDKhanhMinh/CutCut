use crate::models::project::{CaptionCue, CaptionStyle};
use crate::models::edit_plan::EditPlan;
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub struct OutputCue {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

pub struct SubtitleGenerator;

impl SubtitleGenerator {
    /// Maps source-time cues to output-time cues by skipping over enabled cuts.
    pub fn map_to_output_timeline(
        cues: &[CaptionCue],
        edit_plan: &EditPlan,
    ) -> Vec<OutputCue> {
        // 1. Extract and sort enabled cuts
        let mut cuts: Vec<(u64, u64)> = edit_plan
            .actions
            .iter()
            .filter_map(|a| {
                if a.enabled {
                    if let crate::models::edit_plan::ActionPayload::Cut { start_ms, end_ms } = a.payload {
                        return Some((start_ms, end_ms));
                    }
                }
                None
            })
            .collect();
            
        // Note: The EditPlan validator should have already sorted and merged these,
        // but we sort again just in case to guarantee correctness.
        cuts.sort_by_key(|c| c.0);

        let mut output_cues = Vec::new();

        for cue in cues {
            let mut current_start = cue.start_ms;
            let cue_end = cue.end_ms;

            // Find intersections with cuts
            for &(cut_start, cut_end) in &cuts {
                if current_start >= cue_end {
                    break;
                }
                
                if cut_end <= current_start {
                    continue; // Cut is completely before this remaining cue part
                }
                
                if cut_start >= cue_end {
                    break; // All subsequent cuts are after this cue
                }

                // There is an overlap!
                // If there's a valid gap before the cut starts, emit it.
                if current_start < cut_start {
                    let sub_end = cut_start;
                    output_cues.push(OutputCue {
                        start_ms: current_start,
                        end_ms: sub_end,
                        text: cue.text.clone(),
                    });
                }
                
                // Advance the current_start to the end of the cut
                current_start = current_start.max(cut_end);
            }

            // Emit the remaining part of the cue, if any
            if current_start < cue_end {
                output_cues.push(OutputCue {
                    start_ms: current_start,
                    end_ms: cue_end,
                    text: cue.text.clone(),
                });
            }
        }

        // 2. Shift timestamps to output timeline
        for oc in &mut output_cues {
            oc.start_ms = Self::source_to_output_ms(oc.start_ms, &cuts);
            oc.end_ms = Self::source_to_output_ms(oc.end_ms, &cuts);
        }

        output_cues
    }

    /// Converts a source timestamp to output timestamp by subtracting the total duration
    /// of all cuts that occurred strictly before it.
    fn source_to_output_ms(source_ms: u64, sorted_cuts: &[(u64, u64)]) -> u64 {
        let mut shift = 0;
        for &(cut_start, cut_end) in sorted_cuts {
            if cut_end <= source_ms {
                shift += cut_end - cut_start;
            } else if cut_start < source_ms {
                // Should not happen if source_ms is already outside of cuts
                // (which is guaranteed by step 1), but handle defensively.
                shift += source_ms - cut_start;
            } else {
                break;
            }
        }
        source_ms.saturating_sub(shift)
    }

    /// Format time as H:MM:SS.cc (centiseconds) for ASS
    fn format_ass_time(ms: u64) -> String {
        let h = ms / 3600000;
        let m = (ms % 3600000) / 60000;
        let s = (ms % 60000) / 1000;
        let cs = (ms % 1000) / 10;
        format!("{}:{:02}:{:02}.{:02}", h, m, s, cs)
    }

    /// Convert #RRGGBB to ASS color &HAABBGGRR
    fn hex_to_ass_color(hex: &str, alpha_hex: &str) -> String {
        let clean = hex.trim_start_matches('#');
        if clean.len() >= 6 {
            let r = &clean[0..2];
            let g = &clean[2..4];
            let b = &clean[4..6];
            format!("&H{}{}{}{}", alpha_hex, b, g, r)
        } else {
            "&H00FFFFFF".to_string() // fallback white
        }
    }

    /// Convert HTML Alignment to ASS Alignment (numpad mapping)
    /// 1=BotL, 2=BotC, 3=BotR, 4=MidL, 5=MidC, 6=MidR, 7=TopL, 8=TopC, 9=TopR
    fn map_alignment(style: &CaptionStyle) -> u8 {
        // Simplified mapping based on bottom alignment since style_mapper positions from top
        // But ASS uses MarginV.
        match style.alignment.as_str() {
            "left" => 1, // Bottom-left
            "right" => 3, // Bottom-right
            _ => 2, // Bottom-center
        }
    }

    /// Generate ASS file content
    pub fn generate_ass_content(
        cues: &[CaptionCue],
        style: &CaptionStyle,
        edit_plan: &EditPlan,
        video_width: u32,
        video_height: u32,
    ) -> String {
        let output_cues = Self::map_to_output_timeline(cues, edit_plan);

        // Map styles
        let font_size = (style.font_size_vh * video_height as f64).round() as u32;
        let primary_color = Self::hex_to_ass_color(&style.primary_color, "00"); // Solid
        
        let outline_color = if let Some(ref c) = style.outline_color {
            Self::hex_to_ass_color(c, "00")
        } else {
            "&H00000000".to_string()
        };

        let outline_width = if let Some(w) = style.outline_width_vh {
            (w * video_height as f64).round() as u32
        } else {
            0
        };

        let back_color = if let Some(ref c) = style.background_color {
            // Opacity mapping (0.0 to 1.0) -> ASS Alpha (FF to 00)
            let opacity = style.background_opacity.unwrap_or(0.8);
            let alpha = (255.0 * (1.0 - opacity)).round() as u8;
            Self::hex_to_ass_color(c, &format!("{:02X}", alpha))
        } else {
            "&H80000000".to_string() // 50% black
        };

        let border_style = if style.background_color.is_some() { 3 } else { 1 }; // 1=Outline, 3=Opaque box
        
        // Convert positions from Top-Left origin (Project schema) to Bottom-Up MarginV (ASS schema)
        let margin_v = ((1.0 - style.position_y_vh) * video_height as f64).round() as u32;
        
        let alignment = Self::map_alignment(style);

        let mut lines = Vec::new();
        
        // Header
        lines.push("[Script Info]".to_string());
        lines.push("ScriptType: v4.00+".to_string());
        lines.push(format!("PlayResX: {}", video_width));
        lines.push(format!("PlayResY: {}", video_height));
        lines.push("WrapStyle: 1".to_string()); // End-of-line word wrapping
        lines.push("".to_string());

        // Styles
        lines.push("[V4+ Styles]".to_string());
        lines.push("Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding".to_string());
        
        let bold = if style.font_weight >= 600 { -1 } else { 0 };
        let italic = if style.font_style == "italic" { -1 } else { 0 };
        
        lines.push(format!(
            "Style: Default,{},{},{},&H000000FF,{},{},{},{},0,0,100,100,0,0,{},{},0,{},10,10,{},1",
            style.font_family, font_size, primary_color, outline_color, back_color, bold, italic, border_style, outline_width, alignment, margin_v
        ));
        lines.push("".to_string());

        // Events
        lines.push("[Events]".to_string());
        lines.push("Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text".to_string());

        for cue in output_cues {
            // Replace \n with ASS newline \N
            let safe_text = cue.text.replace("\n", "\\N");
            
            lines.push(format!(
                "Dialogue: 0,{},{},Default,,0,0,0,,{}",
                Self::format_ass_time(cue.start_ms),
                Self::format_ass_time(cue.end_ms),
                safe_text
            ));
        }

        lines.join("\n")
    }

    /// Writes the generated ASS content to a temp file and returns its path
    pub fn write_temp_ass_file(content: &str) -> Result<PathBuf, String> {
        let temp_dir = std::env::temp_dir();
        let file_name = format!("cutcut_subs_{}.ass", uuid::Uuid::new_v4());
        let file_path = temp_dir.join(file_name);
        
        std::fs::write(&file_path, content)
            .map_err(|e| format!("Failed to write temp subtitle file: {}", e))?;
            
        Ok(file_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::edit_plan::{EditAction, ActionPayload, ActionSource};

    fn make_cut(start: u64, end: u64) -> EditAction {
        EditAction {
            id: format!("cut_{}_{}", start, end),
            source_media_id: "media_1".to_string(),
            source: ActionSource::AiAgent,
            reason: "test".to_string(),
            confidence: None,
            enabled: true,
            is_manual_modified: None,
            created_at: 0,
            updated_at: 0,
            payload: ActionPayload::Cut {
                start_ms: start,
                end_ms: end,
            },
        }
    }

    fn make_cue(start: u64, end: u64) -> CaptionCue {
        CaptionCue {
            id: "1".into(),
            source_segment_ids: vec![],
            start_ms: start,
            end_ms: end,
            text: "Hello".into(),
            is_manual_modified: false,
        }
    }

    #[test]
    fn test_map_output_no_cuts() {
        let cues = vec![make_cue(1000, 3000)];
        let plan = EditPlan { version: 1, actions: vec![], generation_metadata: None };
        
        let out = SubtitleGenerator::map_to_output_timeline(&cues, &plan);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].start_ms, 1000);
        assert_eq!(out[0].end_ms, 3000);
    }

    #[test]
    fn test_map_output_cue_fully_inside_cut() {
        let cues = vec![make_cue(1000, 3000)];
        // Cut covers the entire cue
        let plan = EditPlan { version: 1, actions: vec![make_cut(500, 4000)], generation_metadata: None };
        
        let out = SubtitleGenerator::map_to_output_timeline(&cues, &plan);
        assert_eq!(out.len(), 0); // Cue is dropped!
    }

    #[test]
    fn test_map_output_cue_split_by_cut() {
        let cues = vec![make_cue(1000, 5000)];
        // Cut right in the middle
        let plan = EditPlan { version: 1, actions: vec![make_cut(2000, 3000)], generation_metadata: None };
        
        let out = SubtitleGenerator::map_to_output_timeline(&cues, &plan);
        assert_eq!(out.len(), 2);
        
        // First part: 1000 to 2000 (no cuts before it)
        assert_eq!(out[0].start_ms, 1000);
        assert_eq!(out[0].end_ms, 2000);
        
        // Second part: 3000 to 5000 (shifted left by 1000ms duration of the cut)
        assert_eq!(out[1].start_ms, 2000); // 3000 - 1000
        assert_eq!(out[1].end_ms, 4000);   // 5000 - 1000
    }

    #[test]
    fn test_map_output_cue_after_multiple_cuts() {
        let cues = vec![make_cue(10000, 12000)];
        let plan = EditPlan { 
            version: 1, 
            actions: vec![make_cut(1000, 2000), make_cut(5000, 7000)], // Total cuts = 3000ms
            generation_metadata: None 
        };
        
        let out = SubtitleGenerator::map_to_output_timeline(&cues, &plan);
        assert_eq!(out.len(), 1);
        
        assert_eq!(out[0].start_ms, 7000); // 10000 - 3000
        assert_eq!(out[0].end_ms, 9000);   // 12000 - 3000
    }

    #[test]
    fn test_hex_to_ass_color() {
        assert_eq!(SubtitleGenerator::hex_to_ass_color("#FF0000", "00"), "&H000000FF");
        assert_eq!(SubtitleGenerator::hex_to_ass_color("#00FF00", "00"), "&H0000FF00");
    }

    #[test]
    fn test_format_ass_time() {
        assert_eq!(SubtitleGenerator::format_ass_time(1000), "0:00:01.00");
        assert_eq!(SubtitleGenerator::format_ass_time(3600000), "1:00:00.00");
        assert_eq!(SubtitleGenerator::format_ass_time(12345), "0:00:12.34");
    }
}
