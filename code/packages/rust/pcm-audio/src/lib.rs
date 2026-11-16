//! # Pulse Code Modulation Audio Container & WAV Codec (MUS01 Stage 5)
//!
//! PCM is the fundamental digital audio representation in the note-to-sound signal chain:
//!
//! ```text
//! typed note -> frequency -> oscillator -> sampler -> PCM audio -> DAC -> speaker
//! ```
//!
//! While an oscillator evaluates a continuous mathematical function $x(t)$ and a sampler
//! snapshots values onto a discrete time grid $x[n] \in [-1.0, 1.0]$, PCM represents
//! the first discrete integer domain: mapping continuous floating amplitudes to finite
//! signed 16-bit integers $[ -32768, 32767 ]$.
//!
//! ## Clamping and Quantization
//!
//! Standard normalized audio signals are bounded within $[-1.0, 1.0]$. In fixed-point digital
//! representations, any amplitude beyond full scale must be saturated (clamped) to avoid
//! catastrophic wrap-around distortion (such as $+32768$ overflowing to $-32768$).
//!
//! - Positive half-scale: $[0.0, 1.0] \implies \text{round}(x \cdot 32767)$
//! - Negative half-scale: $[-1.0, 0.0) \implies \text{round}(x \cdot 32768)$
//! - Clamping: $x > 1.0 \implies 32767$, $x < -1.0 \implies -32768$
//!
//! This asymmetric scaling respects standard signed two's-complement representation where
//! $-32768$ ($0\text{x}8000$) is representable, but $+32768$ is not.
//!
//! ## Secure WAV Serialization and Deserialization
//!
//! This crate implements canonical RIFF/WAVE parsing and serialization in pure safe Rust.
//! It includes explicit security hardening against common audio parser vulnerabilities:
//!
//! - **CWE-400 (Allocation Bombs)**: Never allocates memory based on unchecked chunk sizes
//!   declared in untrusted file headers. Verifies that declared payload lengths fit strictly
//!   within the available buffer before pre-allocation.
//! - **CWE-190 (Integer Overflow)**: Checked arithmetic on byte rates, block alignments, and
//!   slice offsets.
//! - **CWE-20 (Improper Input Validation)**: Rejects malformed headers, inconsistent byte rates,
//!   unsupported compression schemes, and non-finite floating values.

#![forbid(unsafe_code)]

pub mod format;
pub mod pcm;
pub mod wav;

// Re-exports
pub use format::{
    PcmError, PcmFormat, DEFAULT_BIT_DEPTH, DEFAULT_CHANNEL_COUNT, DEFAULT_FULL_SCALE_VOLTAGE,
    DEFAULT_SAMPLE_RATE_HZ, INTEGER_TOLERANCE, MAX_CHANNELS, MAX_DURATION_SECONDS, MAX_SAMPLES,
    MAX_SAMPLE_RATE_HZ, MAX_WAV_BYTES, PCM16_MAX, PCM16_MIN,
};

pub use pcm::{
    encode_sample_buffer, float_to_pcm16, pcm16_to_f32, pcm16_to_f64, safe_round_sample,
    safe_round_sample_f32, samples_to_pcm_buffer, PcmAudio,
};

pub use wav::{from_wav_bytes, to_wav_bytes};

/// Type alias for Python/TypeScript parity.
pub type PCMFormat = PcmFormat;

/// Type alias for Python/TypeScript parity.
pub type PCMBuffer = PcmAudio;

/// Type alias for Python/TypeScript parity.
pub type PcmBuffer = PcmAudio;
