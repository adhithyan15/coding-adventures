//! # aero-flight-model
//!
//! GAME41: deterministic, zero-allocation International Standard Atmosphere (ISA
//! 1976), bounded airfoil coefficient polar tables, air-data computation, and
//! 6-DOF aerodynamic force/moment evaluation for Era 4 (`rigidbody-6dof`,
//! `terrain-lod`, `cockpit-hud-paint`, `game-flight-sim`).
//!
//! ## Coordinate Frame & Sign Conventions
//!
//! Both `rigidbody-6dof` (`GAME40`) and `terrain-lod` (`GAME42`) operate in a
//! right-handed **Y-up** world frame (`+Y` = altitude / up). To align directly
//! at `Quat::quat_identity()` with zero hidden frame conversions, this crate
//! uses the right-handed **Forward-Up-Right (FUR)** aircraft body frame:
//!
//! ```text
//!          +Y_body (Up / Dorsal, w = -v_body.y)
//!          |
//!          |    +X_body (Forward / Nose, u = v_body.x, Roll +p right-wing-down)
//!          |   /
//!          |  /
//!          | /
//!          +--------> +Z_body (Right Wing, v = v_body.z, Pitch +q nose-up)
//! ```
//!
//! Standard aerospace air-data scalars `(u, v, w)` map cleanly from the
//! body-frame airspeed vector `v_air_body = q^-1 * (v_world - wind_world)`:
//! - `u = v_air_body.x` (forward along nose)
//! - `v = v_air_body.z` (rightward along starboard wing)
//! - `w = -v_air_body.y` (downward, positive when nose pitches up relative to flight path)
//! - `V = sqrt(u^2 + v^2 + w^2)`
//! - `alpha = atan2(w, u)` (positive when relative wind strikes underside of wing)
//! - `beta = asin(clamp(v / V, -1, 1))` (positive when relative wind arrives from starboard)
//! - `q_bar = 0.5 * rho * V^2`
//!
//! Body aerodynamic forces and moments follow:
//! - Lift `L = q_bar * S * C_L`, Drag `D = q_bar * S * C_D`, Side force `Y = q_bar * S * C_Y`
//! - `F_x_body = -D * cos(alpha) + L * sin(alpha) + Thrust`
//! - `F_y_body =  D * sin(alpha) + L * cos(alpha)`
//! - `F_z_body =  Y`
//! - Roll `tau_x_body = q_bar * S * b * C_l` (+right wing down)
//! - Pitch `tau_z_body = q_bar * S * c_bar * C_m` (+nose up around `+Z_body`)
//! - Yaw `tau_y_body = -q_bar * S * b * C_n` (`C_n > 0` turns nose starboard around `-Y_body`)
//!
//! ## International Standard Atmosphere (ISA 1976, 0–20 km)
//!
//! ```text
//!   Altitude h [m] | Layer         | Lapse Rate     | Temperature T [K]
//!   ---------------+---------------+----------------+------------------------
//!   [-500, 11000]  | Troposphere   | -0.0065 K/m    | 288.15 - 0.0065 * h
//!   (11000, 20000] | Tropopause    |  0.0000 K/m    | 216.65 (isothermal)
//! ```
//!
//! All exponentials, logarithms, powers, square roots, and inverse/direct
//! trigonometric functions are implemented deterministically in [`det_math`]
//! and [`trig`] with zero calls to platform `libm`, guaranteeing bit-identical
//! digests across Windows, Linux, and macOS.
//!
//! ## Safety, Determinism, and Resource Guarantees
//!
//! * `#![forbid(unsafe_code)]`; `#![deny(missing_docs)]`.
//! * Zero ambient capabilities (`"capabilities": []`, `CAP001`).
//! * Zero heap allocation in every runtime struct and evaluation method
//!   (`VER07`), verified with `CountingAllocator`.
//! * Strict `CWE-20`, `CWE-125`, `CWE-190`, `CWE-369`, `CWE-400`, `CWE-787`,
//!   and `CWE-835` guards, plus atomic state preservation in
//!   [`AircraftModel::step_aircraft_in_place`].

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Generated capability verification witnesses.
pub mod capabilities;
mod det_math;

pub use capabilities::PureCapabilities;

use math3d::{Math3dError, Quat, Vec3};
use rigidbody_6dof::{IntegratorKind, MassProperties, RigidBodyError, RigidBodyState};

/// Minimum supported geometric altitude in meters (`-500.0 m`).
pub const MIN_ALTITUDE_M: f64 = -500.0;

/// Maximum supported geometric/geopotential altitude in meters (`20_000.0 m`).
pub const MAX_ALTITUDE_M: f64 = 20_000.0;

/// Tropopause boundary altitude in meters (`11_000.0 m`).
pub const TROPOPAUSE_ALTITUDE_M: f64 = 11_000.0;

/// Sea-level ISA 1976 temperature in Kelvin (`288.15 K`).
pub const ISA_SEA_LEVEL_TEMP_K: f64 = 288.15;

/// Sea-level ISA 1976 static pressure in Pascals (`101_325.0 Pa`).
pub const ISA_SEA_LEVEL_PRESSURE_PA: f64 = 101_325.0;

/// Sea-level ISA 1976 air density in `kg/m^3` (`1.225 kg/m^3`).
pub const ISA_SEA_LEVEL_DENSITY_KG_M3: f64 = 1.225;

/// Troposphere temperature lapse rate in `K/m` (`-0.0065 K/m`).
pub const ISA_LAPSE_RATE_K_PER_M: f64 = -0.0065;

/// Standard gravitational acceleration in `m/s^2` (`9.80665 m/s^2`).
pub const ISA_GRAVITY_M_S2: f64 = 9.80665;

/// Specific gas constant for dry air in `J/(kg*K)` (`287.05287 J/(kg*K)`).
pub const ISA_GAS_CONSTANT_J_KG_K: f64 = 287.05287;

/// Adiabatic index (ratio of specific heats) for dry air (`1.4`).
pub const ISA_GAMMA: f64 = 1.4;

/// Low-airspeed threshold in `m/s` below which `alpha`, `beta`, `q_bar`,
/// and aerodynamic forces/moments are safely zeroed (`0.25 m/s`, `CWE-369`).
pub const MIN_AIRSPEED: f64 = 0.25;

/// Defensive minimum speed of sound in `m/s` for Mach division (`1.0 m/s`, `CWE-369`).
pub const MIN_SPEED_OF_SOUND: f64 = 1.0;

/// Maximum number of `(x, y)` points allowed in a polar [`AeroTable`] (`64`, `CWE-400`).
pub const MAX_TABLE_POINTS: usize = 64;

/// Minimum reference wing area in `m^2` (`1e-12 m^2`, `CWE-369`).
pub const MIN_AREA_M2: f64 = 1e-12;

/// Maximum reference wing area in `m^2` (`1e9 m^2`, `CWE-400`).
pub const MAX_AREA_M2: f64 = 1e9;

/// Minimum reference span or chord length in meters (`1e-6 m`, `CWE-369`).
pub const MIN_LENGTH_M: f64 = 1e-6;

/// Maximum reference span or chord length in meters (`1e6 m`, `CWE-400`).
pub const MAX_LENGTH_M: f64 = 1e6;

/// Maximum magnitude of any dimensionless aerodynamic coefficient (`1e4`, `CWE-20`).
pub const MAX_COEFF_MAG: f64 = 1e4;

/// Maximum engine thrust in Newtons (`1e9 N`, `CWE-20`).
pub const MAX_THRUST_N: f64 = 1e9;

/// Maximum gravitational acceleration in `m/s^2` (`1e4 m/s^2`, `CWE-20`).
pub const MAX_GRAVITY_M_S2: f64 = 1e4;

