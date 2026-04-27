//! # image-gpu-core — GPU-accelerated point operations (IMG06)
//!
//! Provides GPU-accelerated variants of the pixel-level operations defined
//! in `image-point-ops` (IMG03).  Every function in this crate is
//! *embarrassingly parallel*: each pixel is transformed independently,
//! making them ideal GPU workloads.
//!
//! ## Backend selection
//!
//! Operations delegate to `gpu-runtime`, which selects the best available
//! backend at process startup:
//!
//! ```text
//! Priority:
//!   1. Metal  — macOS / Apple Silicon / Intel Mac
//!   2. CUDA   — Linux / Windows with NVIDIA GPU
//!   3. CPU    — pure-Rust fallback (always available)
//! ```
//!
//! Callers see only `Result<PixelContainer, GpuError>` — backend selection
//! is transparent.
//!
//! ## Shader sources
//!
//! Each operation ships three shader sources compiled into the binary via
//! `include_str!`:
//!
//! | Backend | Language | Path                         |
//! |---------|----------|------------------------------|
//! | Metal   | MSL      | `shaders/metal/<op>.metal`   |
//! | CUDA    | CUDA C   | `shaders/cuda/<op>.cu`       |
//! | CPU     | Rust fn  | inline in this file          |
//!
//! ## Colorspace convention
//!
//! `PixelContainer` stores RGBA8 in sRGB encoding (as specified in IC00).
//! Operations that require accurate arithmetic (gamma, colour matrix,
//! greyscale, saturation) decode sRGB → linear light, operate, then
//! re-encode to sRGB.  The sRGB transfer function is implemented identically
//! in Rust, MSL, and CUDA C to ensure CPU and GPU results agree to ±1 LSB.
//!
//! ## Thread dispatch model
//!
//! All operations use `Runtime::run_pixels`, where each GPU thread handles
//! exactly one pixel:
//!
//! ```text
//!   gid  = thread_position_in_grid  (Metal)
//!        = blockIdx.x * blockDim.x + threadIdx.x  (CUDA)
//!   byte offset = gid * 4   →  [R, G, B, A]
//! ```

pub use gpu_runtime::GpuError;
use gpu_runtime::{Runtime, Shaders};
use pixel_container::PixelContainer;

// ── sRGB helpers (CPU path) ───────────────────────────────────────────────────
//
// These mirror the GPU shader implementations exactly.  They are used by the
// CPU fallback closures registered in each `Shaders` bundle.

#[inline]
fn srgb_decode(byte: u8) -> f32 {
    let v = byte as f32 / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055_f32).powf(2.4)
    }
}

#[inline]
fn srgb_encode(lin: f32) -> u8 {
    let c = lin.clamp(0.0, 1.0);
    let s = if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round() as u8
}

// ── Uniform encoding helpers ──────────────────────────────────────────────────

fn encode_matrix(m: &[[f32; 3]; 3]) -> Vec<u8> {
    let mut v = Vec::with_capacity(36);
    for row in m {
        for &x in row {
            v.extend_from_slice(&x.to_le_bytes());
        }
    }
    v
}

fn encode_f32x3(a: f32, b: f32, c: f32) -> [u8; 12] {
    let mut buf = [0u8; 12];
    buf[0..4].copy_from_slice(&a.to_le_bytes());
    buf[4..8].copy_from_slice(&b.to_le_bytes());
    buf[8..12].copy_from_slice(&c.to_le_bytes());
    buf
}

fn encode_f32(x: f32) -> [u8; 4] { x.to_le_bytes() }

fn encode_i32(x: i32) -> [u8; 4] { x.to_le_bytes() }

// ── Static shader bundles ─────────────────────────────────────────────────────
//
// Each `Shaders` constant bundles MSL source, CUDA C source, and a CPU
// fallback function.  `include_str!` embeds the shader text at compile time;
// the GPU driver (Metal/NVRTC) compiles it to native GPU binary at runtime.

static INVERT_SHADERS: Shaders = Shaders {
    metal: Some(include_str!("../shaders/metal/invert.metal")),
    cuda:  Some(include_str!("../shaders/cuda/invert.cu")),
    cpu:   Some(cpu_invert),
};

static COLOUR_MATRIX_SHADERS: Shaders = Shaders {
    metal: Some(include_str!("../shaders/metal/colour_matrix.metal")),
    cuda:  Some(include_str!("../shaders/cuda/colour_matrix.cu")),
    cpu:   Some(cpu_colour_matrix),
};

