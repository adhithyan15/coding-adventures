//! # game-flight-sim (`GAME45`)
//!
//! Pure Tier 1 deterministic 6-DOF flight simulator uniting:
//! - [`rigidbody_6dof`] (`GAME40`) — 6-DOF rigid-body integration and ground contact
//! - [`aero_flight_model`] (`GAME41`) — ISA 1976 atmosphere, polar aerodynamics, and trim
//! - [`terrain_lod`] (`GAME42`) — quadtree LOD terrain mesh and runway heightfield
//! - [`cockpit_hud_paint`] (`GAME43`) — zero-alloc PFD/HUD vector instrumentation
//! - [`render3d_gpu_bridge`] (`GAME44`) — hardware 3D bridge with deterministic CPU fallback
//! - [`game_clock`] (`GAME10`) — deterministic fixed-timestep clock accumulator
//! - [`rng`] — explicit seeded [`rng::Pcg32`] turbulence generator
//!
//! ## Architecture
//!
//! ```text
//!  FlightSimState (6-DOF + Aircraft + FixedClock + Pcg32 + Fuel + FlightPhase)
//!         │
//!         ├─► step_in_place(&mut state, &heightfield, &runway, &input, Option<&mut BlackBoxRecorder>)
//!         │       ├── 1. Upfront atomic validation (state, input, terrain, clock, FDR capacity)
//!         │       ├── 2. Terrain-coupled ground contact / 6-DOF aero substeps + fuel/flameout
//!         │       └── 3. Commit state + deterministic Pcg32 gust + BlackBoxFrame (0 alloc)
//!         │
//!         ├─► encode_black_box_into / decode_black_box_from_bytes / replay_black_box_verify
//!         │       └── Hardened GFDR binary codec + bit-exact state_digest_fnv1a replay
//!         │
//!         └─► render_frame_into(&state, &heightfield, &lod, &hud_cfg, &target, pref, cam,
//!                               &mut fb, depth, &mut FlightRenderScratch, Option<&mut PaintScene>)
//!                 ├── 1. Upfront atomic validation (byte-for-byte untouched on Err)
//!                 ├── 2. terrain_lod::refine_into + emit_mesh_into -> render3d_gpu_bridge (0 alloc)
//!                 └── 3. cockpit_hud_paint::build_hud_frame_into + update_paint_scene_into (0 alloc)
//! ```
//!
//! ## Safety & Determinism Guarantees
//!
//! - `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]`
//! - `capabilities: []` (`PureCapabilities`, no `window-core`, no driver probing)
//! - Byte-for-byte atomic error preservation on both `step_in_place` and `render_frame_into`
//! - Zero heap allocation steady-state hot path (`VER07`)

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::fmt;

use aero_flight_model::{
    isa_atmosphere, AeroError, AirData, AircraftModel, ControlInputs, FlightEnvironment, IsaSample,
    ISA_GRAVITY_M_S2,
};
use cockpit_hud_paint::{
    AnnunciatorFlags, FlightTelemetry, HudConfig, HudError, MAX_G_LOAD, MAX_RADAR_ALTITUDE_M,
    MAX_VERTICAL_SPEED_MPS,
};
use game_clock::{advance_frame, ClockError, GameClockConfig, GameClockState};
use math3d::{Math3dError, Plane, Quat, Vec3};
use render3d_gpu_bridge::{BridgeError, RasterError};
use rigidbody_6dof::{
    GroundContactParams, IntegratorKind, MassProperties, RigidBodyError, RigidBodyState,
};
use rng::Pcg32;
use terrain_lod::{Heightfield, RunwaySpec, TerrainError};

/// Generated capability verification witnesses.
pub mod capabilities;
mod fdr;
mod render;
mod validate;

pub use capabilities::PureCapabilities;
pub use fdr::{
    decode_black_box_from_bytes, decode_black_box_into, encode_black_box_into,
    replay_black_box_verify, BlackBoxFrame, BlackBoxHeader, BlackBoxRecorder, ReplayVerifyReport,
};
pub use render::{
    build_flight_camera, render_frame_into, telemetry_from_sim_state, FlightFrameReport,
    FlightRenderScratch,
};
pub use validate::{
    is_point_on_runway, validate_altitude_envelope_m, validate_dt_ns, validate_fdr_blob_bytes,
    validate_fdr_frame_count, validate_fuel_mg, validate_input_frame, validate_render_dim,
    validate_sim_config,
};

use render::compute_annunciators;
use validate::{
    canon_zero, check_finite, check_range, check_vec3_bounded, fnv1a_mix_bytes, fnv1a_mix_f64,
    fnv1a_mix_u64, fnv1a_mix_vec3, validate_terrain_and_runway, FNV_OFFSET,
};

// ---------------------------------------------------------------------------
// Normative Constants (aligned with code/specs/game-flight-sim.warrant)
// ---------------------------------------------------------------------------

/// Maximum number of frames storable in a [`BlackBoxRecorder`] (CWE-400).
pub const MAX_BLACK_BOX_FRAMES: usize = 65_536;

/// Four-byte magic prefix for serialized black-box FDR blobs (`GFDR`).
pub const BLACK_BOX_MAGIC: [u8; 4] = *b"GFDR";

/// Binary wire format version for black-box FDR blobs.
pub const BLACK_BOX_VERSION: u32 = 1;

/// Fixed serialized byte length of [`BlackBoxHeader`] + frame count prefix.
pub const BLACK_BOX_HEADER_BYTES: usize = 80;

/// Fixed serialized byte length of a single [`BlackBoxFrame`].
pub const BLACK_BOX_FRAME_BYTES: usize = 152;

/// Fixed serialized byte length of the trailing FNV-1a checksum.
pub const BLACK_BOX_TRAILER_BYTES: usize = 8;

/// Maximum serialized FDR blob size in bytes (`88 + 65_536 * 152`).
pub const MAX_BLACK_BOX_BYTES: usize =
    BLACK_BOX_HEADER_BYTES + MAX_BLACK_BOX_FRAMES * BLACK_BOX_FRAME_BYTES + BLACK_BOX_TRAILER_BYTES;

/// Minimum accepted frame timestep delta in nanoseconds (`1 ns`).
pub const MIN_STEP_DT_NS: u64 = 1;

/// Maximum accepted frame timestep delta in nanoseconds (`1 s`).
pub const MAX_STEP_DT_NS: u64 = 1_000_000_000;

/// Minimum ISA world altitude envelope in meters.
pub const MIN_ALTITUDE_ENVELOPE_M: f64 = aero_flight_model::MIN_ALTITUDE_M;

/// Maximum ISA world altitude envelope in meters.
pub const MAX_ALTITUDE_ENVELOPE_M: f64 = aero_flight_model::MAX_ALTITUDE_M;

/// Soft service ceiling enforced on climbing aircraft inside ISA bounds (meters).
pub const MAX_CEILING_M: f64 = 18_500.0;

/// Maximum horizontal world coordinate extent along `X` and `Z` (meters).
pub const MAX_WORLD_EXTENT_M: f64 = 50_000.0;

/// Maximum fuel load in kilograms.
pub const MAX_FUEL_KG: f64 = 100_000.0;

/// Maximum turbulence gust amplitude in meters per second.
pub const MAX_GUST_MPS: f64 = 50.0;

/// Tricycle landing gear vertical offset below aircraft CG (meters).
pub const GEAR_HEIGHT_M: f64 = 1.25;

/// Maximum vertical sink rate permitted for safe runway touchdown (`m/s`).
pub const CRASH_SINK_RATE_MPS: f64 = 6.0;

