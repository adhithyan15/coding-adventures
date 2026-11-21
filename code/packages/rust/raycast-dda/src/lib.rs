//! # raycast-dda
//!
//! GAME31: deterministic, zero-allocation (steady-state) 2.5D orthogonal grid
//! Digital Differential Analyzer (DDA) wall raycaster, perspective floor/ceiling
//! plane caster, and Z-buffered depth-sorted billboard sprite renderer.
//!
//! Renders directly into a caller-provided [`PixelContainer`] (`IMG00`) and
//! Z-buffer slice (`&mut [f64]`), bridging Era 3 2.5D dungeon/corridor rendering
//! with `math3d` (`GAME30`) and `trig` (`PHY00`).
//!
//! ## Coordinate System & DDA Projection
//!
//! ```text
//!   Grid cell (mx, my) in [0..width) x [0..height)
//!   Camera: pos = (pos_x, pos_y), forward dir = (dir_x, dir_y), plane = (plane_x, plane_y)
//!
//!   For screen column x in 0..fb_width:
//!     camera_x = 2.0 * (x + 0.5) / fb_width - 1.0       // in (-1, +1)
//!     ray_dir  = dir + plane * camera_x
//!
//!   Because |ray_dir| scales with sec(theta) relative to dir, the DDA step
//!   distance (side_dist - delta_dist) equals the perpendicular distance to the
//!   camera plane directly, eliminating fisheye wall curvature.
//! ```
//!
//! ## Safety and Resource Model
//!
//! * `#![forbid(unsafe_code)]`; zero pointer arithmetic anywhere.
//! * Steady-state [`render_frame_into`] performs **zero heap allocation**: all
//!   output pixels, Z-buffer depths, and sprite sort entries live in caller-owned
//!   buffers (`PixelContainer`, `&mut [f64]`, [`SpriteScratch`]), sorted in place
//!   with `slice::sort_unstable_by`.
//! * Bounded resources (CWE-400 / CWE-835): grid dimensions capped at
//!   [`MAX_MAP_DIM`], framebuffer dimensions at [`MAX_FRAME_WIDTH`] x
//!   [`MAX_FRAME_HEIGHT`], texture dimensions at [`MAX_TEXTURE_DIM`], sprite
//!   counts at [`MAX_SPRITES`], and DDA traversal strictly capped at
//!   `grid.width + grid.height` steps per ray so open maps always terminate.
//! * Finite & non-zero guards (CWE-20 / CWE-369): rejects `NaN` / `+/-Inf`,
//!   out-of-range coordinates, degenerate camera bases, near-zero ray components,
//!   and clamps minimum perpendicular distance to [`MIN_PERP_DIST`].
//! * Checked indexing (CWE-125 / CWE-787 / CWE-190): validates all grid,
//!   texture, Z-buffer, scratch, and framebuffer dimensions upfront and uses
//!   checked indexing on every read and write.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Generated capability verification witnesses.
pub mod capabilities;

pub use capabilities::PureCapabilities;
pub use math3d::{Ray3, Vec3};
pub use pixel_container::PixelContainer;

/// Packed RGBA8 pixel type alias (`[r, g, b, a]`).
pub type Rgba8 = [u8; 4];

/// Maximum orthogonal grid map dimension along either axis (`CWE-400`).
pub const MAX_MAP_DIM: usize = 1024;

/// Maximum framebuffer width in pixels (`CWE-400`).
pub const MAX_FRAME_WIDTH: u32 = 4096;

/// Maximum framebuffer height in pixels (`CWE-400`).
pub const MAX_FRAME_HEIGHT: u32 = 4096;

/// Maximum texture width or height in pixels (`CWE-400`).
pub const MAX_TEXTURE_DIM: u32 = 2048;

/// Maximum number of sprites processed per frame (`CWE-400`).
pub const MAX_SPRITES: usize = 4096;

/// Maximum finite world coordinate magnitude (`CWE-20` / `CWE-400`).
pub const MAX_COORD: f64 = 1.0e6;

/// Numerical epsilon for near-zero ray component and camera determinant guards (`CWE-369`).
pub const EPSILON: f64 = 1.0e-12;

/// Minimum perpendicular wall/sprite distance used for projection scaling (`CWE-369`).
pub const MIN_PERP_DIST: f64 = 1.0e-6;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Failure modes for every fallible operation in `raycast-dda`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RaycastError {
    /// A floating-point coordinate, direction, angle, or scale is `NaN` or `+/-Infinity` (`CWE-20`).
    NonFiniteInput,
    /// A coordinate, angle, scale, or count exceeds safe bounds (`CWE-20` / `CWE-400`).
    ValueOutOfBounds,
    /// Camera direction or plane has near-zero length or zero determinant (`CWE-369`).
    DegenerateCamera,
    /// Both ray direction components are near zero (`CWE-369`).
    DegenerateRay,
    /// Map width/height is zero, exceeds [`MAX_MAP_DIM`], or cell buffer length mismatches.
    InvalidGrid,
    /// Texture dimensions are zero, exceed [`MAX_TEXTURE_DIM`], or pixel buffer length mismatches.
    InvalidTexture,
    /// Framebuffer dimensions are zero, exceed [`MAX_FRAME_WIDTH`]/[`MAX_FRAME_HEIGHT`], or byte length mismatches.
    InvalidFramebuffer,
    /// Caller Z-buffer slice is shorter than the framebuffer width.
    ZBufferTooSmall {
        /// Required element count (`framebuffer.width`).
        required: usize,
        /// Provided slice length.
        actual: usize,
    },
    /// Sprite count exceeds [`MAX_SPRITES`] (`CWE-400`).
    TooManySprites {
        /// Number of sprites passed.
        count: usize,
        /// Maximum allowed sprites ([`MAX_SPRITES`]).
        max: usize,
    },
    /// Caller [`SpriteScratch`] capacity is smaller than `sprites.len()`.
    ScratchTooSmall {
        /// Required capacity (`sprites.len()`).
        required: usize,
        /// Scratch buffer capacity.
        actual: usize,
    },
    /// A wall tile ID or floor/ceiling/sprite texture index does not exist in `textures`.
    InvalidMaterialId {
        /// The missing material or texture index.
        id: u16,
    },
    /// Checked integer arithmetic overflow (`CWE-190`).
    ArithmeticOverflow,
}

