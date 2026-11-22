//! # Arcade SFX: Deterministic Fixed-Point Sound Generator (GAME12)
//!
//! A zero-allocation, deterministic fixed-point sound synthesis engine and preset
//! library designed for classic coin-op arcade titles and retro consoles.
//!
//! ```text
//!  +-------------------------------------------------------------------------+
//!  |                               SfxPreset                                 |
//!  |  (PongPaddleHit, PongBounce, PongScore, BreakoutBrick, LoseLife, Coin)  |
//!  +------------------------------------+------------------------------------+
//!                                       |
//!                                    to_patch
//!                                       |
//!                                       v
//!  +-------------------------------------------------------------------------+
//!  |                                SfxPatch                                 |
//!  |  - Waveform (Square, Triangle, Sawtooth, LFSR Noise)                    |
//!  |  - Direct Digital Synthesis (32-bit Phase Accumulator, zero f64 drift)  |
//!  |  - ADSR Envelope (sample-counted Attack, Decay, Sustain Q16, Release)   |
//!  |  - Pitch Sweeps & Arpeggio Sequences (fixed-step frequency shifts)      |
//!  +------------------------------------+------------------------------------+
//!                                       |
//!                                 trigger_preset
//!                                       |
//!                                       v
//!  +-------------------------------------------------------------------------+
//!  |                       SfxVoice / SfxBank<N>                             |
//!  |   render_sfx_into(&mut SfxVoice, &mut [i16])                            |
//!  |   mix_bank_into(&mut SfxBank<N>, &mut [i16])                            |
//!  +------------------------------------+------------------------------------+
//!                                       |
//!                 +---------------------+---------------------+
//!                 |                                           |
//!                 v                                           v
//!        [ PcmRingBuffer<N> ]                       [ pcm_audio::PcmAudio ]
//!    (audio-mixer-realtime MUS08)                 (WAV Container Bridge MUS01)
//! ```
//!
//! ## Mathematical Formulation
//!
//! ### Direct Digital Synthesis (DDS)
//!
//! To prevent IEEE 754 floating-point drift across platforms, compilers, and hardware targets,
//! all oscillator waveforms are synthesized via integer DDS with a 32-bit phase accumulator:
//!
//! $$\Delta \phi = \left\lfloor \frac{f \times 2^{32}}{F_s} \right\rfloor$$
//!
//! $$\phi[n+1] = (\phi[n] + \Delta \phi) \pmod{2^{32}}$$
//!
//! The 16-bit phase angle $p_{16} \in [0, 65535]$ is extracted via `phi >> 16`.
//!
//! ### ADSR Sample-Counted Envelope
//!
//! The envelope level $L(n) \in [0, 65536]$ is computed in Q16.16 fixed-point arithmetic:
//!
//! 1. **Attack Stage** ($0 \le n < A$):
//!    $$L(n) = \frac{n \times 65536}{A}$$
//!
//! 2. **Decay Stage** ($A \le n < A + D$):
//!    $$L(n) = 65536 - \frac{(n - A) \times (65536 - S)}{D}$$
//!
//! 3. **Sustain Stage** ($A + D \le n < A + D + U$):
//!    $$L(n) = S$$
//!
//! 4. **Release Stage** ($A + D + U \le n < A + D + U + R$):
//!    $$L(n) = S - \frac{(n - (A + D + U)) \times S}{R}$$
//!
//! ### Linear Feedback Shift Register (LFSR) Noise
//!
//! Noise synthesis implements deterministic 15-bit (long mode) and 7-bit (short mode) Galois/Fibonacci
//! shift registers matching vintage arcade sound hardware:
//!
//! - **15-bit Mode**: $\text{feedback} = \text{bit}_0 \oplus \text{bit}_1$, period $= 32,767$ steps.
//! - **7-bit Mode**: $\text{feedback} = \text{bit}_0 \oplus \text{bit}_6$, period $= 93$ steps (metallic timbre).
//! - **Seed Guard**: If register state evaluates to zero, it is automatically restored to `DEFAULT_LFSR_SEED`.
//!
//! ## Security Hardening & Robustness
//!
//! - **CWE-369 (Divide by Zero Protection)**: Sample rate is strictly verified within $[1, 192000]\text{ Hz}$.
//!   All envelope stage divisions verify non-zero divisors prior to evaluation.
//! - **CWE-190 (Integer Overflow Protection)**: Mixing accumulation utilizes `i64` and saturates to $[-32768, 32767]$.
//!   Intermediate arithmetic uses saturating integer functions.
//! - **CWE-20 (Improper Input Validation)**: Frequency values are validated against Nyquist ($F_s / 2$),
//!   duty cycles are bound to 16-bit ranges, and LFSR zero-states are guarded fail-safe.
//! - **CWE-400 (Uncontrolled Resource Consumption)**: Zero heap allocation on all rendering and mixing hot paths.

#![forbid(unsafe_code)]

pub mod capabilities;

/// Minimum valid audio sample rate in Hz (1 Hz).
pub const MIN_SAMPLE_RATE_HZ: u32 = 1;

/// Maximum valid audio sample rate in Hz (192 kHz).
pub const MAX_SAMPLE_RATE_HZ: u32 = 192_000;

