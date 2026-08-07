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
//!    redundancy.  The residual (actual − predicted) is stored.  Fourteen
//!    predictor modes are defined in the spec (modes 0-13).
//!
//! 4. **Color-index** — For images with ≤ 256 distinct colours, store a palette
//!    and replace each pixel with its palette index.  The pixel stream then
//!    contains palette indices rather than full ARGB values.
//!
//! ## Current status
//!
//! **Currently active transforms:**
//! - **Subtract-green** — wired in as of v0.3.2.
//! - **Predictor** — wired in as of v0.3.3; encoder uses mode 1 (left prediction)
//!   for all blocks with a 16-pixel block size (block_bits=4).  All 14 modes are
//!   fully implemented for decoding.
//!
//! **Not yet implemented:**
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
// Subtract-green — forward and inverse
// ---------------------------------------------------------------------------

/// Apply the subtract-green transform in-place.
///
/// For each pixel: `R' = (R - G) & 0xFF`, `B' = (B - G) & 0xFF`.
/// The green channel is left unchanged.
pub fn apply_subtract_green(pixels: &mut PixelContainer) {
    for chunk in pixels.data.chunks_exact_mut(4) {
        let g = chunk[1];
        chunk[0] = chunk[0].wrapping_sub(g); // R -= G
        chunk[2] = chunk[2].wrapping_sub(g); // B -= G
    }
}

/// Invert the subtract-green transform in-place.
///
/// For each pixel: `R = (R' + G) & 0xFF`, `B = (B' + G) & 0xFF`.
pub fn inverse_subtract_green(pixels: &mut PixelContainer) {
    for chunk in pixels.data.chunks_exact_mut(4) {
        let g = chunk[1];
        chunk[0] = chunk[0].wrapping_add(g); // R += G
        chunk[2] = chunk[2].wrapping_add(g); // B += G
    }
}

// ---------------------------------------------------------------------------
// Color-index transform — inverse only (decoder side)
// ---------------------------------------------------------------------------

/// Apply the inverse color-index (palette) transform in-place.
///
/// Each pixel's **G channel** contains packed palette indices — the number of
/// indices per pixel is `pack_bits` (1, 2, 4, or 8).  Indices are packed
/// LSB-first within the G byte.  The function expands the packed image back
/// to `orig_width` columns by looking up each index in `palette`.
///
/// After this call `pixels.width == orig_width` and `pixels.data` has been
/// rebuilt with the palette RGBA values.
pub fn inverse_color_index(
    pixels: &mut PixelContainer,
    palette: &[(u8, u8, u8, u8)],
    pack_bits: u32,
    orig_width: u32,
) {
    let height      = pixels.height;
    let packed_width = pixels.width;
    let bits_per_index = 8u32 / pack_bits; // 1, 2, 4, or 8
    let index_mask  = ((1u32 << bits_per_index) - 1) as u8;
    let total = (orig_width * height) as usize;
    let mut out = Vec::with_capacity(total * 4);

    for y in 0..height {
        let mut x_out = 0u32;
        for x in 0..packed_width {
            let base = ((y * packed_width + x) as usize) * 4;
            let g = pixels.data[base + 1]; // G channel holds packed indices
            let mut bit_offset = 0u32;
            while bit_offset < 8 && x_out < orig_width {
                let idx = ((g >> bit_offset) & index_mask) as usize;
                let (r, pg, b, a) = *palette.get(idx).unwrap_or(&(0, 0, 0, 255));
                out.push(r); out.push(pg); out.push(b); out.push(a);
                x_out  += 1;
                bit_offset += bits_per_index;
            }
        }
    }

    pixels.width = orig_width;
    pixels.data  = out;
}

// ---------------------------------------------------------------------------
// Predictor transform — private helpers
// ---------------------------------------------------------------------------

// RGBA tuple used throughout predictor arithmetic.
type Pix = (u8, u8, u8, u8);

/// Clamp an i32 to [0, 255].
#[inline(always)]
fn clamp8(v: i32) -> u8 { v.clamp(0, 255) as u8 }

/// Per-channel floor average: `(a + b) >> 1`.
#[inline(always)]
fn pix_avg2(a: Pix, b: Pix) -> Pix {
    (
        ((a.0 as u16 + b.0 as u16) >> 1) as u8,
        ((a.1 as u16 + b.1 as u16) >> 1) as u8,
        ((a.2 as u16 + b.2 as u16) >> 1) as u8,
        ((a.3 as u16 + b.3 as u16) >> 1) as u8,
    )
}

