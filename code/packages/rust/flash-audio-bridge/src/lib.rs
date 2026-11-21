//! # Flash Audio Bridge (`SWF06`)
//!
//! Deterministic, zero-allocation SWF audio tag parser, decoder, mixer, and
//! timeline clock synchronization bridge for the Coding Adventures Flash Player suite.
//!
//! ```text
//!  swf-container (SWF01)
//!         │
//!         ├─► DefineSound (Tag 14) ──► PCM8/PCM16 / Flash ADPCM (2..5-bit) / MP3 frames
//!         │                                    │
//!         ├─► StartSound (15) / StartSound2 (89) ─► Event Sound Bank + 16 Active Voices
//!         │                                    │
//!         └─► SoundStreamHead(2) + SoundStreamBlock(19) ─► PcmRingBuffer Stream FIFO
//!                                              │
//!                                  game-clock (GAME10) Sync
//!                                              │
//!                                              ▼
//!                     render_flash_audio_into(&mut state, &mut out_i16)
//!                                              │
//!                        ┌─────────────────────┴─────────────────────┐
//!                        ▼                                           ▼
//!            audio-mixer-realtime (MUS08)                pcm-audio / audio-device-sink
//!              PcmRingBuffer<N> Bridge                         (MUS01 / MUS02)
//! ```
//!
//! ## Security & Resource Hardening
//!
//! - **`CAP001` / `CAP002`**: `#![forbid(unsafe_code)]`, `"capabilities": []`.
//! - **CWE-20 (Improper Input Validation)**: Validates sound IDs (`!= 0`), format codes (`0..=3`),
//!   sample rate codes (`0..=3`), ADPCM bit-widths (`2..=5`), step-table indices (`0..=88`),
//!   loop counts (`1..=MAX_LOOP_COUNT`), envelope levels (`<= 32768`), monotonic `pos44`,
//!   `in_point < out_point`, and MPEG Layer III frame headers.
//! - **CWE-125 / CWE-787 (Out-of-Bounds Read/Write)**: Bounded slice parsing and explicit
//!   destination slice capacity checks before decoding.
//! - **CWE-190 / CWE-369 (Overflow & Divide-by-Zero)**: Checked integer arithmetic on
//!   sample counts and byte offsets, `i64`/`i128` mixing accumulators with saturation clamping,
//!   and non-zero divisor guarantees for sample/frame rates and envelope spans.
//! - **CWE-400 (Uncontrolled Resource Consumption)**: `MAX_SOUND_SAMPLES = 8_388_608`,
//!   `MAX_EVENT_SOUNDS = 32`, `MAX_ACTIVE_VOICES = 16`, `STREAM_FIFO_CAPACITY = 8192`,
//!   and zero heap allocations on all decode/mix hot paths.

#![forbid(unsafe_code)]

pub mod capabilities;
pub mod decoder;
pub mod error;
pub mod mixer;
pub mod tags;

pub use capabilities::PureCapabilities;
pub use decoder::{
    decode_adpcm_into, decode_define_sound_into, decode_pcm16_le_into, decode_pcm16_ne_into,
    decode_pcm8_into, extract_define_sound_mp3_info, mix_stereo_to_mono_into,
    parse_mp3_frame_header, Mp3ChannelMode, Mp3Frame, Mp3FrameHeader, Mp3FrameIter, MpegVersion,
    ADPCM_SAMPLES_PER_BLOCK, MAX_MP3_FRAMES,
};
pub use error::AudioBridgeError;
pub use mixer::{
    bridge_render_to_ring_buffer, export_flash_audio_to_pcm_audio,
    export_flash_audio_to_sink_buffer, render_flash_audio_into, FlashAudioState,
    DEFAULT_FRAME_RATE_HZ, DEFAULT_OUTPUT_SAMPLE_RATE_HZ, MAX_ACTIVE_VOICES, MAX_EVENT_SOUNDS,
    STREAM_FIFO_CAPACITY,
};
pub use tags::{
    parse_define_sound, parse_sound_info, parse_sound_stream_block, parse_sound_stream_head,
    parse_start_sound, parse_start_sound2, parse_swf_audio_tag, DefineSoundTag, SoundEnvelopePoint,
    SoundInfo, SoundStreamBlockTag, SoundStreamHead, StartSound2Tag, StartSoundTag,
    SwfAudioChannels, SwfAudioTag, SwfSampleWidth, SwfSoundFormat, SwfSoundProperties,
    SwfSoundRate, MAX_CLASS_NAME_BYTES, MAX_ENVELOPE_LEVEL, MAX_ENVELOPE_POINTS, MAX_LOOP_COUNT,
    MAX_SOUND_SAMPLES, MAX_TAG_PAYLOAD_BYTES, SWF_TAG_START_SOUND2,
};

// Re-exports from sibling substrates for downstream convenience
pub use audio_mixer_realtime::{PcmRingBuffer, MAX_GAIN_Q16, UNITY_GAIN_Q16};

/// Validates that an SWF 2-bit sound sample rate code is within `0..=3` (`5512..=44100 Hz`).
#[inline]
pub const fn validate_sound_rate_code(code: usize) -> Result<bool, AudioBridgeError> {
    if code <= 3 {
        Ok(true)
    } else {
        let clamped = if code > u8::MAX as usize {
            u8::MAX
        } else {
            code as u8
        };
        Err(AudioBridgeError::InvalidSoundRateCode { code: clamped })
    }
}

/// Validates that a Flash ADPCM sample bit width is within `2..=5`.
#[inline]
pub const fn validate_adpcm_bit_width(bits: usize) -> Result<bool, AudioBridgeError> {
    if bits >= 2 && bits <= 5 {
        Ok(true)
    } else {
        Err(AudioBridgeError::InvalidAdpcmBitWidth { bits })
    }
}

/// Validates that a declared SWF sound sample count does not exceed `MAX_SOUND_SAMPLES`.
#[inline]
pub const fn validate_sound_sample_count(samples: usize) -> Result<bool, AudioBridgeError> {
    if samples <= MAX_SOUND_SAMPLES as usize {
        Ok(true)
    } else {
        let clamped = if samples > u32::MAX as usize {
            u32::MAX
        } else {
            samples as u32
        };
        Err(AudioBridgeError::SampleCountExceeded {
            count: clamped,
            max: MAX_SOUND_SAMPLES,
        })
    }
}

/// Validates that a `SOUNDENVELOPE` point count does not exceed `MAX_ENVELOPE_POINTS`.
#[inline]
pub const fn validate_envelope_point_count(count: usize) -> Result<bool, AudioBridgeError> {
    if count <= MAX_ENVELOPE_POINTS {
        Ok(true)
    } else {
        Err(AudioBridgeError::InvalidEnvelopeCount {
            count,
            max: MAX_ENVELOPE_POINTS,
        })
    }
}