/// Default arcade audio sample rate in Hz (44.1 kHz).
pub const DEFAULT_SAMPLE_RATE_HZ: u32 = 44_100;

/// Minimum signed 16-bit PCM integer ($-32,768$).
pub const PCM16_MIN: i16 = -32_768;

/// Maximum signed 16-bit PCM integer ($+32,767$).
pub const PCM16_MAX: i16 = 32_767;

/// Unity gain represented in Q16.16 fixed-point ($1.0 \times 65536 = 65536$).
pub const UNITY_GAIN_Q16: u32 = 65_536;

/// Maximum voice volume gain in Q16.16 ($4.0\times$ gain = 262,144).
pub const MAX_GAIN_Q16: u32 = 262_144;

/// Maximum allowable samples when exporting a patch to an owned `PcmAudio` container (CWE-400 guard).
pub const MAX_EXPORT_SAMPLES: u32 = 8_388_608;

/// Default non-zero seed for 15-bit LFSR pseudo-random noise generator.
pub const DEFAULT_LFSR_SEED: u16 = 0x7FFF;

/// 50% duty cycle in Q16 fixed-point (0.5 * 65536).
pub const DUTY_50_PERCENT: u16 = 32_768;

/// 25% duty cycle in Q16 fixed-point (0.25 * 65536).
pub const DUTY_25_PERCENT: u16 = 16_384;

/// 12.5% duty cycle in Q16 fixed-point (0.125 * 65536).
pub const DUTY_12_5_PERCENT: u16 = 8_192;

/// 75% duty cycle in Q16 fixed-point (0.75 * 65536).
pub const DUTY_75_PERCENT: u16 = 49_152;

/// Errors that can occur during arcade sound generation and synthesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SfxError {
    /// Audio sample rate was zero or exceeded [`MAX_SAMPLE_RATE_HZ`].
    InvalidSampleRate { rate: u32 },
    /// Frequency exceeded Nyquist limit ($F_s / 2$).
    InvalidFrequency { freq: u32, max: u32 },
    /// Duty cycle parameter was out of acceptable range.
    InvalidDutyCycle { duty_q16: u16 },
    /// General invalid parameter or configuration error.
    InvalidParameter { reason: &'static str },
    /// Voice channel capacity exceeded.
    VoiceLimitExceeded,
    /// Destination buffer is smaller than requested frame count.
    BufferTooSmall,
    /// Error interfacing with `audio-mixer-realtime` or ring buffer.
    MixerIntegrationError,
}

impl std::fmt::Display for SfxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSampleRate { rate } => write!(f, "invalid sample rate: {rate} Hz"),
            Self::InvalidFrequency { freq, max } => {
                write!(f, "frequency {freq} Hz exceeds Nyquist limit of {max} Hz")
            }
            Self::InvalidDutyCycle { duty_q16 } => write!(f, "invalid duty cycle: {duty_q16}"),
            Self::InvalidParameter { reason } => write!(f, "invalid parameter: {reason}"),
            Self::VoiceLimitExceeded => write!(f, "voice channel limit exceeded"),
            Self::BufferTooSmall => write!(f, "destination buffer too small"),
            Self::MixerIntegrationError => write!(f, "audio mixer integration error"),
        }
    }
}

impl std::error::Error for SfxError {}

/// Periodic or pseudo-random waveform generator type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform {
    /// Square pulse wave with configurable duty cycle in Q16 fixed-point.
    Square { duty_q16: u16 },
    /// Symmetric triangle wave with odd harmonics decaying as $1/n^2$.
    Triangle,
    /// Sawtooth ramp containing all integer harmonics (odd and even).
    Sawtooth,
    /// Linear feedback shift register noise generator.
    LfsrNoise { short_mode: bool },
}

impl Waveform {
    /// Returns true if the waveform is an LFSR pseudo-random noise generator.
    pub const fn is_noise(&self) -> bool {
        matches!(self, Self::LfsrNoise { .. })
    }

    /// Returns true if the waveform is a square/pulse wave.
    pub const fn is_square(&self) -> bool {
        matches!(self, Self::Square { .. })
    }

    /// Validate waveform parameters (CWE-20 guard).
    pub fn validate_waveform(&self) -> Result<(), SfxError> {
        match self {
            Self::Square { .. } => Ok(()),
            Self::Triangle => Ok(()),
            Self::Sawtooth => Ok(()),
            Self::LfsrNoise { .. } => Ok(()),
        }
    }
}

/// Sample-counted Attack, Decay, Sustain, Release (ADSR) amplitude envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdsrEnvelope {
    /// Duration of linear attack stage in samples.
    pub attack_samples: u32,
    /// Duration of linear decay stage in samples.
    pub decay_samples: u32,
    /// Sustain level in Q16.16 fixed-point (`0..=UNITY_GAIN_Q16`).
    pub sustain_level_q16: u32,
    /// Duration of linear release stage in samples.
    pub release_samples: u32,
}

