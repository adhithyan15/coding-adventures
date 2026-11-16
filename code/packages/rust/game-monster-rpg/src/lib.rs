//! # game-monster-rpg
//!
//! GAME25: A complete playable GBA/Pokémon-style RPG vertical slice combining all Era 2 crates.
//!
//! ## Architectural Features
//!
//! * **Overworld Engine:** Multi-layer background tiles, grid-aligned actor movement,
//!   warp pads, multi-map transitions (Town, Route 1, Tall Grass Meadow, Healing Center, Gym),
//!   NPC interactions with dialogue boxes and party healing, and step encounters in tall grass.
//! * **Turn-Based Battle Engine:** Seamless palette fade transitions (`fade_palette_into`),
//!   turn-based battles against wild creatures and gym trainers, elemental type effectiveness
//!   (15-type chart), stat stages, deterministic damage formulas, capture attempts, fleeing,
//!   switching, and XP/level progression.
//! * **Zero-Allocation Rendering & Audio:** Retained-mode sprite and tile lowering to
//!   universal [`PaintScene`] and raster [`PixelContainer`], deterministic chiptune audio
//!   synthesis (`arcade-sfx`) and real-time PCM ring buffer mixing (`audio-mixer-realtime`),
//!   and fixed-timestep simulation accumulator (`game-clock`).
//! * **Pure Computation & Safety:** Zero ambient authority (`"capabilities": []`),
//!   `#![forbid(unsafe_code)]`, and complete 26-CWE matrix closure.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Pure computation capability witness module.
pub mod capabilities;
pub use capabilities::PureCapabilities;

// Re-exports from underlying Era 2 crates
pub use arcade_sfx::{SfxBank, SfxError, SfxPreset};
pub use audio_mixer_realtime::{MixerError, PcmRingBuffer};
pub use game_clock::{advance_frame, ClockError, ClockStepResult, GameClockConfig, GameClockState};
pub use paint_batch_tiles::{
    lower_sprites_to_paint_scene, PaintBatchError, PaintScene, SpriteQuad,
};
pub use paint_instructions::PixelContainer;
pub use palette_indexed::{fade_palette_into, BitDepth, ChannelBits, Palette, PaletteError, Rgba8};
pub use rpg_battle_engine::{
    attempt_capture, calc_damage, catch_value, dual_multiplier, flee_succeeds, hp_bar_blend_color,
    hp_bar_pixels, hp_bar_state, BaseStats, Battle, BattleError, BattleRng, Category, CombatStats,
    Command, Creature, CreatureSpec, DamageResult, Effect, Event, HpBarState, Move, Phase, Side,
    StatIndex, Status, MAX_PARTY, MULT_ONE, NO_TYPE, TYPE_COUNT,
};
pub use rpg_grid_world::{
    face_npc_toward, interact_target, lower_dialogue_to_paint_scene, DialogueBox, DialogueEvent,
    DialogueInput, DialoguePage, DialogueState, DialogueStyle, Facing, FrameTiles, GridActor,
    GridError, GridRng, MoveState, Npc, NpcKind, WarpPad, WarpTarget, WarpTrigger,
};
pub use sprite_oam::{
    Animator, MetaFrame, OamEntry, OamError, OamTable, PlayMode, SpriteShape, SpriteSize,
};
pub use tilemap2d::{
    lower_layers_to_paint_scene, render_layers_to_container, Camera, TileCell, TileLayer,
    TilemapError, Tileset, Viewport,
};

// ---------------------------------------------------------------------------
// Constants & Public Type Aliases
// ---------------------------------------------------------------------------

/// Standard GBA logical screen width in pixels.
pub const SCREEN_WIDTH: u32 = 240;

/// Standard GBA logical screen height in pixels.
pub const SCREEN_HEIGHT: u32 = 160;

/// Standard overworld tile width and height in pixels.
pub const TILE_SIZE: u32 = 16;

/// Map width in tiles.
pub const MAP_WIDTH: usize = 16;

/// Map height in tiles.
pub const MAP_HEIGHT: usize = 16;

/// Total tiles per map (16 x 16).
pub const MAP_TILES: usize = MAP_WIDTH * MAP_HEIGHT;

/// Total number of distinct maps in the vertical slice.
pub const MAP_COUNT: usize = 5;

/// Map ID for Starter Town.
pub const MAP_TOWN: usize = 0;

/// Map ID for Route 1.
pub const MAP_ROUTE1: usize = 1;

/// Map ID for Tall Grass Meadow.
pub const MAP_TALL_GRASS: usize = 2;

/// Map ID for Healing Center.
pub const MAP_HEALING_CENTER: usize = 3;

/// Map ID for Gym.
pub const MAP_GYM: usize = 4;

/// Tile attribute bit: solid obstacle.
pub const ATTR_SOLID: u8 = 1 << 0;

/// Tile attribute bit: tall grass (triggers wild encounter rolls).
pub const ATTR_TALL_GRASS: u8 = 1 << 1;

/// Tile attribute bit: warp portal trigger.
pub const ATTR_WARP: u8 = 1 << 2;

/// Maximum number of moves per creature.
pub const MAX_MOVES: usize = 4;

/// Maximum number of warps in the world table.
pub const MAX_WARPS: usize = 16;

/// Maximum number of NPCs in the world table.
pub const MAX_NPCS: usize = 8;

/// Maximum sprites emitted per frame.
pub const MAX_RPG_SPRITES: usize = 128;

/// Total number of frames for a palette fade transition.
pub const FADE_TOTAL_STEPS: u16 = 16;

/// Scratch buffer of 64 PCM audio samples.
pub type AudioScratch64 = [i16; 64];

/// Fixed array of 128 sprite quads.
pub type SpriteQuadArray128 = [SpriteQuad; 128];

/// Fixed array of 256 tile cells (16x16 map).
pub type TileCellArray256 = [TileCell; 256];

