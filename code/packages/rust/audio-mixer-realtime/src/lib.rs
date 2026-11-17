//! # Real-time Fixed-Point Audio Mixer and Voice Allocator (MUS08)
//!
//! A zero-allocation, deterministic digital audio mixer designed for low-latency
//! arcade and real-time game audio pipelines.
//!
//! ```text
//!  +------------------+     +------------------+     +------------------+
//!  | Voice 0 (PCM16)  |     | Voice 1 (PCM16)  | ... | Voice 15 (PCM16) |
//!  +--------+---------+     +--------+---------+     +--------+---------+
//!           |                        |                        |
//!    gain_q16, pitch          gain_q16, pitch          gain_q16, pitch
//!           |                        |                        |
//!           v                        v                        v
//!  +--------------------------------------------------------------------+
//!  |                    64-bit Signed Accumulator                       |
//!  |            acc = sum_i( (sample_i * voice_gain_i) >> 16 )         |
//!  +---------------------------------+----------------------------------+
//!                                    |
//!                             master_gain_q16
//!                                    |
//!                                    v
//!  +--------------------------------------------------------------------+
//!  |             Hard Saturation Clamping (CWE-190 Guard)               |
//!  |                  clamp(acc, -32768, 32767)                         |
//!  +---------------------------------+----------------------------------+
//!                                    |
//!                                    v
//!                       [ out_mono: &mut [i16] ]
//!                                    |
//!                   +----------------+----------------+
//!                   |                                 |
//!                   v                                 v
//!      [ PcmRingBuffer<N> ]               [ PcmPlaybackBuffer ]
//!   (SPSC Lockless Staging)             (Device Sink Bridge: MUS02)
//! ```
//!
//! ## Mathematical Formulation
//!
//! ### Fixed-Point Arithmetic (Q16.16)
//!
//! Audio sample gains and phase increments are computed using standard Q16.16
//! fixed-point representations to eliminate floating-point non-determinism:
//!
//! - $\text{UNITY\_GAIN\_Q16} = 65536 = 1.0\times$
//! - $\text{Gain}(g) \in [0, 262144] = [0.0\times, 4.0\times]$
//! - $\text{PitchStep}(p) \in [16384, 262144] = [0.25\times, 4.0\times]$
//!
//! For each active voice $i$ at output frame $n$:
//!
//! $$\text{sample\_index} = \lfloor \phi_i[n] \gg 16 \rfloor$$
//!
//! $$\text{voice\_contrib}_i = \frac{x_i[\text{sample\_index}] \times g_i}{65536}$$
//!
//! $$\text{mixed\_sample} = \text{clamp}\left( \frac{\sum_i \text{voice\_contrib}_i \times g_{\text{master}}}{65536}, -32768, 32767 \right)$$
//!
//! ## Security Hardening & Robustness
//!
//! - **CWE-190 (Integer Overflow Protection)**: Mixing accumulators accumulate into `i64`
//!   before fixed-point downscaling and saturate fail-safe to $[-32768, 32767]$.
//!   Even 16 simultaneous full-scale square waves ($16 \times 32767 = 524272$) will never
//!   wrap around to negative values.
//! - **CWE-400 (Uncontrolled Resource Consumption)**: Fixed channel capacity (`MAX_MIXER_CHANNELS = 16`),
//!   bounded stack/const-generic ring buffer capacity (`MAX_RING_CAPACITY_SAMPLES = 65536`),
//!   and hard zero heap allocations on hot paths (`mix_channels_into`, `PcmRingBuffer::push/pop`).
//! - **CWE-125 (Out-of-bounds Read)**: Voice cursor and phase stepping perform checked
//!   bounds validation against source slices; out-of-bounds voices automatically deactivate
//!   or wrap according to their loop flag.

#![forbid(unsafe_code)]

pub mod capabilities;

/// Maximum simultaneous mixing channels / voices supported.
pub const MAX_MIXER_CHANNELS: usize = 16;

/// Maximum capacity in samples for the single-producer ring buffer (64k samples).
pub const MAX_RING_CAPACITY_SAMPLES: usize = 65_536;

/// Unity gain represented in Q16.16 fixed-point ($1.0 \times 65536 = 65536$).
pub const UNITY_GAIN_Q16: u32 = 65_536;