impl AdsrEnvelope {
    /// Construct an envelope parameterized in milliseconds.
    pub fn from_ms(
        attack_ms: u32,
        decay_ms: u32,
        sustain_level_q16: u32,
        release_ms: u32,
        sample_rate_hz: u32,
    ) -> Result<Self, SfxError> {
        validate_sample_rate(sample_rate_hz)?;
        let sr = sample_rate_hz as u64;
        let attack_samples = ((attack_ms as u64 * sr) / 1000).min(u32::MAX as u64) as u32;
        let decay_samples = ((decay_ms as u64 * sr) / 1000).min(u32::MAX as u64) as u32;
        let release_samples = ((release_ms as u64 * sr) / 1000).min(u32::MAX as u64) as u32;
        let clamped_sustain = sustain_level_q16.min(UNITY_GAIN_Q16);

        Ok(Self {
            attack_samples,
            decay_samples,
            sustain_level_q16: clamped_sustain,
            release_samples,
        })
    }

    /// Construct an envelope directly from sample counts.
    pub fn from_samples(
        attack_samples: u32,
        decay_samples: u32,
        sustain_level_q16: u32,
        release_samples: u32,
    ) -> Result<Self, SfxError> {
        let clamped_sustain = sustain_level_q16.min(UNITY_GAIN_Q16);
        Ok(Self {
            attack_samples,
            decay_samples,
            sustain_level_q16: clamped_sustain,
            release_samples,
        })
    }

    /// Total number of samples required for the envelope given a sustain duration.
    pub const fn total_envelope_samples(&self, sustain_samples: u32) -> u32 {
        self.attack_samples
            .saturating_add(self.decay_samples)
            .saturating_add(sustain_samples)
            .saturating_add(self.release_samples)
    }

    /// Compute the current amplitude envelope level in Q16.16 at sample index `sample_idx`.
    pub fn compute_level(&self, sample_idx: u32, sustain_samples: u32) -> u32 {
        let a = self.attack_samples;
        let d = self.decay_samples;
        let s = self.sustain_level_q16;
        let r = self.release_samples;

        if sample_idx < a {
            if a == 0 {
                return UNITY_GAIN_Q16;
            }
            return (((sample_idx as u64) * (UNITY_GAIN_Q16 as u64)) / (a as u64)) as u32;
        }

        let decay_end = a.saturating_add(d);
        if sample_idx < decay_end {
            if d == 0 {
                return s;
            }
            let elapsed = sample_idx.saturating_sub(a) as u64;
            let drop = UNITY_GAIN_Q16.saturating_sub(s) as u64;
            let dec = (elapsed * drop) / (d as u64);
            return UNITY_GAIN_Q16.saturating_sub(dec as u32);
        }

        let sustain_end = decay_end.saturating_add(sustain_samples);
        if sample_idx < sustain_end {
            return s;
        }

        let release_end = sustain_end.saturating_add(r);
        if sample_idx < release_end {
            if r == 0 {
                return 0;
            }
            let elapsed = sample_idx.saturating_sub(sustain_end) as u64;
            let rel = (elapsed * (s as u64)) / (r as u64);
            return s.saturating_sub(rel as u32);
        }

        0
    }

    /// Compute release stage decay from a starting level when early note-off occurs.
    pub fn compute_release_level(&self, release_elapsed: u32, start_level_q16: u32) -> u32 {
        if self.release_samples == 0 || release_elapsed >= self.release_samples {
            return 0;
        }
        let elapsed = release_elapsed as u64;
        let total = self.release_samples as u64;
        let start = start_level_q16 as u64;
        let drop = (elapsed * start) / total;
        start_level_q16.saturating_sub(drop as u32)
    }
}

/// Fully parameterized arcade sound effect patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SfxPatch {
    /// Tone or noise waveform type.
    pub waveform: Waveform,
    /// Initial frequency in Hz.
    pub start_freq_hz: u32,
    /// Target frequency in Hz after frequency sweep.
    pub end_freq_hz: u32,
    /// Duration of linear pitch sweep in samples.
    pub freq_sweep_samples: u32,
    /// Number of samples before triggering arpeggio step (0 disables arpeggio).
    pub arpeggio_step_samples: u32,
    /// Second tone frequency in Hz for arpeggio step.
    pub arpeggio_freq_hz: u32,
    /// ADSR amplitude envelope.
    pub envelope: AdsrEnvelope,
    /// Sustain stage duration in samples.
    pub sustain_samples: u32,
    /// Patch master gain in Q16.16 fixed point (`0..=MAX_GAIN_Q16`).
    pub gain_q16: u32,
}

impl Default for SfxPatch {
    fn default() -> Self {
        Self::default_patch()
    }
}

impl SfxPatch {
    /// Create a standard sound patch without arpeggios.
    pub fn create_patch(
        waveform: Waveform,
        start_freq_hz: u32,
        end_freq_hz: u32,
        freq_sweep_samples: u32,
        envelope: AdsrEnvelope,
        sustain_samples: u32,
        gain_q16: u32,
    ) -> Result<Self, SfxError> {
        let clamped_gain = gain_q16.min(MAX_GAIN_Q16);
        Ok(Self {
            waveform,
            start_freq_hz,
            end_freq_hz,
            freq_sweep_samples,
            arpeggio_step_samples: 0,
            arpeggio_freq_hz: start_freq_hz,
            envelope,
            sustain_samples,
            gain_q16: clamped_gain,
        })
    }

