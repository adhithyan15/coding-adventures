//! # rpg-grid-world
//!
//! GAME23: a deterministic, zero-allocation overworld RPG engine core in the
//! spirit of the Game Boy / GBA era tile-based adventure games.
//!
//! * **Grid movement** ([`GridActor`]): a finite state machine
//!   ([`MoveState`]: `Idle`, `Walking`, `Turning`, `LedgeHop`, `Surfing`,
//!   `Frozen`) that walks an actor from tile to tile with sub-tile pixel
//!   interpolation. Collision comes from the `tilemap2d` (`GAME21`) tile
//!   attribute bits: *solid* blocks, *one-way* tiles are ledges that can be
//!   hopped in one direction, *water* needs surfing, *hazard* and *trigger*
//!   tiles are reported on arrival.
//! * **Step encounters** ([`EncounterTable`], [`EncounterState`]): walking on
//!   an encounter tile (tall grass, cave floor, water) accumulates steps; a
//!   seeded PRNG ([`GridRng`]) decides when a weighted table lookup fires.
//! * **Warps** ([`WarpPad`], [`resolve_warp`]): door and pad triggers that
//!   resolve to a target map id, tile and facing.
//! * **NPCs and signs** ([`Npc`], [`interact_target`], [`npc_sees_actor`]):
//!   blocking, interaction reach and trainer line of sight queries.
//! * **Dialogue** ([`DialogueBox`]): a paged typewriter text box with choice
//!   selection, lowered into `paint-batch-tiles` (`P2D09`) cells
//!   ([`lower_dialogue_cells_into`]) or a `paint-instructions` (`P2D00`)
//!   scene ([`lower_dialogue_to_paint_scene`]).
//!
//! ## Conventions
//!
//! * Tile coordinates are `i32` but every one is validated against the map
//!   (`0 <= x < cols`); movement never leaves the map, regardless of the
//!   layer's wrap mode.
//! * Sub-tile motion is measured in pixels along the movement axis; a walk
//!   covers one tile (`tile_w` or `tile_h` pixels), a ledge hop covers two.
//! * Dialogue text is printable ASCII (`0x20..=0x7E`), which makes byte
//!   slicing for the typewriter reveal always valid.
//!
//! ## Safety and resource model
//!
//! * `#![forbid(unsafe_code)]`; no pointer arithmetic.
//! * Movement ticks, encounter steps, NPC queries and dialogue ticks never
//!   touch the heap. The only allocating function is
//!   [`lower_dialogue_to_paint_scene`], which builds an owned scene.
//! * Warp, NPC and encounter tables, pages, lines, choices, speeds and
//!   interaction ranges are all bounded (CWE-20 / CWE-400); every coordinate
//!   computation is done in checked or widened integers (CWE-190); divisors
//!   are validated non-zero (CWE-369); buffers and indices are validated
//!   before any read or write (CWE-125 / CWE-787).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Generated capability verification witnesses.
pub mod capabilities;

pub use paint_batch_tiles::{PaintBase, PaintInstruction, PaintRect, PaintScene, TileBatchCell};
pub use tilemap2d::{
    TileCell, TileLayer, ATTR_HAZARD, ATTR_ONE_WAY, ATTR_SOLID, ATTR_TRIGGER, ATTR_WATER,
};

use paint_batch_tiles::PaintBatchError;
use tilemap2d::{attr_at_tile, cell_at_tile};

/// An RGB8 colour: `[red, green, blue]`.
pub type Rgb8 = [u8; 3];

/// Maximum number of warp pads in one [`GridWorld`].
pub const MAX_WARPS: usize = 256;
/// Maximum number of NPCs / interactables in one [`GridWorld`].
pub const MAX_NPCS: usize = 256;
/// Maximum number of encounter tables in one [`GridWorld`].
pub const MAX_ENCOUNTER_TABLES: usize = 8;
/// Maximum number of weighted entries in one [`EncounterTable`].
pub const MAX_ENCOUNTER_ENTRIES: usize = 32;
/// Maximum interaction reach, in tiles, of [`interact_target`].
pub const MAX_INTERACT_RANGE: u8 = 4;
/// Maximum NPC line-of-sight range, in tiles.
pub const MAX_SIGHT_RANGE: u8 = 16;
/// Maximum walking speed in pixels per tick.
pub const MAX_SPEED: u32 = 256;
/// Maximum number of ticks a [`MoveState::Turning`] pause may last.
pub const MAX_TURN_TICKS: u32 = 64;
/// Fixed-point one for encounter rates: `rate_q8 = 256` means every step.
pub const RATE_ONE: u16 = 256;
/// Largest valid warp target coordinate (exclusive); mirrors `tilemap2d`'s map limit.
pub const MAX_TARGET_COORD: i32 = 4096;
/// Maximum number of pages in one dialogue.
pub const MAX_PAGES: usize = 64;
/// Maximum number of text lines in one dialogue page.
pub const MAX_LINES: usize = 4;
/// Maximum number of bytes in one dialogue line or choice label.
pub const MAX_LINE_LEN: usize = 32;
/// Maximum number of choices on one dialogue page.
pub const MAX_CHOICES: usize = 4;
/// Maximum typewriter speed in Q8 characters per tick (`256` is one per tick).
pub const MAX_TEXT_SPEED_Q8: u32 = 256 * 128;
/// Default typewriter speed in Q8 characters per tick (half a character).
pub const DEFAULT_TEXT_SPEED_Q8: u32 = 128;
/// Maximum number of tile cells in one lowered dialogue box.
pub const MAX_DIALOGUE_CELLS: usize = 4096;
/// Maximum tile edge, in pixels, accepted by the scene lowering.
pub const MAX_TILE_PX: u32 = 256;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Failure modes of every fallible operation in this crate.
#[derive(Clone, Debug, PartialEq)]
pub enum GridError {
    /// A tile coordinate lies outside the map.
    OutOfBounds {
        /// Offending tile column.
        x: i64,
        /// Offending tile row.
        y: i64,
    },
    /// A table holds more entries than its bound.
    TooMany {
        /// Which table.
        what: &'static str,
        /// Entries supplied.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// A numeric parameter is outside its valid range.
    InvalidParameter {
        /// Which parameter.
        what: &'static str,
        /// The rejected value.
        value: i64,
    },
    /// An encounter table failed validation.
    InvalidEncounterTable {
        /// Index of the table in the world.
        index: usize,
    },
    /// A dialogue page failed validation.
    InvalidDialogue {
        /// Index of the page.
        page: usize,
        /// Why it was rejected.
        reason: &'static str,
    },
    /// An internal invariant was violated by a hand-built value.
    InvalidState {
        /// What was inconsistent.
        what: &'static str,
    },
    /// The caller buffer is too small; `required` is the exact count needed.
    BufferTooSmall {
        /// Cells required.
        required: usize,
        /// Cells available.
        actual: usize,
    },
    /// A size or id computation overflowed.
    SizeOverflow,
    /// The `paint-batch-tiles` lowering failed.
    Batch(PaintBatchError),
}

impl std::fmt::Display for GridError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfBounds { x, y } => write!(f, "tile ({x}, {y}) is outside the map"),
            Self::TooMany { what, count, max } => {
                write!(f, "{what}: {count} entries exceed the maximum of {max}")
            }
            Self::InvalidParameter { what, value } => {
                write!(f, "invalid {what}: {value}")
            }
            Self::InvalidEncounterTable { index } => {
                write!(f, "encounter table {index} is invalid")
            }
            Self::InvalidDialogue { page, reason } => {
                write!(f, "dialogue page {page} is invalid: {reason}")
            }
            Self::InvalidState { what } => write!(f, "inconsistent state: {what}"),
            Self::BufferTooSmall { required, actual } => {
                write!(f, "buffer too small: need {required} cells, have {actual}")
            }
            Self::SizeOverflow => write!(f, "size computation overflowed"),
            Self::Batch(e) => write!(f, "tile batch lowering failed: {e:?}"),
        }
    }
}

