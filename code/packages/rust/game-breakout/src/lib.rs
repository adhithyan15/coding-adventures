//! # Deterministic Arcade Breakout State Machine & PaintVM Renderer (GAME14)
//!
//! Reusable pure-Rust implementation of the iconic 1976 arcade classic *Breakout*
//! (Nolan Bushnell, Steve Bristow, Steve Wozniak / Atari Inc.).
//!
//! Features:
//! - Exact continuous swept circle-AABB collision detection ([`collision2d`]) to eliminate
//!   high-speed tunneling through paddles or brick boundaries (`CWE-190`, `CWE-20`).
//! - Deterministic integer-nanosecond fixed-timestep simulation loop ([`game-clock`]).
//! - Multi-channel synthesized arcade sound effects ([`arcade-sfx`], [`audio-mixer-realtime`]).
//! - Zero-allocation sprite quad emission ([`paint-batch-tiles`]) and lowering into [`PaintScene`].
//! - Zero ambient capabilities (`"capabilities": []`).
//! - `#![forbid(unsafe_code)]`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Pure computation capability witness module.
pub mod capabilities;
pub use capabilities::PureCapabilities;

pub use arcade_sfx::{SfxBank, SfxError, SfxPreset};
pub use audio_mixer_realtime::{MixerError, PcmRingBuffer};
pub use collision2d::{
    circle_vs_aabb, reflect_velocity, sweep_circle_aabb, Circle, CollisionError, Point, Rect,
    SweepHit, MAX_COORD,
};
pub use game_clock::{advance_frame, ClockError, ClockStepResult, GameClockState};
pub use paint_batch_tiles::{lower_sprites_to_paint_scene, PaintBatchError, PaintScene, SpriteQuad};

/// Type alias for [`SpriteQuad`] conforming to the engine sprite interface.
pub type SpriteItem = SpriteQuad;

/// Maximum number of brick rows supported in the fixed grid.
pub const MAX_BRICK_ROWS: usize = 8;

/// Maximum number of brick columns supported in the fixed grid.
pub const MAX_BRICK_COLS: usize = 14;

/// Maximum number of total bricks in the zero-allocation brick grid.
pub const MAX_BRICKS: usize = MAX_BRICK_ROWS * MAX_BRICK_COLS; // 112

/// Maximum number of sprites generated for a Breakout frame (paddle, ball, bricks, HUD).
pub const MAX_BREAKOUT_SPRITES: usize = 256;

/// Default arena width in logical units.
pub const DEFAULT_ARENA_WIDTH: f64 = 800.0;

/// Default arena height in logical units.
pub const DEFAULT_ARENA_HEIGHT: f64 = 600.0;

/// Default paddle width in logical units.
pub const DEFAULT_PADDLE_WIDTH: f64 = 100.0;

/// Default paddle height in logical units.
pub const DEFAULT_PADDLE_HEIGHT: f64 = 16.0;

/// Default paddle speed in logical units per second.
pub const DEFAULT_PADDLE_SPEED: f64 = 480.0;

/// Default paddle vertical inset distance from arena bottom edge.
pub const DEFAULT_PADDLE_BOTTOM_INSET: f64 = 32.0;

/// Default ball radius in logical units.
pub const DEFAULT_BALL_RADIUS: f64 = 8.0;

/// Default initial ball speed in logical units per second.
pub const DEFAULT_BALL_INITIAL_SPEED: f64 = 360.0;

/// Default maximum ball speed in logical units per second.
pub const DEFAULT_BALL_MAX_SPEED: f64 = 800.0;

/// Default speed increment added on each paddle deflection or brick hit.
pub const DEFAULT_BALL_SPEED_INCREMENT: f64 = 8.0;

/// Default maximum deflection angle in radians (~60 degrees).
pub const DEFAULT_MAX_DEFLECTION_RAD: f64 = 1.0471975511965976;

/// Default initial player lives counter.
pub const DEFAULT_INITIAL_LIVES: u32 = 3;

/// Default fixed timestep in seconds (60 Hz).
pub const DEFAULT_FIXED_DT_SECONDS: f64 = 1.0 / 60.0;

/// Default number of brick rows.
pub const DEFAULT_BRICK_ROWS: usize = 8;

/// Default number of brick columns.
pub const DEFAULT_BRICK_COLS: usize = 14;

/// Default brick width in logical units.
pub const DEFAULT_BRICK_WIDTH: f64 = 48.0;

/// Default brick height in logical units.
pub const DEFAULT_BRICK_HEIGHT: f64 = 18.0;

/// Default spacing gap between adjacent bricks.
pub const DEFAULT_BRICK_GAP: f64 = 4.0;

/// Default top margin from ceiling to first row of bricks.
pub const DEFAULT_BRICK_TOP_OFFSET: f64 = 80.0;

/// Default left margin from wall to first column of bricks.
pub const DEFAULT_BRICK_SIDE_MARGIN: f64 = 38.0;