/// Minimum pitch angle permitted for safe touchdown (radians).
pub const SAFE_LANDING_PITCH_MIN_RAD: f64 = -0.20;

/// Maximum pitch angle permitted for safe touchdown (radians).
pub const SAFE_LANDING_PITCH_MAX_RAD: f64 = 0.45;

/// Maximum absolute bank angle permitted for safe touchdown (radians).
pub const SAFE_LANDING_ROLL_MAX_RAD: f64 = 0.35;

/// Idle fuel burn rate in `kg/s`.
const IDLE_BURN_KG_S: f64 = 0.0025;

/// Additional full-throttle fuel burn rate in `kg/s`.
const FULL_BURN_KG_S: f64 = 0.015;

// ---------------------------------------------------------------------------
// Error & Enum Types
// ---------------------------------------------------------------------------

/// Typed failure modes returned by `game-flight-sim`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlightSimError {
    /// A floating-point scalar or vector was `NaN` or `±Inf` (CWE-20).
    NonFiniteInput,
    /// A scalar, control command, altitude, or coordinate exceeded its safe domain (CWE-20).
    ValueOutOfBounds,
    /// A timestep delta (`0` or `> MAX_STEP_DT_NS`) or clock state was invalid (CWE-20 / CWE-400).
    InvalidClockDelta,
    /// Framebuffer, depth buffer, or terrain dimensions were invalid or mismatched (CWE-125 / CWE-787).
    InvalidDimensions,
    /// A caller-provided output slice or scratch buffer lacked required capacity (CWE-125 / CWE-787).
    BufferTooSmall,
    /// A vertex, index, triangle, node, or primitive budget exceeded safe bounds (CWE-400).
    BudgetExceeded,
    /// The black-box FDR capacity limit was reached (`> MAX_BLACK_BOX_FRAMES` or full).
    BlackBoxFull,
    /// A serialized FDR blob failed magic, version, length, checksum, or domain checks (CWE-502).
    BlackBoxCorrupt,
    /// Deterministic FDR replay diverged from recorded tick digest or telemetry.
    ReplayMismatch,
    /// A hardware-only rendering backend was requested in pure Tier 1 core.
    BackendUnavailable,
    /// Underlying `rigidbody-6dof` failure.
    RigidBody(RigidBodyError),
    /// Underlying `aero-flight-model` failure.
    Aero(AeroError),
    /// Underlying `terrain-lod` failure.
    Terrain(TerrainError),
    /// Underlying `cockpit-hud-paint` failure.
    Hud(HudError),
    /// Underlying `render3d-gpu-bridge` failure.
    Bridge(BridgeError),
    /// Underlying `game-clock` failure.
    Clock(ClockError),
    /// Underlying `math3d` failure.
    Math(Math3dError),
}

impl From<RigidBodyError> for FlightSimError {
    fn from(e: RigidBodyError) -> Self {
        match e {
            RigidBodyError::NonFiniteInput => Self::NonFiniteInput,
            other => Self::RigidBody(other),
        }
    }
}

impl From<AeroError> for FlightSimError {
    fn from(e: AeroError) -> Self {
        match e {
            AeroError::NonFiniteInput => Self::NonFiniteInput,
            other => Self::Aero(other),
        }
    }
}

impl From<TerrainError> for FlightSimError {
    fn from(e: TerrainError) -> Self {
        match e {
            TerrainError::NonFiniteInput => Self::NonFiniteInput,
            TerrainError::BudgetExceeded => Self::BudgetExceeded,
            other => Self::Terrain(other),
        }
    }
}

impl From<RasterError> for FlightSimError {
    fn from(e: RasterError) -> Self {
        Self::from(BridgeError::from(e))
    }
}

impl From<HudError> for FlightSimError {
    fn from(e: HudError) -> Self {
        match e {
            HudError::NonFiniteInput => Self::NonFiniteInput,
            HudError::BufferTooSmall => Self::BufferTooSmall,
            HudError::PrimitiveLimitExceeded => Self::BudgetExceeded,
            other => Self::Hud(other),
        }
    }
}

impl From<BridgeError> for FlightSimError {
    fn from(e: BridgeError) -> Self {
        match e {
            BridgeError::NonFiniteInput => Self::NonFiniteInput,
            BridgeError::InvalidDimensions => Self::InvalidDimensions,
            BridgeError::BudgetExceeded => Self::BudgetExceeded,
            BridgeError::RequestedBackendUnavailable(_) => Self::BackendUnavailable,
            other => Self::Bridge(other),
        }
    }
}

impl From<ClockError> for FlightSimError {
    fn from(e: ClockError) -> Self {
        match e {
            ClockError::TimestampOverflow | ClockError::CorruptedStateInvariant => {
                Self::InvalidClockDelta
            }
            other => Self::Clock(other),
        }
    }
}

impl From<Math3dError> for FlightSimError {
    fn from(e: Math3dError) -> Self {
        match e {
            Math3dError::NonFiniteInput => Self::NonFiniteInput,
            other => Self::Math(other),
        }
    }
}

/// Explicit flight lifecycle phases of the 6-DOF simulator state machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FlightPhase {
    /// Parked stationary on the runway prior to takeoff roll.
    OnRunway = 0,
    /// Accelerating on the runway with landing gear on the ground.
    TakeoffRoll = 1,
    /// In flight clear of the ground envelope.
    Airborne = 2,
    /// Descending or configured with gear extended below 150 m AGL.
    Approach = 3,
    /// Safely touched down and rolled out on a runway.
    Landed = 4,
    /// Terminal crash state caused by gear-up impact, excessive sink rate, or off-runway crash.
    Crashed = 5,
}

impl FlightPhase {
    /// Convert [`FlightPhase`] to its canonical wire byte tag (`0..=5`).
    #[inline]
    pub const fn phase_to_u8(self) -> u8 {
        self as u8
    }

    /// Decode a wire byte tag into [`FlightPhase`], rejecting unknown tags.
    #[inline]
    pub const fn phase_from_u8(tag: u8) -> Result<Self, FlightSimError> {
        match tag {
            0 => Ok(Self::OnRunway),
            1 => Ok(Self::TakeoffRoll),
            2 => Ok(Self::Airborne),
            3 => Ok(Self::Approach),
            4 => Ok(Self::Landed),
            5 => Ok(Self::Crashed),
            _ => Err(FlightSimError::BlackBoxCorrupt),
        }
    }
}

/// Initial scenario mode recorded in [`BlackBoxHeader`] for deterministic replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum InitialFlightMode {
    /// Initialized parked on a runway threshold.
    OnRunway = 0,
    /// Initialized trimmed in steady level airborne flight.
    LevelTrim = 1,
}

impl InitialFlightMode {
    /// Convert [`InitialFlightMode`] to its canonical wire byte tag (`0..=1`).
    #[inline]
    pub const fn initial_mode_to_u8(self) -> u8 {
        self as u8
    }

    /// Decode a wire byte tag into [`InitialFlightMode`].
    #[inline]
    pub const fn initial_mode_from_u8(tag: u8) -> Result<Self, FlightSimError> {
        match tag {
            0 => Ok(Self::OnRunway),
            1 => Ok(Self::LevelTrim),
            _ => Err(FlightSimError::BlackBoxCorrupt),
        }
    }
}

/// Built-in aircraft preset selector recorded in [`BlackBoxHeader`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AircraftPresetKind {
    /// General-aviation trainer (`aero_flight_model::AircraftModel::aircraft_preset_trainer`).
    Trainer = 0,
}

impl AircraftPresetKind {
    /// Convert [`AircraftPresetKind`] to its canonical wire byte tag.
    #[inline]
    pub const fn preset_to_u8(self) -> u8 {
        self as u8
    }