/// Fixed array of 16 RGBA colors for palette bank.
pub type PaletteRgbaArray16 = [Rgba8; 16];

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Comprehensive error enumeration for the Monster RPG vertical slice.
#[derive(Debug, Clone, PartialEq)]
pub enum RpgError {
    /// Provided map identifier is out of bounds (CWE-20, CWE-125).
    InvalidMapId,
    /// Provided tile coordinate exceeds map boundaries (CWE-125, CWE-787).
    InvalidTileCoord,
    /// Party index is out of bounds (CWE-20, CWE-125).
    InvalidPartyIndex,
    /// Creature move slot is out of bounds (CWE-20, CWE-125).
    InvalidMoveSlot,
    /// Palette fade fraction exceeds 255 (CWE-190).
    InvalidFadeFraction,
    /// Step encounter rate exceeds 256 (CWE-190).
    InvalidStepRate,
    /// Invalid game configuration parameters (CWE-20, CWE-369).
    InvalidConfig,
    /// Party is full; cannot add captured monster.
    PartyFull,
    /// Party is empty.
    PartyEmpty,
    /// No active battle in progress.
    NoActiveBattle,
    /// Dialogue is not active.
    DialogueNotActive,
    /// Output buffer length is insufficient.
    BufferTooSmall,
    /// Battle engine error from `rpg-battle-engine`.
    BattleEngineError(BattleError),
    /// Grid world error from `rpg-grid-world`.
    GridWorldError(GridError),
    /// Tilemap error from `tilemap2d`.
    TilemapError(TilemapError),
    /// Sprite OAM error from `sprite-oam`.
    OamError(OamError),
    /// Game clock error from `game-clock`.
    ClockError(ClockError),
    /// SFX synthesizer error from `arcade-sfx`.
    SfxError(SfxError),
    /// Real-time audio mixer error from `audio-mixer-realtime`.
    MixerError(MixerError),
    /// Palette error from `palette-indexed`.
    PaletteError(PaletteError),
    /// Paint batch error from `paint-batch-tiles`.
    PaintBatchError(PaintBatchError),
}

impl From<BattleError> for RpgError {
    fn from(err: BattleError) -> Self {
        RpgError::BattleEngineError(err)
    }
}

impl From<GridError> for RpgError {
    fn from(err: GridError) -> Self {
        RpgError::GridWorldError(err)
    }
}

impl From<TilemapError> for RpgError {
    fn from(err: TilemapError) -> Self {
        RpgError::TilemapError(err)
    }
}

impl From<OamError> for RpgError {
    fn from(err: OamError) -> Self {
        RpgError::OamError(err)
    }
}

impl From<ClockError> for RpgError {
    fn from(err: ClockError) -> Self {
        RpgError::ClockError(err)
    }
}

impl From<SfxError> for RpgError {
    fn from(err: SfxError) -> Self {
        RpgError::SfxError(err)
    }
}

impl From<MixerError> for RpgError {
    fn from(err: MixerError) -> Self {
        RpgError::MixerError(err)
    }
}

impl From<PaletteError> for RpgError {
    fn from(err: PaletteError) -> Self {
        RpgError::PaletteError(err)
    }
}

impl From<PaintBatchError> for RpgError {
    fn from(err: PaintBatchError) -> Self {
        RpgError::PaintBatchError(err)
    }
}

// ---------------------------------------------------------------------------
// Warrant Verification Operations
// ---------------------------------------------------------------------------

/// Validate map identifier against allowed bounds `0..5` (CWE-20, CWE-125).
pub fn validate_map_id(id: usize) -> Result<bool, RpgError> {
    if id < MAP_COUNT {
        Ok(true)
    } else {
        Err(RpgError::InvalidMapId)
    }
}

/// Validate party index against maximum party size `0..6` (CWE-20, CWE-125).
pub fn validate_party_index(idx: usize) -> Result<bool, RpgError> {
    if idx < MAX_PARTY {
        Ok(true)
    } else {
        Err(RpgError::InvalidPartyIndex)
    }
}

/// Validate creature move slot against maximum move count `0..4` (CWE-20, CWE-125).
pub fn validate_move_slot(slot: usize) -> Result<bool, RpgError> {
    if slot < MAX_MOVES {
        Ok(true)
    } else {
        Err(RpgError::InvalidMoveSlot)
    }
}

/// Validate palette fade fraction `0..=255` (CWE-190).
pub fn validate_fade_fraction(frac: u32) -> Result<bool, RpgError> {
    if frac <= 255 {
        Ok(true)
    } else {
        Err(RpgError::InvalidFadeFraction)
    }
}

/// Validate encounter step rate `0..=256` (CWE-190).
pub fn validate_step_rate(rate: u32) -> Result<bool, RpgError> {
    if rate <= 256 {
        Ok(true)
    } else {
        Err(RpgError::InvalidStepRate)
    }
}

// ---------------------------------------------------------------------------
// Game Configuration
// ---------------------------------------------------------------------------

/// Configuration parameters for `game-monster-rpg`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RpgConfig {
    /// Screen width in pixels (typically 240).
    pub screen_width: u32,
    /// Screen height in pixels (typically 160).
    pub screen_height: u32,
    /// Audio sample rate in Hz (typically 44100).
    pub sample_rate_hz: u32,
    /// Walk animation ticks per step.
    pub walk_ticks: u8,
    /// Base encounter rate divisor.
    pub encounter_rate: u32,
}

impl RpgConfig {
    /// Validate configuration invariants (CWE-20, CWE-369).
    pub fn validate_rpg_config(&self) -> Result<(), RpgError> {
        if self.screen_width == 0 || self.screen_height == 0 || self.sample_rate_hz == 0 {
            return Err(RpgError::InvalidConfig);
        }
        if self.walk_ticks == 0 {
            return Err(RpgError::InvalidConfig);
        }
        validate_step_rate(self.encounter_rate)?;
        Ok(())
    }
}

/// Returns standard default configuration for 60Hz GBA simulation.
pub fn default_rpg_config() -> RpgConfig {
    RpgConfig {
        screen_width: SCREEN_WIDTH,
        screen_height: SCREEN_HEIGHT,
        sample_rate_hz: 44_100,
        walk_ticks: 8,
        encounter_rate: 32,
    }
}

// ---------------------------------------------------------------------------
// Inputs & Modes
// ---------------------------------------------------------------------------

/// Directional input intent for the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkDirection {
    /// No movement intent.
    None,
    /// Walk up.
    Up,
    /// Walk down.
    Down,
    /// Walk left.
    Left,
    /// Walk right.
    Right,
}

/// Discrete player controller input for an RPG simulation frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RpgInput {
    /// Directional walking input.
    pub walk: WalkDirection,
    /// Action button (A) to interact with NPCs, advance dialogue, or confirm menus.
    pub action: bool,
    /// Cancel/Run button (B).
    pub cancel: bool,
    /// Turn-based battle command selection.
    pub battle_command: Option<Command>,
}

impl RpgInput {
    /// Create input walking in the given direction.
    pub fn walk_dir(dir: WalkDirection) -> Self {
        RpgInput {
            walk: dir,
            action: false,
            cancel: false,
            battle_command: None,
        }
    }

    /// Walk upward.
    pub fn walk_up() -> Self {
        Self::walk_dir(WalkDirection::Up)
    }

    /// Walk downward.
    pub fn walk_down() -> Self {
        Self::walk_dir(WalkDirection::Down)
    }

    /// Walk leftward.
    pub fn walk_left() -> Self {
        Self::walk_dir(WalkDirection::Left)
    }

    /// Walk rightward.
    pub fn walk_right() -> Self {
        Self::walk_dir(WalkDirection::Right)
    }

