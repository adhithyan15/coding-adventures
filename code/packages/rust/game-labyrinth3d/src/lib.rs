//! # game-labyrinth3d
//!
//! `GAME34`: deterministic, zero-allocation (steady-state), pure Tier 1
//! 2.5D/3D first-person dungeon crawler simulation core with dual software
//! renderers outputting directly into caller-owned [`PixelContainer`] (`IMG00`)
//! and Z-buffers.
//!
//! ## Architecture (`GAME00` Principle 1 & 2)
//!
//! ```text
//!                      ┌──────────────────────┐
//!                      │   LabyrinthInput     │ (forward, strafe, turn, interact, dt_ns)
//!                      └──────────┬───────────┘
//!                                 ▼
//!  LabyrinthState ──► step_in_place(&mut state, input) ──► StepOutcome (state_digest)
//!   │  (grid, player, doors, pickups, entities, GameClockState, LabyrinthRng)
//!   │
//!   ├──► RenderMode::Raycast25D  ──► raycast-dda (DDA walls, floor/ceil, sprites)
//!   └──► RenderMode::Software3D  ──► raster3d-soft (MVP + Z-buffer + 3D meshes)
//!                                 │
//!                                 ▼
//!                PixelContainer (RGBA8) + frame_digest_fnv1a
//! ```
//!
//! ## Safety, Determinism, and Zero-Allocation Guarantees
//!
//! * **Pure Tier 1 (`"capabilities": []`)**: `#![forbid(unsafe_code)]`,
//!   `#![deny(missing_docs)]`, zero filesystem/network/clock/env/process
//!   authority, and zero `std` `libm` calls (all trig/sqrt uses `trig` `PHY00`).
//! * **Explicit RNG & Fixed-Timestep Clock**: All randomness flows through
//!   [`LabyrinthRng`] backed by `rng::Pcg32` and all timing flows through
//!   `game_clock::advance_frame`.
//! * **Atomic Rollback**: Both [`step_in_place`] and [`render_frame_into`]
//!   validate all inputs, dimensions, buffers, and scratch capacities upfront
//!   before mutating any caller-owned state or pixel/Z-buffer memory.
//! * **Zero Steady-State Heap Allocation**: [`step_in_place`] and
//!   [`render_frame_into`] (in both `Raycast25D` and `Software3D` modes) perform
//!   zero heap allocations once [`LabyrinthRenderScratch`] is prepared.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Generated capability verification witnesses.
pub mod capabilities;

pub use capabilities::PureCapabilities;
pub use game_clock::{ClockError, GameClockConfig, GameClockState};
pub use math3d::{Mat3, Mat4, Vec3, Vec4};
pub use pixel_container::PixelContainer;
pub use raster3d_soft::{
    ClipSpace, CullMode, DepthBufferMut, DepthFunc, FrontFace, LightRig, RasterConfig, RasterError,
    RasterScratch, ShadingMode, Texture as RasterTexture, TextureWrap, Vertex as RasterVertex,
};
pub use raycast_dda::{
    Camera2D, GridMap, PlaneSurface, RaycastError, RenderConfig as RaycastRenderConfig, Sprite,
    SpriteScratch, TextureRgba,
};

// ---------------------------------------------------------------------------
// Type Aliases & Domain Constants
// ---------------------------------------------------------------------------

/// Packed 8-bit RGBA color `[r, g, b, a]`.
pub type Rgba8 = [u8; 4];

/// Linear normalized RGB color `[r, g, b]` in `[0.0, 1.0]`.
pub type RgbColor = [f64; 3];

/// Maximum tile grid width or height (`CWE-400`).
pub const MAX_GRID_DIM: usize = 64;

/// Maximum number of doors in a level (`CWE-400`).
pub const MAX_DOORS: usize = 64;

/// Maximum number of pickups in a level (`CWE-400`).
pub const MAX_PICKUPS: usize = 128;

/// Maximum number of patrolling/chasing entities in a level (`CWE-400`).
pub const MAX_ENTITIES: usize = 64;

/// Number of distinct key IDs (`0..MAX_KEYS`) tracked in the 8-bit key mask.
pub const MAX_KEYS: u8 = 8;

/// Maximum number of textures stored in [`LabyrinthAssets`] (`CWE-400`).
pub const MAX_TEXTURES: usize = 16;

/// Maximum supported framebuffer width in pixels (`CWE-400`).
pub const MAX_FRAME_WIDTH: u32 = 4096;

/// Maximum supported framebuffer height in pixels (`CWE-400`).
pub const MAX_FRAME_HEIGHT: u32 = 4096;

/// Maximum finite world coordinate or input magnitude (`CWE-20` / `CWE-400`).
pub const MAX_COORD: f64 = 1.0e6;

/// Minimum player collision radius in grid units.
pub const MIN_PLAYER_RADIUS: f64 = 0.05;

/// Maximum player collision radius in grid units (`< 0.5` to traverse 1-unit corridors).
pub const MAX_PLAYER_RADIUS: f64 = 0.45;

/// Minimum horizontal field of view in radians.
pub const MIN_FOV_RAD: f64 = 0.2;

/// Maximum horizontal field of view in radians (`< pi`).
pub const MAX_FOV_RAD: f64 = 2.8;

/// Default horizontal field of view (~66 degrees, Wolf3D proportion).
pub const DEFAULT_FOV_RAD: f64 = 1.151_917_306_316_257_5;

/// Maximum normalized movement/turn control magnitude per input frame.
pub const MAX_CONTROL_MAGNITUDE: f64 = 10.0;

/// Maximum per-call delta time in nanoseconds (250 ms, matching `game-clock`).
pub const MAX_DT_NS: u64 = game_clock::DEFAULT_MAX_FRAME_DELTA_NS;

/// Maximum vertex buffer capacity pre-allocated for `RenderMode::Software3D`.
pub const MAX_3D_VERTICES: usize = 32_768;

/// Maximum index buffer capacity pre-allocated for `RenderMode::Software3D`.
pub const MAX_3D_INDICES: usize = 49_152;

/// Local tile radius around the player meshed in `RenderMode::Software3D`.
pub const MAX_3D_RENDER_RADIUS: usize = 20;

const EPSILON: f64 = 1.0e-12;
const FNV1A_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV1A_PRIME: u64 = 0x0100_0000_01b3;

// ---------------------------------------------------------------------------
// Error Type
// ---------------------------------------------------------------------------

/// Failure modes returned by `game-labyrinth3d` operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabyrinthError {
    /// A floating-point input or state field is `NaN` or `+/-Infinity` (`CWE-20`).
    NonFiniteInput,
    /// A numeric value, speed, angle, or key ID is outside safe bounds (`CWE-20` / `CWE-400`).
    ValueOutOfBounds,
    /// Grid dimensions, tile buffer length, or start/exit layout is invalid (`CWE-400`).
    InvalidGrid,
    /// Player position, radius, speed, or health invariants are violated (`CWE-20`).
    InvalidPlayer,
    /// Door count, coordinates, texture ID, or key ID is invalid (`CWE-20` / `CWE-400`).
    InvalidDoor,
    /// Pickup count, coordinates, key ID, or texture index is invalid (`CWE-20` / `CWE-400`).
    InvalidPickup,
    /// Entity count, coordinates, radius, speed, or texture index is invalid (`CWE-20` / `CWE-400`).
    InvalidEntity,
    /// Framebuffer dimensions or byte buffer length is invalid (`CWE-125` / `CWE-400`).
    InvalidFramebuffer,
    /// Caller Z-buffer slice is shorter than `width * height` (`CWE-125`).
    ZBufferTooSmall {
        /// Required `f64` element count.
        required: usize,
        /// Provided slice length.
        actual: usize,
    },
    /// Caller [`LabyrinthRenderScratch`] lacks capacity for the level or render pass.
    ScratchTooSmall {
        /// Required capacity.
        required: usize,
        /// Available capacity.
        actual: usize,
    },
    /// Tile wall ID, door texture ID, or sprite texture index is missing from [`LabyrinthAssets`].
    InvalidMaterialId {
        /// The out-of-range material or texture identifier.
        id: u16,
    },
    /// [`LabyrinthAssets`] has empty textures, oversized dimensions, or invalid FOV.
    InvalidAssets,
    /// [`LabyrinthInput`] contains out-of-bounds controls or excessive `dt_ns`.
    InvalidInput,
    /// Checked integer arithmetic overflow (`CWE-190`).
    ArithmeticOverflow,
    /// Propagated `game-clock` error.
    Clock(ClockError),
    /// Propagated `raycast-dda` error.
    Raycast(RaycastError),
    /// Propagated `raster3d-soft` error.
    Raster(RasterError),
    /// Propagated `math3d` error.
    Math(math3d::Math3dError),
}

impl std::fmt::Display for LabyrinthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "input contains NaN or Infinity"),
            Self::ValueOutOfBounds => write!(f, "value is outside allowed bounds"),
            Self::InvalidGrid => write!(f, "invalid labyrinth grid dimensions or layout"),
            Self::InvalidPlayer => write!(f, "invalid player state or collision parameters"),
            Self::InvalidDoor => write!(f, "invalid door state or coordinates"),
            Self::InvalidPickup => write!(f, "invalid pickup state or coordinates"),
            Self::InvalidEntity => write!(f, "invalid entity state or coordinates"),
            Self::InvalidFramebuffer => write!(f, "invalid framebuffer dimensions or byte length"),
            Self::ZBufferTooSmall { required, actual } => {
                write!(f, "z-buffer too small: need {required}, got {actual}")
            }
            Self::ScratchTooSmall { required, actual } => {
                write!(f, "render scratch too small: need {required}, got {actual}")
            }
            Self::InvalidMaterialId { id } => write!(f, "material/texture id {id} out of range"),
            Self::InvalidAssets => write!(f, "invalid labyrinth asset pack or FOV"),
            Self::InvalidInput => write!(f, "invalid labyrinth input frame"),
            Self::ArithmeticOverflow => write!(f, "checked integer arithmetic overflow"),
            Self::Clock(e) => write!(f, "clock error: {e}"),
            Self::Raycast(e) => write!(f, "raycast-dda error: {e}"),
            Self::Raster(e) => write!(f, "raster3d-soft error: {e}"),
            Self::Math(e) => write!(f, "math3d error: {e}"),
        }
    }
}