/// VP8L Select predictor (mode 11).
///
/// Returns L if `sum |L - TL| ≤ sum |T - TL|`, else T.
/// The sum is taken over all four channels.
#[inline(always)]
fn pix_select(l: Pix, t: Pix, tl: Pix) -> Pix {
    let pa = (l.0 as i32 - tl.0 as i32).unsigned_abs()
           + (l.1 as i32 - tl.1 as i32).unsigned_abs()
           + (l.2 as i32 - tl.2 as i32).unsigned_abs()
           + (l.3 as i32 - tl.3 as i32).unsigned_abs();
    let pb = (t.0 as i32 - tl.0 as i32).unsigned_abs()
           + (t.1 as i32 - tl.1 as i32).unsigned_abs()
           + (t.2 as i32 - tl.2 as i32).unsigned_abs()
           + (t.3 as i32 - tl.3 as i32).unsigned_abs();
    if pa <= pb { l } else { t }
}

/// VP8L ClampedAddSubtractFull (mode 12): `Clamp(L + T - TL)` per channel.
#[inline(always)]
fn pix_clamp_add_sub_full(l: Pix, t: Pix, tl: Pix) -> Pix {
    (
        clamp8(l.0 as i32 + t.0 as i32 - tl.0 as i32),
        clamp8(l.1 as i32 + t.1 as i32 - tl.1 as i32),
        clamp8(l.2 as i32 + t.2 as i32 - tl.2 as i32),
        clamp8(l.3 as i32 + t.3 as i32 - tl.3 as i32),
    )
}

/// VP8L ClampedAddSubtractHalf (mode 13): `Clamp(L + (T - TL) / 2)` per channel.
#[inline(always)]
fn pix_clamp_add_sub_half(l: Pix, t: Pix, tl: Pix) -> Pix {
    (
        clamp8(l.0 as i32 + (t.0 as i32 - tl.0 as i32) / 2),
        clamp8(l.1 as i32 + (t.1 as i32 - tl.1 as i32) / 2),
        clamp8(l.2 as i32 + (t.2 as i32 - tl.2 as i32) / 2),
        clamp8(l.3 as i32 + (t.3 as i32 - tl.3 as i32) / 2),
    )
}

/// Read one pixel from `data` (RGBA layout).
///
/// Returns `(0, 0, 0, 0xFF)` (VP8L "outside-image" sentinel 0xFF000000) when
/// `x < 0`, `y < 0`, or `x >= width`.  Height is not checked because callers
/// only ever read from already-processed rows.
#[inline(always)]
fn pix_at(data: &[u8], width: u32, x: i32, y: i32) -> Pix {
    if x < 0 || y < 0 || x >= width as i32 {
        return (0, 0, 0, 0xFF); // 0xFF000000 sentinel
    }
    let idx = (y as usize * width as usize + x as usize) * 4;
    (data[idx], data[idx + 1], data[idx + 2], data[idx + 3])
}

/// Compute the VP8L predicted pixel for position `(x, y)` using `mode`.
///
/// Works for both the **forward** transform (pass original pixel data) and the
/// **inverse** transform (pass partially-reconstructed output array, processing
/// in raster order so L, T, TL are already final values).
///
/// Edge-pixel rules from the VP8L spec:
/// - First pixel `(0, 0)`: always `0xFF000000`.
/// - Top row `(y == 0)`: T, TL, TR are `0xFF000000`.
/// - Left column `(x == 0)`: L, TL are `0xFF000000`.
/// - Right edge `(x == width - 1)`: TR is `0xFF000000`.
pub fn compute_predictor(data: &[u8], width: u32, x: u32, y: u32, mode: u8) -> Pix {
    // The first pixel in the image is always predicted as 0xFF000000.
    if x == 0 && y == 0 {
        return (0, 0, 0, 0xFF);
    }

    let ix = x as i32;
    let iy = y as i32;

    let l  = pix_at(data, width, ix - 1, iy);
    let t  = pix_at(data, width, ix,     iy - 1);
    let tl = pix_at(data, width, ix - 1, iy - 1);
    let tr = pix_at(data, width, ix + 1, iy - 1); // 0xFF000000 for x==width-1 or y==0

    match mode {
        0  => (0, 0, 0, 0xFF),           // constant black
        1  => l,                          // left
        2  => t,                          // top
        3  => tr,                         // top-right
        4  => tl,                         // top-left
        5  => pix_avg2(pix_avg2(l, tr), t),
        6  => pix_avg2(tl, l),
        7  => pix_avg2(l, t),
        8  => pix_avg2(tl, t),
        9  => pix_avg2(t, tr),
        10 => pix_avg2(pix_avg2(tl, l), pix_avg2(t, tr)),
        11 => pix_select(l, t, tl),
        12 => pix_clamp_add_sub_full(l, t, tl),
        13 => pix_clamp_add_sub_half(l, t, tl),
        _  => (0, 0, 0, 0xFF),           // out-of-spec: treat as constant
    }
}