/// Maximum safe finite coordinate/force/torque magnitude (`1e12`, `CWE-190` / `CWE-400`).
pub const MAX_COORD: f64 = math3d::MAX_COORD;

/// Numerical epsilon for near-zero guards (`1e-12`, `CWE-369`).
pub const EPSILON: f64 = math3d::EPSILON;

/// Maximum bisection iterations in [`AircraftModel::aircraft_solve_level_trim`] (`64`, `CWE-835`).
pub const MAX_TRIM_ITERS: usize = 64;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Failure modes for every fallible operation in `aero-flight-model`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AeroError {
    /// A scalar, vector, quaternion, or table entry is `NaN` or `+/-Infinity` (`CWE-20`).
    NonFiniteInput,
    /// Altitude lies outside `[MIN_ALTITUDE_M, MAX_ALTITUDE_M]` (`CWE-20`).
    AltitudeOutOfRange,
    /// Table length is `< 2`, `> MAX_TABLE_POINTS`, or breakpoint/value slices differ (`CWE-125` / `CWE-400`).
    InvalidTableLength,
    /// Table breakpoints are not strictly increasing beyond scale-relative tolerance (`CWE-369`).
    NonMonotonicBreakpoints,
    /// Reference geometry `(S, b, c_bar)` or control limits violate positivity or scale consistency (`CWE-369`).
    InvalidGeometryParameter,
    /// Aerodynamic derivative, thrust, gravity, or table coefficient is out of valid domain (`CWE-20`).
    InvalidAeroParameter,
    /// Computed dynamic pressure, force, or torque became non-finite or exceeded [`MAX_COORD`] (`CWE-190`).
    NumericalDivergence,
    /// Error propagated from `rigidbody-6dof` (`GAME40`).
    RigidBodyError(RigidBodyError),
    /// Error propagated from `math3d` (`GAME30`).
    MathError(Math3dError),
}

impl From<Math3dError> for AeroError {
    #[inline]
    fn from(err: Math3dError) -> Self {
        match err {
            Math3dError::NonFiniteInput => Self::NonFiniteInput,
            Math3dError::ValueOutOfBounds => Self::NumericalDivergence,
            other => Self::MathError(other),
        }
    }
}

impl From<RigidBodyError> for AeroError {
    #[inline]
    fn from(err: RigidBodyError) -> Self {
        match err {
            RigidBodyError::NonFiniteInput => Self::NonFiniteInput,
            RigidBodyError::NumericalDivergence => Self::NumericalDivergence,
            other => Self::RigidBodyError(other),
        }
    }
}

impl std::fmt::Display for AeroError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "input contains NaN or Infinity"),
            Self::AltitudeOutOfRange => {
                write!(
                    f,
                    "altitude must lie in [MIN_ALTITUDE_M, MAX_ALTITUDE_M] (-500..=20000 m)"
                )
            }
            Self::InvalidTableLength => {
                write!(
                    f,
                    "polar table length must match and lie in 2..=MAX_TABLE_POINTS"
                )
            }
            Self::NonMonotonicBreakpoints => {
                write!(f, "polar table breakpoints must be strictly increasing")
            }
            Self::InvalidGeometryParameter => {
                write!(
                    f,
                    "aircraft reference geometry or control limits are invalid"
                )
            }
            Self::InvalidAeroParameter => {
                write!(
                    f,
                    "aerodynamic coefficient, thrust, or environment parameter is out of domain"
                )
            }
            Self::NumericalDivergence => {
                write!(f, "aerodynamic evaluation exceeded finite magnitude bounds")
            }
            Self::RigidBodyError(e) => write!(f, "rigidbody-6dof error: {e}"),
            Self::MathError(e) => write!(f, "math3d error: {e}"),
        }
    }
}

impl std::error::Error for AeroError {}

// ---------------------------------------------------------------------------
// Internal validation & canonicalization helpers
// ---------------------------------------------------------------------------

#[inline]
fn canon_zero(v: f64) -> f64 {
    if v == 0.0 {
        0.0
    } else {
        v
    }
}

#[inline]
fn check_finite(v: f64) -> Result<f64, AeroError> {
    if !v.is_finite() {
        return Err(AeroError::NonFiniteInput);
    }
    Ok(canon_zero(v))
}

#[inline]
fn check_bounded_vec3(v: Vec3) -> Result<Vec3, AeroError> {
    if !v.vec3_is_finite() {
        return Err(AeroError::NonFiniteInput);
    }
    if v.x.abs() > MAX_COORD || v.y.abs() > MAX_COORD || v.z.abs() > MAX_COORD {
        return Err(AeroError::NumericalDivergence);
    }
    Ok(Vec3::new(canon_zero(v.x), canon_zero(v.y), canon_zero(v.z)))
}

#[inline]
fn check_coeff(v: f64) -> Result<f64, AeroError> {
    let v = check_finite(v)?;
    if v.abs() > MAX_COEFF_MAG {
        return Err(AeroError::InvalidAeroParameter);
    }
    Ok(v)
}

#[inline]
fn check_result_vec3(v: Vec3) -> Result<Vec3, AeroError> {
    if !v.vec3_is_finite()
        || v.x.abs() > MAX_COORD
        || v.y.abs() > MAX_COORD
        || v.z.abs() > MAX_COORD
    {
        return Err(AeroError::NumericalDivergence);
    }
    Ok(Vec3::new(canon_zero(v.x), canon_zero(v.y), canon_zero(v.z)))
}

#[inline]
fn clamp_sym(val: f64, limit: f64) -> f64 {
    if val < -limit {
        -limit
    } else if val > limit {
        limit
    } else {
        canon_zero(val)
    }
}

