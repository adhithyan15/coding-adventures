//! # cockpit-hud-paint
//!
//! GAME43: deterministic, zero-allocation Primary Flight Display (PFD) and
//! Heads-Up Display (HUD) vector overlay renderer lowering to `P2D00`
//! [`PaintScene`] instructions.
//!
//! ## Architecture
//!
//! ```text
//!   FlightTelemetry + HudConfig
//!         │
//!         ├─► Atomic pre-validation (viewport 64..=8192, finiteness, caps)
//!         │
//!         ├─► Deterministic trig::{sin, cos, atan2, sqrt} + angle wrapping
//!         │
//!         ├─► 2D Liang-Barsky line clipping & Sutherland-Hodgman sky/ground split
//!         │
//!         ├─► build_hud_frame_into(&telemetry, &config, &mut scratch)  [0 alloc]
//!         │
//!         └─► update_paint_scene_into(&telemetry, &config, &mut scene) [0 alloc]
//! ```
//!
//! ## Safety, Determinism, and Resource Guarantees
//!
//! * `#![forbid(unsafe_code)]`; `#![deny(missing_docs)]`.
//! * Zero ambient capabilities (`"capabilities": []`, `CAP001`).
//! * Zero `std` `libm` calls: all trigonometry and square roots delegate to
//!   [`trig`] (`PHY00`) after explicit angle wrapping within `|x| <= 2^29`.
//! * Zero heap allocation in `build_hud_frame_into` and `update_paint_scene_into`
//!   (`VER07`, `max_alloc_bytes: Some(0)`).
//! * Atomic error semantics: on any validation failure or `BufferTooSmall`,
//!   caller-owned `HudScratch` and `PaintScene` buffers remain byte-for-byte
//!   unmodified.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Generated capability verification witnesses.
pub mod capabilities;

pub use capabilities::PureCapabilities;

use math3d::{Math3dError, Vec3};
use paint_instructions::{
    PaintBase, PaintEllipse, PaintInstruction, PaintLine, PaintPath, PaintRect, PaintScene,
    PaintText, PathCommand, TextAlign,
};
use rigidbody_6dof::{MassProperties, RigidBodyError, RigidBodyState};

/// Minimum allowed viewport width or height in logical pixels (CWE-20).
pub const MIN_VIEWPORT_DIM: f64 = 64.0;

/// Maximum allowed viewport width or height in logical pixels (CWE-20 / CWE-400).
pub const MAX_VIEWPORT_DIM: f64 = 8192.0;

/// Maximum magnitude of any input angle in radians (`2^29`, matching `trig`).
pub const MAX_ANGLE_RAD: f64 = 536_870_912.0;

/// Minimum vertical field-of-view in degrees for HUD pitch scaling.
pub const MIN_FOV_DEG: f64 = 15.0;

/// Maximum vertical field-of-view in degrees for HUD pitch scaling.
pub const MAX_FOV_DEG: f64 = 120.0;

/// Maximum supported airspeed in meters per second (~Mach 6).
pub const MAX_AIRSPEED_MPS: f64 = 2000.0;

/// Maximum supported Mach number.
pub const MAX_MACH: f64 = 10.0;

/// Minimum supported barometric altitude MSL in meters.
pub const MIN_ALTITUDE_MSL_M: f64 = -1000.0;

/// Maximum supported barometric altitude MSL in meters.
pub const MAX_ALTITUDE_MSL_M: f64 = 100_000.0;

/// Maximum supported radar altitude AGL in meters.
pub const MAX_RADAR_ALTITUDE_M: f64 = 100_000.0;

/// Maximum absolute vertical speed in meters per second.
pub const MAX_VERTICAL_SPEED_MPS: f64 = 1000.0;

/// Maximum absolute normal load factor in g.
pub const MAX_G_LOAD: f64 = 25.0;

/// Minimum primitive cap accepted in [`HudConfig`].
pub const MIN_PRIMITIVES: usize = 64;

/// Maximum primitive cap accepted in [`HudConfig`] (CWE-400).
pub const MAX_PRIMITIVES: usize = 4096;

/// Maximum pitch ladder rungs accepted in [`HudConfig`] (CWE-400).
pub const MAX_PITCH_RUNGS: usize = 72;

/// Maximum tape ticks per instrument accepted in [`HudConfig`] (CWE-400).
pub const MAX_TAPE_TICKS: usize = 128;

/// Default capacity of [`HudScratch`] primitive buffer.
pub const DEFAULT_SCRATCH_CAPACITY: usize = 512;

/// Numerical guard against near-zero divisors (CWE-369).
pub const EPSILON: f64 = 1e-12;

const TWO_PI: f64 = 2.0 * trig::PI;
const INV_TWO_PI: f64 = 1.0 / TWO_PI;
const ROUND_MAGIC: f64 = 6_755_399_441_055_744.0; // 1.5 * 2^52
const KNOTS_PER_MPS: f64 = 1.943_844_492_440_6;
const FEET_PER_METER: f64 = 3.280_839_895_013_123;
const FNV1A_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV1A_PRIME: u64 = 0x0100_0000_01b3;
const CANON_NAN_BITS: u64 = 0x7ff8_0000_0000_0000;

const COLOR_BG: &str = "#050b14";
const COLOR_SKY: &str = "#1d4ed8";
const COLOR_GROUND: &str = "#78350f";
const COLOR_HUD_GREEN: &str = "#22c55e";
const COLOR_HUD_WHITE: &str = "#f8fafc";
const COLOR_HUD_CYAN: &str = "#38bdf8";
const COLOR_PANEL_BG: &str = "#0b1320";
const COLOR_WARN_RED: &str = "#ef4444";
const COLOR_WARN_AMBER: &str = "#f59e0b";
const COLOR_TRANSPARENT: &str = "transparent";

const SCENE_SKELETON_LEN: usize = 55;
const PATH_CMD_CAP_PITCH: usize = 512;
const PATH_CMD_CAP_TICKS: usize = 512;
const PATH_CMD_CAP_POLY: usize = 16;
const STR_CAP_ID: usize = 32;
const STR_CAP_TEXT: usize = 32;
const STR_CAP_COLOR: usize = 16;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Failure modes for every fallible operation in `cockpit-hud-paint`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudError {
    /// A configuration or telemetry field is `NaN` or `+/-Infinity` (CWE-20).
    NonFiniteInput,
    /// Viewport dimensions violate `[MIN_VIEWPORT_DIM, MAX_VIEWPORT_DIM]` (`64..=8192`).
    InvalidViewport,
    /// Telemetry physical values violate domain invariants (e.g., negative airspeed).
    InvalidTelemetry,
    /// An angle exceeds `[-MAX_ANGLE_RAD, MAX_ANGLE_RAD]` (`|x| <= 2^29`).
    AngleOutOfBounds,
    /// A numeric parameter (such as FOV or threshold) is outside valid bounds.
    ValueOutOfBounds,
    /// Caller-provided `HudScratch` or `PaintScene` lacks capacity for the frame.
    BufferTooSmall,
    /// Configured or emitted primitive/rung/tick count exceeds safety caps (CWE-400).
    PrimitiveLimitExceeded,
    /// Error propagated from `rigidbody-6dof` when deriving telemetry.
    RigidBodyConversion(RigidBodyError),
}

impl From<RigidBodyError> for HudError {
    #[inline]
    fn from(err: RigidBodyError) -> Self {
        Self::RigidBodyConversion(err)
    }
}

impl From<Math3dError> for HudError {
    #[inline]
    fn from(err: Math3dError) -> Self {
        Self::RigidBodyConversion(RigidBodyError::from(err))
    }
}

impl std::fmt::Display for HudError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "input contains NaN or Infinity"),
            Self::InvalidViewport => {
                write!(f, "viewport width and height must be in 64..=8192")
            }
            Self::InvalidTelemetry => write!(f, "telemetry field is outside physical bounds"),
            Self::AngleOutOfBounds => {
                write!(f, "angle magnitude exceeds MAX_ANGLE_RAD (2^29 rad)")
            }
            Self::ValueOutOfBounds => write!(f, "configuration value is out of valid bounds"),
            Self::BufferTooSmall => {
                write!(
                    f,
                    "caller buffer or PaintScene lacks capacity for HUD frame"
                )
            }
            Self::PrimitiveLimitExceeded => {
                write!(f, "primitive, pitch rung, or tape tick limit exceeded")
            }
            Self::RigidBodyConversion(e) => write!(f, "rigidbody conversion error: {e}"),
        }
    }
}

impl std::error::Error for HudError {}

// ---------------------------------------------------------------------------
// Configuration & Telemetry Types
// ---------------------------------------------------------------------------

/// Rendering mode controlling sky/ground background fills vs pure HUD strokes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudDisplayMode {
    /// Full Primary Flight Display (sky/ground attitude ball + tapes) plus HUD vectors.
    CombinedPfdHud,
    /// Primary Flight Display instrumentation without HUD flight-path overlay.
    PfdOnly,
    /// Transparent-background HUD vector overlay only (green boresight, pitch ladder, FPM, tapes).
    HudOverlayOnly,
}

/// Unit convention for numeric tape and readout labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudUnits {
    /// Aviation standard: knots (IAS), feet (altitude), hundreds of ft/min (VSI).
    Aviation,
    /// Metric SI units: m/s (IAS/VSI) and meters (altitude).
    Si,
}

/// Configuration and safety caps for rendering a cockpit HUD/PFD frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HudConfig {
    /// Viewport width in logical pixels (`64.0..=8192.0`).
    pub viewport_width: f64,
    /// Viewport height in logical pixels (`64.0..=8192.0`).
    pub viewport_height: f64,
    /// Vertical field of view in degrees (`15.0..=120.0`) scaling the pitch ladder.
    pub fov_vertical_deg: f64,
    /// Display presentation mode.
    pub mode: HudDisplayMode,
    /// Unit convention for numeric labels.
    pub units: HudUnits,
    /// Maximum primitive count permitted in a frame (`64..=4096`).
    pub max_primitives: usize,
    /// Maximum pitch ladder rungs emitted (`1..=72`).
    pub max_pitch_rungs: usize,
    /// Maximum ticks emitted per airspeed/altitude/heading tape (`4..=128`).
    pub max_tape_ticks: usize,
    /// Angle-of-attack stall threshold in radians.
    pub aoa_stall_rad: f64,
    /// Minimum indicated airspeed stall threshold in m/s.
    pub ias_stall_mps: f64,
    /// Maximum indicated airspeed overspeed threshold in m/s.
    pub ias_overspeed_mps: f64,
    /// Maximum Mach overspeed threshold.
    pub mach_overspeed: f64,
    /// Radar altitude caution threshold in meters for TERRAIN annunciator.
    pub terrain_caution_agl_m: f64,
    /// Radar altitude pull-up threshold in meters for PULL UP annunciator.
    pub pull_up_agl_m: f64,
}

impl HudConfig {
    /// Construct a default [`HudConfig`] for the given viewport dimensions.
    pub fn hud_config_default(width: f64, height: f64) -> Result<Self, HudError> {
        let cfg = Self {
            viewport_width: width,
            viewport_height: height,
            fov_vertical_deg: 30.0,
            mode: HudDisplayMode::CombinedPfdHud,
            units: HudUnits::Aviation,
            max_primitives: 2048,
            max_pitch_rungs: 36,
            max_tape_ticks: 64,
            aoa_stall_rad: trig::radians(15.0),
            ias_stall_mps: 35.0,
            ias_overspeed_mps: 340.0,
            mach_overspeed: 0.95,
            terrain_caution_agl_m: 150.0,
            pull_up_agl_m: 60.0,
        };
        cfg.hud_config_validate()?;
        Ok(cfg)
    }

    /// Validate all viewport bounds, numeric thresholds, and resource caps.
    pub fn hud_config_validate(&self) -> Result<(), HudError> {
        check_finite(self.viewport_width)?;
        check_finite(self.viewport_height)?;
        if self.viewport_width < MIN_VIEWPORT_DIM
            || self.viewport_width > MAX_VIEWPORT_DIM
            || self.viewport_height < MIN_VIEWPORT_DIM
            || self.viewport_height > MAX_VIEWPORT_DIM
        {
            return Err(HudError::InvalidViewport);
        }

        check_finite(self.fov_vertical_deg)?;
        if self.fov_vertical_deg < MIN_FOV_DEG || self.fov_vertical_deg > MAX_FOV_DEG {
            return Err(HudError::ValueOutOfBounds);
        }

        if self.max_primitives < MIN_PRIMITIVES || self.max_primitives > MAX_PRIMITIVES {
            return Err(HudError::PrimitiveLimitExceeded);
        }
        if self.max_pitch_rungs == 0 || self.max_pitch_rungs > MAX_PITCH_RUNGS {
            return Err(HudError::PrimitiveLimitExceeded);
        }
        if self.max_tape_ticks < 4 || self.max_tape_ticks > MAX_TAPE_TICKS {
            return Err(HudError::PrimitiveLimitExceeded);
        }

        check_angle(self.aoa_stall_rad)?;
        check_finite(self.ias_stall_mps)?;
        check_finite(self.ias_overspeed_mps)?;
        check_finite(self.mach_overspeed)?;
        check_finite(self.terrain_caution_agl_m)?;
        check_finite(self.pull_up_agl_m)?;

        if self.ias_stall_mps < 0.0
            || self.ias_overspeed_mps < 0.0
            || self.mach_overspeed < 0.0
            || self.terrain_caution_agl_m < 0.0
            || self.pull_up_agl_m < 0.0
        {
            return Err(HudError::ValueOutOfBounds);
        }

        Ok(())
    }
}

/// Self-contained aircraft attitude, air-data, and warning telemetry (SI units).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlightTelemetry {
    /// Pitch attitude in radians (nose up positive).
    pub pitch_rad: f64,
    /// Roll / bank angle in radians (right wing down positive).
    pub roll_rad: f64,
    /// Magnetic/true heading in radians (`0` = North, `PI/2` = East).
    pub heading_rad: f64,
    /// Indicated airspeed in m/s (`>= 0.0`).
    pub indicated_airspeed_mps: f64,
    /// True airspeed in m/s (`>= 0.0`).
    pub true_airspeed_mps: f64,
    /// Mach number (`>= 0.0`).
    pub mach: f64,
    /// Barometric altitude above mean sea level in meters.
    pub altitude_msl_m: f64,
    /// Radar altitude above ground level in meters (`>= 0.0`).
    pub radar_altitude_agl_m: f64,
    /// Vertical speed in m/s (climb positive).
    pub vertical_speed_mps: f64,
    /// Flight path elevation angle in radians.
    pub flight_path_pitch_rad: f64,
    /// Flight path azimuth relative to aircraft heading in radians.
    pub flight_path_yaw_rad: f64,
    /// Angle of attack in radians.
    pub aoa_rad: f64,
    /// Sideslip angle in radians.
    pub sideslip_rad: f64,
    /// Normal load factor in g (`1.0` = steady level flight).
    pub g_load: f64,
    /// Explicit stall warning flag.
    pub stall_warning: bool,
    /// Explicit overspeed warning flag.
    pub overspeed_warning: bool,
    /// Explicit terrain caution flag.
    pub terrain_warning: bool,
    /// Explicit GPWS pull-up warning flag.
    pub pull_up_warning: bool,
}

impl FlightTelemetry {
    /// Steady level cruise baseline telemetry (`120 m/s`, `3000 m`, `0°` pitch/roll).
    pub const fn telemetry_level_flight() -> Self {
        Self {
            pitch_rad: 0.0,
            roll_rad: 0.0,
            heading_rad: 0.0,
            indicated_airspeed_mps: 120.0,
            true_airspeed_mps: 135.0,
            mach: 0.40,
            altitude_msl_m: 3000.0,
            radar_altitude_agl_m: 1500.0,
            vertical_speed_mps: 0.0,
            flight_path_pitch_rad: 0.0,
            flight_path_yaw_rad: 0.0,
            aoa_rad: 0.04,
            sideslip_rad: 0.0,
            g_load: 1.0,
            stall_warning: false,
            overspeed_warning: false,
            terrain_warning: false,
            pull_up_warning: false,
        }
    }

    /// Validate finiteness and physical domain invariants of all telemetry fields.
    pub fn telemetry_validate(&self) -> Result<(), HudError> {
        check_angle(self.pitch_rad)?;
        check_angle(self.roll_rad)?;
        check_angle(self.heading_rad)?;
        check_angle(self.flight_path_pitch_rad)?;
        check_angle(self.flight_path_yaw_rad)?;
        check_angle(self.aoa_rad)?;
        check_angle(self.sideslip_rad)?;

        check_finite(self.indicated_airspeed_mps)?;
        check_finite(self.true_airspeed_mps)?;
        check_finite(self.mach)?;
        check_finite(self.altitude_msl_m)?;
        check_finite(self.radar_altitude_agl_m)?;
        check_finite(self.vertical_speed_mps)?;
        check_finite(self.g_load)?;

        if self.indicated_airspeed_mps < 0.0
            || self.indicated_airspeed_mps > MAX_AIRSPEED_MPS
            || self.true_airspeed_mps < 0.0
            || self.true_airspeed_mps > MAX_AIRSPEED_MPS
            || self.mach < 0.0
            || self.mach > MAX_MACH
            || self.altitude_msl_m < MIN_ALTITUDE_MSL_M
            || self.altitude_msl_m > MAX_ALTITUDE_MSL_M
            || self.radar_altitude_agl_m < 0.0
            || self.radar_altitude_agl_m > MAX_RADAR_ALTITUDE_M
            || self.vertical_speed_mps < -MAX_VERTICAL_SPEED_MPS
            || self.vertical_speed_mps > MAX_VERTICAL_SPEED_MPS
            || self.g_load < -MAX_G_LOAD
            || self.g_load > MAX_G_LOAD
        {
            return Err(HudError::InvalidTelemetry);
        }

        Ok(())
    }

