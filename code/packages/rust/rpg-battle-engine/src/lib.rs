//! # rpg-battle-engine
//!
//! GAME24: a deterministic, zero-allocation, turn-based creature battle state
//! machine in the style of 8/16-bit console RPGs.
//!
//! ## What is inside
//!
//! * A 15-type elemental effectiveness chart ([`TYPE_CHART`]) with `0x`, `0.5x`,
//!   `1x` and `2x` single-type multipliers, combined for dual-typed defenders
//!   into a fixed-point multiplier where [`MULT_ONE`] (16) means `1x`
//!   (so `4x` is 64 and `0.25x` is 4).
//! * Creature stats derived with integer math, stage modifiers clamped to
//!   `-6..=+6` (Atk / Def / SpA / SpD / Spe / Acc / Eva), status conditions
//!   (`Burn`, `Paralysis`, `Poison`, `Sleep`, `Freeze`) and a level-scaled
//!   integer damage formula with STAB, critical hits and a deterministic
//!   `85..=100` damage roll ([`calc_damage`]).
//! * A battle finite-state machine ([`Battle`], [`Phase`]): `Intro`,
//!   `CommandSelect`, `TurnResolution`, `SwitchPrompt`, `Victory`, `Defeat`
//!   and `Fled`, with priority / speed turn ordering, PP accounting,
//!   Struggle, capture ([`attempt_capture`]), fleeing ([`flee_succeeds`]) and
//!   XP / level-up progression ([`xp_for_level`], [`xp_yield`]).
//! * HUD helpers ([`hp_bar_state`], [`hp_bar_pixels`], [`hp_bar_color`],
//!   [`hp_bar_blend_color`], [`fade_trigger_for_phase`]) that speak the
//!   `palette-indexed` (`GAME20`) colour and fade vocabulary.
//!
//! ## Safety and resource model
//!
//! * `#![forbid(unsafe_code)]`; no pointer arithmetic anywhere.
//! * All state lives in fixed-size `Copy` values: the hot battle-step and
//!   damage paths perform **zero heap allocation**. Turn events are written
//!   into a fixed-capacity log ([`MAX_EVENTS`]); overflow is counted, never
//!   grown (CWE-400).
//! * Every party / move / type index is validated before use (CWE-20 / 125 /
//!   787); stages are clamped; HP, PP and XP arithmetic saturates (CWE-190);
//!   every divisor is guarded (CWE-369).
//! * Determinism: all randomness comes from the seeded [`BattleRng`] (PCG32
//!   core), so a `(seed, command stream)` pair always replays identically.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Generated capability verification witnesses.
pub mod capabilities;

use palette_indexed::{fade_color, ChannelBits};

pub use palette_indexed::Rgba8;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of elemental types in the chart.
pub const TYPE_COUNT: usize = 15;
/// Sentinel for "no second type".
pub const NO_TYPE: u8 = 255;
/// Fixed-point `1x` effectiveness multiplier (`2x` = 32, `0.5x` = 8, `4x` = 64).
pub const MULT_ONE: u16 = 16;
/// Largest party size per side.
pub const MAX_PARTY: usize = 6;
/// Moves per creature.
pub const MAX_MOVES: usize = 4;
/// Highest creature level.
pub const MAX_LEVEL: u8 = 100;
/// Lowest stat stage.
pub const MIN_STAGE: i8 = -6;
/// Highest stat stage.
pub const MAX_STAGE: i8 = 6;
/// Number of stage-modified quantities (Atk, Def, SpA, SpD, Spe, Acc, Eva).
pub const STAGE_COUNT: usize = 7;
/// Capacity of the per-turn event log.
pub const MAX_EVENTS: usize = 40;
/// Highest move priority magnitude.
pub const MAX_PRIORITY: i8 = 6;
/// Largest accepted capture-ball bonus (fixed point, 10 = `1.0x`).
pub const MAX_BALL_BONUS_X10: u16 = 1000;

/// Normal type id.
pub const NORMAL: u8 = 0;
/// Fire type id.
pub const FIRE: u8 = 1;
/// Water type id.
pub const WATER: u8 = 2;
/// Electric type id.
pub const ELECTRIC: u8 = 3;
/// Grass type id.
pub const GRASS: u8 = 4;
/// Ice type id.
pub const ICE: u8 = 5;
/// Fighting type id.
pub const FIGHTING: u8 = 6;
/// Poison type id.
pub const POISON: u8 = 7;
/// Ground type id.
pub const GROUND: u8 = 8;
/// Flying type id.
pub const FLYING: u8 = 9;
/// Psychic type id.
pub const PSYCHIC: u8 = 10;
/// Bug type id.
pub const BUG: u8 = 11;
/// Rock type id.
pub const ROCK: u8 = 12;
/// Ghost type id.
pub const GHOST: u8 = 13;
/// Dragon type id.
pub const DRAGON: u8 = 14;

/// Chart entry: no effect (`0x`).
pub const EFF_IMMUNE: u8 = 0;
/// Chart entry: not very effective (`0.5x`).
pub const EFF_HALF: u8 = 2;
/// Chart entry: neutral (`1x`).
pub const EFF_NEUTRAL: u8 = 4;
/// Chart entry: super effective (`2x`).
pub const EFF_DOUBLE: u8 = 8;

// ---------------------------------------------------------------------------
// Type chart
// ---------------------------------------------------------------------------

/// Attacker-row by defender-column effectiveness chart (quarter units, `4` = 1x).
pub type TypeChart = [[u8; TYPE_COUNT]; TYPE_COUNT];

/// Non-neutral chart entries: `(attacker, defender, effectiveness)`.
const CHART_OVERRIDES: [(u8, u8, u8); 79] = [
    (NORMAL, ROCK, EFF_HALF),
    (NORMAL, GHOST, EFF_IMMUNE),
    (FIRE, FIRE, EFF_HALF),
    (FIRE, WATER, EFF_HALF),
    (FIRE, GRASS, EFF_DOUBLE),
    (FIRE, ICE, EFF_DOUBLE),
    (FIRE, BUG, EFF_DOUBLE),
    (FIRE, ROCK, EFF_HALF),
    (FIRE, DRAGON, EFF_HALF),
    (WATER, FIRE, EFF_DOUBLE),
    (WATER, WATER, EFF_HALF),
    (WATER, GRASS, EFF_HALF),
    (WATER, GROUND, EFF_DOUBLE),
    (WATER, ROCK, EFF_DOUBLE),
    (WATER, DRAGON, EFF_HALF),
    (ELECTRIC, WATER, EFF_DOUBLE),
    (ELECTRIC, ELECTRIC, EFF_HALF),
    (ELECTRIC, GRASS, EFF_HALF),
    (ELECTRIC, GROUND, EFF_IMMUNE),
    (ELECTRIC, FLYING, EFF_DOUBLE),
    (ELECTRIC, DRAGON, EFF_HALF),
    (GRASS, FIRE, EFF_HALF),
    (GRASS, WATER, EFF_DOUBLE),
    (GRASS, GRASS, EFF_HALF),
    (GRASS, POISON, EFF_HALF),
    (GRASS, GROUND, EFF_DOUBLE),
    (GRASS, FLYING, EFF_HALF),
    (GRASS, BUG, EFF_HALF),
    (GRASS, ROCK, EFF_DOUBLE),
    (GRASS, DRAGON, EFF_HALF),
    (ICE, FIRE, EFF_HALF),
    (ICE, WATER, EFF_HALF),
    (ICE, GRASS, EFF_DOUBLE),
    (ICE, ICE, EFF_HALF),
    (ICE, GROUND, EFF_DOUBLE),
    (ICE, FLYING, EFF_DOUBLE),
    (ICE, DRAGON, EFF_DOUBLE),
    (FIGHTING, NORMAL, EFF_DOUBLE),
    (FIGHTING, ICE, EFF_DOUBLE),
    (FIGHTING, POISON, EFF_HALF),
    (FIGHTING, FLYING, EFF_HALF),
    (FIGHTING, PSYCHIC, EFF_HALF),
    (FIGHTING, BUG, EFF_HALF),
    (FIGHTING, ROCK, EFF_DOUBLE),
    (FIGHTING, GHOST, EFF_IMMUNE),
    (POISON, GRASS, EFF_DOUBLE),
    (POISON, POISON, EFF_HALF),
    (POISON, GROUND, EFF_HALF),
    (POISON, ROCK, EFF_HALF),
    (POISON, GHOST, EFF_HALF),
    (GROUND, FIRE, EFF_DOUBLE),
    (GROUND, ELECTRIC, EFF_DOUBLE),
    (GROUND, GRASS, EFF_HALF),
    (GROUND, POISON, EFF_DOUBLE),
    (GROUND, FLYING, EFF_IMMUNE),
    (GROUND, BUG, EFF_HALF),
    (GROUND, ROCK, EFF_DOUBLE),
    (FLYING, ELECTRIC, EFF_HALF),
    (FLYING, GRASS, EFF_DOUBLE),
    (FLYING, FIGHTING, EFF_DOUBLE),
    (FLYING, BUG, EFF_DOUBLE),
    (FLYING, ROCK, EFF_HALF),
    (PSYCHIC, FIGHTING, EFF_DOUBLE),
    (PSYCHIC, POISON, EFF_DOUBLE),
    (PSYCHIC, PSYCHIC, EFF_HALF),
    (BUG, FIRE, EFF_HALF),
    (BUG, GRASS, EFF_DOUBLE),
    (BUG, FIGHTING, EFF_HALF),
    (BUG, POISON, EFF_HALF),
    (BUG, FLYING, EFF_HALF),
    (BUG, PSYCHIC, EFF_DOUBLE),
    (BUG, GHOST, EFF_HALF),
    (ROCK, FIRE, EFF_DOUBLE),
    (ROCK, ICE, EFF_DOUBLE),
    (ROCK, FIGHTING, EFF_HALF),
    (ROCK, GROUND, EFF_HALF),
    (ROCK, FLYING, EFF_DOUBLE),
    (ROCK, BUG, EFF_DOUBLE),
    (GHOST, NORMAL, EFF_IMMUNE),
];