impl std::error::Error for LabyrinthError {}

impl From<ClockError> for LabyrinthError {
    #[inline]
    fn from(err: ClockError) -> Self {
        Self::Clock(err)
    }
}

impl From<RaycastError> for LabyrinthError {
    #[inline]
    fn from(err: RaycastError) -> Self {
        Self::Raycast(err)
    }
}

impl From<RasterError> for LabyrinthError {
    #[inline]
    fn from(err: RasterError) -> Self {
        Self::Raster(err)
    }
}

impl From<math3d::Math3dError> for LabyrinthError {
    #[inline]
    fn from(err: math3d::Math3dError) -> Self {
        Self::Math(err)
    }
}

// ---------------------------------------------------------------------------
// Warrant Contract Validators (`code/specs/game-labyrinth3d.warrant`)
// ---------------------------------------------------------------------------

/// Validate tile grid dimensions against `[1, MAX_GRID_DIM]` (`game-labyrinth3d.warrant`).
#[inline]
pub fn validate_grid_dim(w: usize, h: usize) -> Result<bool, LabyrinthError> {
    if w == 0 || h == 0 || w > MAX_GRID_DIM || h > MAX_GRID_DIM {
        return Err(LabyrinthError::InvalidGrid);
    }
    Ok(true)
}

/// Validate framebuffer dimensions against `[1, MAX_FRAME_WIDTH]` x `[1, MAX_FRAME_HEIGHT]`.
#[inline]
pub fn validate_frame_dim(w: u32, h: u32) -> Result<bool, LabyrinthError> {
    if w == 0 || h == 0 || w > MAX_FRAME_WIDTH || h > MAX_FRAME_HEIGHT {
        return Err(LabyrinthError::InvalidFramebuffer);
    }
    Ok(true)
}

/// Validate entity slice length against `MAX_ENTITIES` (`game-labyrinth3d.warrant`).
#[inline]
pub fn validate_entity_count(n: usize) -> Result<bool, LabyrinthError> {
    if n > MAX_ENTITIES {
        return Err(LabyrinthError::InvalidEntity);
    }
    Ok(true)
}

/// Validate door slice length against `MAX_DOORS` (`game-labyrinth3d.warrant`).
#[inline]
pub fn validate_door_count(n: usize) -> Result<bool, LabyrinthError> {
    if n > MAX_DOORS {
        return Err(LabyrinthError::InvalidDoor);
    }
    Ok(true)
}

/// Validate pickup slice length against `MAX_PICKUPS` (`game-labyrinth3d.warrant`).
#[inline]
pub fn validate_pickup_count(n: usize) -> Result<bool, LabyrinthError> {
    if n > MAX_PICKUPS {
        return Err(LabyrinthError::InvalidPickup);
    }
    Ok(true)
}

/// Validate a key slot identifier against `0..MAX_KEYS` (`game-labyrinth3d.warrant`).
#[inline]
pub fn validate_key_id(k: u8) -> Result<bool, LabyrinthError> {
    if k >= MAX_KEYS {
        return Err(LabyrinthError::ValueOutOfBounds);
    }
    Ok(true)
}

/// Validate player collision radius in micro-units (`[50_000, 450_000]`).
#[inline]
pub fn validate_player_radius_micro(r_micro: u64) -> Result<bool, LabyrinthError> {
    if !(50_000..=450_000).contains(&r_micro) {
        return Err(LabyrinthError::InvalidPlayer);
    }
    Ok(true)
}

/// Validate player collision radius in grid units (`[MIN_PLAYER_RADIUS, MAX_PLAYER_RADIUS]`).
#[inline]
pub fn validate_player_radius(r: f64) -> Result<bool, LabyrinthError> {
    if !r.is_finite() {
        return Err(LabyrinthError::NonFiniteInput);
    }
    if !(MIN_PLAYER_RADIUS..=MAX_PLAYER_RADIUS).contains(&r) {
        return Err(LabyrinthError::InvalidPlayer);
    }
    Ok(true)
}

// ---------------------------------------------------------------------------
// Internal Validation & Numeric Helpers
// ---------------------------------------------------------------------------

#[inline]
fn check_finite_coord(v: f64) -> Result<f64, LabyrinthError> {
    if !v.is_finite() {
        return Err(LabyrinthError::NonFiniteInput);
    }
    if v.abs() > MAX_COORD {
        return Err(LabyrinthError::ValueOutOfBounds);
    }
    Ok(v)
}

#[inline]
fn check_finite_bounded(v: f64, limit: f64) -> Result<f64, LabyrinthError> {
    if !v.is_finite() {
        return Err(LabyrinthError::NonFiniteInput);
    }
    if v.abs() > limit {
        return Err(LabyrinthError::ValueOutOfBounds);
    }
    Ok(v)
}

#[inline]
fn normalize_angle(rad: f64) -> f64 {
    let two_pi = 2.0 * trig::PI;
    let mut a = rad - (rad / two_pi).floor() * two_pi;
    if a > trig::PI {
        a -= two_pi;
    }
    if a == -0.0 {
        0.0
    } else {
        a
    }
}

#[inline]
fn fnv1a_bytes(h: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *h ^= u64::from(b);
        *h = h.wrapping_mul(FNV1A_PRIME);
    }
}

#[inline]
fn fnv1a_f64(h: &mut u64, v: f64) {
    let canonical = if v.is_nan() {
        f64::NAN
    } else if v == 0.0 {
        0.0_f64
    } else {
        v
    };
    fnv1a_bytes(h, &canonical.to_le_bytes());
}

// ---------------------------------------------------------------------------
// Tile Grid & Map Layout
// ---------------------------------------------------------------------------

/// Logical kind of each cell in [`LabyrinthGrid`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileKind {
    /// Open walkable dungeon floor.
    Floor,
    /// Solid wall with 1-based material ID (`1..=255`).
    Wall(u8),
    /// Open player starting tile.
    Start,
    /// Open level exit goal tile.
    Exit,
}

impl TileKind {
    /// Returns `true` if this tile blocks movement and rays unconditionally.
    #[inline]
    pub const fn tile_is_solid(&self) -> bool {
        matches!(self, Self::Wall(_))
    }

    /// Returns the 1-based wall material ID if this tile is a solid wall.
    #[inline]
    pub const fn tile_wall_id(&self) -> Option<u8> {
        match self {
            Self::Wall(id) => Some(*id),
            Self::Floor | Self::Start | Self::Exit => None,
        }
    }
}

/// Bounded 2D tile grid (`width * height` row-major).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabyrinthGrid {
    width: usize,
    height: usize,
    tiles: Vec<TileKind>,
}

impl LabyrinthGrid {
    /// Construct a validated [`LabyrinthGrid`] from row-major `tiles`.
    pub fn lgrid_new(
        width: usize,
        height: usize,
        tiles: Vec<TileKind>,
    ) -> Result<Self, LabyrinthError> {
        validate_grid_dim(width, height)?;
        let expected = width
            .checked_mul(height)
            .ok_or(LabyrinthError::ArithmeticOverflow)?;
        if tiles.len() != expected {
            return Err(LabyrinthError::InvalidGrid);
        }
        let grid = Self {
            width,
            height,
            tiles,
        };
        grid.lgrid_validate()?;
        Ok(grid)
    }

    /// Parse an ASCII labyrinth grid where `#` = wall 1, `%` = wall 2, `S` = start, `E` = exit, `.` = floor.
    pub fn lgrid_from_ascii(rows: &[&str]) -> Result<Self, LabyrinthError> {
        if rows.is_empty() || rows.len() > MAX_GRID_DIM {
            return Err(LabyrinthError::InvalidGrid);
        }
        let height = rows.len();
        let width = rows[0].len();
        validate_grid_dim(width, height)?;
        let expected = width
            .checked_mul(height)
            .ok_or(LabyrinthError::ArithmeticOverflow)?;
        let mut tiles = Vec::with_capacity(expected);
        for row in rows {
            if row.len() != width {
                return Err(LabyrinthError::InvalidGrid);
            }
            for b in row.bytes() {
                let kind = match b {
                    b'#' => TileKind::Wall(1),
                    b'%' => TileKind::Wall(2),
                    b'S' => TileKind::Start,
                    b'E' => TileKind::Exit,
                    b'.' | b' ' => TileKind::Floor,
                    _ => return Err(LabyrinthError::InvalidGrid),
                };
                tiles.push(kind);
            }
        }
        Self::lgrid_new(width, height, tiles)
    }

    /// Grid width in cells.
    #[inline]
    pub const fn lgrid_width(&self) -> usize {
        self.width
    }

    /// Grid height in cells.
    #[inline]
    pub const fn lgrid_height(&self) -> usize {
        self.height
    }

    /// Borrow the row-major tile slice.
    #[inline]
    pub fn lgrid_tiles(&self) -> &[TileKind] {
        &self.tiles
    }

