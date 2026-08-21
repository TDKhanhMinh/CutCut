use crate::models::vad::{NonSpeechInterval, SpeechInterval, VadAnalysisResult, VadConfig};
use anyhow::{bail, Context, Result};
use regex::Regex;
use std::fs;
use std::path::Path;

/// Provider-neutral local VAD implementation.
///
/// The CPU energy provider is dependency-free so an optional ONNX/Silero
/// runtime cannot block a packaged app. It emits canonical intervals and keeps
/// uncertain regions disabled in the downstream suggestion policy.
pub struct VadDetectionService;

impl VadDetectionService {
    pub fn analyze_wav(
        path: &Path,
        duration_ms: u64,
        config: VadConfig,
    ) -> Result<VadAnalysisResult> {
        let (sample_rate, channels, samples) = read_pcm16_wav(path)?;
        let frame_samples = ((sample_rate as u64 * 20) / 1_000).max(1) as usize;
        let frame_ms = ((frame_samples as u64 * 1_000) / sample_rate as u64).max(1);
        let threshold_db = -55.0 + (1.0 - config.threshold.clamp(0.0, 1.0) as f64) * 25.0;
        let mut active_frames = Vec::new();

        for frame in samples.chunks(frame_samples * channels as usize) {
            if frame.is_empty() {
                continue;
            }
            let mut energy = 0.0_f64;
            let mut zero_crossings = 0_u64;
            let mut previous = 0_i16;
            for (index, sample) in frame.iter().enumerate() {
                let value = *sample as f64 / i16::MAX as f64;
                energy += value * value;
                if index > 0 && ((*sample < 0) != (previous < 0)) {
                    zero_crossings += 1;
                }
                previous = *sample;
            }
            let rms = (energy / frame.len() as f64).sqrt().max(1e-9);
            let db = 20.0 * rms.log10();
            let zcr = zero_crossings as f64 / frame.len() as f64;
            active_frames.push(db >= threshold_db && (0.005..=0.45).contains(&zcr));
        }

        smooth_short_silence_gaps(
            &mut active_frames,
            (config.min_silence_duration_ms as u64).div_ceil(frame_ms) as usize,
        );

        let mut speech_intervals = Vec::new();
        let mut run_start: Option<usize> = None;
        for (index, active) in active_frames.iter().copied().enumerate() {
            if active && run_start.is_none() {
                run_start = Some(index);
            }
            if (!active || index + 1 == active_frames.len()) && run_start.is_some() {
                let start_frame = run_start.take().unwrap_or(index);
                let end_frame = if active && index + 1 == active_frames.len() {
                    index + 1
                } else {
                    index
                };
                let start_ms = start_frame as u64 * frame_ms;
                let end_ms = (end_frame as u64 * frame_ms).min(duration_ms);
                if end_ms.saturating_sub(start_ms) >= config.min_speech_duration_ms as u64 {
                    let padded_start = start_ms.saturating_sub(config.speech_pad_ms as u64);
                    let padded_end = end_ms
                        .saturating_add(config.speech_pad_ms as u64)
                        .min(duration_ms);
                    if padded_start < padded_end {
                        speech_intervals.push(SpeechInterval {
                            start_ms: padded_start,
                            end_ms: padded_end,
                        });
                    }
                }
            }
        }

        let speech_intervals = normalize_speech_intervals(speech_intervals, duration_ms);
        let non_speech_intervals = Self::invert_speech_intervals(&speech_intervals, duration_ms);

        Ok(VadAnalysisResult {
            provider: "local-energy-vad".to_string(),
            version: "energy-v1".to_string(),
            speech_intervals,
            non_speech_intervals,
            config_used: config,
        })
    }

