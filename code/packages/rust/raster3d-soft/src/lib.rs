//! # raster3d-soft
//!
//! `GAME33`: deterministic, zero-allocation (steady-state) software 3D triangle
//! rasterizer outputting directly into a caller-provided [`PixelContainer`]
//! (`IMG00`) and caller-provided depth buffer (`f32` or `f64`).
//!
//! ## Pipeline Architecture
//!
//! Each submitted triangle passes through the classic Era 3 software 3D
//! pipeline (as surveyed in `GAME00` §3 Era 3 & §8):
//!
//! ```text
//!  Model Vertex (pos, normal, rgb, u, v)
//!         │
//!         ▼  1. Vertex Transform (MVP Mat4 + Normal Mat3 + LightRig)
//!  Clip-Space Vertex (x, y, z, w)
//!         │
//!         ▼  2. Homogeneous 4D Sutherland-Hodgman Clipping
//!         │     (w >= MIN_CLIP_W, -w <= x,y <= w, near/far planes)
//!         │     ──► stack-only [ClipVertex; 12], fan triangulation
//!         ▼  3. Perspective Divide (inv_w = 1/w) & Viewport Transform
//!  Screen Vertex (sx, sy, depth, inv_w, attr * inv_w)
//!         │
//!         ▼  4. Backface / Frontface Culling & Area Guard
//!         │     (signed screen-space cross product)
//!         ▼  5. Half-Space Edge-Function Rasterization + Top-Left Rule
//!         │     (perspective-correct 1/w interpolation, Z-test, shading)
//!         ▼
//!  PixelContainer (RGBA8) + Z-Buffer (f32 / f64)
//! ```
//!
//! ## Perspective-Correct Attribute Interpolation
//!
//! Screen-space barycentric weights `(b0, b1, b2)` obtained from the 2D edge
//! functions are **affine in screen coordinates**, whereas 3D attributes (such
//! as texture coordinates `(u, v)`, RGB vertex colors, and vertex normals) vary
//! hyperbolically with depth. To interpolate any attribute `A` without affine
//! texture swimming:
//!
//! 1. At each vertex `i`, pre-divide by clip `w_i`: store `inv_w_i = 1 / w_i`
//!    and `A_i * inv_w_i`.
//! 2. At pixel center `(x + 0.5, y + 0.5)`:
//!    `one_over_w = b0 * inv_w_0 + b1 * inv_w_1 + b2 * inv_w_2`
//!    `A(x, y) = (b0 * (A_0 * inv_w_0) + b1 * (A_1 * inv_w_1) + b2 * (A_2 * inv_w_2)) / one_over_w`.
//!
//! Screen-space depth `z_ndc = z_clip / w_clip` is already affine in screen
//! coordinates under perspective projection, so `depth` is interpolated directly
//! with `(b0, b1, b2)`.
//!
//! ## Safety, Determinism, and Zero-Allocation Guarantees
//!
//! * `#![forbid(unsafe_code)]` — zero pointer arithmetic or unchecked indexing.
//! * `capabilities: []` — pure in-memory computation with zero ambient authority.
//! * **Zero steady-state heap allocation**: [`draw_triangle_into`] uses fixed
//!   stack arrays only, and [`draw_indexed_mesh_into`] reuses a pre-allocated
//!   [`RasterScratch`] buffer (`VER07` `max_alloc_bytes = 0`).
//! * **CWE-20 / CWE-190 / CWE-369 / CWE-400 / CWE-125 / CWE-787 guards**:
//!   all inputs are checked for finiteness and magnitude bounds; dimensions,
//!   vertex counts, and triangle counts are capped; near-zero `w`, `1/w`, and
//!   screen areas are rejected without division by zero; screen bounding boxes
//!   are clamped in `f64` prior to integer conversion; and all buffer accesses
//!   use checked indexing.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Generated capability verification witnesses.
pub mod capabilities;

pub use capabilities::PureCapabilities;
pub use math3d::{Mat3, Mat4, Vec3, Vec4};
pub use pixel_container::PixelContainer;

// ---------------------------------------------------------------------------
// Type Aliases & Constants
// ---------------------------------------------------------------------------

/// Linear normalized RGB color `[r, g, b]` (nominally in `[0.0, 1.0]`).
pub type RgbColor = [f64; 3];

/// Packed 8-bit RGBA pixel `[r, g, b, a]`.
pub type Rgba8 = [u8; 4];

/// 2D texture coordinates `[u, v]`.
pub type UvCoord = [f64; 2];

/// Maximum supported framebuffer width in pixels (CWE-400).
pub const MAX_FRAMEBUFFER_WIDTH: u32 = 4096;

/// Maximum supported framebuffer height in pixels (CWE-400).
pub const MAX_FRAMEBUFFER_HEIGHT: u32 = 4096;

/// Maximum supported texture width or height in pixels (CWE-400).
pub const MAX_TEXTURE_DIM: u32 = 4096;

/// Maximum number of vertices allowed in a single indexed mesh draw call (CWE-400).
pub const MAX_VERTICES: usize = 1_000_000;

/// Maximum number of triangles allowed in a single indexed mesh draw call (CWE-400).
pub const MAX_TRIANGLES: usize = 1_000_000;

/// Stack capacity for Sutherland-Hodgman clipped polygon vertices (3 + 7 planes <= 10 < 12).
pub const MAX_CLIP_VERTICES: usize = 12;

/// Minimum homogeneous `w` coordinate accepted after near-plane clipping (CWE-369).
pub const MIN_CLIP_W: f64 = 1e-5;

/// Minimum absolute doubled screen-space triangle area for rasterization (CWE-369).
pub const AREA_EPSILON: f64 = 1e-9;

/// Maximum coordinate / UV / normal component magnitude accepted on input (CWE-190 / CWE-20).
pub const MAX_COORD: f64 = 1.0e9;

/// FNV-1a 64-bit offset basis for canonical deterministic frame digests.
const FNV1A_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;

/// FNV-1a 64-bit prime.
const FNV1A_PRIME: u64 = 0x0100_0000_01b3;

// ---------------------------------------------------------------------------
// Error Type
// ---------------------------------------------------------------------------

/// Failure modes returned by `raster3d-soft` operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RasterError {
    /// An input float (`position`, `normal`, `color`, `uv`, matrix, light, or depth) is `NaN` or `+/-Inf` (CWE-20).
    NonFiniteInput,
    /// A coordinate, UV, color channel, or intensity lies outside safe bounded ranges (CWE-20 / CWE-190).
    ValueOutOfBounds,
    /// Framebuffer, Z-buffer, or texture dimensions are zero, exceed limits, or do not match buffer length (CWE-400 / CWE-125 / CWE-787).
    InvalidDimensions,
    /// Mesh vertex count or triangle count exceeds `MAX_VERTICES` or `MAX_TRIANGLES` (CWE-400).
    MeshTooLarge,
    /// Index slice length is not a multiple of 3.
    InvalidIndexCount,
    /// A triangle vertex index refers past `vertices.len()` (CWE-125).
    IndexOutOfBounds,
    /// Directional or ambient light parameters are invalid (e.g. zero-length direction or negative intensity) (CWE-20 / CWE-369).
    InvalidLight,
    /// A textured shading mode was requested with `texture == None`.
    MissingTexture,
    /// Homogeneous `w` violates `w >= MIN_CLIP_W` in explicit near-clip validation (CWE-369).
    NearClipRejected,
    /// Underlying `math3d` validation or algebraic failure.
    Math(math3d::Math3dError),
}

impl std::fmt::Display for RasterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "input contains NaN or Infinity"),
            Self::ValueOutOfBounds => write!(f, "input value exceeds safe bounds"),
            Self::InvalidDimensions => write!(
                f,
                "framebuffer, Z-buffer, or texture dimensions are invalid"
            ),
            Self::MeshTooLarge => write!(f, "mesh exceeds MAX_VERTICES or MAX_TRIANGLES"),
            Self::InvalidIndexCount => write!(f, "index slice length must be a multiple of 3"),
            Self::IndexOutOfBounds => write!(f, "mesh vertex index is out of bounds"),
            Self::InvalidLight => write!(f, "light configuration is invalid"),
            Self::MissingTexture => write!(f, "textured shading mode requires a texture"),
            Self::NearClipRejected => write!(f, "homogeneous w is below MIN_CLIP_W"),
            Self::Math(e) => write!(f, "math3d error: {e}"),
        }
    }
}