    /// Checked lookup of the tile at `(x, y)`.
    #[inline]
    pub fn lgrid_tile_at(&self, x: usize, y: usize) -> Option<TileKind> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = y.checked_mul(self.width)?.checked_add(x)?;
        self.tiles.get(idx).copied()
    }

    /// Validate dimensions, buffer length, non-zero wall IDs, and presence of at least one `Exit`.
    pub fn lgrid_validate(&self) -> Result<(), LabyrinthError> {
        validate_grid_dim(self.width, self.height)?;
        let expected = self
            .width
            .checked_mul(self.height)
            .ok_or(LabyrinthError::ArithmeticOverflow)?;
        if self.tiles.len() != expected {
            return Err(LabyrinthError::InvalidGrid);
        }
        let mut has_exit = false;
        for &tile in &self.tiles {
            match tile {
                TileKind::Wall(0) => return Err(LabyrinthError::InvalidGrid),
                TileKind::Exit => has_exit = true,
                _ => {}
            }
        }
        if !has_exit {
            return Err(LabyrinthError::InvalidGrid);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Doors, Pickups, Entities, Player, Explicit RNG
// ---------------------------------------------------------------------------

/// A toggleable or key-locked door placed on a grid cell.
///
/// While `open == false`, the cell `(x, y)` blocks movement and renders as a
/// wall using 1-based `texture_id`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DoorState {
    /// Grid X coordinate of the door cell.
    pub x: usize,
    /// Grid Y coordinate of the door cell.
    pub y: usize,
    /// Optional key ID (`0..MAX_KEYS`) required to unlock this door.
    pub required_key: Option<u8>,
    /// `true` once unlocked and opened.
    pub open: bool,
    /// 1-based wall texture ID when closed.
    pub texture_id: u16,
}

impl DoorState {
    /// Construct a validated closed [`DoorState`].
    pub fn door_new(
        x: usize,
        y: usize,
        required_key: Option<u8>,
        texture_id: u16,
    ) -> Result<Self, LabyrinthError> {
        if let Some(k) = required_key {
            validate_key_id(k)?;
        }
        if texture_id == 0 {
            return Err(LabyrinthError::InvalidDoor);
        }
        Ok(Self {
            x,
            y,
            required_key,
            open: false,
            texture_id,
        })
    }

    /// Validate door bounds and texture/key fields against a grid of `(width, height)`.
    pub fn door_validate(&self, width: usize, height: usize) -> Result<(), LabyrinthError> {
        if self.x >= width || self.y >= height || self.texture_id == 0 {
            return Err(LabyrinthError::InvalidDoor);
        }
        if let Some(k) = self.required_key {
            validate_key_id(k)?;
        }
        Ok(())
    }
}

/// Kind of collectible item placed inside the dungeon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickupKind {
    /// Grants key bit `k` (`0..MAX_KEYS`) for unlocking matching doors.
    Key(u8),
    /// Restores up to `amount` HP (clamped at `max_hp`).
    Health(u16),
    /// Grants `points` to `player.score`.
    Treasure(u32),
}

/// Collectible item instance in world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickupItem {
    /// Unique caller-assigned pickup identifier.
    pub id: u16,
    /// World X coordinate.
    pub x: f64,
    /// World Y coordinate.
    pub y: f64,
    /// Gameplay effect when collected.
    pub kind: PickupKind,
    /// `true` once picked up by the player.
    pub collected: bool,
    /// 0-based texture index in [`LabyrinthAssets`].
    pub texture_index: usize,
}

impl PickupItem {
    /// Construct a validated uncollected [`PickupItem`].
    pub fn pickup_new(
        id: u16,
        x: f64,
        y: f64,
        kind: PickupKind,
        texture_index: usize,
    ) -> Result<Self, LabyrinthError> {
        let x = check_finite_coord(x)?;
        let y = check_finite_coord(y)?;
        if let PickupKind::Key(k) = kind {
            validate_key_id(k)?;
        }
        Ok(Self {
            id,
            x,
            y,
            kind,
            collected: false,
            texture_index,
        })
    }

    /// Validate coordinates and key ID against a grid of `(width, height)`.
    pub fn pickup_validate(&self, width: usize, height: usize) -> Result<(), LabyrinthError> {
        if !self.x.is_finite() || !self.y.is_finite() {
            return Err(LabyrinthError::NonFiniteInput);
        }
        if self.x < 0.0 || self.y < 0.0 || self.x >= width as f64 || self.y >= height as f64 {
            return Err(LabyrinthError::InvalidPickup);
        }
        if let PickupKind::Key(k) = self.kind {
            validate_key_id(k)?;
        }
        Ok(())
    }
}

/// Movement behavior for dungeon entities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityBehavior {
    /// Roams deterministically, turning via [`LabyrinthRng`] when blocked.
    Patrol,
    /// Pursues the player when within range and line-of-sight, otherwise patrols.
    Chase,
}

/// Dynamic patrolling or chasing dungeon entity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntityState {
    /// Unique entity identifier.
    pub id: u16,
    /// World X coordinate.
    pub x: f64,
    /// World Y coordinate.
    pub y: f64,
    /// Current heading angle in radians.
    pub yaw: f64,
    /// Collision radius in grid units.
    pub radius: f64,
    /// Movement speed in grid units per second.
    pub speed: f64,
    /// AI behavior mode.
    pub behavior: EntityBehavior,
    /// Entity hit points.
    pub hp: u16,
    /// Contact damage dealt to player when cooldown reaches zero.
    pub damage: u16,
    /// Whether this entity is active.
    pub alive: bool,
    /// Remaining ticks before next melee contact damage.
    pub cooldown_ticks: u32,
    /// 0-based sprite texture index in [`LabyrinthAssets`].
    pub texture_index: usize,
}

impl EntityState {
    /// Construct a validated [`EntityState`].
    #[allow(clippy::too_many_arguments)]
    pub fn entity_new(
        id: u16,
        x: f64,
        y: f64,
        yaw: f64,
        radius: f64,
        speed: f64,
        behavior: EntityBehavior,
        hp: u16,
        damage: u16,
        texture_index: usize,
    ) -> Result<Self, LabyrinthError> {
        let x = check_finite_coord(x)?;
        let y = check_finite_coord(y)?;
        let yaw = check_finite_bounded(yaw, math3d::MAX_ANGLE_RAD)?;
        if !radius.is_finite() || !speed.is_finite() {
            return Err(LabyrinthError::NonFiniteInput);
        }
        if !(MIN_PLAYER_RADIUS..=MAX_PLAYER_RADIUS).contains(&radius)
            || !(0.0..=50.0).contains(&speed)
        {
            return Err(LabyrinthError::InvalidEntity);
        }
        Ok(Self {
            id,
            x,
            y,
            yaw: normalize_angle(yaw),
            radius,
            speed,
            behavior,
            hp,
            damage,
            alive: hp > 0,
            cooldown_ticks: 0,
            texture_index,
        })
    }

    /// Validate entity fields against a grid of `(width, height)`.
    pub fn entity_validate(&self, width: usize, height: usize) -> Result<(), LabyrinthError> {
        if !self.x.is_finite()
            || !self.y.is_finite()
            || !self.yaw.is_finite()
            || !self.radius.is_finite()
            || !self.speed.is_finite()
        {
            return Err(LabyrinthError::NonFiniteInput);
        }
        if self.x < 0.0
            || self.y < 0.0
            || self.x >= width as f64
            || self.y >= height as f64
            || self.yaw.abs() > math3d::MAX_ANGLE_RAD
            || !(MIN_PLAYER_RADIUS..=MAX_PLAYER_RADIUS).contains(&self.radius)
            || !(0.0..=50.0).contains(&self.speed)
        {
            return Err(LabyrinthError::InvalidEntity);
        }
        Ok(())
    }
}

/// Player state inside the labyrinth.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerState {
    /// World X coordinate.
    pub x: f64,
    /// World Y coordinate.
    pub y: f64,
    /// Facing yaw angle in radians (`0.0` points along `+X`).
    pub yaw: f64,
    /// Circle collision radius in grid units.
    pub radius: f64,
    /// Linear movement speed in grid units per second.
    pub move_speed: f64,
    /// Angular turn speed in radians per second.
    pub turn_speed: f64,
    /// Current hit points.
    pub hp: u16,
    /// Maximum hit points.
    pub max_hp: u16,
    /// 8-bit key possession bitmask (bit `k` = key `k`).
    pub key_mask: u8,
    /// Accumulated treasure score.
    pub score: u32,
}

impl PlayerState {
    /// Construct a validated [`PlayerState`].
    pub fn player_new(
        x: f64,
        y: f64,
        yaw: f64,
        radius: f64,
        move_speed: f64,
        turn_speed: f64,
        max_hp: u16,
    ) -> Result<Self, LabyrinthError> {
        let x = check_finite_coord(x)?;
        let y = check_finite_coord(y)?;
        let yaw = check_finite_bounded(yaw, math3d::MAX_ANGLE_RAD)?;
        validate_player_radius(radius)?;
        if !move_speed.is_finite() || !turn_speed.is_finite() {
            return Err(LabyrinthError::NonFiniteInput);
        }
        if !(0.0..=50.0).contains(&move_speed) || !(0.0..=50.0).contains(&turn_speed) || max_hp == 0
        {
            return Err(LabyrinthError::InvalidPlayer);
        }
        Ok(Self {
            x,
            y,
            yaw: normalize_angle(yaw),
            radius,
            move_speed,
            turn_speed,
            hp: max_hp,
            max_hp,
            key_mask: 0,
            score: 0,
        })
    }

    /// Validate all player fields against a grid of `(width, height)`.
    pub fn player_validate(&self, width: usize, height: usize) -> Result<(), LabyrinthError> {
        if !self.x.is_finite()
            || !self.y.is_finite()
            || !self.yaw.is_finite()
            || !self.radius.is_finite()
            || !self.move_speed.is_finite()
            || !self.turn_speed.is_finite()
        {
            return Err(LabyrinthError::NonFiniteInput);
        }
        validate_player_radius(self.radius)?;
        if self.x < self.radius
            || self.y < self.radius
            || self.x > (width as f64) - self.radius
            || self.y > (height as f64) - self.radius
            || self.yaw.abs() > math3d::MAX_ANGLE_RAD
            || !(0.0..=50.0).contains(&self.move_speed)
            || !(0.0..=50.0).contains(&self.turn_speed)
            || self.max_hp == 0
            || self.hp > self.max_hp
        {
            return Err(LabyrinthError::InvalidPlayer);
        }
        Ok(())
    }

    /// Returns `true` if the player holds key `key_id` (`0..MAX_KEYS`).
    #[inline]
    pub const fn player_has_key(&self, key_id: u8) -> bool {
        if key_id >= MAX_KEYS {
            return false;
        }
        (self.key_mask & (1u8 << key_id)) != 0
    }
}

/// Deterministic explicit RNG state backed by `rng::Pcg32`.
///
/// Storing the explicit `(seed, state_word, step_count)` tuple makes
/// [`LabyrinthState`] trivially `Clone`, `PartialEq`, and digest-hashable while
/// every draw executes `rng::Pcg32` directly with zero heap allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LabyrinthRng {
    seed: u64,
    state_word: u64,
    step_count: u64,
}