    /// Decode a wire byte tag into [`AircraftPresetKind`].
    #[inline]
    pub const fn preset_from_u8(tag: u8) -> Result<Self, FlightSimError> {
        match tag {
            0 => Ok(Self::Trainer),
            _ => Err(FlightSimError::BlackBoxCorrupt),
        }
    }
}

/// Camera viewpoint mode for [`render_frame_into`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraMode {
    /// First-person cockpit forward view.
    Cockpit,
    /// Third-person trailing chase camera.
    Chase,
}

// ---------------------------------------------------------------------------
// Input, Config, State, and Report Structs
// ---------------------------------------------------------------------------

/// Pilot control and frame-timing input for one simulation step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlightInputFrame {
    /// Frame elapsed delta in nanoseconds (`1..=MAX_STEP_DT_NS`).
    pub dt_ns: u64,
    /// Normalized elevator command in `[-1.0, 1.0]` (`+1.0` pulls nose up).
    pub elevator: f64,
    /// Normalized aileron command in `[-1.0, 1.0]`.
    pub aileron: f64,
    /// Normalized rudder command in `[-1.0, 1.0]`.
    pub rudder: f64,
    /// Throttle lever in `[0.0, 1.0]`.
    pub throttle: f64,
    /// Flaps deployment in `[0.0, 1.0]`.
    pub flaps: f64,
    /// Wheel brake application in `[0.0, 1.0]`.
    pub brake: f64,
    /// Landing gear selector (`true` = down, `false` = retracted).
    pub gear_down: bool,
}

impl FlightInputFrame {
    /// Validate that all fields are finite and within control ranges.
    pub fn input_validate(&self) -> Result<(), FlightSimError> {
        let _ = validate_input_frame(self)?;
        Ok(())
    }
}

/// Configuration for initializing a [`FlightSimState`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlightSimConfig {
    /// Deterministic PRNG seed for [`rng::Pcg32`] turbulence gusts.
    pub rng_seed: u64,
    /// Terrain heightfield seed recorded in [`BlackBoxHeader`].
    pub terrain_seed: u64,
    /// Aircraft preset identifier.
    pub aircraft_preset: AircraftPresetKind,
    /// Fixed simulation substep in nanoseconds (e.g. `16_666_667` for 60 Hz).
    pub fixed_dt_ns: u64,
    /// Maximum substeps executed per frame (`game_clock` spiral-of-death cap).
    pub max_frame_steps: u32,
    /// Initial fuel load in kilograms (`0.0..=MAX_FUEL_KG`).
    pub initial_fuel_kg: f64,
    /// Maximum random turbulence gust amplitude in `m/s` (`0.0..=MAX_GUST_MPS`).
    pub gust_intensity_mps: f64,
}

impl FlightSimConfig {
    /// Default 60 Hz trainer configuration with 120 kg fuel and calm gusts.
    pub const fn sim_config_default(rng_seed: u64, terrain_seed: u64) -> Self {
        Self {
            rng_seed,
            terrain_seed,
            aircraft_preset: AircraftPresetKind::Trainer,
            fixed_dt_ns: game_clock::DEFAULT_60HZ_DT_NS,
            max_frame_steps: game_clock::DEFAULT_MAX_FRAME_STEPS,
            initial_fuel_kg: 120.0,
            gust_intensity_mps: 0.0,
        }
    }

    /// Validate all configuration bounds.
    pub fn sim_config_validate(&self) -> Result<(), FlightSimError> {
        let _ = validate_sim_config(self)?;
        Ok(())
    }
}

/// Complete explicit 6-DOF flight simulator state (`VER04m` `state=explicit`).
pub struct FlightSimState {
    /// Rigid-body kinematics (`position`, `linear_momentum`, `orientation`, `angular_momentum`).
    pub body: RigidBodyState,
    /// Aircraft mass and principal inertia tensor.
    pub mass: MassProperties,
    /// Aerodynamic polar and stability derivative model.
    pub aircraft: AircraftModel,
    /// Atmospheric wind, gravity, and world datum elevation.
    pub env: FlightEnvironment,
    /// Active physical control deflections and effective throttle.
    pub controls: ControlInputs,
    /// Ground contact penalty/impulse configuration.
    pub ground_contact: GroundContactParams,
    /// Rigid-body numerical integrator selection (`Rk4` / `SemiImplicitEuler`).
    pub integrator: IntegratorKind,
    /// Deterministic fixed-step simulation clock.
    pub clock: GameClockState,
    /// Explicit seeded [`rng::Pcg32`] state for deterministic gusts/turbulence.
    pub rng: Pcg32,
    /// Initial seed used to construct [`Self::rng`].
    pub rng_seed: u64,
    /// Exact count of `next_u64` draws performed on [`Self::rng`].
    pub rng_steps: u64,
    /// Gust intensity amplitude in `m/s`.
    pub gust_intensity_mps: f64,
    /// Deterministic gust vector applied on the current step (`m/s`).
    pub last_gust_world_mps: Vec3,
    /// Monotonic frame tick counter.
    pub tick: u64,
    /// Total simulated physics time in seconds.
    pub sim_time_s: f64,
    /// Remaining fuel mass in kilograms (monotonic non-negative; `0.0` triggers flameout).
    pub fuel_kg: f64,
    /// Landing gear status (`true` = extended).
    pub gear_down: bool,
    /// Normalized flap extension in `[0.0, 1.0]`.
    pub flaps_norm: f64,
    /// Normalized wheel brake application in `[0.0, 1.0]`.
    pub brake_norm: f64,
    /// Current flight lifecycle phase.
    pub phase: FlightPhase,
    /// Most recent ISA atmospheric sample.
    pub last_isa: IsaSample,
    /// Most recent air-data evaluation.
    pub last_air_data: AirData,
    /// Most recent HUD annunciator flags.
    pub last_annunciators: AnnunciatorFlags,
    /// Most recent radar height above terrain in meters.
    pub last_agl_m: f64,
    /// Most recent normal load factor in `g`.
    pub last_g_load: f64,
    /// Most recent vertical speed (`v_world.y`) in `m/s`.
    pub last_vertical_speed_mps: f64,
    /// Initial scenario mode for FDR header reconstruction.
    pub initial_mode: InitialFlightMode,
    /// Initial altitude (level trim) or runway elevation in meters.
    pub initial_altitude_m: f64,
    /// Initial airspeed in `m/s`.
    pub initial_airspeed_mps: f64,
    /// Initial fuel load in kilograms.
    pub initial_fuel_kg: f64,
    /// Terrain synthesis seed associated with this session.
    pub terrain_seed: u64,
    /// Aircraft preset identifier.
    pub aircraft_preset: AircraftPresetKind,
}

impl fmt::Debug for FlightSimState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FlightSimState")
            .field("tick", &self.tick)
            .field("sim_time_s", &self.sim_time_s)
            .field("phase", &self.phase)
            .field("fuel_kg", &self.fuel_kg)
            .field("gear_down", &self.gear_down)
            .field("flaps_norm", &self.flaps_norm)
            .field("position", &self.body.position)
            .field("rng_seed", &self.rng_seed)
            .field("rng_steps", &self.rng_steps)
            .finish()
    }
}