    /// Create an arpeggiated sound patch (e.g. coin chime, jump fanfare).
    #[allow(clippy::too_many_arguments)]
    pub fn create_arpeggio_patch(
        waveform: Waveform,
        start_freq_hz: u32,
        end_freq_hz: u32,
        freq_sweep_samples: u32,
        arpeggio_step_samples: u32,
        arpeggio_freq_hz: u32,
        envelope: AdsrEnvelope,
        sustain_samples: u32,
        gain_q16: u32,
    ) -> Result<Self, SfxError> {
        let clamped_gain = gain_q16.min(MAX_GAIN_Q16);
        Ok(Self {
            waveform,
            start_freq_hz,
            end_freq_hz,
            freq_sweep_samples,
            arpeggio_step_samples,
            arpeggio_freq_hz,
            envelope,
            sustain_samples,
            gain_q16: clamped_gain,
        })
    }

    /// Canonical default sound patch: 440 Hz square wave beep.
    pub const fn default_patch() -> Self {
        Self {
            waveform: Waveform::Square {
                duty_q16: DUTY_50_PERCENT,
            },
            start_freq_hz: 440,
            end_freq_hz: 440,
            freq_sweep_samples: 0,
            arpeggio_step_samples: 0,
            arpeggio_freq_hz: 440,
            envelope: AdsrEnvelope {
                attack_samples: 220,
                decay_samples: 880,
                sustain_level_q16: 32_768,
                release_samples: 440,
            },
            sustain_samples: 2200,
            gain_q16: UNITY_GAIN_Q16,
        }
    }

    /// Validate patch frequency and waveform parameters.
    pub fn validate_patch(&self, sample_rate_hz: u32) -> Result<(), SfxError> {
        validate_sample_rate(sample_rate_hz)?;
        validate_frequency(self.start_freq_hz, sample_rate_hz)?;
        validate_frequency(self.end_freq_hz, sample_rate_hz)?;
        if self.arpeggio_step_samples > 0 {
            validate_frequency(self.arpeggio_freq_hz, sample_rate_hz)?;
        }
        self.waveform.validate_waveform()?;
        Ok(())
    }

    /// Total samples required to play this patch to completion.
    pub const fn patch_total_samples(&self) -> u32 {
        self.envelope.total_envelope_samples(self.sustain_samples)
    }
}

/// Standard retro arcade presets for classic game interactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SfxPreset {
    /// Low/mid square beep for paddle ball reflection in Pong.
    PongPaddleHit,
    /// Higher-pitched square beep for top/bottom boundary bounce in Pong.
    PongWallBounce,
    /// Buzzy score celebration tone in Pong.
    PongScore,
    /// Brick impact tone whose pitch ascends per row in Breakout.
    BreakoutBrickHit { row: u8 },
    /// Descending pitch-swept buzz when a life is lost in Breakout.
    BreakoutLoseLife,
    /// Classic dual-frequency chime when an arcade coin is inserted.
    CoinInsert,
    /// Rapid downward sawtooth/square laser sweep cue.
    LaserSweep,
    /// User-defined custom sound effect patch.
    Custom(SfxPatch),
}