    /// Derive [`FlightTelemetry`] from a `rigidbody-6dof` [`RigidBodyState`] and
    /// [`MassProperties`], using `trig` exclusively for attitude and air-data extraction.
    ///
    /// Convention matching `rigidbody-6dof`: `+Y` is vertical up, `+X` is forward nose,
    /// `+Z` is starboard (right) wing.
    pub fn telemetry_from_rigid_body(
        body: &RigidBodyState,
        mass: &MassProperties,
        speed_of_sound_mps: f64,
        terrain_elevation_m: f64,
    ) -> Result<Self, HudError> {
        check_finite(speed_of_sound_mps)?;
        check_finite(terrain_elevation_m)?;
        if speed_of_sound_mps <= EPSILON || speed_of_sound_mps > 2000.0 {
            return Err(HudError::ValueOutOfBounds);
        }

        body.body_validate()?;
        mass.mass_props_validate()?;

        let v_world = body.linear_velocity(mass)?;
        let q = body.orientation.quat_normalize().map_err(HudError::from)?;
        let q_inv = q.conjugate().map_err(HudError::from)?;
        let v_body = q_inv.rotate_vec3(v_world).map_err(HudError::from)?;

        // Transform body basis vectors into world space to extract pitch, roll, heading.
        // Forward nose = +X, Up = +Y, Starboard = +Z.
        let fwd = q
            .rotate_vec3(Vec3::new(1.0, 0.0, 0.0))
            .map_err(HudError::from)?;
        let up = q
            .rotate_vec3(Vec3::new(0.0, 1.0, 0.0))
            .map_err(HudError::from)?;
        let right = q
            .rotate_vec3(Vec3::new(0.0, 0.0, 1.0))
            .map_err(HudError::from)?;

        let horiz = trig::sqrt((fwd.x * fwd.x + fwd.z * fwd.z).max(0.0));
        let pitch_rad = trig::atan2(fwd.y, horiz);
        let heading_rad = wrap_2pi(trig::atan2(fwd.z, fwd.x));
        let roll_rad = if horiz > 1e-6 {
            trig::atan2(-right.y, up.y)
        } else {
            0.0
        };

        let speed_sq = v_world.x * v_world.x + v_world.y * v_world.y + v_world.z * v_world.z;
        let speed = trig::sqrt(speed_sq.max(0.0));
        let mach = speed / speed_of_sound_mps;

        let horiz_speed = trig::sqrt((v_world.x * v_world.x + v_world.z * v_world.z).max(0.0));
        let flight_path_pitch_rad = if speed > 0.5 {
            trig::atan2(v_world.y, horiz_speed)
        } else {
            pitch_rad
        };
        let flight_path_yaw_rad = if speed > 0.5 {
            wrap_pi(trig::atan2(v_world.z, v_world.x) - heading_rad)
        } else {
            0.0
        };

        let aoa_rad = if speed > 0.5 {
            trig::atan2(-v_body.y, v_body.x.abs().max(EPSILON))
        } else {
            0.0
        };
        let sideslip_rad = if speed > 0.5 {
            trig::atan2(
                v_body.z,
                trig::sqrt((v_body.x * v_body.x + v_body.y * v_body.y).max(EPSILON)),
            )
        } else {
            0.0
        };

        let f_body = q_inv.rotate_vec3(body.force).map_err(HudError::from)?;
        let g_load = f_body.y * mass.mass_props_inv_mass() / 9.80665;

        let altitude_msl_m = body.position.y;
        let agl = (altitude_msl_m - terrain_elevation_m).max(0.0);

        let telemetry = Self {
            pitch_rad,
            roll_rad,
            heading_rad,
            indicated_airspeed_mps: speed,
            true_airspeed_mps: speed,
            mach,
            altitude_msl_m,
            radar_altitude_agl_m: agl,
            vertical_speed_mps: v_world.y,
            flight_path_pitch_rad,
            flight_path_yaw_rad,
            aoa_rad,
            sideslip_rad,
            g_load: g_load.clamp(-MAX_G_LOAD, MAX_G_LOAD),
            stall_warning: false,
            overspeed_warning: false,
            terrain_warning: false,
            pull_up_warning: false,
        };
        telemetry.telemetry_validate()?;
        Ok(telemetry)
    }
}

/// Active cockpit annunciator flags computed from telemetry and thresholds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct AnnunciatorFlags {
    /// Stall warning active (`STALL`).
    pub stall: bool,
    /// Overspeed warning active (`OVERSPEED`).
    pub overspeed: bool,
    /// Ground proximity terrain caution active (`TERRAIN`).
    pub terrain: bool,
    /// Ground proximity pull-up warning active (`PULL UP`).
    pub pull_up: bool,
}

impl AnnunciatorFlags {
    /// Count active annunciator warnings (`0..=4`).
    #[inline]
    pub const fn annunciator_count(&self) -> usize {
        (self.stall as usize)
            + (self.overspeed as usize)
            + (self.terrain as usize)
            + (self.pull_up as usize)
    }
}

/// Deterministic summary metadata produced for every HUD frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudFrameSummary {
    /// Number of primitives emitted into `HudScratch` or `PaintScene`.
    pub primitive_count: usize,
    /// Number of pitch ladder rungs emitted (after clipping).
    pub pitch_rungs_emitted: usize,
    /// Total number of airspeed/altitude/heading tape ticks emitted.
    pub tape_ticks_emitted: usize,
    /// Evaluated annunciator flags.
    pub annunciators: AnnunciatorFlags,
    /// Canonical FNV-1a 64-bit digest of the emitted frame geometry and labels.
    pub frame_digest_fnv1a: u64,
}

// ---------------------------------------------------------------------------
// Zero-Alloc Scratch Primitives
// ---------------------------------------------------------------------------

/// Geometric payload of a zero-allocation [`HudPrimitive`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HudPrimitiveKind {
    /// Axis-aligned rectangle.
    Rect {
        /// Top-left X coordinate.
        x: f64,
        /// Top-left Y coordinate.
        y: f64,
        /// Rectangle width (`>= 0`).
        width: f64,
        /// Rectangle height (`>= 0`).
        height: f64,
        /// Whether the rectangle interior is filled.
        filled: bool,
        /// Whether the rectangle border is stroked.
        stroked: bool,
    },
    /// Clipped 2D line segment.
    Line {
        /// Start X coordinate.
        x1: f64,
        /// Start Y coordinate.
        y1: f64,
        /// End X coordinate.
        x2: f64,
        /// End Y coordinate.
        y2: f64,
        /// Stroke width in pixels.
        stroke_width: f64,
        /// Dashed line style (used for negative pitch rungs).
        dashed: bool,
    },
    /// Stroked ellipse/circle (e.g., flight-path marker circle).
    Ellipse {
        /// Center X coordinate.
        cx: f64,
        /// Center Y coordinate.
        cy: f64,
        /// X radius.
        rx: f64,
        /// Y radius.
        ry: f64,
        /// Stroke width in pixels.
        stroke_width: f64,
    },
    /// Convex polygon with up to 5 vertices (e.g., clipped sky/ground horizon half-plane).
    Polygon {
        /// Vertex X coordinates (`0..count`).
        vx: [f64; 5],
        /// Vertex Y coordinates (`0..count`).
        vy: [f64; 5],
        /// Number of active vertices (`3..=5`).
        count: u8,
        /// Whether polygon is filled.
        filled: bool,
        /// Whether polygon outline is stroked.
        stroked: bool,
    },
    /// Numeric or annunciator text label.
    Text {
        /// Anchor X coordinate.
        x: f64,
        /// Baseline Y coordinate.
        y: f64,
        /// Font size in pixels.
        font_size: f64,
        /// Center-align text around `x` when true, otherwise left-align.
        align_center: bool,
    },
}

/// Fixed-size stack-friendly primitive record stored inside [`HudScratch`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HudPrimitive {
    id_bytes: [u8; 40],
    id_len: u8,
    label_bytes: [u8; 32],
    label_len: u8,
    color: &'static str,
    /// Primitive geometry payload.
    pub kind: HudPrimitiveKind,
}

impl HudPrimitive {
    fn new(id: &str, label: &str, color: &'static str, kind: HudPrimitiveKind) -> Self {
        let mut id_bytes = [0u8; 40];
        let id_raw = id.as_bytes();
        let id_len = id_raw.len().min(id_bytes.len());
        id_bytes[..id_len].copy_from_slice(&id_raw[..id_len]);

        let mut label_bytes = [0u8; 32];
        let lbl_raw = label.as_bytes();
        let label_len = lbl_raw.len().min(label_bytes.len());
        label_bytes[..label_len].copy_from_slice(&lbl_raw[..label_len]);

        Self {
            id_bytes,
            id_len: id_len as u8,
            label_bytes,
            label_len: label_len as u8,
            color,
            kind,
        }
    }

    /// Return the stable deterministic identifier of this primitive.
    #[inline]
    pub fn primitive_id(&self) -> &str {
        core::str::from_utf8(&self.id_bytes[..self.id_len as usize]).unwrap_or("")
    }

    /// Return the stack-formatted text label (empty for non-text primitives).
    #[inline]
    pub fn primitive_label(&self) -> &str {
        core::str::from_utf8(&self.label_bytes[..self.label_len as usize]).unwrap_or("")
    }

    /// Return the CSS color string associated with this primitive.
    #[inline]
    pub const fn primitive_color(&self) -> &'static str {
        self.color
    }
}

/// Caller-owned pre-allocated primitive buffer for zero-allocation HUD builds (`VER07`).
#[derive(Clone, Debug, PartialEq)]
pub struct HudScratch {
    primitives: Vec<HudPrimitive>,
    last_summary: Option<HudFrameSummary>,
}

impl HudScratch {
    /// Allocate a [`HudScratch`] with [`DEFAULT_SCRATCH_CAPACITY`] (`512`) slots.
    pub fn hud_scratch_new() -> Self {
        Self::hud_scratch_with_capacity(DEFAULT_SCRATCH_CAPACITY)
    }

    /// Allocate a [`HudScratch`] with the specified primitive capacity.
    pub fn hud_scratch_with_capacity(capacity: usize) -> Self {
        Self {
            primitives: Vec::with_capacity(capacity.min(MAX_PRIMITIVES)),
            last_summary: None,
        }
    }

    /// Return the pre-allocated primitive capacity.
    #[inline]
    pub fn hud_scratch_capacity(&self) -> usize {
        self.primitives.capacity()
    }

    /// Return the number of primitives currently stored.
    #[inline]
    pub fn hud_scratch_len(&self) -> usize {
        self.primitives.len()
    }

    /// Borrow the slice of primitives generated in the last frame.
    #[inline]
    pub fn hud_scratch_primitives(&self) -> &[HudPrimitive] {
        &self.primitives
    }

    /// Return the summary of the last successfully built frame.
    #[inline]
    pub const fn hud_scratch_summary(&self) -> Option<HudFrameSummary> {
        self.last_summary
    }

    /// Clear all stored primitives while retaining allocated capacity.
    #[inline]
    pub fn hud_scratch_clear(&mut self) {
        self.primitives.clear();
        self.last_summary = None;
    }

    /// Compute the deterministic canonical FNV-1a digest of the scratch contents.
    pub fn hud_scratch_digest_fnv1a(&self) -> Result<u64, HudError> {
        let mut hash = FNV1A_OFFSET;
        fnv1a_u64(&mut hash, self.primitives.len() as u64);
        for prim in &self.primitives {
            hash_primitive(&mut hash, prim);
        }
        Ok(hash)
    }

    /// Lower the primitives currently stored in `self` into a fresh [`PaintScene`]
    /// with stable `PaintBase.id`s.
    pub fn hud_scratch_to_paint_scene(&self, config: &HudConfig) -> Result<PaintScene, HudError> {
        config.hud_config_validate()?;
        let bg = match config.mode {
            HudDisplayMode::HudOverlayOnly => COLOR_TRANSPARENT.to_string(),
            _ => COLOR_BG.to_string(),
        };
        let mut instructions = Vec::with_capacity(self.primitives.len());
        for prim in &self.primitives {
            instructions.push(primitive_to_instruction(prim));
        }
        Ok(PaintScene {
            width: canonical_f64(config.viewport_width),
            height: canonical_f64(config.viewport_height),
            background: bg,
            instructions,
            id: Some("cockpit-hud-paint:frame".to_string()),
            metadata: None,
        })
    }
}

// ---------------------------------------------------------------------------
// Internal Validation & Deterministic Math Helpers
// ---------------------------------------------------------------------------

#[inline]
fn check_finite(v: f64) -> Result<f64, HudError> {
    if !v.is_finite() {
        return Err(HudError::NonFiniteInput);
    }
    Ok(v)
}

#[inline]
fn check_angle(rad: f64) -> Result<f64, HudError> {
    if !rad.is_finite() {
        return Err(HudError::NonFiniteInput);
    }
    if rad.abs() > MAX_ANGLE_RAD {
        return Err(HudError::AngleOutOfBounds);
    }
    Ok(rad)
}

#[inline]
fn canonical_f64(v: f64) -> f64 {
    if v == 0.0 {
        0.0
    } else {
        v
    }
}

/// Wrap angle in radians (`|x| <= MAX_ANGLE_RAD`) into `[-PI, PI]` using pure arithmetic.
#[inline]
fn wrap_pi(x: f64) -> f64 {
    let turns = x * INV_TWO_PI;
    let k = (turns + ROUND_MAGIC) - ROUND_MAGIC;
    let mut r = x - k * TWO_PI;
    if r > trig::PI {
        r -= TWO_PI;
    }
    if r < -trig::PI {
        r += TWO_PI;
    }
    canonical_f64(r)
}

/// Wrap angle in radians (`|x| <= MAX_ANGLE_RAD`) into `[0, 2*PI)` using pure arithmetic.
#[inline]
fn wrap_2pi(x: f64) -> f64 {
    let mut r = wrap_pi(x);
    if r < 0.0 {
        r += TWO_PI;
    }
    if r >= TWO_PI {
        r -= TWO_PI;
    }
    canonical_f64(r)
}

/// Round finite `f64` to nearest `i64` (saturating safely inside tape ranges) without `libm`.
#[inline]
fn round_to_i64(x: f64) -> i64 {
    let clamped = x.clamp(-1.0e15, 1.0e15);
    let rounded = (clamped + ROUND_MAGIC) - ROUND_MAGIC;
    rounded as i64
}

/// Floor division of `i64` by positive `step > 0`.
#[inline]
fn floor_div_step(val: i64, step: i64) -> i64 {
    if step <= 0 {
        return 0;
    }
    let q = val / step;
    let r = val % step;
    if r < 0 {
        q - 1
    } else {
        q
    }
}

// ---------------------------------------------------------------------------
// Stack Numeric Formatting (Zero Heap Allocation)
// ---------------------------------------------------------------------------

struct StackText {
    buf: [u8; 32],
    len: usize,
}

impl StackText {
    const fn new() -> Self {
        Self {
            buf: [0u8; 32],
            len: 0,
        }
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }

    fn push_byte(&mut self, b: u8) {
        if self.len < self.buf.len() {
            self.buf[self.len] = b;
            self.len += 1;
        }
    }

    fn push_str(&mut self, s: &str) {
        for &b in s.as_bytes() {
            self.push_byte(b);
        }
    }

    fn push_u64(&mut self, mut n: u64) {
        let mut tmp = [0u8; 20];
        let mut i = 0usize;
        if n == 0 {
            self.push_byte(b'0');
            return;
        }
        while n > 0 && i < tmp.len() {
            tmp[i] = b'0' + (n % 10) as u8;
            n /= 10;
            i += 1;
        }
        while i > 0 {
            i -= 1;
            self.push_byte(tmp[i]);
        }
    }

    fn push_i64(&mut self, v: i64) {
        if v < 0 {
            self.push_byte(b'-');
            let mag = v.unsigned_abs();
            self.push_u64(mag);
        } else {
            self.push_u64(v as u64);
        }
    }

    fn push_u64_padded(&mut self, mut n: u64, width: usize) {
        let mut tmp = [b'0'; 20];
        let w = width.min(tmp.len());
        let mut idx = w;
        while idx > 0 {
            idx -= 1;
            tmp[idx] = b'0' + (n % 10) as u8;
            n /= 10;
        }
        for &b in &tmp[..w] {
            self.push_byte(b);
        }
    }

    fn format_int(v: i64) -> Self {
        let mut out = Self::new();
        out.push_i64(v);
        out
    }

    fn format_prefixed_int(prefix: &str, v: i64) -> Self {
        let mut out = Self::new();
        out.push_str(prefix);
        out.push_i64(v);
        out
    }

    fn format_heading_deg(deg: i64) -> Self {
        let wrapped = deg.rem_euclid(360);
        let mut out = Self::new();
        out.push_u64_padded(wrapped as u64, 3);
        out
    }

    fn format_mach(mach: f64) -> Self {
        let scaled = round_to_i64(canonical_f64(mach).clamp(0.0, MAX_MACH) * 100.0);
        let whole = (scaled / 100) as u64;
        let frac = (scaled % 100) as u64;
        let mut out = Self::new();
        out.push_byte(b'M');
        out.push_u64(whole);
        out.push_byte(b'.');
        out.push_u64_padded(frac, 2);
        out
    }

    fn format_signed_tenths(prefix: &str, val: f64) -> Self {
        let scaled = round_to_i64(canonical_f64(val).clamp(-999.9, 999.9) * 10.0);
        let mut out = Self::new();
        out.push_str(prefix);
        if scaled < 0 {
            out.push_byte(b'-');
        } else {
            out.push_byte(b'+');
        }
        let mag = scaled.unsigned_abs();
        out.push_u64(mag / 10);
        out.push_byte(b'.');
        out.push_u64(mag % 10);
        out
    }

    fn format_indexed_id(prefix: &str, index: usize) -> Self {
        let mut out = Self::new();
        out.push_str(prefix);
        out.push_u64(index as u64);
        out
    }
}

// ---------------------------------------------------------------------------
// 2D Clipping Algorithms (Liang-Barsky & Sutherland-Hodgman)
// ---------------------------------------------------------------------------

/// Axis-aligned clipping rectangle `[x, x + width] x [y, y + height]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HudRect {
    /// Left edge X coordinate.
    pub x: f64,
    /// Top edge Y coordinate.
    pub y: f64,
    /// Rectangle width (`>= 0`).
    pub width: f64,
    /// Rectangle height (`>= 0`).
    pub height: f64,
}

