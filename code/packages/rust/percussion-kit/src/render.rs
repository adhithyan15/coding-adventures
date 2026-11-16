//! Single hit and sequence rendering pipelines with hi-hat choke dynamics (MUS07 §7, §9).

use oscillator::{
    sample_count_for_duration, SampleBuffer, UniformSampler,
};
use pcm_audio::{encode_sample_buffer, PcmAudio};

use crate::choke::{apply_hihat_choke, is_choked_by};
use crate::error::{
    safe_round_sample, validate_duration, validate_non_negative, validate_sample_rate,
    validate_unit_float, PercussionError, CHOKE_RELEASE_RAMP_SECONDS, DEFAULT_MAX_MODE_COUNT,
    DEFAULT_MAX_SAMPLE_COUNT, DEFAULT_SAMPLE_RATE_HZ, MAX_HITS, MAX_PERCUSSION_MODES,
    MAX_SAMPLE_COUNT,
};
use crate::kit::{get_percussion_voice, standard_kit, DrumKitProfile};
use crate::profile::PercussionVoiceProfile;
use crate::signal::PercussionSignal;

/// Configuration options for rendering a single percussion hit.
#[derive(Debug, Clone)]
pub struct HitRenderOptions<'a> {
    pub kit: Option<&'a DrumKitProfile>,
    pub sample_rate_hz: f64,
    pub amplitude: f64,
    pub max_sample_count: usize,
    pub max_mode_count: usize,
}

impl<'a> Default for HitRenderOptions<'a> {
    fn default() -> Self {
        Self {
            kit: None,
            sample_rate_hz: DEFAULT_SAMPLE_RATE_HZ,
            amplitude: 1.0,
            max_sample_count: DEFAULT_MAX_SAMPLE_COUNT,
            max_mode_count: DEFAULT_MAX_MODE_COUNT,
        }
    }
}

impl<'a> HitRenderOptions<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn kit(mut self, kit: &'a DrumKitProfile) -> Self {
        self.kit = Some(kit);
        self
    }

    pub fn sample_rate_hz(mut self, sample_rate_hz: f64) -> Self {
        self.sample_rate_hz = sample_rate_hz;
        self
    }

    pub fn amplitude(mut self, amplitude: f64) -> Self {
        self.amplitude = amplitude;
        self
    }

    pub fn max_sample_count(mut self, max_sample_count: usize) -> Self {
        self.max_sample_count = max_sample_count;
        self
    }

    pub fn max_mode_count(mut self, max_mode_count: usize) -> Self {
        self.max_mode_count = max_mode_count;
        self
    }
}

/// Inspectable render result for one percussion hit.
#[derive(Debug, Clone, PartialEq)]
pub struct PercussionHitRender {
    pub hit_id: String,
    pub voice: PercussionVoiceProfile,
    pub signal: PercussionSignal,
    pub floating_samples: SampleBuffer,
    pub pcm_buffer: PcmAudio,
}

/// A scheduled unpitched percussion hit event in a sequence (MUS07 §5.1).
#[derive(Debug, Clone, PartialEq)]
pub struct PercussionHitEvent {
    pub hit_id: String,
    pub start_seconds: f64,
    pub duration_seconds: f64,
    pub velocity: f64,
    pub track_id: String,
    pub variation: Option<String>,
}

impl PercussionHitEvent {
    /// Construct a new scheduled percussion hit event with strict validation.
    pub fn new(
        hit_id: impl Into<String>,
        start_seconds: f64,
        duration_seconds: f64,
        velocity: f64,
        track_id: impl Into<String>,
        variation: Option<String>,
    ) -> Result<Self, PercussionError> {
        let hid = hit_id.into();
        if hid.trim().is_empty() {
            return Err(PercussionError::EmptyIdentifier { entity: "hit_id" });
        }
        let st = validate_non_negative("start_seconds", start_seconds)?;
        let dur = validate_duration("duration_seconds", duration_seconds)?;
        let vel = validate_unit_float("velocity", velocity)?;
        let tid = track_id.into();
        if tid.trim().is_empty() {
            return Err(PercussionError::EmptyIdentifier { entity: "track_id" });
        }

        Ok(Self {
            hit_id: hid,
            start_seconds: st,
            duration_seconds: dur,
            velocity: vel,
            track_id: tid,
            variation,
        })
    }

