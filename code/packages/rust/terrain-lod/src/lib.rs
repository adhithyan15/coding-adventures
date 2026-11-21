//! # terrain-lod
//!
//! GAME42: deterministic, zero-allocation heightfield synthesis, runway
//! placement, and restricted quadtree terrain Level-of-Detail (LOD) mesh
//! emission for Era 4 flight simulation (`aero-flight-model`,
//! `render3d-gpu-bridge`, `game-flight-sim`).
//!
//! ## Architectural Model
//!
//! 1. **Caller-Owned `(2^n + 1)^2` Heightfields (`Heightfield`, `HeightfieldMut`):**
//!    Terrain grids borrow caller-owned `&[f64]` / `&mut [f64]` slices with
//!    `1 <= level <= MAX_LEVELS` (`3 <= side <= 1025`), ensuring zero hidden
//!    allocations and strict `u32` vertex index bounds (`CWE-190`).
//! 2. **Deterministic Synthesis (`rng::Pcg32`):**
//!    Both diamond-square midpoint displacement and multi-octave value-noise
//!    fractional Brownian motion (fBm) execute in a fixed traversal order using
//!    only deterministic IEEE-754 `+`, `-`, `*`, `/`, and `trig` primitives
//!    (zero `std` `libm` `exp`/`powf` calls), guaranteeing bit-identical results
//!    across Windows, Linux, and macOS.
//! 3. **Runway Placement (`place_runway`):**
//!    Flattens a rotated rectangle inside the validated grid domain to target
//!    elevation `h` with linear shoulder blending, atomically refusing any
//!    out-of-bounds or degenerate runway before mutating the heightfield.
//! 4. **Restricted Quadtree LOD & Watertight Stitching:**
//!    Top-down BFS refinement driven by per-node object-space geometric error
//!    divided by AABB-to-camera distance, optional `math3d::Frustum` culling,
//!    and iterative neighbor-level restriction (`|depth_a - depth_b| <= 1`)
//!    with center-fan stitching so adjacent LOD levels share every interior
//!    edge with zero T-junction cracks.
//! 5. **Zero-Allocation Steady State (`VER07`):**
//!    `refine_into` and `emit_mesh_into` reuse a pre-sized `TerrainScratch`
//!    and write triangle indices into a caller-provided `&mut [u32]` buffer,
//!    atomically returning `Err(TerrainError::CapacityExceeded)` rather than
//!    allocating or partially mutating the output buffer.
//!
//! ## Security & Capability Containment (`CAP001` / `CAP002`)
//!
//! * `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]`.
//! * Declares `"capabilities": []` in `required_capabilities.json`.
//! * Guards against `CWE-20` (non-finite inputs, scale-relative checks),
//!   `CWE-125`/`CWE-787` (checked grid and index bounds), `CWE-190`
//!   (`checked_mul` and `u32` vertex index limits), and `CWE-400`/`CWE-835`
//!   (explicit depth, node, restriction-pass, and triangle budgets).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Generated capability verification witnesses.
pub mod capabilities;

pub use capabilities::PureCapabilities;

use math3d::{Aabb3, Frustum, Math3dError, Vec3};
use rng::Pcg32;

/// Minimum supported quadtree subdivision level (`side = 2^1 + 1 = 3`).
pub const MIN_LEVELS: u32 = 1;

/// Maximum supported quadtree subdivision level (`side = 2^10 + 1 = 1025`).
pub const MAX_LEVELS: u32 = 10;

/// Maximum octaves permitted in fBm heightfield synthesis (`CWE-400`).
pub const MAX_OCTAVES: u32 = 12;

/// Maximum active quadtree nodes permitted in [`TerrainScratch`] (`CWE-400`).
pub const MAX_QUADTREE_NODES: usize = 65_536;

/// Maximum triangle count permitted per LOD mesh emission (`CWE-400`).
pub const MAX_OUTPUT_TRIANGLES: usize = 262_144;

/// Bounded iteration cap for restricted quadtree neighbor propagation (`CWE-835`).
pub const MAX_RESTRICT_PASSES: usize = (MAX_LEVELS as usize) + 2;

/// Minimum grid cell spacing in world units (`CWE-20`).
pub const MIN_CELL_SIZE: f64 = 1e-6;

/// Maximum grid cell spacing in world units (`CWE-20`).
pub const MAX_CELL_SIZE: f64 = 1e6;

/// Maximum world-space horizontal coordinate magnitude (`CWE-190` / `CWE-400`).
pub const MAX_COORD: f64 = 1.0e12;

/// Maximum elevation magnitude (`CWE-20` / `CWE-190`).
pub const MAX_HEIGHT: f64 = 1.0e9;

/// Maximum per-node sample grid side when estimating coarse approximation error (`CWE-407`).
const ERROR_SAMPLE_CAP: usize = 64;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Failure modes for heightfield validation, synthesis, runway placement, and LOD refinement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerrainError {
    /// A coordinate, height, camera parameter, or scalar is `NaN` or `+/-Infinity` (`CWE-20`).
    NonFiniteInput,
    /// A numeric parameter violates its valid domain or scale-relative threshold (`CWE-20`).
    InvalidParameter,
    /// Grid buffer length does not match `(2^level + 1)^2` or overflows `u32` bounds (`CWE-190`).
    InvalidGridSize,
    /// A query coordinate, grid index, or runway corner lies outside the terrain grid (`CWE-125`/`CWE-787`).
    OutOfBounds,
    /// Runway geometry is degenerate or violates scale-relative limits (`CWE-20`).
    InvalidRunway,
    /// Caller-provided output slice or scratch capacity is too small (`CWE-400`/`CWE-787`).
    CapacityExceeded,
    /// Quadtree node or triangle budget was exceeded (`CWE-400`/`CWE-835`).
    BudgetExceeded,
    /// Propagated [`Math3dError`] from `math3d` vector/frustum validation.
    MathError(Math3dError),
}

impl From<Math3dError> for TerrainError {
    #[inline]
    fn from(err: Math3dError) -> Self {
        Self::MathError(err)
    }
}

impl std::fmt::Display for TerrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "terrain input contains NaN or Infinity"),
            Self::InvalidParameter => write!(f, "terrain parameter is out of valid domain"),
            Self::InvalidGridSize => write!(f, "heightfield slice does not match (2^level + 1)^2"),
            Self::OutOfBounds => write!(f, "coordinate, index, or runway lies outside grid bounds"),
            Self::InvalidRunway => write!(f, "runway dimensions or orientation are invalid"),
            Self::CapacityExceeded => write!(f, "caller-provided buffer capacity exceeded"),
            Self::BudgetExceeded => write!(f, "quadtree node or triangle budget exceeded"),
            Self::MathError(e) => write!(f, "math3d error: {e}"),
        }
    }
}

impl std::error::Error for TerrainError {}

// ---------------------------------------------------------------------------
// Internal validation & deterministic math helpers
// ---------------------------------------------------------------------------

#[inline]
fn check_finite(v: f64) -> Result<f64, TerrainError> {
    if !v.is_finite() {
        return Err(TerrainError::NonFiniteInput);
    }
    Ok(v)
}

#[inline]
fn check_height(h: f64) -> Result<f64, TerrainError> {
    if !h.is_finite() {
        return Err(TerrainError::NonFiniteInput);
    }
    if h.abs() > MAX_HEIGHT {
        return Err(TerrainError::InvalidParameter);
    }
    Ok(h)
}

#[inline]
fn check_coord(c: f64) -> Result<f64, TerrainError> {
    if !c.is_finite() {
        return Err(TerrainError::NonFiniteInput);
    }
    if c.abs() > MAX_COORD {
        return Err(TerrainError::OutOfBounds);
    }
    Ok(c)
}

#[inline]
fn canonical_bits(v: f64) -> u64 {
    if v == 0.0 {
        0.0_f64.to_bits()
    } else {
        v.to_bits()
    }
}

