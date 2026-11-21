//! Error definitions and safety limits for `flash-player-host` (`SWF07`).

#![forbid(unsafe_code)]

use std::fmt;

use actionscript_compiler::AsCompilerError;
use avm_bytecode_lifter::LifterError;
use flash_audio_bridge::AudioBridgeError;
use flash_display_list::DisplayListError;
use game_clock::ClockError;
use paint_palette_blend::PaletteBlendError;
use paint_vm_runtime::PaintRenderError;
use swf_container::SwfError;
use swf_shape_to_paint::ShapeError;
use window_core::WindowError;

/// Maximum VM execution steps permitted per frame script invocation (`SWF00` §7.2).
pub const MAX_HOST_VM_STEPS: usize = 2_000_000;

/// Maximum supported stage frame rate in Hz (CWE-400 guard).
pub const MAX_HOST_FRAME_RATE_HZ: usize = 1_000;

/// Maximum supported stage dimension in pixels along either axis (CWE-400 guard).
pub const MAX_STAGE_DIM_PX: usize = 16_384;

/// Maximum length in bytes of a navigation URI accepted by `getURL`/`navigateToURL` (CWE-20/400).
pub const MAX_URI_LEN: usize = 2_048;

/// Maximum number of dictionary characters (shapes + sprites) retained in a movie (CWE-400).
pub const MAX_HOST_CHARACTERS: usize = 4_096;

/// Maximum number of timeline frames pre-indexed in a single SWF movie (CWE-400).
pub const MAX_TIMELINE_FRAMES: usize = 65_536;

/// Maximum number of frame scripts allowed across a movie (CWE-400).
pub const MAX_FRAME_SCRIPTS: usize = 4_096;

/// Maximum number of diagnostic security audit events retained in memory (CWE-400).
pub const MAX_AUDIT_EVENTS: usize = 1_024;

/// Maximum number of frames executed in a single headless CI runner invocation (CWE-400).
pub const MAX_HEADLESS_FRAMES: u32 = 65_536;

