//! Error types for SWF audio tag parsing, decoding, and playback synchronization (`SWF06`).

use std::fmt;

/// Errors returned by `flash-audio-bridge` parsers, decoders, and mixer operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioBridgeError {
    /// Sound ID was zero (reserved) or otherwise invalid (CWE-20).
    InvalidSoundId,
    /// SWF sound encoding format code is unsupported or unrecognized (CWE-20).
    UnsupportedSoundFormat { code: u8 },
    /// SWF sound sample rate code was out of `0..=3` range (CWE-20).
    InvalidSoundRateCode { code: u8 },
    /// ADPCM bit width was outside the valid `2..=5` range (CWE-20).
    InvalidAdpcmBitWidth { bits: usize },
    /// ADPCM initial step-table index exceeded `88` (CWE-125).
    InvalidAdpcmStepIndex { index: usize },
    /// Sound sample count exceeded `MAX_SOUND_SAMPLES` (CWE-400).
    SampleCountExceeded { count: u32, max: u32 },
    /// Loop count was zero or exceeded `MAX_LOOP_COUNT` (CWE-20 / CWE-400).
    InvalidLoopCount { count: u16 },
    /// Envelope point count exceeded `MAX_ENVELOPE_POINTS` (CWE-400).
    InvalidEnvelopeCount { count: usize, max: usize },
    /// Envelope point had out-of-range volume level or non-monotonic position (CWE-20).
    InvalidEnvelopePoint { reason: &'static str },
    /// `in_point` was greater than or equal to `out_point` (CWE-20).
    InvalidInOutPoints { in_point: u32, out_point: u32 },
    /// `StartSound2` sound class name was empty, non-UTF-8, unterminated, or too long.
    InvalidClassName,
    /// Unexpected end of input while reading tag headers or audio frames (CWE-125).
    UnexpectedEof,
    /// Tag payload violated length or structural invariants (CWE-20 / CWE-190).
    MalformedTagPayload { reason: &'static str },
    /// Caller-provided output slice was smaller than decoded sample count (CWE-787).
    BufferTooSmall { required: usize, provided: usize },
    /// MP3 frame header failed sync, version, layer, bitrate, or size checks (CWE-20 / CWE-369).
    InvalidMp3Frame { reason: &'static str },
    /// MP3 stream frame count exceeded `MAX_MP3_FRAMES` (CWE-400).
    Mp3FrameLimitExceeded { max: usize },
    /// Sound ID or class name was not registered in the event sound bank.
    SoundNotFound { sound_id: u16 },
    /// Event sound bank reached `MAX_EVENT_SOUNDS` capacity (CWE-400).
    SoundBankFull { max: usize },
    /// All active mixer voice slots (`MAX_ACTIVE_VOICES`) are occupied (CWE-400).
    VoiceLimitExceeded,
    /// `SoundStreamBlock` encountered without a matching `SoundStreamHead` configuration.
    StreamFormatMismatch,
    /// Streaming sound FIFO `PcmRingBuffer` reached capacity (CWE-400).
    RingBufferOverrun,
    /// Underlying `audio-mixer-realtime` state or rate validation failed.
    MixerFailure,
    /// Underlying `game-clock` state or frame rate validation failed.
    ClockFailure,
    /// Exporting to `pcm-audio` or `audio-device-sink` failed.
    PcmExportFailure,
}

impl fmt::Display for AudioBridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSoundId => write!(f, "invalid SWF sound ID (must be non-zero)"),
            Self::UnsupportedSoundFormat { code } => {
                write!(f, "unsupported SWF sound format code: {code}")
            }
            Self::InvalidSoundRateCode { code } => {
                write!(f, "invalid SWF sound rate code: {code} (expected 0..=3)")
            }
            Self::InvalidAdpcmBitWidth { bits } => {
                write!(f, "invalid Flash ADPCM bit width: {bits} (expected 2..=5)")
            }
            Self::InvalidAdpcmStepIndex { index } => {
                write!(f, "invalid ADPCM step index: {index} (expected 0..=88)")
            }
            Self::SampleCountExceeded { count, max } => {
                write!(f, "sound sample count {count} exceeds maximum {max}")
            }
            Self::InvalidLoopCount { count } => {
                write!(f, "invalid sound loop count: {count}")
            }
            Self::InvalidEnvelopeCount { count, max } => {
                write!(f, "envelope point count {count} exceeds maximum {max}")
            }
            Self::InvalidEnvelopePoint { reason } => {
                write!(f, "invalid sound envelope point: {reason}")
            }
            Self::InvalidInOutPoints {
                in_point,
                out_point,
            } => {
                write!(
                    f,
                    "invalid sound in/out points: in_point ({in_point}) >= out_point ({out_point})"
                )
            }
            Self::InvalidClassName => write!(f, "invalid StartSound2 sound class name"),
            Self::UnexpectedEof => write!(f, "unexpected end of input reading SWF audio tag"),
            Self::MalformedTagPayload { reason } => {
                write!(f, "malformed SWF audio tag payload: {reason}")
            }
            Self::BufferTooSmall { required, provided } => {
                write!(
                    f,
                    "output buffer too small: required {required} samples, provided {provided}"
                )
            }
            Self::InvalidMp3Frame { reason } => write!(f, "invalid MP3 frame: {reason}"),
            Self::Mp3FrameLimitExceeded { max } => {
                write!(f, "MP3 frame count exceeded maximum {max}")
            }
            Self::SoundNotFound { sound_id } => {
                write!(f, "sound ID {sound_id} not registered in event sound bank")
            }
            Self::SoundBankFull { max } => {
                write!(f, "event sound bank full (max {max} sounds)")
            }
            Self::VoiceLimitExceeded => write!(f, "active Flash audio voice limit exceeded"),
            Self::StreamFormatMismatch => {
                write!(f, "sound stream block does not match stream head state")
            }
            Self::RingBufferOverrun => write!(f, "streaming sound FIFO ring buffer overrun"),
            Self::MixerFailure => write!(f, "real-time audio mixer failure"),
            Self::ClockFailure => write!(f, "game clock synchronization failure"),
            Self::PcmExportFailure => write!(f, "failed to export PCM audio container"),
        }
    }
}

impl std::error::Error for AudioBridgeError {}