#[inline]
fn fnv1a_step(hash: u64, bytes: &[u8]) -> u64 {
    let mut h = hash;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

#[inline]
fn next_signed_unit(prng: &mut Pcg32) -> f64 {
    (prng.next_float() * 2.0) - 1.0
}

#[inline]
fn smoothstep(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn floor_i64(v: f64) -> i64 {
    let i = v as i64;
    if (i as f64) > v {
        i - 1
    } else {
        i
    }
}

// ---------------------------------------------------------------------------
// Warrant-aligned boundary validators (CWE-190 / CWE-400 / CWE-787)
// ---------------------------------------------------------------------------

/// Validate a quadtree subdivision level `1 <= level <= MAX_LEVELS`.
#[inline]
pub fn validate_grid_level(level: u32) -> Result<bool, TerrainError> {
    if !(MIN_LEVELS..=MAX_LEVELS).contains(&level) {
        return Err(TerrainError::InvalidParameter);
    }
    Ok(true)
}

/// Validate that `side` is in `[3, 1025]` and `len == side * side` without overflow (`CWE-190`).
#[inline]
pub fn validate_grid_dimensions(side: usize, len: usize) -> Result<bool, TerrainError> {
    let min_side = (1usize << MIN_LEVELS) + 1;
    let max_side = (1usize << MAX_LEVELS) + 1;
    if side < min_side || side > max_side {
        return Err(TerrainError::InvalidGridSize);
    }
    let expected = side
        .checked_mul(side)
        .ok_or(TerrainError::InvalidGridSize)?;
    if expected > u32::MAX as usize || len != expected {
        return Err(TerrainError::InvalidGridSize);
    }
    Ok(true)
}

/// Validate quadtree node and triangle budgets against global caps (`CWE-400` / `CWE-835`).
#[inline]
pub fn validate_lod_budget(
    nodes: usize,
    max_nodes: usize,
    triangles: usize,
    max_triangles: usize,
) -> Result<bool, TerrainError> {
    if nodes == 0
        || max_nodes == 0
        || max_nodes > MAX_QUADTREE_NODES
        || nodes > max_nodes
        || max_triangles > MAX_OUTPUT_TRIANGLES
        || triangles > max_triangles
    {
        return Err(TerrainError::BudgetExceeded);
    }
    Ok(true)
}

/// Validate that caller-provided buffer capacity `available` can hold `needed` items (`CWE-787`).
#[inline]
pub fn validate_index_capacity(needed: usize, available: usize) -> Result<bool, TerrainError> {
    if needed > available {
        return Err(TerrainError::CapacityExceeded);
    }
    Ok(true)
}

// ---------------------------------------------------------------------------
// HeightfieldSpec
// ---------------------------------------------------------------------------

/// Geometric specification of a `(2^level + 1) x (2^level + 1)` terrain heightfield.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeightfieldSpec {
    /// Quadtree subdivision level (`1 <= level <= MAX_LEVELS`).
    pub level: u32,
    /// Horizontal world-space distance between adjacent grid samples (`> 0`).
    pub cell_size: f64,
    /// World-space X coordinate of grid vertex `(0, 0)`.
    pub origin_x: f64,
    /// World-space Z coordinate of grid vertex `(0, 0)`.
    pub origin_z: f64,
}

impl HeightfieldSpec {
    /// Construct and validate a [`HeightfieldSpec`].
    pub fn spec_new(
        level: u32,
        cell_size: f64,
        origin_x: f64,
        origin_z: f64,
    ) -> Result<Self, TerrainError> {
        validate_grid_level(level)?;
        check_finite(cell_size)?;
        if !(MIN_CELL_SIZE..=MAX_CELL_SIZE).contains(&cell_size) {
            return Err(TerrainError::InvalidParameter);
        }
        check_coord(origin_x)?;
        check_coord(origin_z)?;

        let cells = 1usize << level;
        let span = (cells as f64) * cell_size;
        if !span.is_finite() {
            return Err(TerrainError::InvalidParameter);
        }
        check_coord(origin_x + span)?;
        check_coord(origin_z + span)?;

        let spec = Self {
            level,
            cell_size,
            origin_x,
            origin_z,
        };
        let _ = spec.spec_vertex_count()?;
        Ok(spec)
    }

    /// Number of vertices along one axis: `2^level + 1`.
    #[inline]
    pub fn spec_side(&self) -> usize {
        (1usize << self.level) + 1
    }

    /// Number of grid cells along one axis: `2^level`.
    #[inline]
    pub fn spec_cells(&self) -> usize {
        1usize << self.level
    }

    /// Total vertex count `(2^level + 1)^2` checked against `usize` and `u32` limits (`CWE-190`).
    #[inline]
    pub fn spec_vertex_count(&self) -> Result<usize, TerrainError> {
        validate_grid_level(self.level)?;
        let side = self.spec_side();
        let total = side
            .checked_mul(side)
            .ok_or(TerrainError::InvalidGridSize)?;
        if total > u32::MAX as usize {
            return Err(TerrainError::InvalidGridSize);
        }
        Ok(total)
    }

    /// Total horizontal world extent along X and Z: `2^level * cell_size`.
    #[inline]
    pub fn spec_world_span(&self) -> f64 {
        (self.spec_cells() as f64) * self.cell_size
    }
}

// ---------------------------------------------------------------------------
// Heightfield & HeightfieldMut (caller-owned grid views)
// ---------------------------------------------------------------------------

/// Immutable view over a caller-owned `(2^level + 1)^2` heightfield grid.
#[derive(Clone, Copy, Debug)]
pub struct Heightfield<'a> {
    spec: HeightfieldSpec,
    heights: &'a [f64],
}

impl<'a> Heightfield<'a> {
    /// Wrap and validate a caller-owned height buffer.
    ///
    /// Rejects mismatched slice lengths (`CWE-190` / `CWE-125`) and any
    /// non-finite or out-of-range height values (`CWE-20`).
    pub fn from_heights(spec: HeightfieldSpec, heights: &'a [f64]) -> Result<Self, TerrainError> {
        let validated =
            HeightfieldSpec::spec_new(spec.level, spec.cell_size, spec.origin_x, spec.origin_z)?;
        let side = validated.spec_side();
        validate_grid_dimensions(side, heights.len())?;
        for &h in heights {
            check_height(h)?;
        }
        Ok(Self {
            spec: validated,
            heights,
        })
    }

    /// Return the validated [`HeightfieldSpec`].
    #[inline]
    pub fn spec(&self) -> HeightfieldSpec {
        self.spec
    }

