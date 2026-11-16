//! # lunar-lander
//!
//! Pure, deterministic simulation core and ASCII telemetry for the classic
//! discrete lunar landing flight simulation (GAME02).
//!
//! ## Historical Lineage
//!
//! - **1969 — Jim Storer's Lunar Landing Game:** Written in autumn 1969 by Jim
//!   Storer, a high school student at Lexington High School in Massachusetts,
//!   on a DEC PDP-8 in FOCAL-69. Inspired by Apollo 11, Storer modelled the
//!   physics of the Apollo Lunar Module (LM) descent stage.
//! - **1973 — 101 BASIC Computer Games:** Adapted to Dartmouth BASIC by David
//!   H. Ahl and published under the title *Rocket* (and later *Lunar*),
//!   becoming an enduring computing classic.
//!
//! ## Architectural Guarantees
//!
//! In accordance with the repository's verification architecture (`VER00`,
//! `VER04m`, `VER07`, `VER08a`, `VER08b`, `VER15`, `VER27`):
//! - Strictly zero ambient system authority: `"capabilities": []` declared in
//!   `required_capabilities.json`.
//! - Pure integer fixed-point physics: 64-bit and 128-bit integer arithmetic
//!   with zero floating-point operations (`f32`/`f64`) in the simulation core.
//! - Zero heap allocation: all core simulation functions (`new_flight`,
//!   `validate_burn`, `step_burn`, `isqrt_u128`) execute with zero dynamic memory
//!   allocations (`VER07`).
//! - Declarative 2D telemetry: generates intermediate representation `PaintScene`
//!   rendered through `paint-vm-ascii` (`P2D02`).
//! - Full 26-CWE weakness closure (`VER15`).

#![forbid(unsafe_code)]

#[allow(dead_code)]
pub mod capabilities;

use paint_instructions::{
    PaintBase, PaintInstruction, PaintRect, PaintScene, PaintText, TextAlign,
};
use paint_vm_ascii::{render, AsciiOptions};

// ============================================================================
// Physical Constants & Apollo LM Parameters
// ============================================================================

/// Lunar surface gravitational acceleration in millifeet/s^2 (5.31 ft/s^2).
pub const G_LUNAR_MFT_S2: i64 = 5_310;

/// Constant dry mass of the Apollo Lunar Module descent stage in pounds.
pub const DRY_MASS_LBS: u32 = 16_500;

/// Initial usable propellant mass in pounds.
pub const INITIAL_FUEL_LBS: u32 = 16_500;

/// Total initial gross mass (dry mass + propellant) in pounds.
pub const TOTAL_INITIAL_MASS_LBS: u32 = 33_000;

/// Initial mission altitude in millifeet (120,000 ft).
pub const INITIAL_ALTITUDE_MFT: i64 = 120_000_000;

/// Initial descent velocity in millifeet/s (-3,000 ft/s, downward).
pub const INITIAL_VELOCITY_MFT_S: i64 = -3_000_000;

/// Descent engine exhaust velocity in millifeet/s (8,000 ft/s).
pub const EXHAUST_VELOCITY_MFT_S: i64 = 8_000_000;

/// Maximum pilot-commanded engine burn rate in pounds per second.
pub const MAX_BURN_RATE_LBS_S: u32 = 200;

/// Minimum turn duration in seconds.
pub const MIN_DURATION_S: u32 = 1;

/// Maximum turn duration in seconds.
pub const MAX_DURATION_S: u32 = 10;

/// Maximum allowable mission elapsed time in seconds (24 hours).
pub const MAX_TIME_SECONDS: u32 = 86_400;

/// Maximum plausible simulation altitude boundary in millifeet (200,000 ft).
pub const MAX_ALTITUDE_MFT: i64 = 200_000_000;

/// Minimum valid velocity boundary in millifeet/s (-10,000 ft/s).
pub const MIN_VELOCITY_MFT_S: i64 = -10_000_000;

/// Maximum valid velocity boundary in millifeet/s (+10,000 ft/s).
pub const MAX_VELOCITY_MFT_S: i64 = 10_000_000;

// ============================================================================
// Domain Types & State Enums
// ============================================================================