impl std::error::Error for GridError {}

// ---------------------------------------------------------------------------
// Facing
// ---------------------------------------------------------------------------

/// The four grid directions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Facing {
    /// Towards decreasing rows.
    Up,
    /// Towards increasing rows.
    Down,
    /// Towards decreasing columns.
    Left,
    /// Towards increasing columns.
    Right,
}

impl Facing {
    /// The unit step `(dx, dy)` of this direction.
    pub const fn delta_xy(self) -> (i32, i32) {
        match self {
            Self::Up => (0, -1),
            Self::Down => (0, 1),
            Self::Left => (-1, 0),
            Self::Right => (1, 0),
        }
    }

    /// The opposite direction.
    pub const fn reversed(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

/// True when tile `(x, y)` lies inside the layer's `cols x rows` grid.
pub fn tile_in_bounds(layer: &TileLayer<'_>, x: i64, y: i64) -> bool {
    x >= 0 && y >= 0 && x < i64::from(layer.layer_cols()) && y < i64::from(layer.layer_rows())
}

// ---------------------------------------------------------------------------
// PRNG
// ---------------------------------------------------------------------------

/// A tiny deterministic xorshift64* generator used for encounter rolls.
///
/// The state is never zero, so the sequence never collapses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridRng {
    state: u64,
}

const RNG_FALLBACK_SEED: u64 = 0x9E37_79B9_7F4A_7C15;
const RNG_MULTIPLIER: u64 = 0x2545_F491_4F6C_DD1D;

impl GridRng {
    /// Seeds the generator. A zero seed is replaced by a fixed constant.
    pub const fn seeded(seed: u64) -> Self {
        Self {
            state: if seed == 0 { RNG_FALLBACK_SEED } else { seed },
        }
    }

    /// Next 32 random bits.
    pub fn next_roll(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        (x.wrapping_mul(RNG_MULTIPLIER) >> 32) as u32
    }

    /// A uniform-ish value in `0..bound` (`0` when `bound == 0`). Uses a
    /// multiply-shift reduction: no division, so no divide-by-zero (CWE-369).
    pub fn roll_below(&mut self, bound: u32) -> u32 {
        let r = u64::from(self.next_roll());
        ((r * u64::from(bound)) >> 32) as u32
    }
}

// ---------------------------------------------------------------------------
// Warps
// ---------------------------------------------------------------------------

/// What makes a [`WarpPad`] fire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WarpTrigger {
    /// The actor arrives on the pad's tile (pads, stairs, cave mouths).
    OnEnter,
    /// The actor, standing on the pad's tile, tries to walk in this
    /// direction and is blocked (a door set in a wall or the map edge).
    OnBump(Facing),
}

/// A door or pad: stepping on / bumping through it moves the player to
/// another map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WarpPad {
    /// Pad tile column.
    pub x: i32,
    /// Pad tile row.
    pub y: i32,
    /// What fires the warp.
    pub trigger: WarpTrigger,
    /// Id of the destination map.
    pub target_map: u16,
    /// Destination tile column.
    pub target_x: i32,
    /// Destination tile row.
    pub target_y: i32,
    /// Facing after the warp.
    pub target_facing: Facing,
}

/// The resolved destination of a warp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WarpTarget {
    /// Id of the destination map.
    pub map: u16,
    /// Destination tile column.
    pub x: i32,
    /// Destination tile row.
    pub y: i32,
    /// Facing after the warp.
    pub facing: Facing,
}

/// Finds the first warp at `(x, y)` that matches `event`.
pub fn resolve_warp(warps: &[WarpPad], x: i32, y: i32, event: WarpTrigger) -> Option<WarpTarget> {
    warps
        .iter()
        .find(|w| w.x == x && w.y == y && w.trigger == event)
        .map(|w| WarpTarget {
            map: w.target_map,
            x: w.target_x,
            y: w.target_y,
            facing: w.target_facing,
        })
}