    /// Borrow the underlying row-major height slice (`iz * side + ix`).
    #[inline]
    pub fn heights(&self) -> &'a [f64] {
        self.heights
    }

    /// Grid side length `2^level + 1`.
    #[inline]
    pub fn grid_side(&self) -> usize {
        self.spec.spec_side()
    }

    /// Total vertex count `(2^level + 1)^2`.
    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.heights.len()
    }

    /// Read height sample at integer grid coordinates `(ix, iz)` (`CWE-125`).
    #[inline]
    pub fn height_at_grid(&self, ix: usize, iz: usize) -> Result<f64, TerrainError> {
        let side = self.grid_side();
        if ix >= side || iz >= side {
            return Err(TerrainError::OutOfBounds);
        }
        let idx = iz
            .checked_mul(side)
            .and_then(|row| row.checked_add(ix))
            .ok_or(TerrainError::InvalidGridSize)?;
        self.heights
            .get(idx)
            .copied()
            .ok_or(TerrainError::OutOfBounds)
    }

    /// Return world-space 3D position `(x, y, z)` of grid vertex `(ix, iz)`.
    #[inline]
    pub fn vertex_position(&self, ix: usize, iz: usize) -> Result<Vec3, TerrainError> {
        let h = self.height_at_grid(ix, iz)?;
        let x = self.spec.origin_x + (ix as f64) * self.spec.cell_size;
        let z = self.spec.origin_z + (iz as f64) * self.spec.cell_size;
        Ok(Vec3::new(x, h, z))
    }

    /// Compute unit surface normal at grid vertex `(ix, iz)` via central finite differences.
    ///
    /// Boundary vertices automatically use one-sided differences with exact
    /// step denominator (`cell_size` on borders, `2 * cell_size` in the interior).
    pub fn vertex_normal(&self, ix: usize, iz: usize) -> Result<Vec3, TerrainError> {
        let side = self.grid_side();
        if ix >= side || iz >= side {
            return Err(TerrainError::OutOfBounds);
        }
        let ix_lo = ix.saturating_sub(1);
        let ix_hi = if ix + 1 < side { ix + 1 } else { ix };
        let iz_lo = iz.saturating_sub(1);
        let iz_hi = if iz + 1 < side { iz + 1 } else { iz };

        let dx = ((ix_hi - ix_lo) as f64) * self.spec.cell_size;
        let dz = ((iz_hi - iz_lo) as f64) * self.spec.cell_size;
        if dx <= self.spec.cell_size * 1e-9 || dz <= self.spec.cell_size * 1e-9 {
            return Err(TerrainError::InvalidParameter);
        }

        let dh_x = self.height_at_grid(ix_hi, iz)? - self.height_at_grid(ix_lo, iz)?;
        let dh_z = self.height_at_grid(ix, iz_hi)? - self.height_at_grid(ix, iz_lo)?;
        let raw = Vec3::new(-dh_x / dx, 1.0, -dh_z / dz);
        Ok(raw.normalize()?)
    }

    /// Evaluate bilinearly interpolated terrain elevation at world coordinates `(x, z)`.
    ///
    /// Rejects non-finite coordinates (`CWE-20`) and points outside
    /// `[origin_x, origin_x + world_span] x [origin_z, origin_z + world_span]` (`CWE-125`).
    pub fn height_at(&self, x: f64, z: f64) -> Result<f64, TerrainError> {
        check_finite(x)?;
        check_finite(z)?;

        let span = self.spec.spec_world_span();
        let min_x = self.spec.origin_x;
        let max_x = min_x + span;
        let min_z = self.spec.origin_z;
        let max_z = min_z + span;

        if x < min_x || x > max_x || z < min_z || z > max_z {
            return Err(TerrainError::OutOfBounds);
        }

        let cells = self.spec.spec_cells();
        let max_g = cells as f64;
        let gx = ((x - min_x) / self.spec.cell_size).clamp(0.0, max_g);
        let gz = ((z - min_z) / self.spec.cell_size).clamp(0.0, max_g);

        let ix = (gx as usize).min(cells - 1);
        let iz = (gz as usize).min(cells - 1);
        let fx = (gx - (ix as f64)).clamp(0.0, 1.0);
        let fz = (gz - (iz as f64)).clamp(0.0, 1.0);

        let h00 = self.height_at_grid(ix, iz)?;
        let h10 = self.height_at_grid(ix + 1, iz)?;
        let h01 = self.height_at_grid(ix, iz + 1)?;
        let h11 = self.height_at_grid(ix + 1, iz + 1)?;

        let hx0 = h00 + fx * (h10 - h00);
        let hx1 = h01 + fx * (h11 - h01);
        let h = hx0 + fz * (hx1 - hx0);
        check_finite(h)
    }

    /// Compute unit surface normal at world coordinates `(x, z)` via central differences.
    ///
    /// Uses a scale-relative sampling step `eps = 0.5 * cell_size` clamped to
    /// the terrain domain boundaries so boundary queries stay strictly inside.
    pub fn normal_at(&self, x: f64, z: f64) -> Result<Vec3, TerrainError> {
        check_finite(x)?;
        check_finite(z)?;

        let span = self.spec.spec_world_span();
        let min_x = self.spec.origin_x;
        let max_x = min_x + span;
        let min_z = self.spec.origin_z;
        let max_z = min_z + span;

        if x < min_x || x > max_x || z < min_z || z > max_z {
            return Err(TerrainError::OutOfBounds);
        }

        let eps = 0.5 * self.spec.cell_size;
        let x_lo = (x - eps).max(min_x);
        let x_hi = (x + eps).min(max_x);
        let z_lo = (z - eps).max(min_z);
        let z_hi = (z + eps).min(max_z);

        let dx = x_hi - x_lo;
        let dz = z_hi - z_lo;
        if dx <= self.spec.cell_size * 1e-9 || dz <= self.spec.cell_size * 1e-9 {
            return Err(TerrainError::InvalidParameter);
        }

        let dh_x = self.height_at(x_hi, z)? - self.height_at(x_lo, z)?;
        let dh_z = self.height_at(x, z_hi)? - self.height_at(x, z_lo)?;
        let raw = Vec3::new(-dh_x / dx, 1.0, -dh_z / dz);
        Ok(raw.normalize()?)
    }

    /// Compute the exact world-space [`Aabb3`] enclosing the entire terrain heightfield.
    pub fn world_bounds(&self) -> Result<Aabb3, TerrainError> {
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for &h in self.heights {
            check_height(h)?;
            if h < min_y {
                min_y = h;
            }
            if h > max_y {
                max_y = h;
            }
        }
        let span = self.spec.spec_world_span();
        Ok(Aabb3::aabb3_from_min_max(
            Vec3::new(self.spec.origin_x, min_y, self.spec.origin_z),
            Vec3::new(self.spec.origin_x + span, max_y, self.spec.origin_z + span),
        )?)
    }

    /// Emit all `(2^level + 1)^2` vertex positions and central-difference normals
    /// into caller-provided buffers with zero heap allocation.
    ///
    /// Atomically returns `Err(TerrainError::CapacityExceeded)` without modifying
    /// `positions_out` or `normals_out` if either buffer is smaller than `vertex_count()`.
    pub fn emit_vertices_into(
        &self,
        positions_out: &mut [Vec3],
        normals_out: &mut [Vec3],
    ) -> Result<usize, TerrainError> {
        let count = self.vertex_count();
        validate_index_capacity(count, positions_out.len())?;
        validate_index_capacity(count, normals_out.len())?;

        let side = self.grid_side();
        for iz in 0..side {
            for ix in 0..side {
                let idx = iz * side + ix;
                positions_out[idx] = self.vertex_position(ix, iz)?;
                normals_out[idx] = self.vertex_normal(ix, iz)?;
            }
        }
        Ok(count)
    }
}

/// Mutable view over a caller-owned `(2^level + 1)^2` heightfield grid for synthesis and runway placement.
#[derive(Debug)]
pub struct HeightfieldMut<'a> {
    spec: HeightfieldSpec,
    heights: &'a mut [f64],
}

impl<'a> HeightfieldMut<'a> {
    /// Wrap and validate a mutable caller-owned height buffer.
    pub fn from_heights_mut(
        spec: HeightfieldSpec,
        heights: &'a mut [f64],
    ) -> Result<Self, TerrainError> {
        let validated =
            HeightfieldSpec::spec_new(spec.level, spec.cell_size, spec.origin_x, spec.origin_z)?;
        let side = validated.spec_side();
        validate_grid_dimensions(side, heights.len())?;
        for &h in heights.iter() {
            check_height(h)?;
        }
        Ok(Self {
            spec: validated,
            heights,
        })
    }