    /// Action/interact button pressed.
    pub fn interact_button() -> Self {
        RpgInput {
            walk: WalkDirection::None,
            action: true,
            cancel: false,
            battle_command: None,
        }
    }

    /// Battle attack command.
    pub fn battle_fight(slot: usize) -> Self {
        RpgInput {
            walk: WalkDirection::None,
            action: false,
            cancel: false,
            battle_command: Some(Command::Fight {
                slot: u8::try_from(slot).unwrap_or(u8::MAX),
            }),
        }
    }

    /// Battle item (Pokéball) command.
    pub fn battle_ball() -> Self {
        RpgInput {
            walk: WalkDirection::None,
            action: false,
            cancel: false,
            battle_command: Some(Command::Catch { bonus_x10: 15 }),
        }
    }

    /// Battle run command.
    pub fn battle_run() -> Self {
        RpgInput {
            walk: WalkDirection::None,
            action: false,
            cancel: false,
            battle_command: Some(Command::Run),
        }
    }

    /// Battle switch command.
    pub fn battle_switch(index: usize) -> Self {
        RpgInput {
            walk: WalkDirection::None,
            action: false,
            cancel: false,
            battle_command: Some(Command::Switch {
                index: u8::try_from(index).unwrap_or(u8::MAX),
            }),
        }
    }
}

/// High-level mode of the RPG engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    /// Active exploration on the overworld tilemap.
    Overworld,
    /// Typewriter dialogue window open.
    Dialogue,
    /// Smooth palette fade-out entering battle.
    BattleTransitionIn {
        /// Current fade step `0..=FADE_TOTAL_STEPS`.
        step: u16,
        /// True if wild encounter, false if trainer battle.
        wild: bool,
        /// Species ID for wild monster.
        species: u8,
        /// Level for wild monster.
        level: u8,
        /// Trainer kind if trainer battle.
        trainer: Option<RpgNpcKind>,
    },
    /// Turn-based creature battle.
    Battle,
    /// Smooth palette fade-out exiting battle back to overworld.
    BattleTransitionOut {
        /// Current fade step `0..=FADE_TOTAL_STEPS`.
        step: u16,
    },
}

// ---------------------------------------------------------------------------
// World Entities: Warps & NPCs
// ---------------------------------------------------------------------------

/// Overworld warp pad connecting two maps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RpgWarp {
    /// Origin map ID.
    pub source_map: u8,
    /// Origin tile X coordinate.
    pub source_x: u8,
    /// Origin tile Y coordinate.
    pub source_y: u8,
    /// Destination map ID.
    pub target_map: u8,
    /// Destination tile X coordinate.
    pub target_x: u8,
    /// Destination tile Y coordinate.
    pub target_y: u8,
}

/// Kind of overworld NPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpgNpcKind {
    /// Town guide providing introductory hints.
    Guide,
    /// Healing Center nurse who restores the party to full health.
    Nurse,
    /// Route 1 trainer Youngster.
    TrainerYoungster,
    /// Starter gym leader Brock.
    GymLeader,
}

/// Overworld Non-Player Character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RpgNpc {
    /// Map where NPC resides.
    pub map_id: u8,
    /// Tile X coordinate.
    pub x: u8,
    /// Tile Y coordinate.
    pub y: u8,
    /// Facing direction.
    pub facing: Facing,
    /// Character kind.
    pub kind: RpgNpcKind,
    /// Whether this trainer has been defeated.
    pub defeated: bool,
}

impl RpgNpc {
    /// Check whether this NPC occupies the given map and tile coordinate.
    pub fn matches_tile_coord(&self, map_id: u8, x: u8, y: u8) -> bool {
        self.map_id == map_id && self.x == x && self.y == y
    }
}

// ---------------------------------------------------------------------------
// Audio State & Sequencer
// ---------------------------------------------------------------------------

/// State machine for background chiptune audio tracks and sound effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RpgAudioState {
    /// Frame tick counter for sequencer.
    pub tick: u32,
    /// Current step in 8-note melodic loop.
    pub bgm_step: u8,
}

impl RpgAudioState {
    /// Initialize fresh audio sequencer state.
    pub fn init_audio_state() -> Self {
        RpgAudioState {
            tick: 0,
            bgm_step: 0,
        }
    }

    /// Advance the audio sequencer by one frame and trigger chiptune note cues.
    pub fn advance_audio_clock(&mut self, sfx: &mut SfxBank<4>, mode: GameMode) {
        self.tick = self.tick.wrapping_add(1);

        if matches!(mode, GameMode::Overworld | GameMode::Dialogue) && self.tick.is_multiple_of(16)
        {
            self.bgm_step = self.bgm_step.wrapping_add(1) % 8;
            let _ = sfx.trigger_bank_preset(SfxPreset::PongPaddleHit);
        } else if matches!(mode, GameMode::Battle) && self.tick.is_multiple_of(8) {
            self.bgm_step = self.bgm_step.wrapping_add(1) % 8;
            let _ = sfx.trigger_bank_preset(SfxPreset::BreakoutBrickHit { row: self.bgm_step });
        }
    }
}

// ---------------------------------------------------------------------------
// Species & Creature Definitions
// ---------------------------------------------------------------------------

/// Species identifier for Sparky (Electric).
pub const SPECIES_SPARKY: u8 = 0;
/// Species identifier for Pyrokid (Fire).
pub const SPECIES_PYROKID: u8 = 1;
/// Species identifier for Aquafin (Water).
pub const SPECIES_AQUAFIN: u8 = 2;
/// Species identifier for Leafox (Grass).
pub const SPECIES_LEAFOX: u8 = 3;
/// Species identifier for Geostone (Rock).
pub const SPECIES_GEOSTONE: u8 = 4;
/// Species identifier for Graveling (Normal).
pub const SPECIES_GRAVELING: u8 = 5;

const MOVE_TACKLE: Move = Move {
    move_type: 0, // Normal
    category: Category::Physical,
    power: 40,
    accuracy: 100,
    max_pp: 35,
    priority: 0,
    effect: Effect::NoEffect,
};

const MOVE_SCRATCH: Move = Move {
    move_type: 0, // Normal
    category: Category::Physical,
    power: 40,
    accuracy: 100,
    max_pp: 35,
    priority: 0,
    effect: Effect::NoEffect,
};

const MOVE_EMBER: Move = Move {
    move_type: 1, // Fire
    category: Category::Special,
    power: 40,
    accuracy: 100,
    max_pp: 25,
    priority: 0,
    effect: Effect::Inflict {
        status: Status::Burn,
        chance: 10,
    },
};

const MOVE_WATER_GUN: Move = Move {
    move_type: 2, // Water
    category: Category::Special,
    power: 40,
    accuracy: 100,
    max_pp: 25,
    priority: 0,
    effect: Effect::NoEffect,
};