/// Specific error conditions encountered in Breakout simulation, physics, or rendering.
#[derive(Debug, Clone, PartialEq)]
pub enum BreakoutError {
    /// Configuration parameters failed validation (non-finite or invalid bounds).
    InvalidConfig,
    /// State fields contain non-finite numbers (NaN or Infinity).
    NonFiniteState,
    /// State coordinates or dimensions exceed allowable physical arena bounds.
    StateOutOfBounds,
    /// Dimension or coordinate was negative when positive was required.
    NegativeDimension,
    /// Geometry or swept query error raised by `collision2d`.
    Collision(CollisionError),
    /// Simulation timestep accumulator error raised by `game-clock`.
    Clock(ClockError),
    /// Retained-mode sprite lowering error raised by `paint-batch-tiles`.
    Paint(PaintBatchError),
    /// Real-time audio synthesizer or bank error raised by `arcade-sfx`.
    Audio(SfxError),
}

impl std::fmt::Display for BreakoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig => write!(f, "invalid breakout configuration parameters"),
            Self::NonFiniteState => write!(f, "breakout state contains non-finite float (NaN or Inf)"),
            Self::StateOutOfBounds => write!(f, "coordinates exceed allowable arena or physical bounds"),
            Self::NegativeDimension => write!(f, "dimension or coordinate cannot be negative"),
            Self::Collision(e) => write!(f, "collision error: {e}"),
            Self::Clock(e) => write!(f, "game clock error: {e}"),
            Self::Paint(e) => write!(f, "paint batch lowering error: {e:?}"),
            Self::Audio(e) => write!(f, "arcade sfx error: {e}"),
        }
    }
}

impl std::error::Error for BreakoutError {}

/// Physical dimensions, movement speeds, and match rules for Breakout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BreakoutConfig {
    /// Arena width in logical scene units.
    pub arena_width: f64,
    /// Arena height in logical scene units.
    pub arena_height: f64,
    /// Paddle width in logical units.
    pub paddle_width: f64,
    /// Paddle height in logical units.
    pub paddle_height: f64,
    /// Paddle horizontal movement speed in units per second.
    pub paddle_speed: f64,
    /// Paddle vertical inset distance from arena bottom edge.
    pub paddle_bottom_inset: f64,
    /// Ball radius in logical units.
    pub ball_radius: f64,
    /// Initial ball speed when launched.
    pub ball_initial_speed: f64,
    /// Absolute maximum speed cap for the ball.
    pub ball_max_speed: f64,
    /// Speed bonus added to the ball on each paddle deflection or brick hit.
    pub ball_speed_increment: f64,
    /// Maximum paddle bounce deflection angle in radians from vertical normal.
    pub max_deflection_angle_rad: f64,
    /// Number of brick rows (1..=MAX_BRICK_ROWS).
    pub brick_rows: usize,
    /// Number of brick columns (1..=MAX_BRICK_COLS).
    pub brick_cols: usize,
    /// Brick width in logical units.
    pub brick_width: f64,
    /// Brick height in logical units.
    pub brick_height: f64,
    /// Spacing gap between adjacent bricks.
    pub brick_gap: f64,
    /// Top margin from ceiling to first row of bricks.
    pub brick_top_offset: f64,
    /// Left margin from wall to first column of bricks.
    pub brick_side_margin: f64,
    /// Initial lives count (default 3).
    pub initial_lives: u32,
    /// Simulation timestep delta in seconds per tick.
    pub fixed_dt_seconds: f64,
}

impl BreakoutConfig {
    /// Constructs default configuration matching classic 60 Hz arcade Breakout specifications.
    pub const fn standard_breakout() -> Self {
        Self {
            arena_width: DEFAULT_ARENA_WIDTH,
            arena_height: DEFAULT_ARENA_HEIGHT,
            paddle_width: DEFAULT_PADDLE_WIDTH,
            paddle_height: DEFAULT_PADDLE_HEIGHT,
            paddle_speed: DEFAULT_PADDLE_SPEED,
            paddle_bottom_inset: DEFAULT_PADDLE_BOTTOM_INSET,
            ball_radius: DEFAULT_BALL_RADIUS,
            ball_initial_speed: DEFAULT_BALL_INITIAL_SPEED,
            ball_max_speed: DEFAULT_BALL_MAX_SPEED,
            ball_speed_increment: DEFAULT_BALL_SPEED_INCREMENT,
            max_deflection_angle_rad: DEFAULT_MAX_DEFLECTION_RAD,
            brick_rows: DEFAULT_BRICK_ROWS,
            brick_cols: DEFAULT_BRICK_COLS,
            brick_width: DEFAULT_BRICK_WIDTH,
            brick_height: DEFAULT_BRICK_HEIGHT,
            brick_gap: DEFAULT_BRICK_GAP,
            brick_top_offset: DEFAULT_BRICK_TOP_OFFSET,
            brick_side_margin: DEFAULT_BRICK_SIDE_MARGIN,
            initial_lives: DEFAULT_INITIAL_LIVES,
            fixed_dt_seconds: DEFAULT_FIXED_DT_SECONDS,
        }
    }