    /// Borrow an immutable [`Heightfield`] view over the current grid.
    pub fn as_heightfield(&self) -> Result<Heightfield<'_>, TerrainError> {
        Heightfield::from_heights(self.spec, self.heights)
    }

    /// Borrow the underlying mutable row-major height slice.
    #[inline]
    pub fn heights_mut(&mut self) -> &mut [f64] {
        self.heights
    }

    #[inline]
    fn get_idx(&self, ix: usize, iz: usize) -> f64 {
        let side = self.spec.spec_side();
        self.heights[iz * side + ix]
    }

    #[inline]
    fn set_idx(&mut self, ix: usize, iz: usize, val: f64) {
        let side = self.spec.spec_side();
        self.heights[iz * side + ix] = val;
    }

    /// Synthesize deterministic fractal terrain via the diamond-square algorithm.
    ///
    /// Uses [`rng::Pcg32`] seeded with `seed` in a fixed row-major traversal
    /// order so identical inputs yield bit-identical heights across all platforms.
    pub fn synthesize_diamond_square(
        &mut self,
        seed: u64,
        base_height: f64,
        amplitude: f64,
        roughness: f64,
    ) -> Result<(), TerrainError> {
        check_height(base_height)?;
        check_height(amplitude)?;
        check_finite(roughness)?;
        if amplitude < 0.0 || roughness <= 0.0 || roughness > 1.0 {
            return Err(TerrainError::InvalidParameter);
        }
        // Conservative worst-case geometric series bound: base +/- 2 * amplitude.
        check_height(base_height + amplitude)?;
        check_height(base_height - amplitude)?;

        let side = self.spec.spec_side();
        let last = side - 1;
        let mut prng = Pcg32::new(seed);

        for slot in self.heights.iter_mut() {
            *slot = base_height;
        }

        // Seed the four outer corners deterministically.
        self.set_idx(0, 0, base_height + next_signed_unit(&mut prng) * amplitude);
        self.set_idx(
            last,
            0,
            base_height + next_signed_unit(&mut prng) * amplitude,
        );
        self.set_idx(
            0,
            last,
            base_height + next_signed_unit(&mut prng) * amplitude,
        );
        self.set_idx(
            last,
            last,
            base_height + next_signed_unit(&mut prng) * amplitude,
        );

        let mut step = last;
        let mut cur_amp = amplitude * roughness;

        // Bounded by `level <= MAX_LEVELS` (halving `step` from `2^level` down to `2`).
        while step >= 2 {
            let half = step / 2;

            // Diamond step: centers of each `step x step` square.
            let mut z = 0usize;
            while z < last {
                let mut x = 0usize;
                while x < last {
                    let avg = 0.25
                        * (self.get_idx(x, z)
                            + self.get_idx(x + step, z)
                            + self.get_idx(x, z + step)
                            + self.get_idx(x + step, z + step));
                    let h = (avg + next_signed_unit(&mut prng) * cur_amp)
                        .clamp(-MAX_HEIGHT, MAX_HEIGHT);
                    self.set_idx(x + half, z + half, h);
                    x += step;
                }
                z += step;
            }

            // Square step: edge midpoints on alternating parities (clamped at borders).
            let num_rows = last / half;
            for iz in 0..=num_rows {
                let pz = iz * half;
                let mut px = if iz % 2 == 0 { half } else { 0 };
                while px < side {
                    let mut sum = 0.0;
                    let mut count = 0.0;
                    if px >= half {
                        sum += self.get_idx(px - half, pz);
                        count += 1.0;
                    }
                    if px + half < side {
                        sum += self.get_idx(px + half, pz);
                        count += 1.0;
                    }
                    if pz >= half {
                        sum += self.get_idx(px, pz - half);
                        count += 1.0;
                    }
                    if pz + half < side {
                        sum += self.get_idx(px, pz + half);
                        count += 1.0;
                    }
                    let avg = sum / count;
                    let h = (avg + next_signed_unit(&mut prng) * cur_amp)
                        .clamp(-MAX_HEIGHT, MAX_HEIGHT);
                    self.set_idx(px, pz, h);
                    px += step;
                }
            }

            step = half;
            cur_amp *= roughness;
        }

        Ok(())
    }

    /// Synthesize deterministic multi-octave value-noise fractional Brownian motion (fBm).
    ///
    /// Uses a 256-entry gradient/value table populated from [`rng::Pcg32`] and
    /// pure polynomial smoothstep interpolation (zero `std` `libm` calls).
    #[allow(clippy::too_many_arguments)]
    pub fn synthesize_fbm(
        &mut self,
        seed: u64,
        base_height: f64,
        amplitude: f64,
        base_frequency: f64,
        octaves: u32,
        persistence: f64,
        lacunarity: f64,
    ) -> Result<(), TerrainError> {
        check_height(base_height)?;
        check_height(amplitude)?;
        check_finite(base_frequency)?;
        check_finite(persistence)?;
        check_finite(lacunarity)?;

        if amplitude < 0.0
            || base_frequency <= 0.0
            || base_frequency > 1.0e6
            || octaves == 0
            || octaves > MAX_OCTAVES
            || persistence <= 0.0
            || persistence > 1.0
            || !(1.0..=16.0).contains(&lacunarity)
        {
            return Err(TerrainError::InvalidParameter);
        }
        check_height(base_height + amplitude)?;
        check_height(base_height - amplitude)?;

        let mut prng = Pcg32::new(seed);
        let mut lattice = [0.0_f64; 256];
        for entry in lattice.iter_mut() {
            *entry = next_signed_unit(&mut prng);
        }

        let side = self.spec.spec_side();
        let inv_cells = 1.0 / (self.spec.spec_cells() as f64);

        for iz in 0..side {
            let v_norm = (iz as f64) * inv_cells;
            for ix in 0..side {
                let u_norm = (ix as f64) * inv_cells;
                let mut freq = base_frequency;
                let mut amp = 1.0;
                let mut sum = 0.0;
                let mut weight = 0.0;

                for oct in 0..octaves {
                    let sx = u_norm * freq;
                    let sz = v_norm * freq;
                    let xi = floor_i64(sx);
                    let zi = floor_i64(sz);
                    let tx = smoothstep(sx - (xi as f64));
                    let tz = smoothstep(sz - (zi as f64));

                    let n00 = sample_lattice(&lattice, xi, zi, oct);
                    let n10 = sample_lattice(&lattice, xi + 1, zi, oct);
                    let n01 = sample_lattice(&lattice, xi, zi + 1, oct);
                    let n11 = sample_lattice(&lattice, xi + 1, zi + 1, oct);

                    let nx0 = n00 + tx * (n10 - n00);
                    let nx1 = n01 + tx * (n11 - n01);
                    let val = nx0 + tz * (nx1 - nx0);

                    sum += val * amp;
                    weight += amp;
                    amp *= persistence;
                    freq *= lacunarity;
                }

                let h = (base_height + amplitude * (sum / weight)).clamp(-MAX_HEIGHT, MAX_HEIGHT);
                self.set_idx(ix, iz, h);
            }
        }

        Ok(())
    }

    /// Flatten a rotated runway rectangle inside the grid to `runway.elevation`,
    /// blending linearly back to the surrounding terrain across `runway.shoulder_width`.
    ///
    /// Atomically validates all runway parameters and checks that every corner
    /// of the outer runway+shoulder footprint lies inside the heightfield bounds
    /// before modifying any vertex (`CWE-125` / `CWE-787`).
    pub fn place_runway(&mut self, runway: &RunwaySpec) -> Result<(), TerrainError> {
        runway.runway_validate(&self.spec)?;

        let half_len = 0.5 * runway.length;
        let half_wid = 0.5 * runway.width;
        let fwd_x = trig::cos(runway.heading_rad);
        let fwd_z = trig::sin(runway.heading_rad);
        let right_x = -fwd_z;
        let right_z = fwd_x;

        let side = self.spec.spec_side();
        for iz in 0..side {
            let vz = self.spec.origin_z + (iz as f64) * self.spec.cell_size;
            let dz = vz - runway.center_z;
            for ix in 0..side {
                let vx = self.spec.origin_x + (ix as f64) * self.spec.cell_size;
                let dx = vx - runway.center_x;

                let lu = (dx * fwd_x + dz * fwd_z).abs();
                let lv = (dx * right_x + dz * right_z).abs();
                let eu = (lu - half_len).max(0.0);
                let ev = (lv - half_wid).max(0.0);

                let d_out = if eu == 0.0 && ev == 0.0 {
                    0.0
                } else if eu > 0.0 && ev > 0.0 {
                    trig::sqrt(eu * eu + ev * ev)
                } else {
                    eu.max(ev)
                };

                if d_out == 0.0 {
                    self.set_idx(ix, iz, runway.elevation);
                } else if runway.shoulder_width > 0.0 && d_out < runway.shoulder_width {
                    let t = d_out / runway.shoulder_width;
                    let old_h = self.get_idx(ix, iz);
                    let blended = runway.elevation * (1.0 - t) + old_h * t;
                    self.set_idx(ix, iz, blended);
                }
            }
        }

        Ok(())
    }
}