#[inline]
fn fnv1a_push_f64(hash: &mut u64, v: f64) {
    const FNV_PRIME: u64 = 0x0100_0000_01b3;
    let canon = canon_zero(v);
    for byte in canon.to_bits().to_le_bytes() {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

// ---------------------------------------------------------------------------
// Public deterministic transcendental wrappers (bit-identical across OSes)
// ---------------------------------------------------------------------------

/// Deterministic cross-platform `exp(x)` avoiding platform `libm`.
#[inline]
pub fn aero_det_exp(x: f64) -> f64 {
    det_math::det_exp(x)
}

/// Deterministic cross-platform `ln(x)` avoiding platform `libm`.
#[inline]
pub fn aero_det_ln(x: f64) -> f64 {
    det_math::det_ln(x)
}

/// Deterministic cross-platform `base^exp` avoiding platform `libm`.
#[inline]
pub fn aero_det_pow(base: f64, exp: f64) -> f64 {
    det_math::det_pow(base, exp)
}

/// Deterministic clamped `asin(x)` built from `trig::atan2` and `trig::sqrt`.
#[inline]
pub fn aero_det_asin(x: f64) -> f64 {
    det_math::det_asin_clamped(x)
}

// ---------------------------------------------------------------------------
// Warrant contract validators (mirroring code/specs/aero-flight-model.warrant)
// ---------------------------------------------------------------------------

/// Validate integer altitude bounds `[-500, 20000]` m (Warrant `validate_altitude`).
#[inline]
pub const fn validate_altitude_bounds_m(alt_m: i64) -> Result<bool, AeroError> {
    if alt_m >= -500 && alt_m <= 20_000 {
        Ok(true)
    } else {
        Err(AeroError::AltitudeOutOfRange)
    }
}

/// Validate polar table length in `2..=MAX_TABLE_POINTS` (Warrant `validate_table_len`).
#[inline]
pub const fn validate_table_len(len: usize) -> Result<bool, AeroError> {
    if len >= 2 && len <= MAX_TABLE_POINTS {
        Ok(true)
    } else {
        Err(AeroError::InvalidTableLength)
    }
}

/// Validate integer-scaled geometry parameters in milli-units (Warrant `validate_geometry`).
#[inline]
pub const fn validate_geometry_milli(
    area_milli: i64,
    span_milli: i64,
    chord_milli: i64,
) -> Result<bool, AeroError> {
    if area_milli >= 1
        && area_milli <= 1_000_000_000
        && span_milli >= 1
        && span_milli <= 1_000_000_000
        && chord_milli >= 1
        && chord_milli <= 1_000_000_000
    {
        Ok(true)
    } else {
        Err(AeroError::InvalidGeometryParameter)
    }
}

/// Check whether speed in milli-m/s meets the `MIN_AIRSPEED` threshold (Warrant `check_airspeed_active`).
#[inline]
pub const fn check_airspeed_active_milli(speed_milli: u64) -> bool {
    speed_milli >= 250
}

// ---------------------------------------------------------------------------
// ISA 1976 Standard Atmosphere (0 - 20 km)
// ---------------------------------------------------------------------------

/// Atmospheric state sampled from the 1976 International Standard Atmosphere (ISA).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IsaSample {
    /// Altitude in meters within `[MIN_ALTITUDE_M, MAX_ALTITUDE_M]`.
    pub altitude_m: f64,
    /// Static air temperature `T` in Kelvin.
    pub temperature_k: f64,
    /// Static air pressure `p` in Pascals.
    pub pressure_pa: f64,
    /// Air density `rho` in `kg/m^3`.
    pub density_kg_m3: f64,
    /// Local speed of sound `a = sqrt(gamma * R * T)` in `m/s`.
    pub speed_of_sound_mps: f64,
}

impl IsaSample {
    /// Validate that all atmospheric properties are finite and physically positive.
    pub fn isa_validate(&self) -> Result<(), AeroError> {
        let alt = check_finite(self.altitude_m)?;
        if !(MIN_ALTITUDE_M..=MAX_ALTITUDE_M).contains(&alt) {
            return Err(AeroError::AltitudeOutOfRange);
        }
        let t = check_finite(self.temperature_k)?;
        let p = check_finite(self.pressure_pa)?;
        let rho = check_finite(self.density_kg_m3)?;
        let a = check_finite(self.speed_of_sound_mps)?;
        if t <= 0.0 || p <= 0.0 || rho <= 0.0 || a < MIN_SPEED_OF_SOUND {
            return Err(AeroError::InvalidAeroParameter);
        }
        Ok(())
    }
}

/// Evaluate the 1976 International Standard Atmosphere (ISA) at `altitude_m`.
///
/// Covers the troposphere (`[-500, 11_000] m`, lapse rate `-6.5 K/km`) and
/// the isothermal lower stratosphere (`(11_000, 20_000] m`, `216.65 K`).
/// Uses deterministic [`aero_det_pow`] and [`aero_det_exp`] rather than `std` `libm`.
pub fn isa_atmosphere(altitude_m: f64) -> Result<IsaSample, AeroError> {
    let h = check_finite(altitude_m)?;
    if !(MIN_ALTITUDE_M..=MAX_ALTITUDE_M).contains(&h) {
        return Err(AeroError::AltitudeOutOfRange);
    }

    // Troposphere exponent: -g0 / (R * L) > 0 since L = -0.0065 < 0.
    let tropo_exp = -ISA_GRAVITY_M_S2 / (ISA_GAS_CONSTANT_J_KG_K * ISA_LAPSE_RATE_K_PER_M);

    let (temperature_k, pressure_pa) = if h <= TROPOPAUSE_ALTITUDE_M {
        let t = ISA_SEA_LEVEL_TEMP_K + ISA_LAPSE_RATE_K_PER_M * h;
        let ratio = t / ISA_SEA_LEVEL_TEMP_K;
        let p = ISA_SEA_LEVEL_PRESSURE_PA * det_math::det_pow(ratio, tropo_exp);
        (t, p)
    } else {
        let t11 = ISA_SEA_LEVEL_TEMP_K + ISA_LAPSE_RATE_K_PER_M * TROPOPAUSE_ALTITUDE_M;
        let p11 =
            ISA_SEA_LEVEL_PRESSURE_PA * det_math::det_pow(t11 / ISA_SEA_LEVEL_TEMP_K, tropo_exp);
        let dh = h - TROPOPAUSE_ALTITUDE_M;
        let arg = -ISA_GRAVITY_M_S2 * dh / (ISA_GAS_CONSTANT_J_KG_K * t11);
        let p = p11 * det_math::det_exp(arg);
        (t11, p)
    };

    if !temperature_k.is_finite()
        || !pressure_pa.is_finite()
        || temperature_k <= 0.0
        || pressure_pa <= 0.0
    {
        return Err(AeroError::NumericalDivergence);
    }

    let density_kg_m3 = pressure_pa / (ISA_GAS_CONSTANT_J_KG_K * temperature_k);
    let a_sq = ISA_GAMMA * ISA_GAS_CONSTANT_J_KG_K * temperature_k;
    if !density_kg_m3.is_finite() || density_kg_m3 <= 0.0 || !a_sq.is_finite() || a_sq <= 0.0 {
        return Err(AeroError::NumericalDivergence);
    }
    let speed_of_sound_mps = trig::sqrt(a_sq);
    if !speed_of_sound_mps.is_finite() || speed_of_sound_mps < MIN_SPEED_OF_SOUND {
        return Err(AeroError::NumericalDivergence);
    }

    Ok(IsaSample {
        altitude_m: h,
        temperature_k,
        pressure_pa,
        density_kg_m3,
        speed_of_sound_mps,
    })
}

// ---------------------------------------------------------------------------
// Bounded Aerodynamic Polar Table (AeroTable)
// ---------------------------------------------------------------------------

/// Fixed-capacity stack-allocated piecewise-linear 1D lookup table (`<= MAX_TABLE_POINTS`).
///
/// Breakpoints are strictly increasing and finite, validated at construction.
/// Queries outside `[breakpoints[0], breakpoints[len - 1]]` clamp safely to
/// the boundary values (`CWE-125`), and checked `.get(...)` indexing guarantees
/// zero out-of-bounds panics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AeroTable {
    len: usize,
    breakpoints: [f64; MAX_TABLE_POINTS],
    values: [f64; MAX_TABLE_POINTS],
}

impl AeroTable {
    /// Construct a validated [`AeroTable`] from matching `breakpoints` and `values` slices.
    ///
    /// Enforces `2 <= len <= MAX_TABLE_POINTS`, finiteness (`CWE-20`), bounded
    /// magnitude, and strictly increasing breakpoints above scale-relative
    /// spacing (`CWE-369`).
    pub fn aero_table_new(breakpoints: &[f64], values: &[f64]) -> Result<Self, AeroError> {
        let len = breakpoints.len();
        if len != values.len() || !(2..=MAX_TABLE_POINTS).contains(&len) {
            return Err(AeroError::InvalidTableLength);
        }

        let mut bp_arr = [0.0_f64; MAX_TABLE_POINTS];
        let mut val_arr = [0.0_f64; MAX_TABLE_POINTS];

        for i in 0..len {
            let x = match breakpoints.get(i) {
                Some(&v) => check_finite(v)?,
                None => return Err(AeroError::InvalidTableLength),
            };
            if x.abs() > MAX_COORD {
                return Err(AeroError::InvalidAeroParameter);
            }
            let y = match values.get(i) {
                Some(&v) => check_coeff(v)?,
                None => return Err(AeroError::InvalidTableLength),
            };
            if i > 0 {
                let prev = match bp_arr.get(i - 1) {
                    Some(&v) => v,
                    None => return Err(AeroError::InvalidTableLength),
                };
                let dx = x - prev;
                let scale = 1.0_f64.max(x.abs()).max(prev.abs());
                if !dx.is_finite() || dx <= EPSILON * scale {
                    return Err(AeroError::NonMonotonicBreakpoints);
                }
            }
            if let Some(slot) = bp_arr.get_mut(i) {
                *slot = x;
            }
            if let Some(slot) = val_arr.get_mut(i) {
                *slot = y;
            }
        }

        Ok(Self {
            len,
            breakpoints: bp_arr,
            values: val_arr,
        })
    }