/// Terminal landing outcome classified by touchdown impact speed magnitude.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandingOutcome {
    /// 0 to 5 ft/s (0 to 5,000 mft/s): Flawless touchdown.
    Perfect,
    /// 5.001 to 15 ft/s (5,001 to 15,000 mft/s): Minor gear stress.
    Hard,
    /// 15.001 to 30 ft/s (15,001 to 30,000 mft/s): Craft crippled, crew survives.
    Crash,
    /// 30.001 to 60 ft/s (30,001 to 60,000 mft/s): Total structural destruction.
    Fatal,
    /// > 60 ft/s (> 60,000 mft/s): Blasted a crater into lunar regolith.
    Crater,
}

/// Lifecycle flight status of the Lunar Module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlightStatus {
    /// Active in-flight descent trajectory.
    InFlight,
    /// Craft has reached the lunar surface (Y = 0) with a classified outcome.
    Landed(LandingOutcome),
}

/// Closed enumeration of burn command rejection errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanderError {
    /// Attempted to execute a burn command when flight has already terminated.
    FlightAlreadyEnded,
    /// Requested burn rate exceeds engine maximum (B > 200 lbs/s).
    ExcessiveBurnRate,
    /// Command duration is 0 or exceeds turn limit (duration == 0 || duration > 10).
    InvalidDuration,
    /// Arithmetic overflow or state coordinate boundary violation.
    ArithmeticOverflow,
}

impl std::fmt::Display for LanderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FlightAlreadyEnded => write!(f, "Flight has already ended"),
            Self::ExcessiveBurnRate => write!(f, "Burn rate exceeds maximum engine capacity"),
            Self::InvalidDuration => {
                write!(f, "Duration is invalid (must be between 1 and 10 seconds)")
            }
            Self::ArithmeticOverflow => {
                write!(f, "Arithmetic overflow or coordinate bounds violation")
            }
        }
    }
}

impl std::error::Error for LanderError {}

/// Immutable state record capturing the complete flight simulation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanderState {
    /// Elapsed mission flight time in seconds (0..=86,400).
    pub time_seconds: u32,
    /// Altitude above lunar surface in millifeet (1 ft = 1,000 mft).
    pub altitude_mft: i64,
    /// Vertical velocity in millifeet/s (+ ascending, - descending).
    pub velocity_mft_s: i64,
    /// Remaining usable propellant in pounds (0..=16,500).
    pub fuel_lbs: u32,
    /// Constant dry mass of the Apollo LM descent stage (16,500 lbs).
    pub dry_mass_lbs: u32,
    /// Current lifecycle status (`InFlight` or `Landed`).
    pub status: FlightStatus,
    /// Terminal vertical impact velocity in millifeet/s at touchdown. `None` in flight.
    pub impact_velocity_mft_s: Option<i64>,
}

/// Pilot burn order for a discrete simulation turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BurnCommand {
    /// Engine propellant burn rate in pounds per second (0..=200).
    pub burn_rate_lbs_s: u32,
    /// Duration of burn interval in integer seconds (1..=10).
    pub duration_seconds: u32,
}

// ============================================================================
// Core Pure Simulation Functions
// ============================================================================

/// Initializes a new Lunar Module descent mission at standard initial conditions.
///
/// Guaranteed zero heap allocations (`VER07`).
#[inline]
pub fn new_flight() -> LanderState {
    LanderState {
        time_seconds: 0,
        altitude_mft: INITIAL_ALTITUDE_MFT,
        velocity_mft_s: INITIAL_VELOCITY_MFT_S,
        fuel_lbs: INITIAL_FUEL_LBS,
        dry_mass_lbs: DRY_MASS_LBS,
        status: FlightStatus::InFlight,
        impact_velocity_mft_s: None,
    }
}

/// Computes the exact integer square root $\lfloor \sqrt{n} \rfloor$ for 128-bit unsigned integers.
///
/// Uses an iterative digit-by-digit bitwise binary square root algorithm with bounded
/// step execution (at most 64 bitwise iterations) and zero recursive calls.
/// Completely division-free to eliminate any potential division-by-zero hazards (`CWE-369`).
/// Guaranteed zero heap allocations (`VER07`).
#[inline]
pub fn isqrt_u128(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    let mut res = 0u128;
    let mut bit = 1u128 << 126;
    while bit > n {
        bit >>= 2;
    }
    let mut num = n;
    while bit != 0 {
        if num >= res + bit {
            num -= res + bit;
            res = (res >> 1) + bit;
        } else {
            res >>= 1;
        }
        bit >>= 2;
    }
    res
}