/// Second batch of non-neutral entries (kept separate for readability).
const CHART_OVERRIDES_B: [(u8, u8, u8); 3] = [
    (GHOST, PSYCHIC, EFF_DOUBLE),
    (GHOST, GHOST, EFF_DOUBLE),
    (DRAGON, DRAGON, EFF_DOUBLE),
];

const fn build_chart() -> TypeChart {
    let mut chart = [[EFF_NEUTRAL; TYPE_COUNT]; TYPE_COUNT];
    let mut i = 0;
    while i < CHART_OVERRIDES.len() {
        let (a, d, e) = CHART_OVERRIDES[i];
        chart[a as usize][d as usize] = e;
        i += 1;
    }
    let mut j = 0;
    while j < CHART_OVERRIDES_B.len() {
        let (a, d, e) = CHART_OVERRIDES_B[j];
        chart[a as usize][d as usize] = e;
        j += 1;
    }
    chart
}

/// The elemental effectiveness chart, indexed `[attacker][defender]`.
pub static TYPE_CHART: TypeChart = build_chart();

// ---------------------------------------------------------------------------
// Errors and small enums
// ---------------------------------------------------------------------------

/// Failure modes of every fallible operation in this crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleError {
    /// A type id is outside the chart.
    InvalidType,
    /// A level is outside `1..=100`.
    InvalidLevel,
    /// A base stat is zero.
    InvalidStat,
    /// A move definition is out of range (accuracy, PP, priority, effect).
    InvalidMove,
    /// A creature violates an internal invariant (HP, PP, stages, counts).
    InvalidCreature,
    /// A party is empty.
    PartyEmpty,
    /// A party exceeds [`MAX_PARTY`].
    PartyTooLarge,
    /// The party composition is not allowed (e.g. wild battle with several foes).
    InvalidParty,
    /// Every creature in a party is fainted.
    AllFainted,
    /// The operation is not valid in the current battle phase.
    InvalidPhase,
    /// A move slot is out of range.
    InvalidMoveSlot,
    /// A party index is out of range or names an unusable creature.
    InvalidPartyIndex,
    /// The selected move has no PP left.
    NoPp,
    /// Capture is only possible against wild creatures with a sane ball bonus.
    CannotCatch,
    /// Fleeing is only possible from wild battles.
    CannotFlee,
    /// The command is not available to this side.
    InvalidCommand,
}

/// Persistent status conditions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// No status.
    None,
    /// Burn: halves physical damage dealt, chip damage each turn.
    Burn,
    /// Paralysis: quarter speed, may be fully paralysed.
    Paralysis,
    /// Poison: chip damage each turn.
    Poison,
    /// Sleep: skip turns until waking.
    Sleep,
    /// Freeze: cannot act until thawed.
    Freeze,
}

/// Move damage category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    /// Uses Atk / Def.
    Physical,
    /// Uses SpA / SpD.
    Special,
    /// Deals no direct damage.
    Status,
}

/// A stage-modified quantity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatIndex {
    /// Attack.
    Atk = 0,
    /// Defense.
    Def = 1,
    /// Special attack.
    SpA = 2,
    /// Special defense.
    SpD = 3,
    /// Speed.
    Spe = 4,
    /// Accuracy.
    Acc = 5,
    /// Evasion.
    Eva = 6,
}

impl StatIndex {
    /// Position of this quantity in a [`StageArray`].
    pub const fn slot(self) -> usize {
        self as usize
    }
}

/// Battle side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// The player's side.
    Player,
    /// The opposing side.
    Enemy,
}

impl Side {
    /// The other side.
    pub const fn foe(self) -> Side {
        match self {
            Side::Player => Side::Enemy,
            Side::Enemy => Side::Player,
        }
    }
}

/// Secondary effect attached to a move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// No secondary effect.
    NoEffect,
    /// Inflict a status on the target with `chance` percent.
    Inflict {
        /// Status to inflict.
        status: Status,
        /// Percent chance `0..=100`.
        chance: u8,
    },
    /// Change a stage with `chance` percent.
    Stage {
        /// Quantity to change.
        stat: StatIndex,
        /// Signed stage delta `-6..=6`.
        delta: i8,
        /// `true` to affect the user, `false` to affect the target.
        on_self: bool,
        /// Percent chance `0..=100`.
        chance: u8,
    },
}

/// A creature move definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Move {
    /// Elemental type id.
    pub move_type: u8,
    /// Damage category.
    pub category: Category,
    /// Base power (`0` for pure status moves).
    pub power: u8,
    /// Accuracy percent `1..=100`, or `0` for "never misses".
    pub accuracy: u8,
    /// Maximum PP (`>= 1`).
    pub max_pp: u8,
    /// Priority bracket `-6..=6`.
    pub priority: i8,
    /// Secondary effect.
    pub effect: Effect,
}

/// The move used when every PP is spent: typeless-normal, 50 power, 25% recoil.
pub const STRUGGLE_MOVE: Move = Move {
    move_type: NORMAL,
    category: Category::Physical,
    power: 50,
    accuracy: 0,
    max_pp: 1,
    priority: 0,
    effect: Effect::NoEffect,
};