/// Validates one warp pad against the map it sits on.
///
/// # Errors
///
/// [`GridError::OutOfBounds`] for a pad outside the map or a target outside
/// `0..MAX_TARGET_COORD`.
pub fn validate_warp(layer: &TileLayer<'_>, warp: &WarpPad) -> Result<(), GridError> {
    if !tile_in_bounds(layer, i64::from(warp.x), i64::from(warp.y)) {
        return Err(GridError::OutOfBounds {
            x: i64::from(warp.x),
            y: i64::from(warp.y),
        });
    }
    if !(0..MAX_TARGET_COORD).contains(&warp.target_x)
        || !(0..MAX_TARGET_COORD).contains(&warp.target_y)
    {
        return Err(GridError::OutOfBounds {
            x: i64::from(warp.target_x),
            y: i64::from(warp.target_y),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// NPCs
// ---------------------------------------------------------------------------

/// What an [`Npc`] entry represents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcKind {
    /// A townsperson: turns to face the player when spoken to.
    Person,
    /// A sign or object: never turns.
    Sign,
    /// A trainer: challenges the player on line of sight.
    Trainer,
}

/// A non-player character or interactable object on the grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Npc {
    /// Caller-defined id.
    pub id: u16,
    /// Tile column.
    pub x: i32,
    /// Tile row.
    pub y: i32,
    /// Facing direction.
    pub facing: Facing,
    /// What the entry represents.
    pub kind: NpcKind,
    /// Dialogue script id triggered by interaction.
    pub script: u16,
    /// Line-of-sight range in tiles (`0` = never sees).
    pub sight_range: u8,
    /// True when the NPC occupies its tile.
    pub blocking: bool,
}

/// Index of the first NPC standing on `(x, y)`.
pub fn find_npc_at(npcs: &[Npc], x: i32, y: i32) -> Option<usize> {
    npcs.iter().position(|n| n.x == x && n.y == y)
}

/// True when a blocking NPC occupies `(x, y)`.
pub fn npc_blocks_tile(npcs: &[Npc], x: i64, y: i64) -> bool {
    npcs.iter()
        .any(|n| n.blocking && i64::from(n.x) == x && i64::from(n.y) == y)
}

/// The NPC an actor at `(x, y)` facing `facing` can talk to: the first NPC
/// within `reach` tiles straight ahead. Solid tiles do not interrupt the
/// scan, so shop counters work. Tiles outside the map end it.
///
/// # Errors
///
/// [`GridError::InvalidParameter`] when `reach` is `0` or above
/// [`MAX_INTERACT_RANGE`].
pub fn interact_target(
    layer: &TileLayer<'_>,
    npcs: &[Npc],
    x: i32,
    y: i32,
    facing: Facing,
    reach: u8,
) -> Result<Option<usize>, GridError> {
    if reach == 0 || reach > MAX_INTERACT_RANGE {
        return Err(GridError::InvalidParameter {
            what: "interaction reach",
            value: i64::from(reach),
        });
    }
    let (dx, dy) = facing.delta_xy();
    for d in 1..=i64::from(reach) {
        let tx = i64::from(x) + i64::from(dx) * d;
        let ty = i64::from(y) + i64::from(dy) * d;
        if !tile_in_bounds(layer, tx, ty) {
            break;
        }
        if let Some(i) = npcs
            .iter()
            .position(|n| i64::from(n.x) == tx && i64::from(n.y) == ty)
        {
            return Ok(Some(i));
        }
    }
    Ok(None)
}

/// True when `npc` can see an actor at `(x, y)`: the actor lies straight
/// ahead of the NPC within its sight range with no solid tile in between.
pub fn npc_sees_actor(layer: &TileLayer<'_>, npc: &Npc, x: i32, y: i32) -> bool {
    let range = npc.sight_range.min(MAX_SIGHT_RANGE);
    let (dx, dy) = npc.facing.delta_xy();
    for d in 1..=i64::from(range) {
        let tx = i64::from(npc.x) + i64::from(dx) * d;
        let ty = i64::from(npc.y) + i64::from(dy) * d;
        if !tile_in_bounds(layer, tx, ty) {
            return false;
        }
        if tx == i64::from(x) && ty == i64::from(y) {
            return true;
        }
        if attr_at_tile(layer, tx, ty) & ATTR_SOLID != 0 {
            return false;
        }
    }
    false
}

/// Turns a [`NpcKind::Person`] or [`NpcKind::Trainer`] to face an actor who
/// is facing `actor_facing` (signs never turn). Returns true when it turned.
pub fn face_npc_toward(npc: &mut Npc, actor_facing: Facing) -> bool {
    if npc.kind == NpcKind::Sign {
        return false;
    }
    let want = actor_facing.reversed();
    let changed = npc.facing != want;
    npc.facing = want;
    changed
}

// ---------------------------------------------------------------------------
// Encounters
// ---------------------------------------------------------------------------

/// One weighted row of an [`EncounterTable`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncounterEntry {
    /// Caller-defined creature id.
    pub species: u16,
    /// Lowest level that can spawn (>= 1).
    pub min_level: u8,
    /// Highest level that can spawn (>= `min_level`).
    pub max_level: u8,
    /// Relative weight (> 0).
    pub weight: u16,
}

/// An encounter table bound to one tile id (tall grass, cave floor, water).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncounterTable<'a> {
    /// The tile id whose steps count towards this table (non-zero).
    pub tile_id: u16,
    /// Steps that must accumulate before an encounter can fire.
    pub min_steps: u16,
    /// Steps at which an encounter is guaranteed (>= 1 and >= `min_steps`).
    pub max_steps: u16,
    /// Per-step chance once `min_steps` passed, Q8 (`0..=256`).
    pub rate_q8: u16,
    /// The weighted creature rows.
    pub entries: &'a [EncounterEntry],
}

/// The outcome of a fired encounter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncounterRoll {
    /// The creature id.
    pub species: u16,
    /// The creature level.
    pub level: u8,
}

/// Validates an encounter table.
///
/// # Errors
///
/// [`GridError::InvalidEncounterTable`] (`index` is reported back) or
/// [`GridError::TooMany`].
pub fn validate_encounter_table(index: usize, table: &EncounterTable<'_>) -> Result<(), GridError> {
    if table.entries.len() > MAX_ENCOUNTER_ENTRIES {
        return Err(GridError::TooMany {
            what: "encounter entries",
            count: table.entries.len(),
            max: MAX_ENCOUNTER_ENTRIES,
        });
    }
    let bad = GridError::InvalidEncounterTable { index };
    if table.tile_id == 0
        || table.entries.is_empty()
        || table.max_steps == 0
        || table.max_steps < table.min_steps
        || table.rate_q8 > RATE_ONE
    {
        return Err(bad);
    }
    for e in table.entries {
        if e.weight == 0 || e.min_level == 0 || e.max_level < e.min_level {
            return Err(bad);
        }
    }
    Ok(())
}

/// Step counter plus PRNG: the deterministic encounter state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncounterState {
    /// Qualifying steps since the last encounter.
    pub steps: u16,
    rng: GridRng,
}

impl EncounterState {
    /// A fresh counter seeded with `seed`.
    pub const fn start(seed: u64) -> Self {
        Self {
            steps: 0,
            rng: GridRng::seeded(seed),
        }
    }

    /// Registers one qualifying step on `table`. Returns the rolled
    /// encounter when one fires (and resets the counter).
    pub fn register_step(&mut self, table: &EncounterTable<'_>) -> Option<EncounterRoll> {
        self.steps = self.steps.saturating_add(1);
        if self.steps < table.min_steps {
            return None;
        }
        let fire = self.steps >= table.max_steps
            || self.rng.roll_below(u32::from(RATE_ONE)) < u32::from(table.rate_q8);
        if !fire {
            return None;
        }
        self.steps = 0;
        pick_entry(&mut self.rng, table)
    }
}

