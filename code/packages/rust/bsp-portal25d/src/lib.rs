//! # bsp-portal25d
//!
//! GAME32: deterministic, zero-allocation (steady-state) 2D sector Binary Space
//! Partitioning (BSP) tree builder, front-to-back portal traversal with a 1D
//! column occlusion clipper (`y_top`, `y_bottom`), dynamic visibility lists,
//! and 2.5D Doom/Duke3D-style sector rendering into caller-provided buffers.
//!
//! Renders directly into a caller-provided [`PixelContainer`] (`IMG00`) using
//! pre-allocated [`RenderScratch`] buffers, bridging Era 3 2.5D portal engines
//! with `math3d` (`GAME30`) and `trig` (`PHY00`).
//!
//! ## Coordinate & Linedef Convention
//!
//! ```text
//!   2D Map Plane (X = East, Y = North), Height = Z (up):
//!
//!       v0 -----------------> v1       Front side is to the RIGHT of (v1 - v0)
//!                 |                    (clockwise around a closed room).
//!                 v  front normal      At yaw = 0, dir = (+1, 0), right = (0, -1).
//!            front_sector
//!
//!   1D Screen Column Occlusion [y_top[x], y_bottom[x]] for x in 0..fb_width:
//!     - Initialized to [0, fb_height - 1]
//!     - Solid 1-sided wall: draws middle texture, sets y_top[x] > y_bottom[x]
//!     - 2-sided portal: draws front ceiling/floor + upper/lower step walls,
//!       shrinks [y_top[x], y_bottom[x]] to the portal opening for far sectors
//!     - When all columns close (open_columns == 0), traversal exits early.
//! ```
//!
//! ## Safety and Resource Model
//!
//! * `#![forbid(unsafe_code)]`; zero pointer arithmetic anywhere.
//! * Steady-state [`render_frame_into`] performs **zero heap allocation**: all
//!   output pixels, 1D column bounds, iterative BSP traversal stack entries,
//!   and dynamic visibility lists live in caller-owned [`PixelContainer`] and
//!   [`RenderScratch`].
//! * Iterative BSP building and traversal (`CWE-674`): neither [`BspTree::bsp_build`]
//!   nor [`render_frame_into`] recurses on the call stack, and tree depth is
//!   strictly capped at [`MAX_BSP_DEPTH`].
//! * Bounded resources (`CWE-400`): caps sectors ([`MAX_SECTORS`]), linedefs
//!   ([`MAX_LINEDEFS`]), BSP nodes ([`MAX_BSP_NODES`]), BSP segs ([`MAX_BSP_SEGS`]),
//!   framebuffer dimensions ([`MAX_FRAME_WIDTH`] x [`MAX_FRAME_HEIGHT`]), and
//!   textures ([`MAX_TEXTURE_DIM`]).
//! * Finite & non-zero guards (`CWE-20` / `CWE-369`): rejects `NaN` / `+/-Inf`,
//!   out-of-range coordinates, degenerate linedefs, degenerate camera bases,
//!   near-zero denominators, and clamps perpendicular depth to [`MIN_PERP_DIST`].
//! * Checked indexing & overflow-safe projection (`CWE-125` / `CWE-787` / `CWE-190`):
//!   culls offscreen spans in `f64`, clamps screen-Y bounds in `f64` before
//!   integer conversion, and performs checked indexing on all buffers.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Generated capability verification witnesses.
pub mod capabilities;

pub use capabilities::PureCapabilities;
pub use math3d::{Aabb3, Mat4, Ray3, Vec3};
pub use pixel_container::PixelContainer;

/// Packed RGBA8 pixel type alias (`[r, g, b, a]`).
pub type Rgba8 = [u8; 4];

/// 2D vector array alias (`[x, y]`).
pub type Vec2Array = [f64; 2];

/// Maximum finite world coordinate or height magnitude (`CWE-20` / `CWE-400`).
pub const MAX_COORD: f64 = 1.0e6;

/// Numerical epsilon for near-zero denominators and determinant guards (`CWE-369`).
pub const EPSILON: f64 = 1.0e-12;

/// Geometric epsilon for BSP line classification and segment splitting.
pub const SPLIT_EPSILON: f64 = 1.0e-6;

/// Minimum perpendicular distance from the camera plane (`CWE-369`).
pub const MIN_PERP_DIST: f64 = 1.0e-6;

/// Maximum number of sectors permitted in a [`Map25D`] (`CWE-400`).
pub const MAX_SECTORS: usize = 4096;

/// Maximum number of linedefs permitted in a [`Map25D`] (`CWE-400`).
pub const MAX_LINEDEFS: usize = 16_384;

/// Maximum number of split segments permitted in a [`BspTree`] (`CWE-400`).
pub const MAX_BSP_SEGS: usize = 65_536;

/// Maximum number of nodes permitted in a [`BspTree`] (`CWE-400`).
pub const MAX_BSP_NODES: usize = 65_536;

/// Maximum BSP tree depth (`CWE-400` / `CWE-674`).
pub const MAX_BSP_DEPTH: usize = 64;

/// Maximum framebuffer width in pixels (`CWE-400`).
pub const MAX_FRAME_WIDTH: u32 = 4096;

/// Maximum framebuffer height in pixels (`CWE-400`).
pub const MAX_FRAME_HEIGHT: u32 = 4096;

/// Maximum texture width or height in pixels (`CWE-400`).
pub const MAX_TEXTURE_DIM: u32 = 2048;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Failure modes for every fallible operation in `bsp-portal25d`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BspError {
    /// A floating-point coordinate, height, angle, or FOV is `NaN` or `+/-Infinity` (`CWE-20`).
    NonFiniteInput,
    /// A coordinate, height, angle, FOV, or distance exceeds safe bounds (`CWE-20` / `CWE-400`).
    ValueOutOfBounds,
    /// A linedef or segment has zero or near-zero length (`CWE-369`).
    DegenerateSegment,
    /// Camera basis vectors are near-zero or collinear (`CWE-369`).
    DegenerateCamera,
    /// Sector list is empty, exceeds [`MAX_SECTORS`], or has `floor_height > ceil_height`.
    InvalidSector,
    /// Linedef list is empty, exceeds [`MAX_LINEDEFS`], or references an out-of-range sector.
    InvalidLinedef,
    /// Texture dimensions are zero, exceed [`MAX_TEXTURE_DIM`], or byte length mismatches.
    InvalidTexture,
    /// Framebuffer dimensions are zero, exceed caps, or byte length mismatches.
    InvalidFramebuffer,
    /// Caller [`RenderScratch`] lacks capacity for the framebuffer width, sector count, or BSP depth.
    ScratchTooSmall {
        /// Required element capacity.
        required: usize,
        /// Provided capacity.
        actual: usize,
    },
    /// BSP construction exceeded [`MAX_BSP_DEPTH`], [`MAX_BSP_NODES`], or [`MAX_BSP_SEGS`] (`CWE-400` / `CWE-674`).
    BspLimitExceeded,
    /// A sector or linedef surface references a texture index outside `textures`.
    InvalidMaterialId {
        /// Missing texture index.
        id: usize,
    },
    /// Checked integer arithmetic overflow (`CWE-190`).
    ArithmeticOverflow,
}

impl std::fmt::Display for BspError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "input contains NaN or Infinity"),
            Self::ValueOutOfBounds => write!(f, "value exceeds safe finite bounds"),
            Self::DegenerateSegment => write!(f, "linedef or segment has near-zero length"),
            Self::DegenerateCamera => write!(f, "camera basis vectors are degenerate"),
            Self::InvalidSector => write!(f, "invalid sector definition or count"),
            Self::InvalidLinedef => write!(f, "invalid linedef definition or sector index"),
            Self::InvalidTexture => write!(f, "invalid texture dimensions or byte buffer"),
            Self::InvalidFramebuffer => write!(f, "invalid framebuffer dimensions or byte buffer"),
            Self::ScratchTooSmall { required, actual } => {
                write!(
                    f,
                    "render scratch capacity too small: need {required}, got {actual}"
                )
            }
            Self::BspLimitExceeded => write!(f, "BSP depth, node, or segment budget exceeded"),
            Self::InvalidMaterialId { id } => write!(f, "texture index {id} out of range"),
            Self::ArithmeticOverflow => write!(f, "checked integer arithmetic overflow"),
        }
    }
}

impl std::error::Error for BspError {}

// ---------------------------------------------------------------------------
// Internal Validation & Color Helpers
// ---------------------------------------------------------------------------

#[inline]
fn check_finite_coord(v: f64) -> Result<f64, BspError> {
    if !v.is_finite() {
        return Err(BspError::NonFiniteInput);
    }
    if v.abs() > MAX_COORD {
        return Err(BspError::ValueOutOfBounds);
    }
    Ok(v)
}

#[inline]
fn check_finite_bounded(v: f64, limit: f64) -> Result<f64, BspError> {
    if !v.is_finite() {
        return Err(BspError::NonFiniteInput);
    }
    if v.abs() > limit {
        return Err(BspError::ValueOutOfBounds);
    }
    Ok(v)
}

#[inline]
fn map_math3d_err(err: math3d::Math3dError) -> BspError {
    match err {
        math3d::Math3dError::NonFiniteInput => BspError::NonFiniteInput,
        math3d::Math3dError::ZeroLength | math3d::Math3dError::DegenerateBasis => {
            BspError::DegenerateCamera
        }
        _ => BspError::ValueOutOfBounds,
    }
}

#[inline]
fn shade_rgba(rgba: Rgba8, light_level: u8) -> Rgba8 {
    if light_level == 255 {
        return rgba;
    }
    let l = u16::from(light_level);
    let r = (u16::from(rgba[0]) * l + 127) / 255;
    let g = (u16::from(rgba[1]) * l + 127) / 255;
    let b = (u16::from(rgba[2]) * l + 127) / 255;
    [r as u8, g as u8, b as u8, rgba[3]]
}

#[inline]
fn clamp_screen_y_i32(y_f: f64, min_bound: i32, max_bound: i32) -> i32 {
    if !y_f.is_finite() {
        return min_bound;
    }
    let lo = f64::from(min_bound);
    let hi = f64::from(max_bound);
    if y_f <= lo {
        min_bound
    } else if y_f >= hi {
        max_bound
    } else {
        y_f as i32
    }
}

#[inline]
fn write_pixel(fb_data: &mut [u8], width: usize, x: usize, y: usize, color: Rgba8) {
    if let Some(row_offset) = y
        .checked_mul(width)
        .and_then(|p| p.checked_add(x))
        .and_then(|p| p.checked_mul(4))
    {
        if let Some(slot) = fb_data.get_mut(row_offset..row_offset + 4) {
            slot[0] = color[0];
            slot[1] = color[1];
            slot[2] = color[2];
            slot[3] = color[3];
        }
    }
}

#[inline]
fn fnv1a_update_bytes(state: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *state ^= u64::from(b);
        *state = state.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

#[inline]
fn fnv1a_update_f64(state: &mut u64, v: f64) {
    // Normalize -0.0 to +0.0 and NaN payloads so digests are canonical across platforms.
    let canonical = if v.is_nan() {
        f64::NAN
    } else if v == 0.0 {
        0.0
    } else {
        v
    };
    fnv1a_update_bytes(state, &canonical.to_le_bytes());
}

// ---------------------------------------------------------------------------
// Vec2 & Aabb2 (2D Geometry & math3d Interop)
// ---------------------------------------------------------------------------

/// A 2D Cartesian point or vector `(x, y)` on the map plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec2 {
    /// East-West X coordinate.
    pub x: f64,
    /// North-South Y coordinate.
    pub y: f64,
}