impl LabyrinthRng {
    /// Construct a new deterministic [`LabyrinthRng`] from `seed`.
    #[inline]
    pub const fn lrng_new(seed: u64) -> Self {
        Self {
            seed,
            state_word: seed ^ 0x9e37_79b9_7f4a_7c15,
            step_count: 0,
        }
    }

    /// Advance the explicit RNG state by one `rng::Pcg32` draw and return `u32`.
    #[inline]
    pub fn lrng_next_u32(&mut self) -> u32 {
        let mut gen = rng::Pcg32::new(self.state_word);
        let out = gen.next_u32();
        self.state_word = gen.next_u64();
        self.step_count = self.step_count.wrapping_add(1);
        out
    }

    /// Advance the explicit RNG state and return a `f64` uniformly distributed in `[0.0, 1.0)`.
    #[inline]
    pub fn lrng_next_float(&mut self) -> f64 {
        let mut gen = rng::Pcg32::new(self.state_word);
        let out = gen.next_float();
        self.state_word = gen.next_u64();
        self.step_count = self.step_count.wrapping_add(1);
        out
    }

    /// Initial seed value.
    #[inline]
    pub const fn lrng_seed(&self) -> u64 {
        self.seed
    }

    /// Current explicit state word.
    #[inline]
    pub const fn lrng_state_word(&self) -> u64 {
        self.state_word
    }

    /// Total number of draws executed.
    #[inline]
    pub const fn lrng_step_count(&self) -> u64 {
        self.step_count
    }
}

// ---------------------------------------------------------------------------
// Game Status, Render Mode, Input, State, Assets, Scratch, Reports
// ---------------------------------------------------------------------------

/// Terminal or active status of a labyrinth run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameStatus {
    /// Run is active and accepting player controls.
    Playing,
    /// Player reached the dungeon exit tile.
    Won,
    /// Player hit points reached zero from entity contact.
    Lost,
}

/// Active software renderer selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderMode {
    /// Classic 2.5D DDA column raycaster + billboard sprites (`raycast-dda`).
    Raycast25D,
    /// Perspective 3D triangle mesh rasterizer with Z-buffer (`raster3d-soft`).
    Software3D,
}

/// Deterministic per-frame player input for [`step_in_place`] and [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabyrinthInput {
    /// Forward/backward throttle in `[-MAX_CONTROL_MAGNITUDE, MAX_CONTROL_MAGNITUDE]`.
    pub forward: f64,
    /// Right/left strafe throttle in `[-MAX_CONTROL_MAGNITUDE, MAX_CONTROL_MAGNITUDE]`.
    pub strafe: f64,
    /// Counter-clockwise yaw turn input in `[-MAX_CONTROL_MAGNITUDE, MAX_CONTROL_MAGNITUDE]`.
    pub turn: f64,
    /// Interact/use trigger (unlock/open facing or adjacent doors).
    pub interact: bool,
    /// Elapsed wall-clock delta in nanoseconds (`<= MAX_DT_NS`).
    pub dt_ns: u64,
}

impl LabyrinthInput {
    /// Construct an idle input with `dt_ns` elapsed.
    #[inline]
    pub const fn input_idle(dt_ns: u64) -> Self {
        Self {
            forward: 0.0,
            strafe: 0.0,
            turn: 0.0,
            interact: false,
            dt_ns,
        }
    }

    /// Construct and validate a [`LabyrinthInput`].
    pub fn input_new(
        forward: f64,
        strafe: f64,
        turn: f64,
        interact: bool,
        dt_ns: u64,
    ) -> Result<Self, LabyrinthError> {
        let input = Self {
            forward,
            strafe,
            turn,
            interact,
            dt_ns,
        };
        input.input_validate()?;
        Ok(input)
    }

    /// Validate all fields of this input frame (`CWE-20` / `CWE-400`).
    pub fn input_validate(&self) -> Result<(), LabyrinthError> {
        let _ = check_finite_bounded(self.forward, MAX_CONTROL_MAGNITUDE)?;
        let _ = check_finite_bounded(self.strafe, MAX_CONTROL_MAGNITUDE)?;
        let _ = check_finite_bounded(self.turn, MAX_CONTROL_MAGNITUDE)?;
        if self.dt_ns > MAX_DT_NS {
            return Err(LabyrinthError::InvalidInput);
        }
        Ok(())
    }
}

/// Deterministic summary emitted by [`step_in_place`] and [`step`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepOutcome {
    /// Number of fixed-timestep simulation ticks executed this call.
    pub ticks_simulated: u32,
    /// Game status after stepping.
    pub status: GameStatus,
    /// Number of locked/closed doors opened during this call.
    pub doors_opened: u32,
    /// Number of pickups collected during this call.
    pub pickups_collected: u32,
    /// Total hit-point damage suffered during this call.
    pub damage_taken: u32,
    /// Canonical FNV-1a digest of the resulting [`LabyrinthState`].
    pub state_digest: u64,
}

/// Complete, deterministic, self-contained game state for `game-labyrinth3d`.
#[derive(Clone, Debug, PartialEq)]
pub struct LabyrinthState {
    /// Tile grid layout.
    pub grid: LabyrinthGrid,
    /// Player position, orientation, health, keys, and score.
    pub player: PlayerState,
    /// Doors placed in the dungeon.
    pub doors: Vec<DoorState>,
    /// Collectible keys, health packs, and treasure.
    pub pickups: Vec<PickupItem>,
    /// Patrolling and chasing entities.
    pub entities: Vec<EntityState>,
    /// Deterministic fixed-timestep clock state (`game-clock`).
    pub clock: GameClockState,
    /// Explicit deterministic RNG state (`rng`).
    pub rng: LabyrinthRng,
    /// Total fixed simulation ticks completed.
    pub tick: u64,
    /// Current terminal or active status.
    pub status: GameStatus,
    /// Grid X coordinate of the primary exit tile.
    pub exit_x: usize,
    /// Grid Y coordinate of the primary exit tile.
    pub exit_y: usize,
}

impl LabyrinthState {
    /// Construct and fully validate a [`LabyrinthState`].
    pub fn state_new(
        grid: LabyrinthGrid,
        player: PlayerState,
        doors: Vec<DoorState>,
        pickups: Vec<PickupItem>,
        entities: Vec<EntityState>,
        seed: u64,
        clock_config: GameClockConfig,
    ) -> Result<Self, LabyrinthError> {
        grid.lgrid_validate()?;
        validate_door_count(doors.len())?;
        validate_pickup_count(pickups.len())?;
        validate_entity_count(entities.len())?;
        let clock = game_clock::new_clock(clock_config)?;

        let mut exit_coords = None;
        for y in 0..grid.height {
            for x in 0..grid.width {
                if matches!(grid.lgrid_tile_at(x, y), Some(TileKind::Exit)) {
                    exit_coords = Some((x, y));
                    break;
                }
            }
            if exit_coords.is_some() {
                break;
            }
        }
        let (exit_x, exit_y) = exit_coords.ok_or(LabyrinthError::InvalidGrid)?;

        let state = Self {
            grid,
            player,
            doors,
            pickups,
            entities,
            clock,
            rng: LabyrinthRng::lrng_new(seed),
            tick: 0,
            status: GameStatus::Playing,
            exit_x,
            exit_y,
        };
        state.state_validate()?;
        Ok(state)
    }

    /// Build a playable 11x11 reference dungeon with locked door, key, pickups, and patrolling/chasing guards.
    pub fn state_build_demo(seed: u64) -> Result<Self, LabyrinthError> {
        let rows = [
            "###########",
            "#S..#.....#",
            "#.#.#.###.#",
            "#...#.#E#.#",
            "###.###.###",
            "#.........#",
            "#.%.###.%.#",
            "#...#.#...#",
            "#.###.###.#",
            "#.........#",
            "###########",
        ];
        let grid = LabyrinthGrid::lgrid_from_ascii(&rows)?;
        let player = PlayerState::player_new(1.5, 1.5, 0.0, 0.25, 3.0, 2.5, 100)?;
        // Door at (7, 4) guards the exit chamber at (7, 3).
        let doors = vec![DoorState::door_new(7, 4, Some(0), 3)?];
        let pickups = vec![
            PickupItem::pickup_new(1, 1.5, 5.5, PickupKind::Key(0), 4)?,
            PickupItem::pickup_new(2, 5.5, 5.5, PickupKind::Treasure(100), 5)?,
            PickupItem::pickup_new(3, 9.5, 7.5, PickupKind::Health(25), 5)?,
        ];
        let entities = vec![
            EntityState::entity_new(
                1,
                5.5,
                9.5,
                0.0,
                0.25,
                1.2,
                EntityBehavior::Patrol,
                20,
                15,
                6,
            )?,
            EntityState::entity_new(
                2,
                8.5,
                1.5,
                trig::PI,
                0.25,
                1.4,
                EntityBehavior::Chase,
                30,
                20,
                6,
            )?,
        ];
        Self::state_new(
            grid,
            player,
            doors,
            pickups,
            entities,
            seed,
            GameClockConfig::standard_60hz(),
        )
    }

    /// Validate every invariant of [`LabyrinthState`] upfront (`CWE-20` / `CWE-125` / `CWE-400`).
    pub fn state_validate(&self) -> Result<(), LabyrinthError> {
        self.grid.lgrid_validate()?;
        validate_door_count(self.doors.len())?;
        validate_pickup_count(self.pickups.len())?;
        validate_entity_count(self.entities.len())?;
        game_clock::validate_state(&self.clock)?;

        let w = self.grid.width;
        let h = self.grid.height;
        self.player.player_validate(w, h)?;

        for door in &self.doors {
            door.door_validate(w, h)?;
            if let Some(t) = self.grid.lgrid_tile_at(door.x, door.y) {
                if t.tile_is_solid() {
                    return Err(LabyrinthError::InvalidDoor);
                }
            }
        }
        for pickup in &self.pickups {
            pickup.pickup_validate(w, h)?;
        }
        for entity in &self.entities {
            entity.entity_validate(w, h)?;
        }

        if !matches!(
            self.grid.lgrid_tile_at(self.exit_x, self.exit_y),
            Some(TileKind::Exit)
        ) {
            return Err(LabyrinthError::InvalidGrid);
        }
        Ok(())
    }