const MOVE_THUNDER_SHOCK: Move = Move {
    move_type: 3, // Electric
    category: Category::Special,
    power: 40,
    accuracy: 100,
    max_pp: 25,
    priority: 0,
    effect: Effect::Inflict {
        status: Status::Paralysis,
        chance: 10,
    },
};

const MOVE_VINE_WHIP: Move = Move {
    move_type: 4, // Grass
    category: Category::Physical,
    power: 45,
    accuracy: 100,
    max_pp: 25,
    priority: 0,
    effect: Effect::NoEffect,
};

const MOVE_ROCK_THROW: Move = Move {
    move_type: 12, // Rock
    category: Category::Physical,
    power: 50,
    accuracy: 90,
    max_pp: 15,
    priority: 0,
    effect: Effect::NoEffect,
};

const MOVE_QUICK_ATTACK: Move = Move {
    move_type: 0, // Normal
    category: Category::Physical,
    power: 40,
    accuracy: 100,
    max_pp: 30,
    priority: 1,
    effect: Effect::NoEffect,
};

/// Construct a creature of the given species at the requested level.
pub fn create_monster(species_id: u8, level: u8) -> Result<Creature, RpgError> {
    let lvl = level.clamp(1, 100);
    let creature = match species_id % 6 {
        SPECIES_SPARKY => {
            static SPARKY_MOVES: [Move; 3] = [MOVE_TACKLE, MOVE_THUNDER_SHOCK, MOVE_QUICK_ATTACK];
            let spec = CreatureSpec {
                type1: 3, // Electric
                type2: NO_TYPE,
                level: lvl,
                base: BaseStats {
                    hp: 35,
                    atk: 55,
                    def: 40,
                    spa: 50,
                    spd: 50,
                    spe: 90,
                },
                moves: &SPARKY_MOVES,
                catch_rate: 190,
                exp_yield: 64,
            };
            Creature::from_spec(&spec)?
        }
        SPECIES_PYROKID => {
            static PYRO_MOVES: [Move; 3] = [MOVE_SCRATCH, MOVE_EMBER, MOVE_QUICK_ATTACK];
            let spec = CreatureSpec {
                type1: 1, // Fire
                type2: NO_TYPE,
                level: lvl,
                base: BaseStats {
                    hp: 39,
                    atk: 52,
                    def: 43,
                    spa: 60,
                    spd: 50,
                    spe: 65,
                },
                moves: &PYRO_MOVES,
                catch_rate: 150,
                exp_yield: 62,
            };
            Creature::from_spec(&spec)?
        }
        SPECIES_AQUAFIN => {
            static AQUA_MOVES: [Move; 2] = [MOVE_TACKLE, MOVE_WATER_GUN];
            let spec = CreatureSpec {
                type1: 2, // Water
                type2: NO_TYPE,
                level: lvl,
                base: BaseStats {
                    hp: 44,
                    atk: 48,
                    def: 65,
                    spa: 50,
                    spd: 64,
                    spe: 43,
                },
                moves: &AQUA_MOVES,
                catch_rate: 150,
                exp_yield: 63,
            };
            Creature::from_spec(&spec)?
        }
        SPECIES_LEAFOX => {
            static LEAF_MOVES: [Move; 2] = [MOVE_TACKLE, MOVE_VINE_WHIP];
            let spec = CreatureSpec {
                type1: 4, // Grass
                type2: NO_TYPE,
                level: lvl,
                base: BaseStats {
                    hp: 45,
                    atk: 49,
                    def: 49,
                    spa: 65,
                    spd: 65,
                    spe: 45,
                },
                moves: &LEAF_MOVES,
                catch_rate: 150,
                exp_yield: 64,
            };
            Creature::from_spec(&spec)?
        }
        SPECIES_GEOSTONE => {
            static GEO_MOVES: [Move; 2] = [MOVE_TACKLE, MOVE_ROCK_THROW];
            let spec = CreatureSpec {
                type1: 12, // Rock
                type2: NO_TYPE,
                level: lvl,
                base: BaseStats {
                    hp: 40,
                    atk: 80,
                    def: 100,
                    spa: 30,
                    spd: 30,
                    spe: 20,
                },
                moves: &GEO_MOVES,
                catch_rate: 120,
                exp_yield: 60,
            };
            Creature::from_spec(&spec)?
        }
        _ => {
            // GRAVELING
            static GRAVEL_MOVES: [Move; 2] = [MOVE_TACKLE, MOVE_QUICK_ATTACK];
            let spec = CreatureSpec {
                type1: 0, // Normal
                type2: NO_TYPE,
                level: lvl,
                base: BaseStats {
                    hp: 30,
                    atk: 56,
                    def: 35,
                    spa: 25,
                    spd: 35,
                    spe: 72,
                },
                moves: &GRAVEL_MOVES,
                catch_rate: 255,
                exp_yield: 50,
            };
            Creature::from_spec(&spec)?
        }
    };
    Ok(creature)
}

// ---------------------------------------------------------------------------
// World Map Layouts
// ---------------------------------------------------------------------------

/// Generate tile attributes for map cell `(x, y)`.
pub fn get_map_tile_attr(map_id: u8, x: u8, y: u8) -> u8 {
    if x >= MAP_WIDTH as u8 || y >= MAP_HEIGHT as u8 {
        return ATTR_SOLID;
    }

    match map_id as usize {
        MAP_TOWN => {
            // North exit to Route 1 at (7, 0)
            if x == 7 && y == 0 {
                ATTR_WARP
            } else if x == 7 && y == 4 {
                // PokéCenter door
                ATTR_WARP
            } else if x == 12 && y == 4 {
                // Gym door
                ATTR_WARP
            } else if y == 0 || y == 15 || x == 0 || x == 15 {
                // Boundary trees
                ATTR_SOLID
            } else if (6..=8).contains(&x) && (2..=3).contains(&y) {
                // PokéCenter building
                ATTR_SOLID
            } else if (11..=13).contains(&x) && (2..=3).contains(&y) {
                // Gym building
                ATTR_SOLID
            } else {
                0 // Walkable town ground
            }
        }
        MAP_ROUTE1 => {
            // South exit to Town at (7, 15)
            if x == 7 && y == 15 {
                ATTR_WARP
            } else if x == 7 && y == 0 {
                // North exit to Tall Grass at (7, 0)
                ATTR_WARP
            } else if x == 0 || x == 15 {
                ATTR_SOLID
            } else if ((2..=5).contains(&x) && (4..=8).contains(&y))
                || ((9..=13).contains(&x) && (7..=12).contains(&y))
            {
                ATTR_TALL_GRASS
            } else {
                0 // Dirt path
            }
        }
        MAP_TALL_GRASS => {
            // South exit to Route 1 at (7, 15)
            if x == 7 && y == 15 {
                ATTR_WARP
            } else if y == 0 || y == 15 || x == 0 || x == 15 {
                ATTR_SOLID
            } else if x == 7 {
                0 // Central path
            } else {
                ATTR_TALL_GRASS // Dense meadow
            }
        }
        MAP_HEALING_CENTER => {
            // South exit to Town at (7, 15)
            if x == 7 && y == 15 {
                ATTR_WARP
            } else if y == 0 || y == 15 || x == 0 || x == 15 || ((5..=9).contains(&x) && y == 4) {
                ATTR_SOLID // Boundary and Nurse counter
            } else {
                0 // Interior floor
            }
        }
        MAP_GYM => {
            // South exit to Town at (7, 15)
            if x == 7 && y == 15 {
                ATTR_WARP
            } else if y == 0
                || y == 15
                || x == 0
                || x == 15
                || ((x == 4 || x == 10) && (y == 5 || y == 9))
            {
                ATTR_SOLID // Boundary and decorative pillars
            } else {
                0 // Arena floor
            }
        }
        _ => ATTR_SOLID,
    }
}

