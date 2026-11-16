//! Sample-to-Score transcription producing MUS04 PortableScore (MUS08, VER15).

use music_machine_core::{
    InstrumentDeclaration, MeterEvent, PortableScore, PortableScoreEvent, ScoreEventKind,
    TempoEvent, TrackDeclaration, PORTABLE_FORMAT_VERSION,
};
use pcm_audio::PcmAudio;

use crate::dsp_utils::validate_pcm_source_bounds;
use crate::features::{rms_energy, rms_energy_framed};
use crate::frequency::frequency_to_note;
use crate::onset::detect_onsets;
use crate::pitch::{hps_pitch, yin_pitch};
use crate::types::{
    validate_positive_float, validate_samples, AnalysisError, MAX_BPM, MIN_BPM,
};

/// Configuration options for score transcription and rhythmic grid quantization.
#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptionOptions {
    /// Metric tempo in beats per minute (default 120.0).
    pub bpm: f64,
    /// Pulses (ticks) per quarter note (default 480).
    pub ppq: u32,
    /// Rhythmic quantization step in beats (0.25 = 16th note).
    pub quantize_grid_beats: f64,
    /// Score title string.
    pub title: String,
    /// Instrument preset profile identifier for the track.
    pub instrument_id: String,
    /// Track identifier string.
    pub track_id: String,
}

impl Default for TranscriptionOptions {
    fn default() -> Self {
        Self {
            bpm: 120.0,
            ppq: 480,
            quantize_grid_beats: 0.25,
            title: "Transcribed Score".to_string(),
            instrument_id: "sine".to_string(),
            track_id: "melody".to_string(),
        }
    }
}

/// Trait abstracting audio sources convertible to normalized floating-point samples.
pub trait TranscribePcmSource {
    /// Convert to floating-point samples and validate sampling rate.
    fn to_float_samples_and_sr(
        &self,
        fallback_sr: Option<f64>,
    ) -> Result<(Vec<f64>, f64), AnalysisError>;
}

impl TranscribePcmSource for &[f64] {
    fn to_float_samples_and_sr(
        &self,
        fallback_sr: Option<f64>,
    ) -> Result<(Vec<f64>, f64), AnalysisError> {
        let sr = fallback_sr.ok_or_else(|| {
            AnalysisError::InvalidInput(
                "sample_rate_hz must be specified when samples is not a PCMBuffer".to_string(),
            )
        })?;
        let valid_sr = validate_pcm_source_bounds(self.len(), sr)?;
        validate_samples(self, "samples")?;

        let is_int_pcm = self.iter().take(100).any(|&s| s.abs() > 1.5);
        let float_samples = if is_int_pcm {
            self.iter().map(|&s| s / 32767.0).collect()
        } else {
            self.to_vec()
        };

        Ok((float_samples, valid_sr))
    }
}

impl TranscribePcmSource for Vec<f64> {
    fn to_float_samples_and_sr(
        &self,
        fallback_sr: Option<f64>,
    ) -> Result<(Vec<f64>, f64), AnalysisError> {
        self.as_slice().to_float_samples_and_sr(fallback_sr)
    }
}

impl TranscribePcmSource for &[i16] {
    fn to_float_samples_and_sr(
        &self,
        fallback_sr: Option<f64>,
    ) -> Result<(Vec<f64>, f64), AnalysisError> {
        let sr = fallback_sr.ok_or_else(|| {
            AnalysisError::InvalidInput(
                "sample_rate_hz must be specified when samples is not a PCMBuffer".to_string(),
            )
        })?;
        let valid_sr = validate_pcm_source_bounds(self.len(), sr)?;
        let float_samples = self.iter().map(|&s| (s as f64) / 32767.0).collect();
        Ok((float_samples, valid_sr))
    }
}

impl TranscribePcmSource for Vec<i16> {
    fn to_float_samples_and_sr(
        &self,
        fallback_sr: Option<f64>,
    ) -> Result<(Vec<f64>, f64), AnalysisError> {
        self.as_slice().to_float_samples_and_sr(fallback_sr)
    }
}