    /// Validates configuration parameters against non-finite floats, negative values, and out-of-range bounds.
    pub fn check_config_validity(&self) -> Result<(), BreakoutError> {
        if !self.arena_width.is_finite()
            || !self.arena_height.is_finite()
            || !self.paddle_width.is_finite()
            || !self.paddle_height.is_finite()
            || !self.paddle_speed.is_finite()
            || !self.paddle_bottom_inset.is_finite()
            || !self.ball_radius.is_finite()
            || !self.ball_initial_speed.is_finite()
            || !self.ball_max_speed.is_finite()
            || !self.ball_speed_increment.is_finite()
            || !self.max_deflection_angle_rad.is_finite()
            || !self.brick_width.is_finite()
            || !self.brick_height.is_finite()
            || !self.brick_gap.is_finite()
            || !self.brick_top_offset.is_finite()
            || !self.brick_side_margin.is_finite()
            || !self.fixed_dt_seconds.is_finite()
        {
            return Err(BreakoutError::NonFiniteState);
        }

        if self.arena_width <= 0.0
            || self.arena_height <= 0.0
            || self.arena_width > MAX_COORD
            || self.arena_height > MAX_COORD
            || self.paddle_width <= 0.0
            || self.paddle_width >= self.arena_width
            || self.paddle_height <= 0.0
            || self.paddle_height >= self.arena_height * 0.5
            || self.paddle_speed <= 0.0
            || self.paddle_bottom_inset < 0.0
            || self.paddle_bottom_inset >= self.arena_height * 0.5
            || self.ball_radius <= 0.0
            || self.ball_radius >= self.arena_height * 0.25
            || self.ball_initial_speed <= 0.0
            || self.ball_initial_speed > self.ball_max_speed
            || self.ball_max_speed > MAX_COORD
            || self.ball_speed_increment < 0.0
            || self.max_deflection_angle_rad <= 0.0
            || self.max_deflection_angle_rad >= std::f64::consts::FRAC_PI_2
            || self.brick_rows == 0
            || self.brick_rows > MAX_BRICK_ROWS
            || self.brick_cols == 0
            || self.brick_cols > MAX_BRICK_COLS
            || self.brick_width <= 0.0
            || self.brick_width > MAX_COORD
            || self.brick_height <= 0.0
            || self.brick_height > MAX_COORD
            || self.brick_gap < 0.0
            || self.brick_gap > MAX_COORD
            || self.brick_top_offset < 0.0
            || self.brick_top_offset > MAX_COORD
            || self.brick_side_margin < 0.0
            || self.brick_side_margin > MAX_COORD
            || self.initial_lives == 0
            || self.fixed_dt_seconds <= 0.0
            || self.fixed_dt_seconds > 1.0
        {
            return Err(BreakoutError::InvalidConfig);
        }

        Ok(())
    }
}

impl Default for BreakoutConfig {
    fn default() -> Self {
        Self::standard_breakout()
    }
}

/// Helper function to create default Breakout configuration.
#[inline]
pub fn default_config() -> BreakoutConfig {
    BreakoutConfig::standard_breakout()
}

/// Helper function to validate Breakout configuration.
#[inline]
pub fn validate_config(config: &BreakoutConfig) -> Result<(), BreakoutError> {
    config.check_config_validity()
}

/// Discrete paddle movement request for a single tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaddleMove {
    /// Stationary paddle.
    Hold,
    /// Move paddle leftward (decreasing X).
    Left,
    /// Move paddle rightward (increasing X).
    Right,
}

/// Frame input specification controlling paddle movement and ball launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakoutInput {
    /// Directional movement requested for the paddle.
    pub movement: PaddleMove,
    /// Whether ball launch was triggered if currently attached to paddle.
    pub launch: bool,
}

impl BreakoutInput {
    /// Constructs input commanding paddle movement leftward.
    pub const fn move_paddle_left(launch: bool) -> Self {
        Self {
            movement: PaddleMove::Left,
            launch,
        }
    }

    /// Constructs input commanding paddle movement rightward.
    pub const fn move_paddle_right(launch: bool) -> Self {
        Self {
            movement: PaddleMove::Right,
            launch,
        }
    }

    /// Constructs input commanding stationary paddle.
    pub const fn hold_paddle(launch: bool) -> Self {
        Self {
            movement: PaddleMove::Hold,
            launch,
        }
    }

    /// Neutral input holding paddle stationary without launch.
    pub const fn idle_breakout_input() -> Self {
        Self {
            movement: PaddleMove::Hold,
            launch: false,
        }
    }

    /// Input triggering ball launch while keeping paddle stationary.
    pub const fn launch_only() -> Self {
        Self {
            movement: PaddleMove::Hold,
            launch: true,
        }
    }
}

impl Default for BreakoutInput {
    fn default() -> Self {
        Self::idle_breakout_input()
    }
}

/// A single brick slot within the fixed-capacity brick grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrickSlot {
    /// Zero-based row index (0..MAX_BRICK_ROWS).
    pub row: u8,
    /// Zero-based column index (0..MAX_BRICK_COLS).
    pub col: u8,
    /// Durability / hit points remaining (0 = inactive/destroyed).
    pub hits_remaining: u8,
    /// Initial maximum hit points (e.g. 1 or 2).
    pub max_hits: u8,
    /// Base points awarded on hit.
    pub points: u32,
    /// RGB display color.
    pub color_rgb: [u8; 3],
}

/// RGB color triplet.
pub type RgbColor = [u8; 3];

impl BrickSlot {
    /// Creates a new brick slot.
    pub const fn new_brick(
        row: u8,
        col: u8,
        max_hits: u8,
        points: u32,
        color_rgb: RgbColor,
    ) -> Self {
        Self {
            row,
            col,
            hits_remaining: max_hits,
            max_hits,
            points,
            color_rgb,
        }
    }

