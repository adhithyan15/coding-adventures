//! Edge and style record structures extracted from SWF bitstreams.

#![deny(unsafe_code)]

use crate::styles::{SwfFillStyle, SwfLineStyle};
use crate::twips::twips_point_to_pixels;

/// A single straight or curved edge segment in an SWF shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeEdge {
    /// Start coordinate in twips.
    pub start: (i32, i32),
    /// End coordinate in twips.
    pub end: (i32, i32),
    /// Optional quadratic Bézier off-curve control point in twips.
    pub control: Option<(i32, i32)>,
    /// Fill style index on the left side of this edge (0 = none).
    pub fill_style0: u32,
    /// Fill style index on the right side of this edge (0 = none).
    pub fill_style1: u32,
    /// Stroke line style index for this edge (0 = none).
    pub line_style: u32,
}

impl ShapeEdge {
    /// Returns true if this edge represents a quadratic Bézier curve.
    #[inline]
    pub const fn is_curved_edge(&self) -> bool {
        self.control.is_some()
    }

    /// Reverses the edge direction, preserving control geometry and swapping dual fills.
    #[inline]
    pub fn reverse_edge_direction(&self) -> Self {
        Self {
            start: self.end,
            end: self.start,
            control: self.control,
            fill_style0: self.fill_style1,
            fill_style1: self.fill_style0,
            line_style: self.line_style,
        }
    }

    /// Converts start point from twips to pixels.
    #[inline]
    pub fn edge_start_pixels(&self) -> (f64, f64) {
        twips_point_to_pixels(self.start.0, self.start.1)
    }

    /// Converts end point from twips to pixels.
    #[inline]
    pub fn edge_end_pixels(&self) -> (f64, f64) {
        twips_point_to_pixels(self.end.0, self.end.1)
    }

    /// Converts optional control point from twips to pixels.
    #[inline]
    pub fn edge_control_pixels(&self) -> Option<(f64, f64)> {
        self.control.map(|(cx, cy)| twips_point_to_pixels(cx, cy))
    }
}

/// Raw parsed shape record from the SWF bitstream.
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeRecord {
    /// Marks the termination of shape records.
    End,
    /// Updates current pen location and/or active fill and line styles.
    StyleChange {
        move_to: Option<(i32, i32)>,
        fill_style0: Option<u32>,
        fill_style1: Option<u32>,
        line_style: Option<u32>,
        new_fill_styles: Option<Vec<SwfFillStyle>>,
        new_line_styles: Option<Vec<SwfLineStyle>>,
    },
    /// A straight line displacement in twips.
    StraightEdge { delta_x: i32, delta_y: i32 },
    /// A quadratic Bézier curve with control point and anchor deltas.
    CurvedEdge {
        control_delta_x: i32,
        control_delta_y: i32,
        anchor_delta_x: i32,
        anchor_delta_y: i32,
    },
}