impl std::error::Error for RasterError {}

impl From<math3d::Math3dError> for RasterError {
    #[inline]
    fn from(err: math3d::Math3dError) -> Self {
        Self::Math(err)
    }
}

// ---------------------------------------------------------------------------
// Warrant Contract Validation Helpers
// ---------------------------------------------------------------------------

/// Validate framebuffer width or height against `[1, MAX_FRAMEBUFFER_WIDTH]` (`raster3d-soft.warrant`).
#[inline]
pub fn validate_framebuffer_dim(n: u32) -> Result<bool, RasterError> {
    if n == 0 || n > MAX_FRAMEBUFFER_WIDTH || n > MAX_FRAMEBUFFER_HEIGHT {
        return Err(RasterError::InvalidDimensions);
    }
    Ok(true)
}

/// Validate texture width or height against `[1, MAX_TEXTURE_DIM]` (`raster3d-soft.warrant`).
#[inline]
pub fn validate_texture_dim(n: u32) -> Result<bool, RasterError> {
    if n == 0 || n > MAX_TEXTURE_DIM {
        return Err(RasterError::InvalidDimensions);
    }
    Ok(true)
}

/// Validate triangle count against `MAX_TRIANGLES` (`raster3d-soft.warrant`).
#[inline]
pub fn validate_triangle_count(n: usize) -> Result<bool, RasterError> {
    if n > MAX_TRIANGLES {
        return Err(RasterError::MeshTooLarge);
    }
    Ok(true)
}

/// Validate a vertex index against `vertex_count` (`raster3d-soft.warrant`, CWE-125).
#[inline]
pub fn validate_vertex_index(idx: usize, vertex_count: usize) -> Result<bool, RasterError> {
    if idx >= vertex_count {
        return Err(RasterError::IndexOutOfBounds);
    }
    Ok(true)
}

/// Validate homogeneous clip-space `w >= MIN_CLIP_W` (`raster3d-soft.warrant`, CWE-369).
#[inline]
pub fn validate_clip_w(w: f64) -> Result<bool, RasterError> {
    if !w.is_finite() {
        return Err(RasterError::NonFiniteInput);
    }
    if w < MIN_CLIP_W {
        return Err(RasterError::NearClipRejected);
    }
    Ok(true)
}

// ---------------------------------------------------------------------------
// Internal Scalar & Matrix Guards
// ---------------------------------------------------------------------------

#[inline]
fn check_finite_f64(v: f64) -> Result<f64, RasterError> {
    if !v.is_finite() {
        return Err(RasterError::NonFiniteInput);
    }
    Ok(v)
}

#[inline]
fn check_bounded_f64(v: f64) -> Result<f64, RasterError> {
    if !v.is_finite() {
        return Err(RasterError::NonFiniteInput);
    }
    if v.abs() > MAX_COORD {
        return Err(RasterError::ValueOutOfBounds);
    }
    Ok(v)
}

#[inline]
fn check_rgb(c: RgbColor) -> Result<RgbColor, RasterError> {
    for ch in c {
        if !ch.is_finite() {
            return Err(RasterError::NonFiniteInput);
        }
        if !(0.0..=MAX_COORD).contains(&ch) {
            return Err(RasterError::ValueOutOfBounds);
        }
    }
    Ok(c)
}

#[inline]
fn check_mat4_finite(m: &Mat4) -> Result<(), RasterError> {
    for col in m.cols {
        for v in col {
            check_finite_f64(v)?;
        }
    }
    Ok(())
}

#[inline]
fn check_mat3_finite(m: &Mat3) -> Result<(), RasterError> {
    for col in m.cols {
        for v in col {
            check_finite_f64(v)?;
        }
    }
    Ok(())
}

#[inline]
fn float_to_u8(c: f64) -> u8 {
    let clamped = c.clamp(0.0, 1.0);
    let scaled = (clamped * 255.0 + 0.5).floor();
    if scaled >= 255.0 {
        255
    } else if scaled <= 0.0 {
        0
    } else {
        scaled as u8
    }
}

// ---------------------------------------------------------------------------
// Configuration Enums
// ---------------------------------------------------------------------------

/// Homogeneous NDC depth convention.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ClipSpace {
    /// OpenGL convention: NDC `z` in `[-1, 1]` (maps to depth `[0, 1]` via `(z + 1) * 0.5`).
    #[default]
    OpenGl,
    /// Vulkan / Metal / WGPU convention: NDC `z` in `[0, 1]`.
    ZeroToOne,
}

/// Triangle shading and texturing mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ShadingMode {
    /// Per-triangle face normal lighting times average triangle color.
    Flat,
    /// Per-vertex lighting interpolated with perspective-correct barycentric weights.
    #[default]
    Gouraud,
    /// Per-pixel perspective-correct normal interpolation and directional + ambient lighting.
    Phong,
    /// Perspective-correct U/V texture sampling with nearest-neighbor filtering, modulated by lit color.
    TexturedNearest,
    /// Perspective-correct U/V texture sampling with bilinear filtering, modulated by lit color.
    TexturedBilinear,
}

/// Triangle face culling mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CullMode {
    /// Do not cull either face (double-sided rendering).
    None,
    /// Cull back-facing triangles (standard closed-mesh optimization).
    #[default]
    Back,
    /// Cull front-facing triangles.
    Front,
}

/// Winding convention for front-facing triangles in clip/NDC space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FrontFace {
    /// Counter-clockwise winding in NDC / view space is front-facing (right-handed default).
    #[default]
    CounterClockwise,
    /// Clockwise winding in NDC / view space is front-facing.
    Clockwise,
}

/// Texture coordinate wrap behavior outside `[0, 1]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextureWrap {
    /// Tile texture periodically (`coord - floor(coord)`).
    #[default]
    Repeat,
    /// Clamp coordinates to `[0, 1]`.
    Clamp,
}

/// Depth buffer comparison predicate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DepthFunc {
    /// Fragment passes when `depth < stored_depth`.
    #[default]
    Less,
    /// Fragment passes when `depth <= stored_depth`.
    LessEqual,
}

// ---------------------------------------------------------------------------
// Vertex, Light, Texture, DepthBuffer, Config, Scratch, Stats
// ---------------------------------------------------------------------------

/// Input 3D vertex carrying position, normal, linear RGB color, and texture `(u, v)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    /// Object-space / world-space 3D position.
    pub position: Vec3,
    /// Surface normal vector.
    pub normal: Vec3,
    /// Linear RGB vertex color (`[0.0, 1.0]`).
    pub color: RgbColor,
    /// Horizontal texture coordinate.
    pub u: f64,
    /// Vertical texture coordinate.
    pub v: f64,
}

impl Vertex {
    /// Construct a `Vertex` directly.
    #[inline]
    pub const fn vertex_new(position: Vec3, normal: Vec3, color: RgbColor, u: f64, v: f64) -> Self {
        Self {
            position,
            normal,
            color,
            u,
            v,
        }
    }

    /// Construct and validate a `Vertex`, rejecting non-finite or out-of-bounds values (CWE-20).
    #[inline]
    pub fn vertex_try_new(
        position: Vec3,
        normal: Vec3,
        color: RgbColor,
        u: f64,
        v: f64,
    ) -> Result<Self, RasterError> {
        let vert = Self {
            position,
            normal,
            color,
            u,
            v,
        };
        vert.vertex_validate()?;
        Ok(vert)
    }

    /// Validate all fields of this vertex for finiteness and magnitude bounds (CWE-20).
    #[inline]
    pub fn vertex_validate(&self) -> Result<(), RasterError> {
        check_bounded_f64(self.position.x)?;
        check_bounded_f64(self.position.y)?;
        check_bounded_f64(self.position.z)?;
        check_bounded_f64(self.normal.x)?;
        check_bounded_f64(self.normal.y)?;
        check_bounded_f64(self.normal.z)?;
        check_rgb(self.color)?;
        check_bounded_f64(self.u)?;
        check_bounded_f64(self.v)?;
        Ok(())
    }
}

