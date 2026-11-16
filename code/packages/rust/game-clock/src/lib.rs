//! # Game Clock: Deterministic Integer-Nanosecond Fixed-Timestep Accumulator & Frame Pacer (GAME10)
//!
//! A zero-runtime-dependency, zero-heap-allocation, deterministic fixed-timestep
//! simulation accumulator and frame pacer designed for cross-platform arcade and retro
//! game engines (Era 1+).
//!
//! ## 1. Theoretical Foundation: "Fix Your Timestep!"
//!
//! Real-time interactive simulations and physical simulations face an inherent tension:
//! - **Real-world render rate (wall clock)** is erratic and variable. It depends on display
//!   refresh rates (60 Hz, 120 Hz, 144 Hz, 240 Hz), OS scheduling, GPU pipeline stalls,
//!   and background thread preemption. Frame deltas fluctuate continuously ($\Delta t \approx 16.6\text{ ms} \pm 4\text{ ms}$).
//! - **Physical & gameplay simulation** requires a strictly deterministic, constant timestep
//!   $dt$. Variable-delta Euler or Verlet integration produces non-deterministic trajectories,
//!   unstable spring dynamics, and tunneling through collision boundaries.
//!
//! To resolve this, Glenn Fiedler's seminal architecture ("Fix Your Timestep!") separates
//! the wall-clock rendering pace from the deterministic simulation tick rate using an
//! **accumulator**:
//!
//! ```text
//! Wall Clock (Variable Delta) ───► [ Clamped Delta (max 250ms) ]
//!                                                │
//!                                                ▼
//!                                     [ Accumulator (ns) ]
//!                                                │
//!           ┌────────────────────────────────────┴────────────────────────────────────┐
//!           ▼                                                                         ▼
//!  While accum >= fixed_dt:                                                   Residual accum
//!    accum -= fixed_dt                                                               │
//!    Run 1 fixed tick                                                                ▼
//!    (clamped to max_frame_steps to prevent Spiral of Death)                  Compute alpha:
//!                                                                              accum / fixed_dt
//!                                                                                    │
//!                                                                                    ▼
//!                                                                           Render Interpolation
//! ```
//!
//! ## 2. Exact Integer Nanoseconds vs Floating-Point Drift
//!
//! Standard game loops frequently store accumulator time in seconds as single-precision (`f32`)
//! or double-precision (`f64`) floats. Over millions of frames, floating-point addition suffers
//! from catastrophic cancellation, non-associativity ($a + (b + c) \neq (a + b) + c$), and
//! subtle architecture-dependent rounding differences (e.g. x87 extended precision vs SSE/AVX vs
//! ARM NEON).
//!
//! `game-clock` uses **exact 64-bit unsigned integer nanoseconds (`u64`)**:
//! - $1\text{ second} = 1{,}000{,}000{,}000\text{ ns}$.
//! - A standard 60 Hz frame step is exactly $16{,}666{,}667\text{ ns}$ ($\approx 16.666667\text{ ms}$).
//! - A 64-bit integer nanosecond counter can track up to $2^{64}-1\text{ ns} \approx 584.9\text{ years}$
//!   of continuous execution before overflow.
//! - The **exact conservation law** holds down to the individual nanosecond:
//!   $$\text{elapsed\_sim\_ns} + \text{accumulator\_ns} + \text{total\_dropped\_ns} = \sum \text{raw\_frame\_delta\_ns}$$
//!
//! ## 3. Spiral of Death Mitigation (CWE-400)
//!
//! The "Spiral of Death" occurs when a computationally expensive physics or AI step takes longer
//! to simulate than the real-world time it represents. For example, if a 60 Hz simulation takes
//! $25\text{ ms}$ of CPU time to compute a $16.67\text{ ms}$ step, the next frame will have a larger
//! wall-clock delta ($> 25\text{ ms}$), demanding *more* simulation steps, spiraling into total
//! process lockup and exhaustion of CPU resources.
//!
//! `game-clock` defeats the Spiral of Death via a two-tier guard:
//! 1. **Wall-Delta Ceiling (`max_frame_delta_ns`, default 250ms)**: Any massive frame delay (e.g.
//!    OS window drag, breakpoint debugging, system suspension) is capped at $250\text{ ms}$ before
//!    entering the accumulator. Excess time is accounted in `dropped_ns`.
//! 2. **Step Ceiling (`max_frame_steps`, default 8 steps)**: The maximum simulation ticks permitted
//!    in a single frame is strictly bounded. If the accumulator contains enough time for more steps,
//!    the simulation runs exactly `max_frame_steps`, sets `spiral_of_death_clamped = true`, and
//!    **drains the excess backlog** so the clock recovers immediately on the next frame rather than
//!    staying permanently backlogged.
//!
//! ## 4. Zero-Heap Allocation & Copy Semantics
//!
//! All data structures (`GameClockConfig`, `GameClockState`, `ClockStepResult`, `ClockError`) are
//! compact, stack-only types implementing `Copy`. The entire crate contains zero dynamic memory
//! allocations (`max_alloc_bytes = 0`) on all code paths.

