//! # Discrete Arcade Audio, 555-Timer & Coin Mechanism Simulator (GAME15c)
//!
//! A deterministic, zero-allocation discrete component audio and coin mechanism simulation
//! engine designed for 1970s coin-operated arcade machines and early discrete TTL logic boards.
//!
//! ```text
//!  +-------------------------------------------------------------------------+
//!  |                            Timer555Astable                              |
//!  |  - Discrete R1, R2, C astable timing circuit                            |
//!  |  - Cycle-accurate arcade square tones: Paddle (~480Hz), Wall (~240Hz)   |
//!  |  - Duty cycle: (R1 + R2) / (R1 + 2*R2)                                  |
//!  +------------------------------------+------------------------------------+
//!                                       |
//!  +------------------------------------+------------------------------------+
//!  |                          DiscreteNoiseFilter                            |
//!  |  - 17-bit / 15-bit shift-register LFSR noise generator                  |
//!  |  - Fixed-point RC single-pole low-pass filter (explosion rumble/thud)  |
//!  |  - Non-linear diode clipping / soft shaping stage                       |
//!  +------------------------------------+------------------------------------+
//!                                       |
//!  +------------------------------------+------------------------------------+
//!  |                          CoinMechDebounce                               |
//!  |  - Mechanical microswitch contact bounce glitch rejection              |
//!  |  - `logic-gates` SR-latch & shift-register digital filter               |
//!  |  - Saturating credit counter (CWE-190 guard)                            |
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
//! ### 555 Astable Multivibrator Dynamics
//!
//! In a classic NE555 astable configuration:
//! - Charging period: $t_{\text{high}} = \ln(2) \cdot (R_1 + R_2) \cdot C$
//! - Discharging period: $t_{\text{low}} = \ln(2) \cdot R_2 \cdot C$
//! - Total period: $T = \ln(2) \cdot (R_1 + 2 R_2) \cdot C$
//! - Frequency: $f = \frac{1}{\ln(2) \cdot (R_1 + 2 R_2) \cdot C} \approx \frac{1.44269504}{(R_1 + 2 R_2) \cdot C}$
//! - Duty cycle: $D = \frac{R_1 + R_2}{R_1 + 2 R_2} \in [0.5, 1.0)$
//!
//! Fixed-point Q16.16 duty cycle:
//! $$D_{\text{Q16}} = \frac{(R_1 + R_2) \times 65536}{R_1 + 2 R_2}$$
//!
//! ### RC Filter & Diode Shaping Dynamics
//!
//! The discrete single-pole low-pass filter operates via fixed-point alpha stepping:
//! $$y[n] = y[n-1] + \frac{\alpha_{\text{Q16}} \times (x[n] - y[n-1])}{65536}$$
//!
//! Non-linear diode clipping provides soft-knee compression:
//! For $|y| > V_{\text{th}}$, the conduction slope is attenuated to simulate diode dynamic resistance.
//!
//! ## Security Hardening & Robustness
//!
//! - **CWE-369 (Divide by Zero Protection)**: Division by zero is prevented across all timer
//!   and filter equations ($R_1 + 2R_2 \ne 0$, $C \ne 0$, $F_s \ne 0$).
//! - **CWE-190 (Integer Overflow Protection)**: Audio mixing utilizes 64-bit accumulators
//!   saturated into $[-32768, 32767]$; credit counters use saturating addition.
//! - **CWE-400 (Uncontrolled Resource Consumption)**: Zero heap allocation on hot audio-stepping
//!   paths; buffer allocation is pre-verified against maximum bounds.
//! - **CWE-20 (Improper Input Validation)**: Nyquist frequency limits ($F_s / 2$) and non-zero
//!   LFSR seed masks ensure fail-safe execution.

#![forbid(unsafe_code)]

pub mod capabilities;

/// Minimum valid audio sample rate in Hz (1 Hz).
pub const MIN_SAMPLE_RATE_HZ: u32 = 1;

/// Maximum valid audio sample rate in Hz (192 kHz).
pub const MAX_SAMPLE_RATE_HZ: u32 = 192_000;

/// Default arcade audio sample rate in Hz (44.1 kHz).
pub const DEFAULT_SAMPLE_RATE_HZ: u32 = 44_100;

/// Minimum signed 16-bit PCM integer (-32,768).
pub const PCM16_MIN: i16 = -32_768;

/// Maximum signed 16-bit PCM integer (+32,767).
pub const PCM16_MAX: i16 = 32_767;

/// Unity gain represented in Q16.16 fixed-point ($1.0 \times 65536 = 65536$).
pub const UNITY_GAIN_Q16: u32 = 65_536;

/// Default non-zero seed for 17-bit LFSR pseudo-random noise generator.
pub const DEFAULT_LFSR17_SEED: u32 = 0x1_FFFF;

/// Default non-zero seed for 15-bit LFSR pseudo-random noise generator.
pub const DEFAULT_LFSR15_SEED: u16 = 0x7FFF;