impl Clone for FlightSimState {
    fn clone(&self) -> Self {
        let mut rng = Pcg32::new(self.rng_seed);
        for _ in 0..self.rng_steps {
            let _ = rng.next_u64();
        }
        Self {
            body: self.body,
            mass: self.mass,
            aircraft: self.aircraft,
            env: self.env,
            controls: self.controls,
            ground_contact: self.ground_contact,
            integrator: self.integrator,
            clock: self.clock,
            rng,
            rng_seed: self.rng_seed,
            rng_steps: self.rng_steps,
            gust_intensity_mps: self.gust_intensity_mps,
            last_gust_world_mps: self.last_gust_world_mps,
            tick: self.tick,
            sim_time_s: self.sim_time_s,
            fuel_kg: self.fuel_kg,
            gear_down: self.gear_down,
            flaps_norm: self.flaps_norm,
            brake_norm: self.brake_norm,
            phase: self.phase,
            last_isa: self.last_isa,
            last_air_data: self.last_air_data,
            last_annunciators: self.last_annunciators,
            last_agl_m: self.last_agl_m,
            last_g_load: self.last_g_load,
            last_vertical_speed_mps: self.last_vertical_speed_mps,
            initial_mode: self.initial_mode,
            initial_altitude_m: self.initial_altitude_m,
            initial_airspeed_mps: self.initial_airspeed_mps,
            initial_fuel_kg: self.initial_fuel_kg,
            terrain_seed: self.terrain_seed,
            aircraft_preset: self.aircraft_preset,
        }
    }
}

impl FlightSimState {
    /// Validate every embedded subsystem state and scalar invariant.
    pub fn sim_state_validate(&self) -> Result<(), FlightSimError> {
        sim_state_validate(self)
    }

    /// Compute the canonical 64-bit FNV-1a digest of this [`FlightSimState`].
    pub fn sim_state_digest_fnv1a(&self) -> Result<u64, FlightSimError> {
        state_digest_fnv1a(self)
    }

    /// Construct a [`BlackBoxHeader`] matching this initial state.
    pub fn sim_state_black_box_header(&self) -> Result<BlackBoxHeader, FlightSimError> {
        self.sim_state_validate()?;
        let digest = state_digest_fnv1a(self)?;
        let header = BlackBoxHeader {
            version: BLACK_BOX_VERSION,
            rng_seed: self.rng_seed,
            terrain_seed: self.terrain_seed,
            aircraft_preset: self.aircraft_preset,
            initial_mode: self.initial_mode,
            fixed_dt_ns: self.clock.fixed_dt_ns,
            initial_altitude_m: canon_zero(self.initial_altitude_m),
            initial_airspeed_mps: canon_zero(self.initial_airspeed_mps),
            initial_fuel_kg: canon_zero(self.initial_fuel_kg),
            gust_intensity_mps: canon_zero(self.gust_intensity_mps),
            initial_digest_fnv1a: digest,
        };
        header.header_validate()?;
        Ok(header)
    }
}

/// Summary returned by [`step_in_place`] and [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepReport {
    /// Updated monotonic frame tick.
    pub tick: u64,
    /// Number of fixed physics substeps executed this frame.
    pub substeps_run: u32,
    /// Updated flight lifecycle phase.
    pub phase: FlightPhase,
    /// Remaining fuel mass in kilograms.
    pub fuel_kg: f64,
    /// True if flameout (`fuel_kg == 0.0`) is active.
    pub flameout: bool,
    /// Post-step altitude above ground level in meters.
    pub agl_m: f64,
    /// Canonical 64-bit FNV-1a digest of the post-step [`FlightSimState`].
    pub state_digest_fnv1a: u64,
}

// ---------------------------------------------------------------------------
// Initialization Helpers
// ---------------------------------------------------------------------------

fn trainer_mass_properties() -> Result<MassProperties, FlightSimError> {
    Ok(MassProperties::mass_props_from_diagonal(
        1000.0, 1285.0, 2667.0, 1824.0,
    )?)
}

fn draw_gust_from_rng(rng: &mut Pcg32, intensity_mps: f64) -> Vec3 {
    if intensity_mps <= 0.0 {
        let _ = rng.next_u64();
        return Vec3::zero();
    }
    let raw = rng.next_u64();
    let sx = ((raw & 0xffff) as i16) as f64 / 32768.0;
    let sy = (((raw >> 16) & 0xffff) as i16) as f64 / 32768.0;
    let sz = (((raw >> 32) & 0xffff) as i16) as f64 / 32768.0;
    Vec3::new(
        canon_zero(sx * intensity_mps),
        canon_zero(0.35 * sy * intensity_mps),
        canon_zero(sz * intensity_mps),
    )
}

struct InitScenario {
    phase: FlightPhase,
    gear_down: bool,
    ground_elevation_m: f64,
    initial_mode: InitialFlightMode,
    initial_altitude_m: f64,
    initial_airspeed_mps: f64,
    agl_m: f64,
}

fn build_initial_state(
    config: FlightSimConfig,
    body: RigidBodyState,
    controls: ControlInputs,
    scenario: InitScenario,
) -> Result<FlightSimState, FlightSimError> {
    let clean_cfg = validate_sim_config(&config)?;
    let mass = trainer_mass_properties()?;
    let aircraft = AircraftModel::aircraft_preset_trainer()?;
    let env =
        FlightEnvironment::env_new(Vec3::zero(), ISA_GRAVITY_M_S2, scenario.ground_elevation_m)?;
    let ground_plane = Plane::plane_from_normal_d(Vec3::unit_y(), -scenario.ground_elevation_m)?;
    let ground_contact =
        GroundContactParams::contact_params_penalty(ground_plane, 85_000.0, 14_000.0, 0.08, 4)?;
    let clock_cfg = GameClockConfig::new(
        clean_cfg.fixed_dt_ns,
        clean_cfg.max_frame_steps,
        game_clock::DEFAULT_MAX_FRAME_DELTA_NS.max(clean_cfg.fixed_dt_ns),
    )?;
    let clock = GameClockState::new(clock_cfg)?;

    let mut rng = Pcg32::new(clean_cfg.rng_seed);
    let last_gust_world_mps = draw_gust_from_rng(&mut rng, clean_cfg.gust_intensity_mps);
    let rng_steps = 1_u64;

    let eval = aircraft.aircraft_evaluate(&body, &mass, &env, &controls)?;
    let v_world = body.linear_velocity(&mass)?;
    let state = FlightSimState {
        body,
        mass,
        aircraft,
        env,
        controls,
        ground_contact,
        integrator: IntegratorKind::RK4,
        clock,
        rng,
        rng_seed: clean_cfg.rng_seed,
        rng_steps,
        gust_intensity_mps: clean_cfg.gust_intensity_mps,
        last_gust_world_mps,
        tick: 0,
        sim_time_s: 0.0,
        fuel_kg: clean_cfg.initial_fuel_kg,
        gear_down: scenario.gear_down,
        flaps_norm: 0.0,
        brake_norm: 0.0,
        phase: scenario.phase,
        last_isa: eval.atmosphere,
        last_air_data: eval.air_data,
        last_annunciators: AnnunciatorFlags {
            stall: false,
            overspeed: false,
            terrain: false,
            pull_up: false,
        },
        last_agl_m: canon_zero(scenario.agl_m),
        last_g_load: 1.0,
        last_vertical_speed_mps: canon_zero(v_world.y),
        initial_mode: scenario.initial_mode,
        initial_altitude_m: canon_zero(scenario.initial_altitude_m),
        initial_airspeed_mps: canon_zero(scenario.initial_airspeed_mps),
        initial_fuel_kg: canon_zero(clean_cfg.initial_fuel_kg),
        terrain_seed: clean_cfg.terrain_seed,
        aircraft_preset: clean_cfg.aircraft_preset,
    };
    sim_state_validate(&state)?;
    Ok(state)
}