impl std::fmt::Display for RaycastError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "input contains NaN or Infinity"),
            Self::ValueOutOfBounds => write!(f, "value exceeds safe bounds"),
            Self::DegenerateCamera => {
                write!(f, "camera direction and plane form a degenerate basis")
            }
            Self::DegenerateRay => write!(f, "ray direction has near-zero length"),
            Self::InvalidGrid => write!(f, "invalid grid map dimensions or cell buffer"),
            Self::InvalidTexture => write!(f, "invalid texture dimensions or pixel buffer"),
            Self::InvalidFramebuffer => write!(f, "invalid framebuffer dimensions or byte buffer"),
            Self::ZBufferTooSmall { required, actual } => {
                write!(f, "z-buffer too small: need {required}, got {actual}")
            }
            Self::TooManySprites { count, max } => {
                write!(f, "sprite count {count} exceeds maximum {max}")
            }
            Self::ScratchTooSmall { required, actual } => {
                write!(
                    f,
                    "sprite scratch capacity too small: need {required}, got {actual}"
                )
            }
            Self::InvalidMaterialId { id } => {
                write!(f, "material or texture index {id} out of range")
            }
            Self::ArithmeticOverflow => write!(f, "checked arithmetic overflow"),
        }
    }
}

impl std::error::Error for RaycastError {}

// ---------------------------------------------------------------------------
// Validation Helpers
// ---------------------------------------------------------------------------

#[inline]
fn check_finite_coord(v: f64) -> Result<f64, RaycastError> {
    if !v.is_finite() {
        return Err(RaycastError::NonFiniteInput);
    }
    if v.abs() > MAX_COORD {
        return Err(RaycastError::ValueOutOfBounds);
    }
    Ok(v)
}

#[inline]
fn check_finite_bounded(v: f64, limit: f64) -> Result<f64, RaycastError> {
    if !v.is_finite() {
        return Err(RaycastError::NonFiniteInput);
    }
    if v.abs() > limit {
        return Err(RaycastError::ValueOutOfBounds);
    }
    Ok(v)
}

#[inline]
fn check_positive_scale(v: f64) -> Result<f64, RaycastError> {
    if !v.is_finite() {
        return Err(RaycastError::NonFiniteInput);
    }
    if v <= EPSILON || v > MAX_COORD {
        return Err(RaycastError::ValueOutOfBounds);
    }
    Ok(v)
}

#[inline]
fn shade_rgba(rgba: Rgba8, scale_256: u8) -> Rgba8 {
    let s = u16::from(scale_256);
    [
        ((u16::from(rgba[0]) * s) >> 8) as u8,
        ((u16::from(rgba[1]) * s) >> 8) as u8,
        ((u16::from(rgba[2]) * s) >> 8) as u8,
        rgba[3],
    ]
}

#[inline]
fn alpha_blend(dst: Rgba8, src: Rgba8) -> Rgba8 {
    let a = u16::from(src[3]);
    if a == 0 {
        return dst;
    }
    if a == 255 {
        return src;
    }
    let inv_a = 255 - a;
    let r = (u16::from(src[0]) * a + u16::from(dst[0]) * inv_a + 127) / 255;
    let g = (u16::from(src[1]) * a + u16::from(dst[1]) * inv_a + 127) / 255;
    let b = (u16::from(src[2]) * a + u16::from(dst[2]) * inv_a + 127) / 255;
    [r as u8, g as u8, b as u8, 255]
}

// ---------------------------------------------------------------------------
// GridMap
// ---------------------------------------------------------------------------

/// 2D orthogonal tile grid map (`width * height` row-major).
///
/// Cell value `0` represents open traversable space; any non-zero value `k >= 1`
/// represents a solid wall whose texture index is `(k - 1)` in the caller's
/// texture slice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridMap {
    width: usize,
    height: usize,
    cells: Vec<u16>,
}

impl GridMap {
    /// Construct a validated [`GridMap`] from row-major `cells`.
    ///
    /// Rejects zero dimensions, dimensions exceeding [`MAX_MAP_DIM`], and
    /// cell slice length mismatches (`CWE-400` / `CWE-190`).
    pub fn grid_new(width: usize, height: usize, cells: Vec<u16>) -> Result<Self, RaycastError> {
        if width == 0 || height == 0 || width > MAX_MAP_DIM || height > MAX_MAP_DIM {
            return Err(RaycastError::InvalidGrid);
        }
        let expected = width
            .checked_mul(height)
            .ok_or(RaycastError::ArithmeticOverflow)?;
        if cells.len() != expected {
            return Err(RaycastError::InvalidGrid);
        }
        Ok(Self {
            width,
            height,
            cells,
        })
    }

    /// Map width in grid cells.
    #[inline]
    pub fn grid_width(&self) -> usize {
        self.width
    }

    /// Map height in grid cells.
    #[inline]
    pub fn grid_height(&self) -> usize {
        self.height
    }

    /// Borrow the underlying row-major cell slice.
    #[inline]
    pub fn grid_cells(&self) -> &[u16] {
        &self.cells
    }