/// Default debounce threshold in clock cycles.
pub const DEFAULT_DEBOUNCE_CYCLES: u16 = 16;

/// Default maximum arcade credits before saturating.
pub const DEFAULT_MAX_CREDITS: u16 = 99;

/// Maximum allowable samples in an exported PCM buffer (CWE-400 guard).
pub const MAX_SAMPLES: usize = 16_777_216;

/// Constant numerator for 555 astable frequency calculation:
/// $1 / \ln(2) \times 10^{12} \approx 1_442_695_040_889$.
pub const TIMER_555_NUMERATOR_PICO: u128 = 1_442_695_040_889;

/// Errors that can occur during discrete audio timer and debouncing simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerAudioError {
    /// Audio sample rate was zero or exceeded [`MAX_SAMPLE_RATE_HZ`].
    InvalidSampleRate { rate: u32 },
    /// Timing resistor sum $(R_1 + 2 R_2)$ was zero (CWE-369).
    ZeroResistance,
    /// Timing capacitor capacitance was zero (CWE-369).
    ZeroCapacitance,
    /// Calculated frequency exceeded the Nyquist limit ($F_s / 2$).
    FrequencyTooHigh { freq: u32, max: u32 },
    /// General invalid parameter or configuration error.
    InvalidParameter { reason: &'static str },
    /// Destination buffer is smaller than requested frame count.
    BufferTooSmall,
    /// Credit counter reached upper saturation limit.
    CreditOverflow,
    /// Error interfacing with `audio-mixer-realtime` or ring buffer.
    MixerIntegrationError,
}

impl std::fmt::Display for TimerAudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSampleRate { rate } => write!(f, "invalid sample rate: {rate} Hz"),
            Self::ZeroResistance => write!(f, "555 timer timing resistance R1 + 2*R2 cannot be zero"),
            Self::ZeroCapacitance => write!(f, "555 timer timing capacitance cannot be zero"),
            Self::FrequencyTooHigh { freq, max } => {
                write!(f, "frequency {freq} Hz exceeds Nyquist limit {max} Hz")
            }
            Self::InvalidParameter { reason } => write!(f, "invalid parameter: {reason}"),
            Self::BufferTooSmall => write!(f, "destination buffer is too small"),
            Self::CreditOverflow => write!(f, "credit counter reached saturation limit"),
            Self::MixerIntegrationError => write!(f, "failed to route audio to mixer ring buffer"),
        }
    }
}

impl std::error::Error for TimerAudioError {}

/// Diode clipping and wave-shaping modes for analog filter simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiodeMode {
    /// Symmetrical back-to-back antiparallel diodes (clips both positive and negative peaks).
    Symmetrical,
    /// Asymmetrical single diode (clips positive peaks only).
    Asymmetrical,
    /// Hard threshold clipping.
    HardClip,
}

// ---------------------------------------------------------------------------
// Pure helper functions and mathematical routines
// ---------------------------------------------------------------------------

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
pub const fn validate_sample_rate(sample_rate_hz: u32) -> Result<bool, TimerAudioError> {
    if sample_rate_hz >= MIN_SAMPLE_RATE_HZ && sample_rate_hz <= MAX_SAMPLE_RATE_HZ {
        Ok(true)
    } else {
        Err(TimerAudioError::InvalidSampleRate {
            rate: sample_rate_hz,
        })
    }
}

/// Calculate 555 astable multivibrator oscillation frequency in Hz from component values.
///
/// Uses standard formula $f = \frac{1}{\ln(2) \cdot (R_1 + 2 R_2) \cdot C}$ with $C$ in picofarads.
/// Includes strict CWE-369 divide-by-zero guards.
pub const fn calculate_555_frequency(
    r1_ohms: u32,
    r2_ohms: u32,
    c_picofarads: u64,
) -> Result<u32, TimerAudioError> {
    let r_sum = (r1_ohms as u64).saturating_add(2 * (r2_ohms as u64));
    if r_sum == 0 {
        return Err(TimerAudioError::ZeroResistance);
    }
    if c_picofarads == 0 {
        return Err(TimerAudioError::ZeroCapacitance);
    }
    let denom = (r_sum as u128) * (c_picofarads as u128);
    if denom == 0 {
        return Err(TimerAudioError::ZeroResistance);
    }
    let raw_freq = TIMER_555_NUMERATOR_PICO / denom;
    let freq = if raw_freq > u32::MAX as u128 {
        u32::MAX
    } else {
        raw_freq as u32
    };
    Ok(freq)
}

/// Calculate 555 astable multivibrator duty cycle in Q16.16 from component resistances.
///
/// Formula: $D = \frac{R_1 + R_2}{R_1 + 2 R_2}$.
pub const fn calculate_555_duty_q16(
    r1_ohms: u32,
    r2_ohms: u32,
) -> Result<u16, TimerAudioError> {
    let r_denom = (r1_ohms as u64).saturating_add(2 * (r2_ohms as u64));
    if r_denom == 0 {
        return Err(TimerAudioError::ZeroResistance);
    }
    let r_num = (r1_ohms as u64).saturating_add(r2_ohms as u64);
    let raw_duty = (r_num * 65536) / r_denom;
    let duty = if raw_duty > u16::MAX as u64 {
        u16::MAX
    } else {
        raw_duty as u16
    };
    Ok(duty)
}