static GREYSCALE_SHADERS: Shaders = Shaders {
    metal: Some(include_str!("../shaders/metal/greyscale.metal")),
    cuda:  Some(include_str!("../shaders/cuda/greyscale.cu")),
    cpu:   Some(cpu_greyscale),
};

static GAMMA_SHADERS: Shaders = Shaders {
    metal: Some(include_str!("../shaders/metal/gamma.metal")),
    cuda:  Some(include_str!("../shaders/cuda/gamma.cu")),
    cpu:   Some(cpu_gamma),
};

static BRIGHTNESS_SHADERS: Shaders = Shaders {
    metal: Some(include_str!("../shaders/metal/brightness.metal")),
    cuda:  Some(include_str!("../shaders/cuda/brightness.cu")),
    cpu:   Some(cpu_brightness),
};

// ── CPU fallback implementations ──────────────────────────────────────────────
//
// These functions are the reference implementations.  They are used by the
// CPU backend of `gpu-runtime` when no GPU is available, and serve as the
// correctness oracle in tests.

fn cpu_invert(src: &[u8], dst: &mut [u8], _uni: &[u8]) {
    for (chunk, out) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
        out[0] = 255 - chunk[0];
        out[1] = 255 - chunk[1];
        out[2] = 255 - chunk[2];
        out[3] = chunk[3];
    }
}

fn cpu_colour_matrix(src: &[u8], dst: &mut [u8], uni: &[u8]) {
    assert!(uni.len() >= 36, "cpu_colour_matrix: uniforms must be ≥ 36 bytes (9 × f32)");
    let m: Vec<f32> = (0..9)
        .map(|i| f32::from_le_bytes(uni[i*4..i*4+4].try_into().unwrap()))
        .collect();
    for (chunk, out) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
        let (rl, gl, bl) = (srgb_decode(chunk[0]), srgb_decode(chunk[1]), srgb_decode(chunk[2]));
        out[0] = srgb_encode(m[0]*rl + m[1]*gl + m[2]*bl);
        out[1] = srgb_encode(m[3]*rl + m[4]*gl + m[5]*bl);
        out[2] = srgb_encode(m[6]*rl + m[7]*gl + m[8]*bl);
        out[3] = chunk[3];
    }
}

fn cpu_greyscale(src: &[u8], dst: &mut [u8], uni: &[u8]) {
    assert!(uni.len() >= 12, "cpu_greyscale: uniforms must be ≥ 12 bytes (3 × f32)");
    let wr = f32::from_le_bytes(uni[0..4].try_into().unwrap());
    let wg = f32::from_le_bytes(uni[4..8].try_into().unwrap());
    let wb = f32::from_le_bytes(uni[8..12].try_into().unwrap());
    for (chunk, out) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
        let y = wr * srgb_decode(chunk[0]) + wg * srgb_decode(chunk[1]) + wb * srgb_decode(chunk[2]);
        let v = srgb_encode(y);
        out[0] = v; out[1] = v; out[2] = v; out[3] = chunk[3];
    }
}

fn cpu_gamma(src: &[u8], dst: &mut [u8], uni: &[u8]) {
    assert!(uni.len() >= 4, "cpu_gamma: uniforms must be ≥ 4 bytes (1 × f32)");
    let g = f32::from_le_bytes(uni[0..4].try_into().unwrap());
    for (chunk, out) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
        out[0] = srgb_encode(srgb_decode(chunk[0]).powf(g));
        out[1] = srgb_encode(srgb_decode(chunk[1]).powf(g));
        out[2] = srgb_encode(srgb_decode(chunk[2]).powf(g));
        out[3] = chunk[3];
    }
}

fn cpu_brightness(src: &[u8], dst: &mut [u8], uni: &[u8]) {
    assert!(uni.len() >= 4, "cpu_brightness: uniforms must be ≥ 4 bytes (1 × i32)");
    let delta = i32::from_le_bytes(uni[0..4].try_into().unwrap());
    for (chunk, out) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
        let adj = |c: u8| (c as i32 + delta).clamp(0, 255) as u8;
        out[0] = adj(chunk[0]);
        out[1] = adj(chunk[1]);
        out[2] = adj(chunk[2]);
        out[3] = chunk[3];
    }
}

// ── Internal dispatch helpers (accept an explicit runtime reference) ───────────
//
// These allow tests to inject a CPU-only runtime without touching the global
// singleton, avoiding Metal/CUDA initialisation in sandboxed test environments.