    /// Construct a scheduled event with default 50 ms duration, unit velocity, and "drums" track.
    pub fn hit(hit_id: impl Into<String>, start_seconds: f64) -> Result<Self, PercussionError> {
        Self::new(hit_id, start_seconds, 0.05, 1.0, "drums", None)
    }

    /// Construct a scheduled event with specific duration and default unit velocity.
    pub fn with_duration(
        hit_id: impl Into<String>,
        start_seconds: f64,
        duration_seconds: f64,
    ) -> Result<Self, PercussionError> {
        Self::new(hit_id, start_seconds, duration_seconds, 1.0, "drums", None)
    }

    /// Alias for `start_seconds` matching score/timeline convention.
    pub fn start_time_seconds(&self) -> f64 {
        self.start_seconds
    }
}

/// Configuration options for rendering a sequence of percussion hits.
#[derive(Debug, Clone)]
pub struct SequenceRenderOptions<'a> {
    pub kit: Option<&'a DrumKitProfile>,
    pub sample_rate_hz: f64,
    pub choke_ramp_seconds: f64,
    pub max_sample_count: usize,
    pub max_sequence_events: usize,
    pub max_mode_count: usize,
}

impl<'a> Default for SequenceRenderOptions<'a> {
    fn default() -> Self {
        Self {
            kit: None,
            sample_rate_hz: DEFAULT_SAMPLE_RATE_HZ,
            choke_ramp_seconds: CHOKE_RELEASE_RAMP_SECONDS,
            max_sample_count: MAX_SAMPLE_COUNT,
            max_sequence_events: MAX_HITS,
            max_mode_count: MAX_PERCUSSION_MODES,
        }
    }
}

impl<'a> SequenceRenderOptions<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn kit(mut self, kit: &'a DrumKitProfile) -> Self {
        self.kit = Some(kit);
        self
    }

    pub fn sample_rate_hz(mut self, sample_rate_hz: f64) -> Self {
        self.sample_rate_hz = sample_rate_hz;
        self
    }

    pub fn choke_ramp_seconds(mut self, choke_ramp_seconds: f64) -> Self {
        self.choke_ramp_seconds = choke_ramp_seconds;
        self
    }

    pub fn max_sample_count(mut self, max_sample_count: usize) -> Self {
        self.max_sample_count = max_sample_count;
        self
    }

    pub fn max_sequence_events(mut self, max_sequence_events: usize) -> Self {
        self.max_sequence_events = max_sequence_events;
        self
    }

    pub fn max_mode_count(mut self, max_mode_count: usize) -> Self {
        self.max_mode_count = max_mode_count;
        self
    }
}

/// Inspectable render result for a sequence of percussion hits (MUS07 §9).
#[derive(Debug, Clone, PartialEq)]
pub struct PercussionSequenceRender {
    pub events: Vec<PercussionHitEvent>,
    pub floating_samples: SampleBuffer,
    pub pcm_buffer: PcmAudio,
}

impl PercussionSequenceRender {
    /// Return the sampling rate in Hertz.
    pub fn sample_rate_hz(&self) -> f64 {
        self.floating_samples.sample_rate_hz()
    }

    /// Return the total duration of the rendered sequence in seconds.
    pub fn duration_seconds(&self) -> f64 {
        self.pcm_buffer.duration_seconds()
    }

    /// Return the number of audio samples.
    pub fn sample_count(&self) -> usize {
        self.floating_samples.sample_count()
    }
}

// ============================================================================
// Single Hit Rendering Pipeline
// ============================================================================