impl Move {
    /// Validate every field of the move (CWE-20).
    pub fn check_move(&self) -> Result<(), BattleError> {
        if (self.move_type as usize) >= TYPE_COUNT {
            return Err(BattleError::InvalidType);
        }
        if self.accuracy > 100 || self.max_pp == 0 {
            return Err(BattleError::InvalidMove);
        }
        if !(-MAX_PRIORITY..=MAX_PRIORITY).contains(&self.priority) {
            return Err(BattleError::InvalidMove);
        }
        match self.effect {
            Effect::NoEffect => Ok(()),
            Effect::Inflict { status, chance } => {
                if chance > 100 || status == Status::None {
                    Err(BattleError::InvalidMove)
                } else {
                    Ok(())
                }
            }
            Effect::Stage { delta, chance, .. } => {
                if chance > 100 || !(MIN_STAGE..=MAX_STAGE).contains(&delta) {
                    Err(BattleError::InvalidMove)
                } else {
                    Ok(())
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Fixed-point helpers
// ---------------------------------------------------------------------------

/// Chart effectiveness (quarter units, `4` = 1x) of `attack` against `defend`.
pub fn type_effectiveness(attack: u8, defend: u8) -> Result<u8, BattleError> {
    match TYPE_CHART
        .get(attack as usize)
        .and_then(|row| row.get(defend as usize))
    {
        Some(e) => Ok(*e),
        None => Err(BattleError::InvalidType),
    }
}

/// Combined multiplier of `attack` against a defender with types `t1` and
/// `t2` (`t2` may be [`NO_TYPE`]), in units where [`MULT_ONE`] = `1x`.
pub fn dual_multiplier(attack: u8, t1: u8, t2: u8) -> Result<u16, BattleError> {
    let e1 = type_effectiveness(attack, t1)? as u16;
    let e2 = if t2 == NO_TYPE {
        EFF_NEUTRAL as u16
    } else {
        type_effectiveness(attack, t2)? as u16
    };
    Ok(e1 * e2)
}

/// Clamp any integer to the legal stage range `-6..=+6`.
pub const fn clamp_stage(stage: i32) -> i8 {
    if stage < MIN_STAGE as i32 {
        MIN_STAGE
    } else if stage > MAX_STAGE as i32 {
        MAX_STAGE
    } else {
        stage as i8
    }
}

/// `(numerator, denominator)` of the battle-stat multiplier for `stage`.
pub const fn stage_ratio(stage: i32) -> (u32, u32) {
    let s = clamp_stage(stage) as i32;
    if s >= 0 {
        (2 + s as u32, 2)
    } else {
        (2, 2 + (-s) as u32)
    }
}

/// `(numerator, denominator)` of the accuracy/evasion multiplier for `stage`.
pub const fn accuracy_ratio(stage: i32) -> (u32, u32) {
    let s = clamp_stage(stage) as i32;
    if s >= 0 {
        (3 + s as u32, 3)
    } else {
        (3, 3 + (-s) as u32)
    }
}

/// Derived maximum HP for a base stat and level.
pub const fn derive_hp(base: u8, level: u8) -> u16 {
    ((2 * base as u32 * level as u32) / 100 + level as u32 + 10) as u16
}

/// Derived non-HP stat for a base stat and level.
pub const fn derive_stat(base: u8, level: u8) -> u16 {
    ((2 * base as u32 * level as u32) / 100 + 5) as u16
}

/// Stat after applying a stage modifier (never below 1).
pub const fn effective_stat(raw: u16, stage: i32) -> u32 {
    let (n, d) = stage_ratio(stage);
    let v = raw as u32 * n / d;
    if v == 0 {
        1
    } else {
        v
    }
}

/// Total XP required to reach `level` (medium-fast growth: `level^3`).
pub const fn xp_for_level(level: u8) -> u32 {
    let l = if level < 1 {
        1
    } else if level > MAX_LEVEL {
        MAX_LEVEL
    } else {
        level
    } as u32;
    l * l * l
}

/// XP awarded for defeating a creature.
pub const fn xp_yield(exp_yield: u16, enemy_level: u8, is_wild: bool) -> u32 {
    let base = exp_yield as u64 * enemy_level as u64;
    let v = if is_wild { base / 7 } else { base * 3 / 14 };
    if v > u32::MAX as u64 {
        u32::MAX
    } else {
        v as u32
    }
}

/// Integer square root (floor), bounded bit-by-bit method.
pub const fn isqrt_u32(n: u32) -> u32 {
    let mut rem = n;
    let mut root = 0u32;
    let mut bit = 1u32 << 30;
    while bit > rem {
        bit >>= 2;
    }
    while bit != 0 {
        if rem >= root + bit {
            rem -= root + bit;
            root = (root >> 1) + bit;
        } else {
            root >>= 1;
        }
        bit >>= 2;
    }
    root
}

// ---------------------------------------------------------------------------
// Deterministic PRNG
// ---------------------------------------------------------------------------

const PCG_MULT: u64 = 6_364_136_223_846_793_005;
const PCG_INC: u64 = 1_442_695_040_888_963_407;

/// Seeded PCG32 generator; `Copy`, allocation-free and fully deterministic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BattleRng {
    state: u64,
}

impl BattleRng {
    /// Create a generator from `seed`.
    pub fn seeded(seed: u64) -> BattleRng {
        let mut r = BattleRng { state: 0 };
        r.next_word();
        r.state = r.state.wrapping_add(seed);
        r.next_word();
        r
    }

    /// Next 32 random bits.
    pub fn next_word(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(PCG_MULT).wrapping_add(PCG_INC);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Uniform value in `0..n` (returns `0` when `n == 0`, CWE-369).
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        ((self.next_word() as u64 * n as u64) >> 32) as u32
    }
}

// ---------------------------------------------------------------------------
// Creatures
// ---------------------------------------------------------------------------

/// Per-quantity stage array, ordered as [`StatIndex`].
pub type StageArray = [i8; STAGE_COUNT];
/// A creature's move list.
pub type MoveSet = [Move; MAX_MOVES];
/// Per-move remaining PP.
pub type PpArray = [u8; MAX_MOVES];
/// A fixed-size party storage.
pub type Roster = [Creature; MAX_PARTY];
/// Turn-resolution order of the two sides.
pub type SideOrder = [Side; 2];
/// Fixed-capacity per-turn event log storage.
pub type EventLog = [Event; MAX_EVENTS];

/// Species base stats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BaseStats {
    /// Base HP.
    pub hp: u8,
    /// Base attack.
    pub atk: u8,
    /// Base defense.
    pub def: u8,
    /// Base special attack.
    pub spa: u8,
    /// Base special defense.
    pub spd: u8,
    /// Base speed.
    pub spe: u8,
}

/// Derived battle stats at the creature's current level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatStats {
    /// Attack.
    pub atk: u16,
    /// Defense.
    pub def: u16,
    /// Special attack.
    pub spa: u16,
    /// Special defense.
    pub spd: u16,
    /// Speed.
    pub spe: u16,
}

/// Construction parameters for a [`Creature`].
#[derive(Clone, Copy, Debug)]
pub struct CreatureSpec<'a> {
    /// Primary type id.
    pub type1: u8,
    /// Secondary type id or [`NO_TYPE`].
    pub type2: u8,
    /// Level `1..=100`.
    pub level: u8,
    /// Species base stats (all `>= 1`).
    pub base: BaseStats,
    /// One to four moves.
    pub moves: &'a [Move],
    /// Capture rate `1..=255`.
    pub catch_rate: u8,
    /// Base experience yield when defeated.
    pub exp_yield: u16,
}

/// A battle creature. All state is plain `Copy` data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Creature {
    /// Primary type id.
    pub type1: u8,
    /// Secondary type id or [`NO_TYPE`].
    pub type2: u8,
    /// Level `1..=100`.
    pub level: u8,
    /// Species base stats.
    pub base: BaseStats,
    /// Maximum HP.
    pub max_hp: u16,
    /// Current HP (`<= max_hp`).
    pub hp: u16,
    /// Derived stats.
    pub stats: CombatStats,
    /// Status condition.
    pub status: Status,
    /// Remaining sleep turns while asleep.
    pub sleep_turns: u8,
    /// Stat stages, each `-6..=6`.
    pub stages: StageArray,
    /// Move list (first `move_count` entries are live).
    pub moves: MoveSet,
    /// Number of live moves `1..=4`.
    pub move_count: u8,
    /// Remaining PP per move.
    pub pp: PpArray,
    /// Total experience points.
    pub xp: u32,
    /// Capture rate `1..=255`.
    pub catch_rate: u8,
    /// Base experience yield.
    pub exp_yield: u16,
}