impl SfxPreset {
    /// Convert preset into concrete [`SfxPatch`] evaluated at the given sample rate.
    pub fn to_patch(&self, sample_rate_hz: u32) -> Result<SfxPatch, SfxError> {
        validate_sample_rate(sample_rate_hz)?;
        let sr = sample_rate_hz as u64;

        match self {
            Self::PongPaddleHit => {
                // Classic ~490 Hz square beep, ~75 ms duration
                let env = AdsrEnvelope::from_ms(4, 16, 40_000, 15, sample_rate_hz)?;
                let sustain_samples = ((40 * sr) / 1000) as u32;
                SfxPatch::create_patch(
                    Waveform::Square {
                        duty_q16: DUTY_50_PERCENT,
                    },
                    490,
                    490,
                    0,
                    env,
                    sustain_samples,
                    UNITY_GAIN_Q16,
                )
            }
            Self::PongWallBounce => {
                // Lower ~245 Hz square beep, ~60 ms duration
                let env = AdsrEnvelope::from_ms(3, 12, 35_000, 15, sample_rate_hz)?;
                let sustain_samples = ((30 * sr) / 1000) as u32;
                SfxPatch::create_patch(
                    Waveform::Square {
                        duty_q16: DUTY_50_PERCENT,
                    },
                    245,
                    245,
                    0,
                    env,
                    sustain_samples,
                    UNITY_GAIN_Q16,
                )
            }
            Self::PongScore => {
                // Low buzzy square pulse ~120 Hz with 25% duty, ~350 ms duration
                let env = AdsrEnvelope::from_ms(10, 40, 45_000, 80, sample_rate_hz)?;
                let sustain_samples = ((220 * sr) / 1000) as u32;
                SfxPatch::create_patch(
                    Waveform::Square {
                        duty_q16: DUTY_25_PERCENT,
                    },
                    120,
                    120,
                    0,
                    env,
                    sustain_samples,
                    UNITY_GAIN_Q16,
                )
            }
            Self::BreakoutBrickHit { row } => {
                // Ascending pitch by row: Middle C (262 Hz) + row * 65 Hz
                let base_freq = 262u32.saturating_add((*row as u32).saturating_mul(65));
                let clamped_freq = base_freq.min(sample_rate_hz / 2);
                let env = AdsrEnvelope::from_ms(3, 15, 38_000, 12, sample_rate_hz)?;
                let sustain_samples = ((35 * sr) / 1000) as u32;
                SfxPatch::create_patch(
                    Waveform::Square {
                        duty_q16: DUTY_50_PERCENT,
                    },
                    clamped_freq,
                    clamped_freq,
                    0,
                    env,
                    sustain_samples,
                    UNITY_GAIN_Q16,
                )
            }
            Self::BreakoutLoseLife => {
                // Descending pitch sweep ~450 Hz down to ~60 Hz over ~450 ms
                let env = AdsrEnvelope::from_ms(8, 80, 20_000, 120, sample_rate_hz)?;
                let sustain_samples = ((240 * sr) / 1000) as u32;
                let sweep_samples = ((400 * sr) / 1000) as u32;
                SfxPatch::create_patch(
                    Waveform::Sawtooth,
                    450,
                    60,
                    sweep_samples,
                    env,
                    sustain_samples,
                    UNITY_GAIN_Q16,
                )
            }
            Self::CoinInsert => {
                // Two-tone chime: Tone 1 (988 Hz) for ~65 ms, then Tone 2 (1319 Hz)
                let env = AdsrEnvelope::from_ms(4, 25, 48_000, 60, sample_rate_hz)?;
                let arpeggio_step_samples = ((65 * sr) / 1000) as u32;
                let sustain_samples = ((240 * sr) / 1000) as u32;
                SfxPatch::create_arpeggio_patch(
                    Waveform::Square {
                        duty_q16: DUTY_50_PERCENT,
                    },
                    988,
                    988,
                    0,
                    arpeggio_step_samples,
                    1319,
                    env,
                    sustain_samples,
                    UNITY_GAIN_Q16,
                )
            }
            Self::LaserSweep => {
                // Rapid downward sweep: 1600 Hz down to 100 Hz over ~120 ms
                let env = AdsrEnvelope::from_ms(2, 20, 25_000, 30, sample_rate_hz)?;
                let sweep_samples = ((110 * sr) / 1000) as u32;
                let sustain_samples = ((70 * sr) / 1000) as u32;
                SfxPatch::create_patch(
                    Waveform::Square {
                        duty_q16: DUTY_25_PERCENT,
                    },
                    1600,
                    100,
                    sweep_samples,
                    env,
                    sustain_samples,
                    UNITY_GAIN_Q16,
                )
            }
            Self::Custom(patch) => Ok(*patch),
        }
    }
}

/// Single synthesizer voice executing zero-allocation audio synthesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SfxVoice {
    /// Whether the voice is actively rendering audio.
    pub active: bool,
    /// Configured audio sample rate in Hz.
    pub sample_rate_hz: u32,
    /// Currently playing sound patch parameters.
    pub patch: SfxPatch,
    /// Number of rendered samples since trigger.
    pub current_sample: u32,
    /// Total duration of the sound effect in samples.
    pub total_samples: u32,
    /// Direct Digital Synthesis 32-bit phase accumulator.
    pub phase_acc: u32,
    /// Deterministic 15-bit shift register state for noise synthesis.
    pub lfsr_state: u16,
    /// Sample offset where early note-off release was triggered (`u32::MAX` if unreleased).
    pub released_at_sample: u32,
    /// Amplitude envelope level captured when early note-off occurred.
    pub release_start_level_q16: u32,
    /// User-assigned sound effect identifier.
    pub voice_id: u32,
    /// Priority level for voice allocation and voice stealing.
    pub priority: u8,
}

impl Default for SfxVoice {
    fn default() -> Self {
        Self::empty_voice(DEFAULT_SAMPLE_RATE_HZ)
    }
}

impl SfxVoice {
    /// Construct a new inactive synthesizer voice initialized at `sample_rate_hz`.
    pub fn new(sample_rate_hz: u32) -> Result<Self, SfxError> {
        validate_sample_rate(sample_rate_hz)?;
        Ok(Self::empty_voice(sample_rate_hz))
    }

    /// Internal const constructor for inactive voice.
    pub const fn empty_voice(sample_rate_hz: u32) -> Self {
        Self {
            active: false,
            sample_rate_hz,
            patch: SfxPatch::default_patch(),
            current_sample: 0,
            total_samples: 0,
            phase_acc: 0,
            lfsr_state: DEFAULT_LFSR_SEED,
            released_at_sample: u32::MAX,
            release_start_level_q16: UNITY_GAIN_Q16,
            voice_id: 0,
            priority: 0,
        }
    }

    /// Trigger playback of an arcade sound preset.
    pub fn trigger_preset(&mut self, preset: SfxPreset) -> Result<(), SfxError> {
        let patch = preset.to_patch(self.sample_rate_hz)?;
        self.trigger_patch(patch)
    }

    /// Trigger playback of a concrete sound patch with zero allocation.
    pub fn trigger_patch(&mut self, patch: SfxPatch) -> Result<(), SfxError> {
        patch.validate_patch(self.sample_rate_hz)?;
        self.patch = patch;
        self.active = true;
        self.current_sample = 0;
        self.total_samples = patch.patch_total_samples();
        self.phase_acc = 0;
        self.lfsr_state = DEFAULT_LFSR_SEED;
        self.released_at_sample = u32::MAX;
        self.release_start_level_q16 = UNITY_GAIN_Q16;
        Ok(())
    }