/// Step 17-bit Galois/Fibonacci LFSR with masked non-zero seed guard.
///
/// Implements maximal-length primitive polynomial $x^{17} + x^{14} + 1$ with period 131,071.
pub const fn lfsr17_step(state: u32) -> u32 {
    let masked = state & 0x1_FFFF;
    let s = if masked == 0 {
        DEFAULT_LFSR17_SEED
    } else {
        masked
    };
    let bit0 = s & 1;
    let bit3 = (s >> 3) & 1;
    let feedback = bit0 ^ bit3;
    ((s >> 1) | (feedback << 16)) & 0x1_FFFF
}

/// Step 15-bit Galois/Fibonacci LFSR with masked non-zero seed guard.
///
/// Period: 32,767 states.
pub const fn lfsr15_step(state: u16) -> u16 {
    let masked = state & 0x7FFF;
    let s = if masked == 0 {
        DEFAULT_LFSR15_SEED
    } else {
        masked
    };
    let bit0 = s & 1;
    let bit1 = (s >> 1) & 1;
    let feedback = bit0 ^ bit1;
    ((s >> 1) | (feedback << 14)) & 0x7FFF
}

/// Compute single-pole RC low-pass filter alpha coefficient in Q16.16 fixed-point.
///
/// Formula: $\alpha = \frac{2\pi f_c}{F_s + 2\pi f_c} \times 65536$.
pub const fn rc_filter_alpha_q16(cutoff_hz: u32, sample_rate_hz: u32) -> u32 {
    if cutoff_hz == 0 || sample_rate_hz == 0 {
        return 0;
    }
    // Fixed-point approximation for 2 * pi * 10000 ~= 62832
    let two_pi_fc = (cutoff_hz as u64) * 62832 / 10000;
    let denom = (sample_rate_hz as u64) + two_pi_fc;
    if denom == 0 {
        return 0;
    }
    let alpha = (two_pi_fc * 65536) / denom;
    if alpha > 65536 {
        65536
    } else if alpha < 1 {
        1
    } else {
        alpha as u32
    }
}

/// Hard threshold clipping into `[-threshold, threshold]`.
pub const fn diode_clip_sample(sample: i32, threshold: i16) -> i16 {
    let th = threshold.unsigned_abs() as i32;
    if sample > th {
        clamp_sample(th as i64)
    } else if sample < -th {
        clamp_sample((-th) as i64)
    } else {
        clamp_sample(sample as i64)
    }
}

