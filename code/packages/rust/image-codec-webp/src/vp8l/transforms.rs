//! VP8L optional transforms.
//!
//! VP8L supports four optional, reversible transforms that the encoder may apply
//! before entropy coding. Each transform is signalled by a `has_transform=1` bit
//! in the bitstream. When multiple transforms are stacked they are applied in
//! reverse order during decoding (the last transform written is the first inverse
//! applied on decode).
//!
//! ## The four transforms
//!
//! 1. **Subtract-green** — For each pixel subtract the green channel from R and B
//!    before coding.  This exploits the correlation between colour channels in
//!    natural images.  On decode: `R += G`, `B += G` for every pixel.
//!
//! 2. **Color** — A spatially-varying linear colour-space transform stored as a
//!    downsampled "colour transform element" image.  Each 2×2 block (or larger
//!    power-of-two block) has its own transform coefficients.
//!
//! 3. **Predictor** — A spatially-varying predictor removes local spatial
//!    redundancy.  The residual (actual − predicted) is stored.  Thirteen
//!    predictor modes are defined in the spec.
//!
//! 4. **Color-index** — For images with ≤ 256 distinct colours, store a palette
//!    and replace each pixel with its palette index.  The pixel stream then
//!    contains palette indices rather than full ARGB values.
//!
//! ## Current status
//!
//! **Currently active transforms:**
//! - **Subtract-green** — wired in as of v0.3.2; the encoder applies it before
//!   LZ77 and writes `has_transform=1, type=2` in the bitstream header.
//!
//! **Not yet implemented:**
//! - Predictor transform — biggest gains on natural images.
//! - Color transform — per-block linear colour correction.
//! - Color-index transform — palette coding for synthetic images.

use pixel_container::PixelContainer;

// ---------------------------------------------------------------------------
// Transform kind enum
// ---------------------------------------------------------------------------

/// The four VP8L transform types, identified by their 2-bit code in the
/// bitstream.
///
/// When `has_transform = 1`, the next 2 bits in the stream specify which
/// transform follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformKind {
    /// Code `0b00` — Spatially-varying predictor transform.
    Predictor = 0,
    /// Code `0b01` — Spatially-varying linear colour transform.
    Color = 1,
    /// Code `0b10` — Subtract the green channel from R and B.
    SubtractGreen = 2,
    /// Code `0b11` — Palette / colour-index transform.
    ColorIndex = 3,
}

// ---------------------------------------------------------------------------
// Forward transforms (applied during encoding, not yet used)
// ---------------------------------------------------------------------------

/// Apply the subtract-green transform in-place.
///
/// For each pixel: `R' = (R - G) & 0xFF`, `B' = (B - G) & 0xFF`.
/// The green channel is left unchanged.
///
/// This is one of the simplest transforms and exploits the fact that in most
/// images R and B are correlated with G.  After the transform the residuals
/// R' and B' tend to cluster near 0, which compresses better.
pub fn apply_subtract_green(pixels: &mut PixelContainer) {
    for chunk in pixels.data.chunks_exact_mut(4) {
        let g = chunk[1];
        chunk[0] = chunk[0].wrapping_sub(g); // R -= G
        chunk[2] = chunk[2].wrapping_sub(g); // B -= G
    }
}

// ---------------------------------------------------------------------------
// Inverse transforms (applied during decoding after pixel reconstruction)
// ---------------------------------------------------------------------------

/// Invert the subtract-green transform in-place.
///
/// For each pixel: `R = (R' + G) & 0xFF`, `B = (B' + G) & 0xFF`.
/// This is the inverse of [`apply_subtract_green`].
pub fn inverse_subtract_green(pixels: &mut PixelContainer) {
    for chunk in pixels.data.chunks_exact_mut(4) {
        let g = chunk[1];
        chunk[0] = chunk[0].wrapping_add(g); // R += G
        chunk[2] = chunk[2].wrapping_add(g); // B += G
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtract_green_round_trip() {
        let mut original = PixelContainer::new(2, 2);
        original.set_pixel(0, 0, 100, 50, 200, 255);
        original.set_pixel(1, 0, 10, 20, 30, 255);
        original.set_pixel(0, 1, 255, 128, 64, 200);
        original.set_pixel(1, 1, 0, 0, 0, 0);

        let snapshot = original.clone();

        apply_subtract_green(&mut original);
        inverse_subtract_green(&mut original);

        assert_eq!(original.data, snapshot.data, "subtract-green round-trip failed");
    }

    #[test]
    fn transform_kind_codes() {
        assert_eq!(TransformKind::Predictor as u8, 0);
        assert_eq!(TransformKind::Color as u8, 1);
        assert_eq!(TransformKind::SubtractGreen as u8, 2);
        assert_eq!(TransformKind::ColorIndex as u8, 3);
    }
}
