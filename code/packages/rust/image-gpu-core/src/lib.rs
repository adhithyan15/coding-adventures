//! # image-gpu-core (v0.2) — image filters on the matrix execution layer
//!
//! Image-domain library that builds [`matrix_ir::Graph`]s for each
//! pixel-level operation and runs them through the matrix execution
//! layer (`matrix-runtime` planner + `matrix-cpu` executor).
//!
//! ## v0.2 migration note
//!
//! v0.1 was implemented directly against `gpu-runtime` with hand-written
//! Metal / CUDA C / Rust shaders per op.  v0.2 swaps the backend to the
//! matrix execution layer (specs MX01–MX04): each op now compiles to a
//! MatrixIR graph, the planner places it on `matrix-cpu`, and the
//! executor evaluates it via straight-line Rust over the matrix layer's
//! 27-op vocabulary.
//!
//! **Public API is unchanged** — the same five functions
//! (`gpu_invert`, `gpu_colour_matrix`, `gpu_greyscale`, `gpu_gamma`,
//! `gpu_brightness`) accept and return the same `PixelContainer`s.
//! New ops (`gpu_sepia`, `gpu_contrast`, `gpu_posterize`) are added for
//! the upcoming Instagram-style filter CLI.
//!
//! ## sRGB / linear light
//!
//! The legacy v0.1 code did sRGB → linear → op → sRGB encode in shader
//! code for accurate gamma/colour-matrix work.  V1 of the migration
//! handles the piecewise sRGB transfer function in **Rust**, before
//! and after the graph runs (the IR's elementwise `Pow` could express
//! it, but as a piecewise `Where(Less(...), ...)` it adds 12+ ops per
//! pixel — V2 work).
//!
//! Filters that don't need linear light (invert, brightness, posterize)
//! work in sRGB byte space directly.
//!
//! ## Embedded-as-constants pattern
//!
//! V1 of the matrix-runtime ↔ matrix-cpu protocol doesn't yet expose a
//! "pre-allocate buffer at planner-assigned id" message, so graphs in
//! this crate embed their runtime inputs as `matrix_ir::Constant`s.
//! See [`pipeline`](crate::pipeline) for details.  V2 will switch to
//! proper graph inputs once the protocol gains that hook.

pub use pixel_container::PixelContainer;

mod pipeline;
mod sergb;

use matrix_ir::{DType, GraphBuilder, Shape};

/// User-facing error type.  Compatible with v0.1's `GpuError` for ABI
/// stability — the variant set is reduced because the matrix execution
/// layer's failure surface is much smaller than the per-backend GPU
/// runtime's.
#[derive(Debug)]
pub enum GpuError {
    /// Free-form error from the matrix execution layer (planner,
    /// executor, transport).
    Other(String),
}

impl core::fmt::Display for GpuError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GpuError::Other(s) => write!(f, "image-gpu-core: {}", s),
        }
    }
}

impl std::error::Error for GpuError {}

/// Luminance weights for greyscale conversion.
#[derive(Clone, Copy, Debug)]
pub enum LuminanceWeights {
    /// Rec.709 / sRGB: 0.2126 R + 0.7152 G + 0.0722 B (linear light).
    Rec709,
    /// BT.601 standard definition: 0.299 R + 0.587 G + 0.114 B.
    Bt601,
    /// Simple average: (R + G + B) / 3.
    Average,
}

impl LuminanceWeights {
    fn coeffs(self) -> (f32, f32, f32) {
        match self {
            LuminanceWeights::Rec709 => (0.2126, 0.7152, 0.0722),
            LuminanceWeights::Bt601 => (0.299, 0.587, 0.114),
            LuminanceWeights::Average => (1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0),
        }
    }
}

/// Pixel count in `img`.  Errors via `GpuError::Other` if dimensions overflow.
fn pixel_count(img: &PixelContainer) -> Result<usize, GpuError> {
    (img.width as usize)
        .checked_mul(img.height as usize)
        .ok_or_else(|| GpuError::Other("pixel_count overflow".to_string()))
}

// ── Helpers: encode an array of bytes as a constant tensor ────────────

/// Embed `bytes` as a u8 constant in the graph builder.  The constant
/// has shape `[bytes.len()]`.
fn const_u8_flat(g: &mut GraphBuilder, bytes: Vec<u8>) -> matrix_ir::Tensor {
    let n = bytes.len() as u32;
    g.constant(DType::U8, Shape::from(&[n]), bytes)
}

