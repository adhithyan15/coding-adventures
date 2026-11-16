//! Interpolated vector morph shape (`MorphShape`).

use swf_container::SwfRect;
use swf_shape_to_paint::{ColorRgba, ParsedShape, ShapeEdge, SwfFillStyle, SwfLineStyle};

use crate::error::DisplayListError;

/// Linear interpolation for signed 32-bit twips.
fn lerp_i32(a: i32, b: i32, t: f64) -> i32 {
    let diff = (b as f64) - (a as f64);
    (a as f64 + diff * t).round() as i32
}

/// Linear interpolation for unsigned 8-bit channel.
fn lerp_u8(a: u8, b: u8, t: f64) -> u8 {
    let diff = (b as f64) - (a as f64);
    (a as f64 + diff * t).round().clamp(0.0, 255.0) as u8
}

/// Linear interpolation for unsigned 16-bit values.
fn lerp_u16(a: u16, b: u16, t: f64) -> u16 {
    let diff = (b as f64) - (a as f64);
    (a as f64 + diff * t).round().clamp(0.0, 65535.0) as u16
}

/// Linear interpolation between two RGBA colors.
fn lerp_color(c0: ColorRgba, c1: ColorRgba, t: f64) -> ColorRgba {
    ColorRgba::rgba(
        lerp_u8(c0.r, c1.r, t),
        lerp_u8(c0.g, c1.g, t),
        lerp_u8(c0.b, c1.b, t),
        lerp_u8(c0.a, c1.a, t),
    )
}

/// Tweened vector shape interpolated between start and end shape geometries.
#[derive(Debug, Clone, PartialEq)]
pub struct MorphShape {
    /// Initial baseline shape (ratio = 0).
    pub start_shape: ParsedShape,
    /// Target transformed shape (ratio = 65535).
    pub end_shape: ParsedShape,
    /// Current interpolation ratio (0..=65535).
    pub ratio: u16,
}

impl MorphShape {
    /// Creates a new morph shape between start and end geometries.
    pub fn new(start_shape: ParsedShape, end_shape: ParsedShape) -> Self {
        Self {
            start_shape,
            end_shape,
            ratio: 0,
        }
    }

    /// Returns the current interpolation ratio (`0..=65535`).
    #[inline]
    pub fn morph_current_ratio(&self) -> u16 {
        self.ratio
    }

    /// Sets the interpolation ratio (`0..=65535`).
    #[inline]
    pub fn morph_set_ratio_val(&mut self, ratio: u16) {
        self.ratio = ratio;
    }

    /// Returns a reference to the start shape.
    #[inline]
    pub fn morph_start_shape_ref(&self) -> &ParsedShape {
        &self.start_shape
    }

    /// Returns a reference to the end shape.
    #[inline]
    pub fn morph_end_shape_ref(&self) -> &ParsedShape {
        &self.end_shape
    }