#![forbid(unsafe_code)]

#[allow(dead_code)]
pub mod capabilities;
pub use capabilities::PureCapabilities;

/// Nanoseconds per SI second ($1{,}000{,}000{,}000\text{ ns}$).
pub const NS_PER_SECOND: u64 = 1_000_000_000;

/// Default fixed simulation timestep for 60 Hz arcade loops ($16{,}666{,}667\text{ ns} \approx 16.666667\text{ ms}$).
pub const DEFAULT_60HZ_DT_NS: u64 = 16_666_667;

/// Default fixed simulation timestep for 120 Hz high-refresh arcade loops ($8{,}333{,}333\text{ ns} \approx 8.333333\text{ ms}$).
pub const DEFAULT_120HZ_DT_NS: u64 = 8_333_333;

/// Default maximum simulation steps allowed in a single frame ($8\text{ steps} \approx 133\text{ ms}$ at 60 Hz).
pub const DEFAULT_MAX_FRAME_STEPS: u32 = 8;

/// Default maximum wall-clock delta allowed in a single frame ($250{,}000{,}000\text{ ns} = 250\text{ ms}$).
pub const DEFAULT_MAX_FRAME_DELTA_NS: u64 = 250_000_000;

/// Hard upper bound on maximum simulation steps allowed per frame (1024 steps).
pub const HARD_MAX_FRAME_STEPS: u32 = 1024;

/// Hard upper bound on maximum frame delta or fixed timestep ($10{,}000{,}000{,}000\text{ ns} = 10\text{ seconds}$).
pub const HARD_MAX_DELTA_NS: u64 = 10_000_000_000;

/// Errors returned by game clock operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockError {
    /// Fixed timestep is zero (`fixed_dt_ns == 0`, CWE-369 / CWE-400).
    ZeroFixedTimestep,
    /// Maximum frame steps out of valid range `1..=HARD_MAX_FRAME_STEPS` (CWE-400).
    InvalidMaxFrameSteps,
    /// Maximum frame delta is smaller than `fixed_dt_ns` or exceeds `HARD_MAX_DELTA_NS`.
    InvalidMaxFrameDelta,
    /// State invariant violated (e.g. `accumulator_ns >= fixed_dt_ns` or corrupted fields).
    CorruptedStateInvariant,
    /// Timestamp or accumulator arithmetic overflow (CWE-190).
    TimestampOverflow,
}

impl std::fmt::Display for ClockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroFixedTimestep => write!(f, "fixed timestep cannot be zero"),
            Self::InvalidMaxFrameSteps => {
                write!(f, "max frame steps must be in 1..={}", HARD_MAX_FRAME_STEPS)
            }
            Self::InvalidMaxFrameDelta => write!(
                f,
                "max frame delta must be >= fixed_dt_ns and <= {}",
                HARD_MAX_DELTA_NS
            ),
            Self::CorruptedStateInvariant => write!(f, "clock state invariant corrupted"),
            Self::TimestampOverflow => write!(f, "timestamp or counter arithmetic overflow"),
        }
    }
}

impl std::error::Error for ClockError {}

/// Configuration for the deterministic fixed-timestep accumulator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameClockConfig {
    /// Duration of a single fixed simulation tick in integer nanoseconds.
    pub fixed_dt_ns: u64,
    /// Maximum number of simulation steps allowed in a single frame.
    pub max_frame_steps: u32,
    /// Maximum allowed raw wall-clock delta per frame in nanoseconds before clamping.
    pub max_frame_delta_ns: u64,
}

impl GameClockConfig {
    /// Constructs a standard 60 Hz arcade loop configuration (dt = 16,666,667 ns, max 8 steps, max 250ms delta).
    #[inline]
    pub const fn standard_60hz() -> Self {
        Self {
            fixed_dt_ns: DEFAULT_60HZ_DT_NS,
            max_frame_steps: DEFAULT_MAX_FRAME_STEPS,
            max_frame_delta_ns: DEFAULT_MAX_FRAME_DELTA_NS,
        }
    }