/// Shape audio sample using analog diode clipping simulation.
pub fn diode_shape_sample(sample: i32, threshold: i16, mode: DiodeMode) -> i16 {
    let th = threshold.unsigned_abs() as i32;
    match mode {
        DiodeMode::HardClip => diode_clip_sample(sample, threshold),
        DiodeMode::Symmetrical => {
            if sample > th {
                let over = sample - th;
                let compressed = th + (over * 16384) / 65536;
                clamp_sample(compressed as i64)
            } else if sample < -th {
                let over = (-sample) - th;
                let compressed = -th - (over * 16384) / 65536;
                clamp_sample(compressed as i64)
            } else {
                clamp_sample(sample as i64)
            }
        }
        DiodeMode::Asymmetrical => {
            if sample > th {
                let over = sample - th;
                let compressed = th + (over * 16384) / 65536;
                clamp_sample(compressed as i64)
            } else {
                clamp_sample(sample as i64)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Subsystem 1: 555 Astable Multivibrator
// ---------------------------------------------------------------------------

/// Deterministic fixed-point / cycle-accurate NE555 astable timer simulator.
///
/// Synthesizes classic arcade audio tones:
/// - Pong paddle deflection tone (~480 Hz)
/// - Wall collision tone (~240 Hz)
/// - Score / out-of-bounds tone (~120 Hz / 960 Hz)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timer555Astable {
    pub r1_ohms: u32,
    pub r2_ohms: u32,
    pub c_picofarads: u64,
    pub sample_rate_hz: u32,
    pub frequency_hz: u32,
    pub duty_q16: u16,
    pub phase_acc: u32,
    pub phase_inc: u32,
    pub output_high: bool,
    pub gain_q16: u32,
    pub amplitude: i16,
}

impl Timer555Astable {
    /// Construct a new 555 astable timer from discrete component parameters.
    pub fn new_astable(
        r1_ohms: u32,
        r2_ohms: u32,
        c_picofarads: u64,
        sample_rate_hz: u32,
    ) -> Result<Self, TimerAudioError> {
        validate_sample_rate(sample_rate_hz)?;
        let freq = calculate_555_frequency(r1_ohms, r2_ohms, c_picofarads)?;
        let nyquist = sample_rate_hz / 2;
        if freq > nyquist {
            return Err(TimerAudioError::FrequencyTooHigh {
                freq,
                max: nyquist,
            });
        }
        let duty = calculate_555_duty_q16(r1_ohms, r2_ohms)?;
        let phase_inc = (((freq as u64) << 32) / (sample_rate_hz as u64)) as u32;

        Ok(Self {
            r1_ohms,
            r2_ohms,
            c_picofarads,
            sample_rate_hz,
            frequency_hz: freq,
            duty_q16: duty,
            phase_acc: 0,
            phase_inc,
            output_high: true,
            gain_q16: UNITY_GAIN_Q16,
            amplitude: 20_000,
        })
    }

    /// Preset: Pong paddle deflection tone (~480 Hz).
    ///
    /// Component values: $R_1 = 1\,\text{k}\Omega$, $R_2 = 14.5\,\text{k}\Omega$, $C = 100\,\text{nF}$.
    pub fn paddle_hit_timer(sample_rate_hz: u32) -> Result<Self, TimerAudioError> {
        Self::new_astable(1_000, 14_500, 100_000, sample_rate_hz)
    }

    /// Preset: Pong wall collision tone (~240 Hz).
    ///
    /// Component values: $R_1 = 1\,\text{k}\Omega$, $R_2 = 29.5\,\text{k}\Omega$, $C = 100\,\text{nF}$.
    pub fn wall_bounce_timer(sample_rate_hz: u32) -> Result<Self, TimerAudioError> {
        Self::new_astable(1_000, 29_500, 100_000, sample_rate_hz)
    }

    /// Preset: Pong score tone (~120 Hz).
    ///
    /// Component values: $R_1 = 1\,\text{k}\Omega$, $R_2 = 59.5\,\text{k}\Omega$, $C = 100\,\text{nF}$.
    pub fn score_timer(sample_rate_hz: u32) -> Result<Self, TimerAudioError> {
        Self::new_astable(1_000, 59_500, 100_000, sample_rate_hz)
    }

    /// Preset: High score / game-over beep tone (~960 Hz).
    ///
    /// Component values: $R_1 = 1\,\text{k}\Omega$, $R_2 = 7.0\,\text{k}\Omega$, $C = 100\,\text{nF}$.
    pub fn high_score_timer(sample_rate_hz: u32) -> Result<Self, TimerAudioError> {
        Self::new_astable(1_000, 7_000, 100_000, sample_rate_hz)
    }

    /// Step the 555 timer by one audio sample and produce a signed 16-bit PCM output.
    pub fn step_timer(&mut self) -> i16 {
        let duty_threshold = ((self.duty_q16 as u64) * 65536) as u32;
        self.output_high = self.phase_acc < duty_threshold;
        self.phase_acc = self.phase_acc.wrapping_add(self.phase_inc);

        let base_sample = if self.output_high {
            self.amplitude
        } else {
            -self.amplitude
        };
        let scaled = ((base_sample as i64) * (self.gain_q16 as i64)) >> 16;
        clamp_sample(scaled)
    }

    /// Query current oscillation frequency in Hz.
    pub fn timer_frequency_hz(&self) -> u32 {
        self.frequency_hz
    }

    /// Query duty cycle in Q16.16 fixed-point format.
    pub fn timer_duty_q16(&self) -> u16 {
        self.duty_q16
    }

    /// Query current digital output pin state (Pin 3: HIGH or LOW).
    pub fn timer_output_high(&self) -> bool {
        self.output_high
    }

    /// Calculate internal timing capacitor voltage in Q16.16 ($V_{CC} = 65536$).
    ///
    /// The capacitor voltage charges between $V_{CC}/3$ (21845) and $2V_{CC}/3$ (43690).
    pub fn capacitor_voltage_q16(&self) -> u32 {
        let duty_threshold = ((self.duty_q16 as u64) * 65536) as u32;
        let v_low = 21_845u32;
        let v_high = 43_690u32;
        let v_span = v_high - v_low;
        if self.phase_acc < duty_threshold && duty_threshold > 0 {
            let frac = ((self.phase_acc as u64) * 65536) / (duty_threshold as u64);
            v_low + (((v_span as u64) * frac) >> 16) as u32
        } else {
            let low_span = u32::MAX - duty_threshold;
            if low_span > 0 {
                let pos = self.phase_acc - duty_threshold;
                let frac = ((pos as u64) * 65536) / (low_span as u64);
                v_high.saturating_sub((((v_span as u64) * frac) >> 16) as u32)
            } else {
                v_low
            }
        }
    }

    /// Reset internal phase accumulator and output state to cycle start.
    pub fn reset_timer(&mut self) {
        self.phase_acc = 0;
        self.output_high = true;
    }

    /// Reconfigure component values dynamically.
    pub fn set_timer_components(
        &mut self,
        r1_ohms: u32,
        r2_ohms: u32,
        c_picofarads: u64,
    ) -> Result<(), TimerAudioError> {
        let freq = calculate_555_frequency(r1_ohms, r2_ohms, c_picofarads)?;
        let nyquist = self.sample_rate_hz / 2;
        if freq > nyquist {
            return Err(TimerAudioError::FrequencyTooHigh {
                freq,
                max: nyquist,
            });
        }
        self.r1_ohms = r1_ohms;
        self.r2_ohms = r2_ohms;
        self.c_picofarads = c_picofarads;
        self.frequency_hz = freq;
        self.duty_q16 = calculate_555_duty_q16(r1_ohms, r2_ohms)?;
        self.phase_inc = (((freq as u64) << 32) / (self.sample_rate_hz as u64)) as u32;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Subsystem 2: Shift-Register Noise & Diode-Shaped Analog Filter
// ---------------------------------------------------------------------------

/// Deterministic 17-bit / 15-bit shift register noise generator paired with an RC low-pass filter
/// and non-linear diode clipping stage for arcade explosion and impact sounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscreteNoiseFilter {
    pub lfsr17_state: u32,
    pub lfsr15_state: u16,
    pub use_17bit: bool,
    pub cutoff_hz: u32,
    pub sample_rate_hz: u32,
    pub alpha_q16: u32,
    pub rc_state: i32,
    pub diode_threshold: i16,
    pub diode_mode: DiodeMode,
    pub gain_q16: u32,
    pub last_sample: i16,
}

impl DiscreteNoiseFilter {
    /// Construct a new noise filter module with specified parameters.
    pub fn new_noise_filter(
        use_17bit: bool,
        cutoff_hz: u32,
        sample_rate_hz: u32,
        diode_threshold: i16,
        diode_mode: DiodeMode,
    ) -> Result<Self, TimerAudioError> {
        validate_sample_rate(sample_rate_hz)?;
        let alpha = rc_filter_alpha_q16(cutoff_hz, sample_rate_hz);
        Ok(Self {
            lfsr17_state: DEFAULT_LFSR17_SEED,
            lfsr15_state: DEFAULT_LFSR15_SEED,
            use_17bit,
            cutoff_hz,
            sample_rate_hz,
            alpha_q16: alpha,
            rc_state: 0,
            diode_threshold,
            diode_mode,
            gain_q16: UNITY_GAIN_Q16,
            last_sample: 0,
        })
    }

    /// Preset: Heavy arcade explosion sound (17-bit noise, 250 Hz RC low-pass, symmetrical diode shaping).
    pub fn explosion_filter(sample_rate_hz: u32) -> Result<Self, TimerAudioError> {
        Self::new_noise_filter(true, 250, sample_rate_hz, 16_384, DiodeMode::Symmetrical)
    }

    /// Preset: Low-frequency arcade thud sound (15-bit noise, 100 Hz RC low-pass, asymmetrical diode shaping).
    pub fn thud_filter(sample_rate_hz: u32) -> Result<Self, TimerAudioError> {
        Self::new_noise_filter(false, 100, sample_rate_hz, 12_000, DiodeMode::Asymmetrical)
    }

    /// Advance LFSR noise by one clock, step RC low-pass filter, apply diode shaping, and return PCM sample.
    pub fn step_filter(&mut self) -> i16 {
        let raw_bit = if self.use_17bit {
            self.lfsr17_state = lfsr17_step(self.lfsr17_state);
            (self.lfsr17_state & 1) as i32
        } else {
            self.lfsr15_state = lfsr15_step(self.lfsr15_state);
            (self.lfsr15_state & 1) as i32
        };

        let raw_noise = if raw_bit != 0 { 20_000i32 } else { -20_000i32 };
        let diff = raw_noise - self.rc_state;
        let delta = ((diff as i64) * (self.alpha_q16 as i64)) >> 16;
        self.rc_state = (self.rc_state as i64 + delta).clamp(-32768, 32767) as i32;

        let shaped = diode_shape_sample(self.rc_state, self.diode_threshold, self.diode_mode);
        let scaled = ((shaped as i64) * (self.gain_q16 as i64)) >> 16;
        let final_sample = clamp_sample(scaled);
        self.last_sample = final_sample;
        final_sample
    }

    /// Reset filter state and restore initial LFSR seeds.
    pub fn reset_filter(&mut self) {
        self.lfsr17_state = DEFAULT_LFSR17_SEED;
        self.lfsr15_state = DEFAULT_LFSR15_SEED;
        self.rc_state = 0;
        self.last_sample = 0;
    }

    /// Update RC cutoff frequency in Hz.
    pub fn set_cutoff_hz(&mut self, cutoff_hz: u32) {
        self.cutoff_hz = cutoff_hz;
        self.alpha_q16 = rc_filter_alpha_q16(cutoff_hz, self.sample_rate_hz);
    }

    /// Query fixed-point filter alpha in Q16.16 format.
    pub fn filter_alpha_q16(&self) -> u32 {
        self.alpha_q16
    }

    /// Query last generated output sample.
    pub fn filter_output_sample(&self) -> i16 {
        self.last_sample
    }
}

// ---------------------------------------------------------------------------
// Subsystem 3: Mechanical Coin Acceptor Switch Debounce & Credit Latch
// ---------------------------------------------------------------------------

/// Mechanical coin acceptor switch debouncer and credit latching logic.
///
/// Features:
/// - Rejection of mechanical contact bounce glitches shorter than debounce threshold.
/// - Functional gate-level SR-latch debouncing using `logic_gates::sequential::sr_latch`.
/// - Credit latching with saturating credit counter (CWE-190 overflow protection).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoinMechDebounce {
    pub debounce_threshold_cycles: u16,
    pub consecutive_high: u16,
    pub consecutive_low: u16,
    pub debounced_state: bool,
    pub sr_q: u8,
    pub sr_q_bar: u8,
    pub credits: u16,
    pub max_credits: u16,
    pub coin_latched: bool,
}

impl CoinMechDebounce {
    /// Construct a coin mechanism debouncer with configurable threshold and credit limits.
    pub fn new_debouncer(threshold_cycles: u16, max_credits: u16) -> Self {
        let th = if threshold_cycles == 0 {
            DEFAULT_DEBOUNCE_CYCLES
        } else {
            threshold_cycles
        };
        let max_c = if max_credits == 0 {
            DEFAULT_MAX_CREDITS
        } else {
            max_credits
        };
        Self {
            debounce_threshold_cycles: th,
            consecutive_high: 0,
            consecutive_low: 0,
            debounced_state: false,
            sr_q: 0,
            sr_q_bar: 1,
            credits: 0,
            max_credits: max_c,
            coin_latched: false,
        }
    }

    /// Step digital shift-register/counter debouncer by one clock cycle.
    ///
    /// Returns `true` on the exact clock cycle when a clean coin credit is registered.
    /// Glitches shorter than `debounce_threshold_cycles` are rejected without triggering credits.
    pub fn step_debounce(&mut self, raw_input: bool) -> bool {
        if raw_input {
            self.consecutive_high = self.consecutive_high.saturating_add(1);
            self.consecutive_low = 0;
            if self.consecutive_high >= self.debounce_threshold_cycles && !self.debounced_state {
                self.debounced_state = true;
                self.coin_latched = true;
                if self.credits < self.max_credits {
                    self.credits = self.credits.saturating_add(1);
                }
                return true;
            }
        } else {
            self.consecutive_low = self.consecutive_low.saturating_add(1);
            self.consecutive_high = 0;
            if self.consecutive_low >= self.debounce_threshold_cycles {
                self.debounced_state = false;
            }
        }
        false
    }

    /// Step gate-level cross-coupled SR-latch debouncer using `logic_gates::sequential::sr_latch`.
    ///
    /// Simulates hardware SPDT coin switch with Normally-Open (`no_val`) and Normally-Closed (`nc_val`) contacts.
    /// Returns `true` on rising edge of latched credit.
    pub fn step_debounce_sr(&mut self, no_val: u8, nc_val: u8) -> bool {
        let prev_q = self.sr_q;
        let (new_q, new_q_bar) =
            logic_gates::sequential::sr_latch(no_val, nc_val, self.sr_q, self.sr_q_bar);
        self.sr_q = new_q;
        self.sr_q_bar = new_q_bar;

        if prev_q == 0 && new_q == 1 {
            self.coin_latched = true;
            if self.credits < self.max_credits {
                self.credits = self.credits.saturating_add(1);
            }
            true
        } else {
            false
        }
    }

    /// Query current registered credits.
    pub fn current_credits(&self) -> u16 {
        self.credits
    }

    /// Manually insert a service/test credit with saturating addition.
    pub fn insert_test_credit(&mut self) {
        if self.credits < self.max_credits {
            self.credits = self.credits.saturating_add(1);
        }
    }

    /// Consume one credit for game play.
    ///
    /// Returns `true` if a credit was available and consumed; `false` otherwise.
    pub fn consume_credit(&mut self) -> bool {
        if self.credits > 0 {
            self.credits -= 1;
            true
        } else {
            false
        }
    }

    /// Reset debouncer state and clear coin latch.
    pub fn reset_debounce(&mut self) {
        self.consecutive_high = 0;
        self.consecutive_low = 0;
        self.debounced_state = false;
        self.sr_q = 0;
        self.sr_q_bar = 1;
        self.coin_latched = false;
    }

    /// Query whether a coin pulse is currently latched.
    pub fn is_coin_latched(&self) -> bool {
        self.coin_latched
    }

    /// Acknowledge and clear latched coin flag.
    pub fn clear_coin_latch(&mut self) {
        self.coin_latched = false;
    }
}

// ---------------------------------------------------------------------------
// Subsystem 4: Composite Arcade Audio System & Zero-Alloc PCM Rendering
// ---------------------------------------------------------------------------

/// Integrated discrete arcade cabinet audio system combining paddle/wall 555 timers,
/// noise filter, and coin mechanism debouncer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscreteArcadeAudio {
    pub paddle_timer: Timer555Astable,
    pub wall_timer: Timer555Astable,
    pub score_timer: Timer555Astable,
    pub noise_filter: DiscreteNoiseFilter,
    pub coin_mech: CoinMechDebounce,
    pub sample_rate_hz: u32,
    pub paddle_remaining_samples: u32,
    pub wall_remaining_samples: u32,
    pub score_remaining_samples: u32,
    pub noise_remaining_samples: u32,
    pub master_gain_q16: u32,
}

impl DiscreteArcadeAudio {
    /// Construct a new composite arcade audio generator at the given sample rate.
    pub fn new_arcade_audio(sample_rate_hz: u32) -> Result<Self, TimerAudioError> {
        let paddle = Timer555Astable::paddle_hit_timer(sample_rate_hz)?;
        let wall = Timer555Astable::wall_bounce_timer(sample_rate_hz)?;
        let score = Timer555Astable::score_timer(sample_rate_hz)?;
        let noise = DiscreteNoiseFilter::explosion_filter(sample_rate_hz)?;
        let coin = CoinMechDebounce::new_debouncer(DEFAULT_DEBOUNCE_CYCLES, DEFAULT_MAX_CREDITS);

        Ok(Self {
            paddle_timer: paddle,
            wall_timer: wall,
            score_timer: score,
            noise_filter: noise,
            coin_mech: coin,
            sample_rate_hz,
            paddle_remaining_samples: 0,
            wall_remaining_samples: 0,
            score_remaining_samples: 0,
            noise_remaining_samples: 0,
            master_gain_q16: UNITY_GAIN_Q16,
        })
    }

    /// Trigger Pong paddle deflection audio cue (~480 Hz) for a duration in samples.
    pub fn trigger_paddle_tone(&mut self, duration_samples: u32) {
        self.paddle_remaining_samples = duration_samples;
        self.paddle_timer.reset_timer();
    }

    /// Trigger Pong wall collision audio cue (~240 Hz) for a duration in samples.
    pub fn trigger_wall_tone(&mut self, duration_samples: u32) {
        self.wall_remaining_samples = duration_samples;
        self.wall_timer.reset_timer();
    }

    /// Trigger score / boundary audio cue (~120 Hz) for a duration in samples.
    pub fn trigger_score_tone(&mut self, duration_samples: u32) {
        self.score_remaining_samples = duration_samples;
        self.score_timer.reset_timer();
    }

    /// Trigger explosion audio cue (diode-shaped filtered noise) for a duration in samples.
    pub fn trigger_explosion(&mut self, duration_samples: u32) {
        self.noise_remaining_samples = duration_samples;
        self.noise_filter.reset_filter();
    }

    /// Step composite arcade audio system by one frame and mix active channels into signed 16-bit PCM.
    pub fn step_arcade_audio(&mut self) -> i16 {
        let mut acc: i64 = 0;
        if self.paddle_remaining_samples > 0 {
            acc += self.paddle_timer.step_timer() as i64;
            self.paddle_remaining_samples -= 1;
        }
        if self.wall_remaining_samples > 0 {
            acc += self.wall_timer.step_timer() as i64;
            self.wall_remaining_samples -= 1;
        }
        if self.score_remaining_samples > 0 {
            acc += self.score_timer.step_timer() as i64;
            self.score_remaining_samples -= 1;
        }
        if self.noise_remaining_samples > 0 {
            acc += self.noise_filter.step_filter() as i64;
            self.noise_remaining_samples -= 1;
        }
        let scaled = (acc * (self.master_gain_q16 as i64)) >> 16;
        clamp_sample(scaled)
    }

    /// Render frames directly into a destination buffer with zero heap allocation.
    pub fn render_audio_frames(&mut self, out: &mut [i16]) -> usize {
        for sample in out.iter_mut() {
            *sample = self.step_arcade_audio();
        }
        out.len()
    }

    /// Reset all active sound cues and channel generators.
    pub fn reset_arcade_audio(&mut self) {
        self.paddle_remaining_samples = 0;
        self.wall_remaining_samples = 0;
        self.score_remaining_samples = 0;
        self.noise_remaining_samples = 0;
        self.paddle_timer.reset_timer();
        self.wall_timer.reset_timer();
        self.score_timer.reset_timer();
        self.noise_filter.reset_filter();
    }
}

// ---------------------------------------------------------------------------
// Zero-allocation PCM rendering & container exports
// ---------------------------------------------------------------------------

/// Render audio from a 555 timer directly into a caller-supplied slice with zero heap allocation.
pub fn render_timer_audio_into(
    timer: &mut Timer555Astable,
    out: &mut [i16],
) -> Result<usize, TimerAudioError> {
    validate_sample_rate(timer.sample_rate_hz)?;
    for sample in out.iter_mut() {
        *sample = timer.step_timer();
    }
    Ok(out.len())
}

/// Render audio from a noise filter directly into a caller-supplied slice with zero heap allocation.
pub fn render_noise_audio_into(
    filter: &mut DiscreteNoiseFilter,
    out: &mut [i16],
) -> Result<usize, TimerAudioError> {
    validate_sample_rate(filter.sample_rate_hz)?;
    for sample in out.iter_mut() {
        *sample = filter.step_filter();
    }
    Ok(out.len())
}

/// Render composite arcade system audio directly into a caller-supplied slice with zero heap allocation.
pub fn render_system_audio_into(
    system: &mut DiscreteArcadeAudio,
    out: &mut [i16],
) -> Result<usize, TimerAudioError> {
    validate_sample_rate(system.sample_rate_hz)?;
    Ok(system.render_audio_frames(out))
}

/// Route rendered 555 timer frames into an `audio_mixer_realtime::PcmRingBuffer` with zero allocation.
pub fn render_timer_to_ring_buffer<const N: usize>(
    timer: &mut Timer555Astable,
    ring: &mut audio_mixer_realtime::PcmRingBuffer<N>,
    frames: usize,
) -> Result<usize, TimerAudioError> {
    validate_sample_rate(timer.sample_rate_hz)?;
    let mut written = 0;
    for _ in 0..frames {
        let sample = timer.step_timer();
        if ring.push(sample).is_ok() {
            written += 1;
        } else {
            return Err(TimerAudioError::MixerIntegrationError);
        }
    }
    Ok(written)
}

/// Route rendered noise filter frames into an `audio_mixer_realtime::PcmRingBuffer` with zero allocation.
pub fn render_noise_to_ring_buffer<const N: usize>(
    filter: &mut DiscreteNoiseFilter,
    ring: &mut audio_mixer_realtime::PcmRingBuffer<N>,
    frames: usize,
) -> Result<usize, TimerAudioError> {
    validate_sample_rate(filter.sample_rate_hz)?;
    let mut written = 0;
    for _ in 0..frames {
        let sample = filter.step_filter();
        if ring.push(sample).is_ok() {
            written += 1;
        } else {
            return Err(TimerAudioError::MixerIntegrationError);
        }
    }
    Ok(written)
}

/// Offline utility: Render 555 timer audio into an owned [`pcm_audio::PcmAudio`] container.
pub fn export_timer_to_pcm_audio(
    timer: &mut Timer555Astable,
    sample_count: usize,
) -> Result<pcm_audio::PcmAudio, TimerAudioError> {
    validate_sample_rate(timer.sample_rate_hz)?;
    if sample_count > MAX_SAMPLES {
        return Err(TimerAudioError::InvalidParameter {
            reason: "requested sample count exceeds maximum allowable",
        });
    }

    let mut buffer = vec![0i16; sample_count];
    render_timer_audio_into(timer, &mut buffer)?;

    let format = pcm_audio::PcmFormat::mono_16bit(timer.sample_rate_hz as f64)
        .map_err(|_| TimerAudioError::InvalidParameter {
            reason: "invalid pcm format",
        })?;

    pcm_audio::PcmAudio::from_i16(buffer, format, 0.0).map_err(|_| {
        TimerAudioError::InvalidParameter {
            reason: "failed to construct pcm container",
        }
    })
}

/// Offline utility: Render noise filter audio into an owned [`pcm_audio::PcmAudio`] container.
pub fn export_noise_to_pcm_audio(
    filter: &mut DiscreteNoiseFilter,
    sample_count: usize,
) -> Result<pcm_audio::PcmAudio, TimerAudioError> {
    validate_sample_rate(filter.sample_rate_hz)?;
    if sample_count > MAX_SAMPLES {
        return Err(TimerAudioError::InvalidParameter {
            reason: "requested sample count exceeds maximum allowable",
        });
    }

    let mut buffer = vec![0i16; sample_count];
    render_noise_audio_into(filter, &mut buffer)?;

    let format = pcm_audio::PcmFormat::mono_16bit(filter.sample_rate_hz as f64)
        .map_err(|_| TimerAudioError::InvalidParameter {
            reason: "invalid pcm format",
        })?;

    pcm_audio::PcmAudio::from_i16(buffer, format, 0.0).map_err(|_| {
        TimerAudioError::InvalidParameter {
            reason: "failed to construct pcm container",
        }
    })
}

/// Offline utility: Render composite arcade audio into an owned [`pcm_audio::PcmAudio`] container.
pub fn export_system_to_pcm_audio(
    system: &mut DiscreteArcadeAudio,
    sample_count: usize,
) -> Result<pcm_audio::PcmAudio, TimerAudioError> {
    validate_sample_rate(system.sample_rate_hz)?;
    if sample_count > MAX_SAMPLES {
        return Err(TimerAudioError::InvalidParameter {
            reason: "requested sample count exceeds maximum allowable",
        });
    }

    let mut buffer = vec![0i16; sample_count];
    render_system_audio_into(system, &mut buffer)?;

    let format = pcm_audio::PcmFormat::mono_16bit(system.sample_rate_hz as f64)
        .map_err(|_| TimerAudioError::InvalidParameter {
            reason: "invalid pcm format",
        })?;

    pcm_audio::PcmAudio::from_i16(buffer, format, 0.0).map_err(|_| {
        TimerAudioError::InvalidParameter {
            reason: "failed to construct pcm container",
        }
    })
}