    /// Returns whether this brick slot is currently active and undestroyed.
    #[inline]
    pub const fn is_brick_active(&self) -> bool {
        self.hits_remaining > 0
    }
}

/// Simulation events produced during a single fixed-timestep tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakoutFrameEvents {
    /// Whether the paddle deflected the ball during this tick.
    pub paddle_hit: bool,
    /// Whether the ball bounced off an arena wall (top, left, or right).
    pub wall_bounce: bool,
    /// Row index of brick hit during this tick, if any.
    pub brick_hit_row: Option<u8>,
    /// Whether a life was lost during this tick.
    pub life_lost: bool,
    /// Whether the current level was cleared during this tick.
    pub level_cleared: bool,
    /// Whether game over state has been reached (all lives depleted).
    pub game_over: bool,
    /// Points scored during this tick (saturating).
    pub points_scored: u32,
    /// Active consecutive combo hit counter.
    pub combo_count: u32,
}

impl BreakoutFrameEvents {
    /// Constructs an empty event structure with no collisions or state transitions.
    pub const fn empty_breakout_events() -> Self {
        Self {
            paddle_hit: false,
            wall_bounce: false,
            brick_hit_row: None,
            life_lost: false,
            level_cleared: false,
            game_over: false,
            points_scored: 0,
            combo_count: 0,
        }
    }
}

impl Default for BreakoutFrameEvents {
    fn default() -> Self {
        Self::empty_breakout_events()
    }
}

/// Complete mutable simulation state for a Breakout match.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BreakoutState {
    /// Active physical and rule configuration.
    pub config: BreakoutConfig,
    /// Horizontal position of paddle's left edge.
    pub paddle_x: f64,
    /// Center coordinates of the ball.
    pub ball_pos: Point,
    /// Velocity vector of the ball in logical units per second.
    pub ball_vel: Point,
    /// Whether ball is attached to the paddle awaiting launch.
    pub ball_attached: bool,
    /// Fixed-size zero-allocation brick array.
    pub bricks: [BrickSlot; MAX_BRICKS],
    /// Number of active (non-destroyed) bricks remaining.
    pub active_bricks: usize,
    /// Total cumulative score (saturating, CWE-190).
    pub score: u32,
    /// Lives counter (default 3).
    pub lives: u32,
    /// Current level number (starts at 1).
    pub level: u32,
    /// Consecutive brick hits in current rally without paddle touch or life loss.
    pub combo_counter: u32,
    /// Deterministic 64-bit PRNG seed.
    pub prng_seed: u64,
    /// True if match is concluded (lives == 0).
    pub game_over: bool,
}

impl BreakoutState {
    /// Constructs a fresh Breakout match state with default brick layout, ball attached to paddle, and full lives.
    pub fn init_breakout_state(
        config: BreakoutConfig,
        initial_prng_seed: u64,
    ) -> Result<Self, BreakoutError> {
        config.check_config_validity()?;
        let seed = if initial_prng_seed == 0 {
            0x517c_c1b7_2722_0a95
        } else {
            initial_prng_seed
        };

        let paddle_x = (config.arena_width - config.paddle_width) * 0.5;
        let paddle_y = config.arena_height - config.paddle_bottom_inset - config.paddle_height;
        let ball_pos = Point::new(
            paddle_x + config.paddle_width * 0.5,
            paddle_y - config.ball_radius,
        );
        let ball_vel = Point::new(0.0, 0.0);

        let dummy_brick = BrickSlot::new_brick(0, 0, 0, 0, [0, 0, 0]);
        let mut state = Self {
            config,
            paddle_x,
            ball_pos,
            ball_vel,
            ball_attached: true,
            bricks: [dummy_brick; MAX_BRICKS],
            active_bricks: 0,
            score: 0,
            lives: config.initial_lives,
            level: 1,
            combo_counter: 0,
            prng_seed: seed,
            game_over: false,
        };

        state.populate_level_bricks(1);
        state.check_breakout_state_validity()?;
        Ok(state)
    }

    /// Validates state invariants, float finiteness, and coordinate bounds.
    pub fn check_breakout_state_validity(&self) -> Result<(), BreakoutError> {
        self.config.check_config_validity()?;

        if !self.paddle_x.is_finite()
            || !self.ball_pos.x.is_finite()
            || !self.ball_pos.y.is_finite()
            || !self.ball_vel.x.is_finite()
            || !self.ball_vel.y.is_finite()
        {
            return Err(BreakoutError::NonFiniteState);
        }

        let max_paddle_x = (self.config.arena_width - self.config.paddle_width) + 1.0;
        if self.paddle_x < -1.0 || self.paddle_x > max_paddle_x {
            return Err(BreakoutError::StateOutOfBounds);
        }

        if self.ball_pos.x.abs() > MAX_COORD || self.ball_pos.y.abs() > MAX_COORD {
            return Err(BreakoutError::StateOutOfBounds);
        }

        if self.ball_vel.magnitude_squared() > MAX_COORD * MAX_COORD {
            return Err(BreakoutError::StateOutOfBounds);
        }

        if self.active_bricks > MAX_BRICKS {
            return Err(BreakoutError::StateOutOfBounds);
        }

        Ok(())
    }