#[inline]
fn sample_lattice(lattice: &[f64; 256], xi: i64, zi: i64, oct: u32) -> f64 {
    let x = xi as u32;
    let z = zi as u32;
    let mixed = x
        .wrapping_mul(0x9e37_79b1)
        .wrapping_add(z.wrapping_mul(0x85eb_ca77))
        .wrapping_add(oct.wrapping_mul(0xc2b2_ae3d));
    let h = mixed ^ (mixed >> 15);
    let idx = ((h.wrapping_mul(0x27d4_eb2d)) >> 24) as usize & 255;
    lattice[idx]
}

// ---------------------------------------------------------------------------
// RunwaySpec
// ---------------------------------------------------------------------------

/// Specification for placing a flattened, shoulder-blended rotated runway rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RunwaySpec {
    /// World-space X coordinate of the runway center.
    pub center_x: f64,
    /// World-space Z coordinate of the runway center.
    pub center_z: f64,
    /// Heading angle in radians in the XZ plane (`0` aligns length along `+X`).
    pub heading_rad: f64,
    /// Full runway length along the heading axis (`> 0`).
    pub length: f64,
    /// Full runway width perpendicular to the heading axis (`> 0`).
    pub width: f64,
    /// Linear transition shoulder width outside the core runway rectangle (`>= 0`).
    pub shoulder_width: f64,
    /// Target runway surface elevation.
    pub elevation: f64,
}

impl RunwaySpec {
    /// Construct a [`RunwaySpec`] with basic finiteness and positivity validation.
    pub fn runway_new(
        center_x: f64,
        center_z: f64,
        heading_rad: f64,
        length: f64,
        width: f64,
        shoulder_width: f64,
        elevation: f64,
    ) -> Result<Self, TerrainError> {
        check_coord(center_x)?;
        check_coord(center_z)?;
        check_finite(heading_rad)?;
        if heading_rad.abs() > math3d::MAX_ANGLE_RAD {
            return Err(TerrainError::InvalidRunway);
        }
        check_finite(length)?;
        check_finite(width)?;
        check_finite(shoulder_width)?;
        check_height(elevation)?;
        if length <= 0.0 || width <= 0.0 || shoulder_width < 0.0 {
            return Err(TerrainError::InvalidRunway);
        }
        Ok(Self {
            center_x,
            center_z,
            heading_rad,
            length,
            width,
            shoulder_width,
            elevation,
        })
    }

    /// Validate that this runway's geometry is non-degenerate relative to `spec.cell_size`
    /// and that all four outer footprint corners lie inside the heightfield domain.
    pub fn runway_validate(&self, spec: &HeightfieldSpec) -> Result<(), TerrainError> {
        let _ = Self::runway_new(
            self.center_x,
            self.center_z,
            self.heading_rad,
            self.length,
            self.width,
            self.shoulder_width,
            self.elevation,
        )?;
        // Scale-relative minimum dimension check against cell_size.
        let min_dim = spec.cell_size * 1e-3;
        if self.length < min_dim || self.width < min_dim {
            return Err(TerrainError::InvalidRunway);
        }

        let span = spec.spec_world_span();
        let min_x = spec.origin_x;
        let max_x = min_x + span;
        let min_z = spec.origin_z;
        let max_z = min_z + span;

        let half_len = 0.5 * self.length + self.shoulder_width;
        let half_wid = 0.5 * self.width + self.shoulder_width;
        let fwd_x = trig::cos(self.heading_rad);
        let fwd_z = trig::sin(self.heading_rad);
        let right_x = -fwd_z;
        let right_z = fwd_x;

        let signs = [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)];
        for (su, sv) in signs {
            let cx = self.center_x + su * half_len * fwd_x + sv * half_wid * right_x;
            let cz = self.center_z + su * half_len * fwd_z + sv * half_wid * right_z;
            if !cx.is_finite()
                || !cz.is_finite()
                || cx < min_x
                || cx > max_x
                || cz < min_z
                || cz > max_z
            {
                return Err(TerrainError::OutOfBounds);
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Quadtree LOD refinement & zero-alloc mesh emission
// ---------------------------------------------------------------------------

/// Configuration parameters for a quadtree LOD refinement pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LodParams {
    /// Camera world position used for distance-based screen-space error.
    pub camera_pos: Vec3,
    /// Maximum allowed projected screen-space error (`> 0`).
    pub max_screen_error: f64,
    /// Perspective projection scale factor (e.g. `viewport_height / (2 * tan(fov_y / 2))`, or `1.0`).
    pub projection_scale: f64,
    /// Maximum quadtree subdivision depth (`1..=MAX_LEVELS`, clamped to `heightfield.level`).
    pub max_depth: u32,
    /// Maximum active quadtree nodes allowed during refinement (`1..=MAX_QUADTREE_NODES`).
    pub max_nodes: usize,
    /// Maximum triangles permitted during mesh emission (`<= MAX_OUTPUT_TRIANGLES`).
    pub max_triangles: usize,
    /// Optional viewing frustum for conservative AABB visibility culling.
    pub frustum: Option<Frustum>,
}

impl LodParams {
    /// Construct and validate [`LodParams`].
    pub fn lod_params_new(
        camera_pos: Vec3,
        max_screen_error: f64,
        projection_scale: f64,
        max_depth: u32,
        max_nodes: usize,
        max_triangles: usize,
        frustum: Option<Frustum>,
    ) -> Result<Self, TerrainError> {
        if !camera_pos.vec3_is_finite() {
            return Err(TerrainError::NonFiniteInput);
        }
        if camera_pos.x.abs() > MAX_COORD
            || camera_pos.y.abs() > MAX_COORD
            || camera_pos.z.abs() > MAX_COORD
        {
            return Err(TerrainError::InvalidParameter);
        }
        check_finite(max_screen_error)?;
        check_finite(projection_scale)?;
        if max_screen_error <= 0.0 || projection_scale <= 0.0 {
            return Err(TerrainError::InvalidParameter);
        }
        if max_depth == 0 || max_depth > MAX_LEVELS {
            return Err(TerrainError::InvalidParameter);
        }
        if max_nodes == 0 || max_nodes > MAX_QUADTREE_NODES {
            return Err(TerrainError::BudgetExceeded);
        }
        if max_triangles > MAX_OUTPUT_TRIANGLES {
            return Err(TerrainError::BudgetExceeded);
        }
        Ok(Self {
            camera_pos,
            max_screen_error,
            projection_scale,
            max_depth,
            max_nodes,
            max_triangles,
            frustum,
        })
    }
}

/// Summary metrics produced by [`refine_into`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LodStats {
    /// Total quadtree nodes allocated inside [`TerrainScratch`] for this pass.
    pub node_count: usize,
    /// Total quadtree leaf nodes (both visible and frustum-culled).
    pub leaf_count: usize,
    /// Number of non-culled leaf nodes that emit triangles.
    pub visible_leaf_count: usize,
    /// Maximum depth reached by any leaf node.
    pub max_leaf_depth: u32,
    /// Exact number of triangles to be emitted by [`emit_mesh_into`].
    pub triangle_count: usize,
    /// Exact number of `u32` vertex indices required (`triangle_count * 3`).
    pub index_count: usize,
}

#[derive(Clone, Copy, Debug)]
struct QuadNode {
    x0: u16,
    z0: u16,
    span: u16,
    depth: u8,
    culled: bool,
    children: [u32; 4],
}

impl QuadNode {
    #[inline]
    const fn empty() -> Self {
        Self {
            x0: 0,
            z0: 0,
            span: 0,
            depth: 0,
            culled: false,
            children: [u32::MAX; 4],
        }
    }

