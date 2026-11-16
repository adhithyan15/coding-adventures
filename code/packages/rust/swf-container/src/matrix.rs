//! SWF 2D affine transformation matrix.

/// A 2D affine transformation matrix used by SWF graphics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwfMatrix {
    /// Horizontal scale factor.
    pub scale_x: f64,
    /// Vertical scale factor.
    pub scale_y: f64,
    /// First rotate / skew factor.
    pub rotate_skew_0: f64,
    /// Second rotate / skew factor.
    pub rotate_skew_1: f64,
    /// Horizontal translation in twips.
    pub translate_x: i32,
    /// Vertical translation in twips.
    pub translate_y: i32,
}

impl SwfMatrix {
    /// Creates the identity transformation matrix.
    #[inline]
    pub const fn default_matrix() -> Self {
        Self {
            scale_x: 1.0,
            scale_y: 1.0,
            rotate_skew_0: 0.0,
            rotate_skew_1: 0.0,
            translate_x: 0,
            translate_y: 0,
        }
    }

    /// Scale components `(scale_x, scale_y)`.
    #[inline]
    pub fn matrix_scale(&self) -> (f64, f64) {
        (self.scale_x, self.scale_y)
    }

    /// Rotation / skew components `(rotate_skew_0, rotate_skew_1)`.
    #[inline]
    pub fn matrix_rotate(&self) -> (f64, f64) {
        (self.rotate_skew_0, self.rotate_skew_1)
    }

    /// Translation components in twips `(translate_x, translate_y)`.
    #[inline]
    pub fn matrix_translate_twips(&self) -> (i32, i32) {
        (self.translate_x, self.translate_y)
    }

    /// Translation components in pixels `(translate_x, translate_y)`.
    #[inline]
    pub fn matrix_translate_pixels(&self) -> (f64, f64) {
        (
            self.translate_x as f64 / 20.0,
            self.translate_y as f64 / 20.0,
        )
    }
}
