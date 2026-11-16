//! # Deterministic 2-Player Arcade Pong State Machine & PaintVM Renderer (GAME13)
//!
//! Reusable pure-Rust implementation of the iconic 1972 arcade classic *Pong*
//! (Allan Alcorn / Atari Inc.).
//!
//! Features:
//! - Exact continuous swept circle-AABB collision detection ([`collision2d`]) to eliminate
//!   high-speed tunneling through paddles or boundary walls (`CWE-190`, `CWE-20`).
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

/// Default court width in logical units.
pub const DEFAULT_COURT_WIDTH: f64 = 800.0;

/// Default court height in logical units.
pub const DEFAULT_COURT_HEIGHT: f64 = 600.0;

/// Default paddle width in logical units.
pub const DEFAULT_PADDLE_WIDTH: f64 = 16.0;

/// Default paddle height in logical units.
pub const DEFAULT_PADDLE_HEIGHT: f64 = 80.0;

/// Default paddle speed in logical units per second.
pub const DEFAULT_PADDLE_SPEED: f64 = 420.0;

/// Default paddle inset distance from court lateral edges.
pub const DEFAULT_PADDLE_INSET: f64 = 32.0;

/// Default ball radius in logical units.
pub const DEFAULT_BALL_RADIUS: f64 = 8.0;

/// Default initial ball speed in logical units per second.
pub const DEFAULT_BALL_INITIAL_SPEED: f64 = 360.0;

/// Default maximum ball speed in logical units per second.
pub const DEFAULT_BALL_MAX_SPEED: f64 = 850.0;

/// Default speed increment added on each paddle hit.
pub const DEFAULT_BALL_SPEED_INCREMENT: f64 = 24.0;

/// Default maximum deflection angle in radians (~50 degrees).
pub const DEFAULT_MAX_DEFLECTION_RAD: f64 = 0.8726646259971648;

/// Default winning score to end the match.
pub const DEFAULT_WINNING_SCORE: u32 = 11;

/// Default serve countdown ticks (60 ticks = 1 second at 60Hz).
pub const DEFAULT_SERVE_COUNTDOWN_TICKS: u32 = 60;

/// Default fixed timestep in seconds (60 Hz).
pub const DEFAULT_FIXED_DT_SECONDS: f64 = 1.0 / 60.0;

/// Maximum number of sprites generated for a Pong frame (net, paddles, ball, score digits).
pub const MAX_PONG_SPRITES: usize = 128;

/// Specific error conditions encountered in Pong simulation, physics, or rendering.
#[derive(Debug, Clone, PartialEq)]
pub enum PongError {
    /// Pong configuration parameters failed validation (non-finite or invalid bounds).
    InvalidConfig,
    /// Pong state fields contain non-finite numbers (NaN or Infinity).
    NonFiniteState,
    /// State coordinates or dimensions exceed allowable physical court bounds.
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

impl std::fmt::Display for PongError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig => write!(f, "invalid pong configuration parameters"),
            Self::NonFiniteState => write!(f, "pong state contains non-finite float (NaN or Inf)"),
            Self::StateOutOfBounds => write!(f, "coordinates exceed allowable court or physical bounds"),
            Self::NegativeDimension => write!(f, "dimension or coordinate cannot be negative"),
            Self::Collision(e) => write!(f, "collision error: {e}"),
            Self::Clock(e) => write!(f, "game clock error: {e}"),
            Self::Paint(e) => write!(f, "paint batch lowering error: {e:?}"),
            Self::Audio(e) => write!(f, "arcade sfx error: {e}"),
        }
    }
}

impl std::error::Error for PongError {}

/// Physical dimensions, movement speeds, and match rules for Pong.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PongConfig {
    /// Court width in logical scene units.
    pub court_width: f64,
    /// Court height in logical scene units.
    pub court_height: f64,
    /// Paddle width in logical units.
    pub paddle_width: f64,
    /// Paddle height in logical units.
    pub paddle_height: f64,
    /// Paddle movement speed in units per second.
    pub paddle_speed: f64,
    /// Horizontal inset of paddles from left/right walls.
    pub paddle_inset: f64,
    /// Ball radius in logical units.
    pub ball_radius: f64,
    /// Initial ball speed when served.
    pub ball_initial_speed: f64,
    /// Absolute maximum speed cap for the ball.
    pub ball_max_speed: f64,
    /// Speed bonus added to the ball on each paddle deflection.
    pub ball_speed_increment: f64,
    /// Maximum bounce deflection angle in radians from horizontal normal.
    pub max_deflection_angle_rad: f64,
    /// Score required to win the match.
    pub winning_score: u32,
    /// Ticks to pause in neutral serve state between points.
    pub serve_countdown_ticks: u32,
    /// Simulation timestep delta in seconds per tick.
    pub fixed_dt_seconds: f64,
}

