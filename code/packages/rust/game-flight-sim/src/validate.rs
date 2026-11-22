//! # Input, State, and Warrant Boundary Validators
//!
//! Pure validation predicates and canonical FNV-1a digest helpers for
//! `game-flight-sim`. Every boundary check mirrors the formal contract in
//! `code/specs/game-flight-sim.warrant` (`VER04j` / `VER04m`) and executes
//! **before** any state or output buffer mutation so failures remain
//! byte-for-byte atomic.

use math3d::{Mat4, Vec3};
use render3d_gpu_bridge::{MAX_FRAMEBUFFER_HEIGHT, MAX_FRAMEBUFFER_WIDTH, MAX_MATRIX_ENTRY};
use terrain_lod::{Heightfield, RunwaySpec};

use crate::{
    FlightInputFrame, FlightSimConfig, FlightSimError, BLACK_BOX_FRAME_BYTES,
    BLACK_BOX_HEADER_BYTES, BLACK_BOX_TRAILER_BYTES, MAX_ALTITUDE_ENVELOPE_M, MAX_BLACK_BOX_FRAMES,
    MAX_FUEL_KG, MAX_GUST_MPS, MAX_STEP_DT_NS, MAX_WORLD_EXTENT_M, MIN_ALTITUDE_ENVELOPE_M,
    MIN_STEP_DT_NS,
};

/// FNV-1a 64-bit offset basis.
pub(crate) const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// FNV-1a 64-bit prime multiplier.
pub(crate) const FNV_PRIME: u64 = 0x0100_0000_01b3;

/// Normalize negative zero (`-0.0`) to `+0.0` so binary digests and encoded
/// records treat `±0.0` identically.
#[inline]
pub(crate) fn canon_zero(v: f64) -> f64 {
    if v == 0.0 {
        0.0
    } else {
        v
    }
}

/// Reject `NaN` and `±Inf`, normalizing `-0.0` to `+0.0`.
#[inline]
pub(crate) fn check_finite(v: f64) -> Result<f64, FlightSimError> {
    if !v.is_finite() {
        return Err(FlightSimError::NonFiniteInput);
    }
    Ok(canon_zero(v))
}

/// Reject `NaN`, `±Inf`, or values outside `[min_v, max_v]`.
#[inline]
pub(crate) fn check_range(v: f64, min_v: f64, max_v: f64) -> Result<f64, FlightSimError> {
    let clean = check_finite(v)?;
    if clean < min_v || clean > max_v {
        return Err(FlightSimError::ValueOutOfBounds);
    }
    Ok(clean)
}

/// Reject non-finite or out-of-envelope [`Vec3`] components.
#[inline]
pub(crate) fn check_vec3_bounded(v: Vec3, max_abs: f64) -> Result<Vec3, FlightSimError> {
    let x = check_range(v.x, -max_abs, max_abs)?;
    let y = check_range(v.y, -max_abs, max_abs)?;
    let z = check_range(v.z, -max_abs, max_abs)?;
    Ok(Vec3::new(x, y, z))
}

/// Verify every entry of a [`Mat4`] is finite and bounded by
/// `render3d_gpu_bridge::MAX_MATRIX_ENTRY`.
#[inline]
pub(crate) fn check_mat4_bridge_bounded(m: &Mat4) -> Result<(), FlightSimError> {
    for col in m.cols {
        for entry in col {
            let _ = check_range(entry, -MAX_MATRIX_ENTRY, MAX_MATRIX_ENTRY)?;
        }
    }
    Ok(())
}

/// Update an FNV-1a accumulator with raw bytes.
#[inline]
pub(crate) fn fnv1a_mix_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Update an FNV-1a accumulator with a `u64` in little-endian byte order.
#[inline]
pub(crate) fn fnv1a_mix_u64(hash: u64, v: u64) -> u64 {
    fnv1a_mix_bytes(hash, &v.to_le_bytes())
}

/// Update an FNV-1a accumulator with a canonicalized `f64` (`-0.0` -> `+0.0`).
#[inline]
pub(crate) fn fnv1a_mix_f64(hash: u64, v: f64) -> Result<u64, FlightSimError> {
    let clean = check_finite(v)?;
    Ok(fnv1a_mix_bytes(hash, &clean.to_bits().to_le_bytes()))
}