fn derive_stats(base: &BaseStats, level: u8) -> CombatStats {
    CombatStats {
        atk: derive_stat(base.atk, level),
        def: derive_stat(base.def, level),
        spa: derive_stat(base.spa, level),
        spd: derive_stat(base.spd, level),
        spe: derive_stat(base.spe, level),
    }
}

impl Creature {
    /// Build a validated creature at full HP and full PP.
    pub fn from_spec(spec: &CreatureSpec<'_>) -> Result<Creature, BattleError> {
        if (spec.type1 as usize) >= TYPE_COUNT {
            return Err(BattleError::InvalidType);
        }
        if spec.type2 != NO_TYPE && (spec.type2 as usize) >= TYPE_COUNT {
            return Err(BattleError::InvalidType);
        }
        if spec.level < 1 || spec.level > MAX_LEVEL {
            return Err(BattleError::InvalidLevel);
        }
        let b = &spec.base;
        if b.hp == 0 || b.atk == 0 || b.def == 0 || b.spa == 0 || b.spd == 0 || b.spe == 0 {
            return Err(BattleError::InvalidStat);
        }
        if spec.moves.is_empty() || spec.moves.len() > MAX_MOVES || spec.catch_rate == 0 {
            return Err(BattleError::InvalidMove);
        }
        let mut moves = [STRUGGLE_MOVE; MAX_MOVES];
        let mut pp = [0u8; MAX_MOVES];
        for (i, m) in spec.moves.iter().enumerate() {
            m.check_move()?;
            if let (Some(ms), Some(ps)) = (moves.get_mut(i), pp.get_mut(i)) {
                *ms = *m;
                *ps = m.max_pp;
            }
        }
        let max_hp = derive_hp(b.hp, spec.level);
        Ok(Creature {
            type1: spec.type1,
            type2: spec.type2,
            level: spec.level,
            base: spec.base,
            max_hp,
            hp: max_hp,
            stats: derive_stats(&spec.base, spec.level),
            status: Status::None,
            sleep_turns: 0,
            stages: [0; STAGE_COUNT],
            moves,
            move_count: spec.moves.len() as u8,
            pp,
            xp: xp_for_level(spec.level),
            catch_rate: spec.catch_rate,
            exp_yield: spec.exp_yield,
        })
    }

    /// Verify every invariant (guards against tampered public fields).
    pub fn check_integrity(&self) -> Result<(), BattleError> {
        if (self.type1 as usize) >= TYPE_COUNT
            || (self.type2 != NO_TYPE && (self.type2 as usize) >= TYPE_COUNT)
        {
            return Err(BattleError::InvalidType);
        }
        if self.level < 1 || self.level > MAX_LEVEL {
            return Err(BattleError::InvalidLevel);
        }
        if self.max_hp == 0 || self.hp > self.max_hp {
            return Err(BattleError::InvalidCreature);
        }
        if self.move_count == 0 || (self.move_count as usize) > MAX_MOVES || self.catch_rate == 0 {
            return Err(BattleError::InvalidCreature);
        }
        if self
            .stages
            .iter()
            .any(|s| !(MIN_STAGE..=MAX_STAGE).contains(s))
        {
            return Err(BattleError::InvalidCreature);
        }
        let s = &self.stats;
        if s.atk == 0 || s.def == 0 || s.spa == 0 || s.spd == 0 || s.spe == 0 {
            return Err(BattleError::InvalidStat);
        }
        for (m, pp) in self
            .moves
            .iter()
            .zip(self.pp.iter())
            .take(self.move_count as usize)
        {
            m.check_move()?;
            if *pp > m.max_pp {
                return Err(BattleError::InvalidCreature);
            }
        }
        Ok(())
    }

    /// `true` when HP is zero.
    pub const fn is_fainted(&self) -> bool {
        self.hp == 0
    }

    /// `true` when no live move has PP left (Struggle applies).
    pub fn all_pp_spent(&self) -> bool {
        !self
            .pp
            .iter()
            .take(self.move_count as usize)
            .any(|p| *p > 0)
    }

    /// Current stage of `stat`.
    pub fn stage_of(&self, stat: StatIndex) -> i8 {
        match self.stages.get(stat.slot()) {
            Some(s) => clamp_stage(*s as i32),
            None => 0,
        }
    }

    /// Add experience, applying level-ups. Returns the number of levels gained.
    pub fn grant_xp(&mut self, amount: u32) -> u8 {
        let cap = xp_for_level(MAX_LEVEL);
        self.xp = self.xp.saturating_add(amount).min(cap);
        let mut gained = 0u8;
        while self.level < MAX_LEVEL && self.xp >= xp_for_level(self.level + 1) {
            self.level += 1;
            gained += 1;
            let new_max = derive_hp(self.base.hp, self.level);
            let delta = new_max.saturating_sub(self.max_hp);
            self.max_hp = new_max.max(self.max_hp);
            if self.hp > 0 {
                self.hp = self.hp.saturating_add(delta).min(self.max_hp);
            }
            self.stats = derive_stats(&self.base, self.level);
        }
        gained
    }
}

// ---------------------------------------------------------------------------
// Damage
// ---------------------------------------------------------------------------

/// Result of a damage calculation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DamageResult {
    /// HP to remove (saturated to `u16`).
    pub damage: u16,
    /// Type multiplier in [`MULT_ONE`] units.
    pub multiplier: u16,
    /// Whether the hit was critical.
    pub crit: bool,
    /// Whether same-type attack bonus applied.
    pub stab: bool,
}

/// Level-scaled integer damage formula.
///
/// `roll` is the random factor and is clamped to `85..=100` (percent).
/// Status moves and zero-power moves deal no damage; type immunity deals none.
pub fn calc_damage(
    attacker: &Creature,
    defender: &Creature,
    mv: &Move,
    roll: u32,
    crit: bool,
) -> DamageResult {
    let multiplier =
        dual_multiplier(mv.move_type, defender.type1, defender.type2).unwrap_or(MULT_ONE);
    let stab = mv.move_type == attacker.type1 || mv.move_type == attacker.type2;
    let none = DamageResult {
        damage: 0,
        multiplier,
        crit,
        stab,
    };
    if mv.power == 0 || mv.category == Category::Status || multiplier == 0 {
        return none;
    }
    let (a_raw, a_idx, d_raw, d_idx, physical) = match mv.category {
        Category::Physical => (
            attacker.stats.atk,
            StatIndex::Atk,
            defender.stats.def,
            StatIndex::Def,
            true,
        ),
        _ => (
            attacker.stats.spa,
            StatIndex::SpA,
            defender.stats.spd,
            StatIndex::SpD,
            false,
        ),
    };
    let mut a_stage = attacker.stage_of(a_idx) as i32;
    let mut d_stage = defender.stage_of(d_idx) as i32;
    if crit {
        a_stage = a_stage.max(0);
        d_stage = d_stage.min(0);
    }
    let a = effective_stat(a_raw, a_stage) as u64;
    let d = effective_stat(d_raw, d_stage) as u64;
    let level = attacker.level.clamp(1, MAX_LEVEL) as u64;
    let mut dmg = ((2 * level / 5 + 2) * mv.power as u64 * a / d) / 50 + 2;
    if physical && attacker.status == Status::Burn {
        dmg /= 2;
    }
    if crit {
        dmg *= 2;
    }
    dmg = dmg * roll.clamp(85, 100) as u64 / 100;
    if stab {
        dmg = dmg * 3 / 2;
    }
    dmg = dmg * multiplier as u64 / MULT_ONE as u64;
    let dmg = dmg.max(1).min(u16::MAX as u64) as u16;
    DamageResult {
        damage: dmg,
        multiplier,
        crit,
        stab,
    }
}