/// Directional light source defined by a unit vector pointing *toward* the light.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirectionalLight {
    /// Unit vector pointing from the lit surface toward the light source.
    pub direction_to_light: Vec3,
    /// Linear RGB light color.
    pub color: RgbColor,
    /// Non-negative scalar intensity multiplier.
    pub intensity: f64,
}

impl DirectionalLight {
    /// Construct a validated [`DirectionalLight`], normalizing `direction_to_light`.
    pub fn dir_light_new(
        direction_to_light: Vec3,
        color: RgbColor,
        intensity: f64,
    ) -> Result<Self, RasterError> {
        if !direction_to_light.vec3_is_finite() || !intensity.is_finite() {
            return Err(RasterError::NonFiniteInput);
        }
        if !(0.0..=MAX_COORD).contains(&intensity) {
            return Err(RasterError::InvalidLight);
        }
        check_rgb(color)?;
        let unit = direction_to_light
            .normalize()
            .map_err(|_| RasterError::InvalidLight)?;
        Ok(Self {
            direction_to_light: unit,
            color,
            intensity,
        })
    }

    /// Validate an existing [`DirectionalLight`] and return its unit direction vector.
    #[inline]
    pub fn dir_light_validate(&self) -> Result<Vec3, RasterError> {
        if !self.direction_to_light.vec3_is_finite() || !self.intensity.is_finite() {
            return Err(RasterError::NonFiniteInput);
        }
        if !(0.0..=MAX_COORD).contains(&self.intensity) {
            return Err(RasterError::InvalidLight);
        }
        check_rgb(self.color)?;
        self.direction_to_light
            .normalize()
            .map_err(|_| RasterError::InvalidLight)
    }
}

/// Combined ambient + directional lighting configuration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightRig {
    /// Linear RGB ambient light color.
    pub ambient_color: RgbColor,
    /// Non-negative ambient intensity multiplier.
    pub ambient_intensity: f64,
    /// Primary directional light source.
    pub directional: DirectionalLight,
}

impl LightRig {
    /// Construct a validated [`LightRig`].
    pub fn light_rig_new(
        ambient_color: RgbColor,
        ambient_intensity: f64,
        directional: DirectionalLight,
    ) -> Result<Self, RasterError> {
        if !ambient_intensity.is_finite() {
            return Err(RasterError::NonFiniteInput);
        }
        if !(0.0..=MAX_COORD).contains(&ambient_intensity) {
            return Err(RasterError::InvalidLight);
        }
        check_rgb(ambient_color)?;
        directional.dir_light_validate()?;
        Ok(Self {
            ambient_color,
            ambient_intensity,
            directional,
        })
    }

    /// Default studio lighting: 25% neutral ambient + 75% key light from `(0, 1, 1)` normalized.
    pub fn light_rig_default() -> Self {
        let inv_sqrt2 = std::f64::consts::FRAC_1_SQRT_2;
        Self {
            ambient_color: [1.0, 1.0, 1.0],
            ambient_intensity: 0.25,
            directional: DirectionalLight {
                direction_to_light: Vec3::new(0.0, inv_sqrt2, inv_sqrt2),
                color: [1.0, 1.0, 1.0],
                intensity: 0.75,
            },
        }
    }

    /// Full-bright unlit rig (`ambient = 1.0`, `directional = 0.0`) so vertex/texture colors render unmodified.
    pub const fn light_rig_unlit() -> Self {
        Self {
            ambient_color: [1.0, 1.0, 1.0],
            ambient_intensity: 1.0,
            directional: DirectionalLight {
                direction_to_light: Vec3::unit_y(),
                color: [1.0, 1.0, 1.0],
                intensity: 0.0,
            },
        }
    }

    /// Validate all fields of this light rig and return the normalized directional light vector.
    #[inline]
    pub fn light_rig_validate(&self) -> Result<Vec3, RasterError> {
        if !self.ambient_intensity.is_finite() {
            return Err(RasterError::NonFiniteInput);
        }
        if !(0.0..=MAX_COORD).contains(&self.ambient_intensity) {
            return Err(RasterError::InvalidLight);
        }
        check_rgb(self.ambient_color)?;
        self.directional.dir_light_validate()
    }
}

impl Default for LightRig {
    fn default() -> Self {
        Self::light_rig_default()
    }
}

/// A sampled texel in normalized `[0.0, 1.0]` RGBA floating-point components.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SampledTexel {
    /// Red channel in `[0.0, 1.0]`.
    pub r: f64,
    /// Green channel in `[0.0, 1.0]`.
    pub g: f64,
    /// Blue channel in `[0.0, 1.0]`.
    pub b: f64,
    /// Alpha channel in `[0.0, 1.0]`.
    pub a: f64,
}

/// Borrowed immutable RGBA8 texture view for zero-copy texture sampling.
///
/// Texture coordinates follow top-left image origin: `(u=0, v=0)` maps to the
/// top-left texel `(0, 0)` and `(u=1, v=1)` maps to the bottom-right edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Texture<'a> {
    width: u32,
    height: u32,
    pixels: &'a [u8],
}