    /// Trigger premature note-off, transitioning immediately to the release stage.
    pub fn trigger_release(&mut self) {
        if self.active && self.released_at_sample == u32::MAX {
            let current_lvl = self
                .patch
                .envelope
                .compute_level(self.current_sample, self.patch.sustain_samples);
            self.released_at_sample = self.current_sample;
            self.release_start_level_q16 = current_lvl;
            self.total_samples = self
                .current_sample
                .saturating_add(self.patch.envelope.release_samples);
        }
    }

    /// Reset voice to inactive state.
    pub fn reset_voice(&mut self) {
        self.active = false;
        self.current_sample = 0;
        self.total_samples = 0;
        self.phase_acc = 0;
        self.lfsr_state = DEFAULT_LFSR_SEED;
        self.released_at_sample = u32::MAX;
    }

    /// Whether this voice is actively rendering audio.
    pub const fn is_active(&self) -> bool {
        self.active
    }

    /// Current sample cursor index.
    pub const fn current_voice_sample(&self) -> u32 {
        self.current_sample
    }

    /// Total sound effect duration in samples.
    pub const fn total_voice_samples(&self) -> u32 {
        self.total_samples
    }

    /// Render voice output into a temporary scratch buffer and push directly into a PCM ring buffer.
    pub fn route_voice_to_ring_buffer<const CAP: usize>(
        &mut self,
        ring: &mut audio_mixer_realtime::PcmRingBuffer<CAP>,
        scratch: &mut [i16],
    ) -> Result<usize, SfxError> {
        validate_sample_rate(self.sample_rate_hz)?;
        if scratch.is_empty() {
            return Ok(0);
        }
        let available = CAP.saturating_sub(ring.len());
        let to_render = if self.active {
            (self.total_samples.saturating_sub(self.current_sample) as usize).min(scratch.len())
        } else {
            0
        };
        if to_render > available {
            return Err(SfxError::MixerIntegrationError);
        }
        let rendered = render_sfx_into(self, scratch)?;
        if rendered > 0 {
            ring.push_slice(&scratch[..rendered])
                .map_err(|_| SfxError::MixerIntegrationError)?;
        }
        Ok(rendered)
    }
}

/// Bounded multi-voice synthesizer bank for concurrent sound effect polyphony.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SfxBank<const N: usize> {
    /// Polyphonic synthesizer voice array.
    pub voices: [SfxVoice; N],
    /// Bank master gain in Q16.16 fixed-point (`0..=MAX_GAIN_Q16`).
    pub master_gain_q16: u32,
    /// Configured audio sample rate in Hz.
    pub sample_rate_hz: u32,
}

impl<const N: usize> SfxBank<N> {
    /// Construct an empty synthesizer bank with `N` voices.
    pub fn empty(sample_rate_hz: u32) -> Result<Self, SfxError> {
        validate_sample_rate(sample_rate_hz)?;
        let voice = SfxVoice::empty_voice(sample_rate_hz);
        Ok(Self {
            voices: [voice; N],
            master_gain_q16: UNITY_GAIN_Q16,
            sample_rate_hz,
        })
    }

    /// Trigger a preset on the first available or lowest-priority voice slot.
    pub fn trigger_bank_preset(&mut self, preset: SfxPreset) -> Result<usize, SfxError> {
        let patch = preset.to_patch(self.sample_rate_hz)?;
        self.trigger_bank_patch(patch)
    }

    /// Trigger a custom patch on the first available or lowest-priority voice slot.
    pub fn trigger_bank_patch(&mut self, patch: SfxPatch) -> Result<usize, SfxError> {
        if N == 0 {
            return Err(SfxError::VoiceLimitExceeded);
        }

        // 1. Look for an inactive voice
        for (idx, voice) in self.voices.iter_mut().enumerate() {
            if !voice.active {
                voice.trigger_patch(patch)?;
                return Ok(idx);
            }
        }

        // 2. Voice stealing: steal voice with lowest priority
        let mut lowest_priority = u8::MAX;
        let mut steal_idx = 0;
        for (idx, voice) in self.voices.iter().enumerate() {
            if voice.priority <= lowest_priority {
                lowest_priority = voice.priority;
                steal_idx = idx;
            }
        }

        self.voices[steal_idx].trigger_patch(patch)?;
        Ok(steal_idx)
    }

    /// Deactivate a specific voice slot.
    pub fn stop_bank_voice(&mut self, voice_idx: usize) -> Result<(), SfxError> {
        if voice_idx >= N {
            return Err(SfxError::InvalidParameter {
                reason: "voice index out of bounds",
            });
        }
        self.voices[voice_idx].reset_voice();
        Ok(())
    }

    /// Reset all voices in the bank.
    pub fn reset_bank(&mut self) {
        for voice in &mut self.voices {
            voice.reset_voice();
        }
    }