/// Render one unpitched percussion hit through a deterministic kit voice.
pub fn render_percussion_hit(
    hit_id: &str,
    duration_seconds: f64,
    options: &HitRenderOptions,
) -> Result<PercussionHitRender, PercussionError> {
    let rate = validate_sample_rate("sample_rate_hz", options.sample_rate_hz)?;
    let dur = validate_duration("duration_seconds", duration_seconds)?;
    let amp = validate_unit_float("amplitude", options.amplitude)?;

    if options.max_mode_count > MAX_PERCUSSION_MODES {
        return Err(PercussionError::ModeCountTooLarge {
            count: options.max_mode_count,
            max: MAX_PERCUSSION_MODES,
        });
    }

    if options.max_sample_count > MAX_SAMPLE_COUNT {
        return Err(PercussionError::SampleCountTooLarge {
            count: options.max_sample_count,
            max: MAX_SAMPLE_COUNT,
        });
    }

    let default_kit;
    let active_kit = match options.kit {
        Some(k) => k,
        None => {
            default_kit = standard_kit();
            &default_kit
        }
    };

    let voice = get_percussion_voice(hit_id, Some(active_kit))?;
    if voice.modes.len() > options.max_mode_count {
        return Err(PercussionError::ModeCountTooLarge {
            count: voice.modes.len(),
            max: options.max_mode_count,
        });
    }

    let rendered_dur = voice.envelope.rendered_duration_seconds(dur)?;
    let needed_count = sample_count_for_duration(rendered_dur, rate)?;
    if needed_count > options.max_sample_count {
        return Err(PercussionError::SampleCountTooLarge {
            count: needed_count,
            max: options.max_sample_count,
        });
    }

    let signal = PercussionSignal::new(voice.clone(), dur, amp, rate)?;
    let sampler = UniformSampler::new(rate)?;
    let samples = sampler.sample(&signal, rendered_dur)?;
    let pcm_buffer = encode_sample_buffer(&samples)?;

    Ok(PercussionHitRender {
        hit_id: hit_id.to_string(),
        voice,
        signal,
        floating_samples: samples,
        pcm_buffer,
    })
}

/// Convenience function rendering a percussion hit directly to a PCM audio buffer.
pub fn render_percussion_hit_pcm(
    hit_id: &str,
    duration_seconds: f64,
    options: &HitRenderOptions,
) -> Result<PcmAudio, PercussionError> {
    let rendered = render_percussion_hit(hit_id, duration_seconds, options)?;
    Ok(rendered.pcm_buffer)
}

// ============================================================================
// Multi-Hit Sequence Rendering Pipeline (MUS07 §7 & §9)
// ============================================================================