/// Update an FNV-1a accumulator with a [`Vec3`].
#[inline]
pub(crate) fn fnv1a_mix_vec3(mut hash: u64, v: Vec3) -> Result<u64, FlightSimError> {
    hash = fnv1a_mix_f64(hash, v.x)?;
    hash = fnv1a_mix_f64(hash, v.y)?;
    hash = fnv1a_mix_f64(hash, v.z)?;
    Ok(hash)
}

/// Validate black-box FDR frame count against `MAX_BLACK_BOX_FRAMES` (`65_536`).
///
/// Mirrors `validate_fdr_frame_count` in `code/specs/game-flight-sim.warrant`.
pub const fn validate_fdr_frame_count(frame_count: usize) -> Result<bool, FlightSimError> {
    if frame_count <= MAX_BLACK_BOX_FRAMES {
        Ok(true)
    } else {
        Err(FlightSimError::BlackBoxFull)
    }
}

/// Validate exact encoded FDR byte length `88 + frame_count * 152`.
///
/// Mirrors `validate_fdr_blob_bytes` in `code/specs/game-flight-sim.warrant`.
pub const fn validate_fdr_blob_bytes(
    blob_bytes: usize,
    frame_count: usize,
) -> Result<bool, FlightSimError> {
    if frame_count > MAX_BLACK_BOX_FRAMES {
        return Err(FlightSimError::BlackBoxCorrupt);
    }
    let header_plus_trailer = BLACK_BOX_HEADER_BYTES + BLACK_BOX_TRAILER_BYTES;
    let Some(frames_bytes) = frame_count.checked_mul(BLACK_BOX_FRAME_BYTES) else {
        return Err(FlightSimError::BlackBoxCorrupt);
    };
    let Some(expected) = header_plus_trailer.checked_add(frames_bytes) else {
        return Err(FlightSimError::BlackBoxCorrupt);
    };
    if blob_bytes == expected {
        Ok(true)
    } else {
        Err(FlightSimError::BlackBoxCorrupt)
    }
}

/// Validate frame timestep delta in nanoseconds (`1..=1_000_000_000`).
///
/// Mirrors `validate_dt_ns` in `code/specs/game-flight-sim.warrant`.
pub const fn validate_dt_ns(dt_ns: u64) -> Result<bool, FlightSimError> {
    if dt_ns >= MIN_STEP_DT_NS && dt_ns <= MAX_STEP_DT_NS {
        Ok(true)
    } else {
        Err(FlightSimError::InvalidClockDelta)
    }
}

/// Validate integer world altitude envelope (`-500..=20_000` meters).
///
/// Mirrors `validate_altitude_envelope_m` in `code/specs/game-flight-sim.warrant`.
pub const fn validate_altitude_envelope_m(alt_m: i64) -> Result<bool, FlightSimError> {
    if alt_m >= -500 && alt_m <= 20_000 {
        Ok(true)
    } else {
        Err(FlightSimError::ValueOutOfBounds)
    }
}

/// Validate fuel mass in milligrams (`0..=100_000_000_000`).
///
/// Mirrors `validate_fuel_mg` in `code/specs/game-flight-sim.warrant`.
pub const fn validate_fuel_mg(fuel_mg: i64) -> Result<bool, FlightSimError> {
    if fuel_mg >= 0 && fuel_mg <= 100_000_000_000 {
        Ok(true)
    } else {
        Err(FlightSimError::ValueOutOfBounds)
    }
}

/// Validate a 3D render target dimension (`1..=4096`).
///
/// Mirrors `validate_render_dim` in `code/specs/game-flight-sim.warrant`.
pub const fn validate_render_dim(dim: u32) -> Result<bool, FlightSimError> {
    if dim >= 1 && dim <= MAX_FRAMEBUFFER_WIDTH && dim <= MAX_FRAMEBUFFER_HEIGHT {
        Ok(true)
    } else {
        Err(FlightSimError::InvalidDimensions)
    }
}