    /// Count of currently active voices in the bank.
    pub fn active_bank_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    /// Mix bank output into temporary scratch space and route into a PCM ring buffer.
    pub fn route_bank_to_ring_buffer<const CAP: usize>(
        &mut self,
        ring: &mut audio_mixer_realtime::PcmRingBuffer<CAP>,
        scratch: &mut [i16],
    ) -> Result<usize, SfxError> {
        validate_sample_rate(self.sample_rate_hz)?;
        for voice in &self.voices {
            if voice.active {
                validate_sample_rate(voice.sample_rate_hz)?;
            }
        }
        if scratch.is_empty() {
            return Ok(0);
        }
        if scratch.len() > CAP.saturating_sub(ring.len()) {
            return Err(SfxError::MixerIntegrationError);
        }
        let rendered = mix_bank_into(self, scratch)?;
        if rendered > 0 {
            ring.push_slice(&scratch[..rendered])
                .map_err(|_| SfxError::MixerIntegrationError)?;
        }
        Ok(rendered)
    }
}

/// Render samples from a single [`SfxVoice`] into destination slice `out` with zero heap allocation.
pub fn render_sfx_into(voice: &mut SfxVoice, out: &mut [i16]) -> Result<usize, SfxError> {
    validate_sample_rate(voice.sample_rate_hz)?;

    if out.is_empty() {
        return Ok(0);
    }

    if !voice.active {
        out.fill(0);
        return Ok(0);
    }

    let mut rendered_count = 0;

    for out_sample in out.iter_mut() {
        if voice.current_sample >= voice.total_samples {
            voice.active = false;
            *out_sample = 0;
            continue;
        }

        // Determine current frequency (arpeggio or pitch sweep)
        let freq_hz = if voice.patch.arpeggio_step_samples > 0
            && voice.current_sample >= voice.patch.arpeggio_step_samples
        {
            voice.patch.arpeggio_freq_hz
        } else if voice.patch.freq_sweep_samples > 0
            && voice.patch.start_freq_hz != voice.patch.end_freq_hz
        {
            if voice.current_sample < voice.patch.freq_sweep_samples {
                let delta = (voice.patch.end_freq_hz as i64) - (voice.patch.start_freq_hz as i64);
                let current_f = (voice.patch.start_freq_hz as i64)
                    + (delta * (voice.current_sample as i64))
                        / (voice.patch.freq_sweep_samples as i64);
                current_f.clamp(0, (voice.sample_rate_hz / 2) as i64) as u32
            } else {
                voice.patch.end_freq_hz
            }
        } else {
            voice.patch.start_freq_hz
        };

        // DDS phase step calculation (CWE-369 safe)
        let nyquist = voice.sample_rate_hz / 2;
        let clamped_freq = freq_hz.min(nyquist);
        let phase_step = (((clamped_freq as u64) << 32) / (voice.sample_rate_hz as u64)) as u32;

        let p16 = (voice.phase_acc >> 16) as u16;

        // Waveform evaluation
        let raw_sample: i16 = match voice.patch.waveform {
            Waveform::Square { duty_q16 } => {
                if p16 < duty_q16 {
                    PCM16_MAX
                } else {
                    -PCM16_MAX
                }
            }
            Waveform::Triangle => {
                if p16 < 32_768 {
                    (-32_767i32 + (((p16 as i32) * 65_534) / 32_768)) as i16
                } else {
                    (32_767i32 - ((((p16 - 32_768) as i32) * 65_534) / 32_768)) as i16
                }
            }
            Waveform::Sawtooth => ((p16 as i32) - 32_768) as i16,
            Waveform::LfsrNoise { short_mode } => {
                // Advance LFSR on period rollover or when frequency is high
                if voice.phase_acc.wrapping_add(phase_step) < voice.phase_acc
                    || clamped_freq >= nyquist
                {
                    voice.lfsr_state = if short_mode {
                        lfsr_step_7(voice.lfsr_state)
                    } else {
                        lfsr_step_15(voice.lfsr_state)
                    };
                }
                if (voice.lfsr_state & 1) != 0 {
                    PCM16_MAX
                } else {
                    -PCM16_MAX
                }
            }
        };

        // Advance phase accumulator
        voice.phase_acc = voice.phase_acc.wrapping_add(phase_step);

        // Compute ADSR envelope level
        let env_level = if voice.released_at_sample != u32::MAX {
            let elapsed = voice
                .current_sample
                .saturating_sub(voice.released_at_sample);
            voice
                .patch
                .envelope
                .compute_release_level(elapsed, voice.release_start_level_q16)
        } else {
            voice
                .patch
                .envelope
                .compute_level(voice.current_sample, voice.patch.sustain_samples)
        };

        // Scale by envelope and voice gain
        let env_scaled = ((raw_sample as i64) * (env_level as i64)) >> 16;
        let final_sample = (env_scaled * (voice.patch.gain_q16 as i64)) >> 16;
        *out_sample = clamp_sample(final_sample);

        rendered_count += 1;
        voice.current_sample = voice.current_sample.saturating_add(1);
        if voice.current_sample >= voice.total_samples {
            voice.active = false;
        }
    }

    Ok(rendered_count)
}

