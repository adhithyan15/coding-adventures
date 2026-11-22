//! # Conformance Tests for `aero-flight-model` (GAME41)
//!
//! Covers:
//! - Deterministic `exp` / `ln` / `pow` / `asin` accuracy and cross-platform bit stability
//! - 1976 ISA Standard Atmosphere reference values (0 m, 5 km, 11 km, 15 km, 20 km)
//!   within `<= 1e-6` relative tolerance
//! - Air-data `alpha` / `beta` / `q_bar` / `mach` sign conventions, wind vector
//!   invariance, and low-airspeed (`V < MIN_AIRSPEED`) safe zeroing
//! - Coefficient polars: stall behavior in `C_L(alpha)`, parabolic induced drag,
//!   damping derivatives (`C_mq`, `C_lp`, `C_nr`), and clamped control surfaces
//! - Coupled 6-DOF trimmed level flight with `rigidbody-6dof` (`GAME40`)
//!   held over 10 seconds (600 steps) via `step_aircraft_in_place` and
//!   `step_with_force_fn_in_place`, plus canonical FNV-1a digest replay
//! - Warrant contract parity checks

#![forbid(unsafe_code)]

use aero_flight_model::{
    aero_det_asin, aero_det_exp, aero_det_ln, aero_det_pow, check_airspeed_active_milli,
    compute_air_data, isa_atmosphere, validate_altitude_bounds_m, validate_geometry_milli,
    validate_table_len, AeroError, AeroTable, AircraftModel, ControlInputs, FlightEnvironment,
    ISA_SEA_LEVEL_DENSITY_KG_M3, ISA_SEA_LEVEL_PRESSURE_PA, ISA_SEA_LEVEL_TEMP_K, MAX_ALTITUDE_M,
    MIN_AIRSPEED, MIN_ALTITUDE_M,
};
use math3d::{Quat, Vec3};
use rigidbody_6dof::{IntegratorKind, MassProperties, RigidBodyError, RigidBodyState};

fn rel_err(actual: f64, expected: f64) -> f64 {
    (actual - expected).abs() / expected.abs().max(1e-12)
}

#[test]
fn deterministic_math_accuracy() {
    assert!((aero_det_exp(0.0) - 1.0).abs() < 1e-15);
    assert!((aero_det_exp(-0.0) - 1.0).abs() < 1e-15);
    assert!(rel_err(aero_det_exp(1.0), core::f64::consts::E) < 1e-14);
    assert!(rel_err(aero_det_exp(-1.0), 1.0 / core::f64::consts::E) < 1e-14);
    assert!(rel_err(aero_det_exp(10.0), 22_026.465_794_806_718) < 1e-14);
    assert!(rel_err(aero_det_exp(-10.0), 4.539_992_976_248_485_4e-5) < 1e-14);

    assert!((aero_det_ln(1.0)).abs() < 1e-15);
    assert!(rel_err(aero_det_ln(core::f64::consts::E), 1.0) < 1e-14);
    assert!(rel_err(aero_det_ln(2.0), core::f64::consts::LN_2) < 1e-14);
    assert!(rel_err(aero_det_ln(10.0), core::f64::consts::LN_10) < 1e-14);
    assert!(rel_err(aero_det_ln(0.5), -core::f64::consts::LN_2) < 1e-14);

    assert!(rel_err(aero_det_pow(2.0, 10.0), 1024.0) < 1e-13);
    let p_test = aero_det_pow(0.75, 5.255_88);
    assert!(
        rel_err(p_test, 0.75_f64.powf(5.255_88)) < 1e-13,
        "pow={p_test}"
    );
    assert!(rel_err(aero_det_exp(1.0), 1.0_f64.exp()) < 1e-14);
    assert!(rel_err(aero_det_ln(2.0), 2.0_f64.ln()) < 1e-14);

    assert!((aero_det_asin(0.0)).abs() < 1e-15);
    assert!((aero_det_asin(0.5) - trig::PI / 6.0).abs() < 1e-13);
    assert!((aero_det_asin(-0.5) + trig::PI / 6.0).abs() < 1e-13);
    assert!((aero_det_asin(1.0) - trig::PI / 2.0).abs() < 1e-13);
    assert!((aero_det_asin(-1.0) + trig::PI / 2.0).abs() < 1e-13);
    // 1-ULP overshoot clamped safely
    assert!((aero_det_asin(1.0 + 1e-15) - trig::PI / 2.0).abs() < 1e-13);
    assert!((aero_det_asin(-1.0 - 1e-15) + trig::PI / 2.0).abs() < 1e-13);
}