impl Vec2 {
    /// Construct a `Vec2` directly without bounds validation.
    #[inline]
    pub const fn vec2_new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Construct a validated `Vec2`, rejecting non-finite or out-of-bounds coordinates (`CWE-20`).
    #[inline]
    pub fn vec2_try_new(x: f64, y: f64) -> Result<Self, BspError> {
        Ok(Self {
            x: check_finite_coord(x)?,
            y: check_finite_coord(y)?,
        })
    }

    /// Construct a validated `Vec2` from `[x, y]`.
    #[inline]
    pub fn vec2_from_array(a: Vec2Array) -> Result<Self, BspError> {
        Self::vec2_try_new(a[0], a[1])
    }

    /// Convert to `[x, y]`.
    #[inline]
    pub const fn vec2_to_array(self) -> Vec2Array {
        [self.x, self.y]
    }

    /// Component-wise addition.
    #[inline]
    pub fn vec2_add(self, rhs: Self) -> Result<Self, BspError> {
        Self::vec2_try_new(self.x + rhs.x, self.y + rhs.y)
    }

    /// Component-wise subtraction.
    #[inline]
    pub fn vec2_sub(self, rhs: Self) -> Result<Self, BspError> {
        Self::vec2_try_new(self.x - rhs.x, self.y - rhs.y)
    }

    /// Uniform scalar multiplication.
    #[inline]
    pub fn vec2_scale(self, s: f64) -> Result<Self, BspError> {
        let s = check_finite_coord(s)?;
        Self::vec2_try_new(self.x * s, self.y * s)
    }

    /// 2D dot product `x1 * x2 + y1 * y2`.
    #[inline]
    pub fn vec2_dot(self, rhs: Self) -> Result<f64, BspError> {
        let a = Self::vec2_try_new(self.x, self.y)?;
        let b = Self::vec2_try_new(rhs.x, rhs.y)?;
        let d = a.x * b.x + a.y * b.y;
        if !d.is_finite() {
            return Err(BspError::ValueOutOfBounds);
        }
        Ok(d)
    }

    /// 2D scalar cross product `x1 * y2 - y1 * x2`.
    #[inline]
    pub fn vec2_cross(self, rhs: Self) -> Result<f64, BspError> {
        let a = Self::vec2_try_new(self.x, self.y)?;
        let b = Self::vec2_try_new(rhs.x, rhs.y)?;
        let c = a.x * b.y - a.y * b.x;
        if !c.is_finite() {
            return Err(BspError::ValueOutOfBounds);
        }
        Ok(c)
    }

    /// Squared Euclidean length.
    #[inline]
    pub fn vec2_length_sq(self) -> Result<f64, BspError> {
        self.vec2_dot(self)
    }

    /// Euclidean length computed deterministically via `trig::sqrt` (`PHY00`).
    #[inline]
    pub fn vec2_length(self) -> Result<f64, BspError> {
        let lsq = self.vec2_length_sq()?;
        Ok(trig::sqrt(lsq))
    }

    /// Lift `(x, y)` with vertical elevation `height` into `math3d::Vec3(x, height, y)` (`GAME30` interop).
    #[inline]
    pub fn vec2_to_vec3(self, height: f64) -> Result<Vec3, BspError> {
        let x = check_finite_coord(self.x)?;
        let y = check_finite_coord(self.y)?;
        let h = check_finite_coord(height)?;
        Vec3::try_vec3(x, h, y).map_err(map_math3d_err)
    }

    /// Project a `math3d::Vec3` onto the 2D map plane `(v.x, v.z)` (`GAME30` interop).
    #[inline]
    pub fn vec2_from_vec3_xz(v: Vec3) -> Result<Self, BspError> {
        if !v.x.is_finite() || !v.y.is_finite() || !v.z.is_finite() {
            return Err(BspError::NonFiniteInput);
        }
        Self::vec2_try_new(v.x, v.z)
    }
}

/// 2D axis-aligned bounding box on the map plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb2 {
    /// Minimum X coordinate.
    pub min_x: f64,
    /// Minimum Y coordinate.
    pub min_y: f64,
    /// Maximum X coordinate.
    pub max_x: f64,
    /// Maximum Y coordinate.
    pub max_y: f64,
}

impl Aabb2 {
    /// Construct an [`Aabb2`] enclosing two validated points.
    pub fn aabb2_from_points(a: Vec2, b: Vec2) -> Result<Self, BspError> {
        let a = Vec2::vec2_try_new(a.x, a.y)?;
        let b = Vec2::vec2_try_new(b.x, b.y)?;
        Ok(Self {
            min_x: a.x.min(b.x),
            min_y: a.y.min(b.y),
            max_x: a.x.max(b.x),
            max_y: a.y.max(b.y),
        })
    }

    /// Expand this box in place to enclose `p`.
    #[inline]
    fn expand_point(&mut self, p: Vec2) {
        self.min_x = self.min_x.min(p.x);
        self.min_y = self.min_y.min(p.y);
        self.max_x = self.max_x.max(p.x);
        self.max_y = self.max_y.max(p.y);
    }

    /// Test whether `(px, py)` lies inside or on the boundary of this box.
    #[inline]
    pub fn aabb2_contains_point(&self, p: Vec2) -> Result<bool, BspError> {
        let min_x = check_finite_coord(self.min_x)?;
        let min_y = check_finite_coord(self.min_y)?;
        let max_x = check_finite_coord(self.max_x)?;
        let max_y = check_finite_coord(self.max_y)?;
        if min_x > max_x || min_y > max_y {
            return Err(BspError::ValueOutOfBounds);
        }
        let p = Vec2::vec2_try_new(p.x, p.y)?;
        Ok(p.x >= min_x && p.x <= max_x && p.y >= min_y && p.y <= max_y)
    }

    /// Lift this 2D bounding box with `[min_z, max_z]` heights into a `math3d::Aabb3` (`GAME30` interop).
    pub fn aabb2_to_aabb3(&self, min_z: f64, max_z: f64) -> Result<Aabb3, BspError> {
        let min_x = check_finite_coord(self.min_x)?;
        let min_y = check_finite_coord(self.min_y)?;
        let max_x = check_finite_coord(self.max_x)?;
        let max_y = check_finite_coord(self.max_y)?;
        if min_x > max_x || min_y > max_y {
            return Err(BspError::ValueOutOfBounds);
        }
        let min_z = check_finite_coord(min_z)?;
        let max_z = check_finite_coord(max_z)?;
        if min_z > max_z {
            return Err(BspError::ValueOutOfBounds);
        }
        Aabb3::aabb3_from_min_max(
            Vec3::new(min_x, min_z, min_y),
            Vec3::new(max_x, max_z, max_y),
        )
        .map_err(map_math3d_err)
    }
}

// ---------------------------------------------------------------------------
// TextureRgba & SurfaceStyle
// ---------------------------------------------------------------------------