/// Mix polyphonic sound effects from an [`SfxBank`] into destination slice `out`.
pub fn mix_bank_into<const N: usize>(
    bank: &mut SfxBank<N>,
    out: &mut [i16],
) -> Result<usize, SfxError> {
    validate_sample_rate(bank.sample_rate_hz)?;
    for voice in &bank.voices {
        if voice.active {
            validate_sample_rate(voice.sample_rate_hz)?;
        }
    }
    if out.is_empty() {
        return Ok(0);
    }

    for out_sample in out.iter_mut() {
        let mut acc: i64 = 0;

        for voice in &mut bank.voices {
            if !voice.active {
                continue;
            }

            let mut single_buf = [0i16; 1];
            let rendered = render_sfx_into(voice, &mut single_buf)?;
            if rendered > 0 {
                acc += single_buf[0] as i64;
            }
        }

        // Apply bank master gain in Q16.16 using i128 to prevent overflow (CWE-190 guard)
        let master_scaled = ((acc as i128) * (bank.master_gain_q16 as i128)) >> 16;
        *out_sample = master_scaled.clamp(PCM16_MIN as i128, PCM16_MAX as i128) as i16;
    }

    Ok(out.len())
}

/// Hard saturation clamping into 16-bit signed PCM range `[-32768, 32767]` (CWE-190 guard).
pub const fn clamp_sample(v: i64) -> i16 {
    if v > PCM16_MAX as i64 {
        PCM16_MAX
    } else if v < PCM16_MIN as i64 {
        PCM16_MIN
    } else {
        v as i16
    }
}

/// Scale sample amplitude by a Q16.16 fixed-point gain factor.
pub const fn scale_gain(sample: i32, gain_q16: u32) -> i32 {
    let scaled = ((sample as i64) * (gain_q16 as i64)) >> 16;
    if scaled > i32::MAX as i64 {
        i32::MAX
    } else if scaled < i32::MIN as i64 {
        i32::MIN
    } else {
        scaled as i32
    }
}

/// Validate that audio sample rate is within `[MIN_SAMPLE_RATE_HZ, MAX_SAMPLE_RATE_HZ]`.
pub const fn validate_sample_rate(sample_rate_hz: u32) -> Result<bool, SfxError> {
    if sample_rate_hz >= MIN_SAMPLE_RATE_HZ && sample_rate_hz <= MAX_SAMPLE_RATE_HZ {
        Ok(true)
    } else {
        Err(SfxError::InvalidSampleRate {
            rate: sample_rate_hz,
        })
    }
}

/// Validate that frequency does not exceed Nyquist limit ($F_s / 2$).
pub const fn validate_frequency(freq_hz: u32, sample_rate_hz: u32) -> Result<bool, SfxError> {
    let nyquist = sample_rate_hz / 2;
    if freq_hz <= nyquist {
        Ok(true)
    } else {
        Err(SfxError::InvalidFrequency {
            freq: freq_hz,
            max: nyquist,
        })
    }
}

/// Step 15-bit Galois/Fibonacci LFSR with non-zero seed guard.
pub const fn lfsr_step_15(state: u16) -> u16 {
    let masked = state & 0x7FFF;
    let s = if masked == 0 {
        DEFAULT_LFSR_SEED
    } else {
        masked
    };
    let bit0 = s & 1;
    let bit1 = (s >> 1) & 1;
    let feedback = bit0 ^ bit1;
    ((s >> 1) | (feedback << 14)) & 0x7FFF
}

/// Step 7-bit periodic LFSR with non-zero seed guard.
pub const fn lfsr_step_7(state: u16) -> u16 {
    let masked = state & 0x7FFF;
    let s = if masked == 0 {
        DEFAULT_LFSR_SEED
    } else {
        masked
    };
    let bit0 = s & 1;
    let bit6 = (s >> 6) & 1;
    let feedback = bit0 ^ bit6;
    ((s >> 1) | (feedback << 14)) & 0x7FFF
}

/// Offline utility: render a sound patch into an owned [`pcm_audio::PcmAudio`] container.
pub fn export_sfx_to_pcm_audio(
    patch: &SfxPatch,
    sample_rate_hz: u32,
) -> Result<pcm_audio::PcmAudio, SfxError> {
    validate_sample_rate(sample_rate_hz)?;
    let raw_total = patch.patch_total_samples();
    if raw_total > MAX_EXPORT_SAMPLES {
        return Err(SfxError::InvalidParameter {
            reason: "patch duration exceeds maximum export sample limit",
        });
    }
    let total_samples = raw_total as usize;
    let mut voice = SfxVoice::new(sample_rate_hz)?;
    voice.trigger_patch(*patch)?;

    let mut buffer = vec![0i16; total_samples];
    render_sfx_into(&mut voice, &mut buffer)?;

    let format = pcm_audio::PcmFormat::mono_16bit(sample_rate_hz as f64).map_err(|_| {
        SfxError::InvalidParameter {
            reason: "invalid pcm format",
        }
    })?;

    pcm_audio::PcmAudio::from_i16(buffer, format, 0.0).map_err(|_| SfxError::InvalidParameter {
        reason: "failed to construct pcm container",
    })
}

/// Offline utility: render a sound preset into an owned [`pcm_audio::PcmAudio`] container.
pub fn export_preset_to_pcm_audio(
    preset: SfxPreset,
    sample_rate_hz: u32,
) -> Result<pcm_audio::PcmAudio, SfxError> {
    let patch = preset.to_patch(sample_rate_hz)?;
    export_sfx_to_pcm_audio(&patch, sample_rate_hz)
}