/// Validates a candidate burn command against current state and physical invariants.
///
/// Enforces canonical validation precedence:
/// 1. Lifecycle Check: craft must be `InFlight`.
/// 2. State Invariants (`VER04m`, CWE-190, CWE-369): dry mass integrity, fuel capacity,
///    in-flight altitude positivity, velocity bounds, mission time limits, and absence of impact velocity.
/// 3. Command Bounds: maximum burn rate (<= 200), turn duration (1..=10), and mission time non-overflow.
///
/// Guaranteed zero heap allocations (`VER07`).
#[inline]
pub fn validate_burn(state: &LanderState, cmd: &BurnCommand) -> Result<(), LanderError> {
    // 1. Lifecycle Check
    if state.status != FlightStatus::InFlight {
        return Err(LanderError::FlightAlreadyEnded);
    }

    // 2. State Invariants
    if state.dry_mass_lbs != DRY_MASS_LBS {
        return Err(LanderError::ArithmeticOverflow);
    }
    if state.fuel_lbs > INITIAL_FUEL_LBS {
        return Err(LanderError::ArithmeticOverflow);
    }
    if state.altitude_mft <= 0 || state.altitude_mft > MAX_ALTITUDE_MFT {
        return Err(LanderError::ArithmeticOverflow);
    }
    if !(MIN_VELOCITY_MFT_S..=MAX_VELOCITY_MFT_S).contains(&state.velocity_mft_s) {
        return Err(LanderError::ArithmeticOverflow);
    }
    if state.time_seconds > MAX_TIME_SECONDS {
        return Err(LanderError::ArithmeticOverflow);
    }
    if state.impact_velocity_mft_s.is_some() {
        return Err(LanderError::ArithmeticOverflow);
    }

    // 3. Command Bounds
    if cmd.burn_rate_lbs_s > MAX_BURN_RATE_LBS_S {
        return Err(LanderError::ExcessiveBurnRate);
    }
    if !(MIN_DURATION_S..=MAX_DURATION_S).contains(&cmd.duration_seconds) {
        return Err(LanderError::InvalidDuration);
    }
    let Some(t_next) = state.time_seconds.checked_add(cmd.duration_seconds) else {
        return Err(LanderError::ArithmeticOverflow);
    };
    if t_next > MAX_TIME_SECONDS {
        return Err(LanderError::ArithmeticOverflow);
    }

    Ok(())
}

