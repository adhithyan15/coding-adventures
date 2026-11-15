//! Note rendering pipeline: continuous synthesis to discrete float & PCM samples (MUS05 §10).

use crate::envelope::{validate_duration, validate_unit};
use crate::error::InstrumentError;
use crate::note::{Note, TryIntoNote};
use crate::presets::{get_instrument, sine};
use crate::profile::{InstrumentProfile, DEFAULT_MAX_PARTIAL_COUNT, MAX_PARTIALS};
use crate::signal::{validate_sample_rate, InstrumentSignal, DEFAULT_SAMPLE_RATE_HZ};
use crate::variation::VariationProfile;
use oscillator::{SampleBuffer, UniformSampler};
use pcm_audio::{encode_sample_buffer, PcmAudio};

/// Maximum allowable sample count in a single rendered note buffer (2^24 samples, ~128 MiB).
pub const MAX_SAMPLE_COUNT: usize = 16_777_216;

/// Default maximum allowable rendered sample count (10 million samples).
pub const DEFAULT_MAX_SAMPLE_COUNT: usize = 10_000_000;

/// Configuration options for rendering a musical instrument note.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderOptions {
    /// Instrument profile to use. If None, `instrument_id` is consulted.
    pub instrument: Option<InstrumentProfile>,
    /// Built-in instrument preset identifier (default "sine" if neither is set).
    pub instrument_id: Option<String>,
    /// Audio sample rate in Hertz (default 44100.0).
    pub sample_rate_hz: f64,
    /// Amplitude/velocity scaling in [0.0, 1.0] (default 1.0).
    pub amplitude: f64,
    /// Start timestamp offset in seconds (default 0.0).
    pub start_time_seconds: f64,
    /// Optional variation profile overriding the instrument profile's variation.
    pub variation_profile: Option<VariationProfile>,
    /// Ceiling on total rendered samples including release tail (CWE-400 guard).
    pub max_sample_count: usize,
    /// Ceiling on number of partials permitted (CWE-400 guard).
    pub max_partial_count: usize,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            instrument: None,
            instrument_id: None,
            sample_rate_hz: DEFAULT_SAMPLE_RATE_HZ,
            amplitude: 1.0,
            start_time_seconds: 0.0,
            variation_profile: None,
            max_sample_count: DEFAULT_MAX_SAMPLE_COUNT,
            max_partial_count: DEFAULT_MAX_PARTIAL_COUNT,
        }
    }
}

impl RenderOptions {
    /// Construct a new RenderOptions with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an explicit instrument profile.
    pub fn instrument(mut self, profile: InstrumentProfile) -> Self {
        self.instrument = Some(profile);
        self
    }

    /// Set a built-in instrument preset by id.
    pub fn instrument_id(mut self, id: &str) -> Self {
        self.instrument_id = Some(id.to_string());
        self
    }

    /// Set audio sampling rate in Hertz.
    pub fn sample_rate_hz(mut self, rate: f64) -> Self {
        self.sample_rate_hz = rate;
        self
    }

    /// Set velocity/amplitude in [0.0, 1.0].
    pub fn amplitude(mut self, amp: f64) -> Self {
        self.amplitude = amp;
        self
    }

    /// Set start timestamp offset in seconds.
    pub fn start_time_seconds(mut self, start: f64) -> Self {
        self.start_time_seconds = start;
        self
    }

    /// Set an overriding variation profile.
    pub fn variation_profile(mut self, var: VariationProfile) -> Self {
        self.variation_profile = Some(var);
        self
    }

    /// Set ceiling on total allowable samples.
    pub fn max_sample_count(mut self, count: usize) -> Self {
        self.max_sample_count = count;
        self
    }

    /// Set ceiling on allowable partial count.
    pub fn max_partial_count(mut self, count: usize) -> Self {
        self.max_partial_count = count;
        self
    }
}

/// Inspectable result of rendering one note with an instrument profile.
#[derive(Debug, Clone, PartialEq)]
pub struct InstrumentNoteRender {
    /// Parsed musical note.
    pub note: Note,
    /// Fundamental frequency in Hertz.
    pub fundamental_hz: f64,
    /// Resolved instrument timbre profile.
    pub instrument: InstrumentProfile,
    /// Signal generator.
    pub signal: InstrumentSignal,
    /// Discrete floating-point samples on the uniform time grid.
    pub floating_samples: SampleBuffer,
    /// Quantized 16-bit linear PCM audio buffer.
    pub pcm_buffer: PcmAudio,
}