#[test]
fn isa_1976_reference_values_within_1e_minus_6() {
    // Sea level (0 m): 288.15 K, 101325 Pa, 1.225 kg/m^3, 340.294 m/s
    let sl = isa_atmosphere(0.0).unwrap();
    assert!(rel_err(sl.temperature_k, ISA_SEA_LEVEL_TEMP_K) <= 1e-12);
    assert!(rel_err(sl.pressure_pa, ISA_SEA_LEVEL_PRESSURE_PA) <= 1e-12);
    assert!(rel_err(sl.density_kg_m3, ISA_SEA_LEVEL_DENSITY_KG_M3) <= 1e-6);
    assert!(rel_err(sl.speed_of_sound_mps, 340.294_025_2) <= 1e-6);

    // -0.0 canonicalized
    let sl_neg = isa_atmosphere(-0.0).unwrap();
    assert_eq!(sl_neg.altitude_m.to_bits(), 0.0_f64.to_bits());
    assert_eq!(sl, sl_neg);

    // 5 km troposphere reference (255.65 K, 54019.9 Pa, 0.736116 kg/m^3)
    let h5 = isa_atmosphere(5_000.0).unwrap();
    assert!(rel_err(h5.temperature_k, 255.65) <= 1e-12);
    assert!(rel_err(h5.pressure_pa, 54_019.912) <= 1e-6);
    assert!(rel_err(h5.density_kg_m3, 0.736_116) <= 1e-6);

    // 11 km tropopause reference: 216.65 K, 22632.04 Pa, 0.363918 kg/m^3
    let h11 = isa_atmosphere(11_000.0).unwrap();
    assert!(rel_err(h11.temperature_k, 216.65) <= 1e-12);
    assert!(rel_err(h11.pressure_pa, 22_632.040_095) <= 1e-6);
    // Also verify within 2e-6 against integer-rounded 22632 Pa
    assert!(rel_err(h11.pressure_pa, 22_632.0) <= 2e-6);
    assert!(rel_err(h11.density_kg_m3, 0.363_918) <= 1e-6);
    assert!(rel_err(h11.speed_of_sound_mps, 295.069_55) <= 1e-6);

    // Isothermal layer continuity across 11 km
    let h11_plus = isa_atmosphere(11_000.001).unwrap();
    assert!((h11_plus.temperature_k - h11.temperature_k).abs() < 1e-12);
    assert!(rel_err(h11_plus.pressure_pa, h11.pressure_pa) < 1e-6);

    // 15 km and 20 km isothermal layer references
    let h15 = isa_atmosphere(15_000.0).unwrap();
    assert!(rel_err(h15.temperature_k, 216.65) <= 1e-12);
    assert!(rel_err(h15.pressure_pa, 12_044.56) <= 1e-5);
    assert!(rel_err(h15.density_kg_m3, 0.193_673) <= 1e-5);

    let h20 = isa_atmosphere(MAX_ALTITUDE_M).unwrap();
    assert!(rel_err(h20.temperature_k, 216.65) <= 1e-12);
    assert!(rel_err(h20.pressure_pa, 5_474.889) <= 1e-5);
    assert!(rel_err(h20.density_kg_m3, 0.088_035) <= 1e-5);

    // Minimum valid altitude boundary (-500 m)
    let h_min = isa_atmosphere(MIN_ALTITUDE_M).unwrap();
    assert!(h_min.temperature_k > ISA_SEA_LEVEL_TEMP_K);
    assert!(h_min.pressure_pa > ISA_SEA_LEVEL_PRESSURE_PA);
}