impl<'a> Texture<'a> {
    /// Create a validated [`Texture`] from raw row-major RGBA8 bytes.
    pub fn texture_from_rgba8(
        width: u32,
        height: u32,
        pixels: &'a [u8],
    ) -> Result<Self, RasterError> {
        validate_texture_dim(width)?;
        validate_texture_dim(height)?;
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or(RasterError::InvalidDimensions)?;
        if pixels.len() != expected {
            return Err(RasterError::InvalidDimensions);
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// Create a validated [`Texture`] borrowing a [`PixelContainer`].
    #[inline]
    pub fn texture_from_container(container: &'a PixelContainer) -> Result<Self, RasterError> {
        Self::texture_from_rgba8(container.width, container.height, &container.data)
    }

    /// Texture width in texels.
    #[inline]
    pub const fn texture_width(&self) -> u32 {
        self.width
    }

    /// Texture height in texels.
    #[inline]
    pub const fn texture_height(&self) -> u32 {
        self.height
    }

    #[inline]
    fn fetch_texel_rgba8(&self, tx: usize, ty: usize) -> Rgba8 {
        let idx = ty
            .saturating_mul(self.width as usize)
            .saturating_add(tx)
            .saturating_mul(4);
        match self.pixels.get(idx..idx + 4) {
            Some(s) => [s[0], s[1], s[2], s[3]],
            None => [0, 0, 0, 255],
        }
    }

    /// Sample the texture at `(u, v)` using nearest-neighbor lookup.
    pub fn texture_sample_nearest(
        &self,
        u: f64,
        v: f64,
        wrap: TextureWrap,
    ) -> Result<Rgba8, RasterError> {
        check_finite_f64(u)?;
        check_finite_f64(v)?;
        let uw = wrap_uv_scalar(u, wrap);
        let vw = wrap_uv_scalar(v, wrap);
        let max_x = (self.width as i64) - 1;
        let max_y = (self.height as i64) - 1;
        let tx = ((uw * f64::from(self.width)).floor() as i64).clamp(0, max_x) as usize;
        let ty = ((vw * f64::from(self.height)).floor() as i64).clamp(0, max_y) as usize;
        Ok(self.fetch_texel_rgba8(tx, ty))
    }

    /// Sample the texture at `(u, v)` using 2x2 bilinear filtering.
    pub fn texture_sample_bilinear(
        &self,
        u: f64,
        v: f64,
        wrap: TextureWrap,
    ) -> Result<SampledTexel, RasterError> {
        check_finite_f64(u)?;
        check_finite_f64(v)?;
        let uw = wrap_uv_scalar(u, wrap);
        let vw = wrap_uv_scalar(v, wrap);

        let w_f = f64::from(self.width);
        let h_f = f64::from(self.height);
        let fx = uw * w_f - 0.5;
        let fy = vw * h_f - 0.5;
        let x0_f = fx.floor();
        let y0_f = fy.floor();
        let frac_x = (fx - x0_f).clamp(0.0, 1.0);
        let frac_y = (fy - y0_f).clamp(0.0, 1.0);

        let x0_i = x0_f as i64;
        let y0_i = y0_f as i64;
        let x1_i = x0_i.saturating_add(1);
        let y1_i = y0_i.saturating_add(1);

        let w_i = i64::from(self.width);
        let h_i = i64::from(self.height);
        let (x0, x1, y0, y1) = match wrap {
            TextureWrap::Repeat => (
                x0_i.rem_euclid(w_i) as usize,
                x1_i.rem_euclid(w_i) as usize,
                y0_i.rem_euclid(h_i) as usize,
                y1_i.rem_euclid(h_i) as usize,
            ),
            TextureWrap::Clamp => (
                x0_i.clamp(0, w_i - 1) as usize,
                x1_i.clamp(0, w_i - 1) as usize,
                y0_i.clamp(0, h_i - 1) as usize,
                y1_i.clamp(0, h_i - 1) as usize,
            ),
        };

        let c00 = self.fetch_texel_rgba8(x0, y0);
        let c10 = self.fetch_texel_rgba8(x1, y0);
        let c01 = self.fetch_texel_rgba8(x0, y1);
        let c11 = self.fetch_texel_rgba8(x1, y1);

        let inv_255 = 1.0 / 255.0;
        let wx0 = 1.0 - frac_x;
        let wy0 = 1.0 - frac_y;
        let w00 = wx0 * wy0;
        let w10 = frac_x * wy0;
        let w01 = wx0 * frac_y;
        let w11 = frac_x * frac_y;

        let bilerp = |k: usize| -> f64 {
            (f64::from(c00[k]) * w00
                + f64::from(c10[k]) * w10
                + f64::from(c01[k]) * w01
                + f64::from(c11[k]) * w11)
                * inv_255
        };

        Ok(SampledTexel {
            r: bilerp(0),
            g: bilerp(1),
            b: bilerp(2),
            a: bilerp(3),
        })
    }
}

#[inline]
fn wrap_uv_scalar(coord: f64, wrap: TextureWrap) -> f64 {
    match wrap {
        TextureWrap::Repeat => {
            let f = coord - coord.floor();
            if !(0.0..1.0).contains(&f) {
                0.0
            } else {
                f
            }
        }
        TextureWrap::Clamp => coord.clamp(0.0, 1.0),
    }
}

/// Caller-provided mutable Z-buffer slice supporting both `f64` and `f32` precision.
pub enum DepthBufferMut<'a> {
    /// 64-bit floating-point depth buffer (`width * height` elements).
    F64(&'a mut [f64]),
    /// 32-bit floating-point depth buffer (`width * height` elements).
    F32(&'a mut [f32]),
}

impl<'a> DepthBufferMut<'a> {
    /// Number of depth elements in this Z-buffer slice.
    #[inline]
    pub fn depth_mut_len(&self) -> usize {
        match self {
            Self::F64(s) => s.len(),
            Self::F32(s) => s.len(),
        }
    }

    /// Fill the entire Z-buffer with `clear_depth` (typically `1.0`).
    pub fn depth_mut_clear(&mut self, clear_depth: f64) -> Result<(), RasterError> {
        check_finite_f64(clear_depth)?;
        match self {
            Self::F64(s) => s.fill(clear_depth),
            Self::F32(s) => {
                let d32 = clear_depth as f32;
                if !d32.is_finite() {
                    return Err(RasterError::ValueOutOfBounds);
                }
                s.fill(d32);
            }
        }
        Ok(())
    }

    /// Read the depth value at pixel index `idx`, returning `None` if out of bounds.
    #[inline]
    pub fn depth_mut_get(&self, idx: usize) -> Option<f64> {
        match self {
            Self::F64(s) => s.get(idx).copied(),
            Self::F32(s) => s.get(idx).map(|&v| f64::from(v)),
        }
    }

    #[inline]
    fn test_and_maybe_write(
        &mut self,
        idx: usize,
        depth: f64,
        depth_test: bool,
        depth_write: bool,
        func: DepthFunc,
    ) -> bool {
        match self {
            Self::F64(s) => {
                let Some(slot) = s.get_mut(idx) else {
                    return false;
                };
                let pass = if !depth_test {
                    true
                } else {
                    match func {
                        DepthFunc::Less => depth < *slot,
                        DepthFunc::LessEqual => depth <= *slot,
                    }
                };
                if pass && depth_write {
                    *slot = depth;
                }
                pass
            }
            Self::F32(s) => {
                let Some(slot) = s.get_mut(idx) else {
                    return false;
                };
                let d32 = depth as f32;
                let pass = if !depth_test {
                    true
                } else {
                    match func {
                        DepthFunc::Less => d32 < *slot,
                        DepthFunc::LessEqual => d32 <= *slot,
                    }
                };
                if pass && depth_write {
                    *slot = d32;
                }
                pass
            }
        }
    }
}

/// Complete configuration for a software 3D rasterization pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RasterConfig {
    /// Model-View-Projection matrix transforming object-space positions to clip space.
    pub mvp: Mat4,
    /// 3x3 normal transformation matrix (identity when vertices/normals are already world-space).
    pub normal_matrix: Mat3,
    /// Triangle shading and texturing mode.
    pub shading: ShadingMode,
    /// Backface / frontface culling mode.
    pub cull: CullMode,
    /// Winding order defining front-facing triangles.
    pub front_face: FrontFace,
    /// Texture wrap mode outside `[0, 1]`.
    pub texture_wrap: TextureWrap,
    /// Clip-space Z convention (`OpenGl` `[-1, 1]` or `ZeroToOne` `[0, 1]`).
    pub clip_space: ClipSpace,
    /// Whether fragments are tested against the Z-buffer.
    pub depth_test: bool,
    /// Whether passing fragments update the Z-buffer.
    pub depth_write: bool,
    /// Depth comparison function.
    pub depth_func: DepthFunc,
    /// Ambient + directional lighting rig.
    pub light: LightRig,
}

impl RasterConfig {
    /// Construct a default [`RasterConfig`] for the given `mvp` matrix.
    pub fn raster_config_new(mvp: Mat4) -> Result<Self, RasterError> {
        check_mat4_finite(&mvp)?;
        Ok(Self {
            mvp,
            normal_matrix: Mat3::mat3_identity(),
            shading: ShadingMode::Gouraud,
            cull: CullMode::Back,
            front_face: FrontFace::CounterClockwise,
            texture_wrap: TextureWrap::Repeat,
            clip_space: ClipSpace::OpenGl,
            depth_test: true,
            depth_write: true,
            depth_func: DepthFunc::Less,
            light: LightRig::light_rig_default(),
        })
    }

    /// Validate all matrices and lighting fields of this configuration (CWE-20 / CWE-369).
    pub fn raster_config_validate(&self) -> Result<Vec3, RasterError> {
        check_mat4_finite(&self.mvp)?;
        check_mat3_finite(&self.normal_matrix)?;
        self.light.light_rig_validate()
    }
}

/// Statistics produced by [`draw_triangle_into`] or [`draw_indexed_mesh_into`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RasterStats {
    /// Total triangles submitted to the pipeline.
    pub triangles_submitted: u64,
    /// Triangles discarded by backface/frontface culling or degenerate screen area.
    pub triangles_culled: u64,
    /// Triangles completely rejected by frustum / near-plane clipping.
    pub triangles_clipped_out: u64,
    /// Clipped triangles that entered screen-space rasterization.
    pub triangles_rasterized: u64,
    /// Individual pixel writes passing depth test and written to the framebuffer.
    pub pixels_written: u64,
}

// ---------------------------------------------------------------------------
// Scratch Buffer & Internal Vertex Representations
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct TransformedVertex {
    clip: Vec4,
    world_pos: Vec3,
    world_normal: Vec3,
    base_color: RgbColor,
    lit_color: RgbColor,
    u: f64,
    v: f64,
}

#[derive(Clone, Copy, Debug)]
struct ClipVertex {
    clip: Vec4,
    color: RgbColor,
    normal: Vec3,
    u: f64,
    v: f64,
}

impl Default for ClipVertex {
    fn default() -> Self {
        Self {
            clip: Vec4::from_xyzw(0.0, 0.0, 0.0, 1.0),
            color: [0.0, 0.0, 0.0],
            normal: Vec3::zero(),
            u: 0.0,
            v: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ScreenVertex {
    sx: f64,
    sy: f64,
    depth: f64,
    inv_w: f64,
    r_over_w: f64,
    g_over_w: f64,
    b_over_w: f64,
    u_over_w: f64,
    v_over_w: f64,
    nx_over_w: f64,
    ny_over_w: f64,
    nz_over_w: f64,
}

/// Pre-allocated reusable scratch buffer for zero-allocation steady-state mesh rendering.
#[derive(Clone, Debug, Default)]
pub struct RasterScratch {
    transformed: Vec<TransformedVertex>,
}

impl RasterScratch {
    /// Create an empty [`RasterScratch`].
    #[inline]
    pub fn scratch_new() -> Self {
        Self {
            transformed: Vec::new(),
        }
    }

    /// Create a [`RasterScratch`] pre-allocated for up to `vertex_capacity` vertices (CWE-400 bounded).
    pub fn scratch_with_capacity(vertex_capacity: usize) -> Result<Self, RasterError> {
        if vertex_capacity > MAX_VERTICES {
            return Err(RasterError::MeshTooLarge);
        }
        Ok(Self {
            transformed: Vec::with_capacity(vertex_capacity),
        })
    }

    /// Current vertex capacity before any heap reallocation would be needed.
    #[inline]
    pub fn scratch_capacity(&self) -> usize {
        self.transformed.capacity()
    }

    /// Clear transient vertex state while retaining allocated capacity.
    #[inline]
    pub fn scratch_clear(&mut self) {
        self.transformed.clear();
    }
}

// ---------------------------------------------------------------------------
// Clearing & Canonical Frame Digest
// ---------------------------------------------------------------------------

/// Validate a target [`PixelContainer`] and Z-buffer length, returning `(width, height)`.
fn validate_render_targets(
    framebuffer: &PixelContainer,
    zbuffer_len: usize,
) -> Result<(u32, u32), RasterError> {
    validate_framebuffer_dim(framebuffer.width)?;
    validate_framebuffer_dim(framebuffer.height)?;
    let pixel_count = (framebuffer.width as usize)
        .checked_mul(framebuffer.height as usize)
        .ok_or(RasterError::InvalidDimensions)?;
    let expected_bytes = pixel_count
        .checked_mul(4)
        .ok_or(RasterError::InvalidDimensions)?;
    if framebuffer.data.len() != expected_bytes || zbuffer_len != pixel_count {
        return Err(RasterError::InvalidDimensions);
    }
    Ok((framebuffer.width, framebuffer.height))
}

/// Clear an entire [`PixelContainer`] to a uniform `Rgba8` color without allocation.
pub fn clear_framebuffer(framebuffer: &mut PixelContainer, rgba: Rgba8) -> Result<(), RasterError> {
    validate_framebuffer_dim(framebuffer.width)?;
    validate_framebuffer_dim(framebuffer.height)?;
    let expected = (framebuffer.width as usize)
        .checked_mul(framebuffer.height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or(RasterError::InvalidDimensions)?;
    if framebuffer.data.len() != expected {
        return Err(RasterError::InvalidDimensions);
    }
    let (pixels, rem) = framebuffer.data.as_chunks_mut::<4>();
    if !rem.is_empty() {
        return Err(RasterError::InvalidDimensions);
    }
    for px in pixels {
        *px = rgba;
    }
    Ok(())
}

/// Clear a caller-provided `f64` Z-buffer slice to `depth` (typically `1.0`).
pub fn clear_depth_f64(zbuffer: &mut [f64], depth: f64) -> Result<(), RasterError> {
    check_finite_f64(depth)?;
    zbuffer.fill(depth);
    Ok(())
}

/// Clear a caller-provided `f32` Z-buffer slice to `depth` (typically `1.0`).
pub fn clear_depth_f32(zbuffer: &mut [f32], depth: f32) -> Result<(), RasterError> {
    if !depth.is_finite() {
        return Err(RasterError::NonFiniteInput);
    }
    zbuffer.fill(depth);
    Ok(())
}

/// Compute a canonical 64-bit FNV-1a digest over framebuffer dimensions and RGBA8 pixels.
///
/// Used for bit-exact deterministic regression and cross-backend verification.
pub fn frame_digest_fnv1a(framebuffer: &PixelContainer) -> u64 {
    let mut h = FNV1A_OFFSET_BASIS;
    for b in framebuffer.width.to_le_bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(FNV1A_PRIME);
    }
    for b in framebuffer.height.to_le_bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(FNV1A_PRIME);
    }
    for &b in &framebuffer.data {
        h ^= u64::from(b);
        h = h.wrapping_mul(FNV1A_PRIME);
    }
    h
}

/// Compute a canonical 64-bit FNV-1a digest over an `f64` Z-buffer slice.
pub fn depth_digest_f64_fnv1a(zbuffer: &[f64]) -> u64 {
    let mut h = FNV1A_OFFSET_BASIS;
    for &d in zbuffer {
        let canonical = if d.is_nan() {
            f64::NAN
        } else if d == 0.0 {
            0.0_f64
        } else {
            d
        };
        for b in canonical.to_le_bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(FNV1A_PRIME);
        }
    }
    h
}

/// Compute a canonical 64-bit FNV-1a digest over an `f32` Z-buffer slice.
pub fn depth_digest_f32_fnv1a(zbuffer: &[f32]) -> u64 {
    let mut h = FNV1A_OFFSET_BASIS;
    for &d in zbuffer {
        let canonical = if d.is_nan() {
            f32::NAN
        } else if d == 0.0 {
            0.0_f32
        } else {
            d
        };
        for b in canonical.to_le_bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(FNV1A_PRIME);
        }
    }
    h
}

// ---------------------------------------------------------------------------
// Lighting & Vertex Transformation
// ---------------------------------------------------------------------------

#[inline]
fn compute_lit_rgb(
    base: RgbColor,
    unit_normal: Vec3,
    light: &LightRig,
    light_dir: Vec3,
) -> RgbColor {
    let ndotl =
        (unit_normal.x * light_dir.x + unit_normal.y * light_dir.y + unit_normal.z * light_dir.z)
            .max(0.0);
    let lr = light.ambient_color[0] * light.ambient_intensity
        + light.directional.color[0] * light.directional.intensity * ndotl;
    let lg = light.ambient_color[1] * light.ambient_intensity
        + light.directional.color[1] * light.directional.intensity * ndotl;
    let lb = light.ambient_color[2] * light.ambient_intensity
        + light.directional.color[2] * light.directional.intensity * ndotl;
    [
        (base[0] * lr).clamp(0.0, 1.0),
        (base[1] * lg).clamp(0.0, 1.0),
        (base[2] * lb).clamp(0.0, 1.0),
    ]
}

#[inline]
fn normalize_or_zero(v: Vec3) -> Result<Vec3, RasterError> {
    let len_sq = v.length_sq()?;
    if len_sq <= math3d::EPSILON * math3d::EPSILON {
        Ok(Vec3::zero())
    } else {
        Ok(v.normalize()?)
    }
}

fn transform_single_vertex(
    v: &Vertex,
    config: &RasterConfig,
    light_dir: Vec3,
) -> Result<TransformedVertex, RasterError> {
    v.vertex_validate()?;
    let clip = config
        .mvp
        .mat4_mul_vec4(Vec4::vec4_from_vec3(v.position, 1.0))?;
    let transformed_n = config.normal_matrix.mat3_mul_vec3(v.normal)?;
    let world_normal = normalize_or_zero(transformed_n)?;
    let lit_color = compute_lit_rgb(v.color, world_normal, &config.light, light_dir);
    Ok(TransformedVertex {
        clip,
        world_pos: v.position,
        world_normal,
        base_color: v.color,
        lit_color,
        u: v.u,
        v: v.v,
    })
}

// ---------------------------------------------------------------------------
// Homogeneous 4D Sutherland-Hodgman Clipping
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum ClipPlane {
    NearW,
    Left,
    Right,
    Bottom,
    Top,
    NearOpenGl,
    NearZeroToOne,
    Far,
}

#[inline]
fn plane_signed_distance(p: ClipPlane, v: &Vec4) -> f64 {
    match p {
        ClipPlane::NearW => v.w - MIN_CLIP_W,
        ClipPlane::Left => v.x + v.w,
        ClipPlane::Right => v.w - v.x,
        ClipPlane::Bottom => v.y + v.w,
        ClipPlane::Top => v.w - v.y,
        ClipPlane::NearOpenGl => v.z + v.w,
        ClipPlane::NearZeroToOne => v.z,
        ClipPlane::Far => v.w - v.z,
    }
}

#[inline]
fn lerp_clip_vertex(a: &ClipVertex, b: &ClipVertex, t: f64) -> ClipVertex {
    let s = if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.5
    };
    let u_s = 1.0 - s;
    ClipVertex {
        clip: Vec4::from_xyzw(
            a.clip.x * u_s + b.clip.x * s,
            a.clip.y * u_s + b.clip.y * s,
            a.clip.z * u_s + b.clip.z * s,
            a.clip.w * u_s + b.clip.w * s,
        ),
        color: [
            a.color[0] * u_s + b.color[0] * s,
            a.color[1] * u_s + b.color[1] * s,
            a.color[2] * u_s + b.color[2] * s,
        ],
        normal: Vec3::new(
            a.normal.x * u_s + b.normal.x * s,
            a.normal.y * u_s + b.normal.y * s,
            a.normal.z * u_s + b.normal.z * s,
        ),
        u: a.u * u_s + b.u * s,
        v: a.v * u_s + b.v * s,
    }
}