fn pick_entry(rng: &mut GridRng, table: &EncounterTable<'_>) -> Option<EncounterRoll> {
    let total: u32 = table.entries.iter().map(|e| u32::from(e.weight)).sum();
    if total == 0 {
        return None;
    }
    let mut pick = rng.roll_below(total);
    for e in table.entries {
        let w = u32::from(e.weight);
        if pick < w {
            let span = u32::from(e.max_level.saturating_sub(e.min_level)) + 1;
            let level = u32::from(e.min_level) + rng.roll_below(span);
            return Some(EncounterRoll {
                species: e.species,
                level: u8::try_from(level).unwrap_or(e.max_level),
            });
        }
        pick -= w;
    }
    None
}

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

/// Everything a [`GridActor`] collides with and triggers: the tile layer,
/// warps, NPCs and encounter tables.
#[derive(Clone, Copy, Debug)]
pub struct GridWorld<'a> {
    /// The collision / appearance layer.
    pub layer: &'a TileLayer<'a>,
    /// Warp pads.
    pub warps: &'a [WarpPad],
    /// NPCs and interactables.
    pub npcs: &'a [Npc],
    /// Encounter tables.
    pub encounters: &'a [EncounterTable<'a>],
    /// The only direction in which one-way (ledge) tiles can be hopped.
    pub ledge_facing: Facing,
}

impl<'a> GridWorld<'a> {
    /// Validates and bundles a world. Ledges hop [`Facing::Down`] by default.
    ///
    /// # Errors
    ///
    /// [`GridError::TooMany`], [`GridError::OutOfBounds`] or
    /// [`GridError::InvalidEncounterTable`].
    pub fn bind(
        layer: &'a TileLayer<'a>,
        warps: &'a [WarpPad],
        npcs: &'a [Npc],
        encounters: &'a [EncounterTable<'a>],
    ) -> Result<Self, GridError> {
        if warps.len() > MAX_WARPS {
            return Err(GridError::TooMany {
                what: "warps",
                count: warps.len(),
                max: MAX_WARPS,
            });
        }
        if npcs.len() > MAX_NPCS {
            return Err(GridError::TooMany {
                what: "npcs",
                count: npcs.len(),
                max: MAX_NPCS,
            });
        }
        if encounters.len() > MAX_ENCOUNTER_TABLES {
            return Err(GridError::TooMany {
                what: "encounter tables",
                count: encounters.len(),
                max: MAX_ENCOUNTER_TABLES,
            });
        }
        for w in warps {
            validate_warp(layer, w)?;
        }
        for n in npcs {
            if !tile_in_bounds(layer, i64::from(n.x), i64::from(n.y)) {
                return Err(GridError::OutOfBounds {
                    x: i64::from(n.x),
                    y: i64::from(n.y),
                });
            }
            if n.sight_range > MAX_SIGHT_RANGE {
                return Err(GridError::InvalidParameter {
                    what: "sight range",
                    value: i64::from(n.sight_range),
                });
            }
        }
        for (i, t) in encounters.iter().enumerate() {
            validate_encounter_table(i, t)?;
        }
        Ok(Self {
            layer,
            warps,
            npcs,
            encounters,
            ledge_facing: Facing::Down,
        })
    }

    /// The encounter table bound to the tile at `(x, y)`, if any.
    pub fn table_for_tile(&self, x: i32, y: i32) -> Option<&'a EncounterTable<'a>> {
        if !tile_in_bounds(self.layer, i64::from(x), i64::from(y)) {
            return None;
        }
        let cell = cell_at_tile(self.layer, i64::from(x), i64::from(y))?;
        self.encounters.iter().find(|t| t.tile_id == cell.tile_id)
    }
}

// ---------------------------------------------------------------------------
// Movement
// ---------------------------------------------------------------------------

/// The movement state of a [`GridActor`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveState {
    /// Standing still on land.
    Idle,
    /// Interpolating towards the next tile.
    Walking,
    /// Pausing to face a new direction.
    Turning,
    /// Hopping two tiles over a ledge.
    LedgeHop,
    /// Standing still on water.
    Surfing,
    /// Input is ignored (cutscene, dialogue, battle).
    Frozen,
}

/// What happened during one [`GridActor::advance`] tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TickReport {
    /// A new walk or hop began.
    pub started: bool,
    /// The actor began or continued a turn.
    pub turned: bool,
    /// The actor completed a move onto a tile.
    pub arrived: bool,
    /// The completed move was a ledge hop.
    pub landed: bool,
    /// The move was blocked (wall, edge, NPC, wrong-way ledge, water).
    pub bumped: bool,
    /// The actor started surfing this tick.
    pub mounted: bool,
    /// The actor left the water this tick.
    pub dismounted: bool,
    /// The arrival tile has the hazard attribute.
    pub hazard: bool,
    /// The arrival tile has the trigger attribute.
    pub trigger: bool,
    /// A warp fired.
    pub warp: Option<WarpTarget>,
    /// An encounter fired.
    pub encounter: Option<EncounterRoll>,
}

/// A grid-bound actor (the player or a walking NPC).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridActor {
    /// Current tile column (the origin tile while moving).
    pub tile_x: i32,
    /// Current tile row (the origin tile while moving).
    pub tile_y: i32,
    /// Facing direction.
    pub facing: Facing,
    /// Movement state.
    pub state: MoveState,
    /// Walking speed in pixels per tick (`1..=MAX_SPEED`).
    pub speed: u32,
    /// Ticks spent in [`MoveState::Turning`] (`0` turns instantly).
    pub turn_ticks: u32,
    /// True when stepping onto water may start surfing.
    pub can_surf: bool,
    /// True while surfing.
    pub surfing: bool,
    progress: u32,
    distance: u32,
    dest_x: i32,
    dest_y: i32,
    turn_left: u32,
    locked: bool,
}

impl GridActor {
    /// Places an idle actor on tile `(x, y)`.
    ///
    /// # Errors
    ///
    /// [`GridError::OutOfBounds`] when the tile is outside the map.
    pub fn place(layer: &TileLayer<'_>, x: i32, y: i32, facing: Facing) -> Result<Self, GridError> {
        if !tile_in_bounds(layer, i64::from(x), i64::from(y)) {
            return Err(GridError::OutOfBounds {
                x: i64::from(x),
                y: i64::from(y),
            });
        }
        Ok(Self {
            tile_x: x,
            tile_y: y,
            facing,
            state: MoveState::Idle,
            speed: 1,
            turn_ticks: 0,
            can_surf: false,
            surfing: false,
            progress: 0,
            distance: 0,
            dest_x: x,
            dest_y: y,
            turn_left: 0,
            locked: false,
        })
    }

