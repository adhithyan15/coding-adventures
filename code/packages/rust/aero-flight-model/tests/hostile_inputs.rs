//! # Mandatory Hostile-Input CVE Proof Suite for `aero-flight-model` (GAME41)
//!
//! Proves with explicit hostile-input test vectors and a seeded 2,000-case
//! fuzz-style campaign that applicable CVE/CWE classes cannot cause memory
//! corruption, panics, division by zero, unbounded work, or partial state
//! mutation:
//! - **CWE-20** (Improper Input Validation): `NaN`, `±Inf`, `-0.0`, subnormals, `±f64::MAX`
//! - **CWE-369** (Divide by Zero): `V = 0`, `V < MIN_AIRSPEED`, `a <= 0`, scale-relative
//!   near-zero geometry (`S`, `b`, `c_bar`), duplicate/near-duplicate polar breakpoints
//! - **CWE-400 / CWE-835** (Resource Exhaustion / Unbounded Loops): oversized/undersized/
//!   non-monotonic polar tables, out-of-range altitudes, invalid `dt`, plus **atomic state
//!   preservation** on error
//! - **CWE-125 / CWE-787 / CWE-190 / CWE-119 / CWE-415 / CWE-416 / CWE-476**:
//!   `#![forbid(unsafe_code)]`, clamped OOB table queries, checked slice indexing
//! - **2,000-case seeded hostile-input campaign** asserting zero panics and valid error returns.

#![forbid(unsafe_code)]

use std::panic::{catch_unwind, AssertUnwindSafe};

use aero_flight_model::{
    aero_det_asin, aero_det_exp, aero_det_ln, aero_det_pow, compute_air_data, isa_atmosphere,
    AeroError, AeroTable, AircraftModel, ControlInputs, ControlLimits, FlightEnvironment,
    IsaSample, MAX_ALTITUDE_M, MAX_AREA_M2, MAX_COEFF_MAG, MAX_COORD, MAX_GRAVITY_M_S2,
    MAX_LENGTH_M, MAX_TABLE_POINTS, MAX_THRUST_N, MIN_AIRSPEED, MIN_ALTITUDE_M, MIN_AREA_M2,
    MIN_LENGTH_M,
};
use math3d::{Quat, Vec3};
use rigidbody_6dof::{IntegratorKind, MassProperties, RigidBodyState, MAX_DT};