fn clip_polygon_against_plane(
    input: &[ClipVertex; MAX_CLIP_VERTICES],
    in_len: usize,
    output: &mut [ClipVertex; MAX_CLIP_VERTICES],
    plane: ClipPlane,
) -> usize {
    if in_len == 0 {
        return 0;
    }
    let mut out_len = 0usize;
    let mut prev = input[in_len - 1];
    let mut prev_d = plane_signed_distance(plane, &prev.clip);

    for curr in input.iter().take(in_len) {
        let curr_d = plane_signed_distance(plane, &curr.clip);
        let prev_in = prev_d >= 0.0;
        let curr_in = curr_d >= 0.0;

        if curr_in != prev_in {
            let denom = prev_d - curr_d;
            let t = if !denom.is_finite() || denom.abs() <= 1e-15 {
                0.5
            } else {
                let ratio = prev_d / denom;
                if ratio.is_finite() {
                    ratio
                } else {
                    0.5
                }
            };
            if out_len < MAX_CLIP_VERTICES {
                output[out_len] = lerp_clip_vertex(&prev, curr, t);
                out_len += 1;
            }
        }
        if curr_in && out_len < MAX_CLIP_VERTICES {
            output[out_len] = *curr;
            out_len += 1;
        }
        prev = *curr;
        prev_d = curr_d;
    }
    out_len
}

