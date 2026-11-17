//! # SWF Shape to Paint
//!
//! Deterministic, zero-dependency dual-fill Bézier shape converter lowering SWF
//! `DefineShape` / `DefineShape2` / `DefineShape3` / `DefineShape4` records to
//! `P2D00` `PaintInstructions`.
//!
//! Provides:
//! - Safe parsing of SWF shape definitions and styles with bounded allocations.
//! - Coordinate conversion between TWIPS (1/20 px) and floating-point pixels.
//! - Dual-fill edge loop reconstruction (`FillStyle0` / `FillStyle1`) resolving
//!   closed contours, complex topology, and interior island cutouts.
//! - Lowering to `paint-instructions::PaintScene`, emitting filled paths, gradients,
//!   bitmap fills, and styled strokes.
//! - Quadratic Bézier curve integration via `bezier2d` (`G2D02`).

#![deny(unsafe_code)]

pub mod capabilities;
pub mod error;
pub mod lowering;
pub mod parser;
pub mod reconstruction;
pub mod records;
pub mod styles;
pub mod twips;

pub use capabilities::PureCapabilities;
pub use error::ShapeError;
pub use lowering::{
    evaluate_quad_edge, flatten_quad_edge, lower_shape_to_paint_scene, quad_edge_bounding_box,
};
pub use parser::{parse_define_shape, ParsedShape, MAX_EDGES, MAX_SHAPE_RECORDS, MAX_STYLES};
pub use reconstruction::{
    reconstruct_all_fills, reconstruct_all_strokes, reconstruct_fill_contours,
    reconstruct_stroke_runs, DirectedContour, FillGroup, StrokeRun,
};
pub use records::{ShapeEdge, ShapeRecord};
pub use styles::{
    ColorRgba, SwfBitmapFill, SwfCapStyle, SwfFillStyle, SwfGradient, SwfGradientKind,
    SwfGradientRecord, SwfInterpolationMode, SwfJoinStyle, SwfLineStyle, SwfSpreadMode,
};
pub use twips::{
    checked_twips_add, checked_twips_point_add, pixels_to_twips_i32, twips_point_to_pixels,
    twips_to_pixels_f64, MAX_SAFE_TWIPS, MIN_SAFE_TWIPS, PIXELS_PER_TWIP, TWIPS_PER_PIXEL,
};

/// Validates that an SWF tag code corresponds to a shape tag (2, 22, 32, or 83).
#[inline]
pub fn validate_shape_tag(tag_code: usize) -> Result<bool, ShapeError> {
    parser::validate_shape_tag(tag_code)
}

/// Validates that a twips coordinate is within safe bounds.
#[inline]
pub fn validate_twips(t: i32) -> Result<bool, ShapeError> {
    twips::validate_twips(t)
}

/// Validates that a style count does not exceed security limits (CWE-400).
#[inline]
pub fn validate_style_count(count: usize) -> Result<bool, ShapeError> {
    parser::validate_style_count(count)
}