// ---------------------------------------------------------------------------
// Frame Events & Clock Step Result
// ---------------------------------------------------------------------------

/// Events generated during a single frame of RPG simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RpgFrameEvents {
    /// True if player completed a step.
    pub stepped: bool,
    /// Destination map ID if a warp was triggered.
    pub warp_triggered: Option<u8>,
    /// Kind of NPC interacted with, if dialogue started.
    pub npc_spoken: Option<RpgNpcKind>,
    /// True if player's entire party was healed to full.
    pub party_healed: bool,
    /// True if a battle transition initiated.
    pub battle_started: bool,
    /// True if battle is against a wild creature.
    pub wild_encounter: bool,
    /// True if a monster was successfully captured.
    pub creature_caught: bool,
    /// True if a party monster leveled up.
    pub creature_leveled_up: bool,
    /// True if a battle finished.
    pub battle_ended: bool,
    /// True if battle ended in victory.
    pub victory: bool,
    /// True if battle ended by fleeing.
    pub fled: bool,
}

impl RpgFrameEvents {
    /// Merge events from another frame into self.
    pub fn merge_frame_events(&mut self, other: &RpgFrameEvents) {
        self.stepped |= other.stepped;
        if other.warp_triggered.is_some() {
            self.warp_triggered = other.warp_triggered;
        }
        if other.npc_spoken.is_some() {
            self.npc_spoken = other.npc_spoken;
        }
        self.party_healed |= other.party_healed;
        self.battle_started |= other.battle_started;
        self.wild_encounter |= other.wild_encounter;
        self.creature_caught |= other.creature_caught;
        self.creature_leveled_up |= other.creature_leveled_up;
        self.battle_ended |= other.battle_ended;
        self.victory |= other.victory;
        self.fled |= other.fled;
    }
}

/// Output of `step_rpg_with_clock`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockRpgResult {
    /// Number of fixed-timestep simulation steps executed.
    pub steps_run: u32,
    /// Fixed-point interpolation alpha `0..=65536`.
    pub alpha_q16: u32,
    /// Aggregated frame events across all executed steps.
    pub events: RpgFrameEvents,
}

// ---------------------------------------------------------------------------
// Main Game State
// ---------------------------------------------------------------------------

/// Complete state machine of the Monster RPG game.
#[derive(Debug, Clone)]
pub struct RpgGame {
    /// Active configuration.
    pub config: RpgConfig,
    /// Current mode of operation.
    pub mode: GameMode,
    /// ID of active map `0..=4`.
    pub current_map: u8,
    /// Player grid X coordinate `0..=15`.
    pub player_x: u8,
    /// Player grid Y coordinate `0..=15`.
    pub player_y: u8,
    /// Player facing orientation.
    pub player_facing: Facing,
    /// True if currently executing a grid transition.
    pub player_moving: bool,
    /// Remaining ticks for current tile step.
    pub move_ticks: u8,
    /// Player's monster party slots.
    pub party: [Option<Creature>; MAX_PARTY],
    /// Number of creatures in party `1..=6`.
    party_len: usize,
    /// Active turn-based battle, if in battle mode.
    pub battle: Option<Battle>,
    /// Enemy creature for wild battle capture tracking.
    pub wild_enemy: Option<Creature>,
    /// Active dialogue text, if in dialogue mode.
    pub dialogue_text: &'static str,
    /// Deterministic pseudo-random seed.
    pub rng_seed: u64,
    /// Number of overworld steps taken.
    pub step_counter: u32,
    /// Audio sequencer state.
    pub audio_state: RpgAudioState,
    /// World NPCs.
    pub npcs: [RpgNpc; 4],
}

impl RpgGame {
    /// Get the active game mode.
    pub fn active_game_mode(&self) -> GameMode {
        self.mode
    }

    /// Current map identifier `0..=4`.
    pub fn current_map_id(&self) -> u8 {
        self.current_map
    }

    /// Player tile coordinates `(x, y)`.
    pub fn player_tile_pos(&self) -> (u8, u8) {
        (self.player_x, self.player_y)
    }

    /// Number of creatures currently in the player's party.
    pub fn party_len(&self) -> usize {
        self.party_len.min(MAX_PARTY)
    }

    /// Number of creatures currently in the player's party.
    pub fn party_count(&self) -> usize {
        self.party_len()
    }

    /// Immutable reference to creature at party slot.
    pub fn party_creature(&self, index: usize) -> Option<&Creature> {
        if index < self.party_len.min(MAX_PARTY) {
            self.party[index].as_ref()
        } else {
            None
        }
    }

    /// Fully restore all HP, PP, and status for every monster in the player's party.
    pub fn heal_entire_party(&mut self) {
        for slot in &mut self.party[..self.party_len.min(MAX_PARTY)] {
            if let Some(c) = slot.as_mut() {
                c.hp = c.max_hp;
                c.status = Status::None;
                c.sleep_turns = 0;
                for i in 0..c.move_count as usize {
                    c.pp[i] = c.moves[i].max_pp;
                }
            }
        }
    }

    /// Initiate wild battle transition with target species and level.
    pub fn trigger_wild_encounter(&mut self, species_id: u8, level: u8) -> Result<(), RpgError> {
        self.mode = GameMode::BattleTransitionIn {
            step: 0,
            wild: true,
            species: species_id,
            level,
            trainer: None,
        };
        Ok(())
    }

    /// Initiate trainer battle transition against the specified NPC.
    pub fn trigger_trainer_encounter(&mut self, trainer: RpgNpcKind) -> Result<(), RpgError> {
        self.mode = GameMode::BattleTransitionIn {
            step: 0,
            wild: false,
            species: 0,
            level: 0,
            trainer: Some(trainer),
        };
        Ok(())
    }