/// Embed `floats` as an f32 constant in the graph builder.  The
/// constant has shape `[floats.len()]`.
fn const_f32_flat(g: &mut GraphBuilder, floats: Vec<f32>) -> matrix_ir::Tensor {
    let n = floats.len();
    let mut bytes = Vec::with_capacity(n * 4);
    for v in &floats {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    g.constant(DType::F32, Shape::from(&[n as u32]), bytes)
}

/// Build a u8 mask of length `n_pixels * 4` that is 1 at every alpha
/// position (offset 3, 7, 11, …) and 0 elsewhere.  Used by `Where` to
/// pass alpha through unchanged.
fn alpha_mask(n_pixels: usize) -> Vec<u8> {
    let mut m = Vec::with_capacity(n_pixels * 4);
    for _ in 0..n_pixels {
        m.extend_from_slice(&[0, 0, 0, 1]);
    }
    m
}

// ── Public API ────────────────────────────────────────────────────────

/// Invert RGB channels of every pixel.  Alpha is unchanged.
///
/// # Example
///
/// ```
/// use pixel_container::PixelContainer;
/// use image_gpu_core::gpu_invert;
///
/// let mut src = PixelContainer::new(1, 1);
/// src.set_pixel(0, 0, 100, 150, 200, 255);
///
/// let dst = gpu_invert(&src).unwrap();
/// assert_eq!(dst.pixel_at(0, 0), (155, 105, 55, 255));
/// ```
pub fn gpu_invert(img: &PixelContainer) -> Result<PixelContainer, GpuError> {
    let n = pixel_count(img)?;
    let n4 = n * 4;

    let mut g = GraphBuilder::new();

    // Embed the image as a constant.  V2 will replace with a proper
    // input once the protocol gains a buffer-pre-alloc hook.
    let img_t = const_u8_flat(&mut g, img.data.clone());
    let max_t = const_u8_flat(&mut g, vec![255u8; n4]);
    let inverted = g.sub(&max_t, &img_t);
    let mask = const_u8_flat(&mut g, alpha_mask(n));
    let result = g.where_(&mask, &img_t, &inverted);
    g.output(&result);

    let graph = g
        .build()
        .map_err(|e| GpuError::Other(format!("build: {:?}", e)))?;
    let bytes = pipeline::run_graph_with_constant_inputs(&graph, result.id, n4)?;

    Ok(PixelContainer::from_data(img.width, img.height, bytes))
}

/// Apply a 3×3 colour matrix in linear light.  Alpha unchanged.
///
/// `matrix[row][col]` — `out_rgb = M × in_rgb` after sRGB decoding,
/// before sRGB re-encoding.
pub fn gpu_colour_matrix(
    img: &PixelContainer,
    matrix: &[[f32; 3]; 3],
) -> Result<PixelContainer, GpuError> {
    let n = pixel_count(img)?;

    // Decode sRGB → linear in Rust.  Linear values laid out as N×3 (RGB).
    let mut linear: Vec<f32> = Vec::with_capacity(n * 3);
    let mut alpha: Vec<u8> = Vec::with_capacity(n);
    for chunk in img.data.chunks_exact(4) {
        linear.push(sergb::decode(chunk[0]));
        linear.push(sergb::decode(chunk[1]));
        linear.push(sergb::decode(chunk[2]));
        alpha.push(chunk[3]);
    }

    // Build matrix tensor as 3×3 row-major f32.
    // Build the graph: pixels[N, 3] @ matrix[3, 3]^T → out[N, 3]
    // Note: `out_rgb = M × in_rgb` means each output channel is the dot
    // product of M's row with input RGB.  In matmul form with row-vector
    // pixels, that's `pixels @ M^T`.
    let mut m_t: Vec<f32> = Vec::with_capacity(9);
    for col in 0..3 {
        for row in 0..3 {
            m_t.push(matrix[row][col]);
        }
    }

    let mut g = GraphBuilder::new();
    let pixels = const_f32_flat(&mut g, linear);
    let pixels_2d = g.reshape(&pixels, Shape::from(&[n as u32, 3]));
    let m_const = const_f32_flat(&mut g, m_t);
    let m_2d = g.reshape(&m_const, Shape::from(&[3, 3]));
    let out = g.matmul(&pixels_2d, &m_2d);
    g.output(&out);

    let graph = g
        .build()
        .map_err(|e| GpuError::Other(format!("build: {:?}", e)))?;
    let out_bytes = pipeline::run_graph_with_constant_inputs(&graph, out.id, n * 3 * 4)?;

    // Encode linear → sRGB and recombine with original alpha.
    let mut result = Vec::with_capacity(n * 4);
    for (i, chunk) in out_bytes.chunks_exact(12).enumerate() {
        let r_lin = f32::from_le_bytes(chunk[0..4].try_into().unwrap());
        let g_lin = f32::from_le_bytes(chunk[4..8].try_into().unwrap());
        let b_lin = f32::from_le_bytes(chunk[8..12].try_into().unwrap());
        result.push(sergb::encode(r_lin));
        result.push(sergb::encode(g_lin));
        result.push(sergb::encode(b_lin));
        result.push(alpha[i]);
    }

    Ok(PixelContainer::from_data(img.width, img.height, result))
}

/// Convert to greyscale in linear light using the specified luminance
/// weights.  All three RGB outputs become the computed luminance Y.
/// Alpha is unchanged.
pub fn gpu_greyscale(
    img: &PixelContainer,
    weights: LuminanceWeights,
) -> Result<PixelContainer, GpuError> {
    let (wr, wg, wb) = weights.coeffs();
    let m = [[wr, wg, wb], [wr, wg, wb], [wr, wg, wb]];
    gpu_colour_matrix(img, &m)
}

/// Apply power-law gamma in linear light.  γ < 1 brightens midtones,
/// γ > 1 darkens.
pub fn gpu_gamma(img: &PixelContainer, gamma: f32) -> Result<PixelContainer, GpuError> {
    let n = pixel_count(img)?;

    // Decode in Rust.
    let mut linear: Vec<f32> = Vec::with_capacity(n * 3);
    let mut alpha: Vec<u8> = Vec::with_capacity(n);
    for chunk in img.data.chunks_exact(4) {
        linear.push(sergb::decode(chunk[0]));
        linear.push(sergb::decode(chunk[1]));
        linear.push(sergb::decode(chunk[2]));
        alpha.push(chunk[3]);
    }

    // Graph: pow(linear, gamma) elementwise.  We Broadcast `gamma` to
    // every element of the input tensor.
    let mut g = GraphBuilder::new();
    let xs = const_f32_flat(&mut g, linear);
    let g_scalar = const_f32_flat(&mut g, vec![gamma]);
    let g_b = g.broadcast(&g_scalar, Shape::from(&[(n * 3) as u32]));
    let out = g.pow(&xs, &g_b);
    g.output(&out);

    let graph = g
        .build()
        .map_err(|e| GpuError::Other(format!("build: {:?}", e)))?;
    let out_bytes = pipeline::run_graph_with_constant_inputs(&graph, out.id, n * 3 * 4)?;

    // Encode + recombine alpha.
    let mut result = Vec::with_capacity(n * 4);
    for (i, chunk) in out_bytes.chunks_exact(12).enumerate() {
        let r = f32::from_le_bytes(chunk[0..4].try_into().unwrap());
        let g_v = f32::from_le_bytes(chunk[4..8].try_into().unwrap());
        let b = f32::from_le_bytes(chunk[8..12].try_into().unwrap());
        result.push(sergb::encode(r));
        result.push(sergb::encode(g_v));
        result.push(sergb::encode(b));
        result.push(alpha[i]);
    }

    Ok(PixelContainer::from_data(img.width, img.height, result))
}

/// Additive brightness shift in sRGB u8 space.  `delta ∈ [-255, 255]`.
/// Each channel value is clamped to `[0, 255]` after addition.  Alpha
/// unchanged.
pub fn gpu_brightness(img: &PixelContainer, delta: i16) -> Result<PixelContainer, GpuError> {
    let n = pixel_count(img)?;
    let n4 = n * 4;

    // Apply in i32: cast(u8 → i32), Add(delta), Min(255), Max(0), cast(i32 → u8).
    // Then mask alpha back via Where.
    let mut g = GraphBuilder::new();
    let img_u8 = const_u8_flat(&mut g, img.data.clone());
    let img_i32 = g.cast(&img_u8, DType::I32);

    let delta_i32 = (delta as i32).to_le_bytes();
    let mut delta_bytes = Vec::with_capacity(4);
    delta_bytes.extend_from_slice(&delta_i32);
    let delta_const = g.constant(DType::I32, Shape::from(&[1]), delta_bytes);
    let delta_b = g.broadcast(&delta_const, Shape::from(&[n4 as u32]));

    let added = g.add(&img_i32, &delta_b);
    let max0 = g.constant(DType::I32, Shape::from(&[1]), 0i32.to_le_bytes().to_vec());
    let max0_b = g.broadcast(&max0, Shape::from(&[n4 as u32]));
    let clamped_low = g.max(&added, &max0_b);
    let max255 = g.constant(DType::I32, Shape::from(&[1]), 255i32.to_le_bytes().to_vec());
    let max255_b = g.broadcast(&max255, Shape::from(&[n4 as u32]));
    let clamped_high = g.min(&clamped_low, &max255_b);

    let result_u8 = g.cast(&clamped_high, DType::U8);
    let mask = const_u8_flat(&mut g, alpha_mask(n));
    let final_out = g.where_(&mask, &img_u8, &result_u8);
    g.output(&final_out);

    let graph = g
        .build()
        .map_err(|e| GpuError::Other(format!("build: {:?}", e)))?;
    let bytes = pipeline::run_graph_with_constant_inputs(&graph, final_out.id, n4)?;

    Ok(PixelContainer::from_data(img.width, img.height, bytes))
}

// ── New ops added for Instagram-style filter CLI (v0.2) ──────────────

/// Apply a sepia tone — 3×3 colour matrix in linear light using the
/// classic Microsoft sepia coefficients.  Alpha unchanged.
pub fn gpu_sepia(img: &PixelContainer) -> Result<PixelContainer, GpuError> {
    // Standard sepia tone matrix.
    let m = [
        [0.393, 0.769, 0.189],
        [0.349, 0.686, 0.168],
        [0.272, 0.534, 0.131],
    ];
    gpu_colour_matrix(img, &m)
}

/// Adjust contrast around the mid-grey point (128) by `scale` in sRGB
/// byte space.  `scale > 1` increases contrast, `0 < scale < 1` lowers.
/// Negative scales are accepted and produce an inversion-like effect.
pub fn gpu_contrast(img: &PixelContainer, scale: f32) -> Result<PixelContainer, GpuError> {
    let n = pixel_count(img)?;
    let n4 = n * 4;

    let mut g = GraphBuilder::new();
    let img_u8 = const_u8_flat(&mut g, img.data.clone());
    let img_f32 = g.cast(&img_u8, DType::F32);

    let mid = const_f32_flat(&mut g, vec![128.0]);
    let mid_b = g.broadcast(&mid, Shape::from(&[n4 as u32]));
    let centered = g.sub(&img_f32, &mid_b);

    let scale_const = const_f32_flat(&mut g, vec![scale]);
    let scale_b = g.broadcast(&scale_const, Shape::from(&[n4 as u32]));
    let scaled = g.mul(&centered, &scale_b);

    let recentered = g.add(&scaled, &mid_b);

    // Clamp [0, 255] in f32, then cast.
    let zero = const_f32_flat(&mut g, vec![0.0]);
    let zero_b = g.broadcast(&zero, Shape::from(&[n4 as u32]));
    let upper = const_f32_flat(&mut g, vec![255.0]);
    let upper_b = g.broadcast(&upper, Shape::from(&[n4 as u32]));
    let clamped_low = g.max(&recentered, &zero_b);
    let clamped = g.min(&clamped_low, &upper_b);
    let result_u8 = g.cast(&clamped, DType::U8);

    let mask = const_u8_flat(&mut g, alpha_mask(n));
    let final_out = g.where_(&mask, &img_u8, &result_u8);
    g.output(&final_out);

    let graph = g
        .build()
        .map_err(|e| GpuError::Other(format!("build: {:?}", e)))?;
    let bytes = pipeline::run_graph_with_constant_inputs(&graph, final_out.id, n4)?;

    Ok(PixelContainer::from_data(img.width, img.height, bytes))
}

/// Posterize: reduce the number of distinct colour levels per channel
/// to `levels` (typically 2, 4, 8, or 16).  Output looks like a
/// poster — flat regions of colour rather than smooth gradients.
pub fn gpu_posterize(img: &PixelContainer, levels: u8) -> Result<PixelContainer, GpuError> {
    if levels == 0 {
        return Err(GpuError::Other("posterize: levels must be ≥ 1".to_string()));
    }
    let n = pixel_count(img)?;
    let n4 = n * 4;
    // Step = 256 / levels (integer division), so levels distinct values:
    // 0, step, 2*step, ..., (levels-1)*step.
    let step = 256u32 / levels as u32;
    let step_u8 = step as u8;

    let mut g = GraphBuilder::new();
    let img_u8 = const_u8_flat(&mut g, img.data.clone());

    // Posterize via integer division: floor(x / step) * step.
    // We don't have integer Div in V1, so cast to i32, do it there, cast back.
    let img_i32 = g.cast(&img_u8, DType::I32);
    let step_const = g.constant(DType::I32, Shape::from(&[1]), (step as i32).to_le_bytes().to_vec());
    let step_b = g.broadcast(&step_const, Shape::from(&[n4 as u32]));
    // i32 Div: matrix-cpu currently treats Div as float-only.  Workaround:
    // approximate via cast→f32→div→cast→i32 (truncates toward zero).
    let img_f32 = g.cast(&img_i32, DType::F32);
    let step_f32 = g.cast(&step_b, DType::F32);
    let quot = g.div(&img_f32, &step_f32);
    let quot_i32 = g.cast(&quot, DType::I32);
    let multiplied = g.mul(&quot_i32, &step_b);
    let result_u8 = g.cast(&multiplied, DType::U8);

    let _ = step_u8;

    let mask = const_u8_flat(&mut g, alpha_mask(n));
    let final_out = g.where_(&mask, &img_u8, &result_u8);
    g.output(&final_out);

    let graph = g
        .build()
        .map_err(|e| GpuError::Other(format!("build: {:?}", e)))?;
    let bytes = pipeline::run_graph_with_constant_inputs(&graph, final_out.id, n4)?;

    Ok(PixelContainer::from_data(img.width, img.height, bytes))
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(r: u8, g: u8, b: u8, a: u8) -> PixelContainer {
        let mut pc = PixelContainer::new(2, 2);
        pc.fill(r, g, b, a);
        pc
    }

    #[test]
    fn invert_rgb() {
        let src = solid(100, 150, 200, 255);
        let dst = gpu_invert(&src).unwrap();
        assert_eq!(dst.pixel_at(0, 0), (155, 105, 55, 255));
    }

    #[test]
    fn invert_preserves_alpha() {
        let src = solid(0, 0, 0, 128);
        let dst = gpu_invert(&src).unwrap();
        assert_eq!(dst.pixel_at(0, 0), (255, 255, 255, 128));
    }

    #[test]
    fn invert_double_is_identity() {
        let src = solid(80, 160, 40, 200);
        let dst = gpu_invert(&gpu_invert(&src).unwrap()).unwrap();
        assert_eq!(dst, src);
    }

    #[test]
    fn colour_matrix_identity() {
        let m = [[1.0_f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let src = solid(120, 80, 40, 255);
        let dst = gpu_colour_matrix(&src, &m).unwrap();
        let (r, g, b, a) = dst.pixel_at(0, 0);
        assert!((r as i16 - 120).abs() <= 1, "R={r}");
        assert!((g as i16 - 80).abs() <= 1, "G={g}");
        assert!((b as i16 - 40).abs() <= 1, "B={b}");
        assert_eq!(a, 255);
    }

    #[test]
    fn colour_matrix_swap_rb() {
        let swap = [[0.0_f32, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]];
        let src = solid(200, 100, 50, 255);
        let dst = gpu_colour_matrix(&src, &swap).unwrap();
        let (r, g, b, a) = dst.pixel_at(0, 0);
        assert!((r as i16 - 50).abs() <= 1, "R={r}");
        assert!((g as i16 - 100).abs() <= 1, "G={g}");
        assert!((b as i16 - 200).abs() <= 1, "B={b}");
        assert_eq!(a, 255);
    }

    #[test]
    fn greyscale_equal_channels_roundtrips() {
        let src = solid(120, 120, 120, 255);
        let dst = gpu_greyscale(&src, LuminanceWeights::Rec709).unwrap();
        let (r, g, b, a) = dst.pixel_at(0, 0);
        assert_eq!(r, g);
        assert_eq!(g, b);
        assert_eq!(a, 255);
        assert!((r as i16 - 120).abs() <= 1, "Y={r}");
    }

    #[test]
    fn greyscale_preserves_alpha() {
        let src = solid(100, 100, 100, 77);
        let dst = gpu_greyscale(&src, LuminanceWeights::Average).unwrap();
        assert_eq!(dst.pixel_at(0, 0).3, 77);
    }

    #[test]
    fn gamma_one_is_identity() {
        let src = solid(100, 150, 200, 255);
        let dst = gpu_gamma(&src, 1.0).unwrap();
        let (r, g, b, _) = dst.pixel_at(0, 0);
        assert!((r as i16 - 100).abs() <= 1, "R={r}");
        assert!((g as i16 - 150).abs() <= 1, "G={g}");
        assert!((b as i16 - 200).abs() <= 1, "B={b}");
    }

    #[test]
    fn gamma_two_darkens_midgrey() {
        let src = solid(200, 200, 200, 255);
        let dst = gpu_gamma(&src, 2.0).unwrap();
        let (r, _, _, _) = dst.pixel_at(0, 0);
        assert!(r < 200, "γ=2 should darken; got {r}");
    }

    #[test]
    fn brightness_zero_is_identity() {
        let src = solid(120, 80, 200, 255);
        let dst = gpu_brightness(&src, 0).unwrap();
        assert_eq!(dst.pixel_at(0, 0), (120, 80, 200, 255));
    }

    #[test]
    fn brightness_positive_lightens_clamped_at_255() {
        let src = solid(250, 100, 50, 255);
        let dst = gpu_brightness(&src, 30).unwrap();
        let (r, g, b, _) = dst.pixel_at(0, 0);
        assert_eq!(r, 255); // saturated
        assert_eq!(g, 130);
        assert_eq!(b, 80);
    }

    #[test]
    fn brightness_negative_darkens_clamped_at_zero() {
        let src = solid(50, 100, 200, 255);
        let dst = gpu_brightness(&src, -100).unwrap();
        let (r, g, b, _) = dst.pixel_at(0, 0);
        assert_eq!(r, 0); // saturated
        assert_eq!(g, 0);
        assert_eq!(b, 100);
    }

    #[test]
    fn brightness_preserves_alpha() {
        let src = solid(100, 100, 100, 77);
        let dst = gpu_brightness(&src, 50).unwrap();
        assert_eq!(dst.pixel_at(0, 0).3, 77);
    }

    #[test]
    fn sepia_warms_grey_toward_orange() {
        let src = solid(120, 120, 120, 255);
        let dst = gpu_sepia(&src).unwrap();
        let (r, g, b, _) = dst.pixel_at(0, 0);
        // Sepia of grey moves toward warm tone: R > G > B.
        assert!(r >= g, "expected R ≥ G, got R={r} G={g}");
        assert!(g >= b, "expected G ≥ B, got G={g} B={b}");
    }

    #[test]
    fn contrast_one_is_identity() {
        let src = solid(100, 150, 200, 255);
        let dst = gpu_contrast(&src, 1.0).unwrap();
        let (r, g, b, _) = dst.pixel_at(0, 0);
        assert!((r as i16 - 100).abs() <= 1);
        assert!((g as i16 - 150).abs() <= 1);
        assert!((b as i16 - 200).abs() <= 1);
    }

    #[test]
    fn contrast_high_pushes_to_extremes() {
        // Pixel at 100 with high contrast should move toward 0.
        let src = solid(100, 100, 100, 255);
        let dst = gpu_contrast(&src, 3.0).unwrap();
        let (r, _, _, _) = dst.pixel_at(0, 0);
        assert!(r < 100, "contrast=3 should darken below-mid; got {r}");
    }

    #[test]
    fn posterize_4_levels_quantizes() {
        let src = solid(200, 100, 50, 255);
        let dst = gpu_posterize(&src, 4).unwrap();
        let (r, g, b, _) = dst.pixel_at(0, 0);
        // With 4 levels, step = 64.  Output values must be multiples of 64.
        assert_eq!(r % 64, 0);
        assert_eq!(g % 64, 0);
        assert_eq!(b % 64, 0);
    }

    #[test]
    fn posterize_zero_levels_errors() {
        let src = solid(100, 100, 100, 255);
        assert!(gpu_posterize(&src, 0).is_err());
    }
}