/// Integrates a single discrete flight step under the commanded burn rate and duration.
///
/// Implements second-order Euler-Verlet numerical kinematics in pure fixed-point millifeet arithmetic.
/// When the projected altitude penetrates or reaches the lunar surface ($Y_{t+\Delta t} \le 0$),
/// solves for the exact touchdown contact time $\delta t \in [1, \Delta t]$ and impact velocity,
/// automatically refunding unburned propellant for any unused interval fraction.
///
/// Guaranteed zero heap allocations (`VER07`).
pub fn step_burn(state: &LanderState, cmd: &BurnCommand) -> Result<LanderState, LanderError> {
    validate_burn(state, cmd)?;

    let dt = cmd.duration_seconds as i64;
    let fuel_req = (cmd.burn_rate_lbs_s as u64) * (cmd.duration_seconds as u64);
    let delta_f = (state.fuel_lbs as u64).min(fuel_req) as u32;
    let fuel_next = state.fuel_lbs - delta_f;

    let avg_mass = (state.dry_mass_lbs as i64) + (state.fuel_lbs as i64) - ((delta_f as i64) / 2);
    let impulse = (delta_f as i64) * EXHAUST_VELOCITY_MFT_S;
    let a_thrust = impulse / (dt * avg_mass);
    let a_net = a_thrust - G_LUNAR_MFT_S2;

    let v_next = state.velocity_mft_s + a_net * dt;
    let y_next = state.altitude_mft + state.velocity_mft_s * dt + (a_net * dt * dt).div_euclid(2);
    let t_next = state.time_seconds + cmd.duration_seconds;

    if y_next > MAX_ALTITUDE_MFT || !(MIN_VELOCITY_MFT_S..=MAX_VELOCITY_MFT_S).contains(&v_next) {
        return Err(LanderError::ArithmeticOverflow);
    }

    if y_next > 0 {
        Ok(LanderState {
            time_seconds: t_next,
            altitude_mft: y_next,
            velocity_mft_s: v_next,
            fuel_lbs: fuel_next,
            dry_mass_lbs: state.dry_mass_lbs,
            status: FlightStatus::InFlight,
            impact_velocity_mft_s: None,
        })
    } else {
        // Ground touchdown solver
        let a_half = a_net.div_euclid(2);
        let delta_t_raw = if a_half == 0 {
            if state.velocity_mft_s == 0 {
                return Err(LanderError::ArithmeticOverflow);
            }
            (-state.altitude_mft).div_euclid(state.velocity_mft_s)
        } else {
            let v128 = state.velocity_mft_s as i128;
            let y128 = state.altitude_mft as i128;
            let a128 = a_half as i128;
            let disc = v128 * v128 - 4 * a128 * y128;
            let disc_pos = disc.max(0) as u128;
            let root = isqrt_u128(disc_pos) as i64;
            (-state.velocity_mft_s - root).div_euclid(2 * a_half)
        };

        let delta_t = delta_t_raw.clamp(1, dt);
        let delta_t_u32 = delta_t as u32;

        let v_touchdown = state.velocity_mft_s + a_net * delta_t;
        let t_final = state.time_seconds + delta_t_u32;

        let fuel_burned_at_touchdown =
            (state.fuel_lbs as u64).min((delta_t_u32 as u64) * (cmd.burn_rate_lbs_s as u64)) as u32;
        let fuel_final = state.fuel_lbs - fuel_burned_at_touchdown;

        let speed_mft_s = v_touchdown.unsigned_abs();
        let outcome = match speed_mft_s {
            0..=5_000 => LandingOutcome::Perfect,
            5_001..=15_000 => LandingOutcome::Hard,
            15_001..=30_000 => LandingOutcome::Crash,
            30_001..=60_000 => LandingOutcome::Fatal,
            _ => LandingOutcome::Crater,
        };

        Ok(LanderState {
            time_seconds: t_final,
            altitude_mft: 0,
            velocity_mft_s: v_touchdown,
            fuel_lbs: fuel_final,
            dry_mass_lbs: state.dry_mass_lbs,
            status: FlightStatus::Landed(outcome),
            impact_velocity_mft_s: Some(v_touchdown),
        })
    }
}

// ============================================================================
// ASCII Telemetry & PaintScene Presentation Bridge
// ============================================================================

fn format_with_commas(n: i64) -> String {
    let negative = n < 0;
    let s = n.unsigned_abs().to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3 + 1);
    if negative {
        result.push('-');
    }
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(ch);
    }
    result
}