/// Outcode bits for fast trivial accept / trivial reject in homogeneous clip space.
#[inline]
fn clip_outcode(v: &Vec4, clip_space: ClipSpace) -> u8 {
    let mut code = 0u8;
    if v.w < MIN_CLIP_W {
        code |= 1 << 0;
    }
    if v.x < -v.w {
        code |= 1 << 1;
    }
    if v.x > v.w {
        code |= 1 << 2;
    }
    if v.y < -v.w {
        code |= 1 << 3;
    }
    if v.y > v.w {
        code |= 1 << 4;
    }
    let near_out = match clip_space {
        ClipSpace::OpenGl => v.z < -v.w,
        ClipSpace::ZeroToOne => v.z < 0.0,
    };
    if near_out {
        code |= 1 << 5;
    }
    if v.z > v.w {
        code |= 1 << 6;
    }
    code
}

// ---------------------------------------------------------------------------
// Screen Projection & Half-Space Triangle Rasterizer
// ---------------------------------------------------------------------------

#[inline]
fn project_to_screen(
    cv: &ClipVertex,
    width: u32,
    height: u32,
    clip_space: ClipSpace,
) -> Option<ScreenVertex> {
    if !cv.clip.vec4_is_finite() || cv.clip.w < MIN_CLIP_W {
        return None;
    }
    let inv_w = 1.0 / cv.clip.w;
    if !inv_w.is_finite() {
        return None;
    }
    let ndc_x = cv.clip.x * inv_w;
    let ndc_y = cv.clip.y * inv_w;
    let ndc_z = cv.clip.z * inv_w;
    if !ndc_x.is_finite() || !ndc_y.is_finite() || !ndc_z.is_finite() {
        return None;
    }

    let sx = (ndc_x + 1.0) * 0.5 * f64::from(width);
    let sy = (1.0 - ndc_y) * 0.5 * f64::from(height);
    let depth = match clip_space {
        ClipSpace::OpenGl => (ndc_z + 1.0) * 0.5,
        ClipSpace::ZeroToOne => ndc_z,
    }
    .clamp(0.0, 1.0);

    Some(ScreenVertex {
        sx,
        sy,
        depth,
        inv_w,
        r_over_w: cv.color[0] * inv_w,
        g_over_w: cv.color[1] * inv_w,
        b_over_w: cv.color[2] * inv_w,
        u_over_w: cv.u * inv_w,
        v_over_w: cv.v * inv_w,
        nx_over_w: cv.normal.x * inv_w,
        ny_over_w: cv.normal.y * inv_w,
        nz_over_w: cv.normal.z * inv_w,
    })
}

#[inline]
fn edge_function(ax: f64, ay: f64, bx: f64, by: f64, px: f64, py: f64) -> f64 {
    (bx - ax) * (py - ay) - (by - ay) * (px - ax)
}

#[inline]
fn is_top_left_edge(ax: f64, ay: f64, bx: f64, by: f64) -> bool {
    let dx = bx - ax;
    let dy = by - ay;
    dy < 0.0 || (dy == 0.0 && dx > 0.0)
}