/// Initialize the aircraft parked at the departure end of `runway` in
/// [`FlightPhase::OnRunway`].
pub fn sim_init_on_runway(
    config: FlightSimConfig,
    runway: &RunwaySpec,
) -> Result<FlightSimState, FlightSimError> {
    let clean_cfg = validate_sim_config(&config)?;
    let elev = check_range(
        runway.elevation,
        MIN_ALTITUDE_ENVELOPE_M,
        MAX_ALTITUDE_ENVELOPE_M,
    )?;
    let cx = check_range(runway.center_x, -MAX_WORLD_EXTENT_M, MAX_WORLD_EXTENT_M)?;
    let cz = check_range(runway.center_z, -MAX_WORLD_EXTENT_M, MAX_WORLD_EXTENT_M)?;
    let len = check_range(runway.length, 10.0, MAX_WORLD_EXTENT_M)?;
    let _ = check_range(runway.width, 1.0, MAX_WORLD_EXTENT_M)?;
    let heading = check_finite(runway.heading_rad)?;

    let fwd_x = trig::cos(heading);
    let fwd_z = trig::sin(heading);
    let start_x = cx - 0.35 * len * fwd_x;
    let start_z = cz - 0.35 * len * fwd_z;
    let start_y = elev + GEAR_HEIGHT_M;

    let pos = Vec3::try_vec3(start_x, start_y, start_z)?;
    let orient = Quat::from_axis_angle(Vec3::unit_y(), -heading)?.quat_normalize()?;
    let body = RigidBodyState::body_new(pos, orient)?;

    build_initial_state(
        clean_cfg,
        body,
        ControlInputs::controls_neutral(),
        InitScenario {
            phase: FlightPhase::OnRunway,
            gear_down: true,
            ground_elevation_m: 0.0,
            initial_mode: InitialFlightMode::OnRunway,
            initial_altitude_m: elev,
            initial_airspeed_mps: 0.0,
            agl_m: GEAR_HEIGHT_M,
        },
    )
}

/// Initialize the aircraft trimmed in steady level airborne flight at
/// `altitude_m` and `airspeed_mps` using [`AircraftModel::aircraft_solve_level_trim`].
pub fn sim_init_level_flight(
    config: FlightSimConfig,
    altitude_m: f64,
    airspeed_mps: f64,
) -> Result<FlightSimState, FlightSimError> {
    let clean_cfg = validate_sim_config(&config)?;
    let alt = check_range(altitude_m, 10.0, MAX_CEILING_M)?;
    let tas = check_range(airspeed_mps, 25.0, 200.0)?;
    let mass = trainer_mass_properties()?;
    let aircraft = AircraftModel::aircraft_preset_trainer()?;
    let env = FlightEnvironment::env_new(Vec3::zero(), ISA_GRAVITY_M_S2, 0.0)?;

    let (alpha_trim, trim_controls) = aircraft.aircraft_solve_level_trim(&mass, &env, alt, tas)?;

    let pos = Vec3::try_vec3(0.0, alt, 0.0)?;
    let orient = Quat::from_axis_angle(Vec3::unit_z(), alpha_trim)?.quat_normalize()?;
    let mut body = RigidBodyState::body_new(pos, orient)?;
    body.linear_momentum = Vec3::try_vec3(mass.mass_props_mass() * tas, 0.0, 0.0)?;

    build_initial_state(
        clean_cfg,
        body,
        trim_controls,
        InitScenario {
            phase: FlightPhase::Airborne,
            gear_down: false,
            ground_elevation_m: 0.0,
            initial_mode: InitialFlightMode::LevelTrim,
            initial_altitude_m: alt,
            initial_airspeed_mps: tas,
            agl_m: alt,
        },
    )
}

/// Reconstruct the exact initial [`FlightSimState`] from a decoded [`BlackBoxHeader`].
pub fn sim_init_from_black_box_header(
    header: &BlackBoxHeader,
    runway: &RunwaySpec,
) -> Result<FlightSimState, FlightSimError> {
    header.header_validate()?;
    let cfg = FlightSimConfig {
        rng_seed: header.rng_seed,
        terrain_seed: header.terrain_seed,
        aircraft_preset: header.aircraft_preset,
        fixed_dt_ns: header.fixed_dt_ns,
        max_frame_steps: game_clock::DEFAULT_MAX_FRAME_STEPS,
        initial_fuel_kg: header.initial_fuel_kg,
        gust_intensity_mps: header.gust_intensity_mps,
    };
    match header.initial_mode {
        InitialFlightMode::OnRunway => sim_init_on_runway(cfg, runway),
        InitialFlightMode::LevelTrim => {
            sim_init_level_flight(cfg, header.initial_altitude_m, header.initial_airspeed_mps)
        }
    }
}

// ---------------------------------------------------------------------------
// State Validation & Canonical Digest
// ---------------------------------------------------------------------------

/// Validate every embedded component and scalar invariant of `state`.
pub fn sim_state_validate(state: &FlightSimState) -> Result<(), FlightSimError> {
    state.body.body_validate()?;
    state.mass.mass_props_validate()?;
    state.aircraft.aircraft_validate()?;
    state.env.env_validate()?;
    state.controls.controls_validate()?;
    let _ = state.ground_contact.contact_params_validate()?;
    state.clock.validate()?;
    state.last_isa.isa_validate()?;

    let _ = check_vec3_bounded(state.body.position, MAX_WORLD_EXTENT_M)?;
    let _ = check_range(
        state.body.position.y + state.env.ground_elevation_m,
        MIN_ALTITUDE_ENVELOPE_M,
        MAX_ALTITUDE_ENVELOPE_M,
    )?;
    let _ = validate_dt_ns(state.clock.fixed_dt_ns)?;
    let _ = check_range(state.gust_intensity_mps, 0.0, MAX_GUST_MPS)?;
    let _ = check_vec3_bounded(state.last_gust_world_mps, MAX_GUST_MPS)?;
    let _ = check_range(state.sim_time_s, 0.0, 1.0e12)?;
    let _ = check_range(state.fuel_kg, 0.0, MAX_FUEL_KG)?;
    let _ = check_range(state.flaps_norm, 0.0, 1.0)?;
    let _ = check_range(state.brake_norm, 0.0, 1.0)?;
    let _ = check_range(state.last_agl_m, 0.0, MAX_RADAR_ALTITUDE_M)?;
    let _ = check_range(state.last_g_load, -MAX_G_LOAD, MAX_G_LOAD)?;
    let _ = check_range(
        state.last_vertical_speed_mps,
        -MAX_VERTICAL_SPEED_MPS,
        MAX_VERTICAL_SPEED_MPS,
    )?;
    let _ = check_range(
        state.last_air_data.true_airspeed_mps,
        0.0,
        cockpit_hud_paint::MAX_AIRSPEED_MPS,
    )?;
    let _ = check_range(
        state.last_air_data.alpha_rad,
        -trig::PI - 1e-9,
        trig::PI + 1e-9,
    )?;
    let _ = check_range(
        state.last_air_data.beta_rad,
        -trig::PI - 1e-9,
        trig::PI + 1e-9,
    )?;
    let _ = check_range(state.last_air_data.dynamic_pressure_pa, 0.0, 1.0e9)?;
    let _ = check_range(state.last_air_data.mach, 0.0, cockpit_hud_paint::MAX_MACH)?;
    let _ = check_vec3_bounded(state.last_air_data.body_air_velocity_mps, 1.0e6)?;
    let _ = check_range(
        state.initial_altitude_m,
        MIN_ALTITUDE_ENVELOPE_M,
        MAX_ALTITUDE_ENVELOPE_M,
    )?;
    let _ = check_range(
        state.initial_airspeed_mps,
        0.0,
        cockpit_hud_paint::MAX_AIRSPEED_MPS,
    )?;
    let _ = check_range(state.initial_fuel_kg, 0.0, MAX_FUEL_KG)?;
    Ok(())
}