/// Clip a 2D line segment `(x1, y1) -> (x2, y2)` to `rect` using Liang-Barsky
/// parametric clipping.
///
/// Returns `None` if the segment lies completely outside `rect` or any
/// coordinate is non-finite.
pub fn clip_line_liang_barsky(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    rect: HudRect,
) -> Option<(f64, f64, f64, f64)> {
    let rx = rect.x;
    let ry = rect.y;
    let rw = rect.width;
    let rh = rect.height;
    if !x1.is_finite()
        || !y1.is_finite()
        || !x2.is_finite()
        || !y2.is_finite()
        || !rx.is_finite()
        || !ry.is_finite()
        || !rw.is_finite()
        || !rh.is_finite()
        || rw < 0.0
        || rh < 0.0
    {
        return None;
    }

    let xmin = rx;
    let xmax = rx + rw;
    let ymin = ry;
    let ymax = ry + rh;

    let dx = x2 - x1;
    let dy = y2 - y1;

    let p = [-dx, dx, -dy, dy];
    let q = [x1 - xmin, xmax - x1, y1 - ymin, ymax - y1];

    let mut u0 = 0.0_f64;
    let mut u1 = 1.0_f64;

    for i in 0..4 {
        let pi = p[i];
        let qi = q[i];
        if pi.abs() <= 0.0 {
            if qi < 0.0 {
                return None;
            }
        } else {
            let t = qi / pi;
            if pi < 0.0 {
                if t > u1 {
                    return None;
                }
                if t > u0 {
                    u0 = t;
                }
            } else {
                if t < u0 {
                    return None;
                }
                if t < u1 {
                    u1 = t;
                }
            }
        }
    }

    let cx1 = canonical_f64((x1 + u0 * dx).clamp(xmin, xmax));
    let cy1 = canonical_f64((y1 + u0 * dy).clamp(ymin, ymax));
    let cx2 = canonical_f64((x1 + u1 * dx).clamp(xmin, xmax));
    let cy2 = canonical_f64((y1 + u1 * dy).clamp(ymin, ymax));

    Some((cx1, cy1, cx2, cy2))
}

/// Clip an axis-aligned rectangle against a half-plane defined by point `(ox, oy)`
/// and unit normal `(nx, ny)`:
/// - `keep_le_zero == true`: keeps `(P - O) . N <= 0` (sky)
/// - `keep_le_zero == false`: keeps `(P - O) . N >= 0` (ground)
fn clip_rect_half_plane(
    rect: HudRect,
    ox: f64,
    oy: f64,
    nx: f64,
    ny: f64,
    keep_le_zero: bool,
) -> ([f64; 5], [f64; 5], usize) {
    let rx = rect.x;
    let ry = rect.y;
    let rw = rect.width;
    let rh = rect.height;
    let rect_x = [rx, rx + rw, rx + rw, rx];
    let rect_y = [ry, ry, ry + rh, ry + rh];

    let mut out_x = [0.0_f64; 5];
    let mut out_y = [0.0_f64; 5];
    let mut count = 0usize;

    let signed = |px: f64, py: f64| -> f64 {
        let d = (px - ox) * nx + (py - oy) * ny;
        if keep_le_zero {
            -d
        } else {
            d
        }
    };

    for i in 0..4 {
        let j = (i + 1) % 4;
        let ax = rect_x[i];
        let ay = rect_y[i];
        let bx = rect_x[j];
        let by = rect_y[j];

        let da = signed(ax, ay);
        let db = signed(bx, by);
        let a_in = da >= 0.0;
        let b_in = db >= 0.0;

        if a_in && count < 5 {
            out_x[count] = canonical_f64(ax.clamp(rx, rx + rw));
            out_y[count] = canonical_f64(ay.clamp(ry, ry + rh));
            count += 1;
        }
        if a_in != b_in {
            let denom = da - db;
            if denom.abs() > EPSILON && count < 5 {
                let t = (da / denom).clamp(0.0, 1.0);
                let ix = (ax + t * (bx - ax)).clamp(rx, rx + rw);
                let iy = (ay + t * (by - ay)).clamp(ry, ry + rh);
                out_x[count] = canonical_f64(ix);
                out_y[count] = canonical_f64(iy);
                count += 1;
            }
        }
    }

    (out_x, out_y, count)
}

// ---------------------------------------------------------------------------
// FNV-1a Canonical Digest Helpers
// ---------------------------------------------------------------------------

#[inline]
fn fnv1a_bytes(hash: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *hash ^= b as u64;
        *hash = hash.wrapping_mul(FNV1A_PRIME);
    }
}

#[inline]
fn fnv1a_u64(hash: &mut u64, v: u64) {
    fnv1a_bytes(hash, &v.to_le_bytes());
}

#[inline]
fn fnv1a_f64(hash: &mut u64, v: f64) {
    let bits = if v.is_nan() {
        CANON_NAN_BITS
    } else if v == 0.0 {
        0_u64
    } else {
        v.to_bits()
    };
    fnv1a_u64(hash, bits);
}

#[inline]
fn fnv1a_str(hash: &mut u64, s: &str) {
    fnv1a_u64(hash, s.len() as u64);
    fnv1a_bytes(hash, s.as_bytes());
}

fn hash_primitive(hash: &mut u64, prim: &HudPrimitive) {
    fnv1a_str(hash, prim.primitive_id());
    fnv1a_str(hash, prim.primitive_label());
    fnv1a_str(hash, prim.color);
    match prim.kind {
        HudPrimitiveKind::Rect {
            x,
            y,
            width,
            height,
            filled,
            stroked,
        } => {
            fnv1a_u64(hash, 1);
            fnv1a_f64(hash, x);
            fnv1a_f64(hash, y);
            fnv1a_f64(hash, width);
            fnv1a_f64(hash, height);
            fnv1a_u64(hash, filled as u64);
            fnv1a_u64(hash, stroked as u64);
        }
        HudPrimitiveKind::Line {
            x1,
            y1,
            x2,
            y2,
            stroke_width,
            dashed,
        } => {
            fnv1a_u64(hash, 2);
            fnv1a_f64(hash, x1);
            fnv1a_f64(hash, y1);
            fnv1a_f64(hash, x2);
            fnv1a_f64(hash, y2);
            fnv1a_f64(hash, stroke_width);
            fnv1a_u64(hash, dashed as u64);
        }
        HudPrimitiveKind::Ellipse {
            cx,
            cy,
            rx,
            ry,
            stroke_width,
        } => {
            fnv1a_u64(hash, 3);
            fnv1a_f64(hash, cx);
            fnv1a_f64(hash, cy);
            fnv1a_f64(hash, rx);
            fnv1a_f64(hash, ry);
            fnv1a_f64(hash, stroke_width);
        }
        HudPrimitiveKind::Polygon {
            vx,
            vy,
            count,
            filled,
            stroked,
        } => {
            fnv1a_u64(hash, 4);
            fnv1a_u64(hash, count as u64);
            let n = (count as usize).min(5);
            for i in 0..n {
                fnv1a_f64(hash, vx[i]);
                fnv1a_f64(hash, vy[i]);
            }
            fnv1a_u64(hash, filled as u64);
            fnv1a_u64(hash, stroked as u64);
        }
        HudPrimitiveKind::Text {
            x,
            y,
            font_size,
            align_center,
        } => {
            fnv1a_u64(hash, 5);
            fnv1a_f64(hash, x);
            fnv1a_f64(hash, y);
            fnv1a_f64(hash, font_size);
            fnv1a_u64(hash, align_center as u64);
        }
    }
}

// ---------------------------------------------------------------------------
// Geometry & Layout Model
// ---------------------------------------------------------------------------

struct HudLayout {
    w: f64,
    h: f64,
    adi_x: f64,
    adi_y: f64,
    adi_w: f64,
    adi_h: f64,
    cx: f64,
    cy: f64,
    px_per_rad: f64,
    ias_x: f64,
    ias_y: f64,
    ias_w: f64,
    ias_h: f64,
    alt_x: f64,
    alt_y: f64,
    alt_w: f64,
    alt_h: f64,
    vsi_x: f64,
    vsi_y: f64,
    vsi_w: f64,
    vsi_h: f64,
    hdg_x: f64,
    hdg_y: f64,
    hdg_w: f64,
    hdg_h: f64,
    top_y: f64,
    top_h: f64,
}

impl HudLayout {
    fn from_config(cfg: &HudConfig) -> Self {
        let w = cfg.viewport_width;
        let h = cfg.viewport_height;
        let adi_x = w * 0.22;
        let adi_y = h * 0.14;
        let adi_w = w * 0.56;
        let adi_h = h * 0.64;
        let cx = adi_x + 0.5 * adi_w;
        let cy = adi_y + 0.5 * adi_h;
        let fov_rad = trig::radians(cfg.fov_vertical_deg).max(EPSILON);
        let px_per_rad = adi_h / fov_rad;

        Self {
            w,
            h,
            adi_x,
            adi_y,
            adi_w,
            adi_h,
            cx,
            cy,
            px_per_rad,
            ias_x: w * 0.03,
            ias_y: h * 0.16,
            ias_w: w * 0.15,
            ias_h: h * 0.60,
            alt_x: w * 0.80,
            alt_y: h * 0.16,
            alt_w: w * 0.14,
            alt_h: h * 0.60,
            vsi_x: w * 0.945,
            vsi_y: h * 0.20,
            vsi_w: w * 0.045,
            vsi_h: h * 0.52,
            hdg_x: w * 0.22,
            hdg_y: h * 0.82,
            hdg_w: w * 0.56,
            hdg_h: h * 0.13,
            top_y: h * 0.02,
            top_h: h * 0.10,
        }
    }

    #[inline]
    const fn adi_rect(&self) -> HudRect {
        HudRect {
            x: self.adi_x,
            y: self.adi_y,
            width: self.adi_w,
            height: self.adi_h,
        }
    }

    #[inline]
    const fn ias_rect(&self) -> HudRect {
        HudRect {
            x: self.ias_x,
            y: self.ias_y,
            width: self.ias_w,
            height: self.ias_h,
        }
    }

    #[inline]
    const fn alt_rect(&self) -> HudRect {
        HudRect {
            x: self.alt_x,
            y: self.alt_y,
            width: self.alt_w,
            height: self.alt_h,
        }
    }

    #[inline]
    const fn vsi_rect(&self) -> HudRect {
        HudRect {
            x: self.vsi_x,
            y: self.vsi_y,
            width: self.vsi_w,
            height: self.vsi_h,
        }
    }

    #[inline]
    const fn hdg_rect(&self) -> HudRect {
        HudRect {
            x: self.hdg_x,
            y: self.hdg_y,
            width: self.hdg_w,
            height: self.hdg_h,
        }
    }
}

fn evaluate_annunciators(t: &FlightTelemetry, cfg: &HudConfig) -> AnnunciatorFlags {
    let stall = t.stall_warning
        || t.aoa_rad > cfg.aoa_stall_rad
        || (t.indicated_airspeed_mps < cfg.ias_stall_mps && t.radar_altitude_agl_m > 5.0);
    let overspeed = t.overspeed_warning
        || t.indicated_airspeed_mps > cfg.ias_overspeed_mps
        || t.mach > cfg.mach_overspeed;
    let terrain = t.terrain_warning
        || (t.radar_altitude_agl_m < cfg.terrain_caution_agl_m && t.vertical_speed_mps < -5.0);
    let pull_up = t.pull_up_warning
        || (t.radar_altitude_agl_m < cfg.pull_up_agl_m && t.vertical_speed_mps < -3.0);
    AnnunciatorFlags {
        stall,
        overspeed,
        terrain,
        pull_up,
    }
}

// ---------------------------------------------------------------------------
// Core HUD Frame Builder (Zero-Alloc Scratch Path)
// ---------------------------------------------------------------------------

struct ScratchEmitter<'a> {
    out: Option<&'a mut Vec<HudPrimitive>>,
    count: usize,
    pitch_rungs: usize,
    tape_ticks: usize,
    hash: u64,
}

impl<'a> ScratchEmitter<'a> {
    fn new(out: Option<&'a mut Vec<HudPrimitive>>) -> Self {
        Self {
            out,
            count: 0,
            pitch_rungs: 0,
            tape_ticks: 0,
            hash: FNV1A_OFFSET,
        }
    }