    /// Re-validate all table invariants.
    pub fn aero_table_validate(&self) -> Result<(), AeroError> {
        if !(2..=MAX_TABLE_POINTS).contains(&self.len) {
            return Err(AeroError::InvalidTableLength);
        }
        let bps = self
            .breakpoints
            .get(..self.len)
            .ok_or(AeroError::InvalidTableLength)?;
        let vals = self
            .values
            .get(..self.len)
            .ok_or(AeroError::InvalidTableLength)?;
        let _ = Self::aero_table_new(bps, vals)?;
        Ok(())
    }

    /// Return the active number of `(breakpoint, value)` entries (`2..=MAX_TABLE_POINTS`).
    #[inline]
    pub const fn aero_table_len(&self) -> usize {
        self.len
    }

    /// Return the active slice of strictly increasing breakpoints.
    #[inline]
    pub fn aero_table_breakpoints(&self) -> &[f64] {
        self.breakpoints.get(..self.len).unwrap_or(&[])
    }

    /// Return the active slice of tabulated coefficient values.
    #[inline]
    pub fn aero_table_values(&self) -> &[f64] {
        self.values.get(..self.len).unwrap_or(&[])
    }

    /// Evaluate the piecewise-linear table at `x`, clamping outside the domain.
    pub fn aero_table_eval(&self, x: f64) -> Result<f64, AeroError> {
        let x = check_finite(x)?;
        if !(2..=MAX_TABLE_POINTS).contains(&self.len) {
            return Err(AeroError::InvalidTableLength);
        }

        let x_first = *self
            .breakpoints
            .first()
            .ok_or(AeroError::InvalidTableLength)?;
        let y_first = *self.values.first().ok_or(AeroError::InvalidTableLength)?;
        if x <= x_first {
            return Ok(y_first);
        }

        let last_idx = self
            .len
            .checked_sub(1)
            .ok_or(AeroError::InvalidTableLength)?;
        let x_last = *self
            .breakpoints
            .get(last_idx)
            .ok_or(AeroError::InvalidTableLength)?;
        let y_last = *self
            .values
            .get(last_idx)
            .ok_or(AeroError::InvalidTableLength)?;
        if x >= x_last {
            return Ok(y_last);
        }

        // Binary search on the bounded prefix `..self.len` (at most 6 steps).
        let bps = self
            .breakpoints
            .get(..self.len)
            .ok_or(AeroError::InvalidTableLength)?;
        let idx = bps.partition_point(|&bp| bp <= x);
        let lo = idx.saturating_sub(1);
        let hi = lo.saturating_add(1).min(last_idx);

        let x0 = *self
            .breakpoints
            .get(lo)
            .ok_or(AeroError::InvalidTableLength)?;
        let x1 = *self
            .breakpoints
            .get(hi)
            .ok_or(AeroError::InvalidTableLength)?;
        let y0 = *self.values.get(lo).ok_or(AeroError::InvalidTableLength)?;
        let y1 = *self.values.get(hi).ok_or(AeroError::InvalidTableLength)?;

        let dx = x1 - x0;
        let scale = 1.0_f64.max(x1.abs()).max(x0.abs());
        if !dx.is_finite() || dx <= EPSILON * scale {
            return Err(AeroError::NonMonotonicBreakpoints);
        }

        let t_raw = (x - x0) / dx;
        let t = t_raw.clamp(0.0, 1.0);
        let out = y0 + t * (y1 - y0);
        if !out.is_finite() {
            return Err(AeroError::NumericalDivergence);
        }
        Ok(canon_zero(out))
    }
}

// ---------------------------------------------------------------------------
// AirData, FlightEnvironment, ControlInputs, Configuration Structs
// ---------------------------------------------------------------------------

/// Air-data quantities computed from rigid-body velocity, wind, attitude, and ISA.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirData {
    /// True airspeed magnitude `V` in `m/s` (`>= 0`).
    pub true_airspeed_mps: f64,
    /// Air-relative velocity in the `(X=Forward, Y=Up, Z=Right)` body frame.
    pub body_air_velocity_mps: Vec3,
    /// Angle of attack `alpha = atan2(w, u)` in radians (`0` when `V < MIN_AIRSPEED`).
    pub alpha_rad: f64,
    /// Sideslip angle `beta = asin(clamp(v / V, -1, 1))` in radians (`0` when `V < MIN_AIRSPEED`).
    pub beta_rad: f64,
    /// Dynamic pressure `q_bar = 0.5 * rho * V^2` in Pascals (`0` when `V < MIN_AIRSPEED`).
    pub dynamic_pressure_pa: f64,
    /// Mach number `V / a` (`0` when `V < MIN_AIRSPEED`).
    pub mach: f64,
    /// `true` when `V < MIN_AIRSPEED` and aerodynamic angles/forces were safely zeroed.
    pub low_airspeed_zeroed: bool,
}

/// Compute air data from world velocity, wind vector, body-to-world orientation, and ISA sample.
///
/// When `V < MIN_AIRSPEED`, `alpha_rad`, `beta_rad`, `dynamic_pressure_pa`,
/// and `mach` are safely zeroed to prevent `0 / 0` singularities (`CWE-369`).
pub fn compute_air_data(
    velocity_world_mps: Vec3,
    wind_world_mps: Vec3,
    orientation: Quat,
    atmosphere: &IsaSample,
) -> Result<AirData, AeroError> {
    let vel = check_bounded_vec3(velocity_world_mps)?;
    let wind = check_bounded_vec3(wind_world_mps)?;
    if !orientation.quat_is_finite() {
        return Err(AeroError::NonFiniteInput);
    }
    atmosphere.isa_validate()?;

    let q_norm = orientation.quat_normalize()?;
    let q_inv = q_norm.conjugate()?;
    let v_air_world = vel.vec3_sub(wind)?;
    let v_body = check_bounded_vec3(q_inv.rotate_vec3(v_air_world)?)?;

    // Aerospace (u, v, w) components in (Forward=+X, Up=+Y, Right=+Z) body frame.
    let u = v_body.x;
    let v = v_body.z;
    let w = -v_body.y;

    let speed_sq = u * u + v * v + w * w;
    if !speed_sq.is_finite() {
        return Err(AeroError::NumericalDivergence);
    }
    let true_airspeed_mps = if speed_sq == 0.0 {
        0.0
    } else {
        trig::sqrt(speed_sq)
    };
    if !true_airspeed_mps.is_finite() {
        return Err(AeroError::NumericalDivergence);
    }

    if true_airspeed_mps < MIN_AIRSPEED {
        return Ok(AirData {
            true_airspeed_mps,
            body_air_velocity_mps: v_body,
            alpha_rad: 0.0,
            beta_rad: 0.0,
            dynamic_pressure_pa: 0.0,
            mach: 0.0,
            low_airspeed_zeroed: true,
        });
    }

    let alpha_rad = canon_zero(trig::atan2(w, u));
    let ratio = v / true_airspeed_mps;
    let beta_rad = canon_zero(det_math::det_asin_clamped(ratio));
    let dynamic_pressure_pa = 0.5 * atmosphere.density_kg_m3 * speed_sq;
    let mach = true_airspeed_mps / atmosphere.speed_of_sound_mps;

    if !alpha_rad.is_finite()
        || !beta_rad.is_finite()
        || !dynamic_pressure_pa.is_finite()
        || !mach.is_finite()
        || dynamic_pressure_pa > MAX_COORD
    {
        return Err(AeroError::NumericalDivergence);
    }

    Ok(AirData {
        true_airspeed_mps,
        body_air_velocity_mps: v_body,
        alpha_rad,
        beta_rad,
        dynamic_pressure_pa,
        mach,
        low_airspeed_zeroed: false,
    })
}