// ---------------------------------------------------------------------------
// Capture and fleeing
// ---------------------------------------------------------------------------

/// Modified catch value `a` in `1..=255`.
///
/// `ball_x10` is the ball bonus in tenths (`10` = 1x). Sleep and Freeze give a
/// 2x status bonus; Burn, Paralysis and Poison give 1.5x.
pub fn catch_value(max_hp: u16, hp: u16, catch_rate: u8, ball_x10: u16, status: Status) -> u32 {
    if max_hp == 0 {
        return 1;
    }
    let max = max_hp as u64;
    let cur = (hp.min(max_hp) as u64).max(1);
    let ball = ball_x10.min(MAX_BALL_BONUS_X10) as u64;
    let mut a = (3 * max - 2 * cur) * catch_rate as u64 * ball / 10 / (3 * max);
    a = match status {
        Status::Sleep | Status::Freeze => a * 2,
        Status::Burn | Status::Paralysis | Status::Poison => a * 3 / 2,
        Status::None => a,
    };
    a.clamp(1, 255) as u32
}

/// Per-shake success threshold (out of 65536) for catch value `a`.
pub fn shake_threshold(a: u32) -> u32 {
    let a = a.clamp(1, 255);
    if a >= 255 {
        return 65_535;
    }
    let r = isqrt_u32(isqrt_u32(16_711_680 / a)).max(1);
    (1_048_560 / r).min(65_535)
}

/// Attempt a capture; returns the number of successful shakes (`4` = caught).
pub fn attempt_capture(rng: &mut BattleRng, a: u32) -> u8 {
    let threshold = shake_threshold(a);
    let mut shakes = 0u8;
    while shakes < 4 {
        if rng.below(65_536) >= threshold {
            break;
        }
        shakes += 1;
    }
    shakes
}

/// Wild flee check. `random_byte` is a value in `0..=255`.
pub fn flee_succeeds(player_spe: u32, enemy_spe: u32, attempts: u8, random_byte: u32) -> bool {
    if player_spe >= enemy_spe {
        return true;
    }
    let f = (player_spe as u64 * 128 / (enemy_spe.max(1) as u64) + 30 * attempts as u64) % 256;
    (random_byte as u64) < f
}

// ---------------------------------------------------------------------------
// Battle
// ---------------------------------------------------------------------------

/// Battle phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Introductory text; call [`Battle::begin`].
    Intro,
    /// Waiting for both commands.
    CommandSelect,
    /// Commands accepted; call [`Battle::resolve_turn`].
    TurnResolution,
    /// The active player creature fainted; pick a replacement.
    SwitchPrompt,
    /// The player won (or captured the wild creature).
    Victory,
    /// All player creatures fainted.
    Defeat,
    /// The player fled.
    Fled,
}

impl Phase {
    /// `true` for `Victory`, `Defeat` and `Fled`.
    pub const fn is_terminal(self) -> bool {
        matches!(self, Phase::Victory | Phase::Defeat | Phase::Fled)
    }
}

/// A side's chosen action for a turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Use the move in `slot`.
    Fight {
        /// Move slot `0..4`.
        slot: u8,
    },
    /// Switch to party member `index`.
    Switch {
        /// Party index.
        index: u8,
    },
    /// Throw a capture ball (player only, wild battles only).
    Catch {
        /// Ball bonus in tenths (`10` = 1x).
        bonus_x10: u16,
    },
    /// Attempt to flee (player only, wild battles only).
    Run,
}

/// Something that happened during a turn, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Unused log slot.
    Empty,
    /// Battle began (`wild` or trainer).
    BattleStart {
        /// Whether the opponent is a wild creature.
        wild: bool,
    },
    /// A creature was sent out.
    SentOut {
        /// Owning side.
        side: Side,
        /// Party index.
        index: u8,
    },
    /// A move was used.
    MoveUsed {
        /// User's side.
        side: Side,
        /// Move slot (or `255` for Struggle).
        slot: u8,
    },
    /// The move missed.
    Missed {
        /// User's side.
        side: Side,
    },
    /// The move had no PP (should not occur through the public API).
    NoPp {
        /// User's side.
        side: Side,
    },
    /// Direct damage was dealt.
    Damage {
        /// Side that took the damage.
        side: Side,
        /// HP removed.
        amount: u16,
        /// Type multiplier ([`MULT_ONE`] units).
        multiplier: u16,
        /// Critical hit.
        crit: bool,
    },
    /// Type immunity.
    NoEffect {
        /// Immune side.
        side: Side,
    },
    /// A creature fainted.
    Fainted {
        /// Fainted side.
        side: Side,
    },
    /// A status was inflicted.
    StatusInflicted {
        /// Afflicted side.
        side: Side,
        /// New status.
        status: Status,
    },
    /// End-of-turn status damage.
    StatusDamage {
        /// Side that took damage.
        side: Side,
        /// HP removed.
        amount: u16,
    },
    /// Fully paralysed this turn.
    FullyParalyzed {
        /// Paralysed side.
        side: Side,
    },
    /// Still asleep this turn.
    Asleep {
        /// Sleeping side.
        side: Side,
    },
    /// Woke up.
    WokeUp {
        /// Side that woke.
        side: Side,
    },
    /// Frozen solid this turn.
    FrozenSolid {
        /// Frozen side.
        side: Side,
    },
    /// Thawed out.
    Thawed {
        /// Side that thawed.
        side: Side,
    },
    /// A stage changed.
    StageChanged {
        /// Affected side.
        side: Side,
        /// Quantity.
        stat: StatIndex,
        /// Actual signed change after clamping.
        delta: i8,
    },
    /// A stage could not move further.
    StageLimit {
        /// Affected side.
        side: Side,
        /// Quantity.
        stat: StatIndex,
    },
    /// Recoil damage.
    Recoil {
        /// Side that took recoil.
        side: Side,
        /// HP removed.
        amount: u16,
    },
    /// The active player creature gained XP.
    XpGained {
        /// XP amount.
        amount: u32,
    },
    /// The active player creature reached a new level.
    LevelUp {
        /// New level.
        level: u8,
    },
    /// Ball shook this many times.
    CaptureShake {
        /// Shake count `0..=3` on failure.
        count: u8,
    },
    /// The wild creature was captured.
    Captured,
    /// The wild creature broke free.
    CaptureFailed,
    /// Fled successfully.
    FleeSuccess,
    /// Failed to flee.
    FleeFailed,
    /// Player won.
    Victory,
    /// Player lost.
    Defeat,
}

/// A deterministic, zero-allocation battle.
#[derive(Clone, Copy, Debug)]
pub struct Battle {
    player: Roster,
    player_len: u8,
    player_active: u8,
    enemy: Roster,
    enemy_len: u8,
    enemy_active: u8,
    phase: Phase,
    turn: u16,
    wild: bool,
    rng: BattleRng,
    flee_attempts: u8,
    pending: Option<(Command, Command)>,
    captured: bool,
    events: EventLog,
    event_len: u8,
    events_dropped: u16,
}

fn first_alive(party: &[Creature]) -> Option<u8> {
    party.iter().position(|c| c.hp > 0).map(|i| i as u8)
}

fn load_roster(src: &[Creature]) -> Result<Roster, BattleError> {
    let first = src.first().ok_or(BattleError::PartyEmpty)?;
    if src.len() > MAX_PARTY {
        return Err(BattleError::PartyTooLarge);
    }
    let mut r = [*first; MAX_PARTY];
    for (dst, c) in r.iter_mut().zip(src.iter()) {
        c.check_integrity()?;
        *dst = *c;
    }
    Ok(r)
}