/// Row-major RGBA8 texture (`width * height * 4` bytes) for walls, floors, and ceilings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextureRgba {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl TextureRgba {
    /// Construct a validated [`TextureRgba`] from raw RGBA8 bytes.
    pub fn texture_new(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, BspError> {
        if width == 0 || height == 0 || width > MAX_TEXTURE_DIM || height > MAX_TEXTURE_DIM {
            return Err(BspError::InvalidTexture);
        }
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or(BspError::ArithmeticOverflow)?;
        if pixels.len() != expected {
            return Err(BspError::InvalidTexture);
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// Wrap a [`PixelContainer`] (`IMG00`) as a validated [`TextureRgba`].
    pub fn texture_from_pixel_container(container: &PixelContainer) -> Result<Self, BspError> {
        Self::texture_new(container.width, container.height, container.data.clone())
    }

    /// Create a solid-color [`TextureRgba`] filled with `color`.
    pub fn texture_solid(width: u32, height: u32, color: Rgba8) -> Result<Self, BspError> {
        if width == 0 || height == 0 || width > MAX_TEXTURE_DIM || height > MAX_TEXTURE_DIM {
            return Err(BspError::InvalidTexture);
        }
        let byte_len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or(BspError::ArithmeticOverflow)?;
        let mut pixels = vec![0u8; byte_len];
        let (chunks, _) = pixels.as_chunks_mut::<4>();
        for px in chunks {
            *px = color;
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// Create a deterministic checkerboard [`TextureRgba`] with square size `cell_size`.
    pub fn texture_checkerboard(
        width: u32,
        height: u32,
        cell_size: u32,
        color_a: Rgba8,
        color_b: Rgba8,
    ) -> Result<Self, BspError> {
        if width == 0
            || height == 0
            || cell_size == 0
            || width > MAX_TEXTURE_DIM
            || height > MAX_TEXTURE_DIM
        {
            return Err(BspError::InvalidTexture);
        }
        let byte_len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or(BspError::ArithmeticOverflow)?;
        let mut pixels = vec![0u8; byte_len];
        let (chunks, _) = pixels.as_chunks_mut::<4>();
        let w = width as usize;
        let cs = cell_size as usize;
        for (idx, px) in chunks.iter_mut().enumerate() {
            let x = idx % w;
            let y = idx / w;
            let checker = ((x / cs) + (y / cs)) & 1;
            *px = if checker == 0 { color_a } else { color_b };
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// Texture width in pixels.
    #[inline]
    pub fn texture_width(&self) -> u32 {
        self.width
    }

    /// Texture height in pixels.
    #[inline]
    pub fn texture_height(&self) -> u32 {
        self.height
    }

    /// Borrow raw RGBA8 texel bytes.
    #[inline]
    pub fn texture_pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Checked texel lookup at `(tx, ty)`. Clamps out-of-range coordinates safely.
    #[inline]
    pub fn texture_sample(&self, tx: usize, ty: usize) -> Rgba8 {
        let w = self.width as usize;
        let h = self.height as usize;
        if w == 0 || h == 0 {
            return [0, 0, 0, 0];
        }
        let cx = tx.min(w - 1);
        let cy = ty.min(h - 1);
        let offset = (cy * w + cx) * 4;
        if let Some(slice) = self.pixels.get(offset..offset + 4) {
            [slice[0], slice[1], slice[2], slice[3]]
        } else {
            [0, 0, 0, 0]
        }
    }

    /// Sample normalized UV coordinates `(u, v)` with wrap-around tiling (`CWE-190`-safe).
    #[inline]
    fn sample_uv(&self, u: f64, v: f64) -> Rgba8 {
        if !u.is_finite() || !v.is_finite() {
            return [0, 0, 0, 0];
        }
        let uf = (u - u.floor()).clamp(0.0, 1.0);
        let vf = (v - v.floor()).clamp(0.0, 1.0);
        let w = self.width as usize;
        let h = self.height as usize;
        let tx = ((uf * w as f64) as usize).min(w.saturating_sub(1));
        let ty = ((vf * h as f64) as usize).min(h.saturating_sub(1));
        self.texture_sample(tx, ty)
    }
}

/// Surface appearance for sector floors/ceilings and linedef upper/middle/lower walls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceStyle {
    /// Fallback solid color used when `texture_index` is `None`.
    pub color: Rgba8,
    /// Optional index into the caller's `textures` slice.
    pub texture_index: Option<usize>,
}

impl SurfaceStyle {
    /// Solid-color surface with no texture lookup.
    #[inline]
    pub const fn surface_solid(color: Rgba8) -> Self {
        Self {
            color,
            texture_index: None,
        }
    }

    /// Textured surface referencing `textures[texture_index]`.
    #[inline]
    pub const fn surface_textured(texture_index: usize, fallback_color: Rgba8) -> Self {
        Self {
            color: fallback_color,
            texture_index: Some(texture_index),
        }
    }
}

// ---------------------------------------------------------------------------
// Sector, Linedef, and Map25D
// ---------------------------------------------------------------------------

/// A 2.5D sector with floor/ceiling elevations, surface styles, and sector light level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sector {
    /// Floor elevation in world units.
    pub floor_height: f64,
    /// Ceiling elevation in world units (`>= floor_height`).
    pub ceil_height: f64,
    /// Floor plane surface style.
    pub floor_surface: SurfaceStyle,
    /// Ceiling plane surface style.
    pub ceil_surface: SurfaceStyle,
    /// Sector light level (`0` = pitch black, `255` = full brightness).
    pub light_level: u8,
}

impl Sector {
    /// Construct a validated solid-surface [`Sector`] (`floor_height <= ceil_height`).
    pub fn sector_new(
        floor_height: f64,
        ceil_height: f64,
        floor_color: Rgba8,
        ceil_color: Rgba8,
        light_level: u8,
    ) -> Result<Self, BspError> {
        Self::sector_with_surfaces(
            floor_height,
            ceil_height,
            SurfaceStyle::surface_solid(floor_color),
            SurfaceStyle::surface_solid(ceil_color),
            light_level,
        )
    }

    /// Construct a validated [`Sector`] with explicit floor and ceiling [`SurfaceStyle`]s.
    pub fn sector_with_surfaces(
        floor_height: f64,
        ceil_height: f64,
        floor_surface: SurfaceStyle,
        ceil_surface: SurfaceStyle,
        light_level: u8,
    ) -> Result<Self, BspError> {
        let floor_height = check_finite_coord(floor_height)?;
        let ceil_height = check_finite_coord(ceil_height)?;
        if floor_height > ceil_height {
            return Err(BspError::InvalidSector);
        }
        Ok(Self {
            floor_height,
            ceil_height,
            floor_surface,
            ceil_surface,
            light_level,
        })
    }
}

/// An oriented 2D wall or portal segment from `v0` to `v1`.
///
/// The **front side** lies to the **right** of `(v1 - v0)` (clockwise around a
/// closed room). When `back_sector` is `None`, the linedef is an opaque 1-sided
/// wall; when `Some(back)`, it is a 2-sided portal connecting `front_sector` and
/// `back_sector` that renders upper/lower step walls and clips the column window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Linedef {
    /// Start vertex in 2D map coordinates.
    pub v0: Vec2,
    /// End vertex in 2D map coordinates.
    pub v1: Vec2,
    /// Sector index on the front (right) side of `v0 -> v1`.
    pub front_sector: usize,
    /// Optional sector index on the back (left) side (`None` = solid 1-sided wall).
    pub back_sector: Option<usize>,
    /// Middle wall surface (rendered on 1-sided solid walls).
    pub middle: SurfaceStyle,
    /// Upper wall surface (rendered on portals where `front.ceil_height > back.ceil_height`).
    pub upper: SurfaceStyle,
    /// Lower wall surface (rendered on portals where `front.floor_height < back.floor_height`).
    pub lower: SurfaceStyle,
}

impl Linedef {
    /// Construct a validated 1-sided opaque wall [`Linedef`].
    pub fn linedef_solid(
        v0: Vec2,
        v1: Vec2,
        front_sector: usize,
        middle: SurfaceStyle,
    ) -> Result<Self, BspError> {
        Self::linedef_portal(v0, v1, front_sector, None, middle, middle, middle)
    }

    /// Construct a validated 1-sided or 2-sided portal [`Linedef`].
    pub fn linedef_portal(
        v0: Vec2,
        v1: Vec2,
        front_sector: usize,
        back_sector: Option<usize>,
        middle: SurfaceStyle,
        upper: SurfaceStyle,
        lower: SurfaceStyle,
    ) -> Result<Self, BspError> {
        let v0 = Vec2::vec2_try_new(v0.x, v0.y)?;
        let v1 = Vec2::vec2_try_new(v1.x, v1.y)?;
        let dx = v1.x - v0.x;
        let dy = v1.y - v0.y;
        let len_sq = dx * dx + dy * dy;
        if !len_sq.is_finite() || len_sq <= EPSILON * EPSILON {
            return Err(BspError::DegenerateSegment);
        }
        Ok(Self {
            v0,
            v1,
            front_sector,
            back_sector,
            middle,
            upper,
            lower,
        })
    }

    /// World-space 2D length of this linedef.
    pub fn linedef_length(&self) -> Result<f64, BspError> {
        let v0 = Vec2::vec2_try_new(self.v0.x, self.v0.y)?;
        let v1 = Vec2::vec2_try_new(self.v1.x, self.v1.y)?;
        let d = v1.vec2_sub(v0)?;
        d.vec2_length()
    }

    /// Signed perpendicular distance from `v0 -> v1` to `point`.
    ///
    /// Positive values lie on the **front** (right-hand) side; negative values
    /// lie on the **back** (left-hand) side.
    pub fn linedef_signed_distance(&self, point: Vec2) -> Result<f64, BspError> {
        let v0 = Vec2::vec2_try_new(self.v0.x, self.v0.y)?;
        let v1 = Vec2::vec2_try_new(self.v1.x, self.v1.y)?;
        let point = Vec2::vec2_try_new(point.x, point.y)?;
        let dx = v1.x - v0.x;
        let dy = v1.y - v0.y;
        let len = trig::sqrt(dx * dx + dy * dy);
        if !len.is_finite() || len <= EPSILON {
            return Err(BspError::DegenerateSegment);
        }
        let sd = ((point.x - v0.x) * dy - (point.y - v0.y) * dx) / len;
        if !sd.is_finite() {
            return Err(BspError::ValueOutOfBounds);
        }
        Ok(sd)
    }
}

/// Validated collection of [`Sector`]s and [`Linedef`]s defining a 2.5D map.
#[derive(Clone, Debug, PartialEq)]
pub struct Map25D {
    sectors: Vec<Sector>,
    linedefs: Vec<Linedef>,
}

impl Map25D {
    /// Construct and validate a [`Map25D`].
    ///
    /// Rejects empty or oversized sector/linedef lists (`CWE-400`), inverted
    /// sectors, degenerate linedefs (`CWE-369`), and out-of-range sector indices
    /// (`CWE-125`).
    pub fn map_new(sectors: Vec<Sector>, linedefs: Vec<Linedef>) -> Result<Self, BspError> {
        if sectors.is_empty() || sectors.len() > MAX_SECTORS {
            return Err(BspError::InvalidSector);
        }
        if linedefs.is_empty() || linedefs.len() > MAX_LINEDEFS {
            return Err(BspError::InvalidLinedef);
        }

        for s in &sectors {
            let floor = check_finite_coord(s.floor_height)?;
            let ceil = check_finite_coord(s.ceil_height)?;
            if floor > ceil {
                return Err(BspError::InvalidSector);
            }
        }

        for ld in &linedefs {
            let v0 = Vec2::vec2_try_new(ld.v0.x, ld.v0.y)?;
            let v1 = Vec2::vec2_try_new(ld.v1.x, ld.v1.y)?;
            let dx = v1.x - v0.x;
            let dy = v1.y - v0.y;
            let len_sq = dx * dx + dy * dy;
            if !len_sq.is_finite() || len_sq <= EPSILON * EPSILON {
                return Err(BspError::DegenerateSegment);
            }
            if ld.front_sector >= sectors.len() {
                return Err(BspError::InvalidLinedef);
            }
            if let Some(back) = ld.back_sector {
                if back >= sectors.len() {
                    return Err(BspError::InvalidLinedef);
                }
            }
        }

        Ok(Self { sectors, linedefs })
    }

    /// Borrow all validated sectors.
    #[inline]
    pub fn map_sectors(&self) -> &[Sector] {
        &self.sectors
    }

    /// Borrow all validated linedefs.
    #[inline]
    pub fn map_linedefs(&self) -> &[Linedef] {
        &self.linedefs
    }

    /// Checked sector lookup.
    #[inline]
    pub fn map_sector_at(&self, idx: usize) -> Option<&Sector> {
        self.sectors.get(idx)
    }

    /// Checked linedef lookup.
    #[inline]
    pub fn map_linedef_at(&self, idx: usize) -> Option<&Linedef> {
        self.linedefs.get(idx)
    }
}

// ---------------------------------------------------------------------------
// 2D BSP Tree Builder & Subsector Queries
// ---------------------------------------------------------------------------

/// A directed line segment produced by splitting a [`Linedef`] during BSP construction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seg {
    v0: Vec2,
    v1: Vec2,
    front_sector: usize,
    back_sector: Option<usize>,
    linedef_index: usize,
    u_start: f64,
    u_end: f64,
}

impl Seg {
    /// Start vertex of this seg.
    #[inline]
    pub const fn seg_v0(&self) -> Vec2 {
        self.v0
    }

    /// End vertex of this seg.
    #[inline]
    pub const fn seg_v1(&self) -> Vec2 {
        self.v1
    }

    /// Sector index on the front (right) side of this seg.
    #[inline]
    pub const fn seg_front_sector(&self) -> usize {
        self.front_sector
    }

    /// Optional sector index on the back (left) side (`None` = opaque wall).
    #[inline]
    pub const fn seg_back_sector(&self) -> Option<usize> {
        self.back_sector
    }

    /// Source linedef index inside [`Map25D`].
    #[inline]
    pub const fn seg_linedef_index(&self) -> usize {
        self.linedef_index
    }

    /// World-space horizontal texture coordinate span `(u_start, u_end)` along the original linedef.
    #[inline]
    pub const fn seg_u_range(&self) -> (f64, f64) {
        (self.u_start, self.u_end)
    }
}

/// Discriminant for nodes in [`BspTree`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BspNodeKind {
    /// Internal partitioning node with `front_child` and `back_child`.
    Internal,
    /// Convex subsector leaf referencing a contiguous slice of [`Seg`]s.
    Leaf,
}

/// A node in a compiled 2D [`BspTree`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BspNode {
    kind: BspNodeKind,
    bounds: Aabb2,
    partition_v0: Vec2,
    partition_v1: Vec2,
    front_child: usize,
    back_child: usize,
    seg_start: usize,
    seg_count: usize,
}

impl BspNode {
    /// Node kind (`Internal` or `Leaf`).
    #[inline]
    pub const fn node_kind(&self) -> BspNodeKind {
        self.kind
    }

    /// 2D bounding box enclosing all segs in this subtree.
    #[inline]
    pub const fn node_bounds(&self) -> Aabb2 {
        self.bounds
    }