/// Minimum valid audio sample rate in Hz (1 Hz).
pub const MIN_SAMPLE_RATE_HZ: u32 = 1;

/// Maximum valid audio sample rate in Hz (192 kHz).
pub const MAX_SAMPLE_RATE_HZ: u32 = 192_000;

/// Maximum voice gain in Q16.16 ($4.0\times$ gain = 262,144).
pub const MAX_GAIN_Q16: u32 = 262_144;

/// Minimum pitch step in Q16.16 ($0.25\times$ speed = 16,384).
pub const MIN_PITCH_STEP_Q16: u32 = 16_384;

/// Maximum pitch step in Q16.16 ($4.0\times$ speed = 262,144).
pub const MAX_PITCH_STEP_Q16: u32 = 262_144;

/// Minimum signed 16-bit PCM integer ($-32,768$).
pub const PCM16_MIN: i16 = -32_768;

/// Maximum signed 16-bit PCM integer ($+32,767$).
pub const PCM16_MAX: i16 = 32_767;

/// Hard left panning in Q16.16 ($-1.0$).
pub const PAN_LEFT_Q16: i32 = -65_536;

/// Center panning in Q16.16 ($0.0$).
pub const PAN_CENTER_Q16: i32 = 0;

/// Hard right panning in Q16.16 ($+1.0$).
pub const PAN_RIGHT_Q16: i32 = 65_536;

/// Errors that can occur during real-time mixing operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerError {
    /// Audio sample rate was zero or exceeded [`MAX_SAMPLE_RATE_HZ`].
    InvalidSampleRate { rate: u32 },
    /// General invalid parameter or out-of-range value.
    InvalidParameter { reason: &'static str },
    /// All 16 mixer slots are active with equal or higher priority voices.
    ChannelLimitExceeded,
    /// Channel index out of bounds (`0..MAX_MIXER_CHANNELS`).
    ChannelIndexOutOfBounds { index: usize, max: usize },
    /// Ring buffer capacity exceeded without overwriting.
    RingBufferOverrun,
    /// Ring buffer empty during pop.
    RingBufferUnderrun,
    /// Output buffer is empty.
    EmptyBuffer,
    /// Output buffer is smaller than requested frame count.
    BufferTooSmall,
    /// Error interfacing with [`audio_device_sink`].
    SinkBufferError,
}

impl std::fmt::Display for MixerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSampleRate { rate } => write!(f, "invalid sample rate: {rate} Hz"),
            Self::InvalidParameter { reason } => write!(f, "invalid parameter: {reason}"),
            Self::ChannelLimitExceeded => {
                write!(f, "maximum channel limit of {MAX_MIXER_CHANNELS} exceeded")
            }
            Self::ChannelIndexOutOfBounds { index, max } => {
                write!(f, "channel index {index} out of bounds (max {max})")
            }
            Self::RingBufferOverrun => write!(f, "PCM ring buffer overrun: capacity exceeded"),
            Self::RingBufferUnderrun => write!(f, "PCM ring buffer underrun: no samples available"),
            Self::EmptyBuffer => write!(f, "audio buffer is empty"),
            Self::BufferTooSmall => write!(f, "destination buffer is too small"),
            Self::SinkBufferError => write!(f, "audio device sink buffer error"),
        }
    }
}

impl std::error::Error for MixerError {}

/// State of an individual voice mixing channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceSlot {
    /// Whether this slot is actively playing.
    pub active: bool,
    /// Application-assigned sound effect or voice identifier.
    pub voice_id: u32,
    /// Priority level (`0` = lowest, `255` = highest) for voice stealing.
    pub priority: u8,
    /// Current integer sample cursor in the source PCM buffer.
    pub cursor: usize,
    /// Voice gain in Q16.16 fixed point (`0..=MAX_GAIN_Q16`).
    pub gain_q16: u32,
    /// Stereo panning in Q16.16 fixed point (`PAN_LEFT_Q16..=PAN_RIGHT_Q16`).
    pub pan_q16: i32,
    /// Pitch/speed step in Q16.16 fixed point (`UNITY_GAIN_Q16` = 1.0x).
    pub pitch_step_q16: u32,
    /// High-resolution phase accumulator in Q48.16 fixed point.
    pub phase_q16: u64,
    /// Whether the voice loops indefinitely upon reaching buffer end.
    pub looping: bool,
}