impl PongConfig {
    /// Constructs default configuration matching classic 60 Hz arcade Pong specifications.
    pub const fn standard() -> Self {
        Self {
            court_width: DEFAULT_COURT_WIDTH,
            court_height: DEFAULT_COURT_HEIGHT,
            paddle_width: DEFAULT_PADDLE_WIDTH,
            paddle_height: DEFAULT_PADDLE_HEIGHT,
            paddle_speed: DEFAULT_PADDLE_SPEED,
            paddle_inset: DEFAULT_PADDLE_INSET,
            ball_radius: DEFAULT_BALL_RADIUS,
            ball_initial_speed: DEFAULT_BALL_INITIAL_SPEED,
            ball_max_speed: DEFAULT_BALL_MAX_SPEED,
            ball_speed_increment: DEFAULT_BALL_SPEED_INCREMENT,
            max_deflection_angle_rad: DEFAULT_MAX_DEFLECTION_RAD,
            winning_score: DEFAULT_WINNING_SCORE,
            serve_countdown_ticks: DEFAULT_SERVE_COUNTDOWN_TICKS,
            fixed_dt_seconds: DEFAULT_FIXED_DT_SECONDS,
        }
    }

    /// Validates configuration parameters against non-finite floats, negative values, and out-of-range bounds.
    pub fn check_validity(&self) -> Result<(), PongError> {
        if !self.court_width.is_finite()
            || !self.court_height.is_finite()
            || !self.paddle_width.is_finite()
            || !self.paddle_height.is_finite()
            || !self.paddle_speed.is_finite()
            || !self.paddle_inset.is_finite()
            || !self.ball_radius.is_finite()
            || !self.ball_initial_speed.is_finite()
            || !self.ball_max_speed.is_finite()
            || !self.ball_speed_increment.is_finite()
            || !self.max_deflection_angle_rad.is_finite()
            || !self.fixed_dt_seconds.is_finite()
        {
            return Err(PongError::NonFiniteState);
        }

        if self.court_width <= 0.0
            || self.court_height <= 0.0
            || self.court_width > MAX_COORD
            || self.court_height > MAX_COORD
            || self.paddle_width <= 0.0
            || self.paddle_width >= self.court_width * 0.5
            || self.paddle_height <= 0.0
            || self.paddle_height >= self.court_height
            || self.paddle_speed <= 0.0
            || self.paddle_inset < 0.0
            || self.paddle_inset >= self.court_width * 0.4
            || self.paddle_inset + self.paddle_width >= self.court_width * 0.5
            || self.ball_radius <= 0.0
            || self.ball_radius >= self.court_height * 0.25
            || self.ball_radius >= self.court_width * 0.5
            || self.ball_initial_speed <= 0.0
            || self.ball_initial_speed > self.ball_max_speed
            || self.ball_max_speed > MAX_COORD
            || self.ball_speed_increment < 0.0
            || self.max_deflection_angle_rad <= 0.0
            || self.max_deflection_angle_rad >= std::f64::consts::FRAC_PI_2
            || self.winning_score == 0
            || self.serve_countdown_ticks > 3600
            || self.fixed_dt_seconds <= 0.0
            || self.fixed_dt_seconds > 1.0
        {
            return Err(PongError::InvalidConfig);
        }

        Ok(())
    }
}

impl Default for PongConfig {
    fn default() -> Self {
        Self::standard()
    }
}

/// Helper function to create default Pong configuration.
#[inline]
pub fn default_config() -> PongConfig {
    PongConfig::standard()
}

/// Helper function to validate Pong configuration.
#[inline]
pub fn validate_config(config: &PongConfig) -> Result<(), PongError> {
    config.check_validity()
}