    /// Sets the walking speed in pixels per tick.
    ///
    /// # Errors
    ///
    /// [`GridError::InvalidParameter`] outside `1..=MAX_SPEED`.
    pub fn set_speed(&mut self, speed: u32) -> Result<(), GridError> {
        if speed == 0 || speed > MAX_SPEED {
            return Err(GridError::InvalidParameter {
                what: "speed",
                value: i64::from(speed),
            });
        }
        self.speed = speed;
        Ok(())
    }

    /// Sets how many ticks a turn pauses for.
    ///
    /// # Errors
    ///
    /// [`GridError::InvalidParameter`] above [`MAX_TURN_TICKS`].
    pub fn set_turn_delay(&mut self, ticks: u32) -> Result<(), GridError> {
        if ticks > MAX_TURN_TICKS {
            return Err(GridError::InvalidParameter {
                what: "turn ticks",
                value: i64::from(ticks),
            });
        }
        self.turn_ticks = ticks;
        Ok(())
    }

    /// Pixels travelled so far in the current walk or hop.
    pub const fn walk_progress(&self) -> u32 {
        self.progress
    }

    /// Total pixels of the current walk or hop (`0` when not moving).
    pub const fn walk_distance(&self) -> u32 {
        self.distance
    }

    /// The tile the current walk or hop is heading to.
    pub const fn walk_dest(&self) -> (i32, i32) {
        (self.dest_x, self.dest_y)
    }

    fn resting_state(&self) -> MoveState {
        if self.surfing {
            MoveState::Surfing
        } else {
            MoveState::Idle
        }
    }

    /// Freezes the actor: immediately when at rest, after arrival when
    /// walking.
    pub fn freeze_actor(&mut self) {
        self.locked = true;
        if !matches!(self.state, MoveState::Walking | MoveState::LedgeHop) {
            self.state = MoveState::Frozen;
            self.turn_left = 0;
        }
    }

    /// Lifts a freeze.
    pub fn thaw_actor(&mut self) {
        self.locked = false;
        if self.state == MoveState::Frozen {
            self.state = self.resting_state();
        }
    }

    /// The actor's layer-space pixel position, including the sub-tile
    /// interpolation of an in-flight walk or hop.
    pub fn pixel_position(&self, layer: &TileLayer<'_>) -> (i64, i64) {
        let mut x = i64::from(self.tile_x) * i64::from(layer.layer_tile_w());
        let mut y = i64::from(self.tile_y) * i64::from(layer.layer_tile_h());
        if matches!(self.state, MoveState::Walking | MoveState::LedgeHop) {
            let (dx, dy) = self.facing.delta_xy();
            x += i64::from(dx) * i64::from(self.progress);
            y += i64::from(dy) * i64::from(self.progress);
        }
        (x, y)
    }

    /// The upward pixel offset of a ledge hop's parabolic arc (`0` when not
    /// hopping): `peak` at the midpoint, `0` at both ends.
    pub fn hop_arc(&self, peak: u32) -> i64 {
        if self.state != MoveState::LedgeHop || self.distance == 0 {
            return 0;
        }
        let p = i64::from(self.progress);
        let d = i64::from(self.distance);
        4 * i64::from(peak) * p * (d - p) / (d * d)
    }

    /// Advances the actor by one tick with the held direction `input`.
    ///
    /// # Errors
    ///
    /// Currently infallible for validated worlds; reserved for hostile
    /// hand-built state ([`GridError::InvalidState`]).
    pub fn advance(
        &mut self,
        world: &GridWorld<'_>,
        input: Option<Facing>,
        enc: &mut EncounterState,
    ) -> Result<TickReport, GridError> {
        let mut report = TickReport::default();
        match self.state {
            MoveState::Frozen => return Ok(report),
            MoveState::Walking | MoveState::LedgeHop => {
                if self.distance == 0 {
                    return Err(GridError::InvalidState {
                        what: "moving actor has zero distance",
                    });
                }
                self.progress = self.progress.saturating_add(self.speed).min(self.distance);
                if self.progress < self.distance {
                    return Ok(report);
                }
                self.arrive_at_dest(world, enc, &mut report);
                if self.state == MoveState::Frozen {
                    return Ok(report);
                }
            }
            MoveState::Turning => {
                self.turn_left = self.turn_left.saturating_sub(1);
                if self.turn_left > 0 {
                    report.turned = true;
                    return Ok(report);
                }
                self.state = self.resting_state();
            }
            MoveState::Idle | MoveState::Surfing => {}
        }
        let Some(want) = input else {
            return Ok(report);
        };
        if want != self.facing {
            self.facing = want;
            report.turned = true;
            if self.turn_ticks > 0 {
                self.state = MoveState::Turning;
                self.turn_left = self.turn_ticks;
                return Ok(report);
            }
        }
        self.begin_move(world, want, &mut report);
        Ok(report)
    }

    fn arrive_at_dest(
        &mut self,
        world: &GridWorld<'_>,
        enc: &mut EncounterState,
        report: &mut TickReport,
    ) {
        let hopped = self.state == MoveState::LedgeHop;
        self.tile_x = self.dest_x;
        self.tile_y = self.dest_y;
        self.progress = 0;
        self.distance = 0;
        report.arrived = true;
        report.landed = hopped;
        let attr = attr_at_tile(world.layer, i64::from(self.tile_x), i64::from(self.tile_y));
        if self.surfing && attr & ATTR_WATER == 0 {
            self.surfing = false;
            report.dismounted = true;
        }
        report.hazard = attr & ATTR_HAZARD != 0;
        report.trigger = attr & ATTR_TRIGGER != 0;
        self.state = if self.locked {
            MoveState::Frozen
        } else {
            self.resting_state()
        };
        report.warp = resolve_warp(world.warps, self.tile_x, self.tile_y, WarpTrigger::OnEnter);
        if report.warp.is_none() && !hopped {
            if let Some(table) = world.table_for_tile(self.tile_x, self.tile_y) {
                report.encounter = enc.register_step(table);
            }
        }
    }