impl Default for VoiceSlot {
    fn default() -> Self {
        Self::empty()
    }
}

impl VoiceSlot {
    /// Create an inactive, default voice slot.
    pub const fn empty() -> Self {
        Self {
            active: false,
            voice_id: 0,
            priority: 0,
            cursor: 0,
            gain_q16: UNITY_GAIN_Q16,
            pan_q16: PAN_CENTER_Q16,
            pitch_step_q16: UNITY_GAIN_Q16,
            phase_q16: 0,
            looping: false,
        }
    }

    /// Reset slot to inactive state.
    pub fn reset(&mut self) {
        *self = Self::empty();
    }
}

/// Real-time mixer state holding active channels and master controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RealtimeMixerState {
    /// Channel slots for simultaneous voices.
    pub slots: [VoiceSlot; MAX_MIXER_CHANNELS],
    /// Master volume gain in Q16.16 fixed-point (`0..=MAX_GAIN_Q16`).
    pub master_gain_q16: u32,
    /// Target audio sample rate in Hz.
    pub sample_rate_hz: u32,
    /// Running counter of saturated/clipped samples (telemetry).
    pub clipped_samples: u64,
    /// Total frames mixed by this engine.
    pub mixed_frames: u64,
}

impl RealtimeMixerState {
    /// Initialize a new mixer state with validated sample rate and unity master gain.
    pub fn new(sample_rate_hz: u32) -> Result<Self, MixerError> {
        if !(MIN_SAMPLE_RATE_HZ..=MAX_SAMPLE_RATE_HZ).contains(&sample_rate_hz) {
            return Err(MixerError::InvalidSampleRate {
                rate: sample_rate_hz,
            });
        }
        Ok(Self {
            slots: [VoiceSlot::empty(); MAX_MIXER_CHANNELS],
            master_gain_q16: UNITY_GAIN_Q16,
            sample_rate_hz,
            clipped_samples: 0,
            mixed_frames: 0,
        })
    }

    /// Alias for initializing with a standard sample rate.
    pub fn default_at_sample_rate(sample_rate_hz: u32) -> Result<Self, MixerError> {
        Self::new(sample_rate_hz)
    }

    /// Allocate a voice channel. If all 16 slots are active, steals the slot
    /// with the lowest priority strictly lower than `priority`.
    pub fn allocate_voice(
        &mut self,
        voice_id: u32,
        priority: u8,
        gain_q16: u32,
        looping: bool,
    ) -> Result<usize, MixerError> {
        // 1. Look for an inactive slot
        for (idx, slot) in self.slots.iter_mut().enumerate() {
            if !slot.active {
                *slot = VoiceSlot {
                    active: true,
                    voice_id,
                    priority,
                    cursor: 0,
                    gain_q16: gain_q16.min(MAX_GAIN_Q16),
                    pan_q16: PAN_CENTER_Q16,
                    pitch_step_q16: UNITY_GAIN_Q16,
                    phase_q16: 0,
                    looping,
                };
                return Ok(idx);
            }
        }

        // 2. Perform voice stealing: find active slot with strictly lower priority
        let mut lowest_priority = priority;
        let mut steal_idx = None;
        for (idx, slot) in self.slots.iter().enumerate() {
            if slot.priority < lowest_priority {
                lowest_priority = slot.priority;
                steal_idx = Some(idx);
            }
        }

        if let Some(idx) = steal_idx {
            self.slots[idx] = VoiceSlot {
                active: true,
                voice_id,
                priority,
                cursor: 0,
                gain_q16: gain_q16.min(MAX_GAIN_Q16),
                pan_q16: PAN_CENTER_Q16,
                pitch_step_q16: UNITY_GAIN_Q16,
                phase_q16: 0,
                looping,
            };
            return Ok(idx);
        }

        Err(MixerError::ChannelLimitExceeded)
    }