    /// Checked lookup of the cell at `(x, y)`; returns `None` if out of bounds.
    #[inline]
    pub fn grid_cell_at(&self, x: usize, y: usize) -> Option<u16> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = y.checked_mul(self.width)?.checked_add(x)?;
        self.cells.get(idx).copied()
    }

    /// Returns `true` if `(x, y)` is within bounds and holds a solid wall (`!= 0`).
    #[inline]
    pub fn grid_is_solid(&self, x: usize, y: usize) -> bool {
        matches!(self.grid_cell_at(x, y), Some(v) if v != 0)
    }

    /// Update the cell value at `(x, y)` with bounds checking.
    pub fn grid_set_cell(&mut self, x: usize, y: usize, value: u16) -> Result<(), RaycastError> {
        if x >= self.width || y >= self.height {
            return Err(RaycastError::InvalidGrid);
        }
        let idx = y
            .checked_mul(self.width)
            .and_then(|row| row.checked_add(x))
            .ok_or(RaycastError::ArithmeticOverflow)?;
        let slot = self.cells.get_mut(idx).ok_or(RaycastError::InvalidGrid)?;
        *slot = value;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Camera2D
// ---------------------------------------------------------------------------

/// 2D raycaster camera defined by world position, forward direction, and camera plane.
///
/// The camera plane vector must be non-collinear with the direction vector so the
/// 2x2 projection matrix `[plane, dir]` is invertible for sprite projection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera2D {
    /// Camera X position in grid units.
    pub pos_x: f64,
    /// Camera Y position in grid units.
    pub pos_y: f64,
    /// Forward unit (or scaled) direction X component.
    pub dir_x: f64,
    /// Forward unit (or scaled) direction Y component.
    pub dir_y: f64,
    /// Rightward camera plane X component (`|plane| / |dir| = tan(fov_x / 2)`).
    pub plane_x: f64,
    /// Rightward camera plane Y component.
    pub plane_y: f64,
}

impl Camera2D {
    /// Construct and validate a [`Camera2D`] from explicit position, direction, and plane vectors.
    ///
    /// Enforces finiteness and coordinate bounds (`CWE-20`), non-zero vector lengths,
    /// and a non-singular `(plane, dir)` basis (`|plane_x * dir_y - dir_x * plane_y| > EPSILON`, `CWE-369`).
    pub fn camera_new(
        pos_x: f64,
        pos_y: f64,
        dir_x: f64,
        dir_y: f64,
        plane_x: f64,
        plane_y: f64,
    ) -> Result<Self, RaycastError> {
        let pos_x = check_finite_coord(pos_x)?;
        let pos_y = check_finite_coord(pos_y)?;
        let dir_x = check_finite_coord(dir_x)?;
        let dir_y = check_finite_coord(dir_y)?;
        let plane_x = check_finite_coord(plane_x)?;
        let plane_y = check_finite_coord(plane_y)?;

        let dir_len_sq = dir_x * dir_x + dir_y * dir_y;
        let plane_len_sq = plane_x * plane_x + plane_y * plane_y;
        if !dir_len_sq.is_finite()
            || !plane_len_sq.is_finite()
            || dir_len_sq <= EPSILON * EPSILON
            || plane_len_sq <= EPSILON * EPSILON
        {
            return Err(RaycastError::DegenerateCamera);
        }

        let det = plane_x * dir_y - dir_x * plane_y;
        if !det.is_finite() || det.abs() <= EPSILON {
            return Err(RaycastError::DegenerateCamera);
        }

        Ok(Self {
            pos_x,
            pos_y,
            dir_x,
            dir_y,
            plane_x,
            plane_y,
        })
    }

    /// Construct a [`Camera2D`] from world position, yaw angle (radians), and horizontal FOV (radians).
    ///
    /// Uses `trig::cos` and `trig::sin` (`PHY00`) for bit-identical cross-platform determinism.
    /// At `yaw_rad = 0`, `dir = (1, 0)` and `plane = (0, tan(fov_rad / 2))`.
    pub fn camera_from_yaw_fov(
        pos_x: f64,
        pos_y: f64,
        yaw_rad: f64,
        fov_rad: f64,
    ) -> Result<Self, RaycastError> {
        let pos_x = check_finite_coord(pos_x)?;
        let pos_y = check_finite_coord(pos_y)?;
        let yaw_rad = check_finite_bounded(yaw_rad, math3d::MAX_ANGLE_RAD)?;
        if !fov_rad.is_finite() {
            return Err(RaycastError::NonFiniteInput);
        }
        if fov_rad <= EPSILON || fov_rad >= trig::PI - EPSILON {
            return Err(RaycastError::ValueOutOfBounds);
        }

        let half_fov = 0.5 * fov_rad;
        let sin_half = trig::sin(half_fov);
        let cos_half = trig::cos(half_fov);
        if cos_half.abs() <= EPSILON {
            return Err(RaycastError::DegenerateCamera);
        }
        let plane_scale = sin_half / cos_half;

        let dir_x = trig::cos(yaw_rad);
        let dir_y = trig::sin(yaw_rad);
        // Rightward camera plane perpendicular to `dir = (cos, sin)` is `(-sin, cos) * tan(fov/2)`.
        let plane_x = -dir_y * plane_scale;
        let plane_y = dir_x * plane_scale;

        Self::camera_new(pos_x, pos_y, dir_x, dir_y, plane_x, plane_y)
    }

    /// Return a new [`Camera2D`] rotated counter-clockwise by `delta_yaw_rad` radians via `PHY00` `trig`.
    pub fn camera_rotate(&self, delta_yaw_rad: f64) -> Result<Self, RaycastError> {
        let delta = check_finite_bounded(delta_yaw_rad, math3d::MAX_ANGLE_RAD)?;
        let cos_a = trig::cos(delta);
        let sin_a = trig::sin(delta);

        let dir_x = self.dir_x * cos_a - self.dir_y * sin_a;
        let dir_y = self.dir_x * sin_a + self.dir_y * cos_a;
        let plane_x = self.plane_x * cos_a - self.plane_y * sin_a;
        let plane_y = self.plane_x * sin_a + self.plane_y * cos_a;

        Self::camera_new(self.pos_x, self.pos_y, dir_x, dir_y, plane_x, plane_y)
    }

    /// Translate the camera position by `(dx, dy)` after checking finite bounds.
    pub fn camera_move_by(&self, dx: f64, dy: f64) -> Result<Self, RaycastError> {
        let dx = check_finite_coord(dx)?;
        let dy = check_finite_coord(dy)?;
        Self::camera_new(
            self.pos_x + dx,
            self.pos_y + dy,
            self.dir_x,
            self.dir_y,
            self.plane_x,
            self.plane_y,
        )
    }

    /// Return camera world position as `(pos_x, pos_y)`.
    #[inline]
    pub const fn camera_pos(&self) -> (f64, f64) {
        (self.pos_x, self.pos_y)
    }

    /// Return camera forward direction as `(dir_x, dir_y)`.
    #[inline]
    pub const fn camera_dir(&self) -> (f64, f64) {
        (self.dir_x, self.dir_y)
    }