    fn emit(&mut self, prim: HudPrimitive) {
        self.count += 1;
        hash_primitive(&mut self.hash, &prim);
        if let Some(vec) = self.out.as_mut() {
            vec.push(prim);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_clipped_line(
        &mut self,
        id: &str,
        color: &'static str,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        rect: HudRect,
        stroke_width: f64,
        dashed: bool,
    ) -> bool {
        if let Some((cx1, cy1, cx2, cy2)) = clip_line_liang_barsky(x1, y1, x2, y2, rect) {
            self.emit(HudPrimitive::new(
                id,
                "",
                color,
                HudPrimitiveKind::Line {
                    x1: cx1,
                    y1: cy1,
                    x2: cx2,
                    y2: cy2,
                    stroke_width,
                    dashed,
                },
            ));
            true
        } else {
            false
        }
    }
}

fn run_scratch_pipeline(
    t: &FlightTelemetry,
    cfg: &HudConfig,
    em: &mut ScratchEmitter<'_>,
) -> HudFrameSummary {
    let layout = HudLayout::from_config(cfg);
    let pitch = wrap_pi(t.pitch_rad);
    let roll = wrap_pi(t.roll_rad);
    let heading = wrap_2pi(t.heading_rad);
    let cos_r = trig::cos(roll);
    let sin_r = trig::sin(roll);
    // Horizon screen-space tangent (t) and ground-pointing normal (n).
    let tx = cos_r;
    let ty = -sin_r;
    let nx = sin_r;
    let ny = cos_r;

    let stroke_primary = match cfg.mode {
        HudDisplayMode::HudOverlayOnly => COLOR_HUD_GREEN,
        _ => COLOR_HUD_WHITE,
    };

    // 1. Viewport backdrop & ADI attitude sky/ground split.
    if cfg.mode != HudDisplayMode::HudOverlayOnly {
        em.emit(HudPrimitive::new(
            "hud.bg",
            "",
            COLOR_BG,
            HudPrimitiveKind::Rect {
                x: 0.0,
                y: 0.0,
                width: canonical_f64(layout.w),
                height: canonical_f64(layout.h),
                filled: true,
                stroked: false,
            },
        ));

        let d_horizon = pitch * layout.px_per_rad;
        let ox = layout.cx + nx * d_horizon;
        let oy = layout.cy + ny * d_horizon;

        let (sky_vx, sky_vy, sky_n) = clip_rect_half_plane(layout.adi_rect(), ox, oy, nx, ny, true);
        if sky_n >= 3 {
            em.emit(HudPrimitive::new(
                "hud.adi.sky",
                "",
                COLOR_SKY,
                HudPrimitiveKind::Polygon {
                    vx: sky_vx,
                    vy: sky_vy,
                    count: sky_n as u8,
                    filled: true,
                    stroked: false,
                },
            ));
        }

        let (gnd_vx, gnd_vy, gnd_n) =
            clip_rect_half_plane(layout.adi_rect(), ox, oy, nx, ny, false);
        if gnd_n >= 3 {
            em.emit(HudPrimitive::new(
                "hud.adi.ground",
                "",
                COLOR_GROUND,
                HudPrimitiveKind::Polygon {
                    vx: gnd_vx,
                    vy: gnd_vy,
                    count: gnd_n as u8,
                    filled: true,
                    stroked: false,
                },
            ));
        }
    }

    // ADI frame border.
    em.emit(HudPrimitive::new(
        "hud.adi.frame",
        "",
        stroke_primary,
        HudPrimitiveKind::Rect {
            x: canonical_f64(layout.adi_x),
            y: canonical_f64(layout.adi_y),
            width: canonical_f64(layout.adi_w),
            height: canonical_f64(layout.adi_h),
            filled: false,
            stroked: true,
        },
    ));

    // 2. Artificial horizon line (clipped to ADI rect).
    let d_horizon = pitch * layout.px_per_rad;
    let h_ox = layout.cx + nx * d_horizon;
    let h_oy = layout.cy + ny * d_horizon;
    let span = layout.adi_w + layout.adi_h;
    em.emit_clipped_line(
        "hud.adi.horizon",
        stroke_primary,
        h_ox - tx * span,
        h_oy - ty * span,
        h_ox + tx * span,
        h_oy + ty * span,
        layout.adi_rect(),
        2.0,
        false,
    );

    // 3. Clipped pitch ladder rungs (-90..=+90 in 5 deg steps, bounded by max_pitch_rungs).
    let rung_half_w = layout.adi_w * 0.16;
    let rung_gap = layout.adi_w * 0.05;
    let hook_len = (layout.adi_h * 0.025).max(3.0);
    let max_rungs = cfg.max_pitch_rungs.min(MAX_PITCH_RUNGS);
    let mut rung_idx = 0usize;
    let mut deg_step = -90_i32;
    while deg_step <= 90 && rung_idx < max_rungs {
        if deg_step != 0 {
            let rung_rad = trig::radians(deg_step as f64);
            let d_rung = (pitch - rung_rad) * layout.px_per_rad;
            let rx_c = layout.cx + nx * d_rung;
            let ry_c = layout.cy + ny * d_rung;
            let dashed = deg_step < 0;
            let hook_dir = if deg_step > 0 { 1.0 } else { -1.0 };

            let left_id = StackText::format_indexed_id("hud.pitch.l.", rung_idx);
            let right_id = StackText::format_indexed_id("hud.pitch.r.", rung_idx);

            let l_ok = em.emit_clipped_line(
                left_id.as_str(),
                stroke_primary,
                rx_c - tx * rung_half_w,
                ry_c - ty * rung_half_w,
                rx_c - tx * rung_gap,
                ry_c - ty * rung_gap,
                layout.adi_rect(),
                1.5,
                dashed,
            );
            let r_ok = em.emit_clipped_line(
                right_id.as_str(),
                stroke_primary,
                rx_c + tx * rung_gap,
                ry_c + ty * rung_gap,
                rx_c + tx * rung_half_w,
                ry_c + ty * rung_half_w,
                layout.adi_rect(),
                1.5,
                dashed,
            );

            if l_ok || r_ok {
                em.pitch_rungs += 1;
                let hl_id = StackText::format_indexed_id("hud.pitch.hl.", rung_idx);
                let hr_id = StackText::format_indexed_id("hud.pitch.hr.", rung_idx);
                em.emit_clipped_line(
                    hl_id.as_str(),
                    stroke_primary,
                    rx_c - tx * rung_half_w,
                    ry_c - ty * rung_half_w,
                    rx_c - tx * rung_half_w + nx * hook_len * hook_dir,
                    ry_c - ty * rung_half_w + ny * hook_len * hook_dir,
                    layout.adi_rect(),
                    1.5,
                    dashed,
                );
                em.emit_clipped_line(
                    hr_id.as_str(),
                    stroke_primary,
                    rx_c + tx * rung_half_w,
                    ry_c + ty * rung_half_w,
                    rx_c + tx * rung_half_w + nx * hook_len * hook_dir,
                    ry_c + ty * rung_half_w + ny * hook_len * hook_dir,
                    layout.adi_rect(),
                    1.5,
                    dashed,
                );
            }
            rung_idx += 1;
        }
        deg_step += 5;
    }

    // 4. Bank angle arc (-60°..+60°) and bank tick marks + pointer.
    let bank_radius = 0.42 * layout.adi_w.min(layout.adi_h);
    let bank_segments = 12usize;
    for i in 0..bank_segments {
        let a0 = trig::radians(-60.0 + (i as f64) * 10.0);
        let a1 = trig::radians(-60.0 + ((i + 1) as f64) * 10.0);
        let x0 = layout.cx + bank_radius * trig::sin(a0);
        let y0 = layout.cy - bank_radius * trig::cos(a0);
        let x1 = layout.cx + bank_radius * trig::sin(a1);
        let y1 = layout.cy - bank_radius * trig::cos(a1);
        let seg_id = StackText::format_indexed_id("hud.bank.arc.", i);
        em.emit_clipped_line(
            seg_id.as_str(),
            stroke_primary,
            x0,
            y0,
            x1,
            y1,
            layout.adi_rect(),
            1.5,
            false,
        );
    }

    let bank_marks = [
        -60.0, -45.0, -30.0, -20.0, -10.0, 0.0, 10.0, 20.0, 30.0, 45.0, 60.0,
    ];
    for (idx, &deg) in bank_marks.iter().enumerate() {
        let a = trig::radians(deg);
        let sa = trig::sin(a);
        let ca = trig::cos(a);
        let tick_len = if deg == 0.0 || deg.abs() == 30.0 || deg.abs() == 60.0 {
            (bank_radius * 0.08).max(4.0)
        } else {
            (bank_radius * 0.05).max(2.5)
        };
        let tick_id = StackText::format_indexed_id("hud.bank.tick.", idx);
        em.emit_clipped_line(
            tick_id.as_str(),
            stroke_primary,
            layout.cx + bank_radius * sa,
            layout.cy - bank_radius * ca,
            layout.cx + (bank_radius + tick_len) * sa,
            layout.cy - (bank_radius + tick_len) * ca,
            layout.adi_rect(),
            1.5,
            false,
        );
    }

    // Bank angle pointer inside arc at `roll`.
    let ptr_r_out = bank_radius - 2.0;
    let ptr_r_in = (bank_radius - (bank_radius * 0.09).max(4.0)).max(2.0);
    let ptr_tip_x = layout.cx - ptr_r_out * sin_r;
    let ptr_tip_y = layout.cy - ptr_r_out * cos_r;
    let ptr_base_x = layout.cx - ptr_r_in * sin_r;
    let ptr_base_y = layout.cy - ptr_r_in * cos_r;
    let ptr_half = (bank_radius * 0.045).max(2.5);
    let ptr_lx = ptr_base_x - cos_r * ptr_half;
    let ptr_ly = ptr_base_y + sin_r * ptr_half;
    let ptr_rx = ptr_base_x + cos_r * ptr_half;
    let ptr_ry = ptr_base_y - sin_r * ptr_half;
    em.emit_clipped_line(
        "hud.bank.ptr.l",
        COLOR_HUD_CYAN,
        ptr_tip_x,
        ptr_tip_y,
        ptr_lx,
        ptr_ly,
        layout.adi_rect(),
        2.0,
        false,
    );
    em.emit_clipped_line(
        "hud.bank.ptr.r",
        COLOR_HUD_CYAN,
        ptr_tip_x,
        ptr_tip_y,
        ptr_rx,
        ptr_ry,
        layout.adi_rect(),
        2.0,
        false,
    );

    // 5. Fixed aircraft boresight watermark.
    let wm_w = (layout.adi_w * 0.07).max(6.0);
    let wm_dip = (layout.adi_h * 0.025).max(3.0);
    em.emit_clipped_line(
        "hud.wm.l",
        COLOR_WARN_AMBER,
        layout.cx - wm_w,
        layout.cy,
        layout.cx - wm_dip,
        layout.cy,
        layout.adi_rect(),
        2.0,
        false,
    );
    em.emit_clipped_line(
        "hud.wm.vl",
        COLOR_WARN_AMBER,
        layout.cx - wm_dip,
        layout.cy,
        layout.cx,
        layout.cy + wm_dip,
        layout.adi_rect(),
        2.0,
        false,
    );
    em.emit_clipped_line(
        "hud.wm.vr",
        COLOR_WARN_AMBER,
        layout.cx,
        layout.cy + wm_dip,
        layout.cx + wm_dip,
        layout.cy,
        layout.adi_rect(),
        2.0,
        false,
    );
    em.emit_clipped_line(
        "hud.wm.r",
        COLOR_WARN_AMBER,
        layout.cx + wm_dip,
        layout.cy,
        layout.cx + wm_w,
        layout.cy,
        layout.adi_rect(),
        2.0,
        false,
    );

    // 6. Flight-Path Marker (FPM / velocity vector) clamped to ADI bounds.
    if cfg.mode != HudDisplayMode::PfdOnly {
        let fpm_r = (layout.adi_w.min(layout.adi_h) * 0.032).clamp(3.0, 14.0);
        let pad = fpm_r * 2.2;
        let fpm_dx = wrap_pi(t.flight_path_yaw_rad) * layout.px_per_rad;
        let fpm_dy = -(wrap_pi(t.flight_path_pitch_rad) - pitch) * layout.px_per_rad;
        let fpm_x = canonical_f64(
            (layout.cx + fpm_dx).clamp(layout.adi_x + pad, layout.adi_x + layout.adi_w - pad),
        );
        let fpm_y = canonical_f64(
            (layout.cy + fpm_dy).clamp(layout.adi_y + pad, layout.adi_y + layout.adi_h - pad),
        );

        em.emit(HudPrimitive::new(
            "hud.fpm.circle",
            "",
            COLOR_HUD_GREEN,
            HudPrimitiveKind::Ellipse {
                cx: fpm_x,
                cy: fpm_y,
                rx: fpm_r,
                ry: fpm_r,
                stroke_width: 1.8,
            },
        ));
        em.emit_clipped_line(
            "hud.fpm.wing.l",
            COLOR_HUD_GREEN,
            fpm_x - fpm_r * 2.0,
            fpm_y,
            fpm_x - fpm_r,
            fpm_y,
            layout.adi_rect(),
            1.8,
            false,
        );
        em.emit_clipped_line(
            "hud.fpm.wing.r",
            COLOR_HUD_GREEN,
            fpm_x + fpm_r,
            fpm_y,
            fpm_x + fpm_r * 2.0,
            fpm_y,
            layout.adi_rect(),
            1.8,
            false,
        );
        em.emit_clipped_line(
            "hud.fpm.tail",
            COLOR_HUD_GREEN,
            fpm_x,
            fpm_y - fpm_r,
            fpm_x,
            fpm_y - fpm_r * 1.9,
            layout.adi_rect(),
            1.8,
            false,
        );
    }

    // 7. Airspeed Tape (IAS + Mach).
    emit_airspeed_tape(t, cfg, &layout, stroke_primary, em);

    // 8. Altimeter Tape + VSI + Radar Altitude.
    emit_altimeter_and_vsi(t, cfg, &layout, stroke_primary, em);

    // 9. Heading / HSI Tape.
    emit_heading_tape(heading, cfg, &layout, stroke_primary, em);

    // 10. AoA, G-load, and Annunciator Warnings.
    let ann = emit_top_bar_and_annunciators(t, cfg, &layout, stroke_primary, em);

    HudFrameSummary {
        primitive_count: em.count,
        pitch_rungs_emitted: em.pitch_rungs,
        tape_ticks_emitted: em.tape_ticks,
        annunciators: ann,
        frame_digest_fnv1a: em.hash,
    }
}

fn emit_airspeed_tape(
    t: &FlightTelemetry,
    cfg: &HudConfig,
    layout: &HudLayout,
    stroke_primary: &'static str,
    em: &mut ScratchEmitter<'_>,
) {
    let filled_bg = cfg.mode != HudDisplayMode::HudOverlayOnly;
    em.emit(HudPrimitive::new(
        "hud.ias.frame",
        "",
        if filled_bg {
            COLOR_PANEL_BG
        } else {
            stroke_primary
        },
        HudPrimitiveKind::Rect {
            x: canonical_f64(layout.ias_x),
            y: canonical_f64(layout.ias_y),
            width: canonical_f64(layout.ias_w),
            height: canonical_f64(layout.ias_h),
            filled: filled_bg,
            stroked: true,
        },
    ));

    let ias_val = match cfg.units {
        HudUnits::Aviation => t.indicated_airspeed_mps * KNOTS_PER_MPS,
        HudUnits::Si => t.indicated_airspeed_mps,
    };
    let step = 10_i64;
    let px_per_unit = layout.ias_h / 100.0;
    let center_y = layout.ias_y + 0.5 * layout.ias_h;
    let base_tick = floor_div_step(round_to_i64(ias_val), step) * step;
    let max_ticks = (cfg.max_tape_ticks / 3).clamp(4, 24);
    let half_ticks = (max_ticks / 2) as i64;
    let font_sz = (layout.h * 0.022).clamp(7.0, 22.0);

    let mut tick_i = 0usize;
    let mut k = -half_ticks;
    while k <= half_ticks {
        let tick_val = base_tick + k * step;
        if tick_val >= 0 {
            let ty = center_y - ((tick_val as f64) - ias_val) * px_per_unit;
            if ty >= layout.ias_y + 4.0 && ty <= layout.ias_y + layout.ias_h - 4.0 {
                let tick_len = if tick_val % 20 == 0 {
                    layout.ias_w * 0.25
                } else {
                    layout.ias_w * 0.14
                };
                let tid = StackText::format_indexed_id("hud.ias.tick.", tick_i);
                em.emit_clipped_line(
                    tid.as_str(),
                    stroke_primary,
                    layout.ias_x + layout.ias_w - tick_len,
                    ty,
                    layout.ias_x + layout.ias_w,
                    ty,
                    layout.ias_rect(),
                    1.2,
                    false,
                );
                em.tape_ticks += 1;
                if tick_val % 20 == 0 && (ty - center_y).abs() > font_sz * 1.1 {
                    let lbl = StackText::format_int(tick_val);
                    let lid = StackText::format_indexed_id("hud.ias.lbl.", tick_i);
                    em.emit(HudPrimitive::new(
                        lid.as_str(),
                        lbl.as_str(),
                        stroke_primary,
                        HudPrimitiveKind::Text {
                            x: canonical_f64(layout.ias_x + layout.ias_w * 0.12),
                            y: canonical_f64(ty + font_sz * 0.35),
                            font_size: canonical_f64(font_sz),
                            align_center: false,
                        },
                    ));
                }
                tick_i += 1;
            }
        }
        k += 1;
    }

    // Center IAS readout box & Mach readout.
    let box_h = (layout.ias_h * 0.10).max(14.0);
    em.emit(HudPrimitive::new(
        "hud.ias.box",
        "",
        COLOR_BG,
        HudPrimitiveKind::Rect {
            x: canonical_f64(layout.ias_x),
            y: canonical_f64(center_y - 0.5 * box_h),
            width: canonical_f64(layout.ias_w),
            height: canonical_f64(box_h),
            filled: filled_bg,
            stroked: true,
        },
    ));
    let ias_lbl = StackText::format_int(round_to_i64(ias_val));
    em.emit(HudPrimitive::new(
        "hud.ias.readout",
        ias_lbl.as_str(),
        stroke_primary,
        HudPrimitiveKind::Text {
            x: canonical_f64(layout.ias_x + 0.5 * layout.ias_w),
            y: canonical_f64(center_y + font_sz * 0.35),
            font_size: canonical_f64(font_sz * 1.1),
            align_center: true,
        },
    ));

    let mach_lbl = StackText::format_mach(t.mach);
    em.emit(HudPrimitive::new(
        "hud.ias.mach",
        mach_lbl.as_str(),
        COLOR_HUD_CYAN,
        HudPrimitiveKind::Text {
            x: canonical_f64(layout.ias_x + 0.5 * layout.ias_w),
            y: canonical_f64(layout.ias_y + layout.ias_h + font_sz * 1.3),
            font_size: canonical_f64(font_sz),
            align_center: true,
        },
    ));
}

fn emit_altimeter_and_vsi(
    t: &FlightTelemetry,
    cfg: &HudConfig,
    layout: &HudLayout,
    stroke_primary: &'static str,
    em: &mut ScratchEmitter<'_>,
) {
    let filled_bg = cfg.mode != HudDisplayMode::HudOverlayOnly;
    em.emit(HudPrimitive::new(
        "hud.alt.frame",
        "",
        if filled_bg {
            COLOR_PANEL_BG
        } else {
            stroke_primary
        },
        HudPrimitiveKind::Rect {
            x: canonical_f64(layout.alt_x),
            y: canonical_f64(layout.alt_y),
            width: canonical_f64(layout.alt_w),
            height: canonical_f64(layout.alt_h),
            filled: filled_bg,
            stroked: true,
        },
    ));

    let alt_val = match cfg.units {
        HudUnits::Aviation => t.altitude_msl_m * FEET_PER_METER,
        HudUnits::Si => t.altitude_msl_m,
    };
    let step = match cfg.units {
        HudUnits::Aviation => 200_i64,
        HudUnits::Si => 50_i64,
    };
    let span_units = match cfg.units {
        HudUnits::Aviation => 1200.0,
        HudUnits::Si => 300.0,
    };
    let px_per_unit = layout.alt_h / span_units;
    let center_y = layout.alt_y + 0.5 * layout.alt_h;
    let base_tick = floor_div_step(round_to_i64(alt_val), step) * step;
    let max_ticks = (cfg.max_tape_ticks / 3).clamp(4, 24);
    let half_ticks = (max_ticks / 2) as i64;
    let font_sz = (layout.h * 0.022).clamp(7.0, 22.0);

    let mut tick_i = 0usize;
    let mut k = -half_ticks;
    while k <= half_ticks {
        let tick_val = base_tick + k * step;
        let ty = center_y - ((tick_val as f64) - alt_val) * px_per_unit;
        if ty >= layout.alt_y + 4.0 && ty <= layout.alt_y + layout.alt_h - 4.0 {
            let is_major = tick_val % (step * 2) == 0;
            let tick_len = if is_major {
                layout.alt_w * 0.25
            } else {
                layout.alt_w * 0.14
            };
            let tid = StackText::format_indexed_id("hud.alt.tick.", tick_i);
            em.emit_clipped_line(
                tid.as_str(),
                stroke_primary,
                layout.alt_x,
                ty,
                layout.alt_x + tick_len,
                ty,
                layout.alt_rect(),
                1.2,
                false,
            );
            em.tape_ticks += 1;
            if is_major && (ty - center_y).abs() > font_sz * 1.1 {
                let lbl = StackText::format_int(tick_val);
                let lid = StackText::format_indexed_id("hud.alt.lbl.", tick_i);
                em.emit(HudPrimitive::new(
                    lid.as_str(),
                    lbl.as_str(),
                    stroke_primary,
                    HudPrimitiveKind::Text {
                        x: canonical_f64(layout.alt_x + layout.alt_w * 0.30),
                        y: canonical_f64(ty + font_sz * 0.35),
                        font_size: canonical_f64(font_sz),
                        align_center: false,
                    },
                ));
            }
            tick_i += 1;
        }
        k += 1;
    }

    // Center altitude readout box + Radar Altitude AGL readout.
    let box_h = (layout.alt_h * 0.10).max(14.0);
    em.emit(HudPrimitive::new(
        "hud.alt.box",
        "",
        COLOR_BG,
        HudPrimitiveKind::Rect {
            x: canonical_f64(layout.alt_x),
            y: canonical_f64(center_y - 0.5 * box_h),
            width: canonical_f64(layout.alt_w),
            height: canonical_f64(box_h),
            filled: filled_bg,
            stroked: true,
        },
    ));
    let alt_lbl = StackText::format_int(round_to_i64(alt_val));
    em.emit(HudPrimitive::new(
        "hud.alt.readout",
        alt_lbl.as_str(),
        stroke_primary,
        HudPrimitiveKind::Text {
            x: canonical_f64(layout.alt_x + 0.5 * layout.alt_w),
            y: canonical_f64(center_y + font_sz * 0.35),
            font_size: canonical_f64(font_sz * 1.1),
            align_center: true,
        },
    ));

    let agl_val = match cfg.units {
        HudUnits::Aviation => t.radar_altitude_agl_m * FEET_PER_METER,
        HudUnits::Si => t.radar_altitude_agl_m,
    };
    let agl_lbl = StackText::format_prefixed_int("RA ", round_to_i64(agl_val));
    em.emit(HudPrimitive::new(
        "hud.alt.agl",
        agl_lbl.as_str(),
        COLOR_HUD_GREEN,
        HudPrimitiveKind::Text {
            x: canonical_f64(layout.alt_x + 0.5 * layout.alt_w),
            y: canonical_f64(layout.alt_y + layout.alt_h + font_sz * 1.3),
            font_size: canonical_f64(font_sz),
            align_center: true,
        },
    ));

    // Vertical Speed Indicator (VSI) bar + needle.
    em.emit(HudPrimitive::new(
        "hud.vsi.frame",
        "",
        stroke_primary,
        HudPrimitiveKind::Rect {
            x: canonical_f64(layout.vsi_x),
            y: canonical_f64(layout.vsi_y),
            width: canonical_f64(layout.vsi_w),
            height: canonical_f64(layout.vsi_h),
            filled: false,
            stroked: true,
        },
    ));
    let vsi_center_y = layout.vsi_y + 0.5 * layout.vsi_h;
    let vsi_norm = (t.vertical_speed_mps / 30.0).clamp(-1.0, 1.0);
    let vsi_target_y = vsi_center_y - vsi_norm * (0.46 * layout.vsi_h);
    em.emit_clipped_line(
        "hud.vsi.bar",
        COLOR_HUD_CYAN,
        layout.vsi_x + 0.5 * layout.vsi_w,
        vsi_center_y,
        layout.vsi_x + 0.5 * layout.vsi_w,
        vsi_target_y,
        layout.vsi_rect(),
        3.0,
        false,
    );
    let vsi_display = match cfg.units {
        HudUnits::Aviation => round_to_i64(t.vertical_speed_mps * FEET_PER_METER * 60.0),
        HudUnits::Si => round_to_i64(t.vertical_speed_mps),
    };
    let vsi_lbl = StackText::format_prefixed_int("VS ", vsi_display);
    em.emit(HudPrimitive::new(
        "hud.vsi.readout",
        vsi_lbl.as_str(),
        COLOR_HUD_CYAN,
        HudPrimitiveKind::Text {
            x: canonical_f64(layout.vsi_x + 0.5 * layout.vsi_w),
            y: canonical_f64(layout.vsi_y - font_sz * 0.4),
            font_size: canonical_f64(font_sz * 0.85),
            align_center: true,
        },
    ));
}

fn emit_heading_tape(
    heading_rad: f64,
    cfg: &HudConfig,
    layout: &HudLayout,
    stroke_primary: &'static str,
    em: &mut ScratchEmitter<'_>,
) {
    let filled_bg = cfg.mode != HudDisplayMode::HudOverlayOnly;
    em.emit(HudPrimitive::new(
        "hud.hdg.frame",
        "",
        if filled_bg {
            COLOR_PANEL_BG
        } else {
            stroke_primary
        },
        HudPrimitiveKind::Rect {
            x: canonical_f64(layout.hdg_x),
            y: canonical_f64(layout.hdg_y),
            width: canonical_f64(layout.hdg_w),
            height: canonical_f64(layout.hdg_h),
            filled: filled_bg,
            stroked: true,
        },
    ));

    let hdg_deg = canonical_f64(trig::degrees(heading_rad));
    let px_per_deg = layout.hdg_w / 60.0;
    let step = 5_i64;
    let base_tick = floor_div_step(round_to_i64(hdg_deg), step) * step;
    let max_ticks = (cfg.max_tape_ticks / 3).clamp(4, 24);
    let half_ticks = (max_ticks / 2) as i64;
    let font_sz = (layout.h * 0.022).clamp(7.0, 22.0);

    let mut tick_i = 0usize;
    let mut k = -half_ticks;
    while k <= half_ticks {
        let tick_deg = base_tick + k * step;
        let tx = layout.cx + ((tick_deg as f64) - hdg_deg) * px_per_deg;
        if tx >= layout.hdg_x + 4.0 && tx <= layout.hdg_x + layout.hdg_w - 4.0 {
            let is_major = tick_deg % 10 == 0;
            let tick_h = if is_major {
                layout.hdg_h * 0.28
            } else {
                layout.hdg_h * 0.16
            };
            let tid = StackText::format_indexed_id("hud.hdg.tick.", tick_i);
            em.emit_clipped_line(
                tid.as_str(),
                stroke_primary,
                tx,
                layout.hdg_y,
                tx,
                layout.hdg_y + tick_h,
                layout.hdg_rect(),
                1.2,
                false,
            );
            em.tape_ticks += 1;
            if is_major && (tx - layout.cx).abs() > font_sz * 1.8 {
                let lbl = StackText::format_heading_deg(tick_deg);
                let lid = StackText::format_indexed_id("hud.hdg.lbl.", tick_i);
                em.emit(HudPrimitive::new(
                    lid.as_str(),
                    lbl.as_str(),
                    stroke_primary,
                    HudPrimitiveKind::Text {
                        x: canonical_f64(tx),
                        y: canonical_f64(layout.hdg_y + layout.hdg_h * 0.72),
                        font_size: canonical_f64(font_sz),
                        align_center: true,
                    },
                ));
            }
            tick_i += 1;
        }
        k += 1;
    }

    // Center lubber mark and numeric heading readout.
    em.emit_clipped_line(
        "hud.hdg.lubber",
        COLOR_HUD_CYAN,
        layout.cx,
        layout.hdg_y,
        layout.cx,
        layout.hdg_y + layout.hdg_h * 0.35,
        layout.hdg_rect(),
        2.0,
        false,
    );

    let hdg_lbl = StackText::format_heading_deg(round_to_i64(hdg_deg));
    em.emit(HudPrimitive::new(
        "hud.hdg.readout",
        hdg_lbl.as_str(),
        COLOR_HUD_CYAN,
        HudPrimitiveKind::Text {
            x: canonical_f64(layout.cx),
            y: canonical_f64(layout.hdg_y + layout.hdg_h * 0.80),
            font_size: canonical_f64(font_sz * 1.05),
            align_center: true,
        },
    ));
}

fn emit_top_bar_and_annunciators(
    t: &FlightTelemetry,
    cfg: &HudConfig,
    layout: &HudLayout,
    stroke_primary: &'static str,
    em: &mut ScratchEmitter<'_>,
) -> AnnunciatorFlags {
    let font_sz = (layout.h * 0.024).clamp(7.0, 24.0);
    let aoa_deg = canonical_f64(trig::degrees(wrap_pi(t.aoa_rad)));
    let aoa_lbl = StackText::format_signed_tenths("AOA ", aoa_deg);
    em.emit(HudPrimitive::new(
        "hud.aoa.readout",
        aoa_lbl.as_str(),
        stroke_primary,
        HudPrimitiveKind::Text {
            x: canonical_f64(layout.ias_x),
            y: canonical_f64(layout.top_y + layout.top_h * 0.65),
            font_size: canonical_f64(font_sz),
            align_center: false,
        },
    ));

    let g_lbl = StackText::format_signed_tenths("G ", t.g_load);
    em.emit(HudPrimitive::new(
        "hud.gload.readout",
        g_lbl.as_str(),
        stroke_primary,
        HudPrimitiveKind::Text {
            x: canonical_f64(layout.alt_x + layout.alt_w),
            y: canonical_f64(layout.top_y + layout.top_h * 0.65),
            font_size: canonical_f64(font_sz),
            align_center: true,
        },
    ));

    let ann = evaluate_annunciators(t, cfg);
    let slot_w = (layout.adi_w * 0.23).max(12.0);
    let slot_h = (layout.top_h * 0.72).max(10.0);
    let slot_y = layout.top_y + 0.14 * layout.top_h;
    let start_x = layout.adi_x + 0.02 * layout.adi_w;
    let gap = layout.adi_w * 0.02;

    let slots = [
        (ann.stall, "hud.ann.stall", "STALL", COLOR_WARN_RED),
        (
            ann.overspeed,
            "hud.ann.overspeed",
            "OVERSPEED",
            COLOR_WARN_RED,
        ),
        (ann.terrain, "hud.ann.terrain", "TERRAIN", COLOR_WARN_AMBER),
        (ann.pull_up, "hud.ann.pullup", "PULL UP", COLOR_WARN_RED),
    ];

    for (idx, &(active, id, label, color)) in slots.iter().enumerate() {
        if active {
            let sx = start_x + (idx as f64) * (slot_w + gap);
            em.emit(HudPrimitive::new(
                id,
                "",
                color,
                HudPrimitiveKind::Rect {
                    x: canonical_f64(sx),
                    y: canonical_f64(slot_y),
                    width: canonical_f64(slot_w),
                    height: canonical_f64(slot_h),
                    filled: true,
                    stroked: true,
                },
            ));
            em.emit(HudPrimitive::new(
                id,
                label,
                COLOR_HUD_WHITE,
                HudPrimitiveKind::Text {
                    x: canonical_f64(sx + 0.5 * slot_w),
                    y: canonical_f64(slot_y + 0.68 * slot_h),
                    font_size: canonical_f64(font_sz * 0.85),
                    align_center: true,
                },
            ));
        }
    }

    ann
}

/// Build a deterministic HUD/PFD frame into a caller-owned [`HudScratch`] with
/// **zero heap allocations** (`VER07`) and **byte-for-byte atomic error preservation**.
///
/// Validates `telemetry` and `config` first, performs a zero-allocation dry-run pass
/// to verify `primitive_count <= config.max_primitives` and `<= scratch.hud_scratch_capacity()`,
/// and writes into `scratch` only when every invariant holds.
pub fn build_hud_frame_into(
    telemetry: &FlightTelemetry,
    config: &HudConfig,
    scratch: &mut HudScratch,
) -> Result<HudFrameSummary, HudError> {
    config.hud_config_validate()?;
    telemetry.telemetry_validate()?;

    // Pass 1: zero-allocation, zero-mutation dry run to verify exact primitive budget.
    let mut dry = ScratchEmitter::new(None);
    let summary = run_scratch_pipeline(telemetry, config, &mut dry);
    if summary.primitive_count > config.max_primitives {
        return Err(HudError::PrimitiveLimitExceeded);
    }
    if summary.primitive_count > scratch.primitives.capacity() {
        return Err(HudError::BufferTooSmall);
    }

    // Pass 2: commit into pre-reserved scratch buffer (guaranteed 0 allocations).
    scratch.primitives.clear();
    let mut live = ScratchEmitter::new(Some(&mut scratch.primitives));
    let committed = run_scratch_pipeline(telemetry, config, &mut live);
    scratch.last_summary = Some(committed);
    Ok(committed)
}

fn primitive_to_instruction(prim: &HudPrimitive) -> PaintInstruction {
    let base = PaintBase {
        id: Some(prim.primitive_id().to_string()),
        metadata: None,
    };
    match prim.kind {
        HudPrimitiveKind::Rect {
            x,
            y,
            width,
            height,
            filled,
            stroked,
        } => PaintInstruction::Rect(PaintRect {
            base,
            x,
            y,
            width,
            height,
            fill: if filled {
                Some(prim.color.to_string())
            } else {
                None
            },
            stroke: if stroked {
                Some(prim.color.to_string())
            } else {
                None
            },
            stroke_width: if stroked { Some(1.5) } else { None },
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }),
        HudPrimitiveKind::Line {
            x1,
            y1,
            x2,
            y2,
            stroke_width,
            dashed,
        } => PaintInstruction::Line(PaintLine {
            base,
            x1,
            y1,
            x2,
            y2,
            stroke: prim.color.to_string(),
            stroke_width: Some(stroke_width),
            stroke_cap: None,
            stroke_dash: if dashed { Some(vec![6.0, 4.0]) } else { None },
            stroke_dash_offset: None,
        }),
        HudPrimitiveKind::Ellipse {
            cx,
            cy,
            rx,
            ry,
            stroke_width,
        } => PaintInstruction::Ellipse(PaintEllipse {
            base,
            cx,
            cy,
            rx,
            ry,
            fill: None,
            stroke: Some(prim.color.to_string()),
            stroke_width: Some(stroke_width),
            stroke_dash: None,
            stroke_dash_offset: None,
        }),
        HudPrimitiveKind::Polygon {
            vx,
            vy,
            count,
            filled,
            stroked,
        } => {
            let n = (count as usize).min(5);
            let mut commands = Vec::with_capacity(n + 1);
            if n > 0 {
                commands.push(PathCommand::MoveTo { x: vx[0], y: vy[0] });
                for i in 1..n {
                    commands.push(PathCommand::LineTo { x: vx[i], y: vy[i] });
                }
                commands.push(PathCommand::Close);
            }
            PaintInstruction::Path(PaintPath {
                base,
                commands,
                fill: if filled {
                    Some(prim.color.to_string())
                } else {
                    None
                },
                fill_rule: None,
                stroke: if stroked {
                    Some(prim.color.to_string())
                } else {
                    None
                },
                stroke_width: if stroked { Some(1.5) } else { None },
                stroke_cap: None,
                stroke_join: None,
                stroke_dash: None,
                stroke_dash_offset: None,
            })
        }
        HudPrimitiveKind::Text {
            x,
            y,
            font_size,
            align_center,
        } => PaintInstruction::Text(PaintText {
            base,
            x,
            y,
            text: prim.primitive_label().to_string(),
            font_ref: None,
            font_size,
            fill: Some(prim.color.to_string()),
            text_align: Some(if align_center {
                TextAlign::Center
            } else {
                TextAlign::Left
            }),
        }),
    }
}

// ---------------------------------------------------------------------------
// Zero-Alloc Retained PaintScene Skeleton (`update_paint_scene_into`)
// ---------------------------------------------------------------------------

// Fixed instruction slot indices in the canonical PaintScene skeleton.
const SLOT_BG: usize = 0;
const SLOT_ADI_SKY: usize = 1;
const SLOT_ADI_GROUND: usize = 2;
const SLOT_ADI_FRAME: usize = 3;
const SLOT_ADI_HORIZON: usize = 4;
const SLOT_ADI_PITCH_POS: usize = 5;
const SLOT_ADI_PITCH_NEG: usize = 6;
const SLOT_ADI_BANK_ARC: usize = 7;
const SLOT_ADI_BANK_PTR: usize = 8;
const SLOT_ADI_WATERMARK: usize = 9;
const SLOT_ADI_FPM_CIRCLE: usize = 10;
const SLOT_ADI_FPM_WINGS: usize = 11;
const SLOT_IAS_FRAME: usize = 12;
const SLOT_IAS_TICKS: usize = 13;
const SLOT_IAS_BOX: usize = 14;
const SLOT_IAS_READOUT: usize = 15;
const SLOT_IAS_MACH: usize = 16;
const SLOT_IAS_LBL_BASE: usize = 17; // 5 labels: 17..22
const SLOT_ALT_FRAME: usize = 22;
const SLOT_ALT_TICKS: usize = 23;
const SLOT_ALT_BOX: usize = 24;
const SLOT_ALT_READOUT: usize = 25;
const SLOT_ALT_AGL: usize = 26;
const SLOT_ALT_LBL_BASE: usize = 27; // 5 labels: 27..32
const SLOT_VSI_FRAME: usize = 32;
const SLOT_VSI_BAR: usize = 33;
const SLOT_VSI_READOUT: usize = 34;
const SLOT_HDG_FRAME: usize = 35;
const SLOT_HDG_TICKS: usize = 36;
const SLOT_HDG_LUBBER: usize = 37;
const SLOT_HDG_READOUT: usize = 38;
const SLOT_HDG_LBL_BASE: usize = 39; // 5 labels: 39..44
const SLOT_AOA_READOUT: usize = 44;
const SLOT_GLOAD_READOUT: usize = 45;
const SLOT_ANN_STALL_BOX: usize = 46;
const SLOT_ANN_STALL_TXT: usize = 47;
const SLOT_ANN_OVERSPEED_BOX: usize = 48;
const SLOT_ANN_OVERSPEED_TXT: usize = 49;
const SLOT_ANN_TERRAIN_BOX: usize = 50;
const SLOT_ANN_TERRAIN_TXT: usize = 51;
const SLOT_ANN_PULLUP_BOX: usize = 52;
const SLOT_ANN_PULLUP_TXT: usize = 53;
const SLOT_HUD_SUMMARY_TAG: usize = 54;

const SKELETON_IDS: [&str; SCENE_SKELETON_LEN] = [
    "hud.bg",
    "hud.adi.sky",
    "hud.adi.ground",
    "hud.adi.frame",
    "hud.adi.horizon",
    "hud.adi.pitch.pos",
    "hud.adi.pitch.neg",
    "hud.adi.bank.arc",
    "hud.adi.bank.ptr",
    "hud.adi.watermark",
    "hud.adi.fpm.circle",
    "hud.adi.fpm.wings",
    "hud.ias.frame",
    "hud.ias.ticks",
    "hud.ias.box",
    "hud.ias.readout",
    "hud.ias.mach",
    "hud.ias.label.0",
    "hud.ias.label.1",
    "hud.ias.label.2",
    "hud.ias.label.3",
    "hud.ias.label.4",
    "hud.alt.frame",
    "hud.alt.ticks",
    "hud.alt.box",
    "hud.alt.readout",
    "hud.alt.agl",
    "hud.alt.label.0",
    "hud.alt.label.1",
    "hud.alt.label.2",
    "hud.alt.label.3",
    "hud.alt.label.4",
    "hud.vsi.frame",
    "hud.vsi.bar",
    "hud.vsi.readout",
    "hud.hdg.frame",
    "hud.hdg.ticks",
    "hud.hdg.lubber",
    "hud.hdg.readout",
    "hud.hdg.label.0",
    "hud.hdg.label.1",
    "hud.hdg.label.2",
    "hud.hdg.label.3",
    "hud.hdg.label.4",
    "hud.aoa.readout",
    "hud.gload.readout",
    "hud.ann.stall.box",
    "hud.ann.stall.text",
    "hud.ann.overspeed.box",
    "hud.ann.overspeed.text",
    "hud.ann.terrain.box",
    "hud.ann.terrain.text",
    "hud.ann.pullup.box",
    "hud.ann.pullup.text",
    "hud.mode.tag",
];

fn cap_string(init: &str, cap: usize) -> String {
    let mut s = String::with_capacity(cap.max(init.len()));
    s.push_str(init);
    s
}

fn cap_base(id: &str) -> PaintBase {
    PaintBase {
        id: Some(cap_string(id, STR_CAP_ID)),
        metadata: None,
    }
}

fn make_rect_slot(id: &str) -> PaintInstruction {
    PaintInstruction::Rect(PaintRect {
        base: cap_base(id),
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
        fill: Some(cap_string(COLOR_TRANSPARENT, STR_CAP_COLOR)),
        stroke: Some(cap_string(COLOR_TRANSPARENT, STR_CAP_COLOR)),
        stroke_width: Some(1.5),
        corner_radius: None,
        stroke_dash: None,
        stroke_dash_offset: None,
    })
}

fn make_line_slot(id: &str) -> PaintInstruction {
    PaintInstruction::Line(PaintLine {
        base: cap_base(id),
        x1: 0.0,
        y1: 0.0,
        x2: 0.0,
        y2: 0.0,
        stroke: cap_string(COLOR_TRANSPARENT, STR_CAP_COLOR),
        stroke_width: Some(1.5),
        stroke_cap: None,
        stroke_dash: None,
        stroke_dash_offset: None,
    })
}

fn make_ellipse_slot(id: &str) -> PaintInstruction {
    PaintInstruction::Ellipse(PaintEllipse {
        base: cap_base(id),
        cx: 0.0,
        cy: 0.0,
        rx: 0.0,
        ry: 0.0,
        fill: Some(cap_string(COLOR_TRANSPARENT, STR_CAP_COLOR)),
        stroke: Some(cap_string(COLOR_TRANSPARENT, STR_CAP_COLOR)),
        stroke_width: Some(1.5),
        stroke_dash: None,
        stroke_dash_offset: None,
    })
}

fn make_path_slot(id: &str, cmd_cap: usize, dashed: bool) -> PaintInstruction {
    PaintInstruction::Path(PaintPath {
        base: cap_base(id),
        commands: Vec::with_capacity(cmd_cap),
        fill: Some(cap_string(COLOR_TRANSPARENT, STR_CAP_COLOR)),
        fill_rule: None,
        stroke: Some(cap_string(COLOR_TRANSPARENT, STR_CAP_COLOR)),
        stroke_width: Some(1.5),
        stroke_cap: None,
        stroke_join: None,
        stroke_dash: if dashed { Some(vec![6.0, 4.0]) } else { None },
        stroke_dash_offset: None,
    })
}

fn make_text_slot(id: &str, align: TextAlign) -> PaintInstruction {
    PaintInstruction::Text(PaintText {
        base: cap_base(id),
        x: 0.0,
        y: 0.0,
        text: String::with_capacity(STR_CAP_TEXT),
        font_ref: None,
        font_size: 12.0,
        fill: Some(cap_string(COLOR_TRANSPARENT, STR_CAP_COLOR)),
        text_align: Some(align),
    })
}

/// Pre-allocate a retained [`PaintScene`] skeleton with stable `PaintBase.id`s and
/// sufficient `String` / `PathCommand` capacities so subsequent calls to
/// [`update_paint_scene_into`] execute with **zero heap allocations** (`VER07`).
pub fn prepare_paint_scene(config: &HudConfig) -> Result<PaintScene, HudError> {
    config.hud_config_validate()?;

    let mut instructions = Vec::with_capacity(SCENE_SKELETON_LEN);
    for (idx, &id) in SKELETON_IDS.iter().enumerate() {
        let instr = match idx {
            SLOT_BG
            | SLOT_ADI_FRAME
            | SLOT_IAS_FRAME
            | SLOT_IAS_BOX
            | SLOT_ALT_FRAME
            | SLOT_ALT_BOX
            | SLOT_VSI_FRAME
            | SLOT_HDG_FRAME
            | SLOT_ANN_STALL_BOX
            | SLOT_ANN_OVERSPEED_BOX
            | SLOT_ANN_TERRAIN_BOX
            | SLOT_ANN_PULLUP_BOX => make_rect_slot(id),
            SLOT_ADI_SKY | SLOT_ADI_GROUND => make_path_slot(id, PATH_CMD_CAP_POLY, false),
            SLOT_ADI_HORIZON | SLOT_VSI_BAR | SLOT_HDG_LUBBER => make_line_slot(id),
            SLOT_ADI_PITCH_POS => make_path_slot(id, PATH_CMD_CAP_PITCH, false),
            SLOT_ADI_PITCH_NEG => make_path_slot(id, PATH_CMD_CAP_PITCH, true),
            SLOT_ADI_BANK_ARC | SLOT_ADI_BANK_PTR | SLOT_ADI_WATERMARK | SLOT_ADI_FPM_WINGS => {
                make_path_slot(id, 128, false)
            }
            SLOT_IAS_TICKS | SLOT_ALT_TICKS | SLOT_HDG_TICKS => {
                make_path_slot(id, PATH_CMD_CAP_TICKS, false)
            }
            SLOT_ADI_FPM_CIRCLE => make_ellipse_slot(id),
            SLOT_IAS_READOUT
            | SLOT_IAS_MACH
            | SLOT_ALT_READOUT
            | SLOT_ALT_AGL
            | SLOT_VSI_READOUT
            | SLOT_HDG_READOUT
            | SLOT_HDG_LBL_BASE..=43
            | SLOT_GLOAD_READOUT
            | SLOT_ANN_STALL_TXT
            | SLOT_ANN_OVERSPEED_TXT
            | SLOT_ANN_TERRAIN_TXT
            | SLOT_ANN_PULLUP_TXT
            | SLOT_HUD_SUMMARY_TAG => make_text_slot(id, TextAlign::Center),
            _ => make_text_slot(id, TextAlign::Left),
        };
        instructions.push(instr);
    }

    Ok(PaintScene {
        width: canonical_f64(config.viewport_width),
        height: canonical_f64(config.viewport_height),
        background: cap_string(COLOR_BG, STR_CAP_COLOR),
        instructions,
        id: Some(cap_string("cockpit-hud-paint:retained", STR_CAP_ID)),
        metadata: None,
    })
}

fn check_opt_str_cap(s: &Option<String>, min_cap: usize) -> bool {
    match s {
        Some(inner) => inner.capacity() >= min_cap,
        None => false,
    }
}

fn verify_scene_capacities(scene: &PaintScene) -> Result<(), HudError> {
    if scene.background.capacity() < STR_CAP_COLOR {
        return Err(HudError::BufferTooSmall);
    }
    if scene.instructions.len() != SCENE_SKELETON_LEN {
        return Err(HudError::BufferTooSmall);
    }
    for (idx, instr) in scene.instructions.iter().enumerate() {
        let base_ok = match instr {
            PaintInstruction::Rect(r) => {
                check_opt_str_cap(&r.base.id, STR_CAP_ID)
                    && check_opt_str_cap(&r.fill, STR_CAP_COLOR)
                    && check_opt_str_cap(&r.stroke, STR_CAP_COLOR)
            }
            PaintInstruction::Line(l) => {
                check_opt_str_cap(&l.base.id, STR_CAP_ID) && l.stroke.capacity() >= STR_CAP_COLOR
            }
            PaintInstruction::Ellipse(e) => {
                check_opt_str_cap(&e.base.id, STR_CAP_ID)
                    && check_opt_str_cap(&e.fill, STR_CAP_COLOR)
                    && check_opt_str_cap(&e.stroke, STR_CAP_COLOR)
            }
            PaintInstruction::Path(p) => {
                let min_cmds = match idx {
                    SLOT_ADI_SKY | SLOT_ADI_GROUND => PATH_CMD_CAP_POLY,
                    SLOT_ADI_PITCH_POS | SLOT_ADI_PITCH_NEG => PATH_CMD_CAP_PITCH,
                    SLOT_IAS_TICKS | SLOT_ALT_TICKS | SLOT_HDG_TICKS => PATH_CMD_CAP_TICKS,
                    _ => 64,
                };
                check_opt_str_cap(&p.base.id, STR_CAP_ID)
                    && check_opt_str_cap(&p.fill, STR_CAP_COLOR)
                    && check_opt_str_cap(&p.stroke, STR_CAP_COLOR)
                    && p.commands.capacity() >= min_cmds
            }
            PaintInstruction::Text(t) => {
                check_opt_str_cap(&t.base.id, STR_CAP_ID)
                    && t.text.capacity() >= STR_CAP_TEXT
                    && check_opt_str_cap(&t.fill, STR_CAP_COLOR)
            }
            _ => false,
        };
        if !base_ok {
            return Err(HudError::BufferTooSmall);
        }
    }
    Ok(())
}

#[inline]
fn set_str_in_place(dst: &mut String, val: &str) {
    dst.clear();
    dst.push_str(val);
}

#[inline]
fn set_opt_str_in_place(dst: &mut Option<String>, val: &str) {
    if let Some(s) = dst.as_mut() {
        s.clear();
        s.push_str(val);
    }
}

#[allow(clippy::too_many_arguments)]
fn update_rect_slot(
    instr: &mut PaintInstruction,
    id: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    fill: &str,
    stroke: &str,
) {
    if let PaintInstruction::Rect(r) = instr {
        set_opt_str_in_place(&mut r.base.id, id);
        r.x = canonical_f64(x);
        r.y = canonical_f64(y);
        r.width = canonical_f64(width);
        r.height = canonical_f64(height);
        set_opt_str_in_place(&mut r.fill, fill);
        set_opt_str_in_place(&mut r.stroke, stroke);
    }
}

#[allow(clippy::too_many_arguments)]
fn update_line_slot(
    instr: &mut PaintInstruction,
    id: &str,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    stroke: &str,
    stroke_width: f64,
) {
    if let PaintInstruction::Line(l) = instr {
        set_opt_str_in_place(&mut l.base.id, id);
        l.x1 = canonical_f64(x1);
        l.y1 = canonical_f64(y1);
        l.x2 = canonical_f64(x2);
        l.y2 = canonical_f64(y2);
        set_str_in_place(&mut l.stroke, stroke);
        l.stroke_width = Some(stroke_width);
    }
}

#[allow(clippy::too_many_arguments)]
fn update_ellipse_slot(
    instr: &mut PaintInstruction,
    id: &str,
    cx: f64,
    cy: f64,
    rx: f64,
    ry: f64,
    stroke: &str,
    stroke_width: f64,
) {
    if let PaintInstruction::Ellipse(e) = instr {
        set_opt_str_in_place(&mut e.base.id, id);
        e.cx = canonical_f64(cx);
        e.cy = canonical_f64(cy);
        e.rx = canonical_f64(rx);
        e.ry = canonical_f64(ry);
        set_opt_str_in_place(&mut e.fill, COLOR_TRANSPARENT);
        set_opt_str_in_place(&mut e.stroke, stroke);
        e.stroke_width = Some(stroke_width);
    }
}

fn update_text_slot(
    instr: &mut PaintInstruction,
    id: &str,
    x: f64,
    y: f64,
    text: &str,
    font_size: f64,
    fill: &str,
) {
    if let PaintInstruction::Text(t) = instr {
        set_opt_str_in_place(&mut t.base.id, id);
        t.x = canonical_f64(x);
        t.y = canonical_f64(y);
        set_str_in_place(&mut t.text, text);
        t.font_size = canonical_f64(font_size);
        set_opt_str_in_place(&mut t.fill, fill);
    }
}

fn path_slot_clear_and_style<'a>(
    instr: &'a mut PaintInstruction,
    id: &str,
    fill: &str,
    stroke: &str,
    stroke_width: f64,
) -> Option<&'a mut Vec<PathCommand>> {
    if let PaintInstruction::Path(p) = instr {
        set_opt_str_in_place(&mut p.base.id, id);
        p.commands.clear();
        set_opt_str_in_place(&mut p.fill, fill);
        set_opt_str_in_place(&mut p.stroke, stroke);
        p.stroke_width = Some(stroke_width);
        Some(&mut p.commands)
    } else {
        None
    }
}

