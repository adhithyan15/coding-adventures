//! Error types, bounded limits, and Warrant validation helpers for `swf-conformance-corpus` (`SWF08`).

#![forbid(unsafe_code)]

use std::fmt;

use avm_bytecode_lifter::LifterError;
use flash_audio_bridge::AudioBridgeError;
use flash_display_list::DisplayListError;
use flash_player_host::PlayerHostError;
use swf_container::SwfError;
use swf_shape_to_paint::ShapeError;

/// Maximum synthetic SWF payload size permitted by the corpus builder (`16 MiB`, CWE-400).
pub const MAX_CORPUS_SWF_BYTES: usize = 16_777_216;

/// Maximum timeline frame count permitted in a single corpus entry (`1024`, CWE-400).
pub const MAX_CORPUS_FRAMES: usize = 1_024;

/// Maximum bit width permitted by `SwfBitWriter` (`<= 31`, SWF 5-bit `UB[5]` bound).
pub const MAX_BIT_WIDTH: u8 = 31;

/// Maximum number of tags per synthetic SWF movie or sprite (CWE-400).
pub const MAX_BUILDER_TAGS: usize = 16_384;

/// Unified error type for `swf-conformance-corpus` (`SWF08`).
#[derive(Debug, Clone, PartialEq)]
pub enum CorpusError {
    /// Integrated Flash player host error (`SWF07`).
    PlayerHost(PlayerHostError),
    /// SWF container parsing or anti-bomb error (`SWF01`).
    SwfContainer(SwfError),
    /// Vector shape parsing error (`SWF02`).
    Shape(ShapeError),
    /// Display list error (`SWF05`).
    DisplayList(DisplayListError),
    /// Audio bridge error (`SWF06`).
    Audio(AudioBridgeError),
    /// Bytecode lifter / VM error (`SWF04`).
    Lifter(LifterError),
    /// DEFLATE compression error while building `CWS` payload.
    Compression(String),
    /// Bit width exceeds `MAX_BIT_WIDTH` (`31`) (CWE-190 / CWE-20).
    BitWidthExceeded,
    /// SWF binary payload is empty or exceeds `MAX_CORPUS_SWF_BYTES` (CWE-400).
    SwfSizeExceeded,
    /// Frame count is zero or exceeds `MAX_CORPUS_FRAMES` (CWE-400).
    FrameCountExceeded,
    /// Digest hex string is not a 64-character lowercase/uppercase hexadecimal string.
    InvalidDigestFormat,
    /// Produced frame or audio SHA-256 digest does not match pinned corpus expectation.
    DigestMismatch {
        case_slug: &'static str,
        frame: u32,
        expected: String,
        actual: String,
    },
    /// Corpus case failed an invariant check (trace output, patch summary, button event, etc.).
    UnexpectedCaseFailure(String),
    /// Checked integer arithmetic or `try_from` conversion overflowed (CWE-190).
    IntegerOverflow,
}

impl fmt::Display for CorpusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlayerHost(e) => write!(f, "player host error: {e}"),
            Self::SwfContainer(e) => write!(f, "swf container error: {e}"),
            Self::Shape(e) => write!(f, "shape error: {e}"),
            Self::DisplayList(e) => write!(f, "display list error: {e}"),
            Self::Audio(e) => write!(f, "audio error: {e}"),
            Self::Lifter(e) => write!(f, "lifter error: {e}"),
            Self::Compression(msg) => write!(f, "compression error: {msg}"),
            Self::BitWidthExceeded => {
                write!(f, "bit width exceeds MAX_BIT_WIDTH ({MAX_BIT_WIDTH})")
            }
            Self::SwfSizeExceeded => {
                write!(f, "SWF byte length must be in 1..={MAX_CORPUS_SWF_BYTES}")
            }
            Self::FrameCountExceeded => {
                write!(f, "corpus frame count must be in 1..={MAX_CORPUS_FRAMES}")
            }
            Self::InvalidDigestFormat => {
                write!(f, "SHA-256 hex digest must be 64 hexadecimal ASCII bytes")
            }
            Self::DigestMismatch {
                case_slug,
                frame,
                expected,
                actual,
            } => write!(
                f,
                "digest mismatch in {case_slug} frame {frame}: expected {expected}, got {actual}"
            ),
            Self::UnexpectedCaseFailure(msg) => write!(f, "corpus case invariant failed: {msg}"),
            Self::IntegerOverflow => write!(f, "checked integer conversion/arithmetic overflow"),
        }
    }
}

impl std::error::Error for CorpusError {}

impl From<PlayerHostError> for CorpusError {
    fn from(e: PlayerHostError) -> Self {
        Self::PlayerHost(e)
    }
}

impl From<SwfError> for CorpusError {
    fn from(e: SwfError) -> Self {
        Self::SwfContainer(e)
    }
}

impl From<ShapeError> for CorpusError {
    fn from(e: ShapeError) -> Self {
        Self::Shape(e)
    }
}

impl From<DisplayListError> for CorpusError {
    fn from(e: DisplayListError) -> Self {
        Self::DisplayList(e)
    }
}

impl From<AudioBridgeError> for CorpusError {
    fn from(e: AudioBridgeError) -> Self {
        Self::Audio(e)
    }
}

impl From<LifterError> for CorpusError {
    fn from(e: LifterError) -> Self {
        Self::Lifter(e)
    }
}