/// Compute a canonical 64-bit FNV-1a digest of `state` (`-0.0` normalized to `+0.0`).
///
/// Zero heap allocations.
pub fn state_digest_fnv1a(state: &FlightSimState) -> Result<u64, FlightSimError> {
    sim_state_validate(state)?;

    let mut h = FNV_OFFSET;
    h = fnv1a_mix_u64(h, state.body.state_digest_fnv1a()?);
    h = fnv1a_mix_f64(h, state.mass.mass_props_mass())?;
    h = fnv1a_mix_vec3(h, state.env.wind_world_mps)?;
    h = fnv1a_mix_f64(h, state.env.gravity_m_s2)?;
    h = fnv1a_mix_f64(h, state.env.ground_elevation_m)?;
    h = fnv1a_mix_f64(h, state.controls.elevator_rad)?;
    h = fnv1a_mix_f64(h, state.controls.aileron_rad)?;
    h = fnv1a_mix_f64(h, state.controls.rudder_rad)?;
    h = fnv1a_mix_f64(h, state.controls.throttle)?;
    h = fnv1a_mix_u64(h, state.clock.tick_count);
    h = fnv1a_mix_u64(h, state.clock.frame_count);
    h = fnv1a_mix_u64(h, state.clock.elapsed_sim_ns);
    h = fnv1a_mix_u64(h, state.clock.accumulator_ns);
    h = fnv1a_mix_u64(h, state.rng_seed);
    h = fnv1a_mix_u64(h, state.rng_steps);
    h = fnv1a_mix_f64(h, state.gust_intensity_mps)?;
    h = fnv1a_mix_vec3(h, state.last_gust_world_mps)?;
    h = fnv1a_mix_u64(h, state.tick);
    h = fnv1a_mix_f64(h, state.sim_time_s)?;
    h = fnv1a_mix_f64(h, state.fuel_kg)?;
    h = fnv1a_mix_bytes(
        h,
        &[
            u8::from(state.gear_down),
            state.phase.phase_to_u8(),
            u8::from(state.last_annunciators.stall),
            u8::from(state.last_annunciators.overspeed),
            u8::from(state.last_annunciators.terrain),
            u8::from(state.last_annunciators.pull_up),
            state.initial_mode.initial_mode_to_u8(),
            state.aircraft_preset.preset_to_u8(),
        ],
    );
    h = fnv1a_mix_f64(h, state.flaps_norm)?;
    h = fnv1a_mix_f64(h, state.brake_norm)?;
    h = fnv1a_mix_f64(h, state.last_agl_m)?;
    h = fnv1a_mix_f64(h, state.last_g_load)?;
    h = fnv1a_mix_f64(h, state.last_vertical_speed_mps)?;
    h = fnv1a_mix_f64(h, state.last_isa.density_kg_m3)?;
    h = fnv1a_mix_f64(h, state.last_air_data.true_airspeed_mps)?;
    h = fnv1a_mix_u64(h, state.terrain_seed);
    Ok(h)
}

// ---------------------------------------------------------------------------
// 6-DOF Physics Step & Ground Contact Lifecycle (Atomic + Zero-Alloc)
// ---------------------------------------------------------------------------

fn body_attitude_rad(orient: Quat) -> Result<(f64, f64, f64), FlightSimError> {
    let q = orient.quat_normalize()?;
    let fwd = q.rotate_vec3(Vec3::new(1.0, 0.0, 0.0))?;
    let up = q.rotate_vec3(Vec3::new(0.0, 1.0, 0.0))?;
    let right = q.rotate_vec3(Vec3::new(0.0, 0.0, 1.0))?;
    let horiz = trig::sqrt((fwd.x * fwd.x + fwd.z * fwd.z).max(0.0));
    let pitch = trig::atan2(fwd.y, horiz);
    let mut heading = trig::atan2(fwd.z, fwd.x);
    if heading < 0.0 {
        heading += 2.0 * trig::PI;
    }
    let roll = if horiz > 1e-6 {
        trig::atan2(-right.y, up.y)
    } else {
        0.0
    };
    Ok((canon_zero(pitch), canon_zero(roll), canon_zero(heading)))
}

fn clamp_world_and_ceiling(
    body: &mut RigidBodyState,
    mass: &MassProperties,
    ground_elevation_m: f64,
) -> Result<(), FlightSimError> {
    let msl = body.position.y + ground_elevation_m;
    if msl > MAX_CEILING_M {
        body.position.y = MAX_CEILING_M - ground_elevation_m;
        if body.linear_momentum.y > 0.0 {
            body.linear_momentum.y = 0.0;
        }
    }
    if body.position.x > MAX_WORLD_EXTENT_M {
        body.position.x = MAX_WORLD_EXTENT_M;
        if body.linear_momentum.x > 0.0 {
            body.linear_momentum.x = 0.0;
        }
    } else if body.position.x < -MAX_WORLD_EXTENT_M {
        body.position.x = -MAX_WORLD_EXTENT_M;
        if body.linear_momentum.x < 0.0 {
            body.linear_momentum.x = 0.0;
        }
    }
    if body.position.z > MAX_WORLD_EXTENT_M {
        body.position.z = MAX_WORLD_EXTENT_M;
        if body.linear_momentum.z > 0.0 {
            body.linear_momentum.z = 0.0;
        }
    } else if body.position.z < -MAX_WORLD_EXTENT_M {
        body.position.z = -MAX_WORLD_EXTENT_M;
        if body.linear_momentum.z < 0.0 {
            body.linear_momentum.z = 0.0;
        }
    }
    let _ = mass;
    Ok(())
}