fn invert_impl(rt: &Runtime, img: &PixelContainer) -> Result<PixelContainer, GpuError> {
    let px    = pixel_count(img);
    let bytes = rt.run_pixels(&INVERT_SHADERS, "gpu_invert", &img.data, &[], px)?;
    Ok(PixelContainer::from_data(img.width, img.height, bytes))
}

fn colour_matrix_impl(rt: &Runtime, img: &PixelContainer, matrix: &[[f32; 3]; 3]) -> Result<PixelContainer, GpuError> {
    let uni   = encode_matrix(matrix);
    let px    = pixel_count(img);
    let bytes = rt.run_pixels(&COLOUR_MATRIX_SHADERS, "gpu_colour_matrix", &img.data, &uni, px)?;
    Ok(PixelContainer::from_data(img.width, img.height, bytes))
}

fn greyscale_impl(rt: &Runtime, img: &PixelContainer, wr: f32, wg: f32, wb: f32) -> Result<PixelContainer, GpuError> {
    let uni   = encode_f32x3(wr, wg, wb);
    let px    = pixel_count(img);
    let bytes = rt.run_pixels(&GREYSCALE_SHADERS, "gpu_greyscale", &img.data, &uni, px)?;
    Ok(PixelContainer::from_data(img.width, img.height, bytes))
}

fn gamma_impl(rt: &Runtime, img: &PixelContainer, gamma: f32) -> Result<PixelContainer, GpuError> {
    let uni   = encode_f32(gamma);
    let px    = pixel_count(img);
    let bytes = rt.run_pixels(&GAMMA_SHADERS, "gpu_gamma", &img.data, &uni, px)?;
    Ok(PixelContainer::from_data(img.width, img.height, bytes))
}

fn brightness_impl(rt: &Runtime, img: &PixelContainer, delta: i16) -> Result<PixelContainer, GpuError> {
    let uni   = encode_i32(delta as i32);
    let px    = pixel_count(img);
    let bytes = rt.run_pixels(&BRIGHTNESS_SHADERS, "gpu_brightness", &img.data, &uni, px)?;
    Ok(PixelContainer::from_data(img.width, img.height, bytes))
}

// ── Public GPU-accelerated API ────────────────────────────────────────────────

/// Invert RGB channels of every pixel.  Alpha is unchanged.
///
/// GPU-accelerates on Metal (macOS) or CUDA (NVIDIA Linux/Windows).
/// Falls back to a pure-Rust implementation on CPU when no GPU is available.
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
    invert_impl(&Runtime::global(), img)
}

/// Apply a 3×3 colour matrix in linear light.
///
/// `matrix[row][col]` — the transformation `out_rgb = M × in_rgb` is applied
/// after sRGB decoding and before sRGB re-encoding.  Alpha is unchanged.
///
/// Common matrices:
/// - Identity: `[[1,0,0],[0,1,0],[0,0,1]]`
/// - Swap R↔B: `[[0,0,1],[0,1,0],[1,0,0]]`
pub fn gpu_colour_matrix(img: &PixelContainer, matrix: &[[f32; 3]; 3]) -> Result<PixelContainer, GpuError> {
    colour_matrix_impl(&Runtime::global(), img, matrix)
}

/// Luminance weights for greyscale conversion.
#[derive(Clone, Copy, Debug)]
pub enum LuminanceWeights {
    /// Rec.709 / sRGB: 0.2126 R + 0.7152 G + 0.0722 B  (linear light)
    Rec709,
    /// BT.601 standard definition: 0.299 R + 0.587 G + 0.114 B
    Bt601,
    /// Simple average: (R + G + B) / 3
    Average,
}

/// Convert to greyscale in linear light using the specified luminance weights.
///
/// All three RGB output channels are set to the computed luminance Y.
/// Alpha is unchanged.
pub fn gpu_greyscale(img: &PixelContainer, weights: LuminanceWeights) -> Result<PixelContainer, GpuError> {
    let (wr, wg, wb) = match weights {
        LuminanceWeights::Rec709  => (0.2126_f32, 0.7152_f32, 0.0722_f32),
        LuminanceWeights::Bt601   => (0.299_f32,  0.587_f32,  0.114_f32),
        LuminanceWeights::Average => (1.0/3.0_f32, 1.0/3.0_f32, 1.0/3.0_f32),
    };
    greyscale_impl(&Runtime::global(), img, wr, wg, wb)
}

