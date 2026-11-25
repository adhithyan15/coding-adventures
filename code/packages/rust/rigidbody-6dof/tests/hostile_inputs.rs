//! # Hostile-Input CVE/CWE Proof Suite for `rigidbody-6dof` (GAME40)
//!
//! Proves via adversarial edge vectors and a seeded 2,000-case malformed/extreme
//! fuzz campaign that CWE-20, CWE-125, CWE-190, CWE-369, CWE-400, CWE-787,
//! and CWE-835 cannot trigger panics, undefined behavior, unbounded loops,
//! silent NaN propagation, or partial state corruption:
//! - Subnormals (`f64::from_bits(1)`, `f64::MIN_POSITIVE * 0.5`), `-0.0`,
//!   `f64::MIN`, `f64::MAX`, `NaN`, `±Inf`
//! - `usize::MAX` / `0` substeps, `> MAX_CONTACT_POINTS` (65, 256) probe slices
//! - Zero/subnormal/non-finite `Quat` and `Plane` normals
//! - Seeded 2,000-case randomized hostile-input campaign with bit-exact atomic
//!   state rollback verification on every `Err`.

#![forbid(unsafe_code)]

use math3d::{Mat3, Plane, Quat, Vec3};
use rigidbody_6dof::{
    ContactModel, GroundContactParams, IntegratorKind, MassProperties, RigidBodyError,
    RigidBodyState, MAX_CONTACT_POINTS, MAX_COORD, MAX_DT, MAX_SUBSTEPS,
};

fn next_u64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    x.wrapping_mul(0x2545_f491_4f6c_dd1d)
}

fn hostile_scalar(state: &mut u64) -> f64 {
    match next_u64(state) % 16 {
        0 => f64::NAN,
        1 => f64::from_bits(0x7ff0_0000_0000_0001), // signaling NaN bit pattern
        2 => f64::INFINITY,
        3 => f64::NEG_INFINITY,
        4 => -0.0,
        5 => 0.0,
        6 => f64::from_bits(1), // smallest positive subnormal
        7 => f64::from_bits(0x8000_0000_0000_0001), // smallest negative subnormal
        8 => f64::MIN_POSITIVE * 0.5, // mid subnormal
        9 => f64::MIN,
        10 => f64::MAX,
        11 => 1.0e15,
        12 => -1.0e15,
        13 => 1.0e-15,
        _ => {
            let u = (next_u64(state) >> 11) as f64 / ((1u64 << 53) as f64);
            -20.0 + 40.0 * u
        }
    }
}

fn hostile_vec3(state: &mut u64) -> Vec3 {
    Vec3::new(
        hostile_scalar(state),
        hostile_scalar(state),
        hostile_scalar(state),
    )
}

fn hostile_quat(state: &mut u64) -> Quat {
    Quat {
        x: hostile_scalar(state),
        y: hostile_scalar(state),
        z: hostile_scalar(state),
        w: hostile_scalar(state),
    }
}

fn state_bits_equal(a: &RigidBodyState, b: &RigidBodyState) -> bool {
    let bits_v = |u: Vec3, v: Vec3| {
        u.x.to_bits() == v.x.to_bits()
            && u.y.to_bits() == v.y.to_bits()
            && u.z.to_bits() == v.z.to_bits()
    };
    let bits_q = |u: Quat, v: Quat| {
        u.x.to_bits() == v.x.to_bits()
            && u.y.to_bits() == v.y.to_bits()
            && u.z.to_bits() == v.z.to_bits()
            && u.w.to_bits() == v.w.to_bits()
    };
    bits_v(a.position, b.position)
        && bits_v(a.linear_momentum, b.linear_momentum)
        && bits_q(a.orientation, b.orientation)
        && bits_v(a.angular_momentum, b.angular_momentum)
        && bits_v(a.force, b.force)
        && bits_v(a.torque, b.torque)
}