    /// Return camera plane vector as `(plane_x, plane_y)`.
    #[inline]
    pub const fn camera_plane(&self) -> (f64, f64) {
        (self.plane_x, self.plane_y)
    }

    /// Convert the 2D forward direction into a normalized `math3d::Vec3` on the XZ ground plane `(dir_x, 0, dir_y)`.
    pub fn camera_forward_vec3(&self) -> Result<Vec3, RaycastError> {
        let v = Vec3::try_vec3(self.dir_x, 0.0, self.dir_y)
            .map_err(|_| RaycastError::ValueOutOfBounds)?;
        v.normalize().map_err(|_| RaycastError::DegenerateCamera)
    }

    /// Build a `math3d::Ray3` on the XZ plane (`y = 0`) from the camera position and forward direction.
    pub fn camera_world_ray(&self) -> Result<Ray3, RaycastError> {
        let origin = Vec3::try_vec3(self.pos_x, 0.0, self.pos_y)
            .map_err(|_| RaycastError::ValueOutOfBounds)?;
        let dir = self.camera_forward_vec3()?;
        Ray3::ray3_from_origin_dir(origin, dir).map_err(|_| RaycastError::DegenerateCamera)
    }
}

// ---------------------------------------------------------------------------
// TextureRgba
// ---------------------------------------------------------------------------