/// Atmospheric and gravitational environment parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlightEnvironment {
    /// Constant ambient wind velocity in world coordinates (`m/s`).
    pub wind_world_mps: Vec3,
    /// Gravitational acceleration magnitude along `-Y_world` (`m/s^2`, `>= 0`).
    pub gravity_m_s2: f64,
    /// Altitude offset corresponding to `position.y == 0` in meters.
    pub ground_elevation_m: f64,
}

impl FlightEnvironment {
    /// Construct a validated [`FlightEnvironment`].
    pub fn env_new(
        wind_world_mps: Vec3,
        gravity_m_s2: f64,
        ground_elevation_m: f64,
    ) -> Result<Self, AeroError> {
        let wind = check_bounded_vec3(wind_world_mps)?;
        let g = check_finite(gravity_m_s2)?;
        let elev = check_finite(ground_elevation_m)?;
        if !(0.0..=MAX_GRAVITY_M_S2).contains(&g) {
            return Err(AeroError::InvalidAeroParameter);
        }
        if !(MIN_ALTITUDE_M..=MAX_ALTITUDE_M).contains(&elev) {
            return Err(AeroError::AltitudeOutOfRange);
        }
        Ok(Self {
            wind_world_mps: wind,
            gravity_m_s2: g,
            ground_elevation_m: elev,
        })
    }

    /// Standard sea-level calm ISA environment (`wind = 0`, `g = 9.80665 m/s^2`, `elev = 0 m`).
    #[inline]
    pub const fn env_isa_calm() -> Self {
        Self {
            wind_world_mps: Vec3::zero(),
            gravity_m_s2: ISA_GRAVITY_M_S2,
            ground_elevation_m: 0.0,
        }
    }

    /// Re-validate all environment invariants.
    pub fn env_validate(&self) -> Result<(), AeroError> {
        let _ = Self::env_new(
            self.wind_world_mps,
            self.gravity_m_s2,
            self.ground_elevation_m,
        )?;
        Ok(())
    }
}

/// Pilot / autopilot control surface deflections and throttle command.
///
/// Finite out-of-range commands are clamped to the aircraft's configured
/// [`ControlLimits`] (and throttle to `[0, 1]`), while `NaN` and `+/-Inf`
/// are rejected with [`AeroError::NonFiniteInput`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlInputs {
    /// Elevator deflection in radians (trailing edge down positive when `cm_de < 0`).
    pub elevator_rad: f64,
    /// Aileron deflection in radians (positive rolls right wing down when `cl_da > 0`).
    pub aileron_rad: f64,
    /// Rudder deflection in radians (positive yaws nose starboard when `cn_dr > 0`).
    pub rudder_rad: f64,
    /// Normalized engine throttle command in `[0.0, 1.0]`.
    pub throttle: f64,
}

impl ControlInputs {
    /// Construct validated finite [`ControlInputs`], rejecting `NaN` / `+/-Infinity`.
    pub fn controls_new(
        elevator_rad: f64,
        aileron_rad: f64,
        rudder_rad: f64,
        throttle: f64,
    ) -> Result<Self, AeroError> {
        Ok(Self {
            elevator_rad: check_finite(elevator_rad)?,
            aileron_rad: check_finite(aileron_rad)?,
            rudder_rad: check_finite(rudder_rad)?,
            throttle: check_finite(throttle)?,
        })
    }

    /// Neutral zeroed control deflections and idle throttle.
    #[inline]
    pub const fn controls_neutral() -> Self {
        Self {
            elevator_rad: 0.0,
            aileron_rad: 0.0,
            rudder_rad: 0.0,
            throttle: 0.0,
        }
    }

    /// Validate that all control inputs are finite.
    pub fn controls_validate(&self) -> Result<(), AeroError> {
        let _ = check_finite(self.elevator_rad)?;
        let _ = check_finite(self.aileron_rad)?;
        let _ = check_finite(self.rudder_rad)?;
        let _ = check_finite(self.throttle)?;
        Ok(())
    }
}

/// Symmetric angular deflection limits in radians for each control surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlLimits {
    /// Maximum absolute elevator deflection in radians (`[0, pi]`).
    pub elevator_max_rad: f64,
    /// Maximum absolute aileron deflection in radians (`[0, pi]`).
    pub aileron_max_rad: f64,
    /// Maximum absolute rudder deflection in radians (`[0, pi]`).
    pub rudder_max_rad: f64,
}

impl ControlLimits {
    /// Construct validated [`ControlLimits`], requiring finite values in `[0.0, trig::PI]`.
    pub fn control_limits_new(
        elevator_max_rad: f64,
        aileron_max_rad: f64,
        rudder_max_rad: f64,
    ) -> Result<Self, AeroError> {
        let e = check_finite(elevator_max_rad)?;
        let a = check_finite(aileron_max_rad)?;
        let r = check_finite(rudder_max_rad)?;
        if !(0.0..=trig::PI).contains(&e)
            || !(0.0..=trig::PI).contains(&a)
            || !(0.0..=trig::PI).contains(&r)
        {
            return Err(AeroError::InvalidGeometryParameter);
        }
        Ok(Self {
            elevator_max_rad: e,
            aileron_max_rad: a,
            rudder_max_rad: r,
        })
    }
}

/// Dimensionless aerodynamic stability, damping, and control-surface derivatives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AeroDerivatives {
    /// Zero-lift parasitic drag coefficient `C_D0 >= 0`.
    pub cd0: f64,
    /// Induced drag factor `k >= 0` in `C_D = C_D0 + k * C_L^2`.
    pub induced_drag_k: f64,
    /// Side-force sideslip derivative `C_{Y,beta}` (`1/rad`).
    pub cy_beta: f64,
    /// Dihedral roll-moment sideslip derivative `C_{l,beta}` (`1/rad`).
    pub cl_beta: f64,
    /// Weathercock yaw-moment sideslip derivative `C_{n,beta}` (`1/rad`).
    pub cn_beta: f64,
    /// Pitch damping derivative `C_{m,q}` (`< 0` for positive damping).
    pub cm_q: f64,
    /// Roll damping derivative `C_{l,p}` (`< 0` for positive damping).
    pub cl_p: f64,
    /// Yaw damping derivative `C_{n,r}` (`< 0` for positive damping).
    pub cn_r: f64,
    /// Elevator lift effectiveness `C_{L,delta_e}` (`1/rad`).
    pub cl_de: f64,
    /// Elevator pitching-moment effectiveness `C_{m,delta_e}` (`1/rad`).
    pub cm_de: f64,
    /// Aileron roll-moment effectiveness `C_{l,delta_a}` (`1/rad`).
    pub cl_da: f64,
    /// Aileron adverse yaw derivative `C_{n,delta_a}` (`1/rad`).
    pub cn_da: f64,
    /// Rudder side-force derivative `C_{Y,delta_r}` (`1/rad`).
    pub cy_dr: f64,
    /// Rudder roll-moment cross derivative `C_{l,delta_r}` (`1/rad`).
    pub cl_dr: f64,
    /// Rudder yaw-moment effectiveness `C_{n,delta_r}` (`1/rad`).
    pub cn_dr: f64,
}