fn push_clipped_segment_to_path(
    cmds: &mut Vec<PathCommand>,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    rect: HudRect,
) -> bool {
    if cmds.len() + 2 > cmds.capacity() {
        return false;
    }
    if let Some((cx1, cy1, cx2, cy2)) = clip_line_liang_barsky(x1, y1, x2, y2, rect) {
        cmds.push(PathCommand::MoveTo { x: cx1, y: cy1 });
        cmds.push(PathCommand::LineTo { x: cx2, y: cy2 });
        true
    } else {
        false
    }
}

/// Update a pre-allocated [`PaintScene`] (created by [`prepare_paint_scene`]) in-place
/// with **zero heap allocations** (`VER07`) and **byte-for-byte atomic error preservation**.
pub fn update_paint_scene_into(
    telemetry: &FlightTelemetry,
    config: &HudConfig,
    scene: &mut PaintScene,
) -> Result<HudFrameSummary, HudError> {
    config.hud_config_validate()?;
    telemetry.telemetry_validate()?;
    verify_scene_capacities(scene)?;

    let layout = HudLayout::from_config(config);
    let pitch = wrap_pi(telemetry.pitch_rad);
    let roll = wrap_pi(telemetry.roll_rad);
    let heading = wrap_2pi(telemetry.heading_rad);
    let cos_r = trig::cos(roll);
    let sin_r = trig::sin(roll);
    let tx = cos_r;
    let ty = -sin_r;
    let nx = sin_r;
    let ny = cos_r;

    let stroke_primary = match config.mode {
        HudDisplayMode::HudOverlayOnly => COLOR_HUD_GREEN,
        _ => COLOR_HUD_WHITE,
    };
    let bg_color = match config.mode {
        HudDisplayMode::HudOverlayOnly => COLOR_TRANSPARENT,
        _ => COLOR_BG,
    };
    let panel_fill = match config.mode {
        HudDisplayMode::HudOverlayOnly => COLOR_TRANSPARENT,
        _ => COLOR_PANEL_BG,
    };

    scene.width = canonical_f64(layout.w);
    scene.height = canonical_f64(layout.h);
    set_str_in_place(&mut scene.background, bg_color);

    // 1. Background + ADI sky/ground polygons + frame.
    update_rect_slot(
        &mut scene.instructions[SLOT_BG],
        SKELETON_IDS[SLOT_BG],
        0.0,
        0.0,
        layout.w,
        layout.h,
        bg_color,
        COLOR_TRANSPARENT,
    );

    let d_horizon = pitch * layout.px_per_rad;
    let ox = layout.cx + nx * d_horizon;
    let oy = layout.cy + ny * d_horizon;

    if config.mode != HudDisplayMode::HudOverlayOnly {
        let (sky_vx, sky_vy, sky_n) = clip_rect_half_plane(layout.adi_rect(), ox, oy, nx, ny, true);
        if let Some(cmds) = path_slot_clear_and_style(
            &mut scene.instructions[SLOT_ADI_SKY],
            SKELETON_IDS[SLOT_ADI_SKY],
            if sky_n >= 3 {
                COLOR_SKY
            } else {
                COLOR_TRANSPARENT
            },
            COLOR_TRANSPARENT,
            0.0,
        ) {
            if sky_n >= 3 {
                cmds.push(PathCommand::MoveTo {
                    x: sky_vx[0],
                    y: sky_vy[0],
                });
                for i in 1..sky_n {
                    cmds.push(PathCommand::LineTo {
                        x: sky_vx[i],
                        y: sky_vy[i],
                    });
                }
                cmds.push(PathCommand::Close);
            }
        }

        let (gnd_vx, gnd_vy, gnd_n) =
            clip_rect_half_plane(layout.adi_rect(), ox, oy, nx, ny, false);
        if let Some(cmds) = path_slot_clear_and_style(
            &mut scene.instructions[SLOT_ADI_GROUND],
            SKELETON_IDS[SLOT_ADI_GROUND],
            if gnd_n >= 3 {
                COLOR_GROUND
            } else {
                COLOR_TRANSPARENT
            },
            COLOR_TRANSPARENT,
            0.0,
        ) {
            if gnd_n >= 3 {
                cmds.push(PathCommand::MoveTo {
                    x: gnd_vx[0],
                    y: gnd_vy[0],
                });
                for i in 1..gnd_n {
                    cmds.push(PathCommand::LineTo {
                        x: gnd_vx[i],
                        y: gnd_vy[i],
                    });
                }
                cmds.push(PathCommand::Close);
            }
        }
    } else {
        path_slot_clear_and_style(
            &mut scene.instructions[SLOT_ADI_SKY],
            SKELETON_IDS[SLOT_ADI_SKY],
            COLOR_TRANSPARENT,
            COLOR_TRANSPARENT,
            0.0,
        );
        path_slot_clear_and_style(
            &mut scene.instructions[SLOT_ADI_GROUND],
            SKELETON_IDS[SLOT_ADI_GROUND],
            COLOR_TRANSPARENT,
            COLOR_TRANSPARENT,
            0.0,
        );
    }

    update_rect_slot(
        &mut scene.instructions[SLOT_ADI_FRAME],
        SKELETON_IDS[SLOT_ADI_FRAME],
        layout.adi_x,
        layout.adi_y,
        layout.adi_w,
        layout.adi_h,
        COLOR_TRANSPARENT,
        stroke_primary,
    );

    // 2. Artificial horizon line.
    let span = layout.adi_w + layout.adi_h;
    if let Some((hx1, hy1, hx2, hy2)) = clip_line_liang_barsky(
        ox - tx * span,
        oy - ty * span,
        ox + tx * span,
        oy + ty * span,
        layout.adi_rect(),
    ) {
        update_line_slot(
            &mut scene.instructions[SLOT_ADI_HORIZON],
            SKELETON_IDS[SLOT_ADI_HORIZON],
            hx1,
            hy1,
            hx2,
            hy2,
            stroke_primary,
            2.0,
        );
    } else {
        update_line_slot(
            &mut scene.instructions[SLOT_ADI_HORIZON],
            SKELETON_IDS[SLOT_ADI_HORIZON],
            layout.cx,
            layout.cy,
            layout.cx,
            layout.cy,
            COLOR_TRANSPARENT,
            0.0,
        );
    }

    // 3. Positive and negative pitch ladder rungs into retained PaintPath slots.
    let rung_half_w = layout.adi_w * 0.16;
    let rung_gap = layout.adi_w * 0.05;
    let hook_len = (layout.adi_h * 0.025).max(3.0);
    let max_rungs = config.max_pitch_rungs.min(MAX_PITCH_RUNGS);

    let mut pitch_rungs_emitted = 0usize;
    let (pos_instr, neg_instr) = {
        let (head, tail) = scene.instructions.split_at_mut(SLOT_ADI_PITCH_NEG);
        (&mut head[SLOT_ADI_PITCH_POS], &mut tail[0])
    };
    let pos_cmds = path_slot_clear_and_style(
        pos_instr,
        SKELETON_IDS[SLOT_ADI_PITCH_POS],
        COLOR_TRANSPARENT,
        stroke_primary,
        1.5,
    );
    let neg_cmds = path_slot_clear_and_style(
        neg_instr,
        SKELETON_IDS[SLOT_ADI_PITCH_NEG],
        COLOR_TRANSPARENT,
        stroke_primary,
        1.5,
    );

    if let (Some(pc), Some(nc)) = (pos_cmds, neg_cmds) {
        let mut rung_idx = 0usize;
        let mut deg_step = -90_i32;
        while deg_step <= 90 && rung_idx < max_rungs {
            if deg_step != 0 {
                let rung_rad = trig::radians(deg_step as f64);
                let d_rung = (pitch - rung_rad) * layout.px_per_rad;
                let rx_c = layout.cx + nx * d_rung;
                let ry_c = layout.cy + ny * d_rung;
                let hook_dir = if deg_step > 0 { 1.0 } else { -1.0 };
                let target_cmds: &mut Vec<PathCommand> = if deg_step > 0 { pc } else { nc };

                let l_ok = push_clipped_segment_to_path(
                    target_cmds,
                    rx_c - tx * rung_half_w,
                    ry_c - ty * rung_half_w,
                    rx_c - tx * rung_gap,
                    ry_c - ty * rung_gap,
                    layout.adi_rect(),
                );
                let r_ok = push_clipped_segment_to_path(
                    target_cmds,
                    rx_c + tx * rung_gap,
                    ry_c + ty * rung_gap,
                    rx_c + tx * rung_half_w,
                    ry_c + ty * rung_half_w,
                    layout.adi_rect(),
                );
                if l_ok || r_ok {
                    pitch_rungs_emitted += 1;
                    push_clipped_segment_to_path(
                        target_cmds,
                        rx_c - tx * rung_half_w,
                        ry_c - ty * rung_half_w,
                        rx_c - tx * rung_half_w + nx * hook_len * hook_dir,
                        ry_c - ty * rung_half_w + ny * hook_len * hook_dir,
                        layout.adi_rect(),
                    );
                    push_clipped_segment_to_path(
                        target_cmds,
                        rx_c + tx * rung_half_w,
                        ry_c + ty * rung_half_w,
                        rx_c + tx * rung_half_w + nx * hook_len * hook_dir,
                        ry_c + ty * rung_half_w + ny * hook_len * hook_dir,
                        layout.adi_rect(),
                    );
                }
                rung_idx += 1;
            }
            deg_step += 5;
        }
    }

    // 4. Bank angle arc + ticks + pointer.
    let bank_radius = 0.42 * layout.adi_w.min(layout.adi_h);
    if let Some(cmds) = path_slot_clear_and_style(
        &mut scene.instructions[SLOT_ADI_BANK_ARC],
        SKELETON_IDS[SLOT_ADI_BANK_ARC],
        COLOR_TRANSPARENT,
        stroke_primary,
        1.5,
    ) {
        for i in 0..12 {
            let a0 = trig::radians(-60.0 + (i as f64) * 10.0);
            let a1 = trig::radians(-60.0 + ((i + 1) as f64) * 10.0);
            push_clipped_segment_to_path(
                cmds,
                layout.cx + bank_radius * trig::sin(a0),
                layout.cy - bank_radius * trig::cos(a0),
                layout.cx + bank_radius * trig::sin(a1),
                layout.cy - bank_radius * trig::cos(a1),
                layout.adi_rect(),
            );
        }
        let bank_marks = [
            -60.0, -45.0, -30.0, -20.0, -10.0, 0.0, 10.0, 20.0, 30.0, 45.0, 60.0,
        ];
        for &deg in &bank_marks {
            let a = trig::radians(deg);
            let sa = trig::sin(a);
            let ca = trig::cos(a);
            let tick_len = if deg == 0.0 || deg.abs() == 30.0 || deg.abs() == 60.0 {
                (bank_radius * 0.08).max(4.0)
            } else {
                (bank_radius * 0.05).max(2.5)
            };
            push_clipped_segment_to_path(
                cmds,
                layout.cx + bank_radius * sa,
                layout.cy - bank_radius * ca,
                layout.cx + (bank_radius + tick_len) * sa,
                layout.cy - (bank_radius + tick_len) * ca,
                layout.adi_rect(),
            );
        }
    }

    let ptr_r_out = bank_radius - 2.0;
    let ptr_r_in = (bank_radius - (bank_radius * 0.09).max(4.0)).max(2.0);
    let ptr_tip_x = layout.cx - ptr_r_out * sin_r;
    let ptr_tip_y = layout.cy - ptr_r_out * cos_r;
    let ptr_base_x = layout.cx - ptr_r_in * sin_r;
    let ptr_base_y = layout.cy - ptr_r_in * cos_r;
    let ptr_half = (bank_radius * 0.045).max(2.5);
    if let Some(cmds) = path_slot_clear_and_style(
        &mut scene.instructions[SLOT_ADI_BANK_PTR],
        SKELETON_IDS[SLOT_ADI_BANK_PTR],
        COLOR_HUD_CYAN,
        COLOR_HUD_CYAN,
        1.5,
    ) {
        push_clipped_segment_to_path(
            cmds,
            ptr_tip_x,
            ptr_tip_y,
            ptr_base_x - cos_r * ptr_half,
            ptr_base_y + sin_r * ptr_half,
            layout.adi_rect(),
        );
        push_clipped_segment_to_path(
            cmds,
            ptr_tip_x,
            ptr_tip_y,
            ptr_base_x + cos_r * ptr_half,
            ptr_base_y - sin_r * ptr_half,
            layout.adi_rect(),
        );
    }

    // 5. Aircraft boresight watermark.
    let wm_w = (layout.adi_w * 0.07).max(6.0);
    let wm_dip = (layout.adi_h * 0.025).max(3.0);
    if let Some(cmds) = path_slot_clear_and_style(
        &mut scene.instructions[SLOT_ADI_WATERMARK],
        SKELETON_IDS[SLOT_ADI_WATERMARK],
        COLOR_TRANSPARENT,
        COLOR_WARN_AMBER,
        2.0,
    ) {
        cmds.push(PathCommand::MoveTo {
            x: canonical_f64(layout.cx - wm_w),
            y: canonical_f64(layout.cy),
        });
        cmds.push(PathCommand::LineTo {
            x: canonical_f64(layout.cx - wm_dip),
            y: canonical_f64(layout.cy),
        });
        cmds.push(PathCommand::LineTo {
            x: canonical_f64(layout.cx),
            y: canonical_f64(layout.cy + wm_dip),
        });
        cmds.push(PathCommand::LineTo {
            x: canonical_f64(layout.cx + wm_dip),
            y: canonical_f64(layout.cy),
        });
        cmds.push(PathCommand::LineTo {
            x: canonical_f64(layout.cx + wm_w),
            y: canonical_f64(layout.cy),
        });
    }

    // 6. Flight-Path Marker (FPM).
    let fpm_r = (layout.adi_w.min(layout.adi_h) * 0.032).clamp(3.0, 14.0);
    let pad = fpm_r * 2.2;
    let fpm_dx = wrap_pi(telemetry.flight_path_yaw_rad) * layout.px_per_rad;
    let fpm_dy = -(wrap_pi(telemetry.flight_path_pitch_rad) - pitch) * layout.px_per_rad;
    let fpm_x = (layout.cx + fpm_dx).clamp(layout.adi_x + pad, layout.adi_x + layout.adi_w - pad);
    let fpm_y = (layout.cy + fpm_dy).clamp(layout.adi_y + pad, layout.adi_y + layout.adi_h - pad);
    let fpm_color = if config.mode == HudDisplayMode::PfdOnly {
        COLOR_TRANSPARENT
    } else {
        COLOR_HUD_GREEN
    };
    update_ellipse_slot(
        &mut scene.instructions[SLOT_ADI_FPM_CIRCLE],
        SKELETON_IDS[SLOT_ADI_FPM_CIRCLE],
        fpm_x,
        fpm_y,
        fpm_r,
        fpm_r,
        fpm_color,
        1.8,
    );
    if let Some(cmds) = path_slot_clear_and_style(
        &mut scene.instructions[SLOT_ADI_FPM_WINGS],
        SKELETON_IDS[SLOT_ADI_FPM_WINGS],
        COLOR_TRANSPARENT,
        fpm_color,
        1.8,
    ) {
        if config.mode != HudDisplayMode::PfdOnly {
            push_clipped_segment_to_path(
                cmds,
                fpm_x - fpm_r * 2.0,
                fpm_y,
                fpm_x - fpm_r,
                fpm_y,
                layout.adi_rect(),
            );
            push_clipped_segment_to_path(
                cmds,
                fpm_x + fpm_r,
                fpm_y,
                fpm_x + fpm_r * 2.0,
                fpm_y,
                layout.adi_rect(),
            );
            push_clipped_segment_to_path(
                cmds,
                fpm_x,
                fpm_y - fpm_r,
                fpm_x,
                fpm_y - fpm_r * 1.9,
                layout.adi_rect(),
            );
        }
    }

    // 7. Airspeed tape + readout + Mach + 5 fixed label slots.
    let font_sz = (layout.h * 0.022).clamp(7.0, 22.0);
    let mut tape_ticks_emitted = 0usize;

    update_rect_slot(
        &mut scene.instructions[SLOT_IAS_FRAME],
        SKELETON_IDS[SLOT_IAS_FRAME],
        layout.ias_x,
        layout.ias_y,
        layout.ias_w,
        layout.ias_h,
        panel_fill,
        stroke_primary,
    );

    let ias_val = match config.units {
        HudUnits::Aviation => telemetry.indicated_airspeed_mps * KNOTS_PER_MPS,
        HudUnits::Si => telemetry.indicated_airspeed_mps,
    };
    let ias_px_per_unit = layout.ias_h / 100.0;
    let ias_cy = layout.ias_y + 0.5 * layout.ias_h;
    let ias_base = floor_div_step(round_to_i64(ias_val), 10) * 10;
    let max_tape_ticks = (config.max_tape_ticks / 3).clamp(4, 24);
    let half_tape = (max_tape_ticks / 2) as i64;

    if let Some(cmds) = path_slot_clear_and_style(
        &mut scene.instructions[SLOT_IAS_TICKS],
        SKELETON_IDS[SLOT_IAS_TICKS],
        COLOR_TRANSPARENT,
        stroke_primary,
        1.2,
    ) {
        let mut k = -half_tape;
        while k <= half_tape {
            let tv = ias_base + k * 10;
            if tv >= 0 {
                let ty = ias_cy - ((tv as f64) - ias_val) * ias_px_per_unit;
                if ty >= layout.ias_y + 4.0 && ty <= layout.ias_y + layout.ias_h - 4.0 {
                    let tick_len = if tv % 20 == 0 {
                        layout.ias_w * 0.25
                    } else {
                        layout.ias_w * 0.14
                    };
                    if push_clipped_segment_to_path(
                        cmds,
                        layout.ias_x + layout.ias_w - tick_len,
                        ty,
                        layout.ias_x + layout.ias_w,
                        ty,
                        layout.ias_rect(),
                    ) {
                        tape_ticks_emitted += 1;
                    }
                }
            }
            k += 1;
        }
    }

    let ias_box_h = (layout.ias_h * 0.10).max(14.0);
    update_rect_slot(
        &mut scene.instructions[SLOT_IAS_BOX],
        SKELETON_IDS[SLOT_IAS_BOX],
        layout.ias_x,
        ias_cy - 0.5 * ias_box_h,
        layout.ias_w,
        ias_box_h,
        bg_color,
        stroke_primary,
    );
    let ias_lbl = StackText::format_int(round_to_i64(ias_val));
    update_text_slot(
        &mut scene.instructions[SLOT_IAS_READOUT],
        SKELETON_IDS[SLOT_IAS_READOUT],
        layout.ias_x + 0.5 * layout.ias_w,
        ias_cy + font_sz * 0.35,
        ias_lbl.as_str(),
        font_sz * 1.1,
        stroke_primary,
    );
    let mach_lbl = StackText::format_mach(telemetry.mach);
    update_text_slot(
        &mut scene.instructions[SLOT_IAS_MACH],
        SKELETON_IDS[SLOT_IAS_MACH],
        layout.ias_x + 0.5 * layout.ias_w,
        layout.ias_y + layout.ias_h + font_sz * 1.3,
        mach_lbl.as_str(),
        font_sz,
        COLOR_HUD_CYAN,
    );

    let ias_major_base = floor_div_step(round_to_i64(ias_val), 20) * 20;
    for slot_off in 0..5_usize {
        let k = (slot_off as i64) - 2;
        let tv = ias_major_base + k * 20;
        let ty = ias_cy - ((tv as f64) - ias_val) * ias_px_per_unit;
        let visible = tv >= 0
            && ty >= layout.ias_y + font_sz
            && ty <= layout.ias_y + layout.ias_h - font_sz * 0.5
            && (ty - ias_cy).abs() > font_sz * 1.1;
        let lbl = if visible {
            StackText::format_int(tv)
        } else {
            StackText::new()
        };
        let slot_idx = SLOT_IAS_LBL_BASE + slot_off;
        update_text_slot(
            &mut scene.instructions[slot_idx],
            SKELETON_IDS[slot_idx],
            layout.ias_x + layout.ias_w * 0.12,
            if visible {
                ty + font_sz * 0.35
            } else {
                layout.ias_y
            },
            lbl.as_str(),
            font_sz,
            if visible {
                stroke_primary
            } else {
                COLOR_TRANSPARENT
            },
        );
    }

    // 8. Altimeter tape + readout + AGL + VSI.
    update_rect_slot(
        &mut scene.instructions[SLOT_ALT_FRAME],
        SKELETON_IDS[SLOT_ALT_FRAME],
        layout.alt_x,
        layout.alt_y,
        layout.alt_w,
        layout.alt_h,
        panel_fill,
        stroke_primary,
    );

    let alt_val = match config.units {
        HudUnits::Aviation => telemetry.altitude_msl_m * FEET_PER_METER,
        HudUnits::Si => telemetry.altitude_msl_m,
    };
    let alt_step = match config.units {
        HudUnits::Aviation => 200_i64,
        HudUnits::Si => 50_i64,
    };
    let alt_span = match config.units {
        HudUnits::Aviation => 1200.0,
        HudUnits::Si => 300.0,
    };
    let alt_px_per_unit = layout.alt_h / alt_span;
    let alt_cy = layout.alt_y + 0.5 * layout.alt_h;
    let alt_base = floor_div_step(round_to_i64(alt_val), alt_step) * alt_step;

    if let Some(cmds) = path_slot_clear_and_style(
        &mut scene.instructions[SLOT_ALT_TICKS],
        SKELETON_IDS[SLOT_ALT_TICKS],
        COLOR_TRANSPARENT,
        stroke_primary,
        1.2,
    ) {
        let mut k = -half_tape;
        while k <= half_tape {
            let tv = alt_base + k * alt_step;
            let ty = alt_cy - ((tv as f64) - alt_val) * alt_px_per_unit;
            if ty >= layout.alt_y + 4.0 && ty <= layout.alt_y + layout.alt_h - 4.0 {
                let tick_len = if tv % (alt_step * 2) == 0 {
                    layout.alt_w * 0.25
                } else {
                    layout.alt_w * 0.14
                };
                if push_clipped_segment_to_path(
                    cmds,
                    layout.alt_x,
                    ty,
                    layout.alt_x + tick_len,
                    ty,
                    layout.alt_rect(),
                ) {
                    tape_ticks_emitted += 1;
                }
            }
            k += 1;
        }
    }

    let alt_box_h = (layout.alt_h * 0.10).max(14.0);
    update_rect_slot(
        &mut scene.instructions[SLOT_ALT_BOX],
        SKELETON_IDS[SLOT_ALT_BOX],
        layout.alt_x,
        alt_cy - 0.5 * alt_box_h,
        layout.alt_w,
        alt_box_h,
        bg_color,
        stroke_primary,
    );
    let alt_lbl = StackText::format_int(round_to_i64(alt_val));
    update_text_slot(
        &mut scene.instructions[SLOT_ALT_READOUT],
        SKELETON_IDS[SLOT_ALT_READOUT],
        layout.alt_x + 0.5 * layout.alt_w,
        alt_cy + font_sz * 0.35,
        alt_lbl.as_str(),
        font_sz * 1.1,
        stroke_primary,
    );

    let agl_val = match config.units {
        HudUnits::Aviation => telemetry.radar_altitude_agl_m * FEET_PER_METER,
        HudUnits::Si => telemetry.radar_altitude_agl_m,
    };
    let agl_lbl = StackText::format_prefixed_int("RA ", round_to_i64(agl_val));
    update_text_slot(
        &mut scene.instructions[SLOT_ALT_AGL],
        SKELETON_IDS[SLOT_ALT_AGL],
        layout.alt_x + 0.5 * layout.alt_w,
        layout.alt_y + layout.alt_h + font_sz * 1.3,
        agl_lbl.as_str(),
        font_sz,
        COLOR_HUD_GREEN,
    );

    let alt_major_step = alt_step * 2;
    let alt_major_base = floor_div_step(round_to_i64(alt_val), alt_major_step) * alt_major_step;
    for slot_off in 0..5_usize {
        let k = (slot_off as i64) - 2;
        let tv = alt_major_base + k * alt_major_step;
        let ty = alt_cy - ((tv as f64) - alt_val) * alt_px_per_unit;
        let visible = ty >= layout.alt_y + font_sz
            && ty <= layout.alt_y + layout.alt_h - font_sz * 0.5
            && (ty - alt_cy).abs() > font_sz * 1.1;
        let lbl = if visible {
            StackText::format_int(tv)
        } else {
            StackText::new()
        };
        let slot_idx = SLOT_ALT_LBL_BASE + slot_off;
        update_text_slot(
            &mut scene.instructions[slot_idx],
            SKELETON_IDS[slot_idx],
            layout.alt_x + layout.alt_w * 0.30,
            if visible {
                ty + font_sz * 0.35
            } else {
                layout.alt_y
            },
            lbl.as_str(),
            font_sz,
            if visible {
                stroke_primary
            } else {
                COLOR_TRANSPARENT
            },
        );
    }

    // VSI frame, bar, readout.
    update_rect_slot(
        &mut scene.instructions[SLOT_VSI_FRAME],
        SKELETON_IDS[SLOT_VSI_FRAME],
        layout.vsi_x,
        layout.vsi_y,
        layout.vsi_w,
        layout.vsi_h,
        COLOR_TRANSPARENT,
        stroke_primary,
    );
    let vsi_cy = layout.vsi_y + 0.5 * layout.vsi_h;
    let vsi_norm = (telemetry.vertical_speed_mps / 30.0).clamp(-1.0, 1.0);
    let vsi_target_y = (vsi_cy - vsi_norm * (0.46 * layout.vsi_h))
        .clamp(layout.vsi_y, layout.vsi_y + layout.vsi_h);
    update_line_slot(
        &mut scene.instructions[SLOT_VSI_BAR],
        SKELETON_IDS[SLOT_VSI_BAR],
        layout.vsi_x + 0.5 * layout.vsi_w,
        vsi_cy,
        layout.vsi_x + 0.5 * layout.vsi_w,
        vsi_target_y,
        COLOR_HUD_CYAN,
        3.0,
    );
    let vsi_display = match config.units {
        HudUnits::Aviation => round_to_i64(telemetry.vertical_speed_mps * FEET_PER_METER * 60.0),
        HudUnits::Si => round_to_i64(telemetry.vertical_speed_mps),
    };
    let vsi_lbl = StackText::format_prefixed_int("VS ", vsi_display);
    update_text_slot(
        &mut scene.instructions[SLOT_VSI_READOUT],
        SKELETON_IDS[SLOT_VSI_READOUT],
        layout.vsi_x + 0.5 * layout.vsi_w,
        layout.vsi_y - font_sz * 0.4,
        vsi_lbl.as_str(),
        font_sz * 0.85,
        COLOR_HUD_CYAN,
    );

    // 9. Heading / HSI tape.
    update_rect_slot(
        &mut scene.instructions[SLOT_HDG_FRAME],
        SKELETON_IDS[SLOT_HDG_FRAME],
        layout.hdg_x,
        layout.hdg_y,
        layout.hdg_w,
        layout.hdg_h,
        panel_fill,
        stroke_primary,
    );

    let hdg_deg = canonical_f64(trig::degrees(heading));
    let hdg_px_per_deg = layout.hdg_w / 60.0;
    let hdg_base = floor_div_step(round_to_i64(hdg_deg), 5) * 5;

    if let Some(cmds) = path_slot_clear_and_style(
        &mut scene.instructions[SLOT_HDG_TICKS],
        SKELETON_IDS[SLOT_HDG_TICKS],
        COLOR_TRANSPARENT,
        stroke_primary,
        1.2,
    ) {
        let mut k = -half_tape;
        while k <= half_tape {
            let td = hdg_base + k * 5;
            let tx = layout.cx + ((td as f64) - hdg_deg) * hdg_px_per_deg;
            if tx >= layout.hdg_x + 4.0 && tx <= layout.hdg_x + layout.hdg_w - 4.0 {
                let tick_h = if td % 10 == 0 {
                    layout.hdg_h * 0.28
                } else {
                    layout.hdg_h * 0.16
                };
                if push_clipped_segment_to_path(
                    cmds,
                    tx,
                    layout.hdg_y,
                    tx,
                    layout.hdg_y + tick_h,
                    layout.hdg_rect(),
                ) {
                    tape_ticks_emitted += 1;
                }
            }
            k += 1;
        }
    }

    update_line_slot(
        &mut scene.instructions[SLOT_HDG_LUBBER],
        SKELETON_IDS[SLOT_HDG_LUBBER],
        layout.cx,
        layout.hdg_y,
        layout.cx,
        layout.hdg_y + layout.hdg_h * 0.35,
        COLOR_HUD_CYAN,
        2.0,
    );
    let hdg_lbl = StackText::format_heading_deg(round_to_i64(hdg_deg));
    update_text_slot(
        &mut scene.instructions[SLOT_HDG_READOUT],
        SKELETON_IDS[SLOT_HDG_READOUT],
        layout.cx,
        layout.hdg_y + layout.hdg_h * 0.80,
        hdg_lbl.as_str(),
        font_sz * 1.05,
        COLOR_HUD_CYAN,
    );

    let hdg_major_base = floor_div_step(round_to_i64(hdg_deg), 10) * 10;
    for slot_off in 0..5_usize {
        let k = (slot_off as i64) - 2;
        let td = hdg_major_base + k * 10;
        let tx = layout.cx + ((td as f64) - hdg_deg) * hdg_px_per_deg;
        let visible = tx >= layout.hdg_x + font_sz
            && tx <= layout.hdg_x + layout.hdg_w - font_sz
            && (tx - layout.cx).abs() > font_sz * 1.8;
        let lbl = if visible {
            StackText::format_heading_deg(td)
        } else {
            StackText::new()
        };
        let slot_idx = SLOT_HDG_LBL_BASE + slot_off;
        update_text_slot(
            &mut scene.instructions[slot_idx],
            SKELETON_IDS[slot_idx],
            if visible { tx } else { layout.hdg_x },
            layout.hdg_y + layout.hdg_h * 0.72,
            lbl.as_str(),
            font_sz,
            if visible {
                stroke_primary
            } else {
                COLOR_TRANSPARENT
            },
        );
    }

    // 10. Top bar AoA / G-load / Annunciators + mode tag.
    let top_font = (layout.h * 0.024).clamp(7.0, 24.0);
    let aoa_lbl =
        StackText::format_signed_tenths("AOA ", trig::degrees(wrap_pi(telemetry.aoa_rad)));
    update_text_slot(
        &mut scene.instructions[SLOT_AOA_READOUT],
        SKELETON_IDS[SLOT_AOA_READOUT],
        layout.ias_x,
        layout.top_y + layout.top_h * 0.65,
        aoa_lbl.as_str(),
        top_font,
        stroke_primary,
    );
    let g_lbl = StackText::format_signed_tenths("G ", telemetry.g_load);
    update_text_slot(
        &mut scene.instructions[SLOT_GLOAD_READOUT],
        SKELETON_IDS[SLOT_GLOAD_READOUT],
        layout.alt_x + 0.5 * layout.alt_w,
        layout.top_y + layout.top_h * 0.65,
        g_lbl.as_str(),
        top_font,
        stroke_primary,
    );

    let ann = evaluate_annunciators(telemetry, config);
    let slot_w = (layout.adi_w * 0.23).max(12.0);
    let slot_h = (layout.top_h * 0.72).max(10.0);
    let slot_y = layout.top_y + 0.14 * layout.top_h;
    let start_x = layout.adi_x + 0.02 * layout.adi_w;
    let gap = layout.adi_w * 0.02;

    let ann_slots = [
        (
            ann.stall,
            SLOT_ANN_STALL_BOX,
            SLOT_ANN_STALL_TXT,
            "STALL",
            COLOR_WARN_RED,
        ),
        (
            ann.overspeed,
            SLOT_ANN_OVERSPEED_BOX,
            SLOT_ANN_OVERSPEED_TXT,
            "OVERSPEED",
            COLOR_WARN_RED,
        ),
        (
            ann.terrain,
            SLOT_ANN_TERRAIN_BOX,
            SLOT_ANN_TERRAIN_TXT,
            "TERRAIN",
            COLOR_WARN_AMBER,
        ),
        (
            ann.pull_up,
            SLOT_ANN_PULLUP_BOX,
            SLOT_ANN_PULLUP_TXT,
            "PULL UP",
            COLOR_WARN_RED,
        ),
    ];

    for (idx, &(active, box_slot, txt_slot, label, color)) in ann_slots.iter().enumerate() {
        let sx = start_x + (idx as f64) * (slot_w + gap);
        update_rect_slot(
            &mut scene.instructions[box_slot],
            SKELETON_IDS[box_slot],
            sx,
            slot_y,
            slot_w,
            slot_h,
            if active { color } else { COLOR_TRANSPARENT },
            if active {
                COLOR_HUD_WHITE
            } else {
                COLOR_TRANSPARENT
            },
        );
        update_text_slot(
            &mut scene.instructions[txt_slot],
            SKELETON_IDS[txt_slot],
            sx + 0.5 * slot_w,
            slot_y + 0.68 * slot_h,
            if active { label } else { "" },
            top_font * 0.85,
            if active {
                COLOR_HUD_WHITE
            } else {
                COLOR_TRANSPARENT
            },
        );
    }

    let mode_txt = match config.mode {
        HudDisplayMode::CombinedPfdHud => "PFD+HUD",
        HudDisplayMode::PfdOnly => "PFD",
        HudDisplayMode::HudOverlayOnly => "HUD",
    };
    update_text_slot(
        &mut scene.instructions[SLOT_HUD_SUMMARY_TAG],
        SKELETON_IDS[SLOT_HUD_SUMMARY_TAG],
        layout.cx,
        layout.h - font_sz * 0.35,
        mode_txt,
        font_sz * 0.8,
        COLOR_HUD_CYAN,
    );

    let digest = paint_scene_digest_fnv1a(scene)?;
    Ok(HudFrameSummary {
        primitive_count: scene.instructions.len(),
        pitch_rungs_emitted,
        tape_ticks_emitted,
        annunciators: ann,
        frame_digest_fnv1a: digest,
    })
}