/// Row-major RGBA8 texture (`width * height * 4` bytes) for walls, floors, ceilings, and sprites.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextureRgba {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl TextureRgba {
    /// Construct a validated [`TextureRgba`] from raw RGBA8 bytes.
    pub fn texture_new(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, RaycastError> {
        if width == 0 || height == 0 || width > MAX_TEXTURE_DIM || height > MAX_TEXTURE_DIM {
            return Err(RaycastError::InvalidTexture);
        }
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or(RaycastError::ArithmeticOverflow)?;
        if pixels.len() != expected {
            return Err(RaycastError::InvalidTexture);
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// Wrap a [`PixelContainer`] (`IMG00`) as a validated [`TextureRgba`].
    pub fn texture_from_pixel_container(container: &PixelContainer) -> Result<Self, RaycastError> {
        Self::texture_new(container.width, container.height, container.data.clone())
    }

    /// Create a solid-color [`TextureRgba`] filled with `color`.
    pub fn texture_solid(width: u32, height: u32, color: Rgba8) -> Result<Self, RaycastError> {
        if width == 0 || height == 0 || width > MAX_TEXTURE_DIM || height > MAX_TEXTURE_DIM {
            return Err(RaycastError::InvalidTexture);
        }
        let count = (width as usize)
            .checked_mul(height as usize)
            .ok_or(RaycastError::ArithmeticOverflow)?;
        let byte_len = count
            .checked_mul(4)
            .ok_or(RaycastError::ArithmeticOverflow)?;
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
    ) -> Result<Self, RaycastError> {
        if width == 0
            || height == 0
            || cell_size == 0
            || width > MAX_TEXTURE_DIM
            || height > MAX_TEXTURE_DIM
        {
            return Err(RaycastError::InvalidTexture);
        }
        let count = (width as usize)
            .checked_mul(height as usize)
            .ok_or(RaycastError::ArithmeticOverflow)?;
        let byte_len = count
            .checked_mul(4)
            .ok_or(RaycastError::ArithmeticOverflow)?;
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

    /// Checked texel lookup at `(tx, ty)`. Clamps out-of-range coordinates safely without panicking.
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
}

// ---------------------------------------------------------------------------
// Sprites & Pre-Allocated Scratch
// ---------------------------------------------------------------------------

/// World-space billboard sprite rendered after walls using column Z-buffer occlusion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sprite {
    /// World X position.
    pub x: f64,
    /// World Y position.
    pub y: f64,
    /// Index into the caller's `textures` slice.
    pub texture_index: usize,
    /// Horizontal billboard scale factor (`1.0` = 1 grid unit wide).
    pub scale_u: f64,
    /// Vertical billboard scale factor (`1.0` = 1 grid unit tall).
    pub scale_v: f64,
    /// Vertical screen-space offset in world height units (`0.0` = centered on horizon).
    pub z_offset: f64,
}

impl Sprite {
    /// Construct a unit-scale billboard [`Sprite`] at `(x, y)` using `texture_index`.
    pub fn sprite_new(x: f64, y: f64, texture_index: usize) -> Result<Self, RaycastError> {
        Self::sprite_with_scale(x, y, texture_index, 1.0, 1.0, 0.0)
    }

    /// Construct a scaled and vertically offset billboard [`Sprite`].
    pub fn sprite_with_scale(
        x: f64,
        y: f64,
        texture_index: usize,
        scale_u: f64,
        scale_v: f64,
        z_offset: f64,
    ) -> Result<Self, RaycastError> {
        Ok(Self {
            x: check_finite_coord(x)?,
            y: check_finite_coord(y)?,
            texture_index,
            scale_u: check_positive_scale(scale_u)?,
            scale_v: check_positive_scale(scale_v)?,
            z_offset: check_finite_coord(z_offset)?,
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct SpriteSortEntry {
    index: usize,
    dist_sq: f64,
}

/// Pre-allocated sprite sorting workspace enabling zero-allocation steady-state rendering.
#[derive(Clone, Debug)]
pub struct SpriteScratch {
    entries: Vec<SpriteSortEntry>,
}

impl SpriteScratch {
    /// Allocate a [`SpriteScratch`] buffer able to sort up to `capacity` sprites without reallocating.
    ///
    /// Rejects capacities exceeding [`MAX_SPRITES`] (`CWE-400`).
    pub fn scratch_with_capacity(capacity: usize) -> Result<Self, RaycastError> {
        if capacity > MAX_SPRITES {
            return Err(RaycastError::TooManySprites {
                count: capacity,
                max: MAX_SPRITES,
            });
        }
        Ok(Self {
            entries: Vec::with_capacity(capacity),
        })
    }

    /// Maximum sprite count supported without heap reallocation.
    #[inline]
    pub fn scratch_capacity(&self) -> usize {
        self.entries.capacity()
    }

    /// Number of entries populated in the most recent render call.
    #[inline]
    pub fn scratch_len(&self) -> usize {
        self.entries.len()
    }
}

// ---------------------------------------------------------------------------
// Render Configuration & Hit / Frame Results
// ---------------------------------------------------------------------------

/// Which orthogonal grid face a DDA ray intersected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WallSide {
    /// Vertical grid line (`X = integer`), facing East or West.
    EastWest,
    /// Horizontal grid line (`Y = integer`), facing North or South.
    NorthSouth,
}

/// Result of casting a single 2D DDA ray through a [`GridMap`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WallHit {
    /// `true` if the ray struck a non-zero wall cell within `width + height` steps.
    pub hit: bool,
    /// Non-zero grid tile ID struck (`0` on miss).
    pub tile_id: u16,
    /// Grid X coordinate of the struck cell (`0` on miss).
    pub map_x: usize,
    /// Grid Y coordinate of the struck cell (`0` on miss).
    pub map_y: usize,
    /// Fisheye-free perpendicular distance from the camera plane (clamped `>= MIN_PERP_DIST`).
    pub perp_wall_dist: f64,
    /// Struck wall orientation (`EastWest` vs `NorthSouth`) for side shading.
    pub side: WallSide,
    /// Horizontal texture coordinate in `[0.0, 1.0)` along the struck wall face.
    pub wall_u: f64,
    /// Number of DDA grid boundary steps executed (`<= grid.width + grid.height`).
    pub steps_taken: usize,
}

/// Surface description for horizontal floor and ceiling planes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaneSurface {
    /// Uniform solid RGBA8 fill.
    Solid(Rgba8),
    /// Perspective-cast tiled texture sampled from `textures[texture_index]`.
    Textured {
        /// Index into the caller's `textures` slice.
        texture_index: usize,
    },
}

/// Frame rendering configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderConfig {
    /// Ceiling surface (top half of framebuffer).
    pub ceiling: PlaneSurface,
    /// Floor surface (bottom half of framebuffer).
    pub floor: PlaneSurface,
    /// Brightness scale (`0..=255`, where `128` = 50% brightness) applied to [`WallSide::NorthSouth`] walls.
    pub ns_wall_shade: u8,
}

impl RenderConfig {
    /// Default Wolf3D-style solid ceiling/floor configuration with 50% N/S wall shading.
    pub const fn render_config_default() -> Self {
        Self {
            ceiling: PlaneSurface::Solid([48, 52, 64, 255]),
            floor: PlaneSurface::Solid([88, 76, 64, 255]),
            ns_wall_shade: 128,
        }
    }

    /// Construct a [`RenderConfig`] with explicit ceiling/floor surfaces and N/S wall shading factor.
    pub const fn render_config_with_planes(
        ceiling: PlaneSurface,
        floor: PlaneSurface,
        ns_wall_shade: u8,
    ) -> Self {
        Self {
            ceiling,
            floor,
            ns_wall_shade,
        }
    }
}

/// Summary telemetry produced by [`render_frame_into`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderStats {
    /// Number of screen columns raycasted (`framebuffer.width`).
    pub columns_cast: usize,
    /// Number of columns whose DDA ray struck a wall cell.
    pub wall_hits: usize,
    /// Total DDA grid steps accumulated across all columns.
    pub total_dda_steps: usize,
    /// Number of sprites projected onto the screen (not behind the camera).
    pub sprites_projected: usize,
}

// ---------------------------------------------------------------------------
// Core DDA Raycaster
// ---------------------------------------------------------------------------

/// Cast a single 2D ray from `(origin_x, origin_y)` along `(ray_dir_x, ray_dir_y)`
/// through `grid` using the Digital Differential Analyzer (DDA) algorithm.
///
/// Guarantees termination on any map (including completely open maps) within
/// `max_steps = grid.width + grid.height` iterations (`CWE-400` / `CWE-835`).
/// Near-zero ray components are guarded against division by zero (`CWE-369`),
/// and `perp_wall_dist` is clamped to `>= MIN_PERP_DIST`.
pub fn cast_dda_ray(
    grid: &GridMap,
    origin_x: f64,
    origin_y: f64,
    ray_dir_x: f64,
    ray_dir_y: f64,
) -> Result<WallHit, RaycastError> {
    let origin_x = check_finite_coord(origin_x)?;
    let origin_y = check_finite_coord(origin_y)?;
    let ray_dir_x = check_finite_coord(ray_dir_x)?;
    let ray_dir_y = check_finite_coord(ray_dir_y)?;

    if ray_dir_x.abs() < EPSILON && ray_dir_y.abs() < EPSILON {
        return Err(RaycastError::DegenerateRay);
    }

    let delta_dist_x = if ray_dir_x.abs() < EPSILON {
        1.0e30
    } else {
        (1.0 / ray_dir_x).abs()
    };
    let delta_dist_y = if ray_dir_y.abs() < EPSILON {
        1.0e30
    } else {
        (1.0 / ray_dir_y).abs()
    };

    let mut map_x = origin_x.floor() as isize;
    let mut map_y = origin_y.floor() as isize;

    // If the origin itself lies inside a solid wall cell, return an immediate
    // clamped hit so callers never divide by zero.
    if map_x >= 0 && map_y >= 0 {
        if let Some(tile) = grid.grid_cell_at(map_x as usize, map_y as usize) {
            if tile != 0 {
                return Ok(WallHit {
                    hit: true,
                    tile_id: tile,
                    map_x: map_x as usize,
                    map_y: map_y as usize,
                    perp_wall_dist: MIN_PERP_DIST,
                    side: WallSide::EastWest,
                    wall_u: 0.0,
                    steps_taken: 0,
                });
            }
        }
    }

    let (step_x, mut side_dist_x) = if ray_dir_x < 0.0 {
        (-1_isize, (origin_x - map_x as f64) * delta_dist_x)
    } else {
        (1_isize, ((map_x as f64 + 1.0) - origin_x) * delta_dist_x)
    };

    let (step_y, mut side_dist_y) = if ray_dir_y < 0.0 {
        (-1_isize, (origin_y - map_y as f64) * delta_dist_y)
    } else {
        (1_isize, ((map_y as f64 + 1.0) - origin_y) * delta_dist_y)
    };

    let max_steps = grid
        .width
        .checked_add(grid.height)
        .ok_or(RaycastError::ArithmeticOverflow)?;
    let w_isize = grid.width as isize;
    let h_isize = grid.height as isize;

    let mut side = WallSide::EastWest;
    let mut steps_taken = 0usize;

    for step_idx in 0..max_steps {
        if side_dist_x < side_dist_y {
            side_dist_x += delta_dist_x;
            map_x += step_x;
            side = WallSide::EastWest;
        } else {
            side_dist_y += delta_dist_y;
            map_y += step_y;
            side = WallSide::NorthSouth;
        }
        steps_taken = step_idx + 1;

        // If the ray has moved past the grid boundary and is stepping away, it can never hit.
        if (step_x < 0 && map_x < 0)
            || (step_x > 0 && map_x >= w_isize)
            || (step_y < 0 && map_y < 0)
            || (step_y > 0 && map_y >= h_isize)
        {
            return Ok(WallHit {
                hit: false,
                tile_id: 0,
                map_x: 0,
                map_y: 0,
                perp_wall_dist: f64::INFINITY,
                side,
                wall_u: 0.0,
                steps_taken,
            });
        }

        if map_x >= 0 && map_y >= 0 {
            let ux = map_x as usize;
            let uy = map_y as usize;
            if let Some(tile) = grid.grid_cell_at(ux, uy) {
                if tile != 0 {
                    let raw_dist = match side {
                        WallSide::EastWest => side_dist_x - delta_dist_x,
                        WallSide::NorthSouth => side_dist_y - delta_dist_y,
                    };
                    let perp_wall_dist = if raw_dist < MIN_PERP_DIST {
                        MIN_PERP_DIST
                    } else {
                        raw_dist
                    };

                    let mut wall_x = match side {
                        WallSide::EastWest => origin_y + raw_dist * ray_dir_y,
                        WallSide::NorthSouth => origin_x + raw_dist * ray_dir_x,
                    };
                    wall_x -= wall_x.floor();
                    let flip = (side == WallSide::EastWest && ray_dir_x < 0.0)
                        || (side == WallSide::NorthSouth && ray_dir_y > 0.0);
                    let mut wall_u = if flip { 1.0 - wall_x } else { wall_x };
                    if !(0.0..1.0).contains(&wall_u) {
                        wall_u = 0.0;
                    }

                    return Ok(WallHit {
                        hit: true,
                        tile_id: tile,
                        map_x: ux,
                        map_y: uy,
                        perp_wall_dist,
                        side,
                        wall_u,
                        steps_taken,
                    });
                }
            }
        }
    }

    Ok(WallHit {
        hit: false,
        tile_id: 0,
        map_x: 0,
        map_y: 0,
        perp_wall_dist: f64::INFINITY,
        side,
        wall_u: 0.0,
        steps_taken,
    })
}

// ---------------------------------------------------------------------------
// Frame Renderer (Floor/Ceiling, Walls, Z-Buffered Billboard Sprites)
// ---------------------------------------------------------------------------

fn validate_render_inputs(
    camera: &Camera2D,
    textures: &[TextureRgba],
    sprites: &[Sprite],
    config: &RenderConfig,
    scratch: &SpriteScratch,
    zbuffer: &[f64],
    framebuffer: &PixelContainer,
) -> Result<(usize, usize), RaycastError> {
    let w = framebuffer.width;
    let h = framebuffer.height;
    if w == 0 || h == 0 || w > MAX_FRAME_WIDTH || h > MAX_FRAME_HEIGHT {
        return Err(RaycastError::InvalidFramebuffer);
    }
    let w_usize = w as usize;
    let h_usize = h as usize;
    let expected_bytes = w_usize
        .checked_mul(h_usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or(RaycastError::ArithmeticOverflow)?;
    if framebuffer.data.len() != expected_bytes {
        return Err(RaycastError::InvalidFramebuffer);
    }
    if zbuffer.len() < w_usize {
        return Err(RaycastError::ZBufferTooSmall {
            required: w_usize,
            actual: zbuffer.len(),
        });
    }
    if sprites.len() > MAX_SPRITES {
        return Err(RaycastError::TooManySprites {
            count: sprites.len(),
            max: MAX_SPRITES,
        });
    }
    if scratch.scratch_capacity() < sprites.len() {
        return Err(RaycastError::ScratchTooSmall {
            required: sprites.len(),
            actual: scratch.scratch_capacity(),
        });
    }

    // Re-validate camera finiteness and determinant in case public fields were mutated.
    let _ = Camera2D::camera_new(
        camera.pos_x,
        camera.pos_y,
        camera.dir_x,
        camera.dir_y,
        camera.plane_x,
        camera.plane_y,
    )?;

    if let PlaneSurface::Textured { texture_index } = config.ceiling {
        if texture_index >= textures.len() {
            return Err(RaycastError::InvalidMaterialId {
                id: u16::try_from(texture_index).unwrap_or(u16::MAX),
            });
        }
    }
    if let PlaneSurface::Textured { texture_index } = config.floor {
        if texture_index >= textures.len() {
            return Err(RaycastError::InvalidMaterialId {
                id: u16::try_from(texture_index).unwrap_or(u16::MAX),
            });
        }
    }

    for sp in sprites {
        let _ = check_finite_coord(sp.x)?;
        let _ = check_finite_coord(sp.y)?;
        let _ = check_positive_scale(sp.scale_u)?;
        let _ = check_positive_scale(sp.scale_v)?;
        let _ = check_finite_coord(sp.z_offset)?;
        if sp.texture_index >= textures.len() {
            return Err(RaycastError::InvalidMaterialId {
                id: u16::try_from(sp.texture_index).unwrap_or(u16::MAX),
            });
        }
    }

    Ok((w_usize, h_usize))
}

fn fill_solid_half(pixels: &mut [u8], width: usize, y_start: usize, y_end: usize, color: Rgba8) {
    let (chunks, _) = pixels.as_chunks_mut::<4>();
    let row_start = y_start.saturating_mul(width);
    let row_end = y_end.saturating_mul(width).min(chunks.len());
    if let Some(span) = chunks.get_mut(row_start..row_end) {
        for px in span {
            *px = color;
        }
    }
}

fn cast_textured_plane_half(
    camera: &Camera2D,
    texture: &TextureRgba,
    pixels: &mut [u8],
    width: usize,
    height: usize,
    is_floor: bool,
) {
    let ray_dir_left_x = camera.dir_x - camera.plane_x;
    let ray_dir_left_y = camera.dir_y - camera.plane_y;
    let ray_dir_right_x = camera.dir_x + camera.plane_x;
    let ray_dir_right_y = camera.dir_y + camera.plane_y;
    let ray_span_x = ray_dir_right_x - ray_dir_left_x;
    let ray_span_y = ray_dir_right_y - ray_dir_left_y;

    let half_h = (height as f64) * 0.5;
    let pos_z = half_h;
    let w_f = width as f64;
    let tex_w = texture.width as usize;
    let tex_h = texture.height as usize;
    let tex_w_f = tex_w as f64;
    let tex_h_f = tex_h as f64;

    let (y_start, y_end) = if is_floor {
        (height / 2, height)
    } else {
        (0, height / 2)
    };

    let (chunks, _) = pixels.as_chunks_mut::<4>();
    for y in y_start..y_end {
        let p = if is_floor {
            (y as f64 + 0.5) - half_h
        } else {
            half_h - (y as f64 + 0.5)
        };
        if p.abs() < EPSILON {
            continue;
        }
        let row_distance = pos_z / p;
        let step_x = row_distance * ray_span_x / w_f;
        let step_y = row_distance * ray_span_y / w_f;
        let mut world_x = camera.pos_x + row_distance * ray_dir_left_x + 0.5 * step_x;
        let mut world_y = camera.pos_y + row_distance * ray_dir_left_y + 0.5 * step_y;

        let row_offset = y * width;
        let Some(row_slice) = chunks.get_mut(row_offset..row_offset + width) else {
            continue;
        };
        for dst in row_slice {
            let u = world_x - world_x.floor();
            let v = world_y - world_y.floor();
            let tx = ((u * tex_w_f) as usize).min(tex_w - 1);
            let ty = ((v * tex_h_f) as usize).min(tex_h - 1);
            *dst = texture.texture_sample(tx, ty);
            world_x += step_x;
            world_y += step_y;
        }
    }
}

fn render_planes(
    camera: &Camera2D,
    textures: &[TextureRgba],
    config: &RenderConfig,
    pixels: &mut [u8],
    width: usize,
    height: usize,
) {
    let mid = height / 2;
    match config.ceiling {
        PlaneSurface::Solid(color) => fill_solid_half(pixels, width, 0, mid, color),
        PlaneSurface::Textured { texture_index } => {
            if let Some(tex) = textures.get(texture_index) {
                cast_textured_plane_half(camera, tex, pixels, width, height, false);
            }
        }
    }
    match config.floor {
        PlaneSurface::Solid(color) => fill_solid_half(pixels, width, mid, height, color),
        PlaneSurface::Textured { texture_index } => {
            if let Some(tex) = textures.get(texture_index) {
                cast_textured_plane_half(camera, tex, pixels, width, height, true);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_walls(
    grid: &GridMap,
    camera: &Camera2D,
    textures: &[TextureRgba],
    config: &RenderConfig,
    zbuffer: &mut [f64],
    pixels: &mut [u8],
    width: usize,
    height: usize,
) -> Result<(usize, usize), RaycastError> {
    let w_f = width as f64;
    let h_f = height as f64;
    let h_isize = height as isize;
    let (chunks, _) = pixels.as_chunks_mut::<4>();

    let mut wall_hits = 0usize;
    let mut total_dda_steps = 0usize;

    for (x, z_slot) in zbuffer.iter_mut().take(width).enumerate() {
        let camera_x = 2.0 * (x as f64 + 0.5) / w_f - 1.0;
        let ray_dir_x = camera.dir_x + camera.plane_x * camera_x;
        let ray_dir_y = camera.dir_y + camera.plane_y * camera_x;

        let hit = cast_dda_ray(grid, camera.pos_x, camera.pos_y, ray_dir_x, ray_dir_y)?;
        total_dda_steps = total_dda_steps.saturating_add(hit.steps_taken);

        if !hit.hit {
            *z_slot = f64::INFINITY;
            continue;
        }
        wall_hits += 1;
        *z_slot = hit.perp_wall_dist;

        let tex_idx = usize::from(hit.tile_id.saturating_sub(1));
        let tex = textures
            .get(tex_idx)
            .ok_or(RaycastError::InvalidMaterialId { id: hit.tile_id })?;
        let tex_w = tex.width as usize;
        let tex_h = tex.height as usize;

        let raw_line_height = (h_f / hit.perp_wall_dist).min(h_f * 1024.0).max(1.0);
        let line_height = raw_line_height.round() as isize;
        let draw_start = (-line_height / 2 + h_isize / 2).max(0);
        let draw_end = (line_height / 2 + h_isize / 2).min(h_isize - 1);

        let tx = ((hit.wall_u * tex_w as f64) as usize).min(tex_w - 1);
        let step = (tex_h as f64) / (line_height.max(1) as f64);
        let mut tex_pos = ((draw_start - h_isize / 2 + line_height / 2) as f64) * step;

        for y in draw_start..=draw_end {
            let ty = (tex_pos.floor() as isize).clamp(0, tex_h as isize - 1) as usize;
            tex_pos += step;

            let mut rgba = tex.texture_sample(tx, ty);
            if hit.side == WallSide::NorthSouth {
                rgba = shade_rgba(rgba, config.ns_wall_shade);
            }
            let idx = (y as usize) * width + x;
            if let Some(dst) = chunks.get_mut(idx) {
                *dst = rgba;
            }
        }
    }

    Ok((wall_hits, total_dda_steps))
}

#[allow(clippy::too_many_arguments)]
fn render_billboard_sprites(
    camera: &Camera2D,
    textures: &[TextureRgba],
    sprites: &[Sprite],
    scratch: &mut SpriteScratch,
    zbuffer: &[f64],
    pixels: &mut [u8],
    width: usize,
    height: usize,
) -> Result<usize, RaycastError> {
    scratch.entries.clear();
    for (idx, sp) in sprites.iter().enumerate() {
        let dx = camera.pos_x - sp.x;
        let dy = camera.pos_y - sp.y;
        let dist_sq = dx * dx + dy * dy;
        scratch.entries.push(SpriteSortEntry {
            index: idx,
            dist_sq,
        });
    }

    // `sort_unstable_by` is guaranteed in-place and zero-allocation in Rust `std`.
    scratch.entries.sort_unstable_by(|a, b| {
        b.dist_sq
            .total_cmp(&a.dist_sq)
            .then_with(|| a.index.cmp(&b.index))
    });

    let det = camera.plane_x * camera.dir_y - camera.dir_x * camera.plane_y;
    if det.abs() <= EPSILON {
        return Err(RaycastError::DegenerateCamera);
    }
    let inv_det = 1.0 / det;

    let w_f = width as f64;
    let h_f = height as f64;
    let w_isize = width as isize;
    let h_isize = height as isize;
    let (chunks, _) = pixels.as_chunks_mut::<4>();

    let mut sprites_projected = 0usize;

    for entry in &scratch.entries {
        let Some(sp) = sprites.get(entry.index) else {
            continue;
        };
        let Some(tex) = textures.get(sp.texture_index) else {
            return Err(RaycastError::InvalidMaterialId {
                id: u16::try_from(sp.texture_index).unwrap_or(u16::MAX),
            });
        };

        let rel_x = sp.x - camera.pos_x;
        let rel_y = sp.y - camera.pos_y;
        let transform_x = inv_det * (camera.dir_y * rel_x - camera.dir_x * rel_y);
        let transform_y = inv_det * (-camera.plane_y * rel_x + camera.plane_x * rel_y);

        if transform_y <= MIN_PERP_DIST {
            continue;
        }
        sprites_projected += 1;

        let sprite_screen_x = ((w_f * 0.5) * (1.0 + transform_x / transform_y)).round() as isize;
        let v_move_screen = ((sp.z_offset / transform_y) * h_f)
            .clamp(-h_f * 16.0, h_f * 16.0)
            .round() as isize;

        let sprite_height = ((h_f / transform_y) * sp.scale_v)
            .abs()
            .min(h_f * 1024.0)
            .round() as isize;
        let sprite_width = ((h_f / transform_y) * sp.scale_u)
            .abs()
            .min(w_f * 1024.0)
            .round() as isize;

        if sprite_height <= 0 || sprite_width <= 0 {
            continue;
        }

        let raw_start_y = -sprite_height / 2 + h_isize / 2 + v_move_screen;
        let raw_end_y = sprite_height / 2 + h_isize / 2 + v_move_screen;
        let draw_start_y = raw_start_y.max(0);
        let draw_end_y = raw_end_y.min(h_isize - 1);
        if draw_start_y > draw_end_y {
            continue;
        }

        let raw_start_x = -sprite_width / 2 + sprite_screen_x;
        let raw_end_x = sprite_width / 2 + sprite_screen_x;
        let draw_start_x = raw_start_x.max(0);
        let draw_end_x = raw_end_x.min(w_isize - 1);
        if draw_start_x > draw_end_x {
            continue;
        }

        let tex_w = tex.width as usize;
        let tex_h = tex.height as usize;

        for stripe in draw_start_x..=draw_end_x {
            let sx = stripe as usize;
            let Some(&wall_z) = zbuffer.get(sx) else {
                continue;
            };
            if transform_y >= wall_z {
                continue;
            }

            let tex_x = (((stripe - (-sprite_width / 2 + sprite_screen_x)) * (tex_w as isize))
                / sprite_width)
                .clamp(0, tex_w as isize - 1) as usize;

            for y in draw_start_y..=draw_end_y {
                let d = y - v_move_screen - h_isize / 2 + sprite_height / 2;
                let tex_y =
                    ((d * (tex_h as isize)) / sprite_height).clamp(0, tex_h as isize - 1) as usize;
                let src = tex.texture_sample(tex_x, tex_y);
                if src[3] == 0 {
                    continue;
                }
                let idx = (y as usize) * width + sx;
                if let Some(dst) = chunks.get_mut(idx) {
                    *dst = alpha_blend(*dst, src);
                }
            }
        }
    }

    Ok(sprites_projected)
}

/// Render a complete 2.5D frame (floor/ceiling planes, DDA walls with N/S shading,
/// and depth-sorted Z-buffered billboard sprites) into caller-provided `framebuffer`
/// and `zbuffer` using pre-allocated `scratch`.
///
/// Steady-state execution performs **zero heap allocations**.
#[allow(clippy::too_many_arguments)]
pub fn render_frame_into(
    grid: &GridMap,
    camera: &Camera2D,
    textures: &[TextureRgba],
    sprites: &[Sprite],
    config: &RenderConfig,
    scratch: &mut SpriteScratch,
    zbuffer: &mut [f64],
    framebuffer: &mut PixelContainer,
) -> Result<RenderStats, RaycastError> {
    let (width, height) = validate_render_inputs(
        camera,
        textures,
        sprites,
        config,
        scratch,
        zbuffer,
        framebuffer,
    )?;

    for slot in zbuffer.iter_mut().take(width) {
        *slot = f64::INFINITY;
    }

    render_planes(
        camera,
        textures,
        config,
        &mut framebuffer.data,
        width,
        height,
    );

    let (wall_hits, total_dda_steps) = render_walls(
        grid,
        camera,
        textures,
        config,
        zbuffer,
        &mut framebuffer.data,
        width,
        height,
    )?;

    let sprites_projected = render_billboard_sprites(
        camera,
        textures,
        sprites,
        scratch,
        zbuffer,
        &mut framebuffer.data,
        width,
        height,
    )?;

    Ok(RenderStats {
        columns_cast: width,
        wall_hits,
        total_dda_steps,
        sprites_projected,
    })
}