    /// Constructs a standard 120 Hz arcade loop configuration (dt = 8,333,333 ns, max 8 steps, max 250ms delta).
    #[inline]
    pub const fn standard_120hz() -> Self {
        Self {
            fixed_dt_ns: DEFAULT_120HZ_DT_NS,
            max_frame_steps: DEFAULT_MAX_FRAME_STEPS,
            max_frame_delta_ns: DEFAULT_MAX_FRAME_DELTA_NS,
        }
    }

    /// Constructs a configuration from a target simulation frequency (in Hz) and step cap.
    ///
    /// # Errors
    /// Returns `Err(ClockError::ZeroFixedTimestep)` if `hz == 0`.
    /// Returns `Err(ClockError::InvalidMaxFrameSteps)` if `max_frame_steps` is 0 or exceeds `HARD_MAX_FRAME_STEPS`.
    pub fn from_hz(hz: u32, max_frame_steps: u32) -> Result<Self, ClockError> {
        if hz == 0 {
            return Err(ClockError::ZeroFixedTimestep);
        }
        if max_frame_steps == 0 || max_frame_steps > HARD_MAX_FRAME_STEPS {
            return Err(ClockError::InvalidMaxFrameSteps);
        }
        let fixed_dt_ns = NS_PER_SECOND / (hz as u64);
        if fixed_dt_ns == 0 {
            return Err(ClockError::ZeroFixedTimestep);
        }
        if fixed_dt_ns > HARD_MAX_DELTA_NS {
            return Err(ClockError::InvalidMaxFrameDelta);
        }
        let max_frame_delta_ns = if DEFAULT_MAX_FRAME_DELTA_NS < fixed_dt_ns {
            fixed_dt_ns
        } else {
            DEFAULT_MAX_FRAME_DELTA_NS
        };
        let config = Self {
            fixed_dt_ns,
            max_frame_steps,
            max_frame_delta_ns,
        };
        config.validate()?;
        Ok(config)
    }

    /// Constructs and validates a custom clock configuration.
    pub fn new(
        fixed_dt_ns: u64,
        max_frame_steps: u32,
        max_frame_delta_ns: u64,
    ) -> Result<Self, ClockError> {
        let config = Self {
            fixed_dt_ns,
            max_frame_steps,
            max_frame_delta_ns,
        };
        config.validate()?;
        Ok(config)
    }

    /// Validates all configuration bounds.
    pub fn validate(&self) -> Result<(), ClockError> {
        if self.fixed_dt_ns == 0 {
            return Err(ClockError::ZeroFixedTimestep);
        }
        if self.fixed_dt_ns > HARD_MAX_DELTA_NS {
            return Err(ClockError::InvalidMaxFrameDelta);
        }
        if self.max_frame_steps == 0 || self.max_frame_steps > HARD_MAX_FRAME_STEPS {
            return Err(ClockError::InvalidMaxFrameSteps);
        }
        if self.max_frame_delta_ns < self.fixed_dt_ns || self.max_frame_delta_ns > HARD_MAX_DELTA_NS
        {
            return Err(ClockError::InvalidMaxFrameDelta);
        }
        Ok(())
    }
}

impl Default for GameClockConfig {
    #[inline]
    fn default() -> Self {
        Self::standard_60hz()
    }
}

/// Pure deterministic state of the game clock accumulator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameClockState {
    /// Total fixed simulation ticks executed across all frames.
    pub tick_count: u64,
    /// Total number of frame advance calls executed.
    pub frame_count: u64,
    /// Total simulated time elapsed in integer nanoseconds (`tick_count * fixed_dt_ns`).
    pub elapsed_sim_ns: u64,
    /// Total clamped wall-clock nanoseconds accumulated into the clock.
    pub elapsed_wall_ns: u64,
    /// Residual sub-tick nanoseconds remaining in the accumulator (`< fixed_dt_ns`).
    pub accumulator_ns: u64,
    /// Fixed simulation timestep in integer nanoseconds.
    pub fixed_dt_ns: u64,
    /// Maximum simulation steps to run in a single frame.
    pub max_frame_steps: u32,
    /// Maximum allowed wall-clock delta per frame in nanoseconds before clamping.
    pub max_frame_delta_ns: u64,
    /// Cumulative nanoseconds dropped due to delta cap or spiral-of-death clamp.
    pub total_dropped_ns: u64,
    /// Count of frames where the spiral-of-death guard was triggered.
    pub spiral_clamped_frames: u64,
}

impl GameClockState {
    /// Initializes a new clock state from configuration with zeroed accumulators and counters.
    pub fn new(config: GameClockConfig) -> Result<Self, ClockError> {
        new_clock(config)
    }