/// Apply power-law gamma in linear light.
///
/// - γ < 1 brightens midtones
/// - γ > 1 darkens midtones
/// - γ = 1 is identity
pub fn gpu_gamma(img: &PixelContainer, gamma: f32) -> Result<PixelContainer, GpuError> {
    gamma_impl(&Runtime::global(), img, gamma)
}

/// Additive brightness shift in sRGB u8.
///
/// `delta` ∈ \[-255, 255\].  Each channel value is clamped to \[0, 255\] after
/// addition.  Alpha is unchanged.
pub fn gpu_brightness(img: &PixelContainer, delta: i16) -> Result<PixelContainer, GpuError> {
    brightness_impl(&Runtime::global(), img, delta)
}

#[inline]
fn pixel_count(img: &PixelContainer) -> usize {
    (img.width as usize)
        .checked_mul(img.height as usize)
        .expect("pixel_count overflow: image dimensions too large for usize")
}

// ── Tests ─────────────────────────────────────────────────────────────────────
//
// Unit tests use `Runtime::cpu_only()` so that they run in sandboxed
// environments (CI, Claude Code) without requiring a GPU driver.  The CPU
// path exercises identical uniform-decoding and dispatch logic; only the
// execution engine (GPU vs Rust fn) differs.

#[cfg(test)]
mod tests {
    use super::*;

    fn cpu() -> Runtime { Runtime::cpu_only() }

    fn solid(r: u8, g: u8, b: u8, a: u8) -> PixelContainer {
        let mut pc = PixelContainer::new(2, 2);
        pc.fill(r, g, b, a);
        pc
    }

    // ── invert ────────────────────────────────────────────────────────────────

    #[test]
    fn invert_rgb() {
        let src = solid(100, 150, 200, 255);
        let dst = invert_impl(&cpu(), &src).unwrap();
        assert_eq!(dst.pixel_at(0, 0), (155, 105, 55, 255));
    }

    #[test]
    fn invert_preserves_alpha() {
        let src = solid(0, 0, 0, 128);
        let dst = invert_impl(&cpu(), &src).unwrap();
        assert_eq!(dst.pixel_at(0, 0), (255, 255, 255, 128));
    }

    #[test]
    fn invert_double_is_identity() {
        let src = solid(80, 160, 40, 200);
        let rt  = cpu();
        let dst = invert_impl(&rt, &invert_impl(&rt, &src).unwrap()).unwrap();
        assert_eq!(dst, src);
    }

    // ── colour_matrix ─────────────────────────────────────────────────────────