    #[inline]
    const fn is_leaf(&self) -> bool {
        self.children[0] == u32::MAX
    }
}

/// Reusable scratch workspace for zero-allocation quadtree LOD refinement and emission.
#[derive(Debug)]
pub struct TerrainScratch {
    nodes: Vec<QuadNode>,
    node_count: usize,
    grid_level: u32,
    cells: u32,
}

impl TerrainScratch {
    /// Allocate a reusable [`TerrainScratch`] with fixed node capacity `max_nodes`.
    ///
    /// No further heap allocations are performed by [`refine_into`] or [`emit_mesh_into`].
    pub fn scratch_with_capacity(max_nodes: usize) -> Result<Self, TerrainError> {
        if max_nodes == 0 || max_nodes > MAX_QUADTREE_NODES {
            return Err(TerrainError::BudgetExceeded);
        }
        Ok(Self {
            nodes: vec![QuadNode::empty(); max_nodes],
            node_count: 0,
            grid_level: 0,
            cells: 0,
        })
    }

    /// Maximum number of quadtree nodes this scratch buffer can hold.
    #[inline]
    pub fn scratch_capacity(&self) -> usize {
        self.nodes.len()
    }

    /// Number of active quadtree nodes from the last successful refinement.
    #[inline]
    pub fn scratch_node_count(&self) -> usize {
        self.node_count
    }

    /// Number of leaf nodes in the current quadtree.
    pub fn scratch_leaf_count(&self) -> usize {
        self.nodes[..self.node_count]
            .iter()
            .filter(|n| n.is_leaf())
            .count()
    }

    /// Number of non-culled leaf nodes in the current quadtree.
    pub fn scratch_visible_leaf_count(&self) -> usize {
        self.nodes[..self.node_count]
            .iter()
            .filter(|n| n.is_leaf() && !n.culled)
            .count()
    }

    /// Maximum depth among all leaf nodes in the current quadtree.
    pub fn scratch_max_leaf_depth(&self) -> u32 {
        self.nodes[..self.node_count]
            .iter()
            .filter(|n| n.is_leaf())
            .map(|n| u32::from(n.depth))
            .max()
            .unwrap_or(0)
    }