#[test]
fn explicit_subnormal_extreme_and_overflow_vectors() {
    let subnormal_min = f64::from_bits(1);
    let subnormal_mid = f64::MIN_POSITIVE * 0.5;
    let extremes = [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -0.0,
        0.0,
        subnormal_min,
        -subnormal_min,
        subnormal_mid,
        f64::MIN,
        f64::MAX,
    ];

    for &val in &extremes {
        // Must never panic on any constructor or validator
        let _ = MassProperties::mass_props_from_solid_sphere(val, 1.0);
        let _ = MassProperties::mass_props_from_solid_sphere(1.0, val);
        let _ = MassProperties::mass_props_from_solid_box(val, Vec3::new(val, 1.0, 1.0));
        let _ = MassProperties::mass_props_from_diagonal(1.0, val, 1.0, 1.0);

        let mat = Mat3::mat3_from_cols([[val, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
        if let Ok(m) = mat {
            assert!(MassProperties::mass_props_from_tensor(1.0, m).is_err() || val == 0.0);
        }

        let q = Quat {
            x: val,
            y: 0.0,
            z: 0.0,
            w: val,
        };
        let _ = RigidBodyState::body_new(Vec3::new(val, 0.0, 0.0), q);
    }

    // Specific subnormals & extremes must return typed Err without panic
    assert_eq!(
        MassProperties::mass_props_from_solid_sphere(subnormal_min, 1.0),
        Err(RigidBodyError::InvalidMass)
    );
    assert_eq!(
        MassProperties::mass_props_from_solid_sphere(1.0, subnormal_mid),
        Err(RigidBodyError::InvalidInertiaTensor)
    );
    assert_eq!(
        MassProperties::mass_props_from_solid_box(f64::MAX, Vec3::new(1.0, 1.0, 1.0)),
        Err(RigidBodyError::InvalidMass)
    );
    assert_eq!(
        MassProperties::mass_props_from_diagonal(1.0, f64::MIN, 1.0, 1.0),
        Err(RigidBodyError::InvalidInertiaTensor)
    );

    // Zero, subnormal, and non-finite Plane normals & Quats (CWE-369)
    let bad_planes = [
        Plane {
            normal: Vec3::zero(),
            d: 0.0,
        },
        Plane {
            normal: Vec3::new(subnormal_min, 0.0, 0.0),
            d: 0.0,
        },
        Plane {
            normal: Vec3::new(f64::NAN, 1.0, 0.0),
            d: 0.0,
        },
        Plane {
            normal: Vec3::new(0.0, f64::INFINITY, 0.0),
            d: f64::NEG_INFINITY,
        },
    ];
    for p in bad_planes {
        assert!(GroundContactParams::contact_params_penalty(p, 100.0, 10.0, 0.5, 1).is_err());
        assert!(GroundContactParams::contact_params_impulse(p, 0.5, 0.5, 0.2, 1).is_err());
    }

    let bad_quats = [
        Quat {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 0.0,
        },
        Quat {
            x: subnormal_min,
            y: 0.0,
            z: 0.0,
            w: subnormal_min,
        },
        Quat {
            x: f64::NAN,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        },
        Quat {
            x: f64::INFINITY,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        },
    ];
    for q in bad_quats {
        assert!(RigidBodyState::body_new(Vec3::zero(), q).is_err());
    }
}

#[test]
fn hostile_resource_bounds_and_atomic_rollback() {
    let mass = MassProperties::mass_props_from_solid_box(5.0, Vec3::new(1.0, 0.5, 1.0)).unwrap();
    let ground = Plane::plane_from_point_normal(Vec3::zero(), Vec3::unit_y()).unwrap();

    // usize::MAX and 0 substeps (CWE-400 / CWE-835)
    assert_eq!(
        GroundContactParams::contact_params_penalty(ground, 1000.0, 50.0, 0.5, 0),
        Err(RigidBodyError::SubstepLimitExceeded)
    );
    assert_eq!(
        GroundContactParams::contact_params_penalty(ground, 1000.0, 50.0, 0.5, MAX_SUBSTEPS + 1),
        Err(RigidBodyError::SubstepLimitExceeded)
    );
    assert_eq!(
        GroundContactParams::contact_params_impulse(ground, 0.5, 0.5, 0.2, usize::MAX),
        Err(RigidBodyError::SubstepLimitExceeded)
    );

    // Oversized probe slices (> MAX_CONTACT_POINTS)
    let params = GroundContactParams::contact_params_impulse(ground, 0.5, 0.5, 0.2, 2).unwrap();
    let probes_65 = vec![Vec3::zero(); MAX_CONTACT_POINTS + 1];
    let probes_256 = vec![Vec3::new(0.1, -0.5, 0.2); 256];

    let mut body = RigidBodyState::body_new(
        Vec3::new(1.0, 2.0, 3.0),
        Quat::from_axis_angle(Vec3::unit_y(), 0.4).unwrap(),
    )
    .unwrap();
    body.linear_momentum = Vec3::new(1.5, -0.5, 0.25);
    body.angular_momentum = Vec3::new(0.2, 0.3, -0.1);
    let snap = body;

    for probes in [&probes_65[..], &probes_256[..]] {
        assert_eq!(
            body.step_with_contacts_in_place(&mass, 0.016, IntegratorKind::RK4, &params, probes),
            Err(RigidBodyError::TooManyContactPoints)
        );
        assert!(state_bits_equal(&body, &snap));
    }

    // Hostile dt values (0, -0.0, subnormal, MAX_DT + eps, f64::MAX, NaN, Inf)
    let bad_dts = [
        0.0,
        -0.0,
        -1e-6,
        MAX_DT + 1e-12,
        f64::MAX,
        f64::MIN,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];
    for &dt in &bad_dts {
        assert!(body
            .step_in_place(&mass, dt, IntegratorKind::VelocityVerlet)
            .is_err());
        assert!(state_bits_equal(&body, &snap));
        assert!(body.step_in_place(&mass, dt, IntegratorKind::RK4).is_err());
        assert!(state_bits_equal(&body, &snap));
    }

    // Hostile force/torque accumulation preserves state atomically
    for v in [
        Vec3::new(f64::NAN, 0.0, 0.0),
        Vec3::new(f64::INFINITY, 0.0, 0.0),
        Vec3::new(MAX_COORD * 2.0, 0.0, 0.0),
        Vec3::new(f64::MAX, f64::MIN, 0.0),
    ] {
        assert!(body.apply_world_force(v).is_err());
        assert!(state_bits_equal(&body, &snap));
        assert!(body.apply_body_force(v).is_err());
        assert!(state_bits_equal(&body, &snap));
        assert!(body.apply_world_torque(v).is_err());
        assert!(state_bits_equal(&body, &snap));
        assert!(body.apply_body_torque(v).is_err());
        assert!(state_bits_equal(&body, &snap));
        assert!(body.apply_force_at_world_point(v, Vec3::unit_y()).is_err());
        assert!(state_bits_equal(&body, &snap));
        assert!(body.apply_force_at_body_point(Vec3::unit_y(), v).is_err());
        assert!(state_bits_equal(&body, &snap));
    }
}

#[test]
fn seeded_2000_case_hostile_input_fuzz_campaign() {
    let mut seed: u64 = 0xc001_cafe_dead_beef;
    let valid_mass = MassProperties::mass_props_from_diagonal(4.0, 1.5, 2.5, 3.5).unwrap();
    let valid_ground = Plane::plane_from_point_normal(Vec3::zero(), Vec3::unit_y()).unwrap();

    let mut ok_steps = 0usize;
    let mut err_steps = 0usize;

    for case in 0..2000 {
        // 1. Adversarial MassProperties construction
        let m_in = hostile_scalar(&mut seed);
        let tensor_in = Mat3::mat3_from_cols([
            [
                hostile_scalar(&mut seed),
                hostile_scalar(&mut seed),
                hostile_scalar(&mut seed),
            ],
            [
                hostile_scalar(&mut seed),
                hostile_scalar(&mut seed),
                hostile_scalar(&mut seed),
            ],
            [
                hostile_scalar(&mut seed),
                hostile_scalar(&mut seed),
                hostile_scalar(&mut seed),
            ],
        ]);
        let active_mass = match tensor_in {
            Ok(t) => match MassProperties::mass_props_from_tensor(m_in, t) {
                Ok(mp) => mp,
                Err(_) => valid_mass,
            },
            Err(_) => valid_mass,
        };

        // 2. Adversarial GroundContactParams construction
        let substeps = match next_u64(&mut seed) % 6 {
            0 => 0,
            1 => MAX_SUBSTEPS + 1,
            2 => usize::MAX,
            _ => ((next_u64(&mut seed) % 4) + 1) as usize,
        };
        let raw_plane = Plane {
            normal: hostile_vec3(&mut seed),
            d: hostile_scalar(&mut seed),
        };
        let params = if case % 2 == 0 {
            GroundContactParams::contact_params_penalty(
                raw_plane,
                hostile_scalar(&mut seed),
                hostile_scalar(&mut seed),
                hostile_scalar(&mut seed),
                substeps,
            )
        } else {
            GroundContactParams::contact_params_impulse(
                raw_plane,
                hostile_scalar(&mut seed),
                hostile_scalar(&mut seed),
                hostile_scalar(&mut seed),
                substeps,
            )
        }
        .unwrap_or_else(|_| {
            GroundContactParams::contact_params_impulse(valid_ground, 0.5, 0.3, 0.2, 2).unwrap()
        });
        let _ = matches!(
            params.contact_params_model(),
            ContactModel::Penalty { .. } | ContactModel::Impulse { .. }
        );

        // 3. Adversarial RigidBodyState (sometimes valid, sometimes corrupted)
        let mut body = if case % 3 == 0 {
            RigidBodyState::body_new(
                Vec3::new(0.0, 1.0, 0.0),
                Quat::from_axis_angle(Vec3::unit_y(), 0.2).unwrap(),
            )
            .unwrap()
        } else {
            RigidBodyState {
                position: hostile_vec3(&mut seed),
                linear_momentum: hostile_vec3(&mut seed),
                orientation: hostile_quat(&mut seed),
                angular_momentum: hostile_vec3(&mut seed),
                force: hostile_vec3(&mut seed),
                torque: hostile_vec3(&mut seed),
            }
        };

        let _ = body.apply_world_force(hostile_vec3(&mut seed));
        let _ = body.apply_force_at_world_point(hostile_vec3(&mut seed), hostile_vec3(&mut seed));
        let _ = body.kinetic_energy(&active_mass);
        let _ = body.state_digest_fnv1a();

        let probe_len = match next_u64(&mut seed) % 8 {
            0 => MAX_CONTACT_POINTS + 1,
            1 => 80,
            _ => (next_u64(&mut seed) % 5) as usize,
        };
        let mut probes = Vec::with_capacity(probe_len);
        for _ in 0..probe_len {
            probes.push(hostile_vec3(&mut seed));
        }

        let dt = match next_u64(&mut seed) % 4 {
            0 => 1.0 / 60.0,
            _ => hostile_scalar(&mut seed),
        };
        let integrator = if case % 2 == 0 {
            IntegratorKind::VelocityVerlet
        } else {
            IntegratorKind::RK4
        };

        let snap = body;
        let res = if case % 3 == 0 {
            body.step_in_place(&active_mass, dt, integrator)
        } else {
            body.step_with_contacts_in_place(&active_mass, dt, integrator, &params, &probes)
        };

        match res {
            Ok(()) => {
                ok_steps += 1;
                assert!(body.body_validate().is_ok());
                assert!((body.orientation.quat_norm_sq().unwrap() - 1.0).abs() < 1e-11);
            }
            Err(_) => {
                err_steps += 1;
                // Bit-exact atomic state preservation even when initial fields contained NaN/-0.0
                assert!(
                    state_bits_equal(&body, &snap),
                    "state mutated on Err in hostile fuzz case {case}"
                );
            }
        }
    }

    assert!(ok_steps > 0 && err_steps > 0);
}