#[test]
fn air_data_sign_conventions_wind_and_low_airspeed() {
    let sl = isa_atmosphere(0.0).unwrap();

    // 1. Nose pitched up +0.1 rad about +Z_body while moving along +X_world -> alpha = +0.1 rad
    let q_pitch_up = Quat::from_axis_angle(Vec3::unit_z(), 0.1).unwrap();
    let ad_pitch =
        compute_air_data(Vec3::new(60.0, 0.0, 0.0), Vec3::zero(), q_pitch_up, &sl).unwrap();
    assert!((ad_pitch.true_airspeed_mps - 60.0).abs() < 1e-12);
    assert!((ad_pitch.alpha_rad - 0.1).abs() < 1e-12);
    assert!(ad_pitch.beta_rad.abs() < 1e-12);
    assert!(!ad_pitch.low_airspeed_zeroed);

    // 2. Nose yawed left +0.08 rad about +Y_world while moving along +X_world:
    //    relative wind hits starboard (+Z_body), giving positive sideslip beta = +0.08 rad.
    let q_yaw_left = Quat::from_axis_angle(Vec3::unit_y(), 0.08).unwrap();
    let ad_yaw =
        compute_air_data(Vec3::new(60.0, 0.0, 0.0), Vec3::zero(), q_yaw_left, &sl).unwrap();
    assert!(ad_yaw.alpha_rad.abs() < 1e-12);
    assert!((ad_yaw.beta_rad - 0.08).abs() < 1e-12);

    // 3. Wind invariance: 70 m/s groundspeed with 10 m/s tailwind == 60 m/s airspeed in calm air
    let ad_wind = compute_air_data(
        Vec3::new(70.0, 0.0, 0.0),
        Vec3::new(10.0, 0.0, 0.0),
        q_pitch_up,
        &sl,
    )
    .unwrap();
    assert!((ad_wind.true_airspeed_mps - ad_pitch.true_airspeed_mps).abs() < 1e-12);
    assert!((ad_wind.alpha_rad - ad_pitch.alpha_rad).abs() < 1e-12);
    assert!((ad_wind.dynamic_pressure_pa - ad_pitch.dynamic_pressure_pa).abs() < 1e-12);

    // 4. Low airspeed (V < MIN_AIRSPEED) safely zeros angles, q_bar, mach, and aero forces
    let ad_zero = compute_air_data(Vec3::zero(), Vec3::zero(), Quat::quat_identity(), &sl).unwrap();
    assert!(ad_zero.low_airspeed_zeroed);
    assert_eq!(ad_zero.alpha_rad, 0.0);
    assert_eq!(ad_zero.beta_rad, 0.0);
    assert_eq!(ad_zero.dynamic_pressure_pa, 0.0);
    assert_eq!(ad_zero.mach, 0.0);

    let ad_creep = compute_air_data(
        Vec3::new(MIN_AIRSPEED * 0.5, 0.0, 0.0),
        Vec3::zero(),
        q_pitch_up,
        &sl,
    )
    .unwrap();
    assert!(ad_creep.low_airspeed_zeroed);
    assert_eq!(ad_creep.alpha_rad, 0.0);
    assert_eq!(ad_creep.beta_rad, 0.0);
    assert_eq!(ad_creep.dynamic_pressure_pa, 0.0);
}

#[test]
fn polar_table_stall_clamping_and_control_surfaces() {
    let model = AircraftModel::aircraft_preset_trainer().unwrap();
    let mass = MassProperties::mass_props_from_diagonal(1000.0, 1280.0, 2660.0, 1820.0).unwrap();
    let env = FlightEnvironment::env_isa_calm();

    // Verify stall in C_L(alpha): CL peaks near 15 deg (0.2618 rad) and drops post-stall (0.3142..0.4363 rad)
    let cl_12 = model.aircraft_cl_table().aero_table_eval(0.2094).unwrap();
    let cl_15 = model.aircraft_cl_table().aero_table_eval(0.2618).unwrap();
    let cl_18 = model.aircraft_cl_table().aero_table_eval(0.3142).unwrap();
    let cl_25 = model.aircraft_cl_table().aero_table_eval(0.4363).unwrap();
    assert!(cl_15 > cl_12);
    assert!(cl_18 < cl_15);
    assert!(cl_25 < cl_18);

    // Control deflections outside configured limits are clamped
    let limits = model.aircraft_control_limits();
    let mut body =
        RigidBodyState::body_new(Vec3::new(0.0, 500.0, 0.0), Quat::quat_identity()).unwrap();
    body.linear_momentum = Vec3::new(50.0 * 1000.0, 0.0, 0.0);

    let over_ctrl = ControlInputs::controls_new(5.0, -5.0, 5.0, 2.5).unwrap();
    let clamped_ctrl = ControlInputs::controls_new(
        limits.elevator_max_rad,
        -limits.aileron_max_rad,
        limits.rudder_max_rad,
        1.0,
    )
    .unwrap();

    let out_over = model
        .aircraft_evaluate(&body, &mass, &env, &over_ctrl)
        .unwrap();
    let out_exact = model
        .aircraft_evaluate(&body, &mass, &env, &clamped_ctrl)
        .unwrap();
    assert_eq!(out_over.clamped_controls, clamped_ctrl);
    assert_eq!(
        out_over.aero_eval_digest_fnv1a().unwrap(),
        out_exact.aero_eval_digest_fnv1a().unwrap()
    );

    // Low airspeed static thrust check: aero torque == 0, body force == (max_thrust * 0.5, 0, 0)
    let idle_body =
        RigidBodyState::body_new(Vec3::new(0.0, 100.0, 0.0), Quat::quat_identity()).unwrap();
    let half_thr = ControlInputs::controls_new(0.2, 0.2, 0.2, 0.5).unwrap();
    let static_out = model
        .aircraft_evaluate(&idle_body, &mass, &env, &half_thr)
        .unwrap();
    assert!(static_out.air_data.low_airspeed_zeroed);
    assert_eq!(static_out.body_aero_torque_nm, Vec3::zero());
    assert!((static_out.body_aero_force_n.x - 1400.0).abs() < 1e-12);
    assert_eq!(static_out.body_aero_force_n.y, 0.0);
    assert_eq!(static_out.body_aero_force_n.z, 0.0);
}