    /// Returns `true` if grid cell `(tx, ty)` is out of bounds, a solid wall, or a closed door.
    #[inline]
    pub fn state_cell_blocked(&self, tx: usize, ty: usize) -> bool {
        cell_blocked(&self.grid, &self.doors, tx, ty)
    }
}

#[inline]
fn cell_blocked(grid: &LabyrinthGrid, doors: &[DoorState], tx: usize, ty: usize) -> bool {
    let Some(tile) = grid.lgrid_tile_at(tx, ty) else {
        return true;
    };
    if tile.tile_is_solid() {
        return true;
    }
    for door in doors {
        if !door.open && door.x == tx && door.y == ty {
            return true;
        }
    }
    false
}

/// Pre-built deterministic textures and camera FOV shared by both renderers.
#[derive(Clone, Debug, PartialEq)]
pub struct LabyrinthAssets {
    /// Texture table indexed by `wall_id - 1`, `door.texture_id - 1`, and `texture_index`.
    pub textures: Vec<TextureRgba>,
    /// Solid floor RGBA8 fill.
    pub floor_color: Rgba8,
    /// Solid ceiling RGBA8 fill.
    pub ceiling_color: Rgba8,
    /// Horizontal field of view in radians (`[MIN_FOV_RAD, MAX_FOV_RAD]`).
    pub fov_rad: f64,
}

impl LabyrinthAssets {
    /// Construct a validated [`LabyrinthAssets`] pack.
    pub fn assets_new(textures: Vec<TextureRgba>, fov_rad: f64) -> Result<Self, LabyrinthError> {
        let assets = Self {
            textures,
            floor_color: [68, 60, 52, 255],
            ceiling_color: [34, 40, 54, 255],
            fov_rad,
        };
        assets.assets_validate()?;
        Ok(assets)
    }

    /// Construct the default 7-texture deterministic 16x16 dungeon asset pack.
    pub fn assets_default() -> Result<Self, LabyrinthError> {
        let wall_stone =
            TextureRgba::texture_checkerboard(16, 16, 4, [118, 124, 136, 255], [88, 94, 108, 255])?;
        let wall_brick =
            TextureRgba::texture_checkerboard(16, 16, 4, [148, 114, 78, 255], [114, 84, 54, 255])?;
        let door_bronze =
            TextureRgba::texture_checkerboard(16, 16, 2, [168, 128, 52, 255], [96, 68, 28, 255])?;
        let exit_portal =
            TextureRgba::texture_checkerboard(16, 16, 4, [52, 224, 132, 255], [24, 138, 84, 255])?;
        let sprite_key = make_centered_orb_texture([248, 212, 56, 255], [180, 140, 20, 255])?;
        let sprite_pickup = make_centered_orb_texture([64, 224, 232, 255], [24, 132, 168, 255])?;
        let sprite_guard = make_centered_orb_texture([228, 56, 64, 255], [144, 24, 32, 255])?;

        Self::assets_new(
            vec![
                wall_stone,
                wall_brick,
                door_bronze,
                exit_portal,
                sprite_key,
                sprite_pickup,
                sprite_guard,
            ],
            DEFAULT_FOV_RAD,
        )
    }

    /// Validate texture count and FOV bounds (`CWE-20` / `CWE-400`).
    pub fn assets_validate(&self) -> Result<(), LabyrinthError> {
        if self.textures.is_empty() || self.textures.len() > MAX_TEXTURES {
            return Err(LabyrinthError::InvalidAssets);
        }
        if !self.fov_rad.is_finite() {
            return Err(LabyrinthError::NonFiniteInput);
        }
        if !(MIN_FOV_RAD..=MAX_FOV_RAD).contains(&self.fov_rad) {
            return Err(LabyrinthError::InvalidAssets);
        }
        Ok(())
    }

    /// Number of textures registered in this asset pack.
    #[inline]
    pub fn assets_texture_count(&self) -> usize {
        self.textures.len()
    }
}

fn make_centered_orb_texture(core: Rgba8, rim: Rgba8) -> Result<TextureRgba, LabyrinthError> {
    let mut pixels = vec![0u8; 16 * 16 * 4];
    let (chunks, _) = pixels.as_chunks_mut::<4>();
    for (idx, px) in chunks.iter_mut().enumerate() {
        let x = (idx % 16) as i32 - 8;
        let y = (idx / 16) as i32 - 8;
        let r2 = x * x + y * y;
        *px = if r2 <= 12 {
            core
        } else if r2 <= 28 {
            rim
        } else {
            [0, 0, 0, 0]
        };
    }
    Ok(TextureRgba::texture_new(16, 16, pixels)?)
}

/// Pre-allocated rendering workspace shared across both `Raycast25D` and `Software3D`.
#[derive(Clone, Debug)]
pub struct LabyrinthRenderScratch {
    cached_grid: Option<GridMap>,
    raycast_sprites: Vec<Sprite>,
    sprite_scratch: SpriteScratch,
    raster_vertices: Vec<RasterVertex>,
    raster_indices: Vec<u32>,
    raster_scratch: RasterScratch,
}

impl LabyrinthRenderScratch {
    /// Allocate a [`LabyrinthRenderScratch`] ready for steady-state zero-alloc rendering.
    pub fn rscratch_new() -> Result<Self, LabyrinthError> {
        let max_sprites = MAX_PICKUPS
            .checked_add(MAX_ENTITIES)
            .ok_or(LabyrinthError::ArithmeticOverflow)?;
        Ok(Self {
            cached_grid: None,
            raycast_sprites: Vec::with_capacity(max_sprites),
            sprite_scratch: SpriteScratch::scratch_with_capacity(max_sprites)?,
            raster_vertices: Vec::with_capacity(MAX_3D_VERTICES),
            raster_indices: Vec::with_capacity(MAX_3D_INDICES),
            raster_scratch: RasterScratch::scratch_with_capacity(MAX_3D_VERTICES)?,
        })
    }

    /// Allocate a [`LabyrinthRenderScratch`] pre-sized for `state`'s grid so even the first frame allocates zero bytes.
    pub fn rscratch_for_state(state: &LabyrinthState) -> Result<Self, LabyrinthError> {
        state.grid.lgrid_validate()?;
        let mut scratch = Self::rscratch_new()?;
        let w = state.grid.width;
        let h = state.grid.height;
        let cell_len = w.checked_mul(h).ok_or(LabyrinthError::ArithmeticOverflow)?;
        scratch.cached_grid = Some(GridMap::grid_new(w, h, vec![0u16; cell_len])?);
        Ok(scratch)
    }

    /// Pre-allocated 3D vertex capacity.
    #[inline]
    pub fn rscratch_vertex_capacity(&self) -> usize {
        self.raster_vertices.capacity()
    }

    /// Pre-allocated 2.5D billboard sprite capacity.
    #[inline]
    pub fn rscratch_sprite_capacity(&self) -> usize {
        self.raycast_sprites.capacity()
    }
}

/// Telemetry and canonical frame digest returned by [`render_frame_into`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderReport {
    /// Active renderer used to generate the frame.
    pub mode: RenderMode,
    /// Canonical 64-bit FNV-1a digest over framebuffer dimensions and RGBA8 pixels.
    pub frame_digest: u64,
    /// Pixels written (reported by `Software3D`, or `width * height` for `Raycast25D`).
    pub pixels_written: u64,
    /// Triangles rasterized (`Software3D`) or `0` (`Raycast25D`).
    pub triangles_rasterized: u64,
    /// Wall columns struck (`Raycast25D`) or `0` (`Software3D`).
    pub wall_hits: usize,
    /// Sprites projected (`Raycast25D`) or entity/pickup meshes submitted (`Software3D`).
    pub sprites_projected: usize,
}

// ---------------------------------------------------------------------------
// Collision, Line-of-Sight, and Simulation Step
// ---------------------------------------------------------------------------

fn resolve_circle_grid_pushout(
    mut cx: f64,
    mut cy: f64,
    radius: f64,
    grid: &LabyrinthGrid,
    doors: &[DoorState],
) -> (f64, f64) {
    let min_x = radius;
    let min_y = radius;
    let max_x = (grid.width as f64) - radius;
    let max_y = (grid.height as f64) - radius;

    for _pass in 0..2 {
        cx = cx.clamp(min_x, max_x);
        cy = cy.clamp(min_y, max_y);

        let tx0 = ((cx - radius).floor() as isize).max(0) as usize;
        let ty0 = ((cy - radius).floor() as isize).max(0) as usize;
        let tx1 = ((cx + radius).floor() as isize).clamp(0, (grid.width as isize) - 1) as usize;
        let ty1 = ((cy + radius).floor() as isize).clamp(0, (grid.height as isize) - 1) as usize;

        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                if !cell_blocked(grid, doors, tx, ty) {
                    continue;
                }
                let rx0 = tx as f64;
                let ry0 = ty as f64;
                let rx1 = rx0 + 1.0;
                let ry1 = ry0 + 1.0;

                let closest_x = cx.clamp(rx0, rx1);
                let closest_y = cy.clamp(ry0, ry1);
                let dx = cx - closest_x;
                let dy = cy - closest_y;
                let dist_sq = dx * dx + dy * dy;

                if dist_sq < radius * radius {
                    if dist_sq > EPSILON {
                        let dist = trig::sqrt(dist_sq);
                        let push = radius - dist;
                        cx += (dx / dist) * push;
                        cy += (dy / dist) * push;
                    } else {
                        // Circle center lies inside the solid tile; push toward nearest edge.
                        let d_left = (cx - rx0).abs();
                        let d_right = (rx1 - cx).abs();
                        let d_top = (cy - ry0).abs();
                        let d_bot = (ry1 - cy).abs();
                        let min_d = d_left.min(d_right).min(d_top).min(d_bot);
                        if min_d == d_left {
                            cx = rx0 - radius;
                        } else if min_d == d_right {
                            cx = rx1 + radius;
                        } else if min_d == d_top {
                            cy = ry0 - radius;
                        } else {
                            cy = ry1 + radius;
                        }
                    }
                }
            }
        }
    }

    (cx.clamp(min_x, max_x), cy.clamp(min_y, max_y))
}