#[allow(clippy::too_many_arguments)]
fn rasterize_screen_triangle(
    framebuffer: &mut PixelContainer,
    zbuffer: &mut DepthBufferMut<'_>,
    config: &RasterConfig,
    texture: Option<&Texture<'_>>,
    light_dir: Vec3,
    sv0: ScreenVertex,
    mut sv1: ScreenVertex,
    mut sv2: ScreenVertex,
    stats: &mut RasterStats,
) {
    let signed_area_2x = edge_function(sv0.sx, sv0.sy, sv1.sx, sv1.sy, sv2.sx, sv2.sy);
    if !signed_area_2x.is_finite() || signed_area_2x.abs() <= AREA_EPSILON {
        stats.triangles_culled += 1;
        return;
    }

    // Because Y is flipped in viewport transform (Y-down screen space), a CCW
    // triangle in NDC has negative `signed_area_2x` and a CW triangle has
    // positive `signed_area_2x`.
    let is_front = match config.front_face {
        FrontFace::CounterClockwise => signed_area_2x < 0.0,
        FrontFace::Clockwise => signed_area_2x > 0.0,
    };
    let culled = match config.cull {
        CullMode::None => false,
        CullMode::Back => !is_front,
        CullMode::Front => is_front,
    };
    if culled {
        stats.triangles_culled += 1;
        return;
    }

    // Normalize winding so `area_2x > 0` for uniform half-space inside tests.
    let area_2x = if signed_area_2x < 0.0 {
        std::mem::swap(&mut sv1, &mut sv2);
        -signed_area_2x
    } else {
        signed_area_2x
    };
    let inv_area = 1.0 / area_2x;
    if !inv_area.is_finite() {
        stats.triangles_culled += 1;
        return;
    }

    let width = framebuffer.width;
    let height = framebuffer.height;
    let max_wf = f64::from(width - 1);
    let max_hf = f64::from(height - 1);

    let min_xf = sv0.sx.min(sv1.sx).min(sv2.sx).floor();
    let max_xf = sv0.sx.max(sv1.sx).max(sv2.sx).ceil();
    let min_yf = sv0.sy.min(sv1.sy).min(sv2.sy).floor();
    let max_yf = sv0.sy.max(sv1.sy).max(sv2.sy).ceil();

    if !min_xf.is_finite()
        || !max_xf.is_finite()
        || !min_yf.is_finite()
        || !max_yf.is_finite()
        || max_xf < 0.0
        || min_xf > max_wf
        || max_yf < 0.0
        || min_yf > max_hf
    {
        stats.triangles_clipped_out += 1;
        return;
    }

    // CWE-190: clamp in f64 inside [0, width-1] / [0, height-1] before integer conversion.
    let min_x = min_xf.clamp(0.0, max_wf) as i64 as u32;
    let max_x = max_xf.clamp(0.0, max_wf) as i64 as u32;
    let min_y = min_yf.clamp(0.0, max_hf) as i64 as u32;
    let max_y = max_yf.clamp(0.0, max_hf) as i64 as u32;

    if min_x > max_x || min_y > max_y {
        stats.triangles_clipped_out += 1;
        return;
    }

    let tl0 = is_top_left_edge(sv1.sx, sv1.sy, sv2.sx, sv2.sy);
    let tl1 = is_top_left_edge(sv2.sx, sv2.sy, sv0.sx, sv0.sy);
    let tl2 = is_top_left_edge(sv0.sx, sv0.sy, sv1.sx, sv1.sy);

    stats.triangles_rasterized += 1;
    let stride = width as usize;
    let (pixels, _) = framebuffer.data.as_chunks_mut::<4>();

    for y in min_y..=max_y {
        let py = f64::from(y) + 0.5;
        let row_base = (y as usize).saturating_mul(stride);
        for x in min_x..=max_x {
            let px = f64::from(x) + 0.5;
            let e0 = edge_function(sv1.sx, sv1.sy, sv2.sx, sv2.sy, px, py);
            let e1 = edge_function(sv2.sx, sv2.sy, sv0.sx, sv0.sy, px, py);
            let e2 = edge_function(sv0.sx, sv0.sy, sv1.sx, sv1.sy, px, py);

            let in0 = if tl0 { e0 >= 0.0 } else { e0 > 0.0 };
            let in1 = if tl1 { e1 >= 0.0 } else { e1 > 0.0 };
            let in2 = if tl2 { e2 >= 0.0 } else { e2 > 0.0 };
            if !(in0 && in1 && in2) {
                continue;
            }

            let b0 = e0 * inv_area;
            let b1 = e1 * inv_area;
            let b2 = e2 * inv_area;

            let one_over_w = b0 * sv0.inv_w + b1 * sv1.inv_w + b2 * sv2.inv_w;
            if !one_over_w.is_finite() || one_over_w <= 1e-12 {
                continue;
            }
            let w_interp = 1.0 / one_over_w;

            let depth = (b0 * sv0.depth + b1 * sv1.depth + b2 * sv2.depth).clamp(0.0, 1.0);
            let pixel_idx = row_base.saturating_add(x as usize);
            if !zbuffer.test_and_maybe_write(
                pixel_idx,
                depth,
                config.depth_test,
                config.depth_write,
                config.depth_func,
            ) {
                continue;
            }

            let r = (b0 * sv0.r_over_w + b1 * sv1.r_over_w + b2 * sv2.r_over_w) * w_interp;
            let g = (b0 * sv0.g_over_w + b1 * sv1.g_over_w + b2 * sv2.g_over_w) * w_interp;
            let b = (b0 * sv0.b_over_w + b1 * sv1.b_over_w + b2 * sv2.b_over_w) * w_interp;

            let rgba: Rgba8 = match config.shading {
                ShadingMode::Flat | ShadingMode::Gouraud => {
                    [float_to_u8(r), float_to_u8(g), float_to_u8(b), 255]
                }
                ShadingMode::Phong => {
                    let nx =
                        (b0 * sv0.nx_over_w + b1 * sv1.nx_over_w + b2 * sv2.nx_over_w) * w_interp;
                    let ny =
                        (b0 * sv0.ny_over_w + b1 * sv1.ny_over_w + b2 * sv2.ny_over_w) * w_interp;
                    let nz =
                        (b0 * sv0.nz_over_w + b1 * sv1.nz_over_w + b2 * sv2.nz_over_w) * w_interp;
                    let unit_n =
                        normalize_or_zero(Vec3::new(nx, ny, nz)).unwrap_or_else(|_| Vec3::zero());
                    let lit = compute_lit_rgb([r, g, b], unit_n, &config.light, light_dir);
                    [
                        float_to_u8(lit[0]),
                        float_to_u8(lit[1]),
                        float_to_u8(lit[2]),
                        255,
                    ]
                }
                ShadingMode::TexturedNearest => {
                    let u = (b0 * sv0.u_over_w + b1 * sv1.u_over_w + b2 * sv2.u_over_w) * w_interp;
                    let v = (b0 * sv0.v_over_w + b1 * sv1.v_over_w + b2 * sv2.v_over_w) * w_interp;
                    if let Some(tex) = texture {
                        if let Ok(texel) = tex.texture_sample_nearest(u, v, config.texture_wrap) {
                            let tr = f64::from(texel[0]) / 255.0;
                            let tg = f64::from(texel[1]) / 255.0;
                            let tb = f64::from(texel[2]) / 255.0;
                            [
                                float_to_u8(tr * r),
                                float_to_u8(tg * g),
                                float_to_u8(tb * b),
                                texel[3],
                            ]
                        } else {
                            [float_to_u8(r), float_to_u8(g), float_to_u8(b), 255]
                        }
                    } else {
                        [float_to_u8(r), float_to_u8(g), float_to_u8(b), 255]
                    }
                }
                ShadingMode::TexturedBilinear => {
                    let u = (b0 * sv0.u_over_w + b1 * sv1.u_over_w + b2 * sv2.u_over_w) * w_interp;
                    let v = (b0 * sv0.v_over_w + b1 * sv1.v_over_w + b2 * sv2.v_over_w) * w_interp;
                    if let Some(tex) = texture {
                        if let Ok(texel) = tex.texture_sample_bilinear(u, v, config.texture_wrap) {
                            [
                                float_to_u8(texel.r * r),
                                float_to_u8(texel.g * g),
                                float_to_u8(texel.b * b),
                                float_to_u8(texel.a),
                            ]
                        } else {
                            [float_to_u8(r), float_to_u8(g), float_to_u8(b), 255]
                        }
                    } else {
                        [float_to_u8(r), float_to_u8(g), float_to_u8(b), 255]
                    }
                }
            };

            if let Some(dst) = pixels.get_mut(pixel_idx) {
                *dst = rgba;
                stats.pixels_written += 1;
            }
        }
    }
}

fn compute_flat_face_normal(
    t0: &TransformedVertex,
    t1: &TransformedVertex,
    t2: &TransformedVertex,
    normal_matrix: &Mat3,
) -> Vec3 {
    let avg_fallback = || -> Vec3 {
        let sum = Vec3::new(
            t0.world_normal.x + t1.world_normal.x + t2.world_normal.x,
            t0.world_normal.y + t1.world_normal.y + t2.world_normal.y,
            t0.world_normal.z + t1.world_normal.z + t2.world_normal.z,
        );
        normalize_or_zero(sum)
            .unwrap_or_else(|_| normalize_or_zero(t0.world_normal).unwrap_or_else(|_| Vec3::zero()))
    };
    let Ok(e1) = t1.world_pos.vec3_sub(t0.world_pos) else {
        return avg_fallback();
    };
    let Ok(e2) = t2.world_pos.vec3_sub(t0.world_pos) else {
        return avg_fallback();
    };
    let Ok(cross) = e1.cross(e2) else {
        return avg_fallback();
    };
    let Ok(unit_cross) = cross.normalize() else {
        return avg_fallback();
    };
    let Ok(raw_face) = normal_matrix.mat3_mul_vec3(unit_cross) else {
        return avg_fallback();
    };
    normalize_or_zero(raw_face).unwrap_or_else(|_| avg_fallback())
}