    #[test]
    fn colour_matrix_identity() {
        let m = [[1.0_f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let src = solid(120, 80, 40, 255);
        let dst = colour_matrix_impl(&cpu(), &src, &m).unwrap();
        let (r, g, b, a) = dst.pixel_at(0, 0);
        assert!((r as i16 - 120).abs() <= 1, "R={r}");
        assert!((g as i16 - 80).abs() <= 1,  "G={g}");
        assert!((b as i16 - 40).abs() <= 1,  "B={b}");
        assert_eq!(a, 255);
    }

    #[test]
    fn colour_matrix_swap_rb() {
        let swap = [[0.0_f32, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]];
        let src  = solid(200, 100, 50, 255);
        let dst  = colour_matrix_impl(&cpu(), &src, &swap).unwrap();
        let (r, g, b, a) = dst.pixel_at(0, 0);
        assert!((r as i16 - 50).abs() <= 1,  "R={r}");
        assert!((g as i16 - 100).abs() <= 1, "G={g}");
        assert!((b as i16 - 200).abs() <= 1, "B={b}");
        assert_eq!(a, 255);
    }

    // ── greyscale ─────────────────────────────────────────────────────────────

    #[test]
    fn greyscale_equal_channels_roundtrips() {
        let src = solid(120, 120, 120, 255);
        let dst = greyscale_impl(&cpu(), &src, 0.2126, 0.7152, 0.0722).unwrap();
        let (r, g, b, a) = dst.pixel_at(0, 0);
        assert_eq!(r, g);
        assert_eq!(g, b);
        assert_eq!(a, 255);
        assert!((r as i16 - 120).abs() <= 1, "Y={r}");
    }

    #[test]
    fn greyscale_preserves_alpha() {
        let src = solid(100, 100, 100, 77);
        let dst = greyscale_impl(&cpu(), &src, 1.0/3.0, 1.0/3.0, 1.0/3.0).unwrap();
        assert_eq!(dst.pixel_at(0, 0).3, 77);
    }

    // ── gamma ─────────────────────────────────────────────────────────────────

    #[test]
    fn gamma_one_is_identity() {
        let src = solid(100, 150, 200, 255);
        let dst = gamma_impl(&cpu(), &src, 1.0).unwrap();
        let (r, g, b, _) = dst.pixel_at(0, 0);
        assert!((r as i16 - 100).abs() <= 1, "R={r}");
        assert!((g as i16 - 150).abs() <= 1, "G={g}");
        assert!((b as i16 - 200).abs() <= 1, "B={b}");
    }

    #[test]
    fn gamma_two_darkens() {
        let src = solid(200, 200, 200, 255);
        let dst = gamma_impl(&cpu(), &src, 2.0).unwrap();
        let (r, _, _, _) = dst.pixel_at(0, 0);
        assert!(r < 200, "γ=2 should darken mid-grey; got {r}");
    }

    #[test]
    fn gamma_half_brightens() {
        let src = solid(50, 50, 50, 255);
        let dst = gamma_impl(&cpu(), &src, 0.5).unwrap();
        let (r, _, _, _) = dst.pixel_at(0, 0);
        assert!(r > 50, "γ=0.5 should brighten; got {r}");
    }

    // ── brightness ────────────────────────────────────────────────────────────

    #[test]
    fn brightness_positive() {
        let src = solid(100, 100, 100, 255);
        let dst = brightness_impl(&cpu(), &src, 50).unwrap();
        assert_eq!(dst.pixel_at(0, 0), (150, 150, 150, 255));
    }

    #[test]
    fn brightness_negative() {
        let src = solid(100, 100, 100, 255);
        let dst = brightness_impl(&cpu(), &src, -50).unwrap();
        assert_eq!(dst.pixel_at(0, 0), (50, 50, 50, 255));
    }

    #[test]
    fn brightness_clamps_high() {
        let src = solid(250, 250, 250, 255);
        let dst = brightness_impl(&cpu(), &src, 100).unwrap();
        assert_eq!(dst.pixel_at(0, 0), (255, 255, 255, 255));
    }

    #[test]
    fn brightness_clamps_low() {
        let src = solid(10, 10, 10, 255);
        let dst = brightness_impl(&cpu(), &src, -100).unwrap();
        assert_eq!(dst.pixel_at(0, 0), (0, 0, 0, 255));
    }

    #[test]
    fn brightness_preserves_alpha() {
        let src = solid(100, 100, 100, 42);
        let dst = brightness_impl(&cpu(), &src, 10).unwrap();
        assert_eq!(dst.pixel_at(0, 0).3, 42);
    }

    // ── GPU vs CPU parity ─────────────────────────────────────────────────────
    //
    // Verify that the uniform-encoding round-trips match between the Rust CPU
    // fallback and hand-computed expected values.  These tests use the CPU
    // path exclusively; on a real GPU machine the same uniforms are passed to
    // the Metal/CUDA kernels.

    #[test]
    fn invert_all_pixels_correct() {
        let mut src = PixelContainer::new(4, 4);
        for (i, b) in src.data.iter_mut().enumerate() { *b = i as u8; }

        let mut cpu_dst = vec![0u8; src.data.len()];
        cpu_invert(&src.data, &mut cpu_dst, &[]);
        let expected = PixelContainer::from_data(4, 4, cpu_dst);

        let got = invert_impl(&cpu(), &src).unwrap();
        assert_eq!(got, expected);
    }

    #[test]
    fn colour_matrix_uniform_encoding_roundtrip() {
        // A known non-trivial matrix: swap R↔B and halve G.
        let m: [[f32; 3]; 3] = [[0.0, 0.0, 1.0], [0.0, 0.5, 0.0], [1.0, 0.0, 0.0]];
        let src = solid(200, 100, 50, 255);
        let dst = colour_matrix_impl(&cpu(), &src, &m).unwrap();
        // R_out = B_in (after sRGB round-trip) ≈ 50, G_out ≈ 50% of linear(100), B_out ≈ 200
        let (r, _, b, _) = dst.pixel_at(0, 0);
        assert!((r as i16 - 50).abs() <= 2,  "R={r}");
        assert!((b as i16 - 200).abs() <= 2, "B={b}");
    }
}