/// Unified error type for `flash-player-host` (`SWF07`).
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerHostError {
    /// SWF container parsing or anti-bomb decompression error (`SWF01`).
    SwfContainer(SwfError),
    /// Vector shape parsing or lowering error (`SWF02`).
    Shape(ShapeError),
    /// Display list hierarchy or timeline error (`SWF05`).
    DisplayList(DisplayListError),
    /// Audio tag decoding or mixing error (`SWF06`).
    Audio(AudioBridgeError),
    /// Bytecode lifting or VM execution error (`SWF04`).
    Lifter(LifterError),
    /// ActionScript source compilation error (`SWF03`).
    ActionScript(AsCompilerError),
    /// Fixed-timestep accumulator error (`GAME10`).
    Clock(ClockError),
    /// Windowing backend or attribute validation error (`UI11`).
    Window(WindowError),
    /// PaintVM backend selection or rendering failure (`P2D01`).
    PaintRuntime(String),
    /// Retained-mode scene diffing failure (`P2D10`).
    Blend(PaletteBlendError),
    /// Frame script VM step budget exceeds `MAX_HOST_VM_STEPS` (CWE-400).
    InvalidStepBudget,
    /// Configured frame rate is outside `1..=MAX_HOST_FRAME_RATE_HZ` (CWE-20/369).
    InvalidFrameRate,
    /// Stage width or height is outside `1..=MAX_STAGE_DIM_PX` (CWE-20/400).
    InvalidStageDimension,
    /// Navigation URI is empty, exceeds `MAX_URI_LEN`, or contains control characters (CWE-20).
    InvalidUri,
    /// Navigation URI uses a disallowed scheme such as `javascript:`, `file:`, `data:`, or `vbscript:` (CWE-94/79).
    BlockedUriScheme,
    /// Attempted invocation of permanently disabled `fscommand("exec")` (CWE-78).
    BlockedFsCommand,
    /// Dictionary character count exceeds `MAX_HOST_CHARACTERS` (CWE-400).
    CharacterLimitExceeded,
    /// Frame script count exceeds `MAX_FRAME_SCRIPTS` (CWE-400).
    FrameScriptLimitExceeded,
    /// Frame script execution exhausted `max_steps <= 2_000_000` (CWE-400).
    ScriptTimeout,
    /// Malformed SWF tag payload encountered during host loading (CWE-20/125).
    MalformedTag(&'static str),
    /// No movie has been loaded or headless digest assertion failed.
    DigestMismatch {
        frame: u32,
        expected: String,
        actual: String,
    },
    /// Headless runner frame budget exceeds `MAX_HEADLESS_FRAMES` (CWE-400).
    HeadlessFrameLimitExceeded,
    /// Non-finite pointer or stage coordinate rejected (CWE-20).
    NonFiniteCoordinate,
}

impl fmt::Display for PlayerHostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SwfContainer(e) => write!(f, "swf container error: {e}"),
            Self::Shape(e) => write!(f, "shape error: {e}"),
            Self::DisplayList(e) => write!(f, "display list error: {e}"),
            Self::Audio(e) => write!(f, "audio bridge error: {e}"),
            Self::Lifter(e) => write!(f, "avm bytecode lifter error: {e}"),
            Self::ActionScript(e) => write!(f, "actionscript compiler error: {e}"),
            Self::Clock(e) => write!(f, "game clock error: {e}"),
            Self::Window(e) => write!(f, "window error: {e}"),
            Self::PaintRuntime(msg) => write!(f, "paint runtime error: {msg}"),
            Self::Blend(e) => write!(f, "scene diff error: {e}"),
            Self::InvalidStepBudget => {
                write!(
                    f,
                    "vm step budget exceeds MAX_HOST_VM_STEPS ({MAX_HOST_VM_STEPS})"
                )
            }
            Self::InvalidFrameRate => {
                write!(f, "frame rate must be in 1..={MAX_HOST_FRAME_RATE_HZ} Hz")
            }
            Self::InvalidStageDimension => {
                write!(f, "stage dimension must be in 1..={MAX_STAGE_DIM_PX} px")
            }
            Self::InvalidUri => write!(f, "invalid or oversized navigation URI"),
            Self::BlockedUriScheme => write!(f, "navigation URI scheme blocked by CWE-94 policy"),
            Self::BlockedFsCommand => write!(f, "fscommand(\"exec\") is permanently disabled"),
            Self::CharacterLimitExceeded => write!(f, "movie character dictionary limit exceeded"),
            Self::FrameScriptLimitExceeded => write!(f, "movie frame script limit exceeded"),
            Self::ScriptTimeout => write!(f, "frame script exceeded VM step budget"),
            Self::MalformedTag(reason) => write!(f, "malformed SWF tag: {reason}"),
            Self::DigestMismatch {
                frame,
                expected,
                actual,
            } => write!(
                f,
                "headless digest mismatch on frame {frame}: expected {expected}, got {actual}"
            ),
            Self::HeadlessFrameLimitExceeded => {
                write!(f, "headless frame count exceeds {MAX_HEADLESS_FRAMES}")
            }
            Self::NonFiniteCoordinate => write!(f, "non-finite coordinate rejected"),
        }
    }
}

impl std::error::Error for PlayerHostError {}

impl From<SwfError> for PlayerHostError {
    fn from(e: SwfError) -> Self {
        Self::SwfContainer(e)
    }
}

impl From<ShapeError> for PlayerHostError {
    fn from(e: ShapeError) -> Self {
        Self::Shape(e)
    }
}

impl From<DisplayListError> for PlayerHostError {
    fn from(e: DisplayListError) -> Self {
        Self::DisplayList(e)
    }
}

impl From<AudioBridgeError> for PlayerHostError {
    fn from(e: AudioBridgeError) -> Self {
        Self::Audio(e)
    }
}

impl From<LifterError> for PlayerHostError {
    fn from(e: LifterError) -> Self {
        match e {
            LifterError::ScriptTimeout => Self::ScriptTimeout,
            other => Self::Lifter(other),
        }
    }
}

impl From<AsCompilerError> for PlayerHostError {
    fn from(e: AsCompilerError) -> Self {
        Self::ActionScript(e)
    }
}

impl From<ClockError> for PlayerHostError {
    fn from(e: ClockError) -> Self {
        Self::Clock(e)
    }
}

impl From<WindowError> for PlayerHostError {
    fn from(e: WindowError) -> Self {
        Self::Window(e)
    }
}

impl From<PaintRenderError> for PlayerHostError {
    fn from(e: PaintRenderError) -> Self {
        Self::PaintRuntime(format!("{e:?}"))
    }
}

impl From<PaletteBlendError> for PlayerHostError {
    fn from(e: PaletteBlendError) -> Self {
        Self::Blend(e)
    }
}