#[allow(clippy::too_many_arguments)]
fn clip_and_rasterize_triangle(
    framebuffer: &mut PixelContainer,
    zbuffer: &mut DepthBufferMut<'_>,
    config: &RasterConfig,
    texture: Option<&Texture<'_>>,
    light_dir: Vec3,
    t0: &TransformedVertex,
    t1: &TransformedVertex,
    t2: &TransformedVertex,
    stats: &mut RasterStats,
) {
    stats.triangles_submitted += 1;

    let o0 = clip_outcode(&t0.clip, config.clip_space);
    let o1 = clip_outcode(&t1.clip, config.clip_space);
    let o2 = clip_outcode(&t2.clip, config.clip_space);

    // Trivial reject if all 3 vertices lie outside the same frustum half-space.
    if (o0 & o1 & o2) != 0 {
        stats.triangles_clipped_out += 1;
        return;
    }

    let (c0, c1, c2) = match config.shading {
        ShadingMode::Flat => {
            let face_normal = compute_flat_face_normal(t0, t1, t2, &config.normal_matrix);
            let avg_color = [
                (t0.base_color[0] + t1.base_color[0] + t2.base_color[0]) / 3.0,
                (t0.base_color[1] + t1.base_color[1] + t2.base_color[1]) / 3.0,
                (t0.base_color[2] + t1.base_color[2] + t2.base_color[2]) / 3.0,
            ];
            let flat = compute_lit_rgb(avg_color, face_normal, &config.light, light_dir);
            (flat, flat, flat)
        }
        ShadingMode::Phong => (t0.base_color, t1.base_color, t2.base_color),
        ShadingMode::Gouraud | ShadingMode::TexturedNearest | ShadingMode::TexturedBilinear => {
            (t0.lit_color, t1.lit_color, t2.lit_color)
        }
    };

    let v0 = ClipVertex {
        clip: t0.clip,
        color: c0,
        normal: t0.world_normal,
        u: t0.u,
        v: t0.v,
    };
    let v1 = ClipVertex {
        clip: t1.clip,
        color: c1,
        normal: t1.world_normal,
        u: t1.u,
        v: t1.v,
    };
    let v2 = ClipVertex {
        clip: t2.clip,
        color: c2,
        normal: t2.world_normal,
        u: t2.u,
        v: t2.v,
    };

    let width = framebuffer.width;
    let height = framebuffer.height;

    // Fast path: trivial accept when all 3 vertices lie completely inside the frustum.
    if (o0 | o1 | o2) == 0 {
        if let (Some(sv0), Some(sv1), Some(sv2)) = (
            project_to_screen(&v0, width, height, config.clip_space),
            project_to_screen(&v1, width, height, config.clip_space),
            project_to_screen(&v2, width, height, config.clip_space),
        ) {
            rasterize_screen_triangle(
                framebuffer,
                zbuffer,
                config,
                texture,
                light_dir,
                sv0,
                sv1,
                sv2,
                stats,
            );
        } else {
            stats.triangles_clipped_out += 1;
        }
        return;
    }

    // Sutherland-Hodgman homogeneous 4D clipping across 7 half-spaces on the stack.
    let mut buf_a = [ClipVertex::default(); MAX_CLIP_VERTICES];
    let mut buf_b = [ClipVertex::default(); MAX_CLIP_VERTICES];
    buf_a[0] = v0;
    buf_a[1] = v1;
    buf_a[2] = v2;
    let mut count = 3usize;

    let near_z_plane = match config.clip_space {
        ClipSpace::OpenGl => ClipPlane::NearOpenGl,
        ClipSpace::ZeroToOne => ClipPlane::NearZeroToOne,
    };
    let planes = [
        ClipPlane::NearW,
        ClipPlane::Left,
        ClipPlane::Right,
        ClipPlane::Bottom,
        ClipPlane::Top,
        near_z_plane,
        ClipPlane::Far,
    ];

    for (step, &plane) in planes.iter().enumerate() {
        if step % 2 == 0 {
            count = clip_polygon_against_plane(&buf_a, count, &mut buf_b, plane);
        } else {
            count = clip_polygon_against_plane(&buf_b, count, &mut buf_a, plane);
        }
        if count < 3 {
            stats.triangles_clipped_out += 1;
            return;
        }
    }

    // 7 planes (odd count): final clipped polygon lives in `buf_b`.
    let poly = &buf_b;
    let Some(sv0) = project_to_screen(&poly[0], width, height, config.clip_space) else {
        stats.triangles_clipped_out += 1;
        return;
    };
    for i in 1..(count - 1) {
        if let (Some(sv1), Some(sv2)) = (
            project_to_screen(&poly[i], width, height, config.clip_space),
            project_to_screen(&poly[i + 1], width, height, config.clip_space),
        ) {
            rasterize_screen_triangle(
                framebuffer,
                zbuffer,
                config,
                texture,
                light_dir,
                sv0,
                sv1,
                sv2,
                stats,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Public Rendering Entry Points
// ---------------------------------------------------------------------------

/// Render a single 3D triangle into caller-provided `framebuffer` and `zbuffer`.
///
/// Performs **zero heap allocations**. All clipping uses fixed stack buffers.
pub fn draw_triangle_into(
    framebuffer: &mut PixelContainer,
    mut zbuffer: DepthBufferMut<'_>,
    config: &RasterConfig,
    texture: Option<&Texture<'_>>,
    v0: &Vertex,
    v1: &Vertex,
    v2: &Vertex,
) -> Result<RasterStats, RasterError> {
    validate_render_targets(framebuffer, zbuffer.depth_mut_len())?;
    let light_dir = config.raster_config_validate()?;
    if matches!(
        config.shading,
        ShadingMode::TexturedNearest | ShadingMode::TexturedBilinear
    ) && texture.is_none()
    {
        return Err(RasterError::MissingTexture);
    }
    if let Some(tex) = texture {
        Texture::texture_from_rgba8(tex.width, tex.height, tex.pixels)?;
    }

    let t0 = transform_single_vertex(v0, config, light_dir)?;
    let t1 = transform_single_vertex(v1, config, light_dir)?;
    let t2 = transform_single_vertex(v2, config, light_dir)?;

    let mut stats = RasterStats::default();
    clip_and_rasterize_triangle(
        framebuffer,
        &mut zbuffer,
        config,
        texture,
        light_dir,
        &t0,
        &t1,
        &t2,
        &mut stats,
    );
    Ok(stats)
}

/// Render an indexed triangle mesh into caller-provided `framebuffer` and `zbuffer`.
///
/// When `scratch` has been pre-sized with [`RasterScratch::scratch_with_capacity`]
/// (`capacity >= vertices.len()`), steady-state frame rendering performs **zero
/// heap allocations**.
pub fn draw_indexed_mesh_into(
    framebuffer: &mut PixelContainer,
    mut zbuffer: DepthBufferMut<'_>,
    config: &RasterConfig,
    texture: Option<&Texture<'_>>,
    vertices: &[Vertex],
    indices: &[u32],
    scratch: &mut RasterScratch,
) -> Result<RasterStats, RasterError> {
    validate_render_targets(framebuffer, zbuffer.depth_mut_len())?;
    let light_dir = config.raster_config_validate()?;
    if matches!(
        config.shading,
        ShadingMode::TexturedNearest | ShadingMode::TexturedBilinear
    ) && texture.is_none()
    {
        return Err(RasterError::MissingTexture);
    }
    if let Some(tex) = texture {
        Texture::texture_from_rgba8(tex.width, tex.height, tex.pixels)?;
    }
    if vertices.len() > MAX_VERTICES {
        return Err(RasterError::MeshTooLarge);
    }
    let (tri_chunks, rem) = indices.as_chunks::<3>();
    if !rem.is_empty() {
        return Err(RasterError::InvalidIndexCount);
    }
    validate_triangle_count(tri_chunks.len())?;

    // Validate every vertex index upfront before modifying framebuffer / Z-buffer.
    for tri in tri_chunks {
        validate_vertex_index(tri[0] as usize, vertices.len())?;
        validate_vertex_index(tri[1] as usize, vertices.len())?;
        validate_vertex_index(tri[2] as usize, vertices.len())?;
    }

    scratch.transformed.clear();
    if scratch.transformed.capacity() < vertices.len() {
        scratch.transformed.reserve(vertices.len());
    }

    for v in vertices {
        match transform_single_vertex(v, config, light_dir) {
            Ok(tv) => scratch.transformed.push(tv),
            Err(err) => {
                scratch.transformed.clear();
                return Err(err);
            }
        }
    }

    let mut stats = RasterStats::default();
    for tri in tri_chunks {
        let Some(t0) = scratch.transformed.get(tri[0] as usize) else {
            return Err(RasterError::IndexOutOfBounds);
        };
        let Some(t1) = scratch.transformed.get(tri[1] as usize) else {
            return Err(RasterError::IndexOutOfBounds);
        };
        let Some(t2) = scratch.transformed.get(tri[2] as usize) else {
            return Err(RasterError::IndexOutOfBounds);
        };
        clip_and_rasterize_triangle(
            framebuffer,
            &mut zbuffer,
            config,
            texture,
            light_dir,
            t0,
            t1,
            t2,
            &mut stats,
        );
    }

    Ok(stats)
}
