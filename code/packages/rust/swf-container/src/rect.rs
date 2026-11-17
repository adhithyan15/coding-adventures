//! SWF RECT structure in twips.

/// A bounding box in twips (1 pixel = 20 twips).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwfRect {
    /// Minimum X coordinate in twips.
    pub xmin: i32,
    /// Maximum X coordinate in twips.
    pub xmax: i32,
    /// Minimum Y coordinate in twips.
    pub ymin: i32,
    /// Maximum Y coordinate in twips.
    pub ymax: i32,
}

impl SwfRect {
    /// Creates a new `SwfRect` with given twip coordinates.
    #[inline]
    pub const fn create_rect(xmin: i32, xmax: i32, ymin: i32, ymax: i32) -> Self {
        Self {
            xmin,
            xmax,
            ymin,
            ymax,
        }
    }

    /// Width of the rectangle in twips.
    #[inline]
    pub fn rect_width_twips(&self) -> i32 {
        self.xmax.saturating_sub(self.xmin)
    }

    /// Height of the rectangle in twips.
    #[inline]
    pub fn rect_height_twips(&self) -> i32 {
        self.ymax.saturating_sub(self.ymin)
    }

    /// Width of the rectangle in pixels (1 pixel = 20 twips).
    #[inline]
    pub fn rect_width_pixels(&self) -> f64 {
        self.rect_width_twips() as f64 / 20.0
    }

    /// Height of the rectangle in pixels (1 pixel = 20 twips).
    #[inline]
    pub fn rect_height_pixels(&self) -> f64 {
        self.rect_height_twips() as f64 / 20.0
    }
}