    /// Evaluates the morph shape at the specified ratio, generating an interpolated `ParsedShape`.
    pub fn morph_evaluate_at_ratio(&self, ratio: u16) -> Result<ParsedShape, DisplayListError> {
        let t = (ratio as f64) / 65535.0;

        let bounds = SwfRect::create_rect(
            lerp_i32(self.start_shape.bounds.xmin, self.end_shape.bounds.xmin, t),
            lerp_i32(self.start_shape.bounds.xmax, self.end_shape.bounds.xmax, t),
            lerp_i32(self.start_shape.bounds.ymin, self.end_shape.bounds.ymin, t),
            lerp_i32(self.start_shape.bounds.ymax, self.end_shape.bounds.ymax, t),
        );

        let edge_bounds = match (&self.start_shape.edge_bounds, &self.end_shape.edge_bounds) {
            (Some(b0), Some(b1)) => Some(SwfRect::create_rect(
                lerp_i32(b0.xmin, b1.xmin, t),
                lerp_i32(b0.xmax, b1.xmax, t),
                lerp_i32(b0.ymin, b1.ymin, t),
                lerp_i32(b0.ymax, b1.ymax, t),
            )),
            (Some(b), None) | (None, Some(b)) => Some(*b),
            (None, None) => None,
        };

        // Interpolate fill styles
        let fill_styles: Vec<SwfFillStyle> = self
            .start_shape
            .fill_styles
            .iter()
            .zip(self.end_shape.fill_styles.iter())
            .map(|(s0, s1)| match (s0, s1) {
                (SwfFillStyle::Solid(c0), SwfFillStyle::Solid(c1)) => {
                    SwfFillStyle::Solid(lerp_color(*c0, *c1, t))
                }
                _ => {
                    if t < 0.5 {
                        s0.clone()
                    } else {
                        s1.clone()
                    }
                }
            })
            .collect();

        // Interpolate line styles
        let line_styles: Vec<SwfLineStyle> = self
            .start_shape
            .line_styles
            .iter()
            .zip(self.end_shape.line_styles.iter())
            .map(|(l0, l1)| SwfLineStyle {
                width_twips: lerp_u16(l0.width_twips, l1.width_twips, t),
                color: lerp_color(l0.color, l1.color, t),
                cap_style: if t < 0.5 { l0.cap_style } else { l1.cap_style },
                join_style: if t < 0.5 {
                    l0.join_style
                } else {
                    l1.join_style
                },
                miter_limit: l0.miter_limit * (1.0 - t) + l1.miter_limit * t,
                no_close: l0.no_close,
                fill_style: None,
            })
            .collect();

        // Interpolate edges
        let edges: Vec<ShapeEdge> = self
            .start_shape
            .edges
            .iter()
            .zip(self.end_shape.edges.iter())
            .map(|(e0, e1)| {
                let start = (
                    lerp_i32(e0.start.0, e1.start.0, t),
                    lerp_i32(e0.start.1, e1.start.1, t),
                );
                let end = (
                    lerp_i32(e0.end.0, e1.end.0, t),
                    lerp_i32(e0.end.1, e1.end.1, t),
                );
                let control = match (e0.control, e1.control) {
                    (Some(c0), Some(c1)) => {
                        Some((lerp_i32(c0.0, c1.0, t), lerp_i32(c0.1, c1.1, t)))
                    }
                    (Some(c0), None) => {
                        let c1 = ((e1.start.0 + e1.end.0) / 2, (e1.start.1 + e1.end.1) / 2);
                        Some((lerp_i32(c0.0, c1.0, t), lerp_i32(c0.1, c1.1, t)))
                    }
                    (None, Some(c1)) => {
                        let c0 = ((e0.start.0 + e0.end.0) / 2, (e0.start.1 + e0.end.1) / 2);
                        Some((lerp_i32(c0.0, c1.0, t), lerp_i32(c0.1, c1.1, t)))
                    }
                    (None, None) => None,
                };

                ShapeEdge {
                    start,
                    end,
                    control,
                    fill_style0: if t < 0.5 {
                        e0.fill_style0
                    } else {
                        e1.fill_style0
                    },
                    fill_style1: if t < 0.5 {
                        e0.fill_style1
                    } else {
                        e1.fill_style1
                    },
                    line_style: if t < 0.5 {
                        e0.line_style
                    } else {
                        e1.line_style
                    },
                }
            })
            .collect();

        Ok(ParsedShape {
            tag_code: self.start_shape.tag_code,
            shape_id: self.start_shape.shape_id,
            bounds,
            edge_bounds,
            uses_non_zero_winding: self.start_shape.uses_non_zero_winding,
            fill_styles,
            line_styles,
            edges,
        })
    }

    /// Evaluates the morph shape at its current `ratio`.
    #[inline]
    pub fn morph_evaluate_current(&self) -> Result<ParsedShape, DisplayListError> {
        self.morph_evaluate_at_ratio(self.ratio)
    }
}