/// Convenience allocating wrapper returning a fresh retained [`PaintScene`] for one frame.
pub fn render_hud_scene(
    telemetry: &FlightTelemetry,
    config: &HudConfig,
) -> Result<PaintScene, HudError> {
    config.hud_config_validate()?;
    telemetry.telemetry_validate()?;
    let mut scene = prepare_paint_scene(config)?;
    let _summary = update_paint_scene_into(telemetry, config, &mut scene)?;
    Ok(scene)
}

// ---------------------------------------------------------------------------
// PaintScene Validation & Deterministic Canonical Digest
// ---------------------------------------------------------------------------

/// Verify that every coordinate and scalar in a [`PaintScene`] is strictly finite.
pub fn verify_paint_scene_finite(scene: &PaintScene) -> Result<(), HudError> {
    check_finite(scene.width)?;
    check_finite(scene.height)?;
    for instr in &scene.instructions {
        verify_instruction_finite(instr)?;
    }
    Ok(())
}

fn verify_instruction_finite(instr: &PaintInstruction) -> Result<(), HudError> {
    match instr {
        PaintInstruction::Rect(r) => {
            check_finite(r.x)?;
            check_finite(r.y)?;
            check_finite(r.width)?;
            check_finite(r.height)?;
            if let Some(w) = r.stroke_width {
                check_finite(w)?;
            }
            if let Some(cr) = r.corner_radius {
                check_finite(cr)?;
            }
        }
        PaintInstruction::Line(l) => {
            check_finite(l.x1)?;
            check_finite(l.y1)?;
            check_finite(l.x2)?;
            check_finite(l.y2)?;
            if let Some(w) = l.stroke_width {
                check_finite(w)?;
            }
        }
        PaintInstruction::Ellipse(e) => {
            check_finite(e.cx)?;
            check_finite(e.cy)?;
            check_finite(e.rx)?;
            check_finite(e.ry)?;
            if let Some(w) = e.stroke_width {
                check_finite(w)?;
            }
        }
        PaintInstruction::Path(p) => {
            if let Some(w) = p.stroke_width {
                check_finite(w)?;
            }
            for cmd in &p.commands {
                match *cmd {
                    PathCommand::MoveTo { x, y } | PathCommand::LineTo { x, y } => {
                        check_finite(x)?;
                        check_finite(y)?;
                    }
                    PathCommand::QuadTo { cx, cy, x, y } => {
                        check_finite(cx)?;
                        check_finite(cy)?;
                        check_finite(x)?;
                        check_finite(y)?;
                    }
                    PathCommand::CubicTo {
                        cx1,
                        cy1,
                        cx2,
                        cy2,
                        x,
                        y,
                    } => {
                        check_finite(cx1)?;
                        check_finite(cy1)?;
                        check_finite(cx2)?;
                        check_finite(cy2)?;
                        check_finite(x)?;
                        check_finite(y)?;
                    }
                    PathCommand::ArcTo {
                        rx,
                        ry,
                        x_rotation,
                        x,
                        y,
                        ..
                    } => {
                        check_finite(rx)?;
                        check_finite(ry)?;
                        check_finite(x_rotation)?;
                        check_finite(x)?;
                        check_finite(y)?;
                    }
                    PathCommand::Close => {}
                }
            }
        }
        PaintInstruction::Text(t) => {
            check_finite(t.x)?;
            check_finite(t.y)?;
            check_finite(t.font_size)?;
        }
        PaintInstruction::Clip(c) => {
            check_finite(c.x)?;
            check_finite(c.y)?;
            check_finite(c.width)?;
            check_finite(c.height)?;
            for child in &c.children {
                verify_instruction_finite(child)?;
            }
        }
        PaintInstruction::Group(g) => {
            for child in &g.children {
                verify_instruction_finite(child)?;
            }
        }
        PaintInstruction::Layer(l) => {
            for child in &l.children {
                verify_instruction_finite(child)?;
            }
        }
        PaintInstruction::GlyphRun(gr) => {
            check_finite(gr.font_size)?;
            for g in &gr.glyphs {
                check_finite(g.x)?;
                check_finite(g.y)?;
            }
        }
        PaintInstruction::Gradient(_) | PaintInstruction::Image(_) => {}
    }
    Ok(())
}