impl Battle {
    /// Create a battle in the `Intro` phase.
    ///
    /// Wild battles require exactly one enemy creature. Both parties need at
    /// least one non-fainted creature.
    pub fn start(
        player: &[Creature],
        enemy: &[Creature],
        wild: bool,
        seed: u64,
    ) -> Result<Battle, BattleError> {
        if wild && enemy.len() != 1 {
            return Err(BattleError::InvalidParty);
        }
        let p = load_roster(player)?;
        let e = load_roster(enemy)?;
        let pa = first_alive(player).ok_or(BattleError::AllFainted)?;
        let ea = first_alive(enemy).ok_or(BattleError::AllFainted)?;
        let mut b = Battle {
            player: p,
            player_len: player.len() as u8,
            player_active: pa,
            enemy: e,
            enemy_len: enemy.len() as u8,
            enemy_active: ea,
            phase: Phase::Intro,
            turn: 0,
            wild,
            rng: BattleRng::seeded(seed),
            flee_attempts: 0,
            pending: None,
            captured: false,
            events: [Event::Empty; MAX_EVENTS],
            event_len: 0,
            events_dropped: 0,
        };
        b.log(Event::BattleStart { wild });
        b.log(Event::SentOut {
            side: Side::Enemy,
            index: ea,
        });
        b.log(Event::SentOut {
            side: Side::Player,
            index: pa,
        });
        Ok(b)
    }

    // --- accessors -------------------------------------------------------

    /// Current phase.
    pub const fn phase(&self) -> Phase {
        self.phase
    }

    /// Completed turn count.
    pub const fn turn_number(&self) -> u16 {
        self.turn
    }

    /// Whether the opponent is wild.
    pub const fn is_wild_battle(&self) -> bool {
        self.wild
    }

    /// Whether the wild creature was captured.
    pub const fn was_captured(&self) -> bool {
        self.captured
    }

    /// Events recorded since the last command submission.
    pub fn event_log(&self) -> &[Event] {
        self.events.get(..self.event_len as usize).unwrap_or(&[])
    }

    /// Events dropped because the log was full.
    pub const fn dropped_events(&self) -> u16 {
        self.events_dropped
    }

    /// Number of creatures on `side`.
    pub const fn party_size(&self, side: Side) -> u8 {
        match side {
            Side::Player => self.player_len,
            Side::Enemy => self.enemy_len,
        }
    }

    /// Index of the active creature on `side`.
    pub const fn active_index(&self, side: Side) -> u8 {
        match side {
            Side::Player => self.player_active,
            Side::Enemy => self.enemy_active,
        }
    }

    /// The active creature on `side`.
    pub fn active_creature(&self, side: Side) -> &Creature {
        let idx = self.active_index(side) as usize;
        self.member_ref(side, idx)
    }

    /// Party member `index` on `side`, if it exists.
    pub fn party_member(&self, side: Side, index: u8) -> Option<&Creature> {
        if index >= self.party_size(side) {
            return None;
        }
        Some(self.member_ref(side, index as usize))
    }

    /// Whether `side` still has an unfainted creature.
    pub fn has_usable_creature(&self, side: Side) -> bool {
        self.roster_ref(side)
            .iter()
            .take(self.party_size(side) as usize)
            .any(|c| c.hp > 0)
    }

    // --- phase transitions ----------------------------------------------

    /// Leave `Intro` and start command selection.
    pub fn begin(&mut self) -> Result<(), BattleError> {
        if self.phase != Phase::Intro {
            return Err(BattleError::InvalidPhase);
        }
        self.event_len = 0;
        self.phase = Phase::CommandSelect;
        Ok(())
    }

    /// Validate and store both commands; moves to `TurnResolution`.
    pub fn submit_turn(&mut self, player: Command, enemy: Command) -> Result<(), BattleError> {
        if self.phase != Phase::CommandSelect {
            return Err(BattleError::InvalidPhase);
        }
        self.check_command(Side::Player, player)?;
        self.check_command(Side::Enemy, enemy)?;
        self.event_len = 0;
        self.events_dropped = 0;
        self.pending = Some((player, enemy));
        self.phase = Phase::TurnResolution;
        Ok(())
    }

    /// Choose a replacement after a faint (`SwitchPrompt` only).
    pub fn switch_in(&mut self, index: u8) -> Result<(), BattleError> {
        if self.phase != Phase::SwitchPrompt {
            return Err(BattleError::InvalidPhase);
        }
        match self.party_member(Side::Player, index) {
            Some(c) if c.hp > 0 => {}
            _ => return Err(BattleError::InvalidPartyIndex),
        }
        self.event_len = 0;
        self.player_active = index;
        self.log(Event::SentOut {
            side: Side::Player,
            index,
        });
        self.phase = Phase::CommandSelect;
        Ok(())
    }

    /// Simple deterministic enemy AI: a random usable move, else Struggle.
    pub fn ai_choose(&mut self) -> Command {
        let me = *self.active_creature(Side::Enemy);
        let usable = me
            .pp
            .iter()
            .take(me.move_count as usize)
            .filter(|p| **p > 0)
            .count() as u32;
        if usable == 0 {
            return Command::Fight { slot: 0 };
        }
        let mut pick = self.rng.below(usable);
        for (i, p) in me.pp.iter().take(me.move_count as usize).enumerate() {
            if *p > 0 {
                if pick == 0 {
                    return Command::Fight { slot: i as u8 };
                }
                pick -= 1;
            }
        }
        Command::Fight { slot: 0 }
    }

    /// Execute the stored turn and settle into the next phase.
    pub fn resolve_turn(&mut self) -> Result<(), BattleError> {
        if self.phase != Phase::TurnResolution {
            return Err(BattleError::InvalidPhase);
        }
        let (pc, ec) = self.pending.take().ok_or(BattleError::InvalidPhase)?;
        let order = self.action_order(pc, ec);
        for side in order {
            let cmd = if side == Side::Player { pc } else { ec };
            self.run_action(side, cmd);
            if self.phase != Phase::TurnResolution {
                return Ok(());
            }
        }
        self.end_of_turn(Side::Player);
        self.end_of_turn(Side::Enemy);
        self.settle_turn();
        Ok(())
    }

    // --- internals --------------------------------------------------------

    fn log(&mut self, e: Event) {
        match self.events.get_mut(self.event_len as usize) {
            Some(slot) => {
                *slot = e;
                self.event_len += 1;
            }
            None => self.events_dropped = self.events_dropped.saturating_add(1),
        }
    }

    fn roster_ref(&self, side: Side) -> &Roster {
        match side {
            Side::Player => &self.player,
            Side::Enemy => &self.enemy,
        }
    }

    fn member_ref(&self, side: Side, idx: usize) -> &Creature {
        let r = self.roster_ref(side);
        &r[idx.min(MAX_PARTY - 1)]
    }

    fn active_mut(&mut self, side: Side) -> &mut Creature {
        let idx = (self.active_index(side) as usize).min(MAX_PARTY - 1);
        match side {
            Side::Player => &mut self.player[idx],
            Side::Enemy => &mut self.enemy[idx],
        }
    }

    fn check_command(&self, side: Side, cmd: Command) -> Result<(), BattleError> {
        let me = self.active_creature(side);
        match cmd {
            Command::Fight { slot } => {
                if slot >= me.move_count {
                    return Err(BattleError::InvalidMoveSlot);
                }
                let pp = me.pp.get(slot as usize).copied().unwrap_or(0);
                if pp == 0 && !me.all_pp_spent() {
                    return Err(BattleError::NoPp);
                }
                Ok(())
            }
            Command::Switch { index } => match self.party_member(side, index) {
                Some(c) if c.hp > 0 && index != self.active_index(side) => Ok(()),
                _ => Err(BattleError::InvalidPartyIndex),
            },
            Command::Catch { bonus_x10 } => {
                if side != Side::Player
                    || !self.wild
                    || bonus_x10 == 0
                    || bonus_x10 > MAX_BALL_BONUS_X10
                {
                    Err(BattleError::CannotCatch)
                } else {
                    Ok(())
                }
            }
            Command::Run => {
                if side != Side::Player {
                    Err(BattleError::InvalidCommand)
                } else if !self.wild {
                    Err(BattleError::CannotFlee)
                } else {
                    Ok(())
                }
            }
        }
    }