    /// Advances the clock state by a given raw frame delta, updating `self` in place.
    pub fn advance(&mut self, raw_frame_delta_ns: u64) -> Result<ClockStepResult, ClockError> {
        let result = advance_frame(self, raw_frame_delta_ns)?;
        *self = result.state;
        Ok(result)
    }

    /// Validates internal state invariants.
    pub fn validate(&self) -> Result<(), ClockError> {
        validate_state(self)
    }
}

/// Result of advancing the game clock by one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockStepResult {
    /// The updated game clock state after this frame.
    pub state: GameClockState,
    /// The number of fixed simulation steps to run for this frame (in `0..=state.max_frame_steps`).
    pub steps_to_run: u32,
    /// Sub-tick blending factor in thousandths (`0..1000`) for integer rendering interpolation.
    pub alpha_milli: u32,
    /// Sub-tick blending factor in Q16.16 fixed-point (`0..65536`) for fixed-point lerp.
    pub alpha_q16: u32,
    /// Whether the spiral-of-death clamp fired on this frame, dropping backlog steps.
    pub spiral_of_death_clamped: bool,
    /// Nanoseconds dropped on this frame due to delta clamping and/or spiral-of-death protection.
    pub dropped_ns: u64,
}

impl ClockStepResult {
    /// Computes the exact sub-tick interpolation alpha in the range `[0.0, 1.0)`.
    ///
    /// Alpha represents how far between the previous simulation state and the current
    /// simulation state the visual renderer should interpolate:
    /// `render_pos = prev_sim_pos * (1.0 - alpha) + current_sim_pos * alpha`.
    #[inline]
    pub fn alpha_f64(&self) -> f64 {
        if self.state.fixed_dt_ns == 0 {
            0.0
        } else {
            (self.state.accumulator_ns as f64) / (self.state.fixed_dt_ns as f64)
        }
    }
}

/// Creates a new zeroed `GameClockState` from a valid `GameClockConfig`.
///
/// # Errors
/// Returns an error if `config` violates any parameter bounds.
pub fn new_clock(config: GameClockConfig) -> Result<GameClockState, ClockError> {
    config.validate()?;
    Ok(GameClockState {
        tick_count: 0,
        frame_count: 0,
        elapsed_sim_ns: 0,
        elapsed_wall_ns: 0,
        accumulator_ns: 0,
        fixed_dt_ns: config.fixed_dt_ns,
        max_frame_steps: config.max_frame_steps,
        max_frame_delta_ns: config.max_frame_delta_ns,
        total_dropped_ns: 0,
        spiral_clamped_frames: 0,
    })
}

/// Validates that a `GameClockState` satisfies all architectural and safety invariants.
///
/// # Invariants Checked
/// - `fixed_dt_ns in 1..=HARD_MAX_DELTA_NS`
/// - `max_frame_steps in 1..=HARD_MAX_FRAME_STEPS`
/// - `max_frame_delta_ns in fixed_dt_ns..=HARD_MAX_DELTA_NS`
/// - `accumulator_ns < fixed_dt_ns`
/// - `elapsed_sim_ns == tick_count * fixed_dt_ns` (no divergence or arithmetic overflow)
pub fn validate_state(state: &GameClockState) -> Result<(), ClockError> {
    if state.fixed_dt_ns == 0 {
        return Err(ClockError::ZeroFixedTimestep);
    }
    if state.fixed_dt_ns > HARD_MAX_DELTA_NS {
        return Err(ClockError::CorruptedStateInvariant);
    }
    if state.max_frame_steps == 0 || state.max_frame_steps > HARD_MAX_FRAME_STEPS {
        return Err(ClockError::CorruptedStateInvariant);
    }
    if state.max_frame_delta_ns < state.fixed_dt_ns || state.max_frame_delta_ns > HARD_MAX_DELTA_NS
    {
        return Err(ClockError::CorruptedStateInvariant);
    }
    if state.accumulator_ns >= state.fixed_dt_ns {
        return Err(ClockError::CorruptedStateInvariant);
    }
    let expected_sim_ns = state
        .tick_count
        .checked_mul(state.fixed_dt_ns)
        .ok_or(ClockError::TimestampOverflow)?;
    if state.elapsed_sim_ns != expected_sim_ns {
        return Err(ClockError::CorruptedStateInvariant);
    }
    Ok(())
}