fn next_u64(state: &mut u64) -> u64 {
    let mut z = *state;
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    *state = z;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn hostile_scalar(state: &mut u64) -> f64 {
    match next_u64(state) % 16 {
        0 => f64::NAN,
        1 => f64::from_bits(0x7ff0_0000_0000_0001), // signaling NaN payload
        2 => f64::INFINITY,
        3 => f64::NEG_INFINITY,
        4 => -0.0,
        5 => 0.0,
        6 => f64::from_bits(1), // smallest positive subnormal
        7 => f64::from_bits(0x8000_0000_0000_0001), // smallest negative subnormal
        8 => f64::MIN_POSITIVE * 0.5, // subnormal
        9 => f64::MAX,
        10 => f64::MIN,
        11 => 1e15,
        12 => -1e15,
        13 => 1e-18,
        14 => -1e-18,
        _ => {
            let u = (next_u64(state) >> 11) as f64 / ((1u64 << 53) as f64);
            -50_000.0 + 100_000.0 * u
        }
    }
}

#[test]
fn cwe_20_non_finite_and_subnormal_inputs_rejected() {
    let bad_floats = [
        f64::NAN,
        f64::from_bits(0x7ff0_0000_0000_0001),
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];

    for bad in bad_floats {
        assert_eq!(isa_atmosphere(bad), Err(AeroError::NonFiniteInput));
        assert_eq!(
            AeroTable::aero_table_new(&[0.0, bad], &[0.0, 1.0]),
            Err(AeroError::NonFiniteInput)
        );
        assert_eq!(
            AeroTable::aero_table_new(&[0.0, 1.0], &[0.0, bad]),
            Err(AeroError::NonFiniteInput)
        );
        assert_eq!(
            ControlInputs::controls_new(bad, 0.0, 0.0, 0.5),
            Err(AeroError::NonFiniteInput)
        );
        assert_eq!(
            ControlInputs::controls_new(0.0, bad, 0.0, 0.5),
            Err(AeroError::NonFiniteInput)
        );
        assert_eq!(
            ControlInputs::controls_new(0.0, 0.0, bad, 0.5),
            Err(AeroError::NonFiniteInput)
        );
        assert_eq!(
            ControlInputs::controls_new(0.0, 0.0, 0.0, bad),
            Err(AeroError::NonFiniteInput)
        );
        assert_eq!(
            FlightEnvironment::env_new(Vec3::new(bad, 0.0, 0.0), 9.81, 0.0),
            Err(AeroError::NonFiniteInput)
        );
        assert_eq!(
            FlightEnvironment::env_new(Vec3::zero(), bad, 0.0),
            Err(AeroError::NonFiniteInput)
        );
        assert_eq!(
            FlightEnvironment::env_new(Vec3::zero(), 9.81, bad),
            Err(AeroError::NonFiniteInput)
        );
        assert_eq!(
            ControlLimits::control_limits_new(bad, 0.3, 0.3),
            Err(AeroError::NonFiniteInput)
        );
    }

    // Subnormals and -0.0 in atmosphere & air data never panic and return finite/canonical values
    let subnormal = f64::from_bits(1);
    let atm_sub = isa_atmosphere(subnormal).unwrap();
    assert!(atm_sub.temperature_k.is_finite() && atm_sub.pressure_pa.is_finite());

    let atm_neg_zero = isa_atmosphere(-0.0).unwrap();
    assert_eq!(atm_neg_zero.altitude_m.to_bits(), 0.0_f64.to_bits());

    let ad_sub = compute_air_data(
        Vec3::new(-subnormal, -0.0, subnormal),
        Vec3::new(-0.0, 0.0, 0.0),
        Quat::quat_identity(),
        &atm_sub,
    )
    .unwrap();
    assert!(ad_sub.low_airspeed_zeroed);
    assert_eq!(ad_sub.alpha_rad.to_bits(), 0.0_f64.to_bits());
    assert_eq!(ad_sub.beta_rad.to_bits(), 0.0_f64.to_bits());
    assert_eq!(ad_sub.dynamic_pressure_pa.to_bits(), 0.0_f64.to_bits());

    // Deterministic transcendentals on hostile floats never panic
    for &x in &[
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -0.0,
        0.0,
        subnormal,
        -subnormal,
        f64::MAX,
        f64::MIN,
    ] {
        let _ = aero_det_exp(x);
        let _ = aero_det_ln(x);
        let _ = aero_det_pow(x, 2.5);
        let _ = aero_det_asin(x);
    }
}

#[test]
fn cwe_369_divide_by_zero_and_scale_relative_geometry_guards() {
    let tbl = AeroTable::aero_table_new(&[-1.0, 1.0], &[0.0, 1.0]).unwrap();
    let derivs = *AircraftModel::aircraft_preset_trainer()
        .unwrap()
        .aircraft_derivatives();
    let limits = *AircraftModel::aircraft_preset_trainer()
        .unwrap()
        .aircraft_control_limits();

    // 1. V = 0, -0.0, subnormal, just below MIN_AIRSPEED -> zero division impossible
    let atm = isa_atmosphere(0.0).unwrap();
    for speed in [0.0, -0.0, 1e-300, MIN_AIRSPEED - 1e-9] {
        let ad = compute_air_data(
            Vec3::new(speed, 0.0, 0.0),
            Vec3::zero(),
            Quat::quat_identity(),
            &atm,
        )
        .unwrap();
        assert!(ad.low_airspeed_zeroed);
        assert_eq!(ad.alpha_rad, 0.0);
        assert_eq!(ad.beta_rad, 0.0);
        assert_eq!(ad.dynamic_pressure_pa, 0.0);
        assert_eq!(ad.mach, 0.0);
    }

    // 2. Degenerate speed of sound a <= 0 in IsaSample rejected
    let bad_atm = IsaSample {
        altitude_m: 0.0,
        temperature_k: 288.15,
        pressure_pa: 101_325.0,
        density_kg_m3: 1.225,
        speed_of_sound_mps: 0.0,
    };
    assert_eq!(
        compute_air_data(
            Vec3::new(50.0, 0.0, 0.0),
            Vec3::zero(),
            Quat::quat_identity(),
            &bad_atm
        ),
        Err(AeroError::InvalidAeroParameter)
    );

    // 3. Zero, negative, subnormal, and scale-inconsistent geometry rejected
    let subnormal = f64::from_bits(1);
    for bad_area in [
        0.0,
        -0.0,
        -10.0,
        subnormal,
        MIN_AREA_M2 * 0.5,
        MAX_AREA_M2 * 2.0,
    ] {
        assert_eq!(
            AircraftModel::aircraft_new(bad_area, 10.0, 1.0, 100.0, tbl, tbl, None, derivs, limits),
            Err(AeroError::InvalidGeometryParameter)
        );
    }
    for bad_len in [
        0.0,
        -0.0,
        -1.0,
        subnormal,
        MIN_LENGTH_M * 0.5,
        MAX_LENGTH_M * 2.0,
    ] {
        assert_eq!(
            AircraftModel::aircraft_new(10.0, bad_len, 1.0, 100.0, tbl, tbl, None, derivs, limits),
            Err(AeroError::InvalidGeometryParameter)
        );
        assert_eq!(
            AircraftModel::aircraft_new(10.0, 10.0, bad_len, 100.0, tbl, tbl, None, derivs, limits),
            Err(AeroError::InvalidGeometryParameter)
        );
    }
    // Scale-relative aspect mismatch: S = 1e6 m^2 with b = 1e-3 m, c = 1e-3 m (b*c = 1e-6 << 1e-6 * S)
    assert_eq!(
        AircraftModel::aircraft_new(1e6, 1e-3, 1e-3, 100.0, tbl, tbl, None, derivs, limits),
        Err(AeroError::InvalidGeometryParameter)
    );

    // 4. Duplicate and scale-relative near-duplicate breakpoints rejected
    assert_eq!(
        AeroTable::aero_table_new(&[1.0, 1.0], &[0.0, 1.0]),
        Err(AeroError::NonMonotonicBreakpoints)
    );
    assert_eq!(
        AeroTable::aero_table_new(&[1000.0, 1000.0 + 1e-12], &[0.0, 1.0]),
        Err(AeroError::NonMonotonicBreakpoints)
    );
}

#[test]
fn cwe_400_125_190_tables_bounds_and_atomic_state_preservation() {
    // Undersized, oversized, mismatched, and non-monotonic tables
    assert_eq!(
        AeroTable::aero_table_new(&[], &[]),
        Err(AeroError::InvalidTableLength)
    );
    assert_eq!(
        AeroTable::aero_table_new(&[0.0], &[1.0]),
        Err(AeroError::InvalidTableLength)
    );
    assert_eq!(
        AeroTable::aero_table_new(&[0.0, 1.0, 2.0], &[0.0, 1.0]),
        Err(AeroError::InvalidTableLength)
    );

    let oversized_bp: Vec<f64> = (0..=MAX_TABLE_POINTS).map(|i| i as f64).collect();
    let oversized_vals = vec![0.0; MAX_TABLE_POINTS + 1];
    assert_eq!(
        AeroTable::aero_table_new(&oversized_bp, &oversized_vals),
        Err(AeroError::InvalidTableLength)
    );

    assert_eq!(
        AeroTable::aero_table_new(&[0.0, 2.0, 1.0], &[0.0, 1.0, 2.0]),
        Err(AeroError::NonMonotonicBreakpoints)
    );
    assert_eq!(
        AeroTable::aero_table_new(&[0.0, 1.0], &[0.0, MAX_COEFF_MAG + 1.0]),
        Err(AeroError::InvalidAeroParameter)
    );

    // Out-of-bounds table queries clamp cleanly with zero OOB reads (CWE-125)
    let tbl = AeroTable::aero_table_new(&[-0.5, 0.5], &[-1.2, 1.8]).unwrap();
    assert_eq!(tbl.aero_table_eval(f64::MIN).unwrap(), -1.2);
    assert_eq!(tbl.aero_table_eval(-1e300).unwrap(), -1.2);
    assert_eq!(tbl.aero_table_eval(1e300).unwrap(), 1.8);
    assert_eq!(tbl.aero_table_eval(f64::MAX).unwrap(), 1.8);
    assert_eq!(
        tbl.aero_table_eval(f64::NAN),
        Err(AeroError::NonFiniteInput)
    );

    // Out-of-range altitudes (CWE-20 / CWE-400)
    assert_eq!(
        isa_atmosphere(MIN_ALTITUDE_M - 0.001),
        Err(AeroError::AltitudeOutOfRange)
    );
    assert_eq!(
        isa_atmosphere(MAX_ALTITUDE_M + 0.001),
        Err(AeroError::AltitudeOutOfRange)
    );
    assert_eq!(isa_atmosphere(-1e9), Err(AeroError::AltitudeOutOfRange));
    assert_eq!(isa_atmosphere(1e9), Err(AeroError::AltitudeOutOfRange));

    // Invalid thrust / gravity / negative drag coefficients
    let model = AircraftModel::aircraft_preset_trainer().unwrap();
    let mut bad_derivs = *model.aircraft_derivatives();
    bad_derivs.cd0 = -0.01;
    assert_eq!(
        AircraftModel::aircraft_new(
            16.2,
            11.0,
            1.49,
            1000.0,
            *model.aircraft_cl_table(),
            *model.aircraft_cm_table(),
            None,
            bad_derivs,
            *model.aircraft_control_limits(),
        ),
        Err(AeroError::InvalidAeroParameter)
    );
    assert_eq!(
        AircraftModel::aircraft_new(
            16.2,
            11.0,
            1.49,
            MAX_THRUST_N + 1.0,
            *model.aircraft_cl_table(),
            *model.aircraft_cm_table(),
            None,
            *model.aircraft_derivatives(),
            *model.aircraft_control_limits(),
        ),
        Err(AeroError::InvalidAeroParameter)
    );
    assert_eq!(
        FlightEnvironment::env_new(Vec3::zero(), -0.1, 0.0),
        Err(AeroError::InvalidAeroParameter)
    );
    assert_eq!(
        FlightEnvironment::env_new(Vec3::zero(), MAX_GRAVITY_M_S2 + 1.0, 0.0),
        Err(AeroError::InvalidAeroParameter)
    );

    // Atomic state preservation on every error path of step_aircraft_in_place!
    let mass = MassProperties::mass_props_from_diagonal(1000.0, 1280.0, 2660.0, 1820.0).unwrap();
    let env = FlightEnvironment::env_isa_calm();
    let ctrl = ControlInputs::controls_neutral();

    let mut body = RigidBodyState::body_new(
        Vec3::new(12.0, MAX_ALTITUDE_M - 0.01, -4.0),
        Quat::from_axis_angle(Vec3::unit_y(), 0.3).unwrap(),
    )
    .unwrap();
    body.linear_momentum = Vec3::new(50_000.0, 25_000.0, 0.0); // climbing fast -> breaches MAX_ALTITUDE_M mid-step!
    body.angular_momentum = Vec3::new(10.0, -5.0, 2.0);
    let snap = body;
    let snap_digest = body.state_digest_fnv1a().unwrap();

    // 1. Mid-step altitude breach inside RK4 evaluation rolls back atomically
    let err_alt =
        model.step_aircraft_in_place(&mut body, &mass, &env, &ctrl, 0.1, IntegratorKind::RK4);
    assert_eq!(err_alt, Err(AeroError::AltitudeOutOfRange));
    assert_eq!(body, snap);
    assert_eq!(body.state_digest_fnv1a().unwrap(), snap_digest);

    // 2. Bad time steps roll back atomically
    for bad_dt in [0.0, -0.01, MAX_DT + 0.01, f64::NAN, f64::INFINITY] {
        assert!(model
            .step_aircraft_in_place(
                &mut body,
                &mass,
                &env,
                &ctrl,
                bad_dt,
                IntegratorKind::VelocityVerlet
            )
            .is_err());
        assert_eq!(body, snap);
    }

    // 3. Non-finite controls roll back atomically
    let bad_ctrl = ControlInputs {
        elevator_rad: f64::NAN,
        aileron_rad: 0.0,
        rudder_rad: 0.0,
        throttle: 0.5,
    };
    assert_eq!(
        model.step_aircraft_in_place(&mut body, &mass, &env, &bad_ctrl, 0.01, IntegratorKind::RK4),
        Err(AeroError::NonFiniteInput)
    );
    assert_eq!(body, snap);

    // 4. Magnitude overflow (> MAX_COORD) leaves state untouched
    let mut huge_body =
        RigidBodyState::body_new(Vec3::new(0.0, 1000.0, 0.0), Quat::quat_identity()).unwrap();
    huge_body.linear_momentum = Vec3::new(MAX_COORD * 0.9, 0.0, 0.0);
    let huge_snap = huge_body;
    assert!(model
        .step_aircraft_in_place(
            &mut huge_body,
            &mass,
            &env,
            &ctrl,
            0.01,
            IntegratorKind::RK4
        )
        .is_err());
    assert_eq!(huge_body, huge_snap);
}

#[test]
fn seeded_2000_case_hostile_fuzz_campaign_zero_panics() {
    let mut seed: u64 = 0x4741_4d45_3431_484f; // "GAME41HO"
    let model = AircraftModel::aircraft_preset_trainer().unwrap();
    let mass = MassProperties::mass_props_from_diagonal(1000.0, 1280.0, 2660.0, 1820.0).unwrap();

    for case in 0..2_000 {
        let h = hostile_scalar(&mut seed);
        let res = catch_unwind(AssertUnwindSafe(|| isa_atmosphere(h)));
        assert!(res.is_ok(), "isa_atmosphere panicked on case {case}");
        if let Ok(sample) = res.unwrap() {
            assert!(sample.temperature_k.is_finite() && sample.temperature_k > 0.0);
            assert!(sample.pressure_pa.is_finite() && sample.pressure_pa > 0.0);
            assert!(sample.density_kg_m3.is_finite() && sample.density_kg_m3 > 0.0);
            assert!(sample.speed_of_sound_mps.is_finite() && sample.speed_of_sound_mps > 0.0);
        }

        // Hostile table construction & lookup
        let n = ((next_u64(&mut seed) % 80) as usize).min(75);
        let mut bps = Vec::with_capacity(n);
        let mut vals = Vec::with_capacity(n);
        for _ in 0..n {
            bps.push(hostile_scalar(&mut seed));
            vals.push(hostile_scalar(&mut seed));
        }
        let qx = hostile_scalar(&mut seed);
        let tbl_res = catch_unwind(AssertUnwindSafe(|| {
            let t = AeroTable::aero_table_new(&bps, &vals)?;
            t.aero_table_eval(qx)
        }));
        assert!(tbl_res.is_ok(), "AeroTable panicked on case {case}");
        if let Ok(v) = tbl_res.unwrap() {
            assert!(v.is_finite());
        }

        // Hostile flight step with atomic state invariant
        let pos = Vec3::new(
            hostile_scalar(&mut seed),
            hostile_scalar(&mut seed),
            hostile_scalar(&mut seed),
        );
        let q = Quat {
            x: hostile_scalar(&mut seed),
            y: hostile_scalar(&mut seed),
            z: hostile_scalar(&mut seed),
            w: hostile_scalar(&mut seed),
        };
        let p = Vec3::new(
            hostile_scalar(&mut seed),
            hostile_scalar(&mut seed),
            hostile_scalar(&mut seed),
        );
        let l = Vec3::new(
            hostile_scalar(&mut seed),
            hostile_scalar(&mut seed),
            hostile_scalar(&mut seed),
        );
        let mut body = RigidBodyState {
            position: pos,
            linear_momentum: p,
            orientation: q,
            angular_momentum: l,
            force: Vec3::zero(),
            torque: Vec3::zero(),
        };
        let snap_bits = [
            body.position.x.to_bits(),
            body.position.y.to_bits(),
            body.position.z.to_bits(),
            body.linear_momentum.x.to_bits(),
            body.linear_momentum.y.to_bits(),
            body.linear_momentum.z.to_bits(),
            body.orientation.x.to_bits(),
            body.orientation.y.to_bits(),
            body.orientation.z.to_bits(),
            body.orientation.w.to_bits(),
            body.angular_momentum.x.to_bits(),
            body.angular_momentum.y.to_bits(),
            body.angular_momentum.z.to_bits(),
        ];
        let env = FlightEnvironment {
            wind_world_mps: Vec3::new(
                hostile_scalar(&mut seed),
                hostile_scalar(&mut seed),
                hostile_scalar(&mut seed),
            ),
            gravity_m_s2: hostile_scalar(&mut seed),
            ground_elevation_m: hostile_scalar(&mut seed),
        };
        let ctrl = ControlInputs {
            elevator_rad: hostile_scalar(&mut seed),
            aileron_rad: hostile_scalar(&mut seed),
            rudder_rad: hostile_scalar(&mut seed),
            throttle: hostile_scalar(&mut seed),
        };
        let dt = hostile_scalar(&mut seed);
        let integrator = if case % 2 == 0 {
            IntegratorKind::VelocityVerlet
        } else {
            IntegratorKind::RK4
        };

        let step_res = catch_unwind(AssertUnwindSafe(|| {
            model.step_aircraft_in_place(&mut body, &mass, &env, &ctrl, dt, integrator)
        }));
        assert!(
            step_res.is_ok(),
            "step_aircraft_in_place panicked on case {case}"
        );
        if step_res.unwrap().is_err() {
            let after_bits = [
                body.position.x.to_bits(),
                body.position.y.to_bits(),
                body.position.z.to_bits(),
                body.linear_momentum.x.to_bits(),
                body.linear_momentum.y.to_bits(),
                body.linear_momentum.z.to_bits(),
                body.orientation.x.to_bits(),
                body.orientation.y.to_bits(),
                body.orientation.z.to_bits(),
                body.orientation.w.to_bits(),
                body.angular_momentum.x.to_bits(),
                body.angular_momentum.y.to_bits(),
                body.angular_momentum.z.to_bits(),
            ];
            assert_eq!(
                after_bits, snap_bits,
                "atomic rollback failed on case {case}"
            );
        }
    }
}