    /// Set voice pitch scaling for an active slot.
    pub fn set_voice_pitch(
        &mut self,
        slot_idx: usize,
        pitch_step_q16: u32,
    ) -> Result<(), MixerError> {
        if slot_idx >= MAX_MIXER_CHANNELS {
            return Err(MixerError::ChannelIndexOutOfBounds {
                index: slot_idx,
                max: MAX_MIXER_CHANNELS,
            });
        }
        let clamped = pitch_step_q16.clamp(MIN_PITCH_STEP_Q16, MAX_PITCH_STEP_Q16);
        self.slots[slot_idx].pitch_step_q16 = clamped;
        Ok(())
    }

    /// Set voice volume gain for an active slot.
    pub fn set_voice_gain(&mut self, slot_idx: usize, gain_q16: u32) -> Result<(), MixerError> {
        if slot_idx >= MAX_MIXER_CHANNELS {
            return Err(MixerError::ChannelIndexOutOfBounds {
                index: slot_idx,
                max: MAX_MIXER_CHANNELS,
            });
        }
        self.slots[slot_idx].gain_q16 = gain_q16.min(MAX_GAIN_Q16);
        Ok(())
    }

    /// Set voice panning for an active slot.
    pub fn set_voice_pan(&mut self, slot_idx: usize, pan_q16: i32) -> Result<(), MixerError> {
        if slot_idx >= MAX_MIXER_CHANNELS {
            return Err(MixerError::ChannelIndexOutOfBounds {
                index: slot_idx,
                max: MAX_MIXER_CHANNELS,
            });
        }
        self.slots[slot_idx].pan_q16 = pan_q16.clamp(PAN_LEFT_Q16, PAN_RIGHT_Q16);
        Ok(())
    }

    /// Deactivate all voices matching `voice_id`.
    pub fn stop_voice(&mut self, voice_id: u32) {
        for slot in &mut self.slots {
            if slot.active && slot.voice_id == voice_id {
                slot.active = false;
            }
        }
    }

    /// Deactivate a single channel slot by index.
    pub fn stop_slot(&mut self, slot_idx: usize) -> Result<(), MixerError> {
        if slot_idx >= MAX_MIXER_CHANNELS {
            return Err(MixerError::ChannelIndexOutOfBounds {
                index: slot_idx,
                max: MAX_MIXER_CHANNELS,
            });
        }
        self.slots[slot_idx].active = false;
        Ok(())
    }

    /// Count of currently active voices.
    pub fn active_slots(&self) -> usize {
        self.slots.iter().filter(|s| s.active).count()
    }

    /// Reset all slots and statistics.
    pub fn reset(&mut self) {
        for slot in &mut self.slots {
            slot.reset();
        }
        self.clipped_samples = 0;
        self.mixed_frames = 0;
    }
}

/// Fixed-capacity single-producer / single-consumer PCM audio ring buffer.
///
/// Implements bounded FIFO staging without dynamic memory reallocation (CWE-400).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PcmRingBuffer<const N: usize> {
    /// Write head index.
    head: usize,
    /// Read tail index.
    tail: usize,
    /// Number of valid samples currently enqueued.
    len: usize,
    /// Number of write attempts rejected due to capacity exhaustion.
    pub overruns: u64,
    /// Number of read attempts returned empty.
    pub underruns: u64,
    /// Fixed sample storage array.
    samples: [i16; N],
}

impl<const N: usize> Default for PcmRingBuffer<N> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<const N: usize> PcmRingBuffer<N> {
    /// Create an empty ring buffer of capacity `N`.
    pub const fn empty() -> Self {
        Self {
            head: 0,
            tail: 0,
            len: 0,
            overruns: 0,
            underruns: 0,
            samples: [0; N],
        }
    }

    /// Push a single sample into the ring buffer.
    ///
    /// If full, rejects the push, increments `overruns`, and returns [`MixerError::RingBufferOverrun`].
    pub fn push(&mut self, sample: i16) -> Result<(), MixerError> {
        if N == 0 || self.len >= N {
            self.overruns = self.overruns.saturating_add(1);
            return Err(MixerError::RingBufferOverrun);
        }
        self.samples[self.head] = sample;
        self.head = (self.head + 1) % N;
        self.len += 1;
        Ok(())
    }

    /// Pop a single sample from the ring buffer.
    ///
    /// If empty, increments `underruns` and returns `None`.
    pub fn pop(&mut self) -> Option<i16> {
        if N == 0 || self.len == 0 {
            self.underruns = self.underruns.saturating_add(1);
            return None;
        }
        let sample = self.samples[self.tail];
        self.tail = (self.tail + 1) % N;
        self.len -= 1;
        Some(sample)
    }