impl AeroDerivatives {
    /// Validate finiteness, magnitude bounds, and non-negative `cd0` / `induced_drag_k`.
    pub fn derivatives_validate(&self) -> Result<(), AeroError> {
        let cd0 = check_coeff(self.cd0)?;
        let k = check_coeff(self.induced_drag_k)?;
        if cd0 < 0.0 || k < 0.0 {
            return Err(AeroError::InvalidAeroParameter);
        }
        let _ = check_coeff(self.cy_beta)?;
        let _ = check_coeff(self.cl_beta)?;
        let _ = check_coeff(self.cn_beta)?;
        let _ = check_coeff(self.cm_q)?;
        let _ = check_coeff(self.cl_p)?;
        let _ = check_coeff(self.cn_r)?;
        let _ = check_coeff(self.cl_de)?;
        let _ = check_coeff(self.cm_de)?;
        let _ = check_coeff(self.cl_da)?;
        let _ = check_coeff(self.cn_da)?;
        let _ = check_coeff(self.cy_dr)?;
        let _ = check_coeff(self.cl_dr)?;
        let _ = check_coeff(self.cn_dr)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// AircraftModel & Evaluation Output
// ---------------------------------------------------------------------------

/// Detailed breakdown of one aerodynamic + propulsion + gravity evaluation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AeroEvalOutput {
    /// ISA atmosphere at the aircraft's altitude.
    pub atmosphere: IsaSample,
    /// Computed air-data state.
    pub air_data: AirData,
    /// Evaluated lift coefficient `C_L`.
    pub cl: f64,
    /// Evaluated drag coefficient `C_D`.
    pub cd: f64,
    /// Evaluated pitching-moment coefficient `C_m`.
    pub cm: f64,
    /// Evaluated side-force coefficient `C_Y`.
    pub cy: f64,
    /// Evaluated rolling-moment coefficient `C_l`.
    pub cl_roll: f64,
    /// Evaluated yawing-moment coefficient `C_n`.
    pub cn_yaw: f64,
    /// Control inputs after clamping deflections to [`ControlLimits`] and throttle to `[0, 1]`.
    pub clamped_controls: ControlInputs,
    /// Applied engine thrust along `+X_body` in Newtons.
    pub thrust_n: f64,
    /// Total aerodynamic + thrust force in body frame `(N)`.
    pub body_aero_force_n: Vec3,
    /// Total aerodynamic torque in body frame `(N*m)`.
    pub body_aero_torque_nm: Vec3,
    /// Total world-space force (aerodynamics + thrust + gravity) `(N)`.
    pub world_total_force_n: Vec3,
    /// Total world-space aerodynamic torque `(N*m)`.
    pub world_total_torque_nm: Vec3,
}

impl AeroEvalOutput {
    /// Compute a canonical 64-bit FNV-1a digest (`-0.0` normalized to `+0.0`).
    pub fn aero_eval_digest_fnv1a(&self) -> Result<u64, AeroError> {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let scalars = [
            self.atmosphere.altitude_m,
            self.atmosphere.temperature_k,
            self.atmosphere.pressure_pa,
            self.atmosphere.density_kg_m3,
            self.atmosphere.speed_of_sound_mps,
            self.air_data.true_airspeed_mps,
            self.air_data.alpha_rad,
            self.air_data.beta_rad,
            self.air_data.dynamic_pressure_pa,
            self.air_data.mach,
            self.cl,
            self.cd,
            self.cm,
            self.cy,
            self.cl_roll,
            self.cn_yaw,
            self.thrust_n,
            self.body_aero_force_n.x,
            self.body_aero_force_n.y,
            self.body_aero_force_n.z,
            self.body_aero_torque_nm.x,
            self.body_aero_torque_nm.y,
            self.body_aero_torque_nm.z,
            self.world_total_force_n.x,
            self.world_total_force_n.y,
            self.world_total_force_n.z,
            self.world_total_torque_nm.x,
            self.world_total_torque_nm.y,
            self.world_total_torque_nm.z,
        ];
        for s in scalars {
            if !s.is_finite() {
                return Err(AeroError::NonFiniteInput);
            }
            fnv1a_push_f64(&mut hash, s);
        }
        Ok(hash)
    }
}

/// Complete 6-DOF fixed-wing aerodynamic and propulsion model.
///
/// Stack-allocated `Copy` value; zero heap allocations in construction or
/// evaluation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AircraftModel {
    wing_area_m2: f64,
    wing_span_m: f64,
    mean_chord_m: f64,
    max_thrust_n: f64,
    cl_alpha: AeroTable,
    cm_alpha: AeroTable,
    cd_alpha: Option<AeroTable>,
    derivatives: AeroDerivatives,
    limits: ControlLimits,
}

impl AircraftModel {
    /// Construct a validated [`AircraftModel`].
    ///
    /// Enforces strictly positive reference geometry `(S, b, c_bar)` with
    /// scale-relative aspect consistency (`CWE-369`), bounded thrust and
    /// derivatives (`CWE-20`), and monotonic bounded polar tables (`CWE-400`).
    #[allow(clippy::too_many_arguments)]
    pub fn aircraft_new(
        wing_area_m2: f64,
        wing_span_m: f64,
        mean_chord_m: f64,
        max_thrust_n: f64,
        cl_alpha: AeroTable,
        cm_alpha: AeroTable,
        cd_alpha: Option<AeroTable>,
        derivatives: AeroDerivatives,
        limits: ControlLimits,
    ) -> Result<Self, AeroError> {
        let s = check_finite(wing_area_m2)?;
        let b = check_finite(wing_span_m)?;
        let c = check_finite(mean_chord_m)?;
        let t = check_finite(max_thrust_n)?;

        if !(MIN_AREA_M2..=MAX_AREA_M2).contains(&s)
            || !(MIN_LENGTH_M..=MAX_LENGTH_M).contains(&b)
            || !(MIN_LENGTH_M..=MAX_LENGTH_M).contains(&c)
        {
            return Err(AeroError::InvalidGeometryParameter);
        }

        // Scale-relative planform consistency: prevent extreme `b*c / S` ratios
        // (e.g. `S = 100 m^2` paired with `c = 1e-6 m`).
        let planform = b * c;
        if !planform.is_finite() || planform < 1e-6 * s || planform > 1e6 * s {
            return Err(AeroError::InvalidGeometryParameter);
        }

        if !(0.0..=MAX_THRUST_N).contains(&t) {
            return Err(AeroError::InvalidAeroParameter);
        }

        cl_alpha.aero_table_validate()?;
        cm_alpha.aero_table_validate()?;
        if let Some(ref cd_tbl) = cd_alpha {
            cd_tbl.aero_table_validate()?;
        }
        derivatives.derivatives_validate()?;
        let _ = ControlLimits::control_limits_new(
            limits.elevator_max_rad,
            limits.aileron_max_rad,
            limits.rudder_max_rad,
        )?;

        Ok(Self {
            wing_area_m2: s,
            wing_span_m: b,
            mean_chord_m: c,
            max_thrust_n: t,
            cl_alpha,
            cm_alpha,
            cd_alpha,
            derivatives,
            limits,
        })
    }

    /// Re-validate all geometry, polar table, derivative, and limit invariants.
    pub fn aircraft_validate(&self) -> Result<(), AeroError> {
        let _ = Self::aircraft_new(
            self.wing_area_m2,
            self.wing_span_m,
            self.mean_chord_m,
            self.max_thrust_n,
            self.cl_alpha,
            self.cm_alpha,
            self.cd_alpha,
            self.derivatives,
            self.limits,
        )?;
        Ok(())
    }