    /// Returns the bounding AABB rectangle for the player paddle.
    #[inline]
    pub fn paddle_bounding_rect(&self) -> Rect {
        let paddle_y =
            self.config.arena_height - self.config.paddle_bottom_inset - self.config.paddle_height;
        Rect::new(
            self.paddle_x,
            paddle_y,
            self.config.paddle_width,
            self.config.paddle_height,
        )
    }

    /// Returns the bounding circle geometry representing the ball.
    #[inline]
    pub fn ball_bounding_circle(&self) -> Circle {
        Circle {
            center: self.ball_pos,
            radius: self.config.ball_radius,
        }
    }

    /// Resets ball to be attached atop the player paddle awaiting launch.
    pub fn reset_ball_on_paddle(&mut self) {
        let paddle_rect = self.paddle_bounding_rect();
        self.ball_pos = Point::new(
            self.paddle_x + self.config.paddle_width * 0.5,
            paddle_rect.y - self.config.ball_radius,
        );
        self.ball_vel = Point::new(0.0, 0.0);
        self.ball_attached = true;
        self.combo_counter = 0;
    }

    /// Populates the brick grid for the specified level.
    pub fn populate_level_bricks(&mut self, level: u32) {
        let mut count = 0;
        for row in 0..self.config.brick_rows {
            // Colors: top 2 rows red, next 2 orange, next 2 green, bottom 2 yellow
            let color = match row {
                0 => [220, 20, 60],  // Crimson
                1 => [255, 69, 0],   // Red-Orange
                2 => [255, 140, 0],  // Dark Orange
                3 => [255, 165, 0],  // Orange
                4 => [50, 205, 50],  // Lime Green
                5 => [34, 139, 34],  // Forest Green
                6 => [255, 215, 0],  // Gold
                _ => [255, 255, 0],  // Yellow
            };

            // Points: top rows 7, next 5, next 3, bottom 1
            let points = match row {
                0 | 1 => 7,
                2 | 3 => 5,
                4 | 5 => 3,
                _ => 1,
            };

            // Durability: top 2 rows are durable (2 hits), lower rows are 1 hit
            // Higher levels make additional rows durable
            let durable_rows = if level >= 3 { 4 } else { 2 };
            let max_hits = if row < durable_rows { 2 } else { 1 };

            for col in 0..self.config.brick_cols {
                let idx = row * MAX_BRICK_COLS + col;
                self.bricks[idx] = BrickSlot::new_brick(
                    row as u8,
                    col as u8,
                    max_hits,
                    points,
                    color,
                );
                count += 1;
            }
        }

        // Fill remaining unused slots with inactive bricks
        for idx in count..MAX_BRICKS {
            self.bricks[idx] = BrickSlot::new_brick(0, 0, 0, 0, [0, 0, 0]);
        }

        self.active_bricks = count;
    }

    /// Returns whether the game is over (all lives depleted).
    #[inline]
    pub fn is_breakout_game_over(&self) -> bool {
        self.game_over
    }

    /// Returns the number of remaining lives.
    #[inline]
    pub fn remaining_lives(&self) -> u32 {
        self.lives
    }

    /// Returns the number of active remaining bricks.
    #[inline]
    pub fn active_brick_count(&self) -> usize {
        self.active_bricks
    }

    /// Returns current cumulative player score.
    #[inline]
    pub fn current_score(&self) -> u32 {
        self.score
    }
}

/// Helper function to initialize a new Breakout game.
#[inline]
pub fn new_game(
    config: BreakoutConfig,
    initial_prng_seed: u64,
) -> Result<BreakoutState, BreakoutError> {
    BreakoutState::init_breakout_state(config, initial_prng_seed)
}

/// Helper function to validate Breakout state invariants.
#[inline]
pub fn validate_state(state: &BreakoutState) -> Result<(), BreakoutError> {
    state.check_breakout_state_validity()
}

/// Computes the bounding rectangle for a brick at the specified row and column.
#[inline]
pub fn brick_bounding_rect(config: &BreakoutConfig, row: u8, col: u8) -> Rect {
    let x = config.brick_side_margin + (col as f64) * (config.brick_width + config.brick_gap);
    let y = config.brick_top_offset + (row as f64) * (config.brick_height + config.brick_gap);
    Rect::new(x, y, config.brick_width, config.brick_height)
}