fn move_circle_sliding(
    x: f64,
    y: f64,
    dx: f64,
    dy: f64,
    radius: f64,
    grid: &LabyrinthGrid,
    doors: &[DoorState],
) -> (f64, f64) {
    let (nx, ny) = resolve_circle_grid_pushout(x + dx, y, radius, grid, doors);
    resolve_circle_grid_pushout(nx, ny + dy, radius, grid, doors)
}

fn has_clear_los(
    grid: &LabyrinthGrid,
    doors: &[DoorState],
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
) -> bool {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let dist_sq = dx * dx + dy * dy;
    if dist_sq <= EPSILON {
        return true;
    }
    let dist = trig::sqrt(dist_sq);
    let steps = ((dist * 4.0).ceil() as usize).clamp(1, MAX_GRID_DIM * 8);
    let inv_steps = 1.0 / (steps as f64);
    for i in 1..steps {
        let t = (i as f64) * inv_steps;
        let sx = x0 + dx * t;
        let sy = y0 + dy * t;
        if sx < 0.0 || sy < 0.0 {
            return false;
        }
        let tx = sx.floor() as usize;
        let ty = sy.floor() as usize;
        if cell_blocked(grid, doors, tx, ty) {
            return false;
        }
    }
    true
}

fn step_single_tick(
    state: &mut LabyrinthState,
    input: &LabyrinthInput,
    dt_sec: f64,
    doors_opened: &mut u32,
    pickups_collected: &mut u32,
    damage_taken: &mut u32,
) {
    if state.status != GameStatus::Playing {
        return;
    }

    // 1. Rotate player yaw using deterministic `trig`.
    let delta_yaw = input.turn * state.player.turn_speed * dt_sec;
    state.player.yaw = normalize_angle(state.player.yaw + delta_yaw);

    let dir_x = trig::cos(state.player.yaw);
    let dir_y = trig::sin(state.player.yaw);
    let right_x = -dir_y;
    let right_y = dir_x;

    // 2. Compute desired displacement and apply circle-vs-grid sliding collision.
    let mut move_x = dir_x * input.forward + right_x * input.strafe;
    let mut move_y = dir_y * input.forward + right_y * input.strafe;
    let mag_sq = move_x * move_x + move_y * move_y;
    if mag_sq > 1.0 {
        let inv_mag = 1.0 / trig::sqrt(mag_sq);
        move_x *= inv_mag;
        move_y *= inv_mag;
    }
    let step_dist = state.player.move_speed * dt_sec;
    let (next_px, next_py) = move_circle_sliding(
        state.player.x,
        state.player.y,
        move_x * step_dist,
        move_y * step_dist,
        state.player.radius,
        &state.grid,
        &state.doors,
    );
    state.player.x = next_px;
    state.player.y = next_py;

    // 3. Interact / unlock nearby doors when holding the matching key (or keyless).
    if input.interact {
        let max_reach_sq = 1.65 * 1.65;
        for door in &mut state.doors {
            if door.open {
                continue;
            }
            let dcx = (door.x as f64) + 0.5;
            let dcy = (door.y as f64) + 0.5;
            let dx = dcx - state.player.x;
            let dy = dcy - state.player.y;
            if dx * dx + dy * dy <= max_reach_sq {
                let authorized = match door.required_key {
                    None => true,
                    Some(k) => state.player.player_has_key(k),
                };
                if authorized {
                    door.open = true;
                    *doors_opened = doors_opened.saturating_add(1);
                }
            }
        }
    }

    // 4. Collect overlapping pickups.
    let pickup_reach = state.player.radius + 0.35;
    let pickup_reach_sq = pickup_reach * pickup_reach;
    for item in &mut state.pickups {
        if item.collected {
            continue;
        }
        let dx = item.x - state.player.x;
        let dy = item.y - state.player.y;
        if dx * dx + dy * dy <= pickup_reach_sq {
            item.collected = true;
            *pickups_collected = pickups_collected.saturating_add(1);
            match item.kind {
                PickupKind::Key(k) if k < MAX_KEYS => {
                    state.player.key_mask |= 1u8 << k;
                }
                PickupKind::Key(_) => {}
                PickupKind::Health(amount) => {
                    state.player.hp = state
                        .player
                        .hp
                        .saturating_add(amount)
                        .min(state.player.max_hp);
                }
                PickupKind::Treasure(pts) => {
                    state.player.score = state.player.score.saturating_add(pts);
                }
            }
        }
    }

    // 5. Update patrolling / chasing entities and resolve player contact.
    for entity in &mut state.entities {
        if !entity.alive {
            continue;
        }
        if entity.cooldown_ticks > 0 {
            entity.cooldown_ticks -= 1;
        }

        let to_px = state.player.x - entity.x;
        let to_py = state.player.y - entity.y;
        let player_dist_sq = to_px * to_px + to_py * to_py;

        if entity.behavior == EntityBehavior::Chase
            && player_dist_sq > EPSILON
            && player_dist_sq <= 64.0
            && has_clear_los(
                &state.grid,
                &state.doors,
                entity.x,
                entity.y,
                state.player.x,
                state.player.y,
            )
        {
            entity.yaw = trig::atan2(to_py, to_px);
        }

        let ex_dir = trig::cos(entity.yaw);
        let ey_dir = trig::sin(entity.yaw);
        let move_dist = entity.speed * dt_sec;
        let old_x = entity.x;
        let old_y = entity.y;
        let (nx, ny) = move_circle_sliding(
            old_x,
            old_y,
            ex_dir * move_dist,
            ey_dir * move_dist,
            entity.radius,
            &state.grid,
            &state.doors,
        );
        entity.x = nx;
        entity.y = ny;

        // If patrol movement stalled against a wall/door, choose a deterministic new heading via RNG.
        let moved_sq = (nx - old_x) * (nx - old_x) + (ny - old_y) * (ny - old_y);
        if move_dist > 1.0e-5 && moved_sq < (move_dist * 0.25) * (move_dist * 0.25) {
            let quarter_turns = (state.rng.lrng_next_u32() % 3) + 1;
            entity.yaw = normalize_angle(entity.yaw + (quarter_turns as f64) * (trig::PI * 0.5));
        }

        let r_sum = state.player.radius + entity.radius;
        let cdx = state.player.x - entity.x;
        let cdy = state.player.y - entity.y;
        if cdx * cdx + cdy * cdy <= r_sum * r_sum && entity.cooldown_ticks == 0 {
            state.player.hp = state.player.hp.saturating_sub(entity.damage);
            *damage_taken = damage_taken.saturating_add(u32::from(entity.damage));
            entity.cooldown_ticks = 30;
            if state.player.hp == 0 {
                state.status = GameStatus::Lost;
            }
        }
    }

    // 6. Check victory upon reaching the exit tile center.
    if state.status == GameStatus::Playing {
        let ex = (state.exit_x as f64) + 0.5;
        let ey = (state.exit_y as f64) + 0.5;
        let win_reach = 0.45 + state.player.radius;
        let dx = state.player.x - ex;
        let dy = state.player.y - ey;
        if dx * dx + dy * dy <= win_reach * win_reach {
            state.status = GameStatus::Won;
        }
    }

    state.tick = state.tick.saturating_add(1);
}

/// Advance `state` deterministically in place using `game-clock` fixed-timestep
/// stepping, circle-vs-grid sliding collision, pickups, door unlocking, and AI.
///
/// Atomic on error: if any validation or clock check fails, `state` is left
/// completely unmodified. Steady-state execution performs **zero heap allocations**.
pub fn step_in_place(
    state: &mut LabyrinthState,
    input: LabyrinthInput,
) -> Result<StepOutcome, LabyrinthError> {
    state.state_validate()?;
    input.input_validate()?;

    let clock_step = game_clock::advance_frame(&state.clock, input.dt_ns)?;

    // Commit clock only after all validation & clock arithmetic succeeded.
    state.clock = clock_step.state;

    let dt_sec = (state.clock.fixed_dt_ns as f64) / (game_clock::NS_PER_SECOND as f64);
    let mut doors_opened = 0u32;
    let mut pickups_collected = 0u32;
    let mut damage_taken = 0u32;

    for _ in 0..clock_step.steps_to_run {
        if state.status != GameStatus::Playing {
            break;
        }
        step_single_tick(
            state,
            &input,
            dt_sec,
            &mut doors_opened,
            &mut pickups_collected,
            &mut damage_taken,
        );
    }

    Ok(StepOutcome {
        ticks_simulated: clock_step.steps_to_run,
        status: state.status,
        doors_opened,
        pickups_collected,
        damage_taken,
        state_digest: state_digest_fnv1a(state),
    })
}

/// Pure functional wrapper around [`step_in_place`], returning the next [`LabyrinthState`].
pub fn step(
    state: &LabyrinthState,
    input: LabyrinthInput,
) -> Result<(LabyrinthState, StepOutcome), LabyrinthError> {
    state.state_validate()?;
    input.input_validate()?;
    let mut next = state.clone();
    let outcome = step_in_place(&mut next, input)?;
    Ok((next, outcome))
}

// ---------------------------------------------------------------------------
// Canonical Digests (FNV-1a)
// ---------------------------------------------------------------------------

/// Compute a canonical 64-bit FNV-1a digest over framebuffer dimensions and RGBA8 pixels.
#[inline]
pub fn frame_digest_fnv1a(framebuffer: &PixelContainer) -> u64 {
    raster3d_soft::frame_digest_fnv1a(framebuffer)
}