// ---------------------------------------------------------------------------
// Predictor transform — public API
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Color transform — inverse only (decoder side)
// ---------------------------------------------------------------------------

/// VP8L ColorTransformDelta (spec section 2.4.4).
///
/// `pred` is a signed 8-bit coefficient; `color` is the channel value.
/// Result: `(int8_t)(((int32_t)pred * (int32_t)color) >> 5)`, returned as `u8`
/// for wrapping addition into the destination channel.
#[inline(always)]
pub(crate) fn color_transform_delta(pred: i8, color: u8) -> u8 {
    (((pred as i32 * color as i32) >> 5) as i8) as u8
}

/// Apply the inverse color transform in-place (during decoding).
///
/// The predictor sub-image pixels encode coefficients in the R, G, B channels:
/// - `R` = `green_to_red`  (int8\_t)
/// - `G` = `green_to_blue` (int8\_t)
/// - `B` = `red_to_blue`   (int8\_t)
///
/// Inverse formula (spec section 2.4.4):
/// ```text
/// new_red  = red  + delta(green_to_red,  green)
/// new_blue = blue + delta(green_to_blue, green) + delta(red_to_blue, new_red)
/// ```
pub fn inverse_color(pixels: &mut PixelContainer, block_bits: u32, sub_image_data: &[u8]) {
    let width  = pixels.width;
    let height = pixels.height;
    let block_size = 1u32 << block_bits;
    let sub_w = width.div_ceil(block_size);
    let n = (width * height) as usize;

    for i in 0..n {
        let x  = (i as u32) % width;
        let y  = (i as u32) / width;
        let bx = x / block_size;
        let by = y / block_size;
        let si = ((by * sub_w + bx) as usize) * 4;
        // R channel of sub-image = green_to_red, G = green_to_blue, B = red_to_blue.
        let green_to_red  = sub_image_data[si    ] as i8;
        let green_to_blue = sub_image_data[si + 1] as i8;
        let red_to_blue   = sub_image_data[si + 2] as i8;

        let b = i * 4;
        let r = pixels.data[b    ];
        let g = pixels.data[b + 1];
        let new_r = r.wrapping_add(color_transform_delta(green_to_red, g));
        let new_b = pixels.data[b + 2]
            .wrapping_add(color_transform_delta(green_to_blue, g))
            .wrapping_add(color_transform_delta(red_to_blue, new_r));
        pixels.data[b    ] = new_r;
        pixels.data[b + 2] = new_b;
    }
}

// ---------------------------------------------------------------------------
// Predictor transform — public API
// ---------------------------------------------------------------------------

/// Block size in pixels used by the encoder's predictor transform.
///
/// `block_bits = 4` → block_size = 16 pixels.  The sub-image that stores
/// predictor modes has dimensions `ceil(W/16) × ceil(H/16)`.
pub const PREDICTOR_BLOCK_BITS: u32 = 4;