/// Deterministic 64-bit pseudo-random number generator step (XorShift64).
#[inline]
pub fn deterministic_prng_next(state: &mut u64) -> u64 {
    let mut x = *state;
    if x == 0 {
        x = 0x853c_49e6_748f_ea9b;
    }
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// Computes a deterministic launch deflection angle in `[-max_angle_rad, max_angle_rad]`.
pub fn deterministic_launch_angle(prng: &mut u64, max_angle_rad: f64) -> f64 {
    let rand_val = deterministic_prng_next(prng);
    let norm = ((rand_val >> 11) as f64) / ((1u64 << 53) as f64);
    let factor = norm * 2.0 - 1.0;
    factor * max_angle_rad
}

enum CollisionTarget {
    Paddle,
    Brick(usize),
}

/// Advances simulation state by a single deterministic fixed timestep tick.
///
/// Uses continuous swept collision detection against paddle and active bricks to prevent
/// tunneling and triggers audio presets via [`arcade_sfx::SfxBank`].
pub fn step_breakout(
    state: &mut BreakoutState,
    input: &BreakoutInput,
    sfx: &mut SfxBank<4>,
) -> Result<BreakoutFrameEvents, BreakoutError> {
    state.check_breakout_state_validity()?;

    if state.game_over {
        return Ok(BreakoutFrameEvents {
            game_over: true,
            ..BreakoutFrameEvents::empty_breakout_events()
        });
    }

    let dt = state.config.fixed_dt_seconds;
    let max_paddle_x = (state.config.arena_width - state.config.paddle_width).max(0.0);

    // 1. Paddle movement
    match input.movement {
        PaddleMove::Left => {
            state.paddle_x = (state.paddle_x - state.config.paddle_speed * dt).max(0.0);
        }
        PaddleMove::Right => {
            state.paddle_x = (state.paddle_x + state.config.paddle_speed * dt).min(max_paddle_x);
        }
        PaddleMove::Hold => {}
    }

    let mut events = BreakoutFrameEvents::empty_breakout_events();

    // 2. Ball attachment and launch handler
    if state.ball_attached {
        let paddle_rect = state.paddle_bounding_rect();
        state.ball_pos = Point::new(
            state.paddle_x + state.config.paddle_width * 0.5,
            paddle_rect.y - state.config.ball_radius,
        );
        state.ball_vel = Point::new(0.0, 0.0);

        if input.launch {
            state.ball_attached = false;
            let angle = deterministic_launch_angle(
                &mut state.prng_seed,
                state.config.max_deflection_angle_rad * 0.5,
            );
            let speed = state.config.ball_initial_speed;
            state.ball_vel = Point::new(speed * angle.sin(), -speed * angle.cos());
        }
        return Ok(events);
    }

    // 3. Continuous swept collision detection
    let disp = state.ball_vel.scale(dt);
    let ball_circle =
        Circle::new(state.ball_pos, state.config.ball_radius).map_err(BreakoutError::Collision)?;

    let mut earliest_hit: Option<(SweepHit, CollisionTarget)> = None;

    // Check paddle collision (only when moving downward)
    if state.ball_vel.y > 0.0 {
        let paddle_rect = state.paddle_bounding_rect();
        if let Some(hit) = sweep_circle_aabb(ball_circle, disp, paddle_rect)
            .map_err(BreakoutError::Collision)?
        {
            if hit.toi <= 1.0 {
                earliest_hit = Some((hit, CollisionTarget::Paddle));
            }
        }
    }

    // Check active brick collisions
    for r in 0..state.config.brick_rows {
        for c in 0..state.config.brick_cols {
            let idx = r * MAX_BRICK_COLS + c;
            if state.bricks[idx].is_brick_active() {
                let b_rect = brick_bounding_rect(&state.config, r as u8, c as u8);
                if let Some(hit) = sweep_circle_aabb(ball_circle, disp, b_rect)
                    .map_err(BreakoutError::Collision)?
                {
                    if hit.toi <= 1.0
                        && earliest_hit.as_ref().is_none_or(|(e, _)| hit.toi < e.toi)
                    {
                        earliest_hit = Some((hit, CollisionTarget::Brick(idx)));
                    }
                }
            }
        }
    }

    // 4. Collision resolution
    if let Some((hit, target)) = earliest_hit {
        match target {
            CollisionTarget::Paddle => {
                events.paddle_hit = true;
                state.combo_counter = 0;
                let _ = sfx.trigger_bank_preset(SfxPreset::PongPaddleHit);

                let hit_x = state.ball_pos.x + disp.x * hit.toi;
                let paddle_center = state.paddle_x + state.config.paddle_width * 0.5;
                let half_w = state.config.paddle_width * 0.5;
                let rel = if half_w > 0.0 {
                    ((hit_x - paddle_center) / half_w).clamp(-1.0, 1.0)
                } else {
                    0.0
                };
                let angle = rel * state.config.max_deflection_angle_rad;
                let cur_spd = (state.ball_vel.magnitude() + state.config.ball_speed_increment)
                    .min(state.config.ball_max_speed);
                state.ball_vel = Point::new(cur_spd * angle.sin(), -cur_spd * angle.cos());

                let rem_t = (1.0 - hit.toi).max(0.0);
                let paddle_y = state.config.arena_height
                    - state.config.paddle_bottom_inset
                    - state.config.paddle_height;
                let new_y = paddle_y - state.config.ball_radius - 0.01;
                state.ball_pos = Point::new(hit_x, new_y).add(state.ball_vel.scale(rem_t * dt));
            }
            CollisionTarget::Brick(idx) => {
                let row = state.bricks[idx].row;
                let _ = sfx.trigger_bank_preset(SfxPreset::BreakoutBrickHit { row });
                events.brick_hit_row = Some(row);

                state.bricks[idx].hits_remaining =
                    state.bricks[idx].hits_remaining.saturating_sub(1);
                let base_pts = state.bricks[idx].points;

                state.combo_counter = state.combo_counter.saturating_add(1);
                let combo_mult = state.combo_counter;
                let pts_earned = base_pts.saturating_mul(combo_mult);
                state.score = state.score.saturating_add(pts_earned);
                events.points_scored = pts_earned;
                events.combo_count = state.combo_counter;

                if state.bricks[idx].hits_remaining == 0 {
                    state.active_bricks = state.active_bricks.saturating_sub(1);
                    if state.active_bricks == 0 {
                        events.level_cleared = true;
                        state.level = state.level.saturating_add(1);
                        state.populate_level_bricks(state.level);
                        state.reset_ball_on_paddle();
                        return Ok(events);
                    }
                }

                // Deterministic surface reflection
                let normal = if hit.normal.magnitude_squared() > 1e-12 {
                    hit.normal
                } else {
                    Point::new(0.0, 1.0)
                };
                state.ball_vel = reflect_velocity(state.ball_vel, normal, 1.0)
                    .map_err(BreakoutError::Collision)?;

                let rem_t = (1.0 - hit.toi).max(0.0);
                let hit_pos = state.ball_pos.add(disp.scale(hit.toi));
                state.ball_pos =
                    hit_pos.add(normal.scale(0.01)).add(state.ball_vel.scale(rem_t * dt));
            }
        }
    } else {
        state.ball_pos = state.ball_pos.add(disp);
    }

    // 5. Wall bounce checks (left, right, top) and bottom gutter
    let radius = state.config.ball_radius;
    if state.ball_pos.x - radius <= 0.0 {
        state.ball_pos.x = radius;
        state.ball_vel.x = state.ball_vel.x.abs();
        events.wall_bounce = true;
        let _ = sfx.trigger_bank_preset(SfxPreset::PongWallBounce);
    } else if state.ball_pos.x + radius >= state.config.arena_width {
        state.ball_pos.x = state.config.arena_width - radius;
        state.ball_vel.x = -state.ball_vel.x.abs();
        events.wall_bounce = true;
        let _ = sfx.trigger_bank_preset(SfxPreset::PongWallBounce);
    }

    if state.ball_pos.y - radius <= 0.0 {
        state.ball_pos.y = radius;
        state.ball_vel.y = state.ball_vel.y.abs();
        events.wall_bounce = true;
        let _ = sfx.trigger_bank_preset(SfxPreset::PongWallBounce);
    } else if state.ball_pos.y >= state.config.arena_height {
        // Exited bottom: lose a life!
        events.life_lost = true;
        state.combo_counter = 0;
        let _ = sfx.trigger_bank_preset(SfxPreset::BreakoutLoseLife);
        state.lives = state.lives.saturating_sub(1);
        if state.lives == 0 {
            state.game_over = true;
            events.game_over = true;
        } else {
            state.reset_ball_on_paddle();
        }
    }

    Ok(events)
}

/// Result returned from fixed-timestep game clock stepping.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClockBreakoutResult {
    /// Clock accumulator stepping report.
    pub clock_step: ClockStepResult,
    /// Number of fixed simulation steps executed during this frame.
    pub steps_simulated: u32,
    /// Aggregated events preserved across all simulation steps executed.
    pub last_frame_events: BreakoutFrameEvents,
}