    /// Active battle instance, if currently battling.
    pub fn active_battle(&self) -> Option<&Battle> {
        self.battle.as_ref()
    }

    /// Whether dialogue window is active.
    pub fn is_dialogue_open(&self) -> bool {
        matches!(self.mode, GameMode::Dialogue)
    }

    /// Active dialogue message text.
    pub fn active_dialogue_text(&self) -> Option<&str> {
        if self.is_dialogue_open() {
            Some(self.dialogue_text)
        } else {
            None
        }
    }
}

/// Initialize a new Monster RPG game state.
pub fn init_rpg_game(config: RpgConfig, seed: u64) -> Result<RpgGame, RpgError> {
    config.validate_rpg_config()?;

    // Starter creature: Sparky at level 5
    let starter = create_monster(SPECIES_SPARKY, 5)?;
    let mut party = [None, None, None, None, None, None];
    party[0] = Some(starter);

    // Initial NPCs
    let npcs = [
        RpgNpc {
            map_id: MAP_TOWN as u8,
            x: 5,
            y: 6,
            facing: Facing::Down,
            kind: RpgNpcKind::Guide,
            defeated: false,
        },
        RpgNpc {
            map_id: MAP_HEALING_CENTER as u8,
            x: 7,
            y: 3,
            facing: Facing::Down,
            kind: RpgNpcKind::Nurse,
            defeated: false,
        },
        RpgNpc {
            map_id: MAP_ROUTE1 as u8,
            x: 5,
            y: 8,
            facing: Facing::Left,
            kind: RpgNpcKind::TrainerYoungster,
            defeated: false,
        },
        RpgNpc {
            map_id: MAP_GYM as u8,
            x: 7,
            y: 3,
            facing: Facing::Down,
            kind: RpgNpcKind::GymLeader,
            defeated: false,
        },
    ];

    Ok(RpgGame {
        config,
        mode: GameMode::Overworld,
        current_map: MAP_TOWN as u8,
        player_x: 7,
        player_y: 8,
        player_facing: Facing::Down,
        player_moving: false,
        move_ticks: 0,
        party,
        party_len: 1,
        battle: None,
        wild_enemy: None,
        dialogue_text: "",
        rng_seed: seed ^ 0x5bf0_3635_bfae_f32d,
        step_counter: 0,
        audio_state: RpgAudioState::init_audio_state(),
        npcs,
    })
}

// ---------------------------------------------------------------------------
// Simulation Step Logic
// ---------------------------------------------------------------------------