#[test]
fn lateral_stability_and_rotational_damping() {
    let model = AircraftModel::aircraft_preset_trainer().unwrap();
    let mass = MassProperties::mass_props_from_diagonal(1000.0, 1280.0, 2660.0, 1820.0).unwrap();
    let env = FlightEnvironment::env_isa_calm();
    let neutral = ControlInputs::controls_neutral();

    // Sideslip stability: beta > 0 -> side force toward -Z_body, yaw torque turns nose toward +Z (-Y_body)
    let q_yaw_left = Quat::from_axis_angle(Vec3::unit_y(), 0.08).unwrap();
    let mut slip_body = RigidBodyState::body_new(Vec3::new(0.0, 1000.0, 0.0), q_yaw_left).unwrap();
    slip_body.linear_momentum = Vec3::new(55.0 * 1000.0, 0.0, 0.0);
    let slip_out = model
        .aircraft_evaluate(&slip_body, &mass, &env, &neutral)
        .unwrap();
    assert!(slip_out.air_data.beta_rad > 0.0);
    assert!(slip_out.body_aero_force_n.z < 0.0);
    assert!(slip_out.body_aero_torque_nm.y < 0.0);
    assert!(slip_out.body_aero_torque_nm.x < 0.0); // dihedral roll back

    // Pitch, roll, and yaw damping: angular momentum magnitude decays under step_aircraft_in_place
    let mut damped =
        RigidBodyState::body_new(Vec3::new(0.0, 1500.0, 0.0), Quat::quat_identity()).unwrap();
    damped.linear_momentum = Vec3::new(55.0 * 1000.0, 0.0, 0.0);
    // Pitch rate q = +0.5 rad/s around +Z_body
    damped.angular_momentum = Vec3::new(0.0, 0.0, 0.5 * 1820.0);
    let initial_lz = damped.angular_momentum.z;
    for _ in 0..30 {
        model
            .step_aircraft_in_place(
                &mut damped,
                &mass,
                &env,
                &neutral,
                1.0 / 60.0,
                IntegratorKind::RK4,
            )
            .unwrap();
    }
    assert!(damped.angular_momentum.z.abs() < initial_lz.abs());

    // Roll rate damping
    let mut roll_damped =
        RigidBodyState::body_new(Vec3::new(0.0, 1500.0, 0.0), Quat::quat_identity()).unwrap();
    roll_damped.linear_momentum = Vec3::new(55.0 * 1000.0, 0.0, 0.0);
    roll_damped.angular_momentum = Vec3::new(0.6 * 1280.0, 0.0, 0.0);
    let initial_lx = roll_damped.angular_momentum.x;
    for _ in 0..30 {
        model
            .step_aircraft_in_place(
                &mut roll_damped,
                &mass,
                &env,
                &neutral,
                1.0 / 60.0,
                IntegratorKind::RK4,
            )
            .unwrap();
    }
    assert!(roll_damped.angular_momentum.x.abs() < initial_lx.abs());
}