/// Compute a canonical 64-bit FNV-1a digest over the full deterministic [`LabyrinthState`].
pub fn state_digest_fnv1a(state: &LabyrinthState) -> u64 {
    let mut h = FNV1A_OFFSET_BASIS;
    fnv1a_bytes(&mut h, &(state.grid.width as u64).to_le_bytes());
    fnv1a_bytes(&mut h, &(state.grid.height as u64).to_le_bytes());
    for tile in &state.grid.tiles {
        match *tile {
            TileKind::Floor => fnv1a_bytes(&mut h, &[0]),
            TileKind::Wall(id) => fnv1a_bytes(&mut h, &[1, id]),
            TileKind::Start => fnv1a_bytes(&mut h, &[2]),
            TileKind::Exit => fnv1a_bytes(&mut h, &[3]),
        }
    }
    fnv1a_f64(&mut h, state.player.x);
    fnv1a_f64(&mut h, state.player.y);
    fnv1a_f64(&mut h, state.player.yaw);
    fnv1a_f64(&mut h, state.player.radius);
    fnv1a_bytes(&mut h, &state.player.hp.to_le_bytes());
    fnv1a_bytes(&mut h, &state.player.max_hp.to_le_bytes());
    fnv1a_bytes(&mut h, &[state.player.key_mask]);
    fnv1a_bytes(&mut h, &state.player.score.to_le_bytes());

    fnv1a_bytes(&mut h, &(state.doors.len() as u64).to_le_bytes());
    for d in &state.doors {
        fnv1a_bytes(&mut h, &(d.x as u64).to_le_bytes());
        fnv1a_bytes(&mut h, &(d.y as u64).to_le_bytes());
        fnv1a_bytes(&mut h, &[u8::from(d.open)]);
    }

    fnv1a_bytes(&mut h, &(state.pickups.len() as u64).to_le_bytes());
    for p in &state.pickups {
        fnv1a_bytes(&mut h, &p.id.to_le_bytes());
        fnv1a_f64(&mut h, p.x);
        fnv1a_f64(&mut h, p.y);
        fnv1a_bytes(&mut h, &[u8::from(p.collected)]);
    }

    fnv1a_bytes(&mut h, &(state.entities.len() as u64).to_le_bytes());
    for e in &state.entities {
        fnv1a_bytes(&mut h, &e.id.to_le_bytes());
        fnv1a_f64(&mut h, e.x);
        fnv1a_f64(&mut h, e.y);
        fnv1a_f64(&mut h, e.yaw);
        fnv1a_bytes(&mut h, &e.hp.to_le_bytes());
        fnv1a_bytes(&mut h, &[u8::from(e.alive)]);
        fnv1a_bytes(&mut h, &e.cooldown_ticks.to_le_bytes());
    }

    fnv1a_bytes(&mut h, &state.rng.lrng_state_word().to_le_bytes());
    fnv1a_bytes(&mut h, &state.rng.lrng_step_count().to_le_bytes());
    fnv1a_bytes(&mut h, &state.tick.to_le_bytes());
    let status_byte = match state.status {
        GameStatus::Playing => 0u8,
        GameStatus::Won => 1u8,
        GameStatus::Lost => 2u8,
    };
    fnv1a_bytes(&mut h, &[status_byte]);
    h
}

// ---------------------------------------------------------------------------
// Dual Renderers: Raycast25D (raycast-dda) & Software3D (raster3d-soft)
// ---------------------------------------------------------------------------

fn validate_render_targets(
    state: &LabyrinthState,
    assets: &LabyrinthAssets,
    scratch: &LabyrinthRenderScratch,
    zbuffer: &[f64],
    framebuffer: &PixelContainer,
) -> Result<(usize, usize), LabyrinthError> {
    state.state_validate()?;
    assets.assets_validate()?;
    validate_frame_dim(framebuffer.width, framebuffer.height)?;

    let w = framebuffer.width as usize;
    let h = framebuffer.height as usize;
    let pixel_count = w.checked_mul(h).ok_or(LabyrinthError::ArithmeticOverflow)?;
    let expected_bytes = pixel_count
        .checked_mul(4)
        .ok_or(LabyrinthError::ArithmeticOverflow)?;
    if framebuffer.data.len() != expected_bytes {
        return Err(LabyrinthError::InvalidFramebuffer);
    }
    if zbuffer.len() < pixel_count {
        return Err(LabyrinthError::ZBufferTooSmall {
            required: pixel_count,
            actual: zbuffer.len(),
        });
    }

    let needed_sprites = state.pickups.len() + state.entities.len();
    if scratch.raycast_sprites.capacity() < needed_sprites
        || scratch.sprite_scratch.scratch_capacity() < needed_sprites
    {
        return Err(LabyrinthError::ScratchTooSmall {
            required: needed_sprites,
            actual: scratch.raycast_sprites.capacity(),
        });
    }

    // Validate every tile, door, pickup, and entity material reference upfront for atomicity.
    let tex_count = assets.textures.len();
    for tile in &state.grid.tiles {
        if let Some(wall_id) = tile.tile_wall_id() {
            if usize::from(wall_id - 1) >= tex_count {
                return Err(LabyrinthError::InvalidMaterialId {
                    id: u16::from(wall_id),
                });
            }
        }
    }
    for door in &state.doors {
        if usize::from(door.texture_id - 1) >= tex_count {
            return Err(LabyrinthError::InvalidMaterialId {
                id: door.texture_id,
            });
        }
    }
    for p in &state.pickups {
        if !p.collected && p.texture_index >= tex_count {
            return Err(LabyrinthError::InvalidMaterialId {
                id: u16::try_from(p.texture_index).map_or(u16::MAX, |v| v),
            });
        }
    }
    for e in &state.entities {
        if e.alive && e.texture_index >= tex_count {
            return Err(LabyrinthError::InvalidMaterialId {
                id: u16::try_from(e.texture_index).map_or(u16::MAX, |v| v),
            });
        }
    }

    Ok((w, h))
}

fn sync_raycast_grid_in_place(
    state: &LabyrinthState,
    scratch: &mut LabyrinthRenderScratch,
) -> Result<(), LabyrinthError> {
    let w = state.grid.width;
    let h = state.grid.height;
    let needs_new = match &scratch.cached_grid {
        Some(g) => g.grid_width() != w || g.grid_height() != h,
        None => true,
    };
    if needs_new {
        let len = w.checked_mul(h).ok_or(LabyrinthError::ArithmeticOverflow)?;
        scratch.cached_grid = Some(GridMap::grid_new(w, h, vec![0u16; len])?);
    }
    let Some(grid) = scratch.cached_grid.as_mut() else {
        return Err(LabyrinthError::InvalidGrid);
    };

    for y in 0..h {
        for x in 0..w {
            let val = match state.grid.lgrid_tile_at(x, y) {
                Some(TileKind::Wall(id)) => u16::from(id),
                _ => 0,
            };
            grid.grid_set_cell(x, y, val)?;
        }
    }
    for door in &state.doors {
        if !door.open {
            grid.grid_set_cell(door.x, door.y, door.texture_id)?;
        }
    }
    Ok(())
}

fn render_raycast_25d(
    state: &LabyrinthState,
    assets: &LabyrinthAssets,
    scratch: &mut LabyrinthRenderScratch,
    zbuffer: &mut [f64],
    framebuffer: &mut PixelContainer,
    width: usize,
    height: usize,
) -> Result<RenderReport, LabyrinthError> {
    let camera = Camera2D::camera_from_yaw_fov(
        state.player.x,
        state.player.y,
        state.player.yaw,
        assets.fov_rad,
    )?;

    sync_raycast_grid_in_place(state, scratch)?;
    let Some(grid) = scratch.cached_grid.as_ref() else {
        return Err(LabyrinthError::InvalidGrid);
    };

    scratch.raycast_sprites.clear();
    for p in &state.pickups {
        if !p.collected {
            scratch.raycast_sprites.push(Sprite::sprite_with_scale(
                p.x,
                p.y,
                p.texture_index,
                0.45,
                0.45,
                0.1,
            )?);
        }
    }
    for e in &state.entities {
        if e.alive {
            scratch.raycast_sprites.push(Sprite::sprite_with_scale(
                e.x,
                e.y,
                e.texture_index,
                0.7,
                0.85,
                0.0,
            )?);
        }
    }

    let cfg = RaycastRenderConfig::render_config_with_planes(
        PlaneSurface::Solid(assets.ceiling_color),
        PlaneSurface::Solid(assets.floor_color),
        148,
    );
    let col_zbuf = &mut zbuffer[..width];
    let stats = raycast_dda::render_frame_into(
        grid,
        &camera,
        &assets.textures,
        &scratch.raycast_sprites,
        &cfg,
        &mut scratch.sprite_scratch,
        col_zbuf,
        framebuffer,
    )?;

    let pixel_count = (width as u64).saturating_mul(height as u64);
    Ok(RenderReport {
        mode: RenderMode::Raycast25D,
        frame_digest: frame_digest_fnv1a(framebuffer),
        pixels_written: pixel_count,
        triangles_rasterized: 0,
        wall_hits: stats.wall_hits,
        sprites_projected: stats.sprites_projected,
    })
}