/// Designates left or right player side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaddleSide {
    /// Left player paddle.
    Left,
    /// Right player paddle.
    Right,
}

/// Discrete paddle movement request for a single tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaddleAction {
    /// Do not move paddle.
    Hold,
    /// Move paddle upward (decreasing Y).
    Up,
    /// Move paddle downward (increasing Y).
    Down,
}

/// Controller policy for a paddle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaddleControl {
    /// Manual control specifying an explicit tick action.
    Manual(PaddleAction),
    /// Autonomous AI control tracking ball trajectory.
    Ai,
}

/// Frame input specification controlling left and right paddles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PongInput {
    /// Control mode for left paddle.
    pub left: PaddleControl,
    /// Control mode for right paddle.
    pub right: PaddleControl,
}

impl PongInput {
    /// 2-player local human match input.
    pub const fn player_vs_player(left: PaddleAction, right: PaddleAction) -> Self {
        Self {
            left: PaddleControl::Manual(left),
            right: PaddleControl::Manual(right),
        }
    }

    /// Single player human (left) vs AI opponent (right).
    pub const fn player_vs_ai(left: PaddleAction) -> Self {
        Self {
            left: PaddleControl::Manual(left),
            right: PaddleControl::Ai,
        }
    }

    /// Autonomous demonstration match (AI vs AI).
    pub const fn ai_vs_ai() -> Self {
        Self {
            left: PaddleControl::Ai,
            right: PaddleControl::Ai,
        }
    }

    /// Neutral input holding both paddles stationary.
    pub const fn idle_input() -> Self {
        Self {
            left: PaddleControl::Manual(PaddleAction::Hold),
            right: PaddleControl::Manual(PaddleAction::Hold),
        }
    }
}

impl Default for PongInput {
    fn default() -> Self {
        Self::idle_input()
    }
}

/// Event record emitted when a player scores a point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoreEvent {
    /// Player who scored the point.
    pub scorer: PaddleSide,
    /// Current score of left player after the point.
    pub left_score: u32,
    /// Current score of right player after the point.
    pub right_score: u32,
}

/// Simulation events produced during a single fixed-timestep tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PongFrameEvents {
    /// Paddle hit by the ball during this tick, if any.
    pub paddle_hit: Option<PaddleSide>,
    /// Whether the ball bounced off top or bottom court wall.
    pub wall_bounce: bool,
    /// Score event produced during this tick, if a point was conceded.
    pub score_event: Option<ScoreEvent>,
    /// Whether the game is finished (winning score reached).
    pub game_over: bool,
    /// Current continuous rally count (paddle hits since last serve).
    pub rally_count: u32,
}

impl PongFrameEvents {
    /// Constructs an empty event structure with no collisions or scoring.
    pub const fn empty_events() -> Self {
        Self {
            paddle_hit: None,
            wall_bounce: false,
            score_event: None,
            game_over: false,
            rally_count: 0,
        }
    }
}

impl Default for PongFrameEvents {
    fn default() -> Self {
        Self::empty_events()
    }
}

/// Complete mutable simulation state for a Pong match.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PongState {
    /// Active physical and rule configuration.
    pub config: PongConfig,
    /// Top-left Y coordinate of the left paddle.
    pub left_paddle_y: f64,
    /// Top-left Y coordinate of the right paddle.
    pub right_paddle_y: f64,
    /// Center coordinates of the ball.
    pub ball_pos: Point,
    /// Velocity vector of the ball in logical units per second.
    pub ball_vel: Point,
    /// Cumulative points scored by left player.
    pub left_score: u32,
    /// Cumulative points scored by right player.
    pub right_score: u32,
    /// Number of consecutive paddle hits in the active rally.
    pub rally_counter: u32,
    /// Ticks remaining before ball launches from center court.
    pub serve_countdown: u32,
    /// Direction of next serve (true = towards right, false = towards left).
    pub serve_towards_right: bool,
    /// Deterministic 64-bit PRNG seed for serve angles.
    pub prng_seed: u64,
    /// True if match is concluded.
    pub game_over: bool,
    /// Winning side if match has concluded.
    pub winning_side: Option<PaddleSide>,
}