/// Advances the clock by a raw wall-clock duration in nanoseconds.
///
/// Implements deterministic fixed-timestep accumulation with Glenn Fiedler's spiral-of-death
/// mitigation and exact integer nanosecond conservation:
///
/// 1. Clamps raw frame delta to `state.max_frame_delta_ns`, counting dropped time.
/// 2. Adds clamped delta to `accumulator_ns`.
/// 3. Computes simulation step count: if steps exceed `state.max_frame_steps`, clamps
///    to `max_frame_steps`, flags `spiral_of_death_clamped = true`, counts excess steps
///    as dropped nanoseconds, and drains excess backlog so the clock recovers immediately.
/// 4. Updates all cumulative counters using checked arithmetic (CWE-190).
/// 5. Computes sub-tick rendering interpolation alphas (`alpha_milli`, `alpha_q16`).
///
/// # Errors
/// - `ClockError::CorruptedStateInvariant` if incoming state is invalid.
/// - `ClockError::TimestampOverflow` if any cumulative counter would overflow `u64`.
pub fn advance_frame(
    state: &GameClockState,
    raw_frame_delta_ns: u64,
) -> Result<ClockStepResult, ClockError> {
    validate_state(state)?;

    // 1. Wall-Delta Clamp
    let (clamped_delta_ns, delta_dropped) = if raw_frame_delta_ns > state.max_frame_delta_ns {
        (
            state.max_frame_delta_ns,
            raw_frame_delta_ns - state.max_frame_delta_ns,
        )
    } else {
        (raw_frame_delta_ns, 0)
    };

    // 2. Accumulate
    let new_accum = state
        .accumulator_ns
        .checked_add(clamped_delta_ns)
        .ok_or(ClockError::TimestampOverflow)?;

    // 3. Step Count & Spiral-of-Death Guard (CWE-400)
    let raw_steps = new_accum / state.fixed_dt_ns;
    let rem_ns = new_accum % state.fixed_dt_ns;

    let (steps_to_run, spiral_of_death_clamped, spiral_dropped) =
        if raw_steps > state.max_frame_steps as u64 {
            let steps = state.max_frame_steps;
            let excess_steps = raw_steps - (state.max_frame_steps as u64);
            let dropped = excess_steps
                .checked_mul(state.fixed_dt_ns)
                .ok_or(ClockError::TimestampOverflow)?;
            (steps, true, dropped)
        } else {
            (raw_steps as u32, false, 0)
        };

    let next_accumulator_ns = rem_ns;

    // 4. Update Counters (checked arithmetic for CWE-190)
    let tick_count = state
        .tick_count
        .checked_add(steps_to_run as u64)
        .ok_or(ClockError::TimestampOverflow)?;

    let frame_count = state
        .frame_count
        .checked_add(1)
        .ok_or(ClockError::TimestampOverflow)?;

    let sim_advance_ns = (steps_to_run as u64)
        .checked_mul(state.fixed_dt_ns)
        .ok_or(ClockError::TimestampOverflow)?;

    let elapsed_sim_ns = state
        .elapsed_sim_ns
        .checked_add(sim_advance_ns)
        .ok_or(ClockError::TimestampOverflow)?;

    let elapsed_wall_ns = state
        .elapsed_wall_ns
        .checked_add(clamped_delta_ns)
        .ok_or(ClockError::TimestampOverflow)?;

    let dropped_ns = delta_dropped
        .checked_add(spiral_dropped)
        .ok_or(ClockError::TimestampOverflow)?;

    let total_dropped_ns = state
        .total_dropped_ns
        .checked_add(dropped_ns)
        .ok_or(ClockError::TimestampOverflow)?;

    let spiral_clamped_frames = state
        .spiral_clamped_frames
        .checked_add(if spiral_of_death_clamped { 1 } else { 0 })
        .ok_or(ClockError::TimestampOverflow)?;

    // 5. Interpolation Factors (u128 intermediate prevents overflow when fixed_dt_ns is large)
    let alpha_milli =
        (((next_accumulator_ns as u128) * 1000) / (state.fixed_dt_ns as u128)) as u32;
    let alpha_q16 = (((next_accumulator_ns as u128) << 16) / (state.fixed_dt_ns as u128)) as u32;

    let next_state = GameClockState {
        tick_count,
        frame_count,
        elapsed_sim_ns,
        elapsed_wall_ns,
        accumulator_ns: next_accumulator_ns,
        fixed_dt_ns: state.fixed_dt_ns,
        max_frame_steps: state.max_frame_steps,
        max_frame_delta_ns: state.max_frame_delta_ns,
        total_dropped_ns,
        spiral_clamped_frames,
    };

    Ok(ClockStepResult {
        state: next_state,
        steps_to_run,
        alpha_milli,
        alpha_q16,
        spiral_of_death_clamped,
        dropped_ns,
    })
}