    /// Partition line endpoints `(v0, v1)` for internal nodes.
    #[inline]
    pub const fn node_partition(&self) -> (Vec2, Vec2) {
        (self.partition_v0, self.partition_v1)
    }

    /// Child node indices `(front_child, back_child)` for internal nodes.
    #[inline]
    pub const fn node_children(&self) -> Option<(usize, usize)> {
        match self.kind {
            BspNodeKind::Internal => Some((self.front_child, self.back_child)),
            BspNodeKind::Leaf => None,
        }
    }

    /// Contiguous `(seg_start, seg_count)` range into `BspTree::bsp_segs()` for leaf nodes.
    #[inline]
    pub const fn node_seg_range(&self) -> Option<(usize, usize)> {
        match self.kind {
            BspNodeKind::Leaf => Some((self.seg_start, self.seg_count)),
            BspNodeKind::Internal => None,
        }
    }
}

/// Signed perpendicular distance from directed line `p0 -> p1` to `q`.
/// Positive is on the right (front) side; negative is on the left (back) side.
#[inline]
fn line_signed_dist(p0: Vec2, p1: Vec2, q: Vec2) -> f64 {
    let dx = p1.x - p0.x;
    let dy = p1.y - p0.y;
    let len = trig::sqrt(dx * dx + dy * dy);
    if len <= EPSILON {
        return 0.0;
    }
    ((q.x - p0.x) * dy - (q.y - p0.y) * dx) / len
}