impl PongState {
    /// Constructs a fresh match state with centered paddles, neutral serve state, and zero scores.
    pub fn init_match(config: PongConfig, initial_prng_seed: u64) -> Result<Self, PongError> {
        config.check_validity()?;
        let seed = if initial_prng_seed == 0 {
            0x517c_c1b7_2722_0a95
        } else {
            initial_prng_seed
        };
        let center_y = (config.court_height - config.paddle_height) * 0.5;
        let ball_pos = Point::new(config.court_width * 0.5, config.court_height * 0.5);
        let ball_vel = Point::new(0.0, 0.0);

        let state = Self {
            config,
            left_paddle_y: center_y,
            right_paddle_y: center_y,
            ball_pos,
            ball_vel,
            left_score: 0,
            right_score: 0,
            rally_counter: 0,
            serve_countdown: config.serve_countdown_ticks,
            serve_towards_right: true,
            prng_seed: seed,
            game_over: false,
            winning_side: None,
        };
        state.check_state_validity()?;
        Ok(state)
    }

    /// Validates state invariants, float finiteness, and coordinate bounds.
    pub fn check_state_validity(&self) -> Result<(), PongError> {
        self.config.check_validity()?;

        if !self.left_paddle_y.is_finite()
            || !self.right_paddle_y.is_finite()
            || !self.ball_pos.x.is_finite()
            || !self.ball_pos.y.is_finite()
            || !self.ball_vel.x.is_finite()
            || !self.ball_vel.y.is_finite()
        {
            return Err(PongError::NonFiniteState);
        }

        let max_paddle_y = (self.config.court_height - self.config.paddle_height) + 1.0;
        if self.left_paddle_y < -1.0 || self.left_paddle_y > max_paddle_y {
            return Err(PongError::StateOutOfBounds);
        }
        if self.right_paddle_y < -1.0 || self.right_paddle_y > max_paddle_y {
            return Err(PongError::StateOutOfBounds);
        }

        if self.ball_pos.x.abs() > MAX_COORD || self.ball_pos.y.abs() > MAX_COORD {
            return Err(PongError::StateOutOfBounds);
        }

        if self.ball_vel.magnitude_squared() > MAX_COORD * MAX_COORD {
            return Err(PongError::StateOutOfBounds);
        }

        Ok(())
    }

    /// Resets ball to center court for the next serve countdown.
    pub fn reset_ball_serve(&mut self, towards_right: bool) {
        self.ball_pos = Point::new(self.config.court_width * 0.5, self.config.court_height * 0.5);
        self.ball_vel = Point::new(0.0, 0.0);
        self.serve_countdown = self.config.serve_countdown_ticks;
        self.serve_towards_right = towards_right;
        self.rally_counter = 0;
    }

    /// Returns whether the game has reached the winning score.
    #[inline]
    pub fn is_game_finished(&self) -> bool {
        self.game_over
    }

    /// Returns the winning player side if the match has concluded.
    #[inline]
    pub fn winning_player(&self) -> Option<PaddleSide> {
        self.winning_side
    }

    /// Returns the bounding box rectangle for the left paddle.
    #[inline]
    pub fn left_paddle_rect(&self) -> Rect {
        Rect::new(
            self.config.paddle_inset,
            self.left_paddle_y,
            self.config.paddle_width,
            self.config.paddle_height,
        )
    }

    /// Returns the bounding box rectangle for the right paddle.
    #[inline]
    pub fn right_paddle_rect(&self) -> Rect {
        Rect::new(
            self.config.court_width - self.config.paddle_inset - self.config.paddle_width,
            self.right_paddle_y,
            self.config.paddle_width,
            self.config.paddle_height,
        )
    }

    /// Returns the circle geometry representing the ball.
    #[inline]
    pub fn ball_circle(&self) -> Circle {
        Circle {
            center: self.ball_pos,
            radius: self.config.ball_radius,
        }
    }
}

/// Helper function to initialize a new Pong match.
#[inline]
pub fn new_game(config: PongConfig, initial_prng_seed: u64) -> Result<PongState, PongError> {
    PongState::init_match(config, initial_prng_seed)
}

