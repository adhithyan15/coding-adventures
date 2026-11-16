//! # Flash Display List
//!
//! Interactive Flash display-object hierarchy, timeline FSM, and retained-mode
//! `PaintScene` generation and diffing (`SWF05`).
//!
//! Provides:
//! - Interactive display object hierarchy: [`Stage`], [`MovieClip`], [`SimpleButton`],
//!   [`TextField`], and [`MorphShape`].
//! - Depth-ordered child container and timeline state machine ([`PlayState::Play`],
//!   [`PlayState::Stop`], [`MovieClip::clip_goto_and_play`], [`MovieClip::clip_goto_and_stop`]).
//! - SWF depth op parsers and applicators for `PlaceObject`, `PlaceObject2`,
//!   `PlaceObject3`, `RemoveObject`, and `RemoveObject2`.
//! - Stencil mask clipping (`clip_depth: Some(D)`) lowering to `PaintInstruction::Clip` (`P2D00`).
//! - Retained-mode [`PaintScene`] generation and diffing via
//!   `paint-palette-blend::diff_paint_scene_into` (`P2D10`).
//! - Strict security limits: `MAX_HIERARCHY_DEPTH = 64`, depth validation,
//!   and bounding box enforcement.

#![forbid(unsafe_code)]

pub mod button;
pub mod capabilities;
pub mod display_object;
pub mod error;
pub mod events;
pub mod lowering;
pub mod morph;
pub mod movie_clip;
pub mod stage;
pub mod tag_parse;
pub mod text;

pub use button::{ButtonState, SimpleButton};
pub use capabilities::PureCapabilities;
pub use display_object::{DisplayObject, DisplaySlot};
pub use error::DisplayListError;
pub use events::{ButtonEvent, DisplayListEvent};
pub use lowering::{
    lower_clip_to_instructions, lower_movie_clip_to_scene, lower_slot_to_instructions,
    swf_matrix_to_affine,
};
pub use morph::MorphShape;
pub use movie_clip::{
    FrameTarget, MovieClip, PlayState, MAX_DEPTH, MAX_HIERARCHY_DEPTH, MIN_DEPTH,
};
pub use stage::Stage;
pub use tag_parse::{
    parse_display_list_tag, parse_place_object, parse_place_object2, parse_place_object3,
    parse_remove_object, parse_remove_object2, DisplayListTag, PlaceObjectRecord,
    RemoveObjectRecord,
};
pub use text::TextField;

// Re-exports from sibling rendering crates for convenient consumption
pub use paint_instructions::{PaintBase, PaintInstruction, PaintRect, PaintScene};
pub use paint_palette_blend::{diff_paint_scene_into, PatchKind, PatchOp, PatchSummary};

/// Maximum ratio for tweened morph shapes (65535).
pub const MAX_MORPH_RATIO: usize = 65535;

/// Validates that an active hierarchy nesting depth does not exceed safety bounds (CWE-400 / CWE-674).
#[inline]
pub fn validate_hierarchy_depth(depth: usize) -> Result<bool, DisplayListError> {
    if depth <= MAX_HIERARCHY_DEPTH {
        Ok(true)
    } else {
        Err(DisplayListError::HierarchyDepthExceeded)
    }
}

/// Validates that a display list depth layer is within supported bounds.
#[inline]
pub fn validate_display_depth(depth: i32) -> Result<bool, DisplayListError> {
    if (MIN_DEPTH..=MAX_DEPTH).contains(&depth) {
        Ok(true)
    } else {
        Err(DisplayListError::InvalidDepth)
    }
}

/// Validates that a morph shape ratio is within valid range (0..=65535).
#[inline]
pub fn validate_morph_ratio(ratio: usize) -> Result<bool, DisplayListError> {
    if ratio <= MAX_MORPH_RATIO {
        Ok(true)
    } else {
        Err(DisplayListError::InvalidRatio)
    }
}