/// Step the Monster RPG simulation by a single frame. Zero heap allocation.
pub fn step_rpg_frame(
    game: &mut RpgGame,
    input: &RpgInput,
    sfx: &mut SfxBank<4>,
) -> Result<RpgFrameEvents, RpgError> {
    let mut events = RpgFrameEvents::default();

    // Advance audio sequencer
    game.audio_state.advance_audio_clock(sfx, game.mode);

    match game.mode {
        GameMode::Overworld => {
            // Check interaction button
            if input.action {
                let (target_x, target_y) = match game.player_facing {
                    Facing::Up => (game.player_x, game.player_y.saturating_sub(1)),
                    Facing::Down => (game.player_x, game.player_y.saturating_add(1)),
                    Facing::Left => (game.player_x.saturating_sub(1), game.player_y),
                    Facing::Right => (game.player_x.saturating_add(1), game.player_y),
                };

                // Check for NPC at target tile
                for npc in &mut game.npcs {
                    if npc.matches_tile_coord(game.current_map, target_x, target_y) {
                        events.npc_spoken = Some(npc.kind);
                        let _ = sfx.trigger_bank_preset(SfxPreset::PongPaddleHit);

                        match npc.kind {
                            RpgNpcKind::Guide => {
                                game.dialogue_text =
                                    "Welcome to Monster RPG! Beware the tall grass on Route 1.";
                                game.mode = GameMode::Dialogue;
                            }
                            RpgNpcKind::Nurse => {
                                game.heal_entire_party();
                                events.party_healed = true;
                                game.dialogue_text = "Welcome to the Healing Center! Your monsters are now fully restored!";
                                game.mode = GameMode::Dialogue;
                            }
                            RpgNpcKind::TrainerYoungster => {
                                if npc.defeated {
                                    game.dialogue_text = "My monsters need more training...";
                                    game.mode = GameMode::Dialogue;
                                } else {
                                    events.battle_started = true;
                                    game.trigger_trainer_encounter(RpgNpcKind::TrainerYoungster)?;
                                }
                            }
                            RpgNpcKind::GymLeader => {
                                if npc.defeated {
                                    game.dialogue_text =
                                        "You proved your strength! Take the Boulder Badge!";
                                    game.mode = GameMode::Dialogue;
                                } else {
                                    events.battle_started = true;
                                    game.trigger_trainer_encounter(RpgNpcKind::GymLeader)?;
                                }
                            }
                        }
                        return Ok(events);
                    }
                }
            }

            // Directional movement
            let target_dir = match input.walk {
                WalkDirection::Up => Some((Facing::Up, 0, -1)),
                WalkDirection::Down => Some((Facing::Down, 0, 1)),
                WalkDirection::Left => Some((Facing::Left, -1, 0)),
                WalkDirection::Right => Some((Facing::Right, 1, 0)),
                WalkDirection::None => None,
            };

            if let Some((facing, dx, dy)) = target_dir {
                game.player_facing = facing;

                let next_x = (game.player_x as i32 + dx).clamp(0, (MAP_WIDTH - 1) as i32) as u8;
                let next_y = (game.player_y as i32 + dy).clamp(0, (MAP_HEIGHT - 1) as i32) as u8;

                let attr = get_map_tile_attr(game.current_map, next_x, next_y);

                // Check collision with solid tiles and NPCs
                let is_solid = (attr & ATTR_SOLID) != 0;
                let mut npc_blocked = false;
                for npc in &game.npcs {
                    if npc.matches_tile_coord(game.current_map, next_x, next_y) {
                        npc_blocked = true;
                        break;
                    }
                }

                if !is_solid && !npc_blocked {
                    game.player_x = next_x;
                    game.player_y = next_y;
                    game.step_counter = game.step_counter.saturating_add(1);
                    events.stepped = true;

                    // Check warps
                    if (attr & ATTR_WARP) != 0 {
                        // Check warp table
                        let warp_dest = match (game.current_map as usize, next_x, next_y) {
                            (MAP_TOWN, 7, 0) => Some((MAP_ROUTE1 as u8, 7, 14)),
                            (MAP_TOWN, 7, 4) => Some((MAP_HEALING_CENTER as u8, 7, 14)),
                            (MAP_TOWN, 12, 4) => Some((MAP_GYM as u8, 7, 14)),
                            (MAP_ROUTE1, 7, 15) => Some((MAP_TOWN as u8, 7, 1)),
                            (MAP_ROUTE1, 7, 0) => Some((MAP_TALL_GRASS as u8, 7, 14)),
                            (MAP_TALL_GRASS, 7, 15) => Some((MAP_ROUTE1 as u8, 7, 1)),
                            (MAP_HEALING_CENTER, 7, 15) => Some((MAP_TOWN as u8, 7, 5)),
                            (MAP_GYM, 7, 15) => Some((MAP_TOWN as u8, 12, 5)),
                            _ => None,
                        };

                        if let Some((target_map, tx, ty)) = warp_dest {
                            game.current_map = target_map;
                            game.player_x = tx;
                            game.player_y = ty;
                            events.warp_triggered = Some(target_map);
                            let _ = sfx.trigger_bank_preset(SfxPreset::PongWallBounce);
                            return Ok(events);
                        }
                    }

                    // Check tall grass step encounters
                    if (attr & ATTR_TALL_GRASS) != 0 {
                        // PRNG roll for encounter
                        game.rng_seed = game
                            .rng_seed
                            .wrapping_mul(6364136223846793005)
                            .wrapping_add(1);
                        let roll = ((game.rng_seed >> 32) as u32) % 256;
                        if roll < game.config.encounter_rate {
                            events.battle_started = true;
                            events.wild_encounter = true;
                            let species_roll = (((game.rng_seed >> 16) as u8) % 3) + 1; // Pyrokid, Aquafin, Leafox
                            let lvl_roll = (((game.rng_seed >> 24) as u8) % 3) + 3; // level 3..5
                            game.trigger_wild_encounter(species_roll, lvl_roll)?;
                            let _ = sfx.trigger_bank_preset(SfxPreset::PongPaddleHit);
                            return Ok(events);
                        }
                    }
                } else {
                    // Collision bump
                    let _ = sfx.trigger_bank_preset(SfxPreset::PongWallBounce);
                }
            }
        }

        GameMode::Dialogue => {
            if input.action || input.cancel {
                game.mode = GameMode::Overworld;
                game.dialogue_text = "";
                let _ = sfx.trigger_bank_preset(SfxPreset::PongPaddleHit);
            }
        }

        GameMode::BattleTransitionIn {
            mut step,
            wild,
            species,
            level,
            trainer,
        } => {
            step = step.saturating_add(1);
            if step >= FADE_TOTAL_STEPS {
                // Initialize active Battle with stack-allocated rosters (zero-alloc)
                let first_creature = match game.party[0] {
                    Some(c) => c,
                    None => return Err(RpgError::PartyEmpty),
                };
                let mut p_roster = [first_creature; MAX_PARTY];
                let mut p_count = 0;
                for c in game.party[..game.party_len.min(MAX_PARTY)].iter().flatten() {
                    if p_count < MAX_PARTY {
                        p_roster[p_count] = *c;
                        p_count += 1;
                    }
                }

                let mut e_roster = [first_creature; MAX_PARTY];
                let mut e_count = 0;
                if wild {
                    let enemy = create_monster(species, level)?;
                    game.wild_enemy = Some(enemy);
                    e_roster[0] = enemy;
                    e_count = 1;
                } else if let Some(tr) = trainer {
                    let enemy = match tr {
                        RpgNpcKind::TrainerYoungster => create_monster(SPECIES_GRAVELING, 4)?,
                        RpgNpcKind::GymLeader => create_monster(SPECIES_GEOSTONE, 8)?,
                        _ => create_monster(SPECIES_GRAVELING, 5)?,
                    };
                    e_roster[0] = enemy;
                    e_count = 1;
                }

                let seed = game.rng_seed;
                let mut battle =
                    Battle::start(&p_roster[..p_count], &e_roster[..e_count], wild, seed)?;
                battle.begin()?;
                game.battle = Some(battle);
                game.mode = GameMode::Battle;
            } else {
                game.mode = GameMode::BattleTransitionIn {
                    step,
                    wild,
                    species,
                    level,
                    trainer,
                };
            }
        }

        GameMode::Battle => {
            if let Some(battle) = game.battle.as_mut() {
                match battle.phase() {
                    Phase::CommandSelect => {
                        if let Some(cmd) = input.battle_command {
                            let _ = sfx.trigger_bank_preset(SfxPreset::PongPaddleHit);
                            let enemy_cmd = battle.ai_choose();
                            let submit_res = battle.submit_turn(cmd, enemy_cmd);
                            if submit_res.is_ok() {
                                let _ = battle.resolve_turn();
                                let _ =
                                    sfx.trigger_bank_preset(SfxPreset::BreakoutBrickHit { row: 2 });
                            }

                            // Inspect event log for capture or flee
                            for ev in battle.event_log() {
                                match ev {
                                    Event::Captured => {
                                        events.creature_caught = true;
                                        events.victory = true;
                                        events.battle_ended = true;
                                        let current_len = game.party_len.min(MAX_PARTY);
                                        if current_len < MAX_PARTY {
                                            if let Some(caught) = game.wild_enemy {
                                                game.party[current_len] = Some(caught);
                                                game.party_len =
                                                    current_len.saturating_add(1).min(MAX_PARTY);
                                            }
                                        }
                                        game.mode = GameMode::BattleTransitionOut { step: 0 };
                                        return Ok(events);
                                    }
                                    Event::FleeSuccess => {
                                        events.fled = true;
                                        events.battle_ended = true;
                                        game.mode = GameMode::BattleTransitionOut { step: 0 };
                                        return Ok(events);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    Phase::Victory => {
                        events.victory = true;
                        events.battle_ended = true;

                        // Check if wild creature was captured
                        if battle.was_captured() {
                            events.creature_caught = true;
                            let current_len = game.party_len.min(MAX_PARTY);
                            if current_len < MAX_PARTY {
                                if let Some(caught) = game.wild_enemy {
                                    game.party[current_len] = Some(caught);
                                    game.party_len = current_len.saturating_add(1).min(MAX_PARTY);
                                }
                            }
                        }

                        // Copy updated creatures back to player party
                        for i in 0..game.party_len.min(MAX_PARTY) {
                            if let Some(c) = battle.party_member(Side::Player, i as u8) {
                                game.party[i] = Some(*c);
                            }
                        }

                        // Mark trainer defeated if trainer battle
                        if !battle.is_wild_battle() {
                            if game.current_map == MAP_ROUTE1 as u8 {
                                game.npcs[2].defeated = true;
                            } else if game.current_map == MAP_GYM as u8 {
                                game.npcs[3].defeated = true;
                            }
                        }

                        game.mode = GameMode::BattleTransitionOut { step: 0 };
                    }
                    Phase::Defeat => {
                        events.battle_ended = true;
                        let _ = sfx.trigger_bank_preset(SfxPreset::BreakoutLoseLife);
                        // Teleport back to Healing Center and restore
                        game.current_map = MAP_HEALING_CENTER as u8;
                        game.player_x = 7;
                        game.player_y = 14;
                        game.heal_entire_party();
                        events.party_healed = true;
                        game.mode = GameMode::BattleTransitionOut { step: 0 };
                    }
                    Phase::Fled => {
                        events.fled = true;
                        events.battle_ended = true;
                        game.mode = GameMode::BattleTransitionOut { step: 0 };
                    }
                    Phase::SwitchPrompt => {
                        // Automatically switch to first alive monster
                        for (i, slot) in game
                            .party
                            .iter()
                            .take(game.party_len.min(MAX_PARTY))
                            .enumerate()
                        {
                            if let Some(c) = slot {
                                if c.hp > 0 {
                                    let _ = battle.switch_in(i as u8);
                                    break;
                                }
                            }
                        }
                    }
                    Phase::Intro => {
                        battle.begin()?;
                    }
                    _ => {}
                }
            }
        }

        GameMode::BattleTransitionOut { mut step } => {
            step = step.saturating_add(1);
            if step >= FADE_TOTAL_STEPS {
                game.mode = GameMode::Overworld;
                game.battle = None;
                game.wild_enemy = None;
            } else {
                game.mode = GameMode::BattleTransitionOut { step };
            }
        }
    }

    Ok(events)
}

// ---------------------------------------------------------------------------
// Fixed-Timestep Clock Integration
// ---------------------------------------------------------------------------

/// Step the Monster RPG simulation using integer-nanosecond clock accumulation.
pub fn step_rpg_with_clock(
    clock: &mut GameClockState,
    delta_ns: u64,
    game: &mut RpgGame,
    input: &RpgInput,
    sfx: &mut SfxBank<4>,
) -> Result<ClockRpgResult, RpgError> {
    let clock_step = clock.advance(delta_ns)?;
    let mut combined_events = RpgFrameEvents::default();

    for _ in 0..clock_step.steps_to_run {
        let events = step_rpg_frame(game, input, sfx)?;
        combined_events.merge_frame_events(&events);
    }

    Ok(ClockRpgResult {
        steps_run: clock_step.steps_to_run,
        alpha_q16: clock_step.alpha_q16,
        events: combined_events,
    })
}

// ---------------------------------------------------------------------------
// Zero-Allocation Sprite Lowering
// ---------------------------------------------------------------------------

/// Generate OAM-style sprite quads for the current frame into pre-allocated storage.
///
/// Operates with **zero heap allocation**.
pub fn build_rpg_sprites(game: &RpgGame, out: &mut [SpriteQuad]) -> Result<usize, RpgError> {
    if out.len() < 8 {
        return Err(RpgError::BufferTooSmall);
    }

    let mut count = 0;

    match game.mode {
        GameMode::Overworld | GameMode::Dialogue => {
            // Player sprite at center/screen position
            let px = (game.player_x as f64) * (TILE_SIZE as f64);
            let py = (game.player_y as f64) * (TILE_SIZE as f64);
            out[count] = SpriteQuad::new(1, px, py, TILE_SIZE as f64, TILE_SIZE as f64);
            count += 1;

            // NPCs on current map
            for npc in &game.npcs {
                if npc.map_id == game.current_map && count < out.len() {
                    let nx = (npc.x as f64) * (TILE_SIZE as f64);
                    let ny = (npc.y as f64) * (TILE_SIZE as f64);
                    out[count] = SpriteQuad::new(2, nx, ny, TILE_SIZE as f64, TILE_SIZE as f64);
                    count += 1;
                }
            }
        }
        GameMode::Battle => {
            // Player monster back sprite at (32, 70)
            out[count] = SpriteQuad::new(10, 32.0, 70.0, 48.0, 48.0);
            count += 1;

            // Enemy monster front sprite at (160, 20)
            out[count] = SpriteQuad::new(20, 160.0, 20.0, 48.0, 48.0);
            count += 1;

            if let Some(battle) = game.battle.as_ref() {
                // Player HP bar at (120, 95)
                let player_c = battle.active_creature(Side::Player);
                let p_bar_w = hp_bar_pixels(player_c.hp, player_c.max_hp, 48);
                out[count] = SpriteQuad::new(30, 120.0, 95.0, p_bar_w as f64, 6.0);
                count += 1;

                // Enemy HP bar at (20, 25)
                let enemy_c = battle.active_creature(Side::Enemy);
                let e_bar_w = hp_bar_pixels(enemy_c.hp, enemy_c.max_hp, 48);
                out[count] = SpriteQuad::new(31, 20.0, 25.0, e_bar_w as f64, 6.0);
                count += 1;
            }
        }
        _ => {}
    }

    Ok(count)
}

// ---------------------------------------------------------------------------
// PaintScene IR Lowering
// ---------------------------------------------------------------------------

/// Lower the complete game visual state into universal `PaintScene` IR.
pub fn lower_rpg_to_paint_scene(game: &RpgGame) -> Result<PaintScene, RpgError> {
    let mut sprites = [SpriteQuad::new(0, 0.0, 0.0, 0.0, 0.0); MAX_RPG_SPRITES];
    let sprite_count = build_rpg_sprites(game, &mut sprites)?;

    let bg_color = match game.mode {
        GameMode::Overworld => "#58a058",
        GameMode::Dialogue => "#489048",
        GameMode::Battle => "#e0e8c8",
        GameMode::BattleTransitionIn { .. } | GameMode::BattleTransitionOut { .. } => "#101010",
    };

    let scene = lower_sprites_to_paint_scene(
        game.config.screen_width as f64,
        game.config.screen_height as f64,
        bg_color,
        &sprites[..sprite_count],
    )?;

    Ok(scene)
}

// ---------------------------------------------------------------------------
// Audio Rendering Helper
// ---------------------------------------------------------------------------

/// Mix and stream synthesized chiptune audio into the real-time PCM ring buffer.
///
/// Operates with **zero heap allocation**.
pub fn render_rpg_audio_into<const CAP: usize>(
    sfx: &mut SfxBank<4>,
    ring: &mut PcmRingBuffer<CAP>,
    scratch: &mut [i16],
) -> Result<usize, RpgError> {
    if scratch.is_empty() {
        return Ok(0);
    }
    let samples = arcade_sfx::mix_bank_into(sfx, scratch)?;
    if samples > 0 {
        let _ = ring.push_slice(&scratch[..samples]);
    }
    Ok(samples)
}