    /// Construct a general-aviation trainer preset (Cessna-172-inspired).
    ///
    /// Includes realistic post-stall `C_L(alpha)` drop beyond `15 deg` (`0.26 rad`),
    /// statically stable `C_m(alpha)` slope, parabolic induced drag, and damped
    /// roll/pitch/yaw axes. Pairs naturally with a `1000 kg` rigid body.
    pub fn aircraft_preset_trainer() -> Result<Self, AeroError> {
        // Angle of attack breakpoints in radians: [-180°, ..., -18°, -12°, 0°, 12°, 15°, 18°, 25°, 45°, 90°, 180°]
        let alpha_bp = [
            -trig::PI,
            -trig::PI / 6.0,
            -0.3142,
            -0.2618,
            -0.2094,
            0.0,
            0.0873,
            0.2094,
            0.2618,
            0.3142,
            0.4363,
            trig::PI / 4.0,
            trig::PI / 2.0,
            trig::PI,
        ];
        // Realistic airfoil lift curve with stall peak at ~15 deg (CL_max = 1.48)
        let cl_vals = [
            0.0, -0.55, -0.82, -1.05, -0.88, 0.25, 0.70, 1.32, 1.48, 1.24, 0.92, 0.72, 0.10, 0.0,
        ];
        // Statically stable pitching moment curve (negative slope ~ -0.65 / rad, zero-alpha Cm0 = +0.04)
        let cm_vals = [
            0.0, 0.35, 0.24, 0.21, 0.176, 0.04, -0.017, -0.096, -0.14, -0.20, -0.32, -0.55, -0.80,
            0.0,
        ];
        // Extra post-stall pressure drag increment
        let cd_stall_vals = [
            0.80, 0.45, 0.18, 0.08, 0.02, 0.0, 0.0, 0.01, 0.04, 0.14, 0.35, 0.75, 1.20, 0.80,
        ];

        let cl_alpha = AeroTable::aero_table_new(&alpha_bp, &cl_vals)?;
        let cm_alpha = AeroTable::aero_table_new(&alpha_bp, &cm_vals)?;
        let cd_alpha = Some(AeroTable::aero_table_new(&alpha_bp, &cd_stall_vals)?);

        let derivatives = AeroDerivatives {
            cd0: 0.027,
            induced_drag_k: 0.054,
            cy_beta: -0.31,
            cl_beta: -0.089,
            cn_beta: 0.065,
            cm_q: -12.4,
            cl_p: -0.48,
            cn_r: -0.15,
            cl_de: 0.43,
            cm_de: -1.12,
            cl_da: 0.17,
            cn_da: -0.015,
            cy_dr: 0.187,
            cl_dr: 0.014,
            cn_dr: -0.065,
        };

        let limits = ControlLimits::control_limits_new(0.45, 0.38, 0.42)?;

        Self::aircraft_new(
            16.2,
            11.0,
            1.49,
            2800.0,
            cl_alpha,
            cm_alpha,
            cd_alpha,
            derivatives,
            limits,
        )
    }

    /// Reference wing area `S` in `m^2`.
    #[inline]
    pub const fn aircraft_wing_area_m2(&self) -> f64 {
        self.wing_area_m2
    }

    /// Reference wing span `b` in `m`.
    #[inline]
    pub const fn aircraft_wing_span_m(&self) -> f64 {
        self.wing_span_m
    }

    /// Mean aerodynamic chord `c_bar` in `m`.
    #[inline]
    pub const fn aircraft_mean_chord_m(&self) -> f64 {
        self.mean_chord_m
    }

    /// Maximum engine thrust in Newtons.
    #[inline]
    pub const fn aircraft_max_thrust_n(&self) -> f64 {
        self.max_thrust_n
    }

    /// Lift polar table `C_L(alpha)`.
    #[inline]
    pub const fn aircraft_cl_table(&self) -> &AeroTable {
        &self.cl_alpha
    }

    /// Pitching moment polar table `C_m(alpha)`.
    #[inline]
    pub const fn aircraft_cm_table(&self) -> &AeroTable {
        &self.cm_alpha
    }

    /// Optional stall/parasitic drag increment table `C_{D,alpha}(alpha)`.
    #[inline]
    pub const fn aircraft_cd_table(&self) -> Option<&AeroTable> {
        self.cd_alpha.as_ref()
    }

    /// Aerodynamic derivative struct.
    #[inline]
    pub const fn aircraft_derivatives(&self) -> &AeroDerivatives {
        &self.derivatives
    }

    /// Symmetric control surface limits.
    #[inline]
    pub const fn aircraft_control_limits(&self) -> &ControlLimits {
        &self.limits
    }

    /// Evaluate the full aerodynamic, thrust, and gravitational wrench on `body`.
    ///
    /// Zero heap allocations. Returns [`AeroEvalOutput`] with all intermediate
    /// atmospheric, air-data, coefficient, body-wrench, and world-wrench fields.
    pub fn aircraft_evaluate(
        &self,
        body: &RigidBodyState,
        mass: &MassProperties,
        env: &FlightEnvironment,
        controls: &ControlInputs,
    ) -> Result<AeroEvalOutput, AeroError> {
        self.aircraft_validate()?;
        body.body_validate()?;
        mass.mass_props_validate()?;
        env.env_validate()?;
        controls.controls_validate()?;

        let altitude_m = check_finite(body.position.y + env.ground_elevation_m)?;
        let atmosphere = isa_atmosphere(altitude_m)?;

        let vel_world = body.linear_velocity(mass)?;
        let omega_body = body.angular_velocity_body(mass)?;
        let air_data =
            compute_air_data(vel_world, env.wind_world_mps, body.orientation, &atmosphere)?;

        let clamped_controls = ControlInputs {
            elevator_rad: clamp_sym(controls.elevator_rad, self.limits.elevator_max_rad),
            aileron_rad: clamp_sym(controls.aileron_rad, self.limits.aileron_max_rad),
            rudder_rad: clamp_sym(controls.rudder_rad, self.limits.rudder_max_rad),
            throttle: if controls.throttle < 0.0 {
                0.0
            } else if controls.throttle > 1.0 {
                1.0
            } else {
                canon_zero(controls.throttle)
            },
        };

        let thrust_n = canon_zero(self.max_thrust_n * clamped_controls.throttle);
        if !thrust_n.is_finite() || thrust_n > MAX_COORD {
            return Err(AeroError::NumericalDivergence);
        }

        let (cl, cd, cm, cy, cl_roll, cn_yaw, body_aero_force_n, body_aero_torque_nm) =
            if air_data.low_airspeed_zeroed {
                (
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    Vec3::new(thrust_n, 0.0, 0.0),
                    Vec3::zero(),
                )
            } else {
                let alpha = air_data.alpha_rad;
                let beta = air_data.beta_rad;
                let v = air_data.true_airspeed_mps;
                let q_bar = air_data.dynamic_pressure_pa;
                let de = clamped_controls.elevator_rad;
                let da = clamped_controls.aileron_rad;
                let dr = clamped_controls.rudder_rad;

                // Non-dimensional angular rates: p (roll +X), q (pitch +Z), r (yaw -Y)
                let inv_2v = 0.5 / v;
                let p_hat = omega_body.x * self.wing_span_m * inv_2v;
                let q_hat = omega_body.z * self.mean_chord_m * inv_2v;
                let r_hat = (-omega_body.y) * self.wing_span_m * inv_2v;

                let d = &self.derivatives;
                let cl_base = self.cl_alpha.aero_table_eval(alpha)?;
                let cl = cl_base + d.cl_de * de;

                let cd_extra = match &self.cd_alpha {
                    Some(tbl) => tbl.aero_table_eval(alpha)?,
                    None => 0.0,
                };
                let cd_raw = d.cd0 + d.induced_drag_k * cl * cl + cd_extra;
                let cd = if cd_raw < 0.0 { 0.0 } else { cd_raw };

                let cm_base = self.cm_alpha.aero_table_eval(alpha)?;
                let cm = cm_base + d.cm_q * q_hat + d.cm_de * de;

                let cy = d.cy_beta * beta + d.cy_dr * dr;
                let cl_roll = d.cl_beta * beta + d.cl_p * p_hat + d.cl_da * da + d.cl_dr * dr;
                let cn_yaw = d.cn_beta * beta + d.cn_r * r_hat + d.cn_da * da + d.cn_dr * dr;

                let q_s = q_bar * self.wing_area_m2;
                let lift = q_s * cl;
                let drag = q_s * cd;
                let side = q_s * cy;

                let sin_a = trig::sin(alpha);
                let cos_a = trig::cos(alpha);
                let fx_body = -drag * cos_a + lift * sin_a + thrust_n;
                let fy_body = drag * sin_a + lift * cos_a;
                let fz_body = side;

                let tau_x = q_s * self.wing_span_m * cl_roll;
                let tau_z = q_s * self.mean_chord_m * cm;
                let tau_y = -q_s * self.wing_span_m * cn_yaw;

                (
                    canon_zero(cl),
                    canon_zero(cd),
                    canon_zero(cm),
                    canon_zero(cy),
                    canon_zero(cl_roll),
                    canon_zero(cn_yaw),
                    check_result_vec3(Vec3::new(fx_body, fy_body, fz_body))?,
                    check_result_vec3(Vec3::new(tau_x, tau_y, tau_z))?,
                )
            };

        let q_norm = body.orientation.quat_normalize()?;
        let world_aero_force = check_result_vec3(q_norm.rotate_vec3(body_aero_force_n)?)?;
        let world_total_torque_nm = check_result_vec3(q_norm.rotate_vec3(body_aero_torque_nm)?)?;
        let gravity_world = Vec3::new(0.0, -mass.mass_props_mass() * env.gravity_m_s2, 0.0);
        let world_total_force_n = check_result_vec3(world_aero_force.vec3_add(gravity_world)?)?;

        Ok(AeroEvalOutput {
            atmosphere,
            air_data,
            cl,
            cd,
            cm,
            cy,
            cl_roll,
            cn_yaw,
            clamped_controls,
            thrust_n,
            body_aero_force_n,
            body_aero_torque_nm,
            world_total_force_n,
            world_total_torque_nm,
        })
    }