    /// Verify the restricted quadtree invariant: every pair of cardinal-adjacent
    /// leaf nodes differs in depth by at most `1`.
    pub fn scratch_is_restricted(&self) -> bool {
        if self.node_count == 0 || self.cells == 0 {
            return true;
        }
        let nodes = &self.nodes[..self.node_count];
        let cells = self.cells;
        for node in nodes {
            if !node.is_leaf() {
                continue;
            }
            let x0 = u32::from(node.x0);
            let z0 = u32::from(node.z0);
            let span = u32::from(node.span);
            let depth = i32::from(node.depth);

            // Check North neighbor edge (two sample points cover any depth+1 or coarser neighbor).
            if z0 > 0 {
                for px in [x0, x0 + span - 1] {
                    let n = find_leaf_index(nodes, px, z0 - 1);
                    if (depth - i32::from(nodes[n].depth)).abs() > 1 {
                        return false;
                    }
                }
            }
            if z0 + span < cells {
                for px in [x0, x0 + span - 1] {
                    let n = find_leaf_index(nodes, px, z0 + span);
                    if (depth - i32::from(nodes[n].depth)).abs() > 1 {
                        return false;
                    }
                }
            }
            if x0 > 0 {
                for pz in [z0, z0 + span - 1] {
                    let n = find_leaf_index(nodes, x0 - 1, pz);
                    if (depth - i32::from(nodes[n].depth)).abs() > 1 {
                        return false;
                    }
                }
            }
            if x0 + span < cells {
                for pz in [z0, z0 + span - 1] {
                    let n = find_leaf_index(nodes, x0 + span, pz);
                    if (depth - i32::from(nodes[n].depth)).abs() > 1 {
                        return false;
                    }
                }
            }
        }
        true
    }
}

/// Iterative quadtree leaf lookup from root `0` (`<= MAX_LEVELS` steps, `CWE-674` acyclic).
#[inline]
fn find_leaf_index(nodes: &[QuadNode], px: u32, pz: u32) -> usize {
    let mut idx = 0usize;
    for _ in 0..=MAX_LEVELS {
        let node = nodes[idx];
        if node.is_leaf() {
            return idx;
        }
        let half = u32::from(node.span) >> 1;
        let cx = usize::from(px >= u32::from(node.x0) + half);
        let cz = usize::from(pz >= u32::from(node.z0) + half);
        idx = node.children[(cz << 1) | cx] as usize;
    }
    idx
}

/// Compute object-space geometric error and vertical bounds `(min_y, max_y, object_error)`
/// for quadtree node `(x0, z0, span)` against its coarse 2-triangle diagonal split.
fn evaluate_node_metrics(
    hf: &Heightfield<'_>,
    x0: usize,
    z0: usize,
    span: usize,
) -> Result<(f64, f64, f64), TerrainError> {
    let side = hf.grid_side();
    let heights = hf.heights();

    let h00 = heights[z0 * side + x0];
    let h10 = heights[z0 * side + (x0 + span)];
    let h01 = heights[(z0 + span) * side + x0];
    let h11 = heights[(z0 + span) * side + (x0 + span)];

    let mut min_y = h00.min(h10).min(h01).min(h11);
    let mut max_y = h00.max(h10).max(h01).max(h11);
    let mut max_err = 0.0_f64;

    if span >= 2 {
        let stride = if span <= ERROR_SAMPLE_CAP {
            1
        } else {
            span / ERROR_SAMPLE_CAP
        };
        let inv_span = 1.0 / (span as f64);

        let mut dz = 0usize;
        while dz <= span {
            let v = (dz as f64) * inv_span;
            let row = (z0 + dz) * side;
            let mut dx = 0usize;
            while dx <= span {
                let h = heights[row + x0 + dx];
                if h < min_y {
                    min_y = h;
                }
                if h > max_y {
                    max_y = h;
                }
                let u = (dx as f64) * inv_span;
                // Coarse split along diagonal (0,0)-(1,1): upper/right triangle vs lower/left.
                let h_coarse = if u >= v {
                    h00 + u * (h10 - h00) + v * (h11 - h10)
                } else {
                    h00 + v * (h01 - h00) + u * (h11 - h01)
                };
                let err = (h - h_coarse).abs();
                if err > max_err {
                    max_err = err;
                }
                dx += stride;
            }
            dz += stride;
        }
    }

    Ok((min_y, max_y, max_err))
}

/// Build the world-space [`Aabb3`] of quadtree node `(x0, z0, span)` given `(min_y, max_y)`.
#[inline]
fn node_aabb(
    spec: &HeightfieldSpec,
    x0: usize,
    z0: usize,
    span: usize,
    min_y: f64,
    max_y: f64,
) -> Result<Aabb3, TerrainError> {
    let min_x = spec.origin_x + (x0 as f64) * spec.cell_size;
    let max_x = spec.origin_x + ((x0 + span) as f64) * spec.cell_size;
    let min_z = spec.origin_z + (z0 as f64) * spec.cell_size;
    let max_z = spec.origin_z + ((z0 + span) as f64) * spec.cell_size;
    Ok(Aabb3::aabb3_from_min_max(
        Vec3::new(min_x, min_y, min_z),
        Vec3::new(max_x, max_y, max_z),
    )?)
}

/// Euclidean distance from `point` to the closest point on `aabb`, clamped below by `min_dist`.
#[inline]
fn distance_to_aabb(point: Vec3, aabb: &Aabb3, min_dist: f64) -> f64 {
    let dx = if point.x < aabb.min.x {
        aabb.min.x - point.x
    } else if point.x > aabb.max.x {
        point.x - aabb.max.x
    } else {
        0.0
    };
    let dy = if point.y < aabb.min.y {
        aabb.min.y - point.y
    } else if point.y > aabb.max.y {
        point.y - aabb.max.y
    } else {
        0.0
    };
    let dz = if point.z < aabb.min.z {
        aabb.min.z - point.z
    } else if point.z > aabb.max.z {
        point.z - aabb.max.z
    } else {
        0.0
    };
    let d_sq = dx * dx + dy * dy + dz * dz;
    if d_sq <= min_dist * min_dist {
        min_dist
    } else {
        trig::sqrt(d_sq)
    }
}

/// Subdivide `scratch.nodes[parent_idx]` into 4 child nodes if capacity allows.
fn subdivide_node(
    hf: &Heightfield<'_>,
    frustum: Option<&Frustum>,
    scratch: &mut TerrainScratch,
    parent_idx: usize,
    max_nodes: usize,
) -> Result<(), TerrainError> {
    if scratch.node_count + 4 > max_nodes {
        return Err(TerrainError::BudgetExceeded);
    }
    let parent = scratch.nodes[parent_idx];
    let half = parent.span >> 1;
    let child_depth = parent.depth + 1;
    let base = scratch.node_count;

    let offsets = [(0u16, 0u16), (half, 0), (0, half), (half, half)];
    for (i, (ox, oz)) in offsets.iter().enumerate() {
        let cx0 = parent.x0 + *ox;
        let cz0 = parent.z0 + *oz;
        let culled = if parent.culled {
            true
        } else if let Some(fr) = frustum {
            let (min_y, max_y, _) =
                evaluate_node_metrics(hf, usize::from(cx0), usize::from(cz0), usize::from(half))?;
            let aabb = node_aabb(
                &hf.spec(),
                usize::from(cx0),
                usize::from(cz0),
                usize::from(half),
                min_y,
                max_y,
            )?;
            !fr.intersects_aabb(&aabb)?
        } else {
            false
        };

        scratch.nodes[base + i] = QuadNode {
            x0: cx0,
            z0: cz0,
            span: half,
            depth: child_depth,
            culled,
            children: [u32::MAX; 4],
        };
    }

    scratch.nodes[parent_idx].children = [
        base as u32,
        (base + 1) as u32,
        (base + 2) as u32,
        (base + 3) as u32,
    ];
    scratch.node_count += 4;
    Ok(())
}

#[inline]
fn count_leaf_triangles(nodes: &[QuadNode], node: QuadNode, cells: u32) -> usize {
    if node.culled {
        return 0;
    }
    if node.span <= 1 {
        return 2;
    }
    let x0 = u32::from(node.x0);
    let z0 = u32::from(node.z0);
    let span = u32::from(node.span);
    let north_finer = z0 > 0 && nodes[find_leaf_index(nodes, x0, z0 - 1)].depth > node.depth;
    let south_finer =
        (z0 + span < cells) && nodes[find_leaf_index(nodes, x0, z0 + span)].depth > node.depth;
    let west_finer = x0 > 0 && nodes[find_leaf_index(nodes, x0 - 1, z0)].depth > node.depth;
    let east_finer =
        (x0 + span < cells) && nodes[find_leaf_index(nodes, x0 + span, z0)].depth > node.depth;

    let finer_sides = usize::from(north_finer)
        + usize::from(south_finer)
        + usize::from(west_finer)
        + usize::from(east_finer);
    if finer_sides == 0 {
        2
    } else {
        4 + finer_sides
    }
}

/// Refine a restricted quadtree over `heightfield` from `params` into `scratch`
/// with zero heap allocation.
///
/// Validates `params` and `scratch.scratch_capacity()` upfront before mutating
/// `scratch`. Returns [`LodStats`] describing the resulting restricted tree and
/// exact triangle/index counts required by [`emit_mesh_into`].
pub fn refine_into(
    heightfield: &Heightfield<'_>,
    params: &LodParams,
    scratch: &mut TerrainScratch,
) -> Result<LodStats, TerrainError> {
    let validated_params = LodParams::lod_params_new(
        params.camera_pos,
        params.max_screen_error,
        params.projection_scale,
        params.max_depth,
        params.max_nodes,
        params.max_triangles,
        params.frustum,
    )?;

    if validated_params.max_nodes > scratch.scratch_capacity() {
        return Err(TerrainError::CapacityExceeded);
    }

    let spec = heightfield.spec();
    let cells = spec.spec_cells();
    let effective_max_depth = validated_params.max_depth.min(spec.level);
    let effective_max_nodes = validated_params.max_nodes;
    let min_dist = spec.cell_size * 1e-4;

    scratch.grid_level = spec.level;
    scratch.cells = cells as u32;
    scratch.nodes[0] = QuadNode {
        x0: 0,
        z0: 0,
        span: cells as u16,
        depth: 0,
        culled: false,
        children: [u32::MAX; 4],
    };
    scratch.node_count = 1;

    // Pass 1: Top-down BFS refinement driven by screen-space error and optional frustum.
    let mut cursor = 0usize;
    while cursor < scratch.node_count {
        let node = scratch.nodes[cursor];
        let x0 = usize::from(node.x0);
        let z0 = usize::from(node.z0);
        let span = usize::from(node.span);

        let (min_y, max_y, obj_err) = evaluate_node_metrics(heightfield, x0, z0, span)?;
        let aabb = node_aabb(&spec, x0, z0, span, min_y, max_y)?;

        if let Some(ref fr) = validated_params.frustum {
            if !fr.intersects_aabb(&aabb)? {
                scratch.nodes[cursor].culled = true;
                cursor += 1;
                continue;
            }
        }

        if u32::from(node.depth) < effective_max_depth && span >= 2 {
            let dist = distance_to_aabb(validated_params.camera_pos, &aabb, min_dist);
            let sse = (obj_err * validated_params.projection_scale) / dist;
            if !sse.is_finite() {
                return Err(TerrainError::NonFiniteInput);
            }
            if sse > validated_params.max_screen_error {
                subdivide_node(
                    heightfield,
                    validated_params.frustum.as_ref(),
                    scratch,
                    cursor,
                    effective_max_nodes,
                )?;
            }
        }
        cursor += 1;
    }

    // Pass 2: Iterative restricted quadtree balancing (|depth_a - depth_b| <= 1).
    let cells_u32 = cells as u32;
    for _pass in 0..MAX_RESTRICT_PASSES {
        let mut changed = false;
        let snapshot = scratch.node_count;
        for idx in 0..snapshot {
            let node = scratch.nodes[idx];
            if !node.is_leaf() || node.depth < 2 {
                continue;
            }
            let x0 = u32::from(node.x0);
            let z0 = u32::from(node.z0);
            let span = u32::from(node.span);

            let neighbor_queries = [
                (z0 > 0, x0, z0.saturating_sub(1)),
                (z0 + span < cells_u32, x0, z0 + span),
                (x0 > 0, x0.saturating_sub(1), z0),
                (x0 + span < cells_u32, x0 + span, z0),
            ];

            for (in_bounds, qx, qz) in neighbor_queries {
                if !in_bounds {
                    continue;
                }
                for _step in 0..MAX_LEVELS {
                    let n_idx = find_leaf_index(&scratch.nodes[..scratch.node_count], qx, qz);
                    if scratch.nodes[n_idx].depth + 1 >= node.depth {
                        break;
                    }
                    subdivide_node(
                        heightfield,
                        validated_params.frustum.as_ref(),
                        scratch,
                        n_idx,
                        effective_max_nodes,
                    )?;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    // Pass 3: Compute exact leaf/triangle counts and enforce triangle budget.
    let active_nodes = &scratch.nodes[..scratch.node_count];
    let mut leaf_count = 0usize;
    let mut visible_leaf_count = 0usize;
    let mut max_leaf_depth = 0u32;
    let mut triangle_count = 0usize;

    for &node in active_nodes {
        if node.is_leaf() {
            leaf_count += 1;
            let d = u32::from(node.depth);
            if d > max_leaf_depth {
                max_leaf_depth = d;
            }
            if !node.culled {
                visible_leaf_count += 1;
                triangle_count = triangle_count
                    .checked_add(count_leaf_triangles(active_nodes, node, cells_u32))
                    .ok_or(TerrainError::BudgetExceeded)?;
            }
        }
    }

    validate_lod_budget(
        scratch.node_count,
        effective_max_nodes,
        triangle_count,
        validated_params.max_triangles,
    )?;

    let index_count = triangle_count
        .checked_mul(3)
        .ok_or(TerrainError::BudgetExceeded)?;

    Ok(LodStats {
        node_count: scratch.node_count,
        leaf_count,
        visible_leaf_count,
        max_leaf_depth,
        triangle_count,
        index_count,
    })
}

#[inline]
fn vertex_id(side: u32, ix: u32, iz: u32) -> u32 {
    iz * side + ix
}

/// Emit watertight, CCW-wound triangle indices from a refined [`TerrainScratch`]
/// into caller-provided `indices_out` with zero heap allocation.
///
/// Atomically checks `max_triangles` and `indices_out.len()` before writing
/// any element, so `Err(TerrainError::CapacityExceeded)` or
/// `Err(TerrainError::BudgetExceeded)` leaves `indices_out` completely untouched.
pub fn emit_mesh_into(
    heightfield: &Heightfield<'_>,
    scratch: &TerrainScratch,
    indices_out: &mut [u32],
    max_triangles: usize,
) -> Result<usize, TerrainError> {
    let spec = heightfield.spec();
    if scratch.node_count == 0 || scratch.grid_level != spec.level {
        return Err(TerrainError::InvalidParameter);
    }
    if max_triangles > MAX_OUTPUT_TRIANGLES {
        return Err(TerrainError::BudgetExceeded);
    }

    let nodes = &scratch.nodes[..scratch.node_count];
    let cells = spec.spec_cells() as u32;
    let side = spec.spec_side() as u32;

    // Pass 1: Count exact triangles before writing anything to `indices_out`.
    let mut total_triangles = 0usize;
    for &node in nodes {
        if node.is_leaf() && !node.culled {
            total_triangles = total_triangles
                .checked_add(count_leaf_triangles(nodes, node, cells))
                .ok_or(TerrainError::BudgetExceeded)?;
        }
    }
    if total_triangles > max_triangles {
        return Err(TerrainError::BudgetExceeded);
    }
    let needed_indices = total_triangles
        .checked_mul(3)
        .ok_or(TerrainError::CapacityExceeded)?;
    validate_index_capacity(needed_indices, indices_out.len())?;

    // Pass 2: Emit crack-free CCW triangles.
    let mut out_pos = 0usize;
    for &node in nodes {
        if !node.is_leaf() || node.culled {
            continue;
        }
        let x0 = u32::from(node.x0);
        let z0 = u32::from(node.z0);
        let span = u32::from(node.span);

        let nw = vertex_id(side, x0, z0);
        let ne = vertex_id(side, x0 + span, z0);
        let se = vertex_id(side, x0 + span, z0 + span);
        let sw = vertex_id(side, x0, z0 + span);

        if span <= 1 {
            indices_out[out_pos] = nw;
            indices_out[out_pos + 1] = sw;
            indices_out[out_pos + 2] = se;
            indices_out[out_pos + 3] = nw;
            indices_out[out_pos + 4] = se;
            indices_out[out_pos + 5] = ne;
            out_pos += 6;
            continue;
        }

        let half = span >> 1;
        let north_finer = z0 > 0 && nodes[find_leaf_index(nodes, x0, z0 - 1)].depth > node.depth;
        let south_finer =
            (z0 + span < cells) && nodes[find_leaf_index(nodes, x0, z0 + span)].depth > node.depth;
        let west_finer = x0 > 0 && nodes[find_leaf_index(nodes, x0 - 1, z0)].depth > node.depth;
        let east_finer =
            (x0 + span < cells) && nodes[find_leaf_index(nodes, x0 + span, z0)].depth > node.depth;

        if !north_finer && !south_finer && !west_finer && !east_finer {
            indices_out[out_pos] = nw;
            indices_out[out_pos + 1] = sw;
            indices_out[out_pos + 2] = se;
            indices_out[out_pos + 3] = nw;
            indices_out[out_pos + 4] = se;
            indices_out[out_pos + 5] = ne;
            out_pos += 6;
            continue;
        }

        let center = vertex_id(side, x0 + half, z0 + half);

        // North edge: NW -> NE
        if north_finer {
            let mid_n = vertex_id(side, x0 + half, z0);
            indices_out[out_pos] = nw;
            indices_out[out_pos + 1] = center;
            indices_out[out_pos + 2] = mid_n;
            indices_out[out_pos + 3] = mid_n;
            indices_out[out_pos + 4] = center;
            indices_out[out_pos + 5] = ne;
            out_pos += 6;
        } else {
            indices_out[out_pos] = nw;
            indices_out[out_pos + 1] = center;
            indices_out[out_pos + 2] = ne;
            out_pos += 3;
        }

        // East edge: NE -> SE
        if east_finer {
            let mid_e = vertex_id(side, x0 + span, z0 + half);
            indices_out[out_pos] = ne;
            indices_out[out_pos + 1] = center;
            indices_out[out_pos + 2] = mid_e;
            indices_out[out_pos + 3] = mid_e;
            indices_out[out_pos + 4] = center;
            indices_out[out_pos + 5] = se;
            out_pos += 6;
        } else {
            indices_out[out_pos] = ne;
            indices_out[out_pos + 1] = center;
            indices_out[out_pos + 2] = se;
            out_pos += 3;
        }

        // South edge: SE -> SW
        if south_finer {
            let mid_s = vertex_id(side, x0 + half, z0 + span);
            indices_out[out_pos] = se;
            indices_out[out_pos + 1] = center;
            indices_out[out_pos + 2] = mid_s;
            indices_out[out_pos + 3] = mid_s;
            indices_out[out_pos + 4] = center;
            indices_out[out_pos + 5] = sw;
            out_pos += 6;
        } else {
            indices_out[out_pos] = se;
            indices_out[out_pos + 1] = center;
            indices_out[out_pos + 2] = sw;
            out_pos += 3;
        }

        // West edge: SW -> NW
        if west_finer {
            let mid_w = vertex_id(side, x0, z0 + half);
            indices_out[out_pos] = sw;
            indices_out[out_pos + 1] = center;
            indices_out[out_pos + 2] = mid_w;
            indices_out[out_pos + 3] = mid_w;
            indices_out[out_pos + 4] = center;
            indices_out[out_pos + 5] = nw;
            out_pos += 6;
        } else {
            indices_out[out_pos] = sw;
            indices_out[out_pos + 1] = center;
            indices_out[out_pos + 2] = nw;
            out_pos += 3;
        }
    }

    Ok(out_pos)
}

// ---------------------------------------------------------------------------
// Deterministic FNV-1a Mesh Digests
// ---------------------------------------------------------------------------

/// Compute a 64-bit FNV-1a digest over `heightfield` geometry and emitted `indices`.
///
/// Validates that `indices.len()` is a multiple of 3 and every vertex index
/// lies within `0..heightfield.vertex_count()` (`CWE-125`). Canonicalizes `-0.0`
/// to `+0.0` so signed zero does not perturb cross-platform digests.
pub fn mesh_digest_fnv1a(
    heightfield: &Heightfield<'_>,
    indices: &[u32],
) -> Result<u64, TerrainError> {
    if !indices.len().is_multiple_of(3) {
        return Err(TerrainError::InvalidParameter);
    }
    let max_v = heightfield.vertex_count() as u32;
    for &idx in indices {
        if idx >= max_v {
            return Err(TerrainError::OutOfBounds);
        }
    }

    let spec = heightfield.spec();
    let heights = heightfield.heights();
    let mut h = 0xcbf2_9ce4_8422_2325_u64;

    h = fnv1a_step(h, &spec.level.to_le_bytes());
    h = fnv1a_step(h, &canonical_bits(spec.cell_size).to_le_bytes());
    h = fnv1a_step(h, &canonical_bits(spec.origin_x).to_le_bytes());
    h = fnv1a_step(h, &canonical_bits(spec.origin_z).to_le_bytes());
    h = fnv1a_step(h, &(indices.len() as u64).to_le_bytes());

    for &idx in indices {
        h = fnv1a_step(h, &idx.to_le_bytes());
        let y = heights[idx as usize];
        h = fnv1a_step(h, &canonical_bits(y).to_le_bytes());
    }

    Ok(h)
}

/// Compute a 64-bit FNV-1a digest over a raw triangle index slice.
pub fn indices_digest_fnv1a(indices: &[u32]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    h = fnv1a_step(h, &(indices.len() as u64).to_le_bytes());
    for &idx in indices {
        h = fnv1a_step(h, &idx.to_le_bytes());
    }
    h
}