/// Advances the Breakout simulation using a deterministic integer-nanosecond [`GameClockState`].
pub fn step_breakout_with_clock(
    clock: &mut GameClockState,
    raw_delta_ns: u64,
    state: &mut BreakoutState,
    input: &BreakoutInput,
    sfx: &mut SfxBank<4>,
) -> Result<ClockBreakoutResult, BreakoutError> {
    let clock_step = advance_frame(clock, raw_delta_ns).map_err(BreakoutError::Clock)?;
    *clock = clock_step.state;

    let mut last_frame_events = BreakoutFrameEvents::empty_breakout_events();
    let mut steps_simulated = 0;

    for _ in 0..clock_step.steps_to_run {
        let step_ev = step_breakout(state, input, sfx)?;
        last_frame_events.paddle_hit |= step_ev.paddle_hit;
        last_frame_events.wall_bounce |= step_ev.wall_bounce;
        if step_ev.brick_hit_row.is_some() {
            last_frame_events.brick_hit_row = step_ev.brick_hit_row;
        }
        last_frame_events.life_lost |= step_ev.life_lost;
        last_frame_events.level_cleared |= step_ev.level_cleared;
        last_frame_events.game_over |= step_ev.game_over;
        last_frame_events.points_scored =
            last_frame_events.points_scored.saturating_add(step_ev.points_scored);
        last_frame_events.combo_count = step_ev.combo_count;
        steps_simulated += 1;
    }

    Ok(ClockBreakoutResult {
        clock_step,
        steps_simulated,
        last_frame_events,
    })
}

/// Seven-segment bitmasks for digits 0..=9.
const SEVEN_SEGMENT_DIGITS: [u8; 10] = [
    0b0011_1111, // 0
    0b0000_0110, // 1
    0b0101_1011, // 2
    0b0100_1111, // 3
    0b0110_0110, // 4
    0b0110_1101, // 5
    0b0111_1101, // 6
    0b0000_0111, // 7
    0b0111_1111, // 8
    0b0110_1111, // 9
];

fn emit_digit_sprites(
    digit: u32,
    (x, y, w, h): (f64, f64, f64, f64),
    base_id: u32,
    out: &mut [SpriteItem],
    written: &mut usize,
) {
    let mask = SEVEN_SEGMENT_DIGITS[(digit % 10) as usize];
    let t = 3.0;
    let half_h = (h - t) * 0.5;

    let segments: [(f64, f64, f64, f64); 7] = [
        (x, y, w, t),
        (x + w - t, y, t, half_h + t),
        (x + w - t, y + half_h, t, half_h + t),
        (x, y + h - t, w, t),
        (x, y + half_h, t, half_h + t),
        (x, y, t, half_h + t),
        (x, y + half_h, w, t),
    ];

    for (seg_idx, &(sx, sy, sw, sh)) in segments.iter().enumerate() {
        if (mask & (1 << seg_idx)) != 0 && *written < out.len() {
            out[*written] = SpriteQuad::new(base_id + seg_idx as u32, sx, sy, sw, sh)
                .with_color([255, 255, 255])
                .with_z_priority(25);
            *written += 1;
        }
    }
}