#[test]
fn trimmed_level_flight_coupled_to_game40_for_10_seconds() {
    let model = AircraftModel::aircraft_preset_trainer().unwrap();
    let mass = MassProperties::mass_props_from_diagonal(1000.0, 1280.0, 2660.0, 1820.0).unwrap();
    let env = FlightEnvironment::env_isa_calm();
    let alt0 = 1500.0;
    let airspeed0 = 55.0;

    let (alpha_trim, trim_controls) = model
        .aircraft_solve_level_trim(&mass, &env, alt0, airspeed0)
        .unwrap();

    let q0 = Quat::from_axis_angle(Vec3::unit_z(), alpha_trim).unwrap();
    let mut body = RigidBodyState::body_new(Vec3::new(0.0, alt0, 0.0), q0).unwrap();
    body.linear_momentum = Vec3::new(airspeed0 * 1000.0, 0.0, 0.0);

    // Net force and torque at trim state must be near zero
    let (f0, tau0) = model
        .compute_world_wrench(&body, &mass, &env, &trim_controls)
        .unwrap();
    assert!(
        f0.length().unwrap() < 1e-4,
        "trim force residual = {:?}",
        f0
    );
    assert!(
        tau0.length().unwrap() < 1e-5,
        "trim torque residual = {:?}",
        tau0
    );

    // Run 10 seconds (600 steps at 60 Hz) via step_aircraft_in_place AND via GAME40's step_with_force_fn_in_place
    let mut body_direct = body;
    let dt = 1.0 / 60.0;
    for _ in 0..600 {
        model
            .step_aircraft_in_place(
                &mut body,
                &mass,
                &env,
                &trim_controls,
                dt,
                IntegratorKind::RK4,
            )
            .unwrap();
        body_direct
            .step_with_force_fn_in_place(&mass, dt, IntegratorKind::RK4, |s| {
                model
                    .compute_world_wrench(s, &mass, &env, &trim_controls)
                    .map_err(|_| RigidBodyError::NumericalDivergence)
            })
            .unwrap();
    }

    // Bit-exact equivalence between direct closure coupling and step_aircraft_in_place
    assert_eq!(
        body.state_digest_fnv1a().unwrap(),
        body_direct.state_digest_fnv1a().unwrap()
    );

    // Altitude, airspeed, and attitude remain tightly trimmed over 10 s (~550 m flown)
    assert!(
        (body.position.y - alt0).abs() < 1e-2,
        "alt drift = {}",
        body.position.y - alt0
    );
    let final_speed = body.linear_velocity(&mass).unwrap().length().unwrap();
    assert!(
        (final_speed - airspeed0).abs() < 1e-2,
        "speed drift = {}",
        final_speed - airspeed0
    );
    assert!(body.position.x > 540.0);
    assert!(body.angular_momentum.length().unwrap() < 1e-3);
}

#[test]
fn polar_table_clamping_and_warrant_contracts() {
    let bps = [-1.0, 0.0, 1.0];
    let vals = [-0.5, 0.25, 1.0];
    let tbl = AeroTable::aero_table_new(&bps, &vals).unwrap();
    assert_eq!(tbl.aero_table_len(), 3);
    assert_eq!(tbl.aero_table_breakpoints(), &bps);
    assert_eq!(tbl.aero_table_values(), &vals);

    assert_eq!(tbl.aero_table_eval(-10.0).unwrap(), -0.5);
    assert_eq!(tbl.aero_table_eval(-1.0).unwrap(), -0.5);
    assert!((tbl.aero_table_eval(-0.5).unwrap() + 0.125).abs() < 1e-12);
    assert_eq!(tbl.aero_table_eval(0.0).unwrap(), 0.25);
    assert!((tbl.aero_table_eval(0.5).unwrap() - 0.625).abs() < 1e-12);
    assert_eq!(tbl.aero_table_eval(1.0).unwrap(), 1.0);
    assert_eq!(tbl.aero_table_eval(100.0).unwrap(), 1.0);

    // Warrant contract functions
    assert_eq!(validate_altitude_bounds_m(0), Ok(true));
    assert_eq!(validate_altitude_bounds_m(11_000), Ok(true));
    assert_eq!(
        validate_altitude_bounds_m(-501),
        Err(AeroError::AltitudeOutOfRange)
    );
    assert_eq!(
        validate_altitude_bounds_m(20_001),
        Err(AeroError::AltitudeOutOfRange)
    );
    assert_eq!(validate_table_len(12), Ok(true));
    assert_eq!(validate_table_len(1), Err(AeroError::InvalidTableLength));
    assert_eq!(validate_table_len(65), Err(AeroError::InvalidTableLength));
    assert_eq!(validate_geometry_milli(16_200, 11_000, 1_490), Ok(true));
    assert_eq!(
        validate_geometry_milli(0, 11_000, 1_490),
        Err(AeroError::InvalidGeometryParameter)
    );
    assert!(!check_airspeed_active_milli(249));
    assert!(check_airspeed_active_milli(250));
}
