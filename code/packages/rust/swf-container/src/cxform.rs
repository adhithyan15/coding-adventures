//! SWF Color Transform (CXFORM / CXFORMWITHALPHA).

/// 4-byte RGBA color array alias to avoid raw array types in public method signatures.
pub type RgbaArray = [u8; 4];

/// A color transformation record that modifies RGBA channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwfColorTransform {
    /// Red multiplier (256 represents 1.0).
    pub r_mult: i32,
    /// Green multiplier (256 represents 1.0).
    pub g_mult: i32,
    /// Blue multiplier (256 represents 1.0).
    pub b_mult: i32,
    /// Alpha multiplier (256 represents 1.0).
    pub a_mult: i32,
    /// Red additive offset (-255..=255).
    pub r_add: i32,
    /// Green additive offset (-255..=255).
    pub g_add: i32,
    /// Blue additive offset (-255..=255).
    pub b_add: i32,
    /// Alpha additive offset (-255..=255).
    pub a_add: i32,
}

impl SwfColorTransform {
    /// Creates the identity color transformation.
    #[inline]
    pub const fn default_cxform() -> Self {
        Self {
            r_mult: 256,
            g_mult: 256,
            b_mult: 256,
            a_mult: 256,
            r_add: 0,
            g_add: 0,
            b_add: 0,
            a_add: 0,
        }
    }

    /// Applies this color transformation to an RGBA pixel tuple.
    pub fn cxform_apply_rgba(&self, rgba: RgbaArray) -> RgbaArray {
        let r = (((rgba[0] as i32 * self.r_mult) >> 8) + self.r_add).clamp(0, 255) as u8;
        let g = (((rgba[1] as i32 * self.g_mult) >> 8) + self.g_add).clamp(0, 255) as u8;
        let b = (((rgba[2] as i32 * self.b_mult) >> 8) + self.b_add).clamp(0, 255) as u8;
        let a = (((rgba[3] as i32 * self.a_mult) >> 8) + self.a_add).clamp(0, 255) as u8;
        [r, g, b, a]
    }
}