#[allow(clippy::too_many_arguments)]
fn push_3d_quad(
    verts: &mut Vec<RasterVertex>,
    indices: &mut Vec<u32>,
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
    p3: Vec3,
    normal: Vec3,
    color: RgbColor,
) {
    if verts.len() + 4 > verts.capacity() || indices.len() + 6 > indices.capacity() {
        return;
    }
    let base = verts.len() as u32;
    verts.push(RasterVertex::vertex_new(p0, normal, color, 0.0, 1.0));
    verts.push(RasterVertex::vertex_new(p1, normal, color, 1.0, 1.0));
    verts.push(RasterVertex::vertex_new(p2, normal, color, 1.0, 0.0));
    verts.push(RasterVertex::vertex_new(p3, normal, color, 0.0, 0.0));
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

#[allow(clippy::too_many_arguments)]
fn push_3d_pillar(
    verts: &mut Vec<RasterVertex>,
    indices: &mut Vec<u32>,
    cx: f64,
    cz: f64,
    half: f64,
    y_bot: f64,
    y_top: f64,
    color: RgbColor,
) {
    let x0 = cx - half;
    let x1 = cx + half;
    let z0 = cz - half;
    let z1 = cz + half;
    push_3d_quad(
        verts,
        indices,
        Vec3::new(x1, y_bot, z0),
        Vec3::new(x0, y_bot, z0),
        Vec3::new(x0, y_top, z0),
        Vec3::new(x1, y_top, z0),
        Vec3::new(0.0, 0.0, -1.0),
        color,
    );
    push_3d_quad(
        verts,
        indices,
        Vec3::new(x0, y_bot, z1),
        Vec3::new(x1, y_bot, z1),
        Vec3::new(x1, y_top, z1),
        Vec3::new(x0, y_top, z1),
        Vec3::new(0.0, 0.0, 1.0),
        color,
    );
    push_3d_quad(
        verts,
        indices,
        Vec3::new(x0, y_bot, z0),
        Vec3::new(x0, y_bot, z1),
        Vec3::new(x0, y_top, z1),
        Vec3::new(x0, y_top, z0),
        Vec3::new(-1.0, 0.0, 0.0),
        color,
    );
    push_3d_quad(
        verts,
        indices,
        Vec3::new(x1, y_bot, z1),
        Vec3::new(x1, y_bot, z0),
        Vec3::new(x1, y_top, z0),
        Vec3::new(x1, y_top, z1),
        Vec3::new(1.0, 0.0, 0.0),
        color,
    );
}

fn build_3d_scene_geometry(state: &LabyrinthState, scratch: &mut LabyrinthRenderScratch) -> usize {
    scratch.raster_vertices.clear();
    scratch.raster_indices.clear();

    let px_cell = (state.player.x.floor() as isize).clamp(0, (state.grid.width as isize) - 1);
    let py_cell = (state.player.y.floor() as isize).clamp(0, (state.grid.height as isize) - 1);
    let r = MAX_3D_RENDER_RADIUS as isize;
    let x_min = (px_cell - r).max(0) as usize;
    let y_min = (py_cell - r).max(0) as usize;
    let x_max = ((px_cell + r) as usize).min(state.grid.width - 1);
    let y_max = ((py_cell + r) as usize).min(state.grid.height - 1);

    let verts = &mut scratch.raster_vertices;
    let indices = &mut scratch.raster_indices;

    // 1. Floor & ceiling spans over the active local view window.
    let fx0 = x_min as f64;
    let fx1 = (x_max + 1) as f64;
    let fz0 = y_min as f64;
    let fz1 = (y_max + 1) as f64;
    push_3d_quad(
        verts,
        indices,
        Vec3::new(fx0, 0.0, fz1),
        Vec3::new(fx1, 0.0, fz1),
        Vec3::new(fx1, 0.0, fz0),
        Vec3::new(fx0, 0.0, fz0),
        Vec3::new(0.0, 1.0, 0.0),
        [0.27, 0.24, 0.20],
    );
    push_3d_quad(
        verts,
        indices,
        Vec3::new(fx0, 1.0, fz0),
        Vec3::new(fx1, 1.0, fz0),
        Vec3::new(fx1, 1.0, fz1),
        Vec3::new(fx0, 1.0, fz1),
        Vec3::new(0.0, -1.0, 0.0),
        [0.13, 0.16, 0.22],
    );

    // 2. Exposed wall faces and closed doors.
    for gy in y_min..=y_max {
        for gx in x_min..=x_max {
            let blocked = cell_blocked(&state.grid, &state.doors, gx, gy);
            if !blocked {
                if matches!(state.grid.lgrid_tile_at(gx, gy), Some(TileKind::Exit)) {
                    let cx = gx as f64 + 0.5;
                    let cz = gy as f64 + 0.5;
                    push_3d_pillar(verts, indices, cx, cz, 0.22, 0.0, 0.95, [0.18, 0.88, 0.52]);
                }
                continue;
            }
            let is_door = state
                .doors
                .iter()
                .any(|d| !d.open && d.x == gx && d.y == gy);
            let color: RgbColor = if is_door {
                [0.72, 0.52, 0.20]
            } else if matches!(state.grid.lgrid_tile_at(gx, gy), Some(TileKind::Wall(2))) {
                [0.58, 0.44, 0.30]
            } else {
                [0.48, 0.52, 0.58]
            };

            let x0 = gx as f64;
            let x1 = x0 + 1.0;
            let z0 = gy as f64;
            let z1 = z0 + 1.0;

            if gy == 0 || !cell_blocked(&state.grid, &state.doors, gx, gy - 1) {
                push_3d_quad(
                    verts,
                    indices,
                    Vec3::new(x1, 0.0, z0),
                    Vec3::new(x0, 0.0, z0),
                    Vec3::new(x0, 1.0, z0),
                    Vec3::new(x1, 1.0, z0),
                    Vec3::new(0.0, 0.0, -1.0),
                    color,
                );
            }
            if gy + 1 == state.grid.height || !cell_blocked(&state.grid, &state.doors, gx, gy + 1) {
                push_3d_quad(
                    verts,
                    indices,
                    Vec3::new(x0, 0.0, z1),
                    Vec3::new(x1, 0.0, z1),
                    Vec3::new(x1, 1.0, z1),
                    Vec3::new(x0, 1.0, z1),
                    Vec3::new(0.0, 0.0, 1.0),
                    color,
                );
            }
            if gx == 0 || !cell_blocked(&state.grid, &state.doors, gx - 1, gy) {
                push_3d_quad(
                    verts,
                    indices,
                    Vec3::new(x0, 0.0, z0),
                    Vec3::new(x0, 0.0, z1),
                    Vec3::new(x0, 1.0, z1),
                    Vec3::new(x0, 1.0, z0),
                    Vec3::new(-1.0, 0.0, 0.0),
                    color,
                );
            }
            if gx + 1 == state.grid.width || !cell_blocked(&state.grid, &state.doors, gx + 1, gy) {
                push_3d_quad(
                    verts,
                    indices,
                    Vec3::new(x1, 0.0, z1),
                    Vec3::new(x1, 0.0, z0),
                    Vec3::new(x1, 1.0, z0),
                    Vec3::new(x1, 1.0, z1),
                    Vec3::new(1.0, 0.0, 0.0),
                    color,
                );
            }
        }
    }

    // 3. Pickups and patrolling/chasing entities as 3D pillars.
    let mut actors_drawn = 0usize;
    for p in &state.pickups {
        if !p.collected {
            let color = match p.kind {
                PickupKind::Key(_) => [0.98, 0.84, 0.20],
                PickupKind::Health(_) => [0.25, 0.90, 0.45],
                PickupKind::Treasure(_) => [0.25, 0.85, 0.95],
            };
            push_3d_pillar(verts, indices, p.x, p.y, 0.14, 0.15, 0.55, color);
            actors_drawn += 1;
        }
    }
    for e in &state.entities {
        if e.alive {
            let color = match e.behavior {
                EntityBehavior::Patrol => [0.85, 0.35, 0.22],
                EntityBehavior::Chase => [0.92, 0.15, 0.25],
            };
            push_3d_pillar(verts, indices, e.x, e.y, e.radius * 0.8, 0.0, 0.85, color);
            actors_drawn += 1;
        }
    }
    actors_drawn
}

fn render_software_3d(
    state: &LabyrinthState,
    assets: &LabyrinthAssets,
    scratch: &mut LabyrinthRenderScratch,
    zbuffer: &mut [f64],
    framebuffer: &mut PixelContainer,
    width: usize,
    height: usize,
) -> Result<RenderReport, LabyrinthError> {
    let aspect = (width as f64) / (height as f64);
    let tan_half_x = trig::tan(0.5 * assets.fov_rad);
    let raw_fov_y = 2.0 * trig::atan(tan_half_x / aspect);
    let fov_y = raw_fov_y.clamp(0.05, trig::PI - 0.05);

    let eye = Vec3::new(state.player.x, 0.5, state.player.y);
    let forward = Vec3::new(
        trig::cos(state.player.yaw),
        0.0,
        trig::sin(state.player.yaw),
    );
    let target = eye.vec3_add(forward)?;
    let view = Mat4::look_at(eye, target, Vec3::unit_y())?;
    let proj = Mat4::perspective(fov_y, aspect, 0.05, 64.0)?;
    let mvp = proj.mul(&view)?;
    let mut cfg = RasterConfig::raster_config_new(mvp)?;
    cfg.shading = ShadingMode::Gouraud;
    cfg.cull = CullMode::Back;
    cfg.front_face = FrontFace::CounterClockwise;
    cfg.clip_space = ClipSpace::OpenGl;
    let _ = cfg.raster_config_validate()?;

    let pixel_count = width
        .checked_mul(height)
        .ok_or(LabyrinthError::ArithmeticOverflow)?;
    raster3d_soft::clear_framebuffer(framebuffer, assets.ceiling_color)?;
    raster3d_soft::clear_depth_f64(&mut zbuffer[..pixel_count], 1.0)?;

    let actors_drawn = build_3d_scene_geometry(state, scratch);
    let stats = raster3d_soft::draw_indexed_mesh_into(
        framebuffer,
        DepthBufferMut::F64(&mut zbuffer[..pixel_count]),
        &cfg,
        None,
        &scratch.raster_vertices,
        &scratch.raster_indices,
        &mut scratch.raster_scratch,
    )?;

    Ok(RenderReport {
        mode: RenderMode::Software3D,
        frame_digest: frame_digest_fnv1a(framebuffer),
        pixels_written: stats.pixels_written,
        triangles_rasterized: stats.triangles_rasterized,
        wall_hits: 0,
        sprites_projected: actors_drawn,
    })
}

/// Render the current dungeon view into caller-owned `framebuffer` and `zbuffer`
/// using either [`RenderMode::Raycast25D`] or [`RenderMode::Software3D`].
///
/// Atomic on error: validates all inputs, material IDs, Z-buffer length, and
/// scratch capacities before modifying `framebuffer`, `zbuffer`, or `scratch`.
/// Steady-state execution performs **zero heap allocations**.
pub fn render_frame_into(
    state: &LabyrinthState,
    assets: &LabyrinthAssets,
    mode: RenderMode,
    scratch: &mut LabyrinthRenderScratch,
    zbuffer: &mut [f64],
    framebuffer: &mut PixelContainer,
) -> Result<RenderReport, LabyrinthError> {
    let (width, height) = validate_render_targets(state, assets, scratch, zbuffer, framebuffer)?;
    match mode {
        RenderMode::Raycast25D => {
            render_raycast_25d(state, assets, scratch, zbuffer, framebuffer, width, height)
        }
        RenderMode::Software3D => {
            render_software_3d(state, assets, scratch, zbuffer, framebuffer, width, height)
        }
    }
}