/// Helper function to validate Pong state invariants.
#[inline]
pub fn validate_state(state: &PongState) -> Result<(), PongError> {
    state.check_state_validity()
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

/// Computes a deterministic serve deflection angle in `[-max_angle_rad, max_angle_rad]`.
pub fn deterministic_serve_angle(prng: &mut u64, max_angle_rad: f64) -> f64 {
    let rand_val = deterministic_prng_next(prng);
    let norm = ((rand_val >> 11) as f64) / ((1u64 << 53) as f64);
    let factor = norm * 2.0 - 1.0;
    factor * max_angle_rad
}

/// Advances simulation state by a single deterministic fixed timestep tick.
///
/// Uses continuous swept collision detection against paddles to prevent tunneling
/// and triggers audio presets via [`arcade_sfx::SfxBank`].
pub fn step_pong(
    state: &mut PongState,
    input: &PongInput,
    sfx: &mut SfxBank<4>,
) -> Result<PongFrameEvents, PongError> {
    state.check_state_validity()?;

    if state.game_over {
        return Ok(PongFrameEvents {
            paddle_hit: None,
            wall_bounce: false,
            score_event: None,
            game_over: true,
            rally_count: state.rally_counter,
        });
    }

    let dt = state.config.fixed_dt_seconds;
    let max_paddle_y = (state.config.court_height - state.config.paddle_height).max(0.0);

    // 1. Move left paddle
    match input.left {
        PaddleControl::Manual(action) => match action {
            PaddleAction::Up => state.left_paddle_y -= state.config.paddle_speed * dt,
            PaddleAction::Down => state.left_paddle_y += state.config.paddle_speed * dt,
            PaddleAction::Hold => {}
        },
        PaddleControl::Ai => {
            let paddle_center = state.left_paddle_y + state.config.paddle_height * 0.5;
            let deadband = state.config.paddle_height * 0.15;
            if state.ball_pos.y < paddle_center - deadband {
                state.left_paddle_y -= state.config.paddle_speed * dt;
            } else if state.ball_pos.y > paddle_center + deadband {
                state.left_paddle_y += state.config.paddle_speed * dt;
            }
        }
    }
    state.left_paddle_y = state.left_paddle_y.clamp(0.0, max_paddle_y);

    // 2. Move right paddle
    match input.right {
        PaddleControl::Manual(action) => match action {
            PaddleAction::Up => state.right_paddle_y -= state.config.paddle_speed * dt,
            PaddleAction::Down => state.right_paddle_y += state.config.paddle_speed * dt,
            PaddleAction::Hold => {}
        },
        PaddleControl::Ai => {
            let paddle_center = state.right_paddle_y + state.config.paddle_height * 0.5;
            let deadband = state.config.paddle_height * 0.15;
            if state.ball_pos.y < paddle_center - deadband {
                state.right_paddle_y -= state.config.paddle_speed * dt;
            } else if state.ball_pos.y > paddle_center + deadband {
                state.right_paddle_y += state.config.paddle_speed * dt;
            }
        }
    }
    state.right_paddle_y = state.right_paddle_y.clamp(0.0, max_paddle_y);

    // 3. Serve countdown handler
    if state.serve_countdown > 0 {
        state.serve_countdown -= 1;
        if state.serve_countdown == 0 {
            let angle = deterministic_serve_angle(
                &mut state.prng_seed,
                state.config.max_deflection_angle_rad,
            );
            let speed = state.config.ball_initial_speed;
            let dir_x = if state.serve_towards_right { 1.0 } else { -1.0 };
            state.ball_vel = Point::new(dir_x * speed * angle.cos(), speed * angle.sin());
        }
        return Ok(PongFrameEvents {
            paddle_hit: None,
            wall_bounce: false,
            score_event: None,
            game_over: false,
            rally_count: 0,
        });
    }

    // 4. Ball motion and continuous swept collision
    let disp = state.ball_vel.scale(dt);
    let ball_circle = Circle::new(state.ball_pos, state.config.ball_radius)
        .map_err(PongError::Collision)?;

    let mut paddle_hit_side: Option<PaddleSide> = None;
    let mut wall_bounce_occurred = false;
    let mut score_ev: Option<ScoreEvent> = None;

    if state.ball_vel.x < 0.0 {
        let left_rect = state.left_paddle_rect();
        if let Some(hit) = sweep_circle_aabb(ball_circle, disp, left_rect)
            .map_err(PongError::Collision)?
        {
            if hit.toi <= 1.0 {
                paddle_hit_side = Some(PaddleSide::Left);
                state.rally_counter = state.rally_counter.saturating_add(1);
                let _ = sfx.trigger_bank_preset(SfxPreset::PongPaddleHit);

                let hit_y = state.ball_pos.y + disp.y * hit.toi;
                let paddle_center = state.left_paddle_y + state.config.paddle_height * 0.5;
                let half_h = state.config.paddle_height * 0.5;
                let rel = if half_h > 0.0 {
                    ((hit_y - paddle_center) / half_h).clamp(-1.0, 1.0)
                } else {
                    0.0
                };
                let angle = rel * state.config.max_deflection_angle_rad;
                let cur_spd = (state.ball_vel.magnitude() + state.config.ball_speed_increment)
                    .min(state.config.ball_max_speed);
                state.ball_vel = Point::new(cur_spd * angle.cos(), cur_spd * angle.sin());

                let rem_t = (1.0 - hit.toi).max(0.0);
                let new_x = left_rect.x + left_rect.width + state.config.ball_radius + 0.01;
                state.ball_pos = Point::new(new_x, hit_y).add(state.ball_vel.scale(rem_t * dt));
            } else {
                state.ball_pos = state.ball_pos.add(disp);
            }
        } else {
            state.ball_pos = state.ball_pos.add(disp);
        }
    } else if state.ball_vel.x > 0.0 {
        let right_rect = state.right_paddle_rect();
        if let Some(hit) = sweep_circle_aabb(ball_circle, disp, right_rect)
            .map_err(PongError::Collision)?
        {
            if hit.toi <= 1.0 {
                paddle_hit_side = Some(PaddleSide::Right);
                state.rally_counter = state.rally_counter.saturating_add(1);
                let _ = sfx.trigger_bank_preset(SfxPreset::PongPaddleHit);

                let hit_y = state.ball_pos.y + disp.y * hit.toi;
                let paddle_center = state.right_paddle_y + state.config.paddle_height * 0.5;
                let half_h = state.config.paddle_height * 0.5;
                let rel = if half_h > 0.0 {
                    ((hit_y - paddle_center) / half_h).clamp(-1.0, 1.0)
                } else {
                    0.0
                };
                let angle = rel * state.config.max_deflection_angle_rad;
                let cur_spd = (state.ball_vel.magnitude() + state.config.ball_speed_increment)
                    .min(state.config.ball_max_speed);
                state.ball_vel = Point::new(-cur_spd * angle.cos(), cur_spd * angle.sin());

                let rem_t = (1.0 - hit.toi).max(0.0);
                let new_x = right_rect.x - state.config.ball_radius - 0.01;
                state.ball_pos = Point::new(new_x, hit_y).add(state.ball_vel.scale(rem_t * dt));
            } else {
                state.ball_pos = state.ball_pos.add(disp);
            }
        } else {
            state.ball_pos = state.ball_pos.add(disp);
        }
    } else {
        state.ball_pos = state.ball_pos.add(disp);
    }

    // 5. Wall bounce checks (top & bottom boundaries)
    let radius = state.config.ball_radius;
    if state.ball_pos.y - radius <= 0.0 {
        state.ball_pos.y = radius;
        state.ball_vel.y = state.ball_vel.y.abs();
        wall_bounce_occurred = true;
        let _ = sfx.trigger_bank_preset(SfxPreset::PongWallBounce);
    } else if state.ball_pos.y + radius >= state.config.court_height {
        state.ball_pos.y = state.config.court_height - radius;
        state.ball_vel.y = -state.ball_vel.y.abs();
        wall_bounce_occurred = true;
        let _ = sfx.trigger_bank_preset(SfxPreset::PongWallBounce);
    }

    // 6. Scoring checks (ball exits court through lateral walls)
    if state.ball_pos.x <= 0.0 {
        // Exited left: Right player scores!
        state.right_score = state.right_score.saturating_add(1);
        state.rally_counter = 0;
        let _ = sfx.trigger_bank_preset(SfxPreset::PongScore);
        score_ev = Some(ScoreEvent {
            scorer: PaddleSide::Right,
            left_score: state.left_score,
            right_score: state.right_score,
        });

        if state.right_score >= state.config.winning_score {
            state.game_over = true;
            state.winning_side = Some(PaddleSide::Right);
        } else {
            state.reset_ball_serve(false);
        }
    } else if state.ball_pos.x >= state.config.court_width {
        // Exited right: Left player scores!
        state.left_score = state.left_score.saturating_add(1);
        state.rally_counter = 0;
        let _ = sfx.trigger_bank_preset(SfxPreset::PongScore);
        score_ev = Some(ScoreEvent {
            scorer: PaddleSide::Left,
            left_score: state.left_score,
            right_score: state.right_score,
        });

        if state.left_score >= state.config.winning_score {
            state.game_over = true;
            state.winning_side = Some(PaddleSide::Left);
        } else {
            state.reset_ball_serve(true);
        }
    }

    Ok(PongFrameEvents {
        paddle_hit: paddle_hit_side,
        wall_bounce: wall_bounce_occurred,
        score_event: score_ev,
        game_over: state.game_over,
        rally_count: state.rally_counter,
    })
}

/// Result returned from fixed-timestep game clock stepping.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClockPongResult {
    /// Clock accumulator stepping report.
    pub clock_step: ClockStepResult,
    /// Number of fixed simulation steps executed during this frame.
    pub steps_simulated: u32,
    /// Events returned from the final simulation step executed.
    pub last_frame_events: PongFrameEvents,
}