    pub fn parse_vad_output(output: &str, intervals: &mut Vec<SpeechInterval>) {
        let Ok(re) = Regex::new(r"start = ([\d.]+),\s*end = ([\d.]+)") else {
            return;
        };
        for caps in re.captures_iter(output) {
            let Ok(start_sec) = caps[1].parse::<f64>() else {
                continue;
            };
            let Ok(end_sec) = caps[2].parse::<f64>() else {
                continue;
            };
            if !start_sec.is_finite() || !end_sec.is_finite() || end_sec <= start_sec {
                continue;
            }
            intervals.push(SpeechInterval {
                start_ms: (start_sec * 1_000.0).round().max(0.0) as u64,
                end_ms: (end_sec * 1_000.0).round().max(0.0) as u64,
            });
        }
    }

    pub fn invert_speech_intervals(
        speech: &[SpeechInterval],
        duration_ms: u64,
    ) -> Vec<NonSpeechInterval> {
        let mut normalized = normalize_speech_intervals(speech.to_vec(), duration_ms);
        normalized.sort_by_key(|interval| interval.start_ms);
        let mut non_speech = Vec::new();
        let mut last_end = 0_u64;
        for interval in normalized {
            if interval.start_ms > last_end {
                non_speech.push(NonSpeechInterval {
                    start_ms: last_end,
                    end_ms: interval.start_ms,
                    reason: "non_speech".to_string(),
                });
            }
            last_end = last_end.max(interval.end_ms);
        }
        if duration_ms > last_end {
            non_speech.push(NonSpeechInterval {
                start_ms: last_end,
                end_ms: duration_ms,
                reason: "non_speech".to_string(),
            });
        }
        non_speech
    }
}

fn normalize_speech_intervals(
    mut intervals: Vec<SpeechInterval>,
    duration_ms: u64,
) -> Vec<SpeechInterval> {
    intervals
        .retain(|interval| interval.start_ms < interval.end_ms && interval.start_ms < duration_ms);
    for interval in &mut intervals {
        interval.end_ms = interval.end_ms.min(duration_ms);
    }
    intervals.sort_by_key(|interval| (interval.start_ms, interval.end_ms));
    let mut merged: Vec<SpeechInterval> = Vec::new();
    for interval in intervals {
        if let Some(last) = merged.last_mut() {
            if interval.start_ms <= last.end_ms {
                last.end_ms = last.end_ms.max(interval.end_ms);
                continue;
            }
        }
        merged.push(interval);
    }
    merged
}

fn smooth_short_silence_gaps(active_frames: &mut [bool], max_gap_frames: usize) {
    if max_gap_frames == 0 || active_frames.len() < 3 {
        return;
    }

    let mut index = 0;
    while index < active_frames.len() {
        if active_frames[index] {
            index += 1;
            continue;
        }
        let start = index;
        while index < active_frames.len() && !active_frames[index] {
            index += 1;
        }
        let end = index;
        if start > 0 && end < active_frames.len() && end - start <= max_gap_frames {
            active_frames[start..end].fill(true);
        }
    }
}

