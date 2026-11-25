//! # Flash Player Host (`SWF07`)
//!
//! Integrated Flash player application shell uniting:
//! - Container & anti-bomb decompression (`swf-container` `SWF01`)
//! - Vector shape lowering (`swf-shape-to-paint` `SWF02`)
//! - Display list hierarchy & retained-mode diffing (`flash-display-list` `SWF05` / `paint-palette-blend` `P2D10`)
//! - Event & streaming audio synchronization (`flash-audio-bridge` `SWF06`)
//! - AVM1/AVM2 bytecode & ActionScript execution (`avm-bytecode-lifter` `SWF04` / `actionscript-compiler` `SWF03`)
//! - Fixed-timestep simulation pacing (`game-clock` `GAME10`)
//! - Window & canvas presentation (`window-core` `UI11` / `paint-vm-runtime` `P2D01`)
//! - Deterministic headless CI runner with canonical SHA-256 frame digests
//!
//! ## Mandatory Security Policy (`SWF00` §7.5)
//!
//! - **`CAP001` / `CAP002`**: `#![forbid(unsafe_code)]`, `"capabilities": []`.
//! - **CWE-78 (`fscommand("exec")`)**: Permanently disabled as a no-op with diagnostic audit recording.
//! - **CWE-94 (`getURL` / `navigateToURL`)**: Strictly sanitized to allow only `http:`, `https:`, or relative URIs,
//!   blocking `javascript:`, `file:`, `data:`, `vbscript:`, and control-character smuggling.
//! - **CWE-400 (DoS Bounds)**: Frame scripts execute under `max_steps <= 2_000_000`, display hierarchy depth `<= 64`,
//!   dictionary characters `<= 4_096`, and URI length `<= 2_048`.

#![forbid(unsafe_code)]

pub mod capabilities;
pub mod digest;
pub mod error;
pub mod headless;
pub mod input;
pub mod player;
pub mod security;

pub use capabilities::PureCapabilities;
pub use digest::{
    compute_pcm_digest_hex, compute_scene_digest, compute_scene_digest_hex, SceneDigest,
    MAX_SCENE_DIGEST_DEPTH,
};
pub use error::{
    PlayerHostError, MAX_AUDIT_EVENTS, MAX_FRAME_SCRIPTS, MAX_HEADLESS_FRAMES, MAX_HOST_CHARACTERS,
    MAX_HOST_FRAME_RATE_HZ, MAX_HOST_VM_STEPS, MAX_STAGE_DIM_PX, MAX_TIMELINE_FRAMES, MAX_URI_LEN,
};
pub use headless::{
    run_headless_swf, verify_headless_digests, HeadlessFrameDigest, HeadlessRunConfig,
    HeadlessRunReport,
};
pub use input::{
    dispatch_window_event_to_stage, key_to_flash_code, HostInputDispatchResult, KeyboardState,
    MAX_TRACKED_KEYS,
};
pub use player::{
    FlashHostSession, FlashPlayer, FlashPlayerConfig, NavigationRequest, PlayerTickSummary,
    PlayerZeroAllocTick, MAX_CONTROL_TAGS,
};
pub use security::{
    evaluate_fscommand, is_fscommand_exec, sanitize_navigation_uri, trim_whitespace_and_control,
    FsCommandOutcome, SecurityAuditEvent, SecurityAuditKind, SecurityAuditLog,
};

// Re-exports from integrated substrates
pub use actionscript_compiler::AsDialect;
pub use flash_display_list::{
    diff_paint_scene_into, DisplayListEvent, PaintInstruction, PaintScene, PatchKind, PatchOp,
    PatchSummary, Stage,
};
pub use paint_vm_runtime::{
    PaintAcceleration, PaintBackendCapabilities, PaintBackendDescriptor, PaintBackendFamily,
    PaintBackendPreference, PaintBackendRegistry, PaintBackendTier, PaintPlatformSupport,
    PaintRenderError, PaintRenderOptions, PaintRenderer, SupportLevel,
};
pub use window_core::{
    ElementState, Key, LogicalSize, ModifiersState, NamedKey, PointerButton, Window,
    WindowAttributes, WindowBackend, WindowBuilder, WindowError, WindowEvent, WindowId,
};

/// Validates that a frame script VM execution budget does not exceed `MAX_HOST_VM_STEPS` (`2_000_000`).
#[inline]
pub const fn validate_vm_step_budget(steps: usize) -> Result<bool, PlayerHostError> {
    if steps <= MAX_HOST_VM_STEPS {
        Ok(true)
    } else {
        Err(PlayerHostError::InvalidStepBudget)
    }
}

/// Validates that a playback frame rate is within `1..=MAX_HOST_FRAME_RATE_HZ` (`1..=1000` Hz).
#[inline]
pub const fn validate_frame_rate_hz(fps: usize) -> Result<bool, PlayerHostError> {
    if fps >= 1 && fps <= MAX_HOST_FRAME_RATE_HZ {
        Ok(true)
    } else {
        Err(PlayerHostError::InvalidFrameRate)
    }
}

/// Validates that a stage pixel dimension is within `1..=MAX_STAGE_DIM_PX` (`1..=16384` px).
#[inline]
pub const fn validate_stage_dimension_px(px: usize) -> Result<bool, PlayerHostError> {
    if px >= 1 && px <= MAX_STAGE_DIM_PX {
        Ok(true)
    } else {
        Err(PlayerHostError::InvalidStageDimension)
    }
}

/// Validates that a navigation URI byte length is within `1..=MAX_URI_LEN` (`1..=2048`).
#[inline]
pub const fn validate_uri_length(len: usize) -> Result<bool, PlayerHostError> {
    if len >= 1 && len <= MAX_URI_LEN {
        Ok(true)
    } else {
        Err(PlayerHostError::InvalidUri)
    }
}