/// Advance the flight simulation by one frame in place (`VER04m` `state=explicit`,
/// `VER07` zero-alloc).
///
/// Strictly atomic on `Err`: validates `state`, `heightfield`, `runway`, `input`,
/// `game_clock::advance_frame`, and optional `recorder` capacity **before**
/// mutating `state` (including `state.rng`) or pushing to `recorder`.
pub fn step_in_place(
    state: &mut FlightSimState,
    heightfield: &Heightfield<'_>,
    runway: &RunwaySpec,
    input: &FlightInputFrame,
    recorder: Option<&mut BlackBoxRecorder>,
) -> Result<StepReport, FlightSimError> {
    // 1. Pre-validate everything on read-only inputs & local copies.
    sim_state_validate(state)?;
    let clean_in = validate_input_frame(input)?;
    let _ = validate_terrain_and_runway(heightfield, runway, state.body.position)?;

    let mut next_clock = state.clock;
    let clock_step = advance_frame(&next_clock, clean_in.dt_ns)?;
    next_clock = clock_step.state;

    if let Some(rec) = recorder.as_deref() {
        if rec.recorder_is_full() {
            return Err(FlightSimError::BlackBoxFull);
        }
    }

    // 2. Run physics & ground-contact transitions on local copies.
    let mut next_body = state.body;
    let mut next_fuel = state.fuel_kg;
    let mut next_phase = state.phase;
    let mut next_controls = state.controls;
    let mut step_env = state.env;
    step_env.wind_world_mps = state
        .env
        .wind_world_mps
        .vec3_add(state.last_gust_world_mps)?;
    let sub_dt = (next_clock.fixed_dt_ns as f64) * 1.0e-9;

    let gear_probes = [
        Vec3::new(1.8, -GEAR_HEIGHT_M, 0.0),
        Vec3::new(-0.3, -GEAR_HEIGHT_M, -1.2),
        Vec3::new(-0.3, -GEAR_HEIGHT_M, 1.2),
    ];
    let belly_probes = [Vec3::new(0.0, -0.5, 0.0)];

    if next_phase == FlightPhase::Crashed {
        next_body.linear_momentum = Vec3::zero();
        next_body.angular_momentum = Vec3::zero();
        next_body.body_clear_accumulators();
    } else {
        let limits = state.aircraft.aircraft_control_limits();
        for _ in 0..clock_step.steps_to_run {
            let effective_throttle = if next_fuel <= 0.0 {
                0.0
            } else {
                let burn = (IDLE_BURN_KG_S + FULL_BURN_KG_S * clean_in.throttle) * sub_dt;
                next_fuel = canon_zero((next_fuel - burn).max(0.0));
                clean_in.throttle
            };

            next_controls = ControlInputs::controls_new(
                -clean_in.elevator * limits.elevator_max_rad,
                clean_in.aileron * limits.aileron_max_rad,
                clean_in.rudder * limits.rudder_max_rad,
                effective_throttle,
            )?;

            let (gy, gn) = validate_terrain_and_runway(heightfield, runway, next_body.position)?;
            let pre_agl = next_body.position.y - gy;
            let pre_vel = next_body.linear_velocity(&state.mass)?;
            let sink_rate = -pre_vel.y;

            let near_ground = pre_agl <= GEAR_HEIGHT_M + 0.35;
            if near_ground {
                let eval = state.aircraft.aircraft_evaluate(
                    &next_body,
                    &state.mass,
                    &step_env,
                    &next_controls,
                )?;
                let horiz_speed = trig::sqrt(pre_vel.x * pre_vel.x + pre_vel.z * pre_vel.z);
                let mut total_force = eval.world_total_force_n;

                // Flap lift/drag modulation and wheel braking on runway/ground.
                if clean_in.flaps > 0.0 && !eval.air_data.low_airspeed_zeroed {
                    let q_s =
                        eval.air_data.dynamic_pressure_pa * state.aircraft.aircraft_wing_area_m2();
                    let flap_lift = Vec3::new(0.0, 0.35 * clean_in.flaps * q_s, 0.0);
                    total_force = total_force.vec3_add(flap_lift)?;
                }
                if clean_in.gear_down && pre_agl <= GEAR_HEIGHT_M + 0.15 && horiz_speed > 0.01 {
                    let brake_drag =
                        (180.0 + 3600.0 * clean_in.brake) * (horiz_speed / (horiz_speed + 0.5));
                    let inv_h = 1.0 / horiz_speed;
                    total_force = total_force.vec3_add(Vec3::new(
                        -brake_drag * pre_vel.x * inv_h,
                        0.0,
                        -brake_drag * pre_vel.z * inv_h,
                    ))?;
                }

                next_body.force = total_force;
                next_body.torque = eval.world_total_torque_nm;

                let local_plane = Plane::plane_from_point_normal(
                    Vec3::new(next_body.position.x, gy, next_body.position.z),
                    gn,
                )?;
                let friction = if clean_in.gear_down {
                    0.04 + 0.72 * clean_in.brake
                } else {
                    0.85
                };
                let local_contact = GroundContactParams::contact_params_penalty(
                    local_plane,
                    85_000.0,
                    14_000.0,
                    friction,
                    4,
                )?;
                let probes: &[Vec3] = if clean_in.gear_down {
                    &gear_probes
                } else {
                    &belly_probes
                };
                let _contact_report = next_body.step_with_contacts_report_in_place(
                    &state.mass,
                    sub_dt,
                    state.integrator,
                    &local_contact,
                    probes,
                )?;
                next_body.body_clear_accumulators();
            } else {
                state.aircraft.step_aircraft_in_place(
                    &mut next_body,
                    &state.mass,
                    &step_env,
                    &next_controls,
                    sub_dt,
                    state.integrator,
                )?;
            }

            clamp_world_and_ceiling(&mut next_body, &state.mass, step_env.ground_elevation_m)?;

            let (post_gy, _) =
                validate_terrain_and_runway(heightfield, runway, next_body.position)?;
            let post_agl = next_body.position.y - post_gy;
            let post_vel = next_body.linear_velocity(&state.mass)?;
            let post_speed = post_vel.length()?;
            let (pitch, roll, _) = body_attitude_rad(next_body.orientation)?;
            let on_runway = is_point_on_runway(runway, next_body.position.x, next_body.position.z)?;

            if !clean_in.gear_down && post_agl <= 0.65 {
                next_phase = FlightPhase::Crashed;
            } else if post_agl <= GEAR_HEIGHT_M + 0.20 {
                let unsafe_attitude = roll.abs() > SAFE_LANDING_ROLL_MAX_RAD
                    || !(SAFE_LANDING_PITCH_MIN_RAD..=SAFE_LANDING_PITCH_MAX_RAD).contains(&pitch);
                let hard_impact = sink_rate > CRASH_SINK_RATE_MPS;
                let was_flying =
                    matches!(next_phase, FlightPhase::Airborne | FlightPhase::Approach);
                if (was_flying && (!on_runway || unsafe_attitude || hard_impact))
                    || (!on_runway && unsafe_attitude)
                {
                    next_phase = FlightPhase::Crashed;
                } else if was_flying && on_runway {
                    next_phase = if post_speed < 15.0 || clean_in.brake > 0.4 {
                        FlightPhase::Landed
                    } else {
                        FlightPhase::Approach
                    };
                } else if next_phase == FlightPhase::Landed {
                    if post_speed > 8.0 && effective_throttle > 0.4 {
                        next_phase = FlightPhase::TakeoffRoll;
                    }
                } else if next_phase == FlightPhase::OnRunway && post_speed > 1.0 {
                    next_phase = FlightPhase::TakeoffRoll;
                } else if next_phase == FlightPhase::TakeoffRoll
                    && post_speed < 0.5
                    && effective_throttle < 0.05
                {
                    next_phase = FlightPhase::OnRunway;
                }
            } else {
                next_phase = if clean_in.gear_down && post_agl < 150.0 && post_vel.y < -0.5 {
                    FlightPhase::Approach
                } else {
                    FlightPhase::Airborne
                };
            }

            if next_phase == FlightPhase::Crashed {
                next_body.position.y = post_gy + 0.5;
                next_body.linear_momentum = Vec3::zero();
                next_body.angular_momentum = Vec3::zero();
                next_body.body_clear_accumulators();
                break;
            }
        }
    }

    // 3. Final post-step evaluation & validation BEFORE committing to `state`.
    let (final_gy, _) = validate_terrain_and_runway(heightfield, runway, next_body.position)?;
    let final_eval =
        state
            .aircraft
            .aircraft_evaluate(&next_body, &state.mass, &step_env, &next_controls)?;
    let final_vel = next_body.linear_velocity(&state.mass)?;
    let final_agl = canon_zero((next_body.position.y - final_gy).clamp(0.0, MAX_RADAR_ALTITUDE_M));
    let final_vsi = canon_zero(
        final_vel
            .y
            .clamp(-MAX_VERTICAL_SPEED_MPS, MAX_VERTICAL_SPEED_MPS),
    );
    let (pitch_rad, roll_rad, heading_rad) = body_attitude_rad(next_body.orientation)?;
    let ias_mps = canon_zero(
        (final_eval.air_data.true_airspeed_mps
            * trig::sqrt(
                (final_eval.atmosphere.density_kg_m3
                    / aero_flight_model::ISA_SEA_LEVEL_DENSITY_KG_M3)
                    .max(0.0),
            ))
        .clamp(0.0, cockpit_hud_paint::MAX_AIRSPEED_MPS),
    );
    let msl_m = canon_zero(
        (next_body.position.y + step_env.ground_elevation_m)
            .clamp(MIN_ALTITUDE_ENVELOPE_M, MAX_ALTITUDE_ENVELOPE_M),
    );
    let g_load = if final_agl <= GEAR_HEIGHT_M + 0.25 {
        1.0
    } else {
        let up = next_body
            .orientation
            .quat_normalize()?
            .rotate_vec3(Vec3::unit_y())?;
        let aero_world = next_body
            .orientation
            .quat_normalize()?
            .rotate_vec3(final_eval.body_aero_force_n)?;
        let weight = state.mass.mass_props_mass() * step_env.gravity_m_s2;
        if weight > 1e-6 {
            canon_zero((aero_world.dot(up)? / weight).clamp(-MAX_G_LOAD, MAX_G_LOAD))
        } else {
            1.0
        }
    };

    let default_hud = HudConfig::hud_config_default(1280.0, 720.0)?;
    let dummy_telemetry = FlightTelemetry {
        pitch_rad,
        roll_rad,
        heading_rad,
        indicated_airspeed_mps: ias_mps,
        true_airspeed_mps: final_eval.air_data.true_airspeed_mps,
        mach: final_eval.air_data.mach,
        altitude_msl_m: msl_m,
        radar_altitude_agl_m: final_agl,
        vertical_speed_mps: final_vsi,
        flight_path_pitch_rad: pitch_rad,
        flight_path_yaw_rad: 0.0,
        aoa_rad: final_eval.air_data.alpha_rad,
        sideslip_rad: final_eval.air_data.beta_rad,
        g_load,
        stall_warning: false,
        overspeed_warning: false,
        terrain_warning: false,
        pull_up_warning: false,
    };
    let annunciators = compute_annunciators(&dummy_telemetry, &default_hud);

    let next_tick = state
        .tick
        .checked_add(1)
        .ok_or(FlightSimError::InvalidClockDelta)?;
    let next_sim_time = canon_zero((next_clock.elapsed_sim_ns as f64) * 1.0e-9);
    let next_rng_steps = state
        .rng_steps
        .checked_add(1)
        .ok_or(FlightSimError::InvalidClockDelta)?;
    let _ = isa_atmosphere(msl_m)?;

    // 4. Commit point (infallible from here onward).
    state.body = next_body;
    state.controls = next_controls;
    state.clock = next_clock;
    state.last_gust_world_mps = draw_gust_from_rng(&mut state.rng, state.gust_intensity_mps);
    state.rng_steps = next_rng_steps;
    state.tick = next_tick;
    state.sim_time_s = next_sim_time;
    state.fuel_kg = next_fuel;
    state.gear_down = clean_in.gear_down;
    state.flaps_norm = clean_in.flaps;
    state.brake_norm = clean_in.brake;
    state.phase = next_phase;
    state.last_isa = final_eval.atmosphere;
    state.last_air_data = final_eval.air_data;
    state.last_annunciators = annunciators;
    state.last_agl_m = final_agl;
    state.last_g_load = g_load;
    state.last_vertical_speed_mps = final_vsi;

    let digest = state_digest_fnv1a(state)?;

    if let Some(rec) = recorder {
        rec.push_frame(BlackBoxFrame {
            tick: next_tick,
            input: clean_in,
            phase: next_phase,
            annunciators,
            pitch_rad,
            roll_rad,
            heading_rad,
            ias_mps,
            msl_m,
            agl_m: final_agl,
            vsi_mps: final_vsi,
            g_load,
            fuel_kg: next_fuel,
            state_digest_fnv1a: digest,
        })?;
    }

    Ok(StepReport {
        tick: next_tick,
        substeps_run: clock_step.steps_to_run,
        phase: next_phase,
        fuel_kg: next_fuel,
        flameout: next_fuel <= 0.0,
        agl_m: final_agl,
        state_digest_fnv1a: digest,
    })
}