fn read_pcm16_wav(path: &Path) -> Result<(u32, u16, Vec<i16>)> {
    let bytes =
        fs::read(path).with_context(|| format!("failed to read WAV: {}", path.display()))?;
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        bail!("unsupported WAV header");
    }
    let mut cursor = 12_usize;
    let mut sample_rate = None;
    let mut channels = None;
    let mut data = None;
    while cursor + 8 <= bytes.len() {
        let id = &bytes[cursor..cursor + 4];
        let size = u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into()?) as usize;
        cursor += 8;
        let end = cursor.saturating_add(size).min(bytes.len());
        if id == b"fmt " && end.saturating_sub(cursor) >= 16 {
            let format = u16::from_le_bytes(bytes[cursor..cursor + 2].try_into()?);
            let channel_count = u16::from_le_bytes(bytes[cursor + 2..cursor + 4].try_into()?);
            let rate = u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into()?);
            let bits = u16::from_le_bytes(bytes[cursor + 14..cursor + 16].try_into()?);
            if format != 1 || bits != 16 || channel_count == 0 || rate == 0 {
                bail!("VAD requires PCM16 WAV");
            }
            sample_rate = Some(rate);
            channels = Some(channel_count);
        } else if id == b"data" {
            data = Some(bytes[cursor..end].to_vec());
        }
        cursor = end + (size % 2);
    }
    let rate = sample_rate.context("WAV fmt chunk missing")?;
    let channel_count = channels.context("WAV channel count missing")?;
    let raw = data.context("WAV data chunk missing")?;
    let samples = raw
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    Ok((rate, channel_count, samples))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_rejects_invalid_and_keeps_canonical_ranges() {
        let mut intervals = Vec::new();
        VadDetectionService::parse_vad_output(
            "start = 2.00, end = 1.00\nstart = 0.25, end = 0.75",
            &mut intervals,
        );
        assert_eq!(intervals.len(), 1);
        assert_eq!(intervals[0].start_ms, 250);
    }

    #[test]
    fn inversion_sorts_merges_and_clamps() {
        let gaps = VadDetectionService::invert_speech_intervals(
            &[
                SpeechInterval {
                    start_ms: 900,
                    end_ms: 1_100,
                },
                SpeechInterval {
                    start_ms: 100,
                    end_ms: 500,
                },
                SpeechInterval {
                    start_ms: 450,
                    end_ms: 700,
                },
            ],
            1_000,
        );
        assert_eq!(
            gaps,
            vec![
                NonSpeechInterval {
                    start_ms: 0,
                    end_ms: 100,
                    reason: "non_speech".into()
                },
                NonSpeechInterval {
                    start_ms: 700,
                    end_ms: 900,
                    reason: "non_speech".into()
                }
            ]
        );
    }

    #[test]
    fn smooths_only_short_internal_inactive_runs() {
        let mut frames = vec![true, false, false, true, false, false, false, true];
        smooth_short_silence_gaps(&mut frames, 2);
        assert_eq!(
            frames,
            vec![true, true, true, true, false, false, false, true]
        );
    }

    #[test]
    fn local_energy_provider_keeps_speech_and_marks_noise_only_as_non_speech() {
        let sample_rate = 16_000_u32;
        let mut samples = Vec::with_capacity(sample_rate as usize * 3);
        for index in 0..(sample_rate as usize * 3) {
            let second = index / sample_rate as usize;
            let sample = match second {
                0 => {
                    let phase =
                        (index as f64 * 2.0 * std::f64::consts::PI * 220.0) / sample_rate as f64;
                    (phase.sin() * 8_000.0) as i16
                }
                1 => {
                    if index % 2 == 0 {
                        8_000
                    } else {
                        -8_000
                    }
                }
                _ => 0,
            };
            samples.push(sample);
        }

        let mut wav = Vec::new();
        let data_size = (samples.len() * 2) as u32;
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_size).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16_u32.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&1_u16.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        wav.extend_from_slice(&2_u16.to_le_bytes());
        wav.extend_from_slice(&16_u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_size.to_le_bytes());
        for sample in samples {
            wav.extend_from_slice(&sample.to_le_bytes());
        }

        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), wav).unwrap();
        let result = VadDetectionService::analyze_wav(
            file.path(),
            3_000,
            VadConfig {
                min_speech_duration_ms: 200,
                min_silence_duration_ms: 100,
                speech_pad_ms: 0,
                threshold: 0.5,
            },
        )
        .unwrap();

        assert_eq!(result.provider, "local-energy-vad");
        assert!(result
            .speech_intervals
            .iter()
            .any(|interval| interval.start_ms == 0 && interval.end_ms >= 980));
        assert!(result
            .non_speech_intervals
            .iter()
            .any(|interval| interval.start_ms <= 1_020 && interval.end_ms == 3_000));
        assert!(result
            .speech_intervals
            .iter()
            .all(|interval| interval.end_ms <= 3_000 && interval.start_ms < interval.end_ms));
    }
}