/// Compute a deterministic 64-bit FNV-1a digest of a [`PaintScene`],
/// canonicalizing `-0.0` to `+0.0` and any `NaN` to a single quiet NaN bit pattern.
pub fn paint_scene_digest_fnv1a(scene: &PaintScene) -> Result<u64, HudError> {
    let mut hash = FNV1A_OFFSET;
    fnv1a_f64(&mut hash, scene.width);
    fnv1a_f64(&mut hash, scene.height);
    fnv1a_str(&mut hash, &scene.background);
    if let Some(id) = &scene.id {
        fnv1a_u64(&mut hash, 1);
        fnv1a_str(&mut hash, id);
    } else {
        fnv1a_u64(&mut hash, 0);
    }
    fnv1a_u64(&mut hash, scene.instructions.len() as u64);
    for instr in &scene.instructions {
        hash_paint_instruction(&mut hash, instr);
    }
    Ok(hash)
}

fn hash_opt_str(hash: &mut u64, s: &Option<String>) {
    match s {
        Some(v) => {
            fnv1a_u64(hash, 1);
            fnv1a_str(hash, v);
        }
        None => fnv1a_u64(hash, 0),
    }
}

fn hash_paint_instruction(hash: &mut u64, instr: &PaintInstruction) {
    match instr {
        PaintInstruction::Rect(r) => {
            fnv1a_u64(hash, 1);
            hash_opt_str(hash, &r.base.id);
            fnv1a_f64(hash, r.x);
            fnv1a_f64(hash, r.y);
            fnv1a_f64(hash, r.width);
            fnv1a_f64(hash, r.height);
            hash_opt_str(hash, &r.fill);
            hash_opt_str(hash, &r.stroke);
            fnv1a_f64(hash, r.stroke_width.unwrap_or(0.0));
        }
        PaintInstruction::Line(l) => {
            fnv1a_u64(hash, 2);
            hash_opt_str(hash, &l.base.id);
            fnv1a_f64(hash, l.x1);
            fnv1a_f64(hash, l.y1);
            fnv1a_f64(hash, l.x2);
            fnv1a_f64(hash, l.y2);
            fnv1a_str(hash, &l.stroke);
            fnv1a_f64(hash, l.stroke_width.unwrap_or(0.0));
        }
        PaintInstruction::Ellipse(e) => {
            fnv1a_u64(hash, 3);
            hash_opt_str(hash, &e.base.id);
            fnv1a_f64(hash, e.cx);
            fnv1a_f64(hash, e.cy);
            fnv1a_f64(hash, e.rx);
            fnv1a_f64(hash, e.ry);
            hash_opt_str(hash, &e.fill);
            hash_opt_str(hash, &e.stroke);
            fnv1a_f64(hash, e.stroke_width.unwrap_or(0.0));
        }
        PaintInstruction::Path(p) => {
            fnv1a_u64(hash, 4);
            hash_opt_str(hash, &p.base.id);
            hash_opt_str(hash, &p.fill);
            hash_opt_str(hash, &p.stroke);
            fnv1a_f64(hash, p.stroke_width.unwrap_or(0.0));
            fnv1a_u64(hash, p.commands.len() as u64);
            for cmd in &p.commands {
                match *cmd {
                    PathCommand::MoveTo { x, y } => {
                        fnv1a_u64(hash, 1);
                        fnv1a_f64(hash, x);
                        fnv1a_f64(hash, y);
                    }
                    PathCommand::LineTo { x, y } => {
                        fnv1a_u64(hash, 2);
                        fnv1a_f64(hash, x);
                        fnv1a_f64(hash, y);
                    }
                    PathCommand::QuadTo { cx, cy, x, y } => {
                        fnv1a_u64(hash, 3);
                        fnv1a_f64(hash, cx);
                        fnv1a_f64(hash, cy);
                        fnv1a_f64(hash, x);
                        fnv1a_f64(hash, y);
                    }
                    PathCommand::CubicTo {
                        cx1,
                        cy1,
                        cx2,
                        cy2,
                        x,
                        y,
                    } => {
                        fnv1a_u64(hash, 4);
                        fnv1a_f64(hash, cx1);
                        fnv1a_f64(hash, cy1);
                        fnv1a_f64(hash, cx2);
                        fnv1a_f64(hash, cy2);
                        fnv1a_f64(hash, x);
                        fnv1a_f64(hash, y);
                    }
                    PathCommand::ArcTo {
                        rx,
                        ry,
                        x_rotation,
                        large_arc,
                        sweep,
                        x,
                        y,
                    } => {
                        fnv1a_u64(hash, 5);
                        fnv1a_f64(hash, rx);
                        fnv1a_f64(hash, ry);
                        fnv1a_f64(hash, x_rotation);
                        fnv1a_u64(hash, large_arc as u64);
                        fnv1a_u64(hash, sweep as u64);
                        fnv1a_f64(hash, x);
                        fnv1a_f64(hash, y);
                    }
                    PathCommand::Close => {
                        fnv1a_u64(hash, 6);
                    }
                }
            }
        }
        PaintInstruction::Text(t) => {
            fnv1a_u64(hash, 5);
            hash_opt_str(hash, &t.base.id);
            fnv1a_f64(hash, t.x);
            fnv1a_f64(hash, t.y);
            fnv1a_str(hash, &t.text);
            fnv1a_f64(hash, t.font_size);
            hash_opt_str(hash, &t.fill);
        }
        PaintInstruction::Clip(c) => {
            fnv1a_u64(hash, 6);
            fnv1a_f64(hash, c.x);
            fnv1a_f64(hash, c.y);
            fnv1a_f64(hash, c.width);
            fnv1a_f64(hash, c.height);
            for child in &c.children {
                hash_paint_instruction(hash, child);
            }
        }
        PaintInstruction::Group(g) => {
            fnv1a_u64(hash, 7);
            for child in &g.children {
                hash_paint_instruction(hash, child);
            }
        }
        PaintInstruction::Layer(l) => {
            fnv1a_u64(hash, 8);
            for child in &l.children {
                hash_paint_instruction(hash, child);
            }
        }
        _ => {
            fnv1a_u64(hash, 9);
        }
    }
}