    /// Push a slice of samples into the ring buffer atomically.
    ///
    /// Rejects on overrun without partial mutation if `slice.len() > N - self.len`.
    pub fn push_slice(&mut self, slice: &[i16]) -> Result<usize, MixerError> {
        if slice.is_empty() {
            return Ok(0);
        }
        if N == 0 || slice.len() > N.saturating_sub(self.len) {
            self.overruns = self.overruns.saturating_add(1);
            return Err(MixerError::RingBufferOverrun);
        }
        for &s in slice {
            self.samples[self.head] = s;
            self.head = (self.head + 1) % N;
            self.len += 1;
        }
        Ok(slice.len())
    }

    /// Drain up to `out.len()` samples into the destination slice.
    ///
    /// Returns the number of samples successfully transferred.
    pub fn pop_slice(&mut self, out: &mut [i16]) -> usize {
        if N == 0 || out.is_empty() {
            if !out.is_empty() {
                self.underruns = self.underruns.saturating_add(1);
            }
            return 0;
        }
        let mut count = 0;
        for dest in out.iter_mut() {
            if self.len == 0 {
                self.underruns = self.underruns.saturating_add(1);
                break;
            }
            *dest = self.samples[self.tail];
            self.tail = (self.tail + 1) % N;
            self.len -= 1;
            count += 1;
        }
        count
    }

    /// Maximum number of samples this ring buffer can hold.
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Current number of samples available to read.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether the buffer contains zero samples.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Whether the buffer is filled to capacity `N`.
    pub const fn is_full(&self) -> bool {
        self.len >= N
    }

    /// Clear all stored samples and reset pointers.
    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.len = 0;
    }
}

/// Saturate an `i64` mixed accumulator into signed 16-bit PCM integer range `[-32768, 32767]`.
pub const fn clamp_sample(v: i64) -> i16 {
    if v > PCM16_MAX as i64 {
        PCM16_MAX
    } else if v < PCM16_MIN as i64 {
        PCM16_MIN
    } else {
        v as i16
    }
}

/// Scale a single sample by voice gain and master gain in Q16.16 fixed-point arithmetic.
pub const fn scale_voice_sample(sample: i16, gain_q16: u32, master_gain_q16: u32) -> i64 {
    let scaled_voice = ((sample as i64) * (gain_q16 as i64)) >> 16;
    (scaled_voice * (master_gain_q16 as i64)) >> 16
}

/// Validate that a sample rate is within the allowable range `[1, 192_000]`.
pub const fn validate_sample_rate(sample_rate_hz: u32) -> Result<bool, MixerError> {
    if sample_rate_hz >= MIN_SAMPLE_RATE_HZ && sample_rate_hz <= MAX_SAMPLE_RATE_HZ {
        Ok(true)
    } else {
        Err(MixerError::InvalidSampleRate {
            rate: sample_rate_hz,
        })
    }
}