/// Apply the predictor transform (forward, during encoding).
///
/// Uses **mode 1 (left prediction)** for every block.  Returns:
/// - `sub_image_data` — raw RGBA bytes for the predictor sub-image
///   (`ceil(W / block_size) × ceil(H / block_size)` pixels, each with `G=1`).
/// - `transformed` — the residual pixel data after subtracting predictions.
pub fn apply_predictor(pixels: &PixelContainer) -> (Vec<u8>, Vec<u8>) {
    let width  = pixels.width;
    let height = pixels.height;
    let block_size = 1u32 << PREDICTOR_BLOCK_BITS;
    let sub_w = width.div_ceil(block_size);
    let sub_h = height.div_ceil(block_size);
    let n_blocks = (sub_w * sub_h) as usize;

    // Sub-image: G channel = predictor mode = 1 (left prediction).
    // R, B, A channels are 0 (ignored by the decoder).
    let mut sub_image_data = vec![0u8; n_blocks * 4];
    for i in 0..n_blocks {
        sub_image_data[i * 4 + 1] = 1; // G = mode 1
    }

    let data = &pixels.data;
    let n = (width * height) as usize;
    let mut out = Vec::with_capacity(n * 4);

    for i in 0..n {
        let x = (i as u32) % width;
        let y = (i as u32) / width;
        // For the forward transform, predict from the *original* pixel array.
        let (pr, pg, pb, pa) = compute_predictor(data, width, x, y, 1);
        let b = i * 4;
        out.push(data[b    ].wrapping_sub(pr));
        out.push(data[b + 1].wrapping_sub(pg));
        out.push(data[b + 2].wrapping_sub(pb));
        out.push(data[b + 3].wrapping_sub(pa));
    }

    (sub_image_data, out)
}