/// Advances the Pong simulation using a deterministic integer-nanosecond [`GameClockState`].
pub fn step_pong_with_clock(
    clock: &mut GameClockState,
    raw_delta_ns: u64,
    state: &mut PongState,
    input: &PongInput,
    sfx: &mut SfxBank<4>,
) -> Result<ClockPongResult, PongError> {
    let clock_step = advance_frame(clock, raw_delta_ns).map_err(PongError::Clock)?;
    *clock = clock_step.state;

    let mut last_frame_events = PongFrameEvents::empty_events();
    let mut steps_simulated = 0;

    for _ in 0..clock_step.steps_to_run {
        let step_ev = step_pong(state, input, sfx)?;
        if step_ev.paddle_hit.is_some() {
            last_frame_events.paddle_hit = step_ev.paddle_hit;
        }
        last_frame_events.wall_bounce |= step_ev.wall_bounce;
        if step_ev.score_event.is_some() {
            last_frame_events.score_event = step_ev.score_event;
        }
        last_frame_events.game_over |= step_ev.game_over;
        last_frame_events.rally_count = step_ev.rally_count;
        steps_simulated += 1;
    }

    Ok(ClockPongResult {
        clock_step,
        steps_simulated,
        last_frame_events,
    })
}

/// Seven-segment bitmasks for digits 0..=9.
///
/// Bit layout:
/// - Bit 0: Top (a)
/// - Bit 1: Top-Right (b)
/// - Bit 2: Bottom-Right (c)
/// - Bit 3: Bottom (d)
/// - Bit 4: Bottom-Left (e)
/// - Bit 5: Top-Left (f)
/// - Bit 6: Middle (g)
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
    let t = 4.0;
    let half_h = (h - t) * 0.5;

    // Segment definitions (x, y, width, height)
    let segments: [(f64, f64, f64, f64); 7] = [
        (x, y, w, t),                      // a: top
        (x + w - t, y, t, half_h + t),     // b: top-right
        (x + w - t, y + half_h, t, half_h + t), // c: bottom-right
        (x, y + h - t, w, t),              // d: bottom
        (x, y + half_h, t, half_h + t),    // e: bottom-left
        (x, y, t, half_h + t),             // f: top-left
        (x, y + half_h, w, t),             // g: middle
    ];

    for (seg_idx, &(sx, sy, sw, sh)) in segments.iter().enumerate() {
        if (mask & (1 << seg_idx)) != 0 && *written < out.len() {
            out[*written] = SpriteQuad::new(base_id + seg_idx as u32, sx, sy, sw, sh)
                .with_color([255, 255, 255]);
            *written += 1;
        }
    }
}