/// Mix active channel voices into a mono destination buffer with zero heap allocations.
///
/// # Arguments
/// - `state`: Mutable mixer state tracking active voices, cursors, gains, and clipping.
/// - `sources`: Array of source audio slices corresponding to each voice slot.
/// - `out_mono`: Output buffer filled with mixed, saturated 16-bit PCM samples.
///
/// # Returns
/// - `Ok(frames_written)` on success.
/// - `Err(MixerError)` if sample rate is invalid or state invariant is violated.
pub fn mix_channels_into(
    state: &mut RealtimeMixerState,
    sources: &[&[i16]],
    out_mono: &mut [i16],
) -> Result<usize, MixerError> {
    if !(MIN_SAMPLE_RATE_HZ..=MAX_SAMPLE_RATE_HZ).contains(&state.sample_rate_hz) {
        return Err(MixerError::InvalidSampleRate {
            rate: state.sample_rate_hz,
        });
    }

    if out_mono.is_empty() {
        return Ok(0);
    }

    for out_sample in out_mono.iter_mut() {
        let mut acc: i64 = 0;

        for slot_idx in 0..MAX_MIXER_CHANNELS {
            let slot = &mut state.slots[slot_idx];
            if !slot.active {
                continue;
            }

            // CWE-125 guard: check if slot_idx is within sources slice
            if slot_idx >= sources.len() {
                slot.active = false;
                continue;
            }

            let source = sources[slot_idx];
            if source.is_empty() {
                slot.active = false;
                continue;
            }

            // Phase Q16 to integer index (guarded in u64 before usize cast for 32-bit safety)
            let phase_idx_u64 = slot.phase_q16 >> 16;

            if phase_idx_u64 >= source.len() as u64 {
                if slot.looping {
                    let total_phase = (source.len() as u64) << 16;
                    if total_phase > 0 {
                        slot.phase_q16 %= total_phase;
                    }
                    let wrapped_idx_u64 = slot.phase_q16 >> 16;
                    if wrapped_idx_u64 < source.len() as u64 {
                        let wrapped_idx = wrapped_idx_u64 as usize;
                        let raw = source[wrapped_idx] as i64;
                        let scaled = (raw * (slot.gain_q16 as i64)) >> 16;
                        acc += scaled;
                        slot.cursor = wrapped_idx;
                        slot.phase_q16 = slot.phase_q16.saturating_add(slot.pitch_step_q16 as u64);
                    } else {
                        slot.active = false;
                    }
                } else {
                    slot.active = false;
                }
            } else {
                let sample_idx = phase_idx_u64 as usize;
                let raw = source[sample_idx] as i64;
                let scaled = (raw * (slot.gain_q16 as i64)) >> 16;
                acc += scaled;
                slot.cursor = sample_idx;
                slot.phase_q16 = slot.phase_q16.saturating_add(slot.pitch_step_q16 as u64);
            }
        }

        // Apply master gain in Q16.16 using i128 to prevent intermediate overflow (CWE-190)
        let master_scaled = ((acc as i128) * (state.master_gain_q16 as i128)) >> 16;

        // Saturate to [-32768, 32767] (CWE-190 protection)
        if master_scaled > PCM16_MAX as i128 {
            state.clipped_samples = state.clipped_samples.saturating_add(1);
            *out_sample = PCM16_MAX;
        } else if master_scaled < PCM16_MIN as i128 {
            state.clipped_samples = state.clipped_samples.saturating_add(1);
            *out_sample = PCM16_MIN;
        } else {
            *out_sample = master_scaled as i16;
        }

        state.mixed_frames = state.mixed_frames.saturating_add(1);
    }

    Ok(out_mono.len())
}

/// Export a slice of mixed samples into an owned [`audio_device_sink::PcmPlaybackBuffer`].
pub fn export_to_sink_buffer(
    samples: &[i16],
    sample_rate_hz: u32,
) -> Result<audio_device_sink::PcmPlaybackBuffer, MixerError> {
    if !(MIN_SAMPLE_RATE_HZ..=MAX_SAMPLE_RATE_HZ).contains(&sample_rate_hz) {
        return Err(MixerError::InvalidSampleRate {
            rate: sample_rate_hz,
        });
    }
    let format = audio_device_sink::PcmFormat::new(sample_rate_hz, 1, 16)
        .map_err(|_| MixerError::SinkBufferError)?;
    audio_device_sink::PcmPlaybackBuffer::new(samples.to_vec(), format)
        .map_err(|_| MixerError::SinkBufferError)
}

/// Export a slice of mixed samples into a [`pcm_audio::PcmAudio`] container.
pub fn export_to_pcm_audio(
    samples: &[i16],
    sample_rate_hz: u32,
) -> Result<pcm_audio::PcmAudio, MixerError> {
    if !(MIN_SAMPLE_RATE_HZ..=MAX_SAMPLE_RATE_HZ).contains(&sample_rate_hz) {
        return Err(MixerError::InvalidSampleRate {
            rate: sample_rate_hz,
        });
    }
    let format = pcm_audio::PcmFormat::mono_16bit(sample_rate_hz as f64)
        .map_err(|_| MixerError::InvalidParameter {
            reason: "invalid pcm format",
        })?;
    pcm_audio::PcmAudio::from_i16(samples.to_vec(), format, 0.0).map_err(|_| {
        MixerError::InvalidParameter {
            reason: "invalid pcm buffer",
        }
    })
}