    fn effective_speed(&self, side: Side) -> u32 {
        let c = self.active_creature(side);
        let mut s = effective_stat(c.stats.spe, c.stage_of(StatIndex::Spe) as i32);
        if c.status == Status::Paralysis {
            s = (s / 4).max(1);
        }
        s
    }

    fn bracket(&self, side: Side, cmd: Command) -> i32 {
        match cmd {
            Command::Switch { .. } => 100,
            Command::Run => 90,
            Command::Catch { .. } => 80,
            Command::Fight { slot } => {
                let c = self.active_creature(side);
                if c.all_pp_spent() {
                    0
                } else {
                    c.moves.get(slot as usize).map_or(0, |m| m.priority as i32)
                }
            }
        }
    }

    fn action_order(&mut self, pc: Command, ec: Command) -> SideOrder {
        let pb = self.bracket(Side::Player, pc);
        let eb = self.bracket(Side::Enemy, ec);
        let player_first = if pb != eb {
            pb > eb
        } else {
            let ps = self.effective_speed(Side::Player);
            let es = self.effective_speed(Side::Enemy);
            if ps != es {
                ps > es
            } else {
                self.rng.below(2) == 0
            }
        };
        if player_first {
            [Side::Player, Side::Enemy]
        } else {
            [Side::Enemy, Side::Player]
        }
    }

    fn run_action(&mut self, side: Side, cmd: Command) {
        if self.active_creature(side).hp == 0 {
            return;
        }
        match cmd {
            Command::Switch { index } => {
                let old = self.active_mut(side);
                old.stages = [0; STAGE_COUNT];
                match side {
                    Side::Player => self.player_active = index,
                    Side::Enemy => self.enemy_active = index,
                }
                self.log(Event::SentOut { side, index });
            }
            Command::Catch { bonus_x10 } => self.run_catch(bonus_x10),
            Command::Run => self.run_flee(),
            Command::Fight { slot } => self.run_move(side, slot),
        }
    }

    fn run_catch(&mut self, bonus_x10: u16) {
        let foe = *self.active_creature(Side::Enemy);
        let a = catch_value(foe.max_hp, foe.hp, foe.catch_rate, bonus_x10, foe.status);
        let shakes = attempt_capture(&mut self.rng, a);
        if shakes >= 4 {
            self.captured = true;
            self.log(Event::CaptureShake { count: 4 });
            self.log(Event::Captured);
            if (self.player_len as usize) < MAX_PARTY {
                let mut c = foe;
                c.stages = [0; STAGE_COUNT];
                let idx = self.player_len as usize;
                self.player[idx] = c;
                self.player_len += 1;
            }
            self.log(Event::Victory);
            self.phase = Phase::Victory;
        } else {
            self.log(Event::CaptureShake { count: shakes });
            self.log(Event::CaptureFailed);
        }
    }

    fn run_flee(&mut self) {
        let ps = self.effective_speed(Side::Player);
        let es = self.effective_speed(Side::Enemy);
        let byte = self.rng.below(256);
        let ok = flee_succeeds(ps, es, self.flee_attempts, byte);
        self.flee_attempts = self.flee_attempts.saturating_add(1);
        if ok {
            self.log(Event::FleeSuccess);
            self.phase = Phase::Fled;
        } else {
            self.log(Event::FleeFailed);
        }
    }

    /// Returns `true` when the actor may proceed with its move.
    fn pass_status_gate(&mut self, side: Side) -> bool {
        let status = self.active_creature(side).status;
        match status {
            Status::Sleep => {
                let c = self.active_mut(side);
                c.sleep_turns = c.sleep_turns.saturating_sub(1);
                if c.sleep_turns == 0 {
                    c.status = Status::None;
                    self.log(Event::WokeUp { side });
                    true
                } else {
                    self.log(Event::Asleep { side });
                    false
                }
            }
            Status::Freeze => {
                if self.rng.below(100) < 20 {
                    self.active_mut(side).status = Status::None;
                    self.log(Event::Thawed { side });
                    true
                } else {
                    self.log(Event::FrozenSolid { side });
                    false
                }
            }
            Status::Paralysis => {
                if self.rng.below(100) < 25 {
                    self.log(Event::FullyParalyzed { side });
                    false
                } else {
                    true
                }
            }
            Status::Burn | Status::Poison | Status::None => true,
        }
    }

    fn move_hits(&mut self, att: &Creature, def: &Creature, mv: &Move) -> bool {
        if mv.accuracy == 0 {
            return true;
        }
        let stage = att.stage_of(StatIndex::Acc) as i32 - def.stage_of(StatIndex::Eva) as i32;
        let (n, d) = accuracy_ratio(stage);
        let threshold = mv.accuracy as u32 * n / d;
        self.rng.below(100) < threshold
    }

    fn run_move(&mut self, side: Side, slot: u8) {
        if !self.pass_status_gate(side) {
            return;
        }
        let foe_side = side.foe();
        let att = *self.active_creature(side);
        let struggling = att.all_pp_spent();
        let mv = if struggling {
            STRUGGLE_MOVE
        } else {
            let idx = slot as usize;
            let pp = att.pp.get(idx).copied().unwrap_or(0);
            match att.moves.get(idx) {
                Some(m) if pp > 0 && slot < att.move_count => {
                    if let Some(p) = self.active_mut(side).pp.get_mut(idx) {
                        *p = p.saturating_sub(1);
                    }
                    *m
                }
                _ => {
                    self.log(Event::NoPp { side });
                    return;
                }
            }
        };
        self.log(Event::MoveUsed {
            side,
            slot: if struggling { 255 } else { slot },
        });
        let def = *self.active_creature(foe_side);
        if !self.move_hits(&att, &def, &mv) {
            self.log(Event::Missed { side });
            return;
        }
        if mv.power == 0 || mv.category == Category::Status {
            self.apply_effect(side, mv.effect);
            return;
        }
        let crit = self.rng.below(16) == 0;
        let roll = 85 + self.rng.below(16);
        let res = calc_damage(&att, &def, &mv, roll, crit);
        if res.multiplier == 0 {
            self.log(Event::NoEffect { side: foe_side });
            return;
        }
        let dealt = res.damage.min(def.hp);
        self.log(Event::Damage {
            side: foe_side,
            amount: dealt,
            multiplier: res.multiplier,
            crit: res.crit,
        });
        self.damage_creature(foe_side, dealt);
        if struggling {
            let recoil = (dealt / 4).max(1).min(self.active_creature(side).hp);
            self.log(Event::Recoil {
                side,
                amount: recoil,
            });
            self.damage_creature(side, recoil);
        }
        if self.active_creature(foe_side).hp > 0 {
            self.apply_effect(side, mv.effect);
        }
    }

    /// Remove HP from the active creature of `side`, handling a faint.
    fn damage_creature(&mut self, side: Side, amount: u16) {
        let c = self.active_mut(side);
        c.hp = c.hp.saturating_sub(amount);
        if c.hp == 0 {
            c.status = Status::None;
            c.sleep_turns = 0;
            self.log(Event::Fainted { side });
            if side == Side::Enemy {
                self.award_xp();
            }
        }
    }

    fn award_xp(&mut self) {
        let foe = *self.active_creature(Side::Enemy);
        let amount = xp_yield(foe.exp_yield, foe.level, self.wild);
        if self.active_creature(Side::Player).hp == 0 {
            return;
        }
        self.log(Event::XpGained { amount });
        let before = self.active_creature(Side::Player).level;
        let gained = self.active_mut(Side::Player).grant_xp(amount);
        for i in 1..=gained {
            self.log(Event::LevelUp {
                level: before.saturating_add(i),
            });
        }
    }