/// Emits zero-allocation retained-mode quad sprites for the court net dashes,
/// left/right paddles, ball, and seven-segment score digits into a preallocated slice.
///
/// Returns the number of sprites written into `out`.
pub fn build_pong_sprites(state: &PongState, out: &mut [SpriteItem]) -> usize {
    if out.is_empty() {
        return 0;
    }
    let mut written = 0;

    // 1. Left paddle
    if written < out.len() {
        out[written] = SpriteQuad::new(
            1,
            state.config.paddle_inset,
            state.left_paddle_y,
            state.config.paddle_width,
            state.config.paddle_height,
        )
        .with_color([255, 255, 255]);
        written += 1;
    }

    // 2. Right paddle
    if written < out.len() {
        let right_x = state.config.court_width - state.config.paddle_inset - state.config.paddle_width;
        out[written] = SpriteQuad::new(
            2,
            right_x,
            state.right_paddle_y,
            state.config.paddle_width,
            state.config.paddle_height,
        )
        .with_color([255, 255, 255]);
        written += 1;
    }

    // 3. Ball
    if written < out.len() {
        let diam = state.config.ball_radius * 2.0;
        out[written] = SpriteQuad::new(
            3,
            state.ball_pos.x - state.config.ball_radius,
            state.ball_pos.y - state.config.ball_radius,
            diam,
            diam,
        )
        .with_color([255, 255, 255]);
        written += 1;
    }

    // 4. Seven-segment score displays
    let digit_w = 24.0;
    let digit_h = 40.0;
    let digit_y = 30.0;

    // Left score
    if state.left_score >= 10 {
        let tens = (state.left_score / 10) % 10;
        let ones = state.left_score % 10;
        let left_center = state.config.court_width * 0.25;
        emit_digit_sprites(
            tens,
            (left_center - digit_w - 4.0, digit_y, digit_w, digit_h),
            500,
            out,
            &mut written,
        );
        emit_digit_sprites(
            ones,
            (left_center + 4.0, digit_y, digit_w, digit_h),
            510,
            out,
            &mut written,
        );
    } else {
        let left_center = state.config.court_width * 0.25 - digit_w * 0.5;
        emit_digit_sprites(
            state.left_score,
            (left_center, digit_y, digit_w, digit_h),
            500,
            out,
            &mut written,
        );
    }

    // Right score
    if state.right_score >= 10 {
        let tens = (state.right_score / 10) % 10;
        let ones = state.right_score % 10;
        let right_center = state.config.court_width * 0.75;
        emit_digit_sprites(
            tens,
            (right_center - digit_w - 4.0, digit_y, digit_w, digit_h),
            600,
            out,
            &mut written,
        );
        emit_digit_sprites(
            ones,
            (right_center + 4.0, digit_y, digit_w, digit_h),
            610,
            out,
            &mut written,
        );
    } else {
        let right_center = state.config.court_width * 0.75 - digit_w * 0.5;
        emit_digit_sprites(
            state.right_score,
            (right_center, digit_y, digit_w, digit_h),
            600,
            out,
            &mut written,
        );
    }

    // 5. Center court net dashes (emitted after critical gameplay/score sprites)
    let dash_width = 4.0;
    let dash_height = 12.0;
    let dash_gap = 12.0;
    let net_x = (state.config.court_width - dash_width) * 0.5;
    let total_step = dash_height + dash_gap;
    let dash_count = (state.config.court_height / total_step) as usize;

    for i in 0..dash_count {
        if written >= out.len() {
            break;
        }
        let y = (i as f64) * total_step + dash_gap * 0.5;
        out[written] = SpriteQuad::new(100 + i as u32, net_x, y, dash_width, dash_height)
            .with_color([180, 180, 180]);
        written += 1;
    }

    written
}

/// Lowers current Pong state into a universal [`PaintScene`] IR via [`paint_batch_tiles`].
pub fn lower_pong_to_paint_scene(state: &PongState) -> Result<PaintScene, PongError> {
    let mut sprites = [SpriteQuad::new(0, 0.0, 0.0, 0.0, 0.0); MAX_PONG_SPRITES];
    let count = build_pong_sprites(state, &mut sprites);
    lower_sprites_to_paint_scene(
        state.config.court_width,
        state.config.court_height,
        "#000000",
        &sprites[..count],
    )
    .map_err(PongError::Paint)
}

/// Mixes the active arcade audio voices and streams into a low-latency PCM ring buffer.
pub fn render_pong_audio_into<const N: usize>(
    bank: &mut SfxBank<4>,
    ring: &mut PcmRingBuffer<N>,
    scratch: &mut [i16],
) -> Result<usize, PongError> {
    bank.route_bank_to_ring_buffer(ring, scratch)
        .map_err(PongError::Audio)
}