/// Render a single musical note into floating-point samples and 16-bit PCM.
pub fn render_instrument_note(
    note: impl TryIntoNote,
    duration_seconds: f64,
    options: &RenderOptions,
) -> Result<InstrumentNoteRender, InstrumentError> {
    let parsed_note = note.try_into_note()?;
    let note_duration = validate_duration("duration_seconds", duration_seconds)?;
    let sample_rate = validate_sample_rate("sample_rate_hz", options.sample_rate_hz)?;
    let amplitude = validate_unit("amplitude", options.amplitude)?;

    if !options.start_time_seconds.is_finite() {
        return Err(InstrumentError::NonFiniteNumber {
            name: "start_time_seconds",
            value: options.start_time_seconds,
        });
    }

    let profile = if let Some(ref prof) = options.instrument {
        prof.clone()
    } else if let Some(ref id) = options.instrument_id {
        get_instrument(id)?
    } else {
        sine()
    };

    if options.max_partial_count == 0 {
        return Err(InstrumentError::InvalidParameter {
            name: "max_partial_count",
            reason: "must be > 0".to_string(),
        });
    }
    if options.max_partial_count > MAX_PARTIALS {
        return Err(InstrumentError::BudgetExceeded {
            reason: format!(
                "max_partial_count {} exceeds MAX_PARTIALS={MAX_PARTIALS}",
                options.max_partial_count
            ),
        });
    }
    if profile.harmonic_profile.len() > options.max_partial_count {
        return Err(InstrumentError::BudgetExceeded {
            reason: format!(
                "instrument has {} partials, above max_partial_count={}",
                profile.harmonic_profile.len(),
                options.max_partial_count
            ),
        });
    }

    if options.max_sample_count == 0 {
        return Err(InstrumentError::InvalidParameter {
            name: "max_sample_count",
            reason: "must be > 0".to_string(),
        });
    }
    if options.max_sample_count > MAX_SAMPLE_COUNT {
        return Err(InstrumentError::BudgetExceeded {
            reason: format!(
                "max_sample_count {} exceeds MAX_SAMPLE_COUNT={MAX_SAMPLE_COUNT}",
                options.max_sample_count
            ),
        });
    }

    let fundamental_hz = parsed_note.frequency();
    if !fundamental_hz.is_finite() || fundamental_hz <= 0.0 {
        return Err(InstrumentError::InvalidParameter {
            name: "fundamental_hz",
            reason: format!("must be positive finite, got {fundamental_hz}"),
        });
    }
    let nyquist_hz = sample_rate / 2.0;
    if fundamental_hz >= nyquist_hz {
        return Err(InstrumentError::InvalidParameter {
            name: "fundamental_hz",
            reason: format!("fundamental_hz {fundamental_hz} must be below Nyquist {nyquist_hz}"),
        });
    }

    let rendered_duration = profile
        .envelope_profile
        .rendered_duration_seconds(note_duration)?;

    let prod = rendered_duration * sample_rate;
    if !prod.is_finite() {
        return Err(InstrumentError::NonFiniteNumber {
            name: "sample_count",
            value: prod,
        });
    }
    let expected_sample_count = prod.round() as usize;
    if expected_sample_count > options.max_sample_count {
        return Err(InstrumentError::BudgetExceeded {
            reason: format!(
                "render would create {expected_sample_count} samples, above max_sample_count={}",
                options.max_sample_count
            ),
        });
    }

    let signal = InstrumentSignal::new(
        fundamental_hz,
        profile.clone(),
        note_duration,
        amplitude,
        sample_rate,
        options.variation_profile.clone(),
    )?;

    let sampler = UniformSampler::new(sample_rate)?;
    let floating_samples = sampler.sample_with_start_time(
        &signal,
        rendered_duration,
        options.start_time_seconds,
    )?;

    let pcm_buffer = encode_sample_buffer(&floating_samples)?;

    Ok(InstrumentNoteRender {
        note: parsed_note,
        fundamental_hz,
        instrument: profile,
        signal,
        floating_samples,
        pcm_buffer,
    })
}

/// Render a single musical note directly to a 16-bit PCM audio buffer.
pub fn render_instrument_note_pcm(
    note: impl TryIntoNote,
    duration_seconds: f64,
    options: &RenderOptions,
) -> Result<PcmAudio, InstrumentError> {
    let rendered = render_instrument_note(note, duration_seconds, options)?;
    Ok(rendered.pcm_buffer)
}