/// Emits zero-allocation retained-mode quad sprites for the paddle, ball,
/// active bricks with row colors, score, and lives HUD into a preallocated slice.
///
/// Returns the number of sprites written into `out`.
pub fn build_breakout_sprites(state: &BreakoutState, out: &mut [SpriteItem]) -> usize {
    if out.is_empty() {
        return 0;
    }
    let mut written = 0;

    // 1. Paddle
    if written < out.len() {
        let paddle_rect = state.paddle_bounding_rect();
        out[written] = SpriteQuad::new(
            1,
            paddle_rect.x,
            paddle_rect.y,
            paddle_rect.width,
            paddle_rect.height,
        )
        .with_color([200, 220, 255])
        .with_z_priority(10);
        written += 1;
    }

    // 2. Ball
    if written < out.len() {
        let diam = state.config.ball_radius * 2.0;
        out[written] = SpriteQuad::new(
            2,
            state.ball_pos.x - state.config.ball_radius,
            state.ball_pos.y - state.config.ball_radius,
            diam,
            diam,
        )
        .with_color([255, 255, 255])
        .with_z_priority(20);
        written += 1;
    }

    // 3. Active bricks
    let rows = state.config.brick_rows.min(MAX_BRICK_ROWS);
    let cols = state.config.brick_cols.min(MAX_BRICK_COLS);
    for r in 0..rows {
        for c in 0..cols {
            if written >= out.len() {
                break;
            }
            let idx = r * MAX_BRICK_COLS + c;
            let brick = &state.bricks[idx];
            if brick.is_brick_active() {
                let b_rect = brick_bounding_rect(&state.config, r as u8, c as u8);
                // If durable brick took damage, tint color lighter
                let color = if brick.max_hits > 1 && brick.hits_remaining == 1 {
                    [
                        brick.color_rgb[0].saturating_add(50),
                        brick.color_rgb[1].saturating_add(50),
                        brick.color_rgb[2].saturating_add(50),
                    ]
                } else {
                    brick.color_rgb
                };

                out[written] = SpriteQuad::new(
                    100 + idx as u32,
                    b_rect.x,
                    b_rect.y,
                    b_rect.width,
                    b_rect.height,
                )
                .with_color(color)
                .with_z_priority(5);
                written += 1;
            }
        }
    }

    // 4. Score HUD digits (top left, base ID 500..)
    let digit_w = 16.0;
    let digit_h = 26.0;
    let digit_y = 20.0;
    let mut score_val = state.score;
    let mut digits = [0u32; 6];
    let mut num_digits = 0;
    if score_val == 0 {
        digits[0] = 0;
        num_digits = 1;
    } else {
        while score_val > 0 && num_digits < 6 {
            digits[num_digits] = score_val % 10;
            score_val /= 10;
            num_digits += 1;
        }
    }

    let start_x = 40.0;
    for i in 0..num_digits {
        let d = digits[num_digits - 1 - i];
        let dx = start_x + (i as f64) * (digit_w + 4.0);
        emit_digit_sprites(
            d,
            (dx, digit_y, digit_w, digit_h),
            500 + (i as u32) * 10,
            out,
            &mut written,
        );
    }

    // 5. Lives HUD (top right, base ID 600..)
    let life_icon_w = 18.0;
    let life_icon_h = 8.0;
    let lives_to_draw = state.lives.min(5) as usize;
    for i in 0..lives_to_draw {
        if written >= out.len() {
            break;
        }
        let lx = state.config.arena_width - 40.0 - (i as f64) * (life_icon_w + 6.0);
        out[written] = SpriteQuad::new(600 + i as u32, lx, digit_y + 8.0, life_icon_w, life_icon_h)
            .with_color([220, 20, 60])
            .with_z_priority(15);
        written += 1;
    }

    written
}

/// Lowers current Breakout state into a universal [`PaintScene`] IR via [`paint_batch_tiles`].
pub fn lower_breakout_to_paint_scene(state: &BreakoutState) -> Result<PaintScene, BreakoutError> {
    let mut sprites = [SpriteQuad::new(0, 0.0, 0.0, 0.0, 0.0); MAX_BREAKOUT_SPRITES];
    let count = build_breakout_sprites(state, &mut sprites);
    lower_sprites_to_paint_scene(
        state.config.arena_width,
        state.config.arena_height,
        "#000000",
        &sprites[..count],
    )
    .map_err(BreakoutError::Paint)
}

/// Mixes the active arcade audio voices and streams into a low-latency PCM ring buffer.
pub fn render_breakout_audio_into<const N: usize>(
    bank: &mut SfxBank<4>,
    ring: &mut PcmRingBuffer<N>,
    scratch: &mut [i16],
) -> Result<usize, BreakoutError> {
    bank.route_bank_to_ring_buffer(ring, scratch)
        .map_err(BreakoutError::Audio)
}