#[inline]
fn segs_bounds(segs: &[Seg]) -> Aabb2 {
    if let Some(first) = segs.first() {
        let mut b = Aabb2 {
            min_x: first.v0.x.min(first.v1.x),
            min_y: first.v0.y.min(first.v1.y),
            max_x: first.v0.x.max(first.v1.x),
            max_y: first.v0.y.max(first.v1.y),
        };
        for seg in segs.iter().skip(1) {
            b.expand_point(seg.v0);
            b.expand_point(seg.v1);
        }
        b
    } else {
        Aabb2 {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 0.0,
            max_y: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Classify {
    Front,
    Back,
    Split { d0: f64, d1: f64 },
}

#[inline]
fn classify_seg_against_line(p0: Vec2, p1: Vec2, seg: &Seg) -> Classify {
    let d0 = line_signed_dist(p0, p1, seg.v0);
    let d1 = line_signed_dist(p0, p1, seg.v1);

    if d0.abs() <= SPLIT_EPSILON && d1.abs() <= SPLIT_EPSILON {
        let pdx = p1.x - p0.x;
        let pdy = p1.y - p0.y;
        let qdx = seg.v1.x - seg.v0.x;
        let qdy = seg.v1.y - seg.v0.y;
        if pdx * qdx + pdy * qdy >= 0.0 {
            Classify::Front
        } else {
            Classify::Back
        }
    } else if d0 >= -SPLIT_EPSILON && d1 >= -SPLIT_EPSILON {
        Classify::Front
    } else if d0 <= SPLIT_EPSILON && d1 <= SPLIT_EPSILON {
        Classify::Back
    } else {
        Classify::Split { d0, d1 }
    }
}

fn find_best_splitter(segs: &[Seg]) -> Option<usize> {
    let mut best_idx = None;
    let mut best_score = usize::MAX;

    for (i, cand) in segs.iter().enumerate() {
        let mut front_count = 0usize;
        let mut back_count = 0usize;
        let mut splits = 0usize;

        for other in segs {
            match classify_seg_against_line(cand.v0, cand.v1, other) {
                Classify::Front => front_count += 1,
                Classify::Back => back_count += 1,
                Classify::Split { .. } => {
                    front_count += 1;
                    back_count += 1;
                    splits += 1;
                }
            }
        }

        if front_count > 0 && back_count > 0 {
            let balance = front_count.abs_diff(back_count);
            let score = splits.saturating_mul(8).saturating_add(balance);
            if score < best_score {
                best_score = score;
                best_idx = Some(i);
            }
        }
    }

    best_idx
}

fn split_and_partition_segs(
    splitter_v0: Vec2,
    splitter_v1: Vec2,
    segs: &[Seg],
) -> (Vec<Seg>, Vec<Seg>) {
    let mut front = Vec::new();
    let mut back = Vec::new();

    for &seg in segs {
        match classify_seg_against_line(splitter_v0, splitter_v1, &seg) {
            Classify::Front => front.push(seg),
            Classify::Back => back.push(seg),
            Classify::Split { d0, d1 } => {
                let denom = d0 - d1;
                if denom.abs() <= EPSILON {
                    if d0 >= 0.0 {
                        front.push(seg);
                    } else {
                        back.push(seg);
                    }
                    continue;
                }
                let t = (d0 / denom).clamp(1.0e-7, 1.0 - 1.0e-7);
                let mid = Vec2::vec2_new(
                    seg.v0.x + (seg.v1.x - seg.v0.x) * t,
                    seg.v0.y + (seg.v1.y - seg.v0.y) * t,
                );
                let u_mid = seg.u_start + (seg.u_end - seg.u_start) * t;

                let piece_a = Seg {
                    v0: seg.v0,
                    v1: mid,
                    front_sector: seg.front_sector,
                    back_sector: seg.back_sector,
                    linedef_index: seg.linedef_index,
                    u_start: seg.u_start,
                    u_end: u_mid,
                };
                let piece_b = Seg {
                    v0: mid,
                    v1: seg.v1,
                    front_sector: seg.front_sector,
                    back_sector: seg.back_sector,
                    linedef_index: seg.linedef_index,
                    u_start: u_mid,
                    u_end: seg.u_end,
                };

                let len_a_sq =
                    (piece_a.v1.x - piece_a.v0.x).powi(2) + (piece_a.v1.y - piece_a.v0.y).powi(2);
                let len_b_sq =
                    (piece_b.v1.x - piece_b.v0.x).powi(2) + (piece_b.v1.y - piece_b.v0.y).powi(2);

                if len_a_sq <= EPSILON * EPSILON || len_b_sq <= EPSILON * EPSILON {
                    if d0.abs() >= d1.abs() {
                        if d0 >= 0.0 {
                            front.push(seg);
                        } else {
                            back.push(seg);
                        }
                    } else if d1 >= 0.0 {
                        front.push(seg);
                    } else {
                        back.push(seg);
                    }
                    continue;
                }

                if d0 > 0.0 {
                    front.push(piece_a);
                    back.push(piece_b);
                } else {
                    back.push(piece_a);
                    front.push(piece_b);
                }
            }
        }
    }

    (front, back)
}

struct BuildTask {
    node_slot: usize,
    segs: Vec<Seg>,
    depth: usize,
}

/// Compiled 2D Binary Space Partitioning tree over a [`Map25D`].
#[derive(Clone, Debug, PartialEq)]
pub struct BspTree {
    nodes: Vec<BspNode>,
    segs: Vec<Seg>,
    max_depth: usize,
}

impl BspTree {
    /// Compile a deterministic 2D BSP tree from `map` using an explicit iterative
    /// work stack (`CWE-674` recursion-free) and bounded budgets (`CWE-400`).
    pub fn bsp_build(map: &Map25D) -> Result<Self, BspError> {
        let mut initial_segs = Vec::new();
        for (linedef_index, ld) in map.linedefs.iter().enumerate() {
            let len = ld.linedef_length()?;
            if initial_segs.len() >= MAX_BSP_SEGS {
                return Err(BspError::BspLimitExceeded);
            }
            initial_segs.push(Seg {
                v0: ld.v0,
                v1: ld.v1,
                front_sector: ld.front_sector,
                back_sector: ld.back_sector,
                linedef_index,
                u_start: 0.0,
                u_end: len,
            });
            if let Some(back_sector) = ld.back_sector {
                if initial_segs.len() >= MAX_BSP_SEGS {
                    return Err(BspError::BspLimitExceeded);
                }
                initial_segs.push(Seg {
                    v0: ld.v1,
                    v1: ld.v0,
                    front_sector: back_sector,
                    back_sector: Some(ld.front_sector),
                    linedef_index,
                    u_start: 0.0,
                    u_end: len,
                });
            }
        }

        if initial_segs.is_empty() {
            return Err(BspError::InvalidLinedef);
        }

        let placeholder = BspNode {
            kind: BspNodeKind::Leaf,
            bounds: Aabb2 {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 0.0,
                max_y: 0.0,
            },
            partition_v0: Vec2::vec2_new(0.0, 0.0),
            partition_v1: Vec2::vec2_new(1.0, 0.0),
            front_child: 0,
            back_child: 0,
            seg_start: 0,
            seg_count: 0,
        };

        let mut nodes = vec![placeholder];
        let mut final_segs = Vec::new();
        let mut max_depth = 0usize;
        let mut stack = vec![BuildTask {
            node_slot: 0,
            segs: initial_segs,
            depth: 0,
        }];

        while let Some(task) = stack.pop() {
            if task.depth > MAX_BSP_DEPTH {
                return Err(BspError::BspLimitExceeded);
            }
            max_depth = max_depth.max(task.depth);

            let bounds = segs_bounds(&task.segs);
            let splitter_opt = if task.segs.len() <= 1 {
                None
            } else {
                find_best_splitter(&task.segs)
            };

            match splitter_opt {
                None => {
                    let seg_start = final_segs.len();
                    let seg_count = task.segs.len();
                    if seg_start.saturating_add(seg_count) > MAX_BSP_SEGS {
                        return Err(BspError::BspLimitExceeded);
                    }
                    final_segs.extend_from_slice(&task.segs);
                    if let Some(slot) = nodes.get_mut(task.node_slot) {
                        *slot = BspNode {
                            kind: BspNodeKind::Leaf,
                            bounds,
                            partition_v0: Vec2::vec2_new(0.0, 0.0),
                            partition_v1: Vec2::vec2_new(1.0, 0.0),
                            front_child: 0,
                            back_child: 0,
                            seg_start,
                            seg_count,
                        };
                    }
                }
                Some(split_idx) => {
                    let splitter = task.segs[split_idx];
                    let (front_segs, back_segs) =
                        split_and_partition_segs(splitter.v0, splitter.v1, &task.segs);

                    if front_segs.is_empty() || back_segs.is_empty() {
                        // Fallback if numeric splitting collapsed to one side: emit convex leaf.
                        let seg_start = final_segs.len();
                        let seg_count = task.segs.len();
                        if seg_start.saturating_add(seg_count) > MAX_BSP_SEGS {
                            return Err(BspError::BspLimitExceeded);
                        }
                        final_segs.extend_from_slice(&task.segs);
                        if let Some(slot) = nodes.get_mut(task.node_slot) {
                            *slot = BspNode {
                                kind: BspNodeKind::Leaf,
                                bounds,
                                partition_v0: Vec2::vec2_new(0.0, 0.0),
                                partition_v1: Vec2::vec2_new(1.0, 0.0),
                                front_child: 0,
                                back_child: 0,
                                seg_start,
                                seg_count,
                            };
                        }
                        continue;
                    }

                    if task.depth >= MAX_BSP_DEPTH {
                        return Err(BspError::BspLimitExceeded);
                    }
                    if nodes.len().saturating_add(2) > MAX_BSP_NODES {
                        return Err(BspError::BspLimitExceeded);
                    }
                    let front_child = nodes.len();
                    nodes.push(placeholder);
                    let back_child = nodes.len();
                    nodes.push(placeholder);

                    if let Some(slot) = nodes.get_mut(task.node_slot) {
                        *slot = BspNode {
                            kind: BspNodeKind::Internal,
                            bounds,
                            partition_v0: splitter.v0,
                            partition_v1: splitter.v1,
                            front_child,
                            back_child,
                            seg_start: 0,
                            seg_count: 0,
                        };
                    }

                    stack.push(BuildTask {
                        node_slot: back_child,
                        segs: back_segs,
                        depth: task.depth + 1,
                    });
                    stack.push(BuildTask {
                        node_slot: front_child,
                        segs: front_segs,
                        depth: task.depth + 1,
                    });
                }
            }
        }

        Ok(Self {
            nodes,
            segs: final_segs,
            max_depth,
        })
    }

    /// Root BSP node index (always `0`).
    #[inline]
    pub const fn bsp_root_index(&self) -> usize {
        0
    }

    /// Borrow all compiled BSP nodes.
    #[inline]
    pub fn bsp_nodes(&self) -> &[BspNode] {
        &self.nodes
    }

    /// Borrow all compiled BSP segs.
    #[inline]
    pub fn bsp_segs(&self) -> &[Seg] {
        &self.segs
    }

    /// Maximum depth reached in this BSP tree (`<= MAX_BSP_DEPTH`).
    #[inline]
    pub const fn bsp_max_depth(&self) -> usize {
        self.max_depth
    }

    /// Locate the convex leaf subsector index containing `(point.x, point.y)` iteratively (`CWE-674`).
    pub fn bsp_find_subsector(&self, point: Vec2) -> Result<usize, BspError> {
        let point = Vec2::vec2_try_new(point.x, point.y)?;
        let mut idx = 0usize;
        for _ in 0..=MAX_BSP_DEPTH {
            let node = self.nodes.get(idx).ok_or(BspError::BspLimitExceeded)?;
            match node.kind {
                BspNodeKind::Leaf => return Ok(idx),
                BspNodeKind::Internal => {
                    let sd = line_signed_dist(node.partition_v0, node.partition_v1, point);
                    idx = if sd >= 0.0 {
                        node.front_child
                    } else {
                        node.back_child
                    };
                }
            }
        }
        Err(BspError::BspLimitExceeded)
    }
}

// ---------------------------------------------------------------------------
// Camera25D
// ---------------------------------------------------------------------------

/// 2.5D camera defined by world `(pos_x, pos_y)`, eye height `pos_z`, forward
/// unit direction `(dir_x, dir_y)`, rightward unit vector `(right_x, right_y)`,
/// horizontal FOV, and near clip plane distance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera25D {
    /// World X position.
    pub pos_x: f64,
    /// World Y position.
    pub pos_y: f64,
    /// Eye elevation above Z = 0.
    pub pos_z: f64,
    /// Forward unit vector X component.
    pub dir_x: f64,
    /// Forward unit vector Y component.
    pub dir_y: f64,
    /// Rightward unit vector X component.
    pub right_x: f64,
    /// Rightward unit vector Y component.
    pub right_y: f64,
    /// Horizontal field of view in radians (`(0, pi)`).
    pub fov_rad: f64,
    /// Near clipping distance (`>= MIN_PERP_DIST`).
    pub near_dist: f64,
}

impl Camera25D {
    /// Construct and validate a [`Camera25D`] from explicit basis vectors.
    #[allow(clippy::too_many_arguments)]
    pub fn camera_new(
        pos_x: f64,
        pos_y: f64,
        pos_z: f64,
        dir_x: f64,
        dir_y: f64,
        right_x: f64,
        right_y: f64,
        fov_rad: f64,
        near_dist: f64,
    ) -> Result<Self, BspError> {
        let pos_x = check_finite_coord(pos_x)?;
        let pos_y = check_finite_coord(pos_y)?;
        let pos_z = check_finite_coord(pos_z)?;
        let dir_x = check_finite_coord(dir_x)?;
        let dir_y = check_finite_coord(dir_y)?;
        let right_x = check_finite_coord(right_x)?;
        let right_y = check_finite_coord(right_y)?;
        if !fov_rad.is_finite() || !near_dist.is_finite() {
            return Err(BspError::NonFiniteInput);
        }
        if fov_rad <= EPSILON || fov_rad >= trig::PI - EPSILON {
            return Err(BspError::ValueOutOfBounds);
        }
        if !(MIN_PERP_DIST..=MAX_COORD).contains(&near_dist) {
            return Err(BspError::ValueOutOfBounds);
        }

        let dir_len_sq = dir_x * dir_x + dir_y * dir_y;
        let right_len_sq = right_x * right_x + right_y * right_y;
        if !dir_len_sq.is_finite()
            || !right_len_sq.is_finite()
            || dir_len_sq <= EPSILON * EPSILON
            || right_len_sq <= EPSILON * EPSILON
        {
            return Err(BspError::DegenerateCamera);
        }
        let det = dir_x * right_y - dir_y * right_x;
        if !det.is_finite() || det.abs() <= EPSILON {
            return Err(BspError::DegenerateCamera);
        }

        let dir_len = trig::sqrt(dir_len_sq);
        let right_len = trig::sqrt(right_len_sq);

        Ok(Self {
            pos_x,
            pos_y,
            pos_z,
            dir_x: dir_x / dir_len,
            dir_y: dir_y / dir_len,
            right_x: right_x / right_len,
            right_y: right_y / right_len,
            fov_rad,
            near_dist,
        })
    }

    /// Construct a [`Camera25D`] from position `(pos_x, pos_y, pos_z)`, yaw angle, and horizontal FOV.
    ///
    /// At `yaw_rad = 0`: `dir = (1, 0)` (East) and `right = (0, -1)` (South).
    pub fn camera_from_yaw_fov(
        pos_x: f64,
        pos_y: f64,
        pos_z: f64,
        yaw_rad: f64,
        fov_rad: f64,
    ) -> Result<Self, BspError> {
        let yaw_rad = check_finite_bounded(yaw_rad, math3d::MAX_ANGLE_RAD)?;
        let dir_x = trig::cos(yaw_rad);
        let dir_y = trig::sin(yaw_rad);
        // 90 degrees clockwise from (cos, sin) is (sin, -cos).
        let right_x = dir_y;
        let right_y = -dir_x;
        Self::camera_new(
            pos_x, pos_y, pos_z, dir_x, dir_y, right_x, right_y, fov_rad, 0.05,
        )
    }

    /// Return a new [`Camera25D`] rotated counter-clockwise by `delta_yaw_rad` radians.
    pub fn camera_rotate(&self, delta_yaw_rad: f64) -> Result<Self, BspError> {
        let delta = check_finite_bounded(delta_yaw_rad, math3d::MAX_ANGLE_RAD)?;
        let c = trig::cos(delta);
        let s = trig::sin(delta);
        Self::camera_new(
            self.pos_x,
            self.pos_y,
            self.pos_z,
            self.dir_x * c - self.dir_y * s,
            self.dir_x * s + self.dir_y * c,
            self.right_x * c - self.right_y * s,
            self.right_x * s + self.right_y * c,
            self.fov_rad,
            self.near_dist,
        )
    }

    /// Return a new [`Camera25D`] translated by `(dx, dy, dz)`.
    pub fn camera_move_by(&self, dx: f64, dy: f64, dz: f64) -> Result<Self, BspError> {
        let dx = check_finite_coord(dx)?;
        let dy = check_finite_coord(dy)?;
        let dz = check_finite_coord(dz)?;
        Self::camera_new(
            self.pos_x + dx,
            self.pos_y + dy,
            self.pos_z + dz,
            self.dir_x,
            self.dir_y,
            self.right_x,
            self.right_y,
            self.fov_rad,
            self.near_dist,
        )
    }

    /// Return a new [`Camera25D`] with updated eye elevation `eye_z`.
    pub fn camera_set_eye_height(&self, eye_z: f64) -> Result<Self, BspError> {
        Self::camera_new(
            self.pos_x,
            self.pos_y,
            eye_z,
            self.dir_x,
            self.dir_y,
            self.right_x,
            self.right_y,
            self.fov_rad,
            self.near_dist,
        )
    }

    /// Camera 2D position `(pos_x, pos_y)`.
    #[inline]
    pub const fn camera_pos(&self) -> Vec2 {
        Vec2::vec2_new(self.pos_x, self.pos_y)
    }

    /// Camera forward unit vector `(dir_x, dir_y)`.
    #[inline]
    pub const fn camera_dir(&self) -> Vec2 {
        Vec2::vec2_new(self.dir_x, self.dir_y)
    }

    /// Camera rightward unit vector `(right_x, right_y)`.
    #[inline]
    pub const fn camera_right(&self) -> Vec2 {
        Vec2::vec2_new(self.right_x, self.right_y)
    }

    /// Camera eye height `pos_z`.
    #[inline]
    pub const fn camera_eye_height(&self) -> f64 {
        self.pos_z
    }

    /// Camera horizontal FOV in radians.
    #[inline]
    pub const fn camera_fov(&self) -> f64 {
        self.fov_rad
    }

    /// Camera near clip distance.
    #[inline]
    pub const fn camera_near_dist(&self) -> f64 {
        self.near_dist
    }

    /// Convert camera forward direction into a normalized `math3d::Vec3(dir_x, 0, dir_y)` (`GAME30` interop).
    pub fn camera_forward_vec3(&self) -> Result<Vec3, BspError> {
        let dir_x = check_finite_coord(self.dir_x)?;
        let dir_y = check_finite_coord(self.dir_y)?;
        let v = Vec3::try_vec3(dir_x, 0.0, dir_y).map_err(map_math3d_err)?;
        v.normalize().map_err(map_math3d_err)
    }

    /// Construct a `math3d::Ray3` from the camera eye `(pos_x, pos_z, pos_y)` along `camera_forward_vec3` (`GAME30` interop).
    pub fn camera_world_ray(&self) -> Result<Ray3, BspError> {
        let pos_x = check_finite_coord(self.pos_x)?;
        let pos_y = check_finite_coord(self.pos_y)?;
        let pos_z = check_finite_coord(self.pos_z)?;
        let origin = Vec3::try_vec3(pos_x, pos_z, pos_y).map_err(map_math3d_err)?;
        let dir = self.camera_forward_vec3()?;
        Ray3::ray3_from_origin_dir(origin, dir).map_err(map_math3d_err)
    }

    /// Construct a 3D view matrix via `math3d::Mat4::look_at` (`GAME30` interop).
    pub fn camera_view_matrix(&self) -> Result<Mat4, BspError> {
        let pos_x = check_finite_coord(self.pos_x)?;
        let pos_y = check_finite_coord(self.pos_y)?;
        let pos_z = check_finite_coord(self.pos_z)?;
        let eye = Vec3::try_vec3(pos_x, pos_z, pos_y).map_err(map_math3d_err)?;
        let fwd = self.camera_forward_vec3()?;
        let target = eye.vec3_add(fwd).map_err(map_math3d_err)?;
        Mat4::look_at(eye, target, Vec3::unit_y()).map_err(map_math3d_err)
    }

    fn validate_for_render(&self) -> Result<Self, BspError> {
        Self::camera_new(
            self.pos_x,
            self.pos_y,
            self.pos_z,
            self.dir_x,
            self.dir_y,
            self.right_x,
            self.right_y,
            self.fov_rad,
            self.near_dist,
        )
    }
}

// ---------------------------------------------------------------------------
// RenderConfig, RenderScratch, RenderStats
// ---------------------------------------------------------------------------

/// Frame rendering configuration for [`render_frame_into`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderConfig {
    /// Background / void color cleared before BSP traversal.
    pub background_color: Rgba8,
    /// Enable 2D AABB frustum & closed-column culling on internal BSP nodes.
    pub enable_node_culling: bool,
}

impl RenderConfig {
    /// Default configuration with dark sky/void clear and node culling enabled.
    #[inline]
    pub const fn render_config_default() -> Self {
        Self {
            background_color: [16, 20, 32, 255],
            enable_node_culling: true,
        }
    }

    /// Construct a [`RenderConfig`] with an explicit background color and culling toggle.
    #[inline]
    pub const fn render_config_with_background(
        background_color: Rgba8,
        enable_node_culling: bool,
    ) -> Self {
        Self {
            background_color,
            enable_node_culling,
        }
    }
}

/// Pre-allocated rendering workspace enabling zero-allocation steady-state frames.
///
/// Holds the 1D column occlusion window (`y_top`, `y_bottom`), iterative BSP
/// traversal stack, and dynamic visibility lists (`visible_sectors`,
/// `visible_subsectors`).
#[derive(Clone, Debug)]
pub struct RenderScratch {
    y_top: Vec<i32>,
    y_bottom: Vec<i32>,
    traverse_stack: Vec<usize>,
    sector_gen: Vec<u32>,
    visible_sectors: Vec<usize>,
    visible_subsectors: Vec<usize>,
    frame_gen: u32,
}

impl RenderScratch {
    /// Allocate a [`RenderScratch`] sized for `max_frame_width`, `max_sectors`, and `max_nodes`.
    pub fn scratch_new(
        max_frame_width: usize,
        max_sectors: usize,
        max_nodes: usize,
    ) -> Result<Self, BspError> {
        if max_frame_width == 0 || max_frame_width > MAX_FRAME_WIDTH as usize {
            return Err(BspError::InvalidFramebuffer);
        }
        if max_sectors == 0 || max_sectors > MAX_SECTORS {
            return Err(BspError::InvalidSector);
        }
        if max_nodes == 0 || max_nodes > MAX_BSP_NODES {
            return Err(BspError::BspLimitExceeded);
        }
        let stack_cap = MAX_BSP_DEPTH + 4;
        Ok(Self {
            y_top: vec![0; max_frame_width],
            y_bottom: vec![0; max_frame_width],
            traverse_stack: Vec::with_capacity(stack_cap),
            sector_gen: vec![0; max_sectors],
            visible_sectors: Vec::with_capacity(max_sectors),
            visible_subsectors: Vec::with_capacity(max_nodes),
            frame_gen: 0,
        })
    }

    /// Convenience constructor sizing [`RenderScratch`] to match a specific framebuffer, map, and BSP tree.
    pub fn scratch_for_frame_and_map(
        framebuffer: &PixelContainer,
        map: &Map25D,
        bsp: &BspTree,
    ) -> Result<Self, BspError> {
        Self::scratch_new(
            framebuffer.width as usize,
            map.sectors.len(),
            bsp.nodes.len(),
        )
    }

    /// Column capacity of this scratch workspace.
    #[inline]
    pub fn scratch_frame_width(&self) -> usize {
        self.y_top.len()
    }

    /// Distinct sector IDs reached during the most recent [`render_frame_into`] call, in front-to-back traversal order.
    #[inline]
    pub fn scratch_visible_sectors(&self) -> &[usize] {
        &self.visible_sectors
    }

    /// Subsector (leaf node) indices reached during the most recent [`render_frame_into`] call, in front-to-back order.
    #[inline]
    pub fn scratch_visible_subsectors(&self) -> &[usize] {
        &self.visible_subsectors
    }
}

/// Telemetry produced by [`render_frame_into`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderStats {
    /// Framebuffer column count (`framebuffer.width`).
    pub columns_drawn: usize,
    /// Number of BSP nodes popped and visited.
    pub nodes_visited: usize,
    /// Number of BSP subtrees culled by 2D frustum or closed-column bounds.
    pub nodes_culled: usize,
    /// Number of convex leaf subsectors visited.
    pub subsectors_visited: usize,
    /// Number of segs tested for front-facing projection.
    pub segs_tested: usize,
    /// Number of segs that drew at least one screen column.
    pub segs_drawn: usize,
    /// Number of 2-sided portal segs that drew onto open columns.
    pub portals_traversed: usize,
    /// `true` if traversal terminated early because every column closed (`open_columns == 0`).
    pub early_occlusion_exits: bool,
}