/// Translates a flight state into a declarative 76x5 [`PaintScene`] (`P2D00`).
///
/// Telemetry HUD visual layout:
/// - Outer border drawn via multiline box characters.
/// - Numerical instruments for flight time, altitude, velocity, fuel, burn rate, and descent rate.
/// - 20-character proportional bar gauges for altitude and usable fuel.
pub fn to_paint_scene(state: &LanderState, last_burn_rate_lbs_s: u32) -> PaintScene {
    let mut scene = PaintScene::new(76.0, 5.0);
    scene.background = "#000000".to_string();

    let alt_clamped = state.altitude_mft.clamp(0, INITIAL_ALTITUDE_MFT) as u64;
    let alt_pct = (alt_clamped * 100) / (INITIAL_ALTITUDE_MFT as u64);
    let alt_bars = ((alt_clamped * 20) / (INITIAL_ALTITUDE_MFT as u64)) as u32;

    let fuel_clamped = state.fuel_lbs.min(INITIAL_FUEL_LBS) as u64;
    let fuel_pct = (fuel_clamped * 100) / (INITIAL_FUEL_LBS as u64);
    let fuel_bars = ((fuel_clamped * 20) / (INITIAL_FUEL_LBS as u64)) as u32;

    let alt_ft = state.altitude_mft / 1_000;
    let vel_fps = state.velocity_mft_s / 1_000;
    let fuel_lbs = state.fuel_lbs;

    let alt_str = format_with_commas(alt_ft);
    let fuel_str = format_with_commas(fuel_lbs as i64);

    let (rate_label, rate_text, rate_symbol) = match state.status {
        FlightStatus::InFlight => {
            if state.velocity_mft_s < 0 {
                ("RATE:", "DESCENT", "▼")
            } else if state.velocity_mft_s > 0 {
                ("RATE:", "ASCENT", "▲")
            } else {
                ("RATE:", "HOVER", "◆")
            }
        }
        FlightStatus::Landed(outcome) => {
            let desc = match outcome {
                LandingOutcome::Perfect => "PERFECT",
                LandingOutcome::Hard => "HARD",
                LandingOutcome::Crash => "CRASH",
                LandingOutcome::Fatal => "FATAL",
                LandingOutcome::Crater => "CRATER",
            };
            ("STATUS:", desc, "★")
        }
    };

    let row0 = "┌───────────────────────── LUNAR LANDER TELEMETRY ─────────────────────────┐";
    let left1 = format!(" TIME: {:>6} SEC    ", state.time_seconds);
    let right1 = format!(
        " ALTITUDE: {:>7} FT [░░░░░░░░░░░░░░░░░░░░] {:>3}%   ",
        alt_str, alt_pct
    );
    let row1 = format!("│{}│{}│", left1, right1);

    let left2 = format!(" VELOCITY: {:>5} FT/S", vel_fps);
    let right2 = format!(
        " FUEL:    {:>7} LBS [░░░░░░░░░░░░░░░░░░░░] {:>3}%   ",
        fuel_str, fuel_pct
    );
    let row2 = format!("│{}│{}│", left2, right2);

    let left3 = format!(" BURN:     {:>4} LB/S ", last_burn_rate_lbs_s);
    let rate_col = format!("{:<7} {:<8} {:<1}", rate_label, rate_text, rate_symbol);
    let row3 = format!("│{}│ {:<51}│", left3, rate_col);
    let row4 = "└──────────────────────────────────────────────────────────────────────────┘";

    let full_text = format!("{}\n{}\n{}\n{}\n{}", row0, row1, row2, row3, row4);

    scene.instructions.push(PaintInstruction::Text(PaintText {
        base: PaintBase::default(),
        x: 0.0,
        y: 0.0,
        text: full_text,
        font_ref: None,
        font_size: 1.0,
        fill: Some("#ffffff".to_string()),
        text_align: Some(TextAlign::Left),
    }));

    if alt_bars > 0 {
        scene.instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: 46.0,
            y: 1.0,
            width: (alt_bars - 1) as f64,
            height: 0.0,
            fill: Some("#00ff00".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));
    }

    if fuel_bars > 0 {
        scene.instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase::default(),
            x: 46.0,
            y: 2.0,
            width: (fuel_bars - 1) as f64,
            height: 0.0,
            fill: Some("#00ff00".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));
    }

    scene
}

/// Convenience helper translating a flight state into a declarative [`PaintScene`] with zero initial burn.
#[inline]
pub fn telemetry_scene(state: &LanderState) -> PaintScene {
    to_paint_scene(state, 0)
}

/// Renders a flight state telemetry dashboard into a deterministic ASCII string via `paint-vm-ascii`.
pub fn render_telemetry_ascii(state: &LanderState, last_burn_rate_lbs_s: u32) -> String {
    let scene = to_paint_scene(state, last_burn_rate_lbs_s);
    render(
        &scene,
        AsciiOptions {
            scale_x: 1,
            scale_y: 1,
        },
    )
    .unwrap_or_else(|_| "Failed to render telemetry".to_string())
}

/// Convenience helper rendering a flight state telemetry dashboard with zero burn rate.
#[inline]
pub fn render_telemetry(state: &LanderState) -> String {
    render_telemetry_ascii(state, 0)
}
