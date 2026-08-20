use crate::models::project::CaptionStyle;

impl CaptionStyle {
    pub fn get_default_16_9_preset() -> Self {
        CaptionStyle {
            preset_id: "default_16_9".into(),
            font_family: "Arial".into(),
            font_weight: 700,
            font_style: "normal".into(),
            font_size_vh: 0.06,  // 6% of video height
            position_x_vw: 0.5,  // Center horizontally
            position_y_vh: 0.85, // 85% down from top
            alignment: "center".into(),
            primary_color: "#FFFFFF".into(),
            outline_color: Some("#000000".into()),
            outline_width_vh: Some(0.005), // 0.5% of height
            background_color: None,
            background_opacity: None,
        }
    }

    pub fn get_default_9_16_preset() -> Self {
        CaptionStyle {
            preset_id: "default_9_16".into(),
            font_family: "Arial".into(),
            font_weight: 700,
            font_style: "normal".into(),
            font_size_vh: 0.04, // smaller relative to height because height is so large in 9:16
            position_x_vw: 0.5,
            position_y_vh: 0.8, // a bit higher to avoid tiktok UI
            alignment: "center".into(),
            primary_color: "#FFFFFF".into(),
            outline_color: Some("#000000".into()),
            outline_width_vh: Some(0.004),
            background_color: None,
            background_opacity: None,
        }
    }

    /// Generates FFmpeg drawtext filter parameters string.
    ///
    /// Note:
    /// - Does not include `text` or `enable` properties as those vary per cue.
    /// - `fontfile` resolution relies on FFmpeg's fontconfig on the system for V1.
    pub fn to_ffmpeg_drawtext_args(&self, _video_width: u32, video_height: u32) -> String {
        let font_size = (self.font_size_vh.clamp(0.01, 0.25) * video_height as f64)
            .round()
            .max(1.0) as u32;
        let x = self.position_x_vw.clamp(0.0, 1.0);
        let y = self.position_y_vh.clamp(0.0, 1.0);
        let primary_color = normalize_color(&self.primary_color, "FFFFFF");
        let font_reference = normalize_font_reference(&self.font_family);

        let mut args = vec![
            format!("fontfile='{font_reference}'"),
            format!("fontsize={}", font_size),
            format!("fontcolor=0x{primary_color}"),
        ];

        // Alignment logic (center is default)
        let x_expr = match self.alignment.as_str() {
            "left" => format!("(w*{x})"),
            "right" => format!("(w*{x})-text_w"),
            _ => "(w-text_w)/2".to_string(), // Center
        };

        let y_expr = format!("(h*{y})-text_h");

        args.push(format!("x='{}'", x_expr));
        args.push(format!("y='{}'", y_expr));

        if let Some(ref o_col) = self.outline_color {
            if let Some(o_width) = self.outline_width_vh {
                let borderw = (o_width.clamp(0.0, 0.05) * video_height as f64)
                    .round()
                    .max(0.0) as u32;
                if borderw > 0 {
                    args.push(format!("borderw={}", borderw));
                    args.push(format!(
                        "bordercolor=0x{}",
                        normalize_color(o_col, "000000")
                    ));
                }
            }
        }

        if let Some(ref bg_col) = self.background_color {
            let opacity = self.background_opacity.unwrap_or(1.0).clamp(0.0, 1.0);
            args.push("box=1".to_string());
            args.push(format!(
                "boxcolor={}@{}",
                normalize_color(bg_col, "000000"),
                opacity
            ));
            args.push("boxborderw=5".to_string());
        }

        args.join(":")
    }
}

fn normalize_color(value: &str, fallback: &str) -> String {
    let candidate = value.trim().trim_start_matches('#');
    if candidate.len() == 6
        && candidate
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        candidate.to_ascii_uppercase()
    } else {
        fallback.to_string()
    }
}

/// V1 uses a system-font reference resolved by the FFmpeg/fontconfig runtime.
/// Reject path separators and filter delimiters so a project cannot smuggle a
/// filesystem path or drawtext expression through the semantic style model.
fn normalize_font_reference(value: &str) -> String {
    let candidate = value.trim();
    if !candidate.is_empty()
        && candidate.len() <= 64
        && candidate
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || " -_".contains(character))
    {
        candidate.to_string()
    } else {
        "Arial".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffmpeg_drawtext_args_16_9() {
        let style = CaptionStyle::get_default_16_9_preset();
        let args = style.to_ffmpeg_drawtext_args(1920, 1080);

        assert!(args.contains("fontfile='Arial'"));
        assert!(args.contains("fontsize=65")); // 0.06 * 1080 = 64.8 => 65
        assert!(args.contains("fontcolor=0xFFFFFF"));
        assert!(args.contains("x='(w-text_w)/2'"));
        assert!(args.contains("y='(h*0.85)-text_h'"));
        assert!(args.contains("borderw=5")); // 0.005 * 1080 = 5.4 => 5
        assert!(args.contains("bordercolor=0x000000"));
    }

    #[test]
    fn left_and_right_alignment_use_expected_edges() {
        let mut style = CaptionStyle::get_default_16_9_preset();
        style.alignment = "left".into();
        assert!(style
            .to_ffmpeg_drawtext_args(1920, 1080)
            .contains("x='(w*0.5)'"));
        style.alignment = "right".into();
        assert!(style
            .to_ffmpeg_drawtext_args(1920, 1080)
            .contains("x='(w*0.5)-text_w'"));
    }

    #[test]
    fn unsafe_font_reference_falls_back_to_system_font() {
        let mut style = CaptionStyle::get_default_16_9_preset();
        style.font_family = "C:\\Windows\\Fonts\\evil.ttf'".into();
        assert!(style
            .to_ffmpeg_drawtext_args(1920, 1080)
            .contains("fontfile='Arial'"));
    }
}