    /// Compute `(world_total_force, world_total_torque)` for `rigidbody-6dof`'s
    /// `step_with_force_fn_in_place` closure. Zero heap allocations.
    #[inline]
    pub fn compute_world_wrench(
        &self,
        body: &RigidBodyState,
        mass: &MassProperties,
        env: &FlightEnvironment,
        controls: &ControlInputs,
    ) -> Result<(Vec3, Vec3), AeroError> {
        let out = self.aircraft_evaluate(body, mass, env, controls)?;
        Ok((out.world_total_force_n, out.world_total_torque_nm))
    }

    /// Advance `body` by `dt` seconds under aerodynamic, thrust, and gravity forces
    /// using `rigidbody-6dof`'s `step_with_force_fn_in_place`.
    ///
    /// Zero heap allocation and strictly atomic: on any validation error,
    /// altitude breach, or numerical divergence, `*body` is left completely
    /// unmodified and the exact [`AeroError`] is returned.
    pub fn step_aircraft_in_place(
        &self,
        body: &mut RigidBodyState,
        mass: &MassProperties,
        env: &FlightEnvironment,
        controls: &ControlInputs,
        dt: f64,
        integrator: IntegratorKind,
    ) -> Result<(), AeroError> {
        self.aircraft_validate()?;
        body.body_validate()?;
        mass.mass_props_validate()?;
        env.env_validate()?;
        controls.controls_validate()?;

        let mut captured_err: Option<AeroError> = None;
        let rb_res = body.step_with_force_fn_in_place(mass, dt, integrator, |state| {
            match self.compute_world_wrench(state, mass, env, controls) {
                Ok(wrench) => Ok(wrench),
                Err(e) => {
                    if captured_err.is_none() {
                        captured_err = Some(e);
                    }
                    Err(match e {
                        AeroError::NonFiniteInput => RigidBodyError::NonFiniteInput,
                        AeroError::RigidBodyError(rb) => rb,
                        _ => RigidBodyError::NumericalDivergence,
                    })
                }
            }
        });

        match rb_res {
            Ok(()) => Ok(()),
            Err(rb_err) => {
                if let Some(aero_err) = captured_err {
                    Err(aero_err)
                } else {
                    Err(AeroError::from(rb_err))
                }
            }
        }
    }

    /// Solve for steady level-flight trim `(alpha_trim_rad, ControlInputs)` at
    /// `altitude_m` and `airspeed_mps` using bounded bisection (`<= MAX_TRIM_ITERS`).
    pub fn aircraft_solve_level_trim(
        &self,
        mass: &MassProperties,
        env: &FlightEnvironment,
        altitude_m: f64,
        airspeed_mps: f64,
    ) -> Result<(f64, ControlInputs), AeroError> {
        self.aircraft_validate()?;
        mass.mass_props_validate()?;
        env.env_validate()?;
        let v = check_finite(airspeed_mps)?;
        if !(MIN_AIRSPEED..=MAX_COORD).contains(&v) {
            return Err(AeroError::InvalidAeroParameter);
        }
        let atm = isa_atmosphere(altitude_m)?;
        let q_bar = 0.5 * atm.density_kg_m3 * v * v;
        let q_s = q_bar * self.wing_area_m2;
        if !q_s.is_finite() || q_s <= EPSILON {
            return Err(AeroError::InvalidAeroParameter);
        }
        let weight = mass.mass_props_mass() * env.gravity_m_s2;

        // Evaluate vertical force balance residual `L + D * tan(alpha) - W`
        // with elevator trimmed to `C_m(alpha) + C_m_de * delta_e = 0`.
        let eval_trim = |alpha: f64| -> Result<(f64, f64, f64), AeroError> {
            let cm0 = self.cm_alpha.aero_table_eval(alpha)?;
            let de_raw = if self.derivatives.cm_de.abs() > EPSILON {
                -cm0 / self.derivatives.cm_de
            } else {
                0.0
            };
            let de = clamp_sym(de_raw, self.limits.elevator_max_rad);
            let cl = self.cl_alpha.aero_table_eval(alpha)? + self.derivatives.cl_de * de;
            let cd_extra = match &self.cd_alpha {
                Some(t) => t.aero_table_eval(alpha)?,
                None => 0.0,
            };
            let cd = (self.derivatives.cd0 + self.derivatives.induced_drag_k * cl * cl + cd_extra)
                .max(0.0);
            let lift = q_s * cl;
            let drag = q_s * cd;
            let cos_a = trig::cos(alpha);
            if cos_a.abs() <= EPSILON {
                return Err(AeroError::NumericalDivergence);
            }
            let thrust = drag / cos_a;
            let vertical_net = lift + thrust * trig::sin(alpha) - weight;
            Ok((vertical_net, de, thrust))
        };

        let mut lo = -0.20_f64;
        let mut hi = 0.25_f64;
        let (f_lo, _, _) = eval_trim(lo)?;
        let (f_hi, _, _) = eval_trim(hi)?;
        if f_lo > 0.0 || f_hi < 0.0 {
            return Err(AeroError::InvalidAeroParameter);
        }

        let mut mid = 0.5 * (lo + hi);
        for _ in 0..MAX_TRIM_ITERS {
            mid = 0.5 * (lo + hi);
            let (f_mid, _, _) = eval_trim(mid)?;
            if f_mid.abs() <= 1e-9 * weight.max(1.0) {
                break;
            }
            if f_mid < 0.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }

        let (_, de_trim, thrust_trim) = eval_trim(mid)?;
        if !thrust_trim.is_finite() || thrust_trim < 0.0 || thrust_trim > self.max_thrust_n {
            return Err(AeroError::InvalidAeroParameter);
        }
        let throttle_trim = if self.max_thrust_n > EPSILON {
            (thrust_trim / self.max_thrust_n).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let controls = ControlInputs::controls_new(de_trim, 0.0, 0.0, throttle_trim)?;
        Ok((canon_zero(mid), controls))
    }
}