    fn begin_move(&mut self, world: &GridWorld<'_>, dir: Facing, report: &mut TickReport) {
        let (dx, dy) = dir.delta_xy();
        let (ox, oy) = (i64::from(self.tile_x), i64::from(self.tile_y));
        let (x1, y1) = (ox + i64::from(dx), oy + i64::from(dy));
        let layer = world.layer;
        let mut hop = false;
        let mut mount = false;
        let mut open = tile_in_bounds(layer, x1, y1);
        let (mut tx, mut ty) = (x1, y1);
        if open {
            let attr = attr_at_tile(layer, x1, y1);
            if attr & ATTR_SOLID != 0 || npc_blocks_tile(world.npcs, x1, y1) {
                open = false;
            } else if attr & ATTR_ONE_WAY != 0 {
                let (x2, y2) = (x1 + i64::from(dx), y1 + i64::from(dy));
                let landing_ok = dir == world.ledge_facing
                    && !self.surfing
                    && tile_in_bounds(layer, x2, y2)
                    && attr_at_tile(layer, x2, y2) & (ATTR_SOLID | ATTR_ONE_WAY | ATTR_WATER) == 0
                    && !npc_blocks_tile(world.npcs, x2, y2);
                if landing_ok {
                    hop = true;
                    tx = x2;
                    ty = y2;
                } else {
                    open = false;
                }
            } else if attr & ATTR_WATER != 0 && !self.surfing {
                if self.can_surf {
                    mount = true;
                } else {
                    open = false;
                }
            }
        }
        if !open {
            report.bumped = true;
            report.warp = resolve_warp(
                world.warps,
                self.tile_x,
                self.tile_y,
                WarpTrigger::OnBump(dir),
            );
            return;
        }
        let (Ok(dest_x), Ok(dest_y)) = (i32::try_from(tx), i32::try_from(ty)) else {
            report.bumped = true;
            return;
        };
        let tile_px = if dx != 0 {
            layer.layer_tile_w()
        } else {
            layer.layer_tile_h()
        };
        self.distance = if hop {
            tile_px.saturating_mul(2)
        } else {
            tile_px
        };
        self.dest_x = dest_x;
        self.dest_y = dest_y;
        self.progress = 0;
        self.state = if hop {
            MoveState::LedgeHop
        } else {
            MoveState::Walking
        };
        if mount {
            self.surfing = true;
            report.mounted = true;
        }
        report.started = true;
    }
}

// ---------------------------------------------------------------------------
// Dialogue
// ---------------------------------------------------------------------------

/// One page of a dialogue: up to [`MAX_LINES`] text lines, optionally
/// followed by up to [`MAX_CHOICES`] choices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DialoguePage<'a> {
    /// Text lines (printable ASCII, <= [`MAX_LINE_LEN`] bytes each).
    pub lines: &'a [&'a str],
    /// Choice labels; empty for a plain page.
    pub choices: &'a [&'a str],
    /// Page index each choice jumps to; empty means "fall through to the
    /// next page", otherwise one entry per choice.
    pub choice_targets: &'a [u16],
}

/// The state of a [`DialogueBox`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogueState {
    /// The typewriter is revealing the current page.
    Typing,
    /// The page is fully shown and waits for confirmation.
    WaitAdvance,
    /// The page is fully shown and a choice cursor is active.
    Choosing,
    /// The dialogue is over.
    Finished,
}

/// Player input for one dialogue tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogueInput {
    /// Nothing pressed.
    Idle,
    /// The confirm / A button.
    Confirm,
    /// The cancel / B button.
    Cancel,
    /// Cursor up.
    Up,
    /// Cursor down.
    Down,
}

/// What happened during one [`DialogueBox::tick_dialogue`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogueEvent {
    /// Nothing notable (more characters may have been revealed).
    Quiet,
    /// The current page just became fully visible.
    PageComplete,
    /// The dialogue moved to this page index.
    PageAdvanced(u16),
    /// The choice cursor moved to this index.
    CursorMoved(u8),
    /// This choice was confirmed (the page change, if any, happened too).
    ChoiceMade(u8),
    /// The last page was confirmed.
    Finished,
    /// The player cancelled out of the dialogue.
    Cancelled,
}

/// A paged typewriter dialogue box over borrowed text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DialogueBox<'a> {
    pages: &'a [DialoguePage<'a>],
    state: DialogueState,
    page: usize,
    revealed: u32,
    accum_q8: u32,
    speed_q8: u32,
    cursor: u8,
    last_choice: Option<u8>,
}

fn check_text(page: usize, text: &str) -> Result<(), GridError> {
    if text.len() > MAX_LINE_LEN {
        return Err(GridError::InvalidDialogue {
            page,
            reason: "line longer than MAX_LINE_LEN",
        });
    }
    if !text.bytes().all(|b| (0x20..=0x7E).contains(&b)) {
        return Err(GridError::InvalidDialogue {
            page,
            reason: "text must be printable ASCII",
        });
    }
    Ok(())
}

fn page_chars(page: &DialoguePage<'_>) -> u32 {
    page.lines
        .iter()
        .map(|l| u32::try_from(l.len()).unwrap_or(u32::MAX))
        .fold(0u32, u32::saturating_add)
}