/// Render a sequence of scheduled percussion hits with choke groups (MUS07 §7 & §9).
///
/// Strictly iterative, streaming synthesis function enforcing all CWE-400 bounds.
/// Guarantees peak working memory is strictly O(max_end_sample) rather than
/// O(MAX_HITS * MAX_SAMPLE_COUNT).
pub fn render_percussion_sequence(
    events: &[PercussionHitEvent],
    options: &SequenceRenderOptions,
) -> Result<PercussionSequenceRender, PercussionError> {
    // 1. Unconditional validation of sample rate and choke ramp parameters
    let rate = validate_sample_rate("sample_rate_hz", options.sample_rate_hz)?;
    let ramp_dur = validate_duration("choke_ramp_seconds", options.choke_ramp_seconds)?;

    if options.max_sequence_events > MAX_HITS {
        return Err(PercussionError::SequenceEventsTooLarge {
            count: options.max_sequence_events,
            max: MAX_HITS,
        });
    }
    if options.max_sample_count > MAX_SAMPLE_COUNT {
        return Err(PercussionError::SampleCountTooLarge {
            count: options.max_sample_count,
            max: MAX_SAMPLE_COUNT,
        });
    }
    if options.max_mode_count > MAX_PERCUSSION_MODES {
        return Err(PercussionError::ModeCountTooLarge {
            count: options.max_mode_count,
            max: MAX_PERCUSSION_MODES,
        });
    }

    if events.len() > options.max_sequence_events {
        return Err(PercussionError::SequenceEventsTooLarge {
            count: events.len(),
            max: options.max_sequence_events,
        });
    }

    if events.is_empty() {
        let empty_buf = SampleBuffer::new(Vec::new(), rate, 0.0)?;
        let empty_pcm = encode_sample_buffer(&empty_buf)?;
        return Ok(PercussionSequenceRender {
            events: Vec::new(),
            floating_samples: empty_buf,
            pcm_buffer: empty_pcm,
        });
    }

    // 2. Pre-flight budget & choke calculation (CWE-400 defense)
    let default_kit;
    let active_kit = match options.kit {
        Some(k) => k,
        None => {
            default_kit = standard_kit();
            &default_kit
        }
    };

    let num_events = events.len();
    let mut event_voices = Vec::with_capacity(num_events);
    let mut event_start_indices = Vec::with_capacity(num_events);
    let mut event_rendered_durs = Vec::with_capacity(num_events);
    let mut max_end_sample = 0usize;

    for ev in events {
        let voice = get_percussion_voice(&ev.hit_id, Some(active_kit))?;
        if voice.modes.len() > options.max_mode_count {
            return Err(PercussionError::ModeCountTooLarge {
                count: voice.modes.len(),
                max: options.max_mode_count,
            });
        }
        event_voices.push(voice.clone());

        let rendered_dur = voice.envelope.rendered_duration_seconds(ev.duration_seconds)?;
        event_rendered_durs.push(rendered_dur);

        let hit_samples = sample_count_for_duration(rendered_dur, rate)?;
        if hit_samples > options.max_sample_count {
            return Err(PercussionError::SampleCountTooLarge {
                count: hit_samples,
                max: options.max_sample_count,
            });
        }

        let start_idx = safe_round_sample(ev.start_seconds, rate)?;
        event_start_indices.push(start_idx);

        let hit_end = match start_idx.checked_add(hit_samples) {
            Some(end) => end,
            None => {
                return Err(PercussionError::SampleCountTooLarge {
                    count: usize::MAX,
                    max: options.max_sample_count,
                })
            }
        };

        if hit_end > options.max_sample_count {
            return Err(PercussionError::SampleCountTooLarge {
                count: hit_end,
                max: options.max_sample_count,
            });
        }
        if hit_end > max_end_sample {
            max_end_sample = hit_end;
        }
    }

    // Determine earliest choke time for each event from event metadata
    let mut earliest_choke_times: Vec<Option<f64>> = vec![None; num_events];
    for i in 0..num_events {
        let v_i = &event_voices[i];
        if v_i.choke_group.is_none() {
            continue;
        }
        let ev_i = &events[i];
        let mut choke_t: Option<f64> = None;
        for (j, ev_j) in events.iter().enumerate() {
            if i == j {
                continue;
            }
            if ev_j.start_seconds < ev_i.start_seconds {
                continue;
            }
            if is_choked_by(v_i, &ev_i.hit_id, &event_voices[j], &ev_j.hit_id)
                && choke_t.is_none_or(|t| ev_j.start_seconds < t)
            {
                choke_t = Some(ev_j.start_seconds);
            }
        }
        earliest_choke_times[i] = choke_t;
    }

    // 3. Allocate timeline buffer (guaranteed <= max_sample_count <= MAX_SAMPLE_COUNT)
    let mut timeline = vec![0.0f64; max_end_sample];

    // 4. Streaming Hit Synthesis: render and mix ONE hit at a time
    let sampler = UniformSampler::new(rate)?;
    for i in 0..num_events {
        let ev = &events[i];
        let voice = &event_voices[i];
        let rendered_dur = event_rendered_durs[i];
        let st = event_start_indices[i];

        let sig = PercussionSignal::new(voice.clone(), ev.duration_seconds, ev.velocity, rate)?;
        let sbuf = sampler.sample(&sig, rendered_dur)?;
        let mut hit_samples = sbuf.into_samples();

        if let Some(choke_t) = earliest_choke_times[i] {
            hit_samples = apply_hihat_choke(
                &hit_samples,
                ev.start_seconds,
                choke_t,
                rate,
                ramp_dur,
            )?;
        }

        for (offset, &sample_val) in hit_samples.iter().enumerate() {
            timeline[st + offset] += sample_val;
        }
    }

    // 5. Safely clamp mixed floating point samples to [-1.0, 1.0]
    for s in &mut timeline {
        *s = s.clamp(-1.0, 1.0);
    }

    let sample_buf = SampleBuffer::new(timeline, rate, 0.0)?;
    let pcm_buf = encode_sample_buffer(&sample_buf)?;

    Ok(PercussionSequenceRender {
        events: events.to_vec(),
        floating_samples: sample_buf,
        pcm_buffer: pcm_buf,
    })
}

/// Convenience function rendering a sequence directly to a PCM audio buffer.
pub fn render_percussion_sequence_pcm(
    events: &[PercussionHitEvent],
    options: &SequenceRenderOptions,
) -> Result<PcmAudio, PercussionError> {
    let rendered = render_percussion_sequence(events, options)?;
    Ok(rendered.pcm_buffer)
}