/// Apply the inverse predictor transform in-place (during decoding).
///
/// `sub_image_data` is the raw RGBA bytes of the predictor sub-image.  The
/// predictor mode for each block is stored in the **G channel** (lower 4 bits
/// are the mode; upper bits are ignored).
///
/// `block_bits` encodes the block size: `block_size = 1 << block_bits`.
/// Pixels are reconstructed in raster order so L, T, TL are always available
/// when needed.
pub fn inverse_predictor(
    pixels: &mut PixelContainer,
    block_bits: u32,
    sub_image_data: &[u8],
) {
    let width  = pixels.width;
    let height = pixels.height;
    let block_size = 1u32 << block_bits;
    let sub_w = width.div_ceil(block_size);
    let n = (width * height) as usize;

    for i in 0..n {
        let x  = (i as u32) % width;
        let y  = (i as u32) / width;
        let bx = x / block_size;
        let by = y / block_size;
        // G channel of the sub-image pixel holds the mode (lower 4 bits).
        let sub_idx = ((by * sub_w + bx) as usize) * 4 + 1; // +1 = G channel
        let mode = sub_image_data[sub_idx] & 0xF;

        // Predict from the *already-reconstructed* pixels (in-place, raster order).
        let (pr, pg, pb, pa) = compute_predictor(&pixels.data, width, x, y, mode);
        let b = i * 4;
        pixels.data[b    ] = pixels.data[b    ].wrapping_add(pr);
        pixels.data[b + 1] = pixels.data[b + 1].wrapping_add(pg);
        pixels.data[b + 2] = pixels.data[b + 2].wrapping_add(pb);
        pixels.data[b + 3] = pixels.data[b + 3].wrapping_add(pa);
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

    #[test]
    fn predictor_round_trip_mode1_solid() {
        // Solid-colour image: after mode-1 prediction all residuals except
        // the first pixel should be 0 (or near 0).
        let mut original = PixelContainer::new(4, 4);
        original.fill(200, 100, 50, 255);

        let (sub_data, transformed_data) = apply_predictor(&original);

        let mut pc = PixelContainer::from_data(original.width, original.height, transformed_data);
        inverse_predictor(&mut pc, PREDICTOR_BLOCK_BITS, &sub_data);
        assert_eq!(pc.data, original.data, "predictor round-trip failed on solid colour");
    }

    #[test]
    fn predictor_round_trip_mode1_gradient() {
        let mut original = PixelContainer::new(8, 8);
        for y in 0..8u32 {
            for x in 0..8u32 {
                original.set_pixel(x, y, (x * 30) as u8, (y * 30) as u8, 128, 255);
            }
        }

        let (sub_data, transformed_data) = apply_predictor(&original);
        let mut pc = PixelContainer::from_data(original.width, original.height, transformed_data);
        inverse_predictor(&mut pc, PREDICTOR_BLOCK_BITS, &sub_data);
        assert_eq!(pc.data, original.data, "predictor round-trip failed on gradient");
    }

    #[test]
    fn predictor_first_pixel_sentinel() {
        // The very first pixel's predictor must always be 0xFF000000.
        let pred = compute_predictor(&[200, 100, 50, 255, 0, 0, 0, 0], 2, 0, 0, 7);
        assert_eq!(pred, (0, 0, 0, 0xFF), "first-pixel predictor must be 0xFF000000");
    }

    #[test]
    fn predictor_left_edge_mode1_uses_sentinel() {
        // x=0, y=1: L is unavailable → predictor = 0xFF000000.
        // Data: row0=(R=10,G=20,B=30,A=255), row1=(R=0,G=0,B=0,A=0).
        let data = vec![10u8, 20, 30, 255, 0, 0, 0, 0];
        let pred = compute_predictor(&data, 1, 0, 1, 1); // mode 1, x=0, y=1, width=1
        assert_eq!(pred, (0, 0, 0, 0xFF), "leftmost pixel of row 1 should use sentinel L");
    }

    #[test]
    fn predictor_mode7_avg_left_top() {
        // Place known pixels and verify avg(L, T) for mode 7.
        // Width=2: pixel(0,0)=(10,20,30,255), pixel(1,0)=(50,60,70,200).
        // At (0,1): L=0xFF000000, T=pixel(0,0)=(10,20,30,255).
        // avg2((0,0,0,255), (10,20,30,255)) = (5,10,15,255).
        let data = vec![10u8,20,30,255, 50,60,70,200, 0,0,0,0, 0,0,0,0];
        let pred = compute_predictor(&data, 2, 0, 1, 7);
        assert_eq!(pred.0, 5);
        assert_eq!(pred.1, 10);
        assert_eq!(pred.2, 15);
    }

    #[test]
    fn color_transform_zero_coefficients_is_noop() {
        // All-zero sub-image → every ColorTransformDelta = 0 → pixel unchanged.
        let mut pixels = PixelContainer::new(2, 2);
        pixels.set_pixel(0, 0, 200, 100, 50, 255);
        pixels.set_pixel(1, 0, 10, 20, 30, 128);
        let snapshot = pixels.clone();

        // Sub-image: 1 block, all channels = 0.
        let sub_data = vec![0u8; 4]; // R=G=B=A=0
        inverse_color(&mut pixels, 4, &sub_data); // block_bits=4, 16-pixel blocks
        assert_eq!(pixels.data, snapshot.data, "zero-coeff color transform must be no-op");
    }

    #[test]
    fn color_transform_round_trip() {
        // Build a non-trivial sub-image with one coefficient set.
        // green_to_red = 16 (int8), green_to_blue = 0, red_to_blue = 0.
        // For pixel (R=100, G=50, B=200, A=255):
        //   delta(16_i8, 50_u8) = ((16 * 50) >> 5) as i8 = (800 >> 5) as i8 = 25 as i8 = 25.
        //   new_red  = 100 + 25 = 125
        //   new_blue = 200 + 0 + 0 = 200
        // Inverse of forward: store (125, 50, 200, 255) in the pixel, then inverse_color
        // should give us back 100 for red.
        //
        // Actually we test forward (subtract delta) then inverse (add delta) round-trip.
        let mut pixels = PixelContainer::new(1, 1);
        pixels.set_pixel(0, 0, 100, 50, 200, 255);
        let original_data = pixels.data.clone();

        // Forward color transform (manual): R' = R - delta(g2r, G) = 100 - 25 = 75.
        let g2r: i8 = 16;
        let delta = color_transform_delta(g2r, 50);
        pixels.data[0] = pixels.data[0].wrapping_sub(delta);

        // Sub-image: R channel = 16 (= 16u8 = int8_t 16 → green_to_red = 16).
        let sub_data = vec![16u8, 0, 0, 0]; // R=16(green_to_red), G=0, B=0, A=0
        inverse_color(&mut pixels, 4, &sub_data);
        assert_eq!(pixels.data, original_data, "color transform round-trip failed");
    }

    #[test]
    fn predictor_all_modes_do_not_panic_1x1() {
        // A 1×1 image only has the first pixel, so every mode should return
        // the 0xFF000000 sentinel.
        let data = vec![128u8, 64, 32, 200];
        for mode in 0..14u8 {
            let pred = compute_predictor(&data, 1, 0, 0, mode);
            assert_eq!(
                pred, (0, 0, 0, 0xFF),
                "mode {mode}: first pixel must return sentinel"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Color-index inverse tests
    // -----------------------------------------------------------------------

    /// pack_bits=1 means 1 index per pixel (no bit-packing).  The G channel
    /// of each packed pixel is the palette index directly.
    #[test]
    fn color_index_inverse_no_packing() {
        // Palette: index 0 → red, index 1 → blue
        let palette: Vec<(u8, u8, u8, u8)> = vec![(255, 0, 0, 255), (0, 0, 255, 255)];
        // Packed image: 2 pixels wide, 1 pixel tall.  G=0 → red, G=1 → blue.
        // pack_bits=1 so packed_width == orig_width == 2; no expansion.
        let mut packed = PixelContainer::from_data(
            2, 1,
            vec![
                0, 0, 0, 255, // G=0 → palette[0] = red
                0, 1, 0, 255, // G=1 → palette[1] = blue
            ],
        );
        inverse_color_index(&mut packed, &palette, 1, 2);
        assert_eq!(packed.width, 2);
        assert_eq!(packed.height, 1);
        // First pixel = red (255, 0, 0, 255)
        assert_eq!(&packed.data[0..4], &[255, 0, 0, 255], "pixel 0 should be red");
        // Second pixel = blue (0, 0, 255, 255)
        assert_eq!(&packed.data[4..8], &[0, 0, 255, 255], "pixel 1 should be blue");
    }

    /// pack_bits=4 means 4 indices packed into one G byte (2 bits each, LSB-first).
    /// A 1×1 packed image expands to a 4×1 output.
    #[test]
    fn color_index_inverse_pack4() {
        // Palette: 0=red, 1=green, 2=blue, 3=yellow
        let palette: Vec<(u8, u8, u8, u8)> = vec![
            (255, 0, 0, 255),
            (0, 255, 0, 255),
            (0, 0, 255, 255),
            (255, 255, 0, 255),
        ];
        // Pack 4 indices (2 bits each) into one byte, LSB-first:
        //   bits [1:0]=0 (red), [3:2]=1 (green), [5:4]=2 (blue), [7:6]=3 (yellow)
        //   g = 0b11_10_01_00 = 0xE4
        let g: u8 = 0b11_10_01_00;
        let mut packed = PixelContainer::from_data(
            1, 1,
            vec![0, g, 0, 255],
        );
        inverse_color_index(&mut packed, &palette, 4, 4);
        assert_eq!(packed.width, 4);
        assert_eq!(packed.height, 1);
        assert_eq!(&packed.data[0..4],  &[255, 0, 0, 255],   "pixel 0 = red");
        assert_eq!(&packed.data[4..8],  &[0, 255, 0, 255],   "pixel 1 = green");
        assert_eq!(&packed.data[8..12], &[0, 0, 255, 255],   "pixel 2 = blue");
        assert_eq!(&packed.data[12..16], &[255, 255, 0, 255], "pixel 3 = yellow");
    }

    /// pack_bits=1, 2 rows — verify rows are handled independently.
    #[test]
    fn color_index_inverse_two_rows() {
        let palette: Vec<(u8, u8, u8, u8)> = vec![(10, 20, 30, 255), (40, 50, 60, 255)];
        // 3 pixels wide, 2 rows tall.  Row 0: [0,1,0]; row 1: [1,0,1].
        let mut packed = PixelContainer::from_data(
            3, 2,
            vec![
                0, 0, 0, 255,  0, 1, 0, 255,  0, 0, 0, 255,  // row 0
                0, 1, 0, 255,  0, 0, 0, 255,  0, 1, 0, 255,  // row 1
            ],
        );
        inverse_color_index(&mut packed, &palette, 1, 3);
        assert_eq!(packed.width, 3);
        assert_eq!(packed.height, 2);
        // Row 0
        assert_eq!(&packed.data[0..4],  &[10, 20, 30, 255], "r0p0");
        assert_eq!(&packed.data[4..8],  &[40, 50, 60, 255], "r0p1");
        assert_eq!(&packed.data[8..12], &[10, 20, 30, 255], "r0p2");
        // Row 1
        assert_eq!(&packed.data[12..16], &[40, 50, 60, 255], "r1p0");
        assert_eq!(&packed.data[16..20], &[10, 20, 30, 255], "r1p1");
        assert_eq!(&packed.data[20..24], &[40, 50, 60, 255], "r1p2");
    }
}