impl<'a> DialogueBox<'a> {
    /// Validates `pages` and opens the box on page 0.
    ///
    /// # Errors
    ///
    /// [`GridError::TooMany`] for too many pages and
    /// [`GridError::InvalidDialogue`] for out-of-bounds lines, choices,
    /// jump targets or non-ASCII text.
    pub fn open(pages: &'a [DialoguePage<'a>]) -> Result<Self, GridError> {
        if pages.is_empty() {
            return Err(GridError::InvalidDialogue {
                page: 0,
                reason: "dialogue has no pages",
            });
        }
        if pages.len() > MAX_PAGES {
            return Err(GridError::TooMany {
                what: "dialogue pages",
                count: pages.len(),
                max: MAX_PAGES,
            });
        }
        for (i, p) in pages.iter().enumerate() {
            if p.lines.len() > MAX_LINES {
                return Err(GridError::InvalidDialogue {
                    page: i,
                    reason: "too many lines",
                });
            }
            if p.choices.len() > MAX_CHOICES {
                return Err(GridError::InvalidDialogue {
                    page: i,
                    reason: "too many choices",
                });
            }
            if !p.choice_targets.is_empty() && p.choice_targets.len() != p.choices.len() {
                return Err(GridError::InvalidDialogue {
                    page: i,
                    reason: "choice_targets must match choices",
                });
            }
            if p.choice_targets
                .iter()
                .any(|&t| usize::from(t) >= pages.len())
            {
                return Err(GridError::InvalidDialogue {
                    page: i,
                    reason: "choice target out of range",
                });
            }
            for l in p.lines.iter().chain(p.choices.iter()) {
                check_text(i, l)?;
            }
        }
        Ok(Self {
            pages,
            state: DialogueState::Typing,
            page: 0,
            revealed: 0,
            accum_q8: 0,
            speed_q8: DEFAULT_TEXT_SPEED_Q8,
            cursor: 0,
            last_choice: None,
        })
    }

    /// Sets the typewriter speed in Q8 characters per tick.
    ///
    /// # Errors
    ///
    /// [`GridError::InvalidParameter`] outside `1..=MAX_TEXT_SPEED_Q8`.
    pub fn set_text_speed(&mut self, speed_q8: u32) -> Result<(), GridError> {
        if speed_q8 == 0 || speed_q8 > MAX_TEXT_SPEED_Q8 {
            return Err(GridError::InvalidParameter {
                what: "text speed",
                value: i64::from(speed_q8),
            });
        }
        self.speed_q8 = speed_q8;
        Ok(())
    }

    /// The current state.
    pub const fn dialogue_state(&self) -> DialogueState {
        self.state
    }

    /// The current page index.
    pub const fn page_index(&self) -> usize {
        self.page
    }

    /// The choice cursor.
    pub const fn cursor_index(&self) -> u8 {
        self.cursor
    }

    /// The most recently confirmed choice.
    pub const fn last_choice_made(&self) -> Option<u8> {
        self.last_choice
    }

    /// Characters revealed so far on the current page.
    pub const fn revealed_chars(&self) -> u32 {
        self.revealed
    }

    /// The current page.
    pub fn current_page(&self) -> Option<&'a DialoguePage<'a>> {
        self.pages.get(self.page)
    }

    /// The revealed prefix of line `index` of the current page (`""` when
    /// the line does not exist or nothing of it is visible yet).
    pub fn visible_line(&self, index: usize) -> &'a str {
        let Some(page) = self.current_page() else {
            return "";
        };
        let Some(line) = page.lines.get(index) else {
            return "";
        };
        let prior: u32 = page
            .lines
            .iter()
            .take(index)
            .map(|l| u32::try_from(l.len()).unwrap_or(u32::MAX))
            .fold(0u32, u32::saturating_add);
        let shown = self.revealed.saturating_sub(prior) as usize;
        line.get(..shown.min(line.len())).unwrap_or("")
    }

    fn enter_page(&mut self, next: usize) -> DialogueEvent {
        if next >= self.pages.len() {
            self.state = DialogueState::Finished;
            return DialogueEvent::Finished;
        }
        self.page = next;
        self.revealed = 0;
        self.accum_q8 = 0;
        self.cursor = 0;
        self.state = DialogueState::Typing;
        DialogueEvent::PageAdvanced(u16::try_from(next).unwrap_or(u16::MAX))
    }

    fn complete_page(&mut self, page: &DialoguePage<'_>) {
        self.revealed = page_chars(page);
        self.accum_q8 = 0;
        self.state = if page.choices.is_empty() {
            DialogueState::WaitAdvance
        } else {
            DialogueState::Choosing
        };
    }

    /// Advances the dialogue by one tick.
    ///
    /// # Errors
    ///
    /// [`GridError::InvalidState`] when a hand-built box points past its
    /// pages.
    pub fn tick_dialogue(&mut self, input: DialogueInput) -> Result<DialogueEvent, GridError> {
        let Some(page) = self.pages.get(self.page) else {
            return Err(GridError::InvalidState {
                what: "dialogue page index out of range",
            });
        };
        match self.state {
            DialogueState::Finished => Ok(DialogueEvent::Quiet),
            DialogueState::Typing => {
                if input == DialogueInput::Cancel {
                    self.state = DialogueState::Finished;
                    return Ok(DialogueEvent::Cancelled);
                }
                if input == DialogueInput::Confirm {
                    self.complete_page(page);
                    return Ok(DialogueEvent::PageComplete);
                }
                let total = page_chars(page);
                self.accum_q8 = self.accum_q8.saturating_add(self.speed_q8);
                let n = self.accum_q8 >> 8;
                self.accum_q8 &= 0xFF;
                self.revealed = self.revealed.saturating_add(n).min(total);
                if self.revealed >= total {
                    self.complete_page(page);
                    return Ok(DialogueEvent::PageComplete);
                }
                Ok(DialogueEvent::Quiet)
            }
            DialogueState::WaitAdvance => match input {
                DialogueInput::Confirm => Ok(self.enter_page(self.page.saturating_add(1))),
                DialogueInput::Cancel => {
                    self.state = DialogueState::Finished;
                    Ok(DialogueEvent::Cancelled)
                }
                _ => Ok(DialogueEvent::Quiet),
            },
            DialogueState::Choosing => {
                let last = u8::try_from(page.choices.len().saturating_sub(1)).unwrap_or(0);
                match input {
                    DialogueInput::Up if self.cursor > 0 => {
                        self.cursor -= 1;
                        Ok(DialogueEvent::CursorMoved(self.cursor))
                    }
                    DialogueInput::Down if self.cursor < last => {
                        self.cursor += 1;
                        Ok(DialogueEvent::CursorMoved(self.cursor))
                    }
                    DialogueInput::Confirm => {
                        let picked = self.cursor.min(last);
                        self.last_choice = Some(picked);
                        let next = page
                            .choice_targets
                            .get(usize::from(picked))
                            .map_or(self.page.saturating_add(1), |&t| usize::from(t));
                        self.enter_page(next);
                        Ok(DialogueEvent::ChoiceMade(picked))
                    }
                    _ => Ok(DialogueEvent::Quiet),
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Dialogue lowering
// ---------------------------------------------------------------------------

/// The nine frame tiles of a dialogue box border plus its fill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameTiles {
    /// Top-left corner.
    pub tl: u16,
    /// Top edge.
    pub t: u16,
    /// Top-right corner.
    pub tr: u16,
    /// Left edge.
    pub l: u16,
    /// Interior fill (also used for space characters).
    pub fill: u16,
    /// Right edge.
    pub r: u16,
    /// Bottom-left corner.
    pub bl: u16,
    /// Bottom edge.
    pub b: u16,
    /// Bottom-right corner.
    pub br: u16,
}

/// Where and how a [`DialogueBox`] is drawn on the tile grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DialogueStyle {
    /// Grid column of the frame's top-left corner.
    pub col: u16,
    /// Grid row of the frame's top-left corner.
    pub row: u16,
    /// Interior width in tiles (frame excluded).
    pub inner_cols: u16,
    /// Interior height in tiles (frame excluded).
    pub inner_rows: u16,
    /// Tile id of glyph `0x20`; glyph `b` is `glyph_base + (b - 0x20)` (>= 1).
    pub glyph_base: u16,
    /// Tile id of the choice cursor.
    pub cursor_tile: u16,
    /// Frame tiles.
    pub frame: FrameTiles,
    /// Tint applied to every cell.
    pub tint: Rgb8,
}

fn style_extent(style: &DialogueStyle) -> Result<(usize, usize), GridError> {
    if style.inner_cols == 0 || style.inner_rows == 0 || style.glyph_base == 0 {
        return Err(GridError::InvalidParameter {
            what: "dialogue style",
            value: 0,
        });
    }
    let cols = usize::from(style.inner_cols) + 2;
    let rows = usize::from(style.inner_rows) + 2;
    let cells = cols.checked_mul(rows).ok_or(GridError::SizeOverflow)?;
    if cells > MAX_DIALOGUE_CELLS {
        return Err(GridError::TooMany {
            what: "dialogue cells",
            count: cells,
            max: MAX_DIALOGUE_CELLS,
        });
    }
    let end_col = usize::from(style.col) + cols;
    let end_row = usize::from(style.row) + rows;
    if end_col > usize::from(u16::MAX) || end_row > usize::from(u16::MAX) {
        return Err(GridError::SizeOverflow);
    }
    Ok((cols, rows))
}

fn glyph_tile(style: &DialogueStyle, byte: u8) -> Result<u16, GridError> {
    if byte == b' ' {
        return Ok(style.frame.fill);
    }
    let offset = u16::from(byte.saturating_sub(0x20));
    style
        .glyph_base
        .checked_add(offset)
        .ok_or(GridError::SizeOverflow)
}

/// The tile shown at interior position `(ic, ir)` of the box.
fn interior_tile(
    dlg: &DialogueBox<'_>,
    style: &DialogueStyle,
    ic: usize,
    ir: usize,
) -> Result<u16, GridError> {
    let Some(page) = dlg.current_page() else {
        return Ok(style.frame.fill);
    };
    if ir < page.lines.len() {
        return match dlg.visible_line(ir).as_bytes().get(ic) {
            Some(&b) => glyph_tile(style, b),
            None => Ok(style.frame.fill),
        };
    }
    if dlg.state != DialogueState::Choosing {
        return Ok(style.frame.fill);
    }
    let ci = ir - page.lines.len();
    let Some(label) = page.choices.get(ci) else {
        return Ok(style.frame.fill);
    };
    if ic == 0 {
        return Ok(if usize::from(dlg.cursor) == ci {
            style.cursor_tile
        } else {
            style.frame.fill
        });
    }
    match label.as_bytes().get(ic - 1) {
        Some(&b) => glyph_tile(style, b),
        None => Ok(style.frame.fill),
    }
}

/// The number of cells [`lower_dialogue_cells_into`] writes for `style`.
///
/// # Errors
///
/// Style validation errors ([`GridError::InvalidParameter`],
/// [`GridError::TooMany`], [`GridError::SizeOverflow`]).
pub fn count_dialogue_cells(style: &DialogueStyle) -> Result<usize, GridError> {
    let (cols, rows) = style_extent(style)?;
    Ok(cols * rows)
}

/// Lowers the box (frame, revealed glyphs, choices and cursor) into `out`
/// as `paint-batch-tiles` (`P2D09`) cells at absolute grid positions, one
/// cell per grid position. Returns the number of cells written. Zero heap
/// allocation.
///
/// Text and choices must fit the interior: lines and choices may not exceed
/// `inner_rows` rows nor `inner_cols` columns (choices are indented by one
/// column for the cursor).
///
/// # Errors
///
/// Style validation errors, [`GridError::InvalidParameter`] when the page
/// does not fit the interior, [`GridError::BufferTooSmall`] (`required`
/// reports the exact count) or [`GridError::SizeOverflow`].
pub fn lower_dialogue_cells_into(
    dlg: &DialogueBox<'_>,
    style: &DialogueStyle,
    out: &mut [TileBatchCell],
) -> Result<usize, GridError> {
    let (cols, rows) = style_extent(style)?;
    let needed = cols * rows;
    if let Some(page) = dlg.current_page() {
        let widest_line = page.lines.iter().map(|l| l.len()).max().unwrap_or(0);
        let widest_choice = page.choices.iter().map(|c| c.len() + 1).max().unwrap_or(0);
        let used_rows = page.lines.len() + page.choices.len();
        if widest_line.max(widest_choice) > usize::from(style.inner_cols)
            || used_rows > usize::from(style.inner_rows)
        {
            return Err(GridError::InvalidParameter {
                what: "dialogue page does not fit the box interior",
                value: i64::try_from(used_rows).unwrap_or(i64::MAX),
            });
        }
    }
    if out.len() < needed {
        return Err(GridError::BufferTooSmall {
            required: needed,
            actual: out.len(),
        });
    }
    let f = &style.frame;
    let mut n = 0usize;
    for r in 0..rows {
        for c in 0..cols {
            let top = r == 0;
            let bottom = r + 1 == rows;
            let left = c == 0;
            let right = c + 1 == cols;
            let tile = match (top, bottom, left, right) {
                (true, _, true, _) => f.tl,
                (true, _, _, true) => f.tr,
                (true, ..) => f.t,
                (_, true, true, _) => f.bl,
                (_, true, _, true) => f.br,
                (_, true, ..) => f.b,
                (_, _, true, _) => f.l,
                (_, _, _, true) => f.r,
                _ => interior_tile(dlg, style, c - 1, r - 1)?,
            };
            let col =
                u16::try_from(usize::from(style.col) + c).map_err(|_| GridError::SizeOverflow)?;
            let row =
                u16::try_from(usize::from(style.row) + r).map_err(|_| GridError::SizeOverflow)?;
            if let Some(slot) = out.get_mut(n) {
                *slot = TileBatchCell::new(col, row, tile).with_color(style.tint);
            }
            n += 1;
        }
    }
    Ok(n)
}

/// Lowers the box into a `paint-instructions` (`P2D00`) scene of
/// `tile_w x tile_h` pixel tiles on a `view_w x view_h` canvas. Allocates;
/// not a hot path.
///
/// # Errors
///
/// Everything [`lower_dialogue_cells_into`] reports,
/// [`GridError::InvalidParameter`] for tile sizes outside
/// `1..=MAX_TILE_PX`, or [`GridError::Batch`].
pub fn lower_dialogue_to_paint_scene(
    dlg: &DialogueBox<'_>,
    style: &DialogueStyle,
    tile_w: u32,
    tile_h: u32,
    view_w: u32,
    view_h: u32,
    background: &str,
) -> Result<PaintScene, GridError> {
    for (what, v) in [("tile width", tile_w), ("tile height", tile_h)] {
        if v == 0 || v > MAX_TILE_PX {
            return Err(GridError::InvalidParameter {
                what,
                value: i64::from(v),
            });
        }
    }
    let count = count_dialogue_cells(style)?;
    let mut cells = vec![TileBatchCell::new(0, 0, 0); count];
    let written = lower_dialogue_cells_into(dlg, style, &mut cells)?;
    cells.truncate(written);
    paint_batch_tiles::lower_tile_batch_to_paint_scene(
        f64::from(view_w),
        f64::from(view_h),
        f64::from(tile_w),
        f64::from(tile_h),
        background,
        &cells,
    )
    .map_err(GridError::Batch)
}