/// Functional wrapper around [`step_in_place`] returning a new [`FlightSimState`]
/// without mutating `state`.
pub fn step(
    state: &FlightSimState,
    heightfield: &Heightfield<'_>,
    runway: &RunwaySpec,
    input: &FlightInputFrame,
) -> Result<(FlightSimState, StepReport), FlightSimError> {
    let mut next = state.clone();
    let report = step_in_place(&mut next, heightfield, runway, input, None)?;
    Ok((next, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use terrain_lod::{HeightfieldMut, HeightfieldSpec};

    fn make_flat_terrain_with_runway() -> (HeightfieldSpec, Vec<f64>, RunwaySpec) {
        let spec = HeightfieldSpec::spec_new(4, 250.0, -2000.0, -2000.0).expect("spec");
        let mut heights = vec![0.0; spec.spec_vertex_count().expect("vcount")];
        let runway = RunwaySpec::runway_new(0.0, 0.0, 0.0, 1200.0, 60.0, 20.0, 0.0).expect("rwy");
        {
            let mut hmut = HeightfieldMut::from_heights_mut(spec, &mut heights).expect("hmut");
            hmut.place_runway(&runway).expect("place runway");
        }
        (spec, heights, runway)
    }

    #[test]
    fn warrant_boundary_validators_match_spec() {
        assert_eq!(validate_fdr_frame_count(0), Ok(true));
        assert_eq!(validate_fdr_frame_count(65_536), Ok(true));
        assert_eq!(
            validate_fdr_frame_count(65_537),
            Err(FlightSimError::BlackBoxFull)
        );
        assert_eq!(validate_fdr_blob_bytes(88, 0), Ok(true));
        assert_eq!(validate_fdr_blob_bytes(392, 2), Ok(true));
        assert_eq!(
            validate_fdr_blob_bytes(391, 2),
            Err(FlightSimError::BlackBoxCorrupt)
        );
        assert_eq!(validate_dt_ns(16_666_667), Ok(true));
        assert_eq!(validate_dt_ns(0), Err(FlightSimError::InvalidClockDelta));
        assert_eq!(validate_altitude_envelope_m(1500), Ok(true));
        assert_eq!(
            validate_altitude_envelope_m(20_001),
            Err(FlightSimError::ValueOutOfBounds)
        );
        assert_eq!(validate_fuel_mg(0), Ok(true));
        assert_eq!(validate_fuel_mg(-1), Err(FlightSimError::ValueOutOfBounds));
        assert_eq!(validate_render_dim(32), Ok(true));
        assert_eq!(
            validate_render_dim(0),
            Err(FlightSimError::InvalidDimensions)
        );
    }

    #[test]
    fn flameout_at_zero_fuel_is_monotonic() {
        let (spec, heights, runway) = make_flat_terrain_with_runway();
        let hf = Heightfield::from_heights(spec, &heights).expect("hf");
        let mut cfg = FlightSimConfig::sim_config_default(7, 11);
        cfg.initial_fuel_kg = 0.01;
        let mut sim = sim_init_on_runway(cfg, &runway).expect("init");
        let input = FlightInputFrame {
            dt_ns: cfg.fixed_dt_ns,
            elevator: 0.0,
            aileron: 0.0,
            rudder: 0.0,
            throttle: 1.0,
            flaps: 0.0,
            brake: 0.0,
            gear_down: true,
        };
        for _ in 0..120 {
            let rep = step_in_place(&mut sim, &hf, &runway, &input, None).expect("step");
            assert!(rep.fuel_kg >= 0.0);
        }
        assert_eq!(sim.fuel_kg, 0.0);
        assert_eq!(sim.controls.throttle, 0.0);
    }
}