impl TranscribePcmSource for &PcmAudio {
    fn to_float_samples_and_sr(
        &self,
        _fallback_sr: Option<f64>,
    ) -> Result<(Vec<f64>, f64), AnalysisError> {
        let sr = self.format().sample_rate_hz();
        let raw = self.samples();
        let valid_sr = validate_pcm_source_bounds(raw.len(), sr)?;
        let float_samples = raw.iter().map(|&s| (s as f64) / 32767.0).collect();
        Ok((float_samples, valid_sr))
    }
}

impl TranscribePcmSource for PcmAudio {
    fn to_float_samples_and_sr(
        &self,
        fallback_sr: Option<f64>,
    ) -> Result<(Vec<f64>, f64), AnalysisError> {
        (&self).to_float_samples_and_sr(fallback_sr)
    }
}

/// Transcribe raw audio samples or `PcmAudio` buffer into a MUS04 `PortableScore`.
///
/// Segments audio into discrete notes via multi-resolution onset detection,
/// estimates pitch using YIN and HPS, quantizes note timings to a musical
/// beat grid at the given tempo, and returns a verified `PortableScore`.
pub fn transcribe_pcm_to_portable_score(
    source: impl TranscribePcmSource,
    sample_rate_hz: Option<f64>,
    options: Option<&TranscriptionOptions>,
) -> Result<PortableScore, AnalysisError> {
    let (float_samples, valid_sr) = source.to_float_samples_and_sr(sample_rate_hz)?;
    validate_samples(&float_samples, "samples")?;

    let default_opts = TranscriptionOptions::default();
    let opts = options.unwrap_or(&default_opts);

    if opts.ppq == 0 || opts.ppq > 9600 {
        return Err(AnalysisError::InvalidInput(format!(
            "ppq {} out of valid range [1, 9600]",
            opts.ppq
        )));
    }

    let valid_quantize = validate_positive_float(
        opts.quantize_grid_beats,
        "quantize_grid_beats",
        Some(16.0),
    )?;

    let valid_bpm = validate_positive_float(opts.bpm, "bpm", Some(MAX_BPM))?;
    if valid_bpm < MIN_BPM {
        return Err(AnalysisError::InvalidInput(format!(
            "bpm {valid_bpm} below minimum {MIN_BPM}"
        )));
    }

    let sig = float_samples;
    let n = sig.len();
    let overall_rms = rms_energy(&sig)?;

    let mut events = Vec::new();

    if overall_rms >= 1e-4 {
        let mut onsets = detect_onsets(&sig, valid_sr, 512, 1024, 1.5, 0.08)?;
        if onsets.is_empty() {
            onsets = vec![0.0];
        }

        let total_dur_sec = (n as f64) / valid_sr;
        let tps = (valid_bpm * (opts.ppq as f64)) / 60.0;
        let grid_step_ticks = 1.max((valid_quantize * (opts.ppq as f64)).round() as u64);

        for (idx, &t_start) in onsets.iter().enumerate() {
            let t_next = if idx + 1 < onsets.len() {
                onsets[idx + 1]
            } else {
                total_dur_sec
            };

            let s_start = (t_start * valid_sr) as usize;
            let s_end = n.min((t_next * valid_sr) as usize);

            if s_end.saturating_sub(s_start) < 128 {
                continue;
            }

            let slice_samples = &sig[s_start..s_end];
            let slice_dur = (slice_samples.len() as f64) / valid_sr;

            // Estimate active duration
            let hop_len = 64.max(slice_samples.len() / 16);
            let frame_len = 256.min(slice_samples.len());
            let rms_frames = if slice_samples.len() >= frame_len {
                rms_energy_framed(slice_samples, frame_len, hop_len)?
            } else {
                vec![overall_rms]
            };

            let dur_sec = if !rms_frames.is_empty() {
                let mut max_frame_rms = 0.0f64;
                for &r in &rms_frames {
                    if r > max_frame_rms {
                        max_frame_rms = r;
                    }
                }
                let active_count = rms_frames
                    .iter()
                    .filter(|&&r| r >= 0.2 * max_frame_rms)
                    .count();
                0.05f64.max(slice_dur.min(((active_count * hop_len) as f64) / valid_sr))
            } else {
                slice_dur
            };

            // Extract pitch from stable portion
            let skip_transient = (slice_samples.len() / 4).min((0.015 * valid_sr) as usize);
            let stable = &slice_samples[skip_transient..];
            let mut pitch = yin_pitch(stable, valid_sr, 40.0, 2000.0, 0.15)?;
            if pitch <= 0.0 {
                pitch = hps_pitch(stable, valid_sr, 5, 40.0, 2000.0, None)?;
            }

            if pitch > 0.0 {
                let note_match = frequency_to_note(pitch, 440.0)?;
                let raw_start_tick = t_start * tps;
                let start_tick_units = (raw_start_tick / (grid_step_ticks as f64))
                    .round()
                    .max(0.0) as u64;
                let start_tick = start_tick_units.saturating_mul(grid_step_ticks);

                let raw_dur_ticks = dur_sec * tps;
                let dur_tick_units = (raw_dur_ticks / (grid_step_ticks as f64))
                    .round()
                    .max(1.0) as u64;
                let dur_ticks =
                    grid_step_ticks.max(dur_tick_units.saturating_mul(grid_step_ticks));

                let vel = (rms_energy(stable)? * 2.5).clamp(0.1, 1.0);
                let rounded_vel = (vel * 100.0).round() / 100.0;

                let event = PortableScoreEvent::new(
                    &opts.track_id,
                    start_tick,
                    dur_ticks,
                    ScoreEventKind::Note,
                    vec![note_match.note],
                    rounded_vel,
                    events.len(),
                    None,
                )
                .map_err(|e| AnalysisError::InvalidInput(format!("Event creation error: {e}")))?;

                events.push(event);
            }
        }
    }

    let kind_str = if opts.instrument_id == "sine" || opts.instrument_id == "silence" {
        &opts.instrument_id
    } else {
        "preset"
    };

    let inst_decl = InstrumentDeclaration::new(
        &opts.instrument_id,
        &opts.instrument_id,
        1.0,
        kind_str,
        None,
        None,
        None,
    )
    .map_err(|e| AnalysisError::InvalidInput(format!("InstrumentDeclaration error: {e}")))?;

    let track_decl = TrackDeclaration::new(&opts.track_id, &opts.instrument_id)
        .map_err(|e| AnalysisError::InvalidInput(format!("TrackDeclaration error: {e}")))?;

    let score = PortableScore::new(
        PORTABLE_FORMAT_VERSION,
        &opts.title,
        opts.ppq,
        valid_sr as u32,
        vec![TempoEvent {
            start_tick: 0,
            bpm: valid_bpm,
        }],
        vec![MeterEvent {
            start_tick: 0,
            meter: "4/4".to_string(),
        }],
        vec![inst_decl],
        vec![track_decl],
        events,
    )
    .map_err(|e| AnalysisError::InvalidInput(format!("PortableScore error: {e}")))?;

    Ok(score)
}

/// Convenience wrapper for transcribing a [`PcmAudio`] buffer directly.
pub fn transcribe_pcm_buffer(
    pcm: &PcmAudio,
    options: Option<&TranscriptionOptions>,
) -> Result<PortableScore, AnalysisError> {
    transcribe_pcm_to_portable_score(pcm, None, options)
}

/// Convenience wrapper for transcribing a slice of `f64` samples.
pub fn transcribe_samples(
    samples: &[f64],
    sample_rate_hz: f64,
    options: Option<&TranscriptionOptions>,
) -> Result<PortableScore, AnalysisError> {
    transcribe_pcm_to_portable_score(samples, Some(sample_rate_hz), options)
}
