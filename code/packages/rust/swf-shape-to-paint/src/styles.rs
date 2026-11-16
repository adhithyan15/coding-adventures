//! Fill and line styles for SWF vector shapes.

#![deny(unsafe_code)]

use paint_instructions::{StrokeCap, StrokeJoin};
use swf_container::SwfMatrix;

/// 32-bit RGBA color representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorRgba {
    /// Red channel (0..=255).
    pub r: u8,
    /// Green channel (0..=255).
    pub g: u8,
    /// Blue channel (0..=255).
    pub b: u8,
    /// Alpha channel (0..=255, 255 = fully opaque).
    pub a: u8,
}

impl ColorRgba {
    /// Creates an opaque RGB color (`alpha = 255`).
    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Creates an RGBA color.
    #[inline]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Formats the color as a standard CSS color string (`#rrggbb` or `rgba(...)`).
    pub fn color_to_css_string(&self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            let alpha_unit = (self.a as f64) / 255.0;
            format!(
                "rgba({}, {}, {}, {:.3})",
                self.r, self.g, self.b, alpha_unit
            )
        }
    }

    /// Returns color channels as a 4-tuple `(r, g, b, a)`.
    #[inline]
    pub fn color_to_components(&self) -> (u8, u8, u8, u8) {
        (self.r, self.g, self.b, self.a)
    }
}

/// Gradient geometry kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwfGradientKind {
    Linear,
    Radial,
    FocalRadial,
}

/// Gradient spread mode for handling coordinates outside standard range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SwfSpreadMode {
    #[default]
    Pad,
    Reflect,
    Repeat,
}

/// Color interpolation mode for gradients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SwfInterpolationMode {
    #[default]
    NormalRgb,
    LinearRgb,
}

/// A single color stop in a gradient.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwfGradientRecord {
    /// Position along gradient axis (0..=255).
    pub ratio: u8,
    /// Color at this position.
    pub color: ColorRgba,
}

impl SwfGradientRecord {
    /// Converts the 0..255 ratio to a 0.0..1.0 unit offset.
    #[inline]
    pub fn grad_stop_offset(&self) -> f64 {
        self.ratio as f64 / 255.0
    }
}

/// Complete SWF gradient definition.
#[derive(Debug, Clone, PartialEq)]
pub struct SwfGradient {
    /// Linear, radial, or focal radial.
    pub kind: SwfGradientKind,
    /// Edge wrapping / spread behavior.
    pub spread: SwfSpreadMode,
    /// Color interpolation space.
    pub interpolation: SwfInterpolationMode,
    /// Ordered color stops (1..=15).
    pub stops: Vec<SwfGradientRecord>,
    /// Transformation matrix mapping standard gradient space to shape space.
    pub matrix: SwfMatrix,
    /// Focal point offset for focal gradients (-1.0..=1.0).
    pub focal_point: f64,
}

/// Bitmap fill reference in an SWF shape.
#[derive(Debug, Clone, PartialEq)]
pub struct SwfBitmapFill {
    /// ID of the referenced bitmap character.
    pub bitmap_id: u16,
    /// Texture mapping affine matrix.
    pub matrix: SwfMatrix,
    /// Whether the bitmap tiles or clamps at edges.
    pub is_repeating: bool,
    /// Whether bilinear smoothing is applied.
    pub is_smoothed: bool,
}

/// Fill style definition for shape interiors.
#[derive(Debug, Clone, PartialEq)]
pub enum SwfFillStyle {
    /// Solid flat color fill.
    Solid(ColorRgba),
    /// Linear color gradient fill.
    LinearGradient(SwfGradient),
    /// Radial color gradient fill.
    RadialGradient(SwfGradient),
    /// Focal radial color gradient fill.
    FocalGradient(SwfGradient),
    /// Repeating smoothed bitmap texture.
    RepeatingBitmap(SwfBitmapFill),
    /// Clipped smoothed bitmap texture.
    ClippedBitmap(SwfBitmapFill),
    /// Repeating non-smoothed bitmap texture.
    NonSmoothedRepeatingBitmap(SwfBitmapFill),
    /// Clipped non-smoothed bitmap texture.
    NonSmoothedClippedBitmap(SwfBitmapFill),
}

/// Stroke cap style at endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SwfCapStyle {
    #[default]
    Round,
    Butt,
    Square,
}

impl SwfCapStyle {
    /// Converts to the paint-instructions `StrokeCap` equivalent.
    pub fn cap_to_paint_cap(&self) -> StrokeCap {
        match self {
            Self::Round => StrokeCap::Round,
            Self::Butt => StrokeCap::Butt,
            Self::Square => StrokeCap::Square,
        }
    }
}

/// Stroke corner join style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SwfJoinStyle {
    #[default]
    Round,
    Bevel,
    Miter,
}

impl SwfJoinStyle {
    /// Converts to the paint-instructions `StrokeJoin` equivalent.
    pub fn join_to_paint_join(&self) -> StrokeJoin {
        match self {
            Self::Round => StrokeJoin::Round,
            Self::Bevel => StrokeJoin::Bevel,
            Self::Miter => StrokeJoin::Miter,
        }
    }
}

/// Line style definition for shape strokes.
#[derive(Debug, Clone, PartialEq)]
pub struct SwfLineStyle {
    /// Stroke width in twips.
    pub width_twips: u16,
    /// Stroke color.
    pub color: ColorRgba,
    /// Endpoint cap style.
    pub cap_style: SwfCapStyle,
    /// Corner join style.
    pub join_style: SwfJoinStyle,
    /// Miter limit factor (typically 1.0..=10.0).
    pub miter_limit: f64,
    /// Suppresses stroke closure.
    pub no_close: bool,
    /// Optional complex fill for stroked paths (DefineShape4).
    pub fill_style: Option<SwfFillStyle>,
}

impl SwfLineStyle {
    /// Stroke width converted to logical screen pixels.
    #[inline]
    pub fn line_width_pixels(&self) -> f64 {
        self.width_twips as f64 / 20.0
    }
}