    fn apply_effect(&mut self, user: Side, effect: Effect) {
        match effect {
            Effect::NoEffect => {}
            Effect::Inflict { status, chance } => {
                if !self.chance_roll(chance) {
                    return;
                }
                let target = user.foe();
                let rolled = 1 + self.rng.below(3) as u8;
                let c = self.active_mut(target);
                if c.hp == 0 || c.status != Status::None || status_immune(c, status) {
                    return;
                }
                c.status = status;
                c.sleep_turns = if status == Status::Sleep { rolled } else { 0 };
                self.log(Event::StatusInflicted {
                    side: target,
                    status,
                });
            }
            Effect::Stage {
                stat,
                delta,
                on_self,
                chance,
            } => {
                if !self.chance_roll(chance) {
                    return;
                }
                let target = if on_self { user } else { user.foe() };
                let c = self.active_mut(target);
                if c.hp == 0 {
                    return;
                }
                let Some(cell) = c.stages.get_mut(stat.slot()) else {
                    return;
                };
                let old = clamp_stage(*cell as i32);
                let new = clamp_stage(old as i32 + delta as i32);
                *cell = new;
                let actual = new - old;
                if actual == 0 {
                    self.log(Event::StageLimit { side: target, stat });
                } else {
                    self.log(Event::StageChanged {
                        side: target,
                        stat,
                        delta: actual,
                    });
                }
            }
        }
    }

    fn chance_roll(&mut self, chance: u8) -> bool {
        chance >= 100 || (chance > 0 && self.rng.below(100) < chance as u32)
    }

    fn end_of_turn(&mut self, side: Side) {
        let c = *self.active_creature(side);
        if c.hp == 0 {
            return;
        }
        let amount = match c.status {
            Status::Burn => (c.max_hp / 16).max(1),
            Status::Poison => (c.max_hp / 8).max(1),
            _ => return,
        };
        let dealt = amount.min(c.hp);
        self.log(Event::StatusDamage {
            side,
            amount: dealt,
        });
        self.damage_creature(side, dealt);
    }

    fn settle_turn(&mut self) {
        self.turn = self.turn.saturating_add(1);
        let player_down = self.active_creature(Side::Player).hp == 0;
        let enemy_down = self.active_creature(Side::Enemy).hp == 0;
        if enemy_down {
            let next = self
                .enemy
                .iter()
                .take(self.enemy_len as usize)
                .position(|c| c.hp > 0);
            match next {
                Some(i) if !self.wild => {
                    self.enemy_active = i as u8;
                    self.log(Event::SentOut {
                        side: Side::Enemy,
                        index: i as u8,
                    });
                }
                _ => {
                    if self.has_usable_creature(Side::Player) || !player_down {
                        self.log(Event::Victory);
                        self.phase = Phase::Victory;
                    } else {
                        self.log(Event::Defeat);
                        self.phase = Phase::Defeat;
                    }
                    return;
                }
            }
        }
        if player_down {
            let next = self
                .player
                .iter()
                .take(self.player_len as usize)
                .position(|c| c.hp > 0);
            if next.is_some() {
                self.phase = Phase::SwitchPrompt;
            } else {
                self.log(Event::Defeat);
                self.phase = Phase::Defeat;
            }
            return;
        }
        self.phase = Phase::CommandSelect;
    }
}

fn status_immune(c: &Creature, status: Status) -> bool {
    let immune_type = match status {
        Status::Burn => FIRE,
        Status::Paralysis => ELECTRIC,
        Status::Poison => POISON,
        Status::Freeze => ICE,
        Status::Sleep => return false,
        Status::None => return true,
    };
    c.type1 == immune_type || c.type2 == immune_type
}

// ---------------------------------------------------------------------------
// HUD / palette integration
// ---------------------------------------------------------------------------

/// HP bar colour band.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HpBarState {
    /// More than 50% HP.
    Green,
    /// More than 20% and at most 50% HP.
    Yellow,
    /// 1 HP up to 20% HP.
    Red,
    /// Zero HP.
    Empty,
}

/// Fade the HUD should start for a phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FadeTrigger {
    /// No fade.
    None,
    /// Fade in from black (battle entry).
    FadeInFromBlack,
    /// Fade out to white (victory / escape).
    FadeOutToWhite,
    /// Slow fade out to black (defeat).
    FadeOutToBlack,
}

/// Classify HP into a bar colour band (guards `max_hp == 0`).
pub fn hp_bar_state(hp: u16, max_hp: u16) -> HpBarState {
    if max_hp == 0 || hp == 0 {
        return HpBarState::Empty;
    }
    let hp = hp.min(max_hp) as u32;
    let max = max_hp as u32;
    if hp * 2 > max {
        HpBarState::Green
    } else if hp * 5 > max {
        HpBarState::Yellow
    } else {
        HpBarState::Red
    }
}

/// Filled pixels of an HP bar `width` pixels wide; never `0` while `hp > 0`.
pub fn hp_bar_pixels(hp: u16, max_hp: u16, width: u16) -> u16 {
    if max_hp == 0 || hp == 0 || width == 0 {
        return 0;
    }
    let hp = hp.min(max_hp) as u64;
    let px = (hp * width as u64).div_ceil(max_hp as u64);
    px.clamp(1, width as u64) as u16
}

/// Canonical colour of an HP bar band.
pub const fn hp_bar_color(state: HpBarState) -> Rgba8 {
    match state {
        HpBarState::Green => [48, 208, 80, 255],
        HpBarState::Yellow => [248, 208, 32, 255],
        HpBarState::Red => [232, 56, 48, 255],
        HpBarState::Empty => [40, 40, 40, 255],
    }
}

/// Palette index conventionally used for each HP bar band (`0` = transparent).
pub const fn hp_bar_palette_index(state: HpBarState) -> u8 {
    match state {
        HpBarState::Green => 1,
        HpBarState::Yellow => 2,
        HpBarState::Red => 3,
        HpBarState::Empty => 0,
    }
}

/// Smoothly blend green to red by missing HP using `palette-indexed` fading.
pub fn hp_bar_blend_color(hp: u16, max_hp: u16) -> Rgba8 {
    if max_hp == 0 || hp == 0 {
        return hp_bar_color(HpBarState::Empty);
    }
    let hp = hp.min(max_hp) as u64;
    let missing_pct = ((max_hp as u64 - hp) * 100 / max_hp as u64) as u16;
    fade_color(
        hp_bar_color(HpBarState::Green),
        hp_bar_color(HpBarState::Red),
        missing_pct,
        100,
        ChannelBits::Bits8,
    )
    .unwrap_or_else(|_| hp_bar_color(HpBarState::Red))
}

/// Which `palette-indexed` fade to start when entering `phase`.
pub const fn fade_trigger_for_phase(phase: Phase) -> FadeTrigger {
    match phase {
        Phase::Intro => FadeTrigger::FadeInFromBlack,
        Phase::Victory | Phase::Fled => FadeTrigger::FadeOutToWhite,
        Phase::Defeat => FadeTrigger::FadeOutToBlack,
        Phase::CommandSelect | Phase::TurnResolution | Phase::SwitchPrompt => FadeTrigger::None,
    }
}

/// `(target colour, total steps)` for a fade, ready for `fade_palette_into`.
pub const fn fade_params(trigger: FadeTrigger) -> (Rgba8, u16) {
    match trigger {
        FadeTrigger::None => ([0, 0, 0, 255], 1),
        FadeTrigger::FadeInFromBlack => ([0, 0, 0, 255], 16),
        FadeTrigger::FadeOutToWhite => ([255, 255, 255, 255], 16),
        FadeTrigger::FadeOutToBlack => ([0, 0, 0, 255], 32),
    }
}