// ---------------------------------------------------------------------------
// Core 2.5D Portal Renderer
// ---------------------------------------------------------------------------

struct ProjectionParams {
    fb_width: usize,
    fb_height: usize,
    half_w: f64,
    horizon_y: f64,
    focal: f64,
    tan_half_fov: f64,
    near_dist: f64,
}

#[inline]
fn validate_surface_style(style: &SurfaceStyle, tex_len: usize) -> Result<(), BspError> {
    if let Some(id) = style.texture_index {
        if id >= tex_len {
            return Err(BspError::InvalidMaterialId { id });
        }
    }
    Ok(())
}

fn validate_render_inputs(
    map: &Map25D,
    bsp: &BspTree,
    cam: &Camera25D,
    textures: &[TextureRgba],
    scratch: &RenderScratch,
    framebuffer: &PixelContainer,
) -> Result<(Camera25D, ProjectionParams), BspError> {
    let norm_cam = cam.validate_for_render()?;

    if framebuffer.width == 0
        || framebuffer.height == 0
        || framebuffer.width > MAX_FRAME_WIDTH
        || framebuffer.height > MAX_FRAME_HEIGHT
    {
        return Err(BspError::InvalidFramebuffer);
    }
    let expected_bytes = (framebuffer.width as usize)
        .checked_mul(framebuffer.height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or(BspError::ArithmeticOverflow)?;
    if framebuffer.data.len() != expected_bytes {
        return Err(BspError::InvalidFramebuffer);
    }

    let fb_width = framebuffer.width as usize;
    let fb_height = framebuffer.height as usize;

    let half_fov = 0.5 * norm_cam.fov_rad;
    let sin_half = trig::sin(half_fov);
    let cos_half = trig::cos(half_fov);
    if cos_half.abs() <= EPSILON || sin_half.abs() <= EPSILON {
        return Err(BspError::DegenerateCamera);
    }
    let tan_half_fov = sin_half / cos_half;
    let half_w = 0.5 * (fb_width as f64);
    let focal = half_w / tan_half_fov;
    if !focal.is_finite() || focal <= EPSILON {
        return Err(BspError::DegenerateCamera);
    }

    let proj = ProjectionParams {
        fb_width,
        fb_height,
        half_w,
        horizon_y: 0.5 * (fb_height as f64),
        focal,
        tan_half_fov,
        near_dist: norm_cam.near_dist,
    };

    if scratch.y_top.len() < fb_width || scratch.y_bottom.len() < fb_width {
        return Err(BspError::ScratchTooSmall {
            required: fb_width,
            actual: scratch.y_top.len().min(scratch.y_bottom.len()),
        });
    }
    if scratch.sector_gen.len() < map.sectors.len()
        || scratch.visible_sectors.capacity() < map.sectors.len()
    {
        return Err(BspError::ScratchTooSmall {
            required: map.sectors.len(),
            actual: scratch
                .sector_gen
                .len()
                .min(scratch.visible_sectors.capacity()),
        });
    }
    if scratch.visible_subsectors.capacity() < bsp.nodes.len() {
        return Err(BspError::ScratchTooSmall {
            required: bsp.nodes.len(),
            actual: scratch.visible_subsectors.capacity(),
        });
    }
    if scratch.traverse_stack.capacity() < MAX_BSP_DEPTH + 2 {
        return Err(BspError::ScratchTooSmall {
            required: MAX_BSP_DEPTH + 2,
            actual: scratch.traverse_stack.capacity(),
        });
    }

    if bsp.nodes.is_empty() || bsp.nodes.len() > MAX_BSP_NODES || bsp.max_depth > MAX_BSP_DEPTH {
        return Err(BspError::BspLimitExceeded);
    }
    for node in &bsp.nodes {
        match node.kind {
            BspNodeKind::Internal => {
                if node.front_child >= bsp.nodes.len() || node.back_child >= bsp.nodes.len() {
                    return Err(BspError::BspLimitExceeded);
                }
            }
            BspNodeKind::Leaf => {
                let Some(seg_end) = node.seg_start.checked_add(node.seg_count) else {
                    return Err(BspError::BspLimitExceeded);
                };
                if seg_end > bsp.segs.len() {
                    return Err(BspError::BspLimitExceeded);
                }
            }
        }
    }

    let tex_len = textures.len();
    for sector in &map.sectors {
        validate_surface_style(&sector.floor_surface, tex_len)?;
        validate_surface_style(&sector.ceil_surface, tex_len)?;
    }
    for ld in &map.linedefs {
        if ld.front_sector >= map.sectors.len() {
            return Err(BspError::InvalidLinedef);
        }
        if let Some(back) = ld.back_sector {
            if back >= map.sectors.len() {
                return Err(BspError::InvalidLinedef);
            }
        }
        validate_surface_style(&ld.middle, tex_len)?;
        validate_surface_style(&ld.upper, tex_len)?;
        validate_surface_style(&ld.lower, tex_len)?;
    }

    for seg in &bsp.segs {
        if seg.front_sector >= map.sectors.len() || seg.linedef_index >= map.linedefs.len() {
            return Err(BspError::InvalidLinedef);
        }
        if let Some(back) = seg.back_sector {
            if back >= map.sectors.len() {
                return Err(BspError::InvalidLinedef);
            }
        }
    }

    Ok((norm_cam, proj))
}

/// Conservative frustum & 1D column-occlusion test for a BSP node's 2D AABB.
fn node_is_culled(
    bounds: Aabb2,
    cam: &Camera25D,
    proj: &ProjectionParams,
    y_top: &[i32],
    y_bottom: &[i32],
) -> bool {
    let corners = [
        (bounds.min_x, bounds.min_y),
        (bounds.max_x, bounds.min_y),
        (bounds.min_x, bounds.max_y),
        (bounds.max_x, bounds.max_y),
    ];

    let mut min_vx = f64::INFINITY;
    let mut max_vx = f64::NEG_INFINITY;
    let mut min_vz = f64::INFINITY;
    let mut max_vz = f64::NEG_INFINITY;
    let mut all_left = true;
    let mut all_right = true;

    for &(cx, cy) in &corners {
        let dx = cx - cam.pos_x;
        let dy = cy - cam.pos_y;
        let vx = dx * cam.right_x + dy * cam.right_y;
        let vz = dx * cam.dir_x + dy * cam.dir_y;
        min_vx = min_vx.min(vx);
        max_vx = max_vx.max(vx);
        min_vz = min_vz.min(vz);
        max_vz = max_vz.max(vz);
        if vz > 0.0 {
            let limit = vz * proj.tan_half_fov + 1.0e-4;
            if vx >= -limit {
                all_left = false;
            }
            if vx <= limit {
                all_right = false;
            }
        } else {
            // If behind the camera plane, only cull if strictly on one side of the cone.
            if vx >= 0.0 {
                all_left = false;
            }
            if vx <= 0.0 {
                all_right = false;
            }
        }
    }

    if max_vz < proj.near_dist {
        return true;
    }
    if all_left || all_right {
        return true;
    }

    // If the entire AABB lies strictly in front of the near plane, check whether
    // its projected horizontal screen interval is already 100% occluded.
    if min_vz >= proj.near_dist {
        let sx_min_f =
            proj.half_w + (min_vx / if min_vx < 0.0 { min_vz } else { max_vz }) * proj.focal;
        let sx_max_f =
            proj.half_w + (max_vx / if max_vx > 0.0 { min_vz } else { max_vz }) * proj.focal;
        let x0_f = (sx_min_f - 0.5).floor().max(0.0);
        let x1_f = (sx_max_f - 0.5).ceil().min((proj.fb_width as f64) - 1.0);
        if !x0_f.is_finite() || !x1_f.is_finite() || x0_f > x1_f {
            return true;
        }
        let x0 = x0_f as usize;
        let x1 = (x1_f as usize).min(proj.fb_width - 1);
        for x in x0..=x1 {
            if let (Some(&t), Some(&b)) = (y_top.get(x), y_bottom.get(x)) {
                if t <= b {
                    return false;
                }
            }
        }
        return true;
    }

    false
}

#[allow(clippy::too_many_arguments)]
#[inline]
fn sample_plane_column(
    fb_data: &mut [u8],
    fb_width: usize,
    x: usize,
    y_start: i32,
    y_end: i32,
    plane_height: f64,
    surface: &SurfaceStyle,
    light_level: u8,
    is_ceiling: bool,
    cam: &Camera25D,
    proj: &ProjectionParams,
    textures: &[TextureRgba],
) {
    if y_start > y_end {
        return;
    }
    let y0 = y_start.max(0) as usize;
    let y1 = y_end.min(proj.fb_height as i32 - 1) as usize;
    if y0 > y1 {
        return;
    }

    let tex_opt = surface.texture_index.and_then(|id| textures.get(id));
    match tex_opt {
        None => {
            let shaded = shade_rgba(surface.color, light_level);
            for y in y0..=y1 {
                write_pixel(fb_data, fb_width, x, y, shaded);
            }
        }
        Some(tex) => {
            let rx = ((x as f64 + 0.5) - proj.half_w) / proj.focal;
            let ray_dir_x = cam.dir_x + cam.right_x * rx;
            let ray_dir_y = cam.dir_y + cam.right_y * rx;
            let dz = if is_ceiling {
                plane_height - cam.pos_z
            } else {
                cam.pos_z - plane_height
            };
            if dz <= EPSILON {
                let shaded = shade_rgba(surface.color, light_level);
                for y in y0..=y1 {
                    write_pixel(fb_data, fb_width, x, y, shaded);
                }
                return;
            }
            for y in y0..=y1 {
                let dy = if is_ceiling {
                    (proj.horizon_y - (y as f64 + 0.5)).max(0.5)
                } else {
                    ((y as f64 + 0.5) - proj.horizon_y).max(0.5)
                };
                let row_dist = (dz * proj.focal) / dy;
                let wx = cam.pos_x + row_dist * ray_dir_x;
                let wy = cam.pos_y + row_dist * ray_dir_y;
                let texel = tex.sample_uv(wx, wy);
                write_pixel(fb_data, fb_width, x, y, shade_rgba(texel, light_level));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[inline]
fn sample_wall_column(
    fb_data: &mut [u8],
    fb_width: usize,
    x: usize,
    y_start: i32,
    y_end: i32,
    wall_top_z: f64,
    inv_z: f64,
    wall_u: f64,
    surface: &SurfaceStyle,
    light_level: u8,
    cam: &Camera25D,
    proj: &ProjectionParams,
    textures: &[TextureRgba],
) {
    if y_start > y_end {
        return;
    }
    let y0 = y_start.max(0) as usize;
    let y1 = y_end.min(proj.fb_height as i32 - 1) as usize;
    if y0 > y1 {
        return;
    }

    let tex_opt = surface.texture_index.and_then(|id| textures.get(id));
    match tex_opt {
        None => {
            let shaded = shade_rgba(surface.color, light_level);
            for y in y0..=y1 {
                write_pixel(fb_data, fb_width, x, y, shaded);
            }
        }
        Some(tex) => {
            for y in y0..=y1 {
                let world_z = cam.pos_z + (proj.horizon_y - (y as f64 + 0.5)) / inv_z;
                let wall_v = wall_top_z - world_z;
                let texel = tex.sample_uv(wall_u, wall_v);
                write_pixel(fb_data, fb_width, x, y, shade_rgba(texel, light_level));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_seg(
    seg: &Seg,
    map: &Map25D,
    cam: &Camera25D,
    proj: &ProjectionParams,
    textures: &[TextureRgba],
    y_top: &mut [i32],
    y_bottom: &mut [i32],
    open_columns: &mut usize,
    fb_data: &mut [u8],
    stats: &mut RenderStats,
) {
    stats.segs_tested += 1;

    // Transform endpoints into camera view space (vx = right, vz = forward).
    let dx0 = seg.v0.x - cam.pos_x;
    let dy0 = seg.v0.y - cam.pos_y;
    let mut vx0 = dx0 * cam.right_x + dy0 * cam.right_y;
    let mut vz0 = dx0 * cam.dir_x + dy0 * cam.dir_y;

    let dx1 = seg.v1.x - cam.pos_x;
    let dy1 = seg.v1.y - cam.pos_y;
    let mut vx1 = dx1 * cam.right_x + dy1 * cam.right_y;
    let mut vz1 = dx1 * cam.dir_x + dy1 * cam.dir_y;

    // Near-plane clipping in camera space so projection never divides by <= 0 (CWE-369).
    if vz0 < proj.near_dist && vz1 < proj.near_dist {
        return;
    }

    let mut u0 = seg.u_start;
    let mut u1 = seg.u_end;

    if vz0 < proj.near_dist {
        let denom = vz1 - vz0;
        if denom.abs() <= EPSILON {
            return;
        }
        let t = ((proj.near_dist - vz0) / denom).clamp(0.0, 1.0);
        vx0 += (vx1 - vx0) * t;
        vz0 = proj.near_dist;
        u0 += (u1 - u0) * t;
    } else if vz1 < proj.near_dist {
        let denom = vz0 - vz1;
        if denom.abs() <= EPSILON {
            return;
        }
        let t = ((proj.near_dist - vz1) / denom).clamp(0.0, 1.0);
        vx1 += (vx0 - vx1) * t;
        vz1 = proj.near_dist;
        u1 += (u0 - u1) * t;
    }

    let sx0 = proj.half_w + (vx0 / vz0) * proj.focal;
    let sx1 = proj.half_w + (vx1 / vz1) * proj.focal;

    // Backface & offscreen horizontal culling in f64 before integer casts (CWE-190).
    if !sx0.is_finite() || !sx1.is_finite() || sx1 <= sx0 + 1.0e-9 {
        return;
    }
    if sx1 <= 0.0 || sx0 >= proj.fb_width as f64 {
        return;
    }

    let x_start_f = (sx0 - 0.5).ceil().max(0.0);
    let x_end_f = (sx1 - 0.5).floor().min((proj.fb_width as f64) - 1.0);
    if !x_start_f.is_finite() || !x_end_f.is_finite() || x_start_f > x_end_f {
        return;
    }

    let x_start = x_start_f as usize;
    let x_end = (x_end_f as usize).min(proj.fb_width - 1);

    let Some(front_sector) = map.sectors.get(seg.front_sector) else {
        return;
    };
    let Some(linedef) = map.linedefs.get(seg.linedef_index) else {
        return;
    };
    let back_sector_opt = seg.back_sector.and_then(|id| map.sectors.get(id));

    let seg_dvx = vx1 - vx0;
    let seg_dvz = vz1 - vz0;
    let sx_span = sx1 - sx0;
    let mut drew_any_column = false;

    for x in x_start..=x_end {
        let (col_top, col_bot) = match (y_top.get(x), y_bottom.get(x)) {
            (Some(&t), Some(&b)) => (t, b),
            _ => continue,
        };
        if col_top > col_bot {
            continue;
        }
        drew_any_column = true;

        let screen_x = x as f64 + 0.5;
        let rx = (screen_x - proj.half_w) / proj.focal;
        let den = seg_dvx - rx * seg_dvz;
        let s = if den.abs() > EPSILON {
            ((rx * vz0 - vx0) / den).clamp(0.0, 1.0)
        } else {
            ((screen_x - sx0) / sx_span).clamp(0.0, 1.0)
        };

        let perp_dist = (vz0 + s * seg_dvz).max(MIN_PERP_DIST);
        let wall_u = u0 + s * (u1 - u0);
        let inv_z = proj.focal / perp_dist;

        let y_ceil_f = proj.horizon_y - (front_sector.ceil_height - cam.pos_z) * inv_z;
        let y_floor_f = proj.horizon_y - (front_sector.floor_height - cam.pos_z) * inv_z;

        let open_top = clamp_screen_y_i32(y_ceil_f.ceil(), col_top, col_bot.saturating_add(1));
        let open_bot = clamp_screen_y_i32(y_floor_f.floor(), col_top.saturating_sub(1), col_bot);

        // Draw visible ceiling span of front_sector above the opening.
        if front_sector.ceil_height > cam.pos_z && col_top < open_top {
            let ceil_end = (open_top - 1).min(col_bot);
            sample_plane_column(
                fb_data,
                proj.fb_width,
                x,
                col_top,
                ceil_end,
                front_sector.ceil_height,
                &front_sector.ceil_surface,
                front_sector.light_level,
                true,
                cam,
                proj,
                textures,
            );
        }

        // Draw visible floor span of front_sector below the opening.
        if front_sector.floor_height < cam.pos_z && open_bot < col_bot {
            let floor_start = (open_bot + 1).max(col_top);
            sample_plane_column(
                fb_data,
                proj.fb_width,
                x,
                floor_start,
                col_bot,
                front_sector.floor_height,
                &front_sector.floor_surface,
                front_sector.light_level,
                false,
                cam,
                proj,
                textures,
            );
        }

        let wall_top = open_top.max(col_top);
        let wall_bot = open_bot.min(col_bot);

        match back_sector_opt {
            None => {
                // 1-sided solid wall: draw middle texture and close the column.
                if wall_top <= wall_bot {
                    sample_wall_column(
                        fb_data,
                        proj.fb_width,
                        x,
                        wall_top,
                        wall_bot,
                        front_sector.ceil_height,
                        inv_z,
                        wall_u,
                        &linedef.middle,
                        front_sector.light_level,
                        cam,
                        proj,
                        textures,
                    );
                }
                if let Some(slot) = y_top.get_mut(x) {
                    *slot = col_bot.saturating_add(1);
                }
                *open_columns = open_columns.saturating_sub(1);
            }
            Some(back_sector) => {
                let b_ceil_f = proj.horizon_y - (back_sector.ceil_height - cam.pos_z) * inv_z;
                let b_floor_f = proj.horizon_y - (back_sector.floor_height - cam.pos_z) * inv_z;
                let portal_top =
                    clamp_screen_y_i32(b_ceil_f.ceil(), col_top, col_bot.saturating_add(1));
                let portal_bot =
                    clamp_screen_y_i32(b_floor_f.floor(), col_top.saturating_sub(1), col_bot);

                let mut new_top = col_top.max(open_top);
                let mut new_bot = col_bot.min(open_bot);

                if back_sector.ceil_height <= back_sector.floor_height {
                    // Closed portal / door: partition wall_top..=wall_bot seamlessly with no 1px seam.
                    if back_sector.ceil_height < front_sector.ceil_height
                        && back_sector.floor_height > front_sector.floor_height
                    {
                        let upper_bot = portal_bot.min(wall_bot);
                        if wall_top <= upper_bot {
                            sample_wall_column(
                                fb_data,
                                proj.fb_width,
                                x,
                                wall_top,
                                upper_bot,
                                front_sector.ceil_height,
                                inv_z,
                                wall_u,
                                &linedef.upper,
                                front_sector.light_level,
                                cam,
                                proj,
                                textures,
                            );
                        }
                        let lower_top = upper_bot.saturating_add(1).max(wall_top);
                        if lower_top <= wall_bot {
                            sample_wall_column(
                                fb_data,
                                proj.fb_width,
                                x,
                                lower_top,
                                wall_bot,
                                back_sector.floor_height,
                                inv_z,
                                wall_u,
                                &linedef.lower,
                                front_sector.light_level,
                                cam,
                                proj,
                                textures,
                            );
                        }
                    } else if back_sector.ceil_height < front_sector.ceil_height {
                        if wall_top <= wall_bot {
                            sample_wall_column(
                                fb_data,
                                proj.fb_width,
                                x,
                                wall_top,
                                wall_bot,
                                front_sector.ceil_height,
                                inv_z,
                                wall_u,
                                &linedef.upper,
                                front_sector.light_level,
                                cam,
                                proj,
                                textures,
                            );
                        }
                    } else if back_sector.floor_height > front_sector.floor_height
                        && wall_top <= wall_bot
                    {
                        sample_wall_column(
                            fb_data,
                            proj.fb_width,
                            x,
                            wall_top,
                            wall_bot,
                            back_sector.floor_height,
                            inv_z,
                            wall_u,
                            &linedef.lower,
                            front_sector.light_level,
                            cam,
                            proj,
                            textures,
                        );
                    }
                    new_top = col_bot.saturating_add(1);
                } else {
                    if back_sector.ceil_height < front_sector.ceil_height {
                        let upper_bot = (portal_top - 1).min(wall_bot);
                        if wall_top <= upper_bot {
                            sample_wall_column(
                                fb_data,
                                proj.fb_width,
                                x,
                                wall_top,
                                upper_bot,
                                front_sector.ceil_height,
                                inv_z,
                                wall_u,
                                &linedef.upper,
                                front_sector.light_level,
                                cam,
                                proj,
                                textures,
                            );
                        }
                        new_top = new_top.max(portal_top);
                    }

                    if back_sector.floor_height > front_sector.floor_height {
                        let lower_top = (portal_bot + 1).max(wall_top);
                        if lower_top <= wall_bot {
                            sample_wall_column(
                                fb_data,
                                proj.fb_width,
                                x,
                                lower_top,
                                wall_bot,
                                back_sector.floor_height,
                                inv_z,
                                wall_u,
                                &linedef.lower,
                                front_sector.light_level,
                                cam,
                                proj,
                                textures,
                            );
                        }
                        new_bot = new_bot.min(portal_bot);
                    }
                }

                if let Some(slot) = y_top.get_mut(x) {
                    *slot = new_top;
                }
                if let Some(slot) = y_bottom.get_mut(x) {
                    *slot = new_bot;
                }
                if new_top > new_bot {
                    *open_columns = open_columns.saturating_sub(1);
                }
            }
        }
    }

    if drew_any_column {
        stats.segs_drawn += 1;
        if back_sector_opt.is_some() {
            stats.portals_traversed += 1;
        }
    }
}

/// Render a 2.5D BSP/portal scene into caller-provided `framebuffer` and `scratch`.
///
/// Steady-state execution performs **zero heap allocation** (`VER07`): all
/// column occlusion spans, iterative BSP traversal stack frames, and dynamic
/// visibility lists live in `scratch`, and all RGBA8 pixels are written directly
/// into `framebuffer.data`.
pub fn render_frame_into(
    map: &Map25D,
    bsp: &BspTree,
    cam: &Camera25D,
    textures: &[TextureRgba],
    config: &RenderConfig,
    scratch: &mut RenderScratch,
    framebuffer: &mut PixelContainer,
) -> Result<RenderStats, BspError> {
    let (norm_cam, proj) = validate_render_inputs(map, bsp, cam, textures, scratch, framebuffer)?;
    let cam = &norm_cam;

    let fb_width = proj.fb_width;
    let fb_height = proj.fb_height;

    // Clear framebuffer with background color (zero allocation).
    let (chunks, _) = framebuffer.data.as_chunks_mut::<4>();
    for px in chunks {
        *px = config.background_color;
    }

    // Reset pre-allocated scratch buffers in place without reallocating.
    let y_bot_init = (fb_height as i32) - 1;
    for x in 0..fb_width {
        if let Some(t) = scratch.y_top.get_mut(x) {
            *t = 0;
        }
        if let Some(b) = scratch.y_bottom.get_mut(x) {
            *b = y_bot_init;
        }
    }
    scratch.traverse_stack.clear();
    scratch.visible_sectors.clear();
    scratch.visible_subsectors.clear();

    scratch.frame_gen = scratch.frame_gen.wrapping_add(1);
    if scratch.frame_gen == 0 {
        for g in &mut scratch.sector_gen {
            *g = 0;
        }
        scratch.frame_gen = 1;
    }
    let gen = scratch.frame_gen;

    let mut open_columns = fb_width;
    let mut stats = RenderStats {
        columns_drawn: fb_width,
        nodes_visited: 0,
        nodes_culled: 0,
        subsectors_visited: 0,
        segs_tested: 0,
        segs_drawn: 0,
        portals_traversed: 0,
        early_occlusion_exits: false,
    };

    scratch.traverse_stack.push(0);
    let cam_pos = cam.camera_pos();

    while let Some(node_idx) = scratch.traverse_stack.pop() {
        if open_columns == 0 {
            stats.early_occlusion_exits = true;
            break;
        }

        let Some(&node) = bsp.nodes.get(node_idx) else {
            continue;
        };

        if config.enable_node_culling
            && node_is_culled(
                node.bounds,
                cam,
                &proj,
                &scratch.y_top[..fb_width],
                &scratch.y_bottom[..fb_width],
            )
        {
            stats.nodes_culled += 1;
            continue;
        }

        stats.nodes_visited += 1;

        match node.kind {
            BspNodeKind::Internal => {
                if scratch.traverse_stack.len().saturating_add(2)
                    > scratch.traverse_stack.capacity()
                {
                    scratch.traverse_stack.clear();
                    scratch.visible_sectors.clear();
                    scratch.visible_subsectors.clear();
                    return Err(BspError::BspLimitExceeded);
                }
                let sd = line_signed_dist(node.partition_v0, node.partition_v1, cam_pos);
                // LIFO stack: push far child first so near child pops next (front-to-back).
                if sd >= 0.0 {
                    scratch.traverse_stack.push(node.back_child);
                    scratch.traverse_stack.push(node.front_child);
                } else {
                    scratch.traverse_stack.push(node.front_child);
                    scratch.traverse_stack.push(node.back_child);
                }
            }
            BspNodeKind::Leaf => {
                stats.subsectors_visited += 1;
                scratch.visible_subsectors.push(node_idx);

                let seg_end = node.seg_start.saturating_add(node.seg_count);
                let Some(leaf_segs) = bsp.segs.get(node.seg_start..seg_end) else {
                    continue;
                };

                for seg in leaf_segs {
                    if let Some(g_slot) = scratch.sector_gen.get_mut(seg.front_sector) {
                        if *g_slot != gen {
                            *g_slot = gen;
                            scratch.visible_sectors.push(seg.front_sector);
                        }
                    }
                    draw_seg(
                        seg,
                        map,
                        cam,
                        &proj,
                        textures,
                        &mut scratch.y_top[..fb_width],
                        &mut scratch.y_bottom[..fb_width],
                        &mut open_columns,
                        &mut framebuffer.data,
                        &mut stats,
                    );
                }
            }
        }
    }

    Ok(stats)
}

// ---------------------------------------------------------------------------
// Deterministic Digests
// ---------------------------------------------------------------------------

/// Compute a deterministic 64-bit FNV-1a digest over a [`PixelContainer`] framebuffer.
pub fn frame_digest_fnv1a(framebuffer: &PixelContainer) -> u64 {
    let mut state = 0xcbf2_9ce4_8422_2325_u64;
    fnv1a_update_bytes(&mut state, &framebuffer.width.to_le_bytes());
    fnv1a_update_bytes(&mut state, &framebuffer.height.to_le_bytes());
    fnv1a_update_bytes(&mut state, &framebuffer.data);
    state
}

/// Compute a deterministic 64-bit FNV-1a digest over a [`Map25D`] and its compiled [`BspTree`].
pub fn scene_digest_fnv1a(map: &Map25D, bsp: &BspTree) -> u64 {
    let mut state = 0xcbf2_9ce4_8422_2325_u64;
    fnv1a_update_bytes(&mut state, &(map.sectors.len() as u64).to_le_bytes());
    for s in &map.sectors {
        fnv1a_update_f64(&mut state, s.floor_height);
        fnv1a_update_f64(&mut state, s.ceil_height);
        fnv1a_update_bytes(&mut state, &[s.light_level]);
        fnv1a_update_bytes(&mut state, &s.floor_surface.color);
        fnv1a_update_bytes(&mut state, &s.ceil_surface.color);
    }
    fnv1a_update_bytes(&mut state, &(map.linedefs.len() as u64).to_le_bytes());
    for ld in &map.linedefs {
        fnv1a_update_f64(&mut state, ld.v0.x);
        fnv1a_update_f64(&mut state, ld.v0.y);
        fnv1a_update_f64(&mut state, ld.v1.x);
        fnv1a_update_f64(&mut state, ld.v1.y);
        fnv1a_update_bytes(&mut state, &(ld.front_sector as u64).to_le_bytes());
        let back_enc = ld.back_sector.map_or(u64::MAX, |b| b as u64);
        fnv1a_update_bytes(&mut state, &back_enc.to_le_bytes());
    }
    fnv1a_update_bytes(&mut state, &(bsp.nodes.len() as u64).to_le_bytes());
    fnv1a_update_bytes(&mut state, &(bsp.segs.len() as u64).to_le_bytes());
    for seg in &bsp.segs {
        fnv1a_update_f64(&mut state, seg.v0.x);
        fnv1a_update_f64(&mut state, seg.v0.y);
        fnv1a_update_f64(&mut state, seg.v1.x);
        fnv1a_update_f64(&mut state, seg.v1.y);
        fnv1a_update_bytes(&mut state, &(seg.front_sector as u64).to_le_bytes());
    }
    state
}