/// Validate [`FlightInputFrame`] bounds before mutating simulation state.
pub fn validate_input_frame(input: &FlightInputFrame) -> Result<FlightInputFrame, FlightSimError> {
    let _ = validate_dt_ns(input.dt_ns)?;
    let elevator = check_range(input.elevator, -1.0, 1.0)?;
    let aileron = check_range(input.aileron, -1.0, 1.0)?;
    let rudder = check_range(input.rudder, -1.0, 1.0)?;
    let throttle = check_range(input.throttle, 0.0, 1.0)?;
    let flaps = check_range(input.flaps, 0.0, 1.0)?;
    let brake = check_range(input.brake, 0.0, 1.0)?;
    Ok(FlightInputFrame {
        dt_ns: input.dt_ns,
        elevator,
        aileron,
        rudder,
        throttle,
        flaps,
        brake,
        gear_down: input.gear_down,
    })
}

/// Validate [`FlightSimConfig`] parameters.
pub fn validate_sim_config(config: &FlightSimConfig) -> Result<FlightSimConfig, FlightSimError> {
    let _ = validate_dt_ns(config.fixed_dt_ns)?;
    if config.max_frame_steps == 0 || config.max_frame_steps > game_clock::HARD_MAX_FRAME_STEPS {
        return Err(FlightSimError::InvalidClockDelta);
    }
    let initial_fuel_kg = check_range(config.initial_fuel_kg, 0.0, MAX_FUEL_KG)?;
    let gust_intensity_mps = check_range(config.gust_intensity_mps, 0.0, MAX_GUST_MPS)?;
    Ok(FlightSimConfig {
        rng_seed: config.rng_seed,
        terrain_seed: config.terrain_seed,
        aircraft_preset: config.aircraft_preset,
        fixed_dt_ns: config.fixed_dt_ns,
        max_frame_steps: config.max_frame_steps,
        initial_fuel_kg,
        gust_intensity_mps,
    })
}

/// Validate a [`Heightfield`] and [`RunwaySpec`] together, including sampling at
/// `position` to ensure terrain elevation & normal are finite and inside the ISA
/// altitude envelope.
pub(crate) fn validate_terrain_and_runway(
    heightfield: &Heightfield<'_>,
    runway: &RunwaySpec,
    pos: Vec3,
) -> Result<(f64, Vec3), FlightSimError> {
    let spec = heightfield.spec();
    let _ = Heightfield::from_heights(spec, heightfield.heights())?;
    runway.runway_validate(&spec)?;
    let _ = check_range(
        runway.elevation,
        MIN_ALTITUDE_ENVELOPE_M,
        MAX_ALTITUDE_ENVELOPE_M,
    )?;
    let _ = check_vec3_bounded(pos, MAX_WORLD_EXTENT_M)?;
    let ground_y = heightfield.height_at(pos.x, pos.z)?;
    let _ = check_range(ground_y, MIN_ALTITUDE_ENVELOPE_M, MAX_ALTITUDE_ENVELOPE_M)?;
    let ground_n = heightfield.normal_at(pos.x, pos.z)?;
    let _ = check_vec3_bounded(ground_n, 2.0)?;
    Ok((ground_y, ground_n))
}

/// Return `true` if `(x, z)` lies within the paved rectangle of `runway`.
pub fn is_point_on_runway(runway: &RunwaySpec, x: f64, z: f64) -> Result<bool, FlightSimError> {
    let x = check_finite(x)?;
    let z = check_finite(z)?;
    let cx = check_finite(runway.center_x)?;
    let cz = check_finite(runway.center_z)?;
    let heading = check_finite(runway.heading_rad)?;
    let length = check_range(runway.length, 1.0, MAX_WORLD_EXTENT_M)?;
    let width = check_range(runway.width, 1.0, MAX_WORLD_EXTENT_M)?;

    let fwd_x = trig::cos(heading);
    let fwd_z = trig::sin(heading);
    let right_x = -fwd_z;
    let right_z = fwd_x;

    let dx = x - cx;
    let dz = z - cz;
    let along = (dx * fwd_x + dz * fwd_z).abs();
    let across = (dx * right_x + dz * right_z).abs();
    Ok(along <= 0.5 * length && across <= 0.5 * width)
}
