//! # Conformance Tests for `rigidbody-6dof` (GAME40)
//!
//! Covers:
//! - `MassProperties`: solid box/sphere/diagonal/tensor construction, symmetry,
//!   positive-definiteness (Sylvester minors + `det > MIN_DET`)
//! - `RigidBodyState`: world/body force & torque accumulation, point-force moments,
//!   diagnostics (`linear_velocity`, `angular_velocity_world/body`, `kinetic_energy`,
//!   canonical `state_digest_fnv1a` with `-0.0` normalization)
//! - Analytic free-fall trajectory (`VelocityVerlet` and `RK4`)
//! - Torque-free conservation of `p`, `L`, and rotational `KE` for an asymmetric
//!   inertia tensor (Dzhanibekov / Poinsot motion)
//! - Constant-torque spin-up (`alpha = I^-1 * tau`, work-energy theorem)
//! - `dt`-halving convergence: 2nd-order (`~4x`) for `VelocityVerlet` vs
//!   4th-order (`~16x`) for `RK4`
//! - Ground contact models: 4-probe penalty spring-damper settling & Coulomb
//!   friction deceleration, plus impulse restitution bounce height (`h1/h0 = e^2`)
//! - Security / CWE-20/190/369/400/835 refusal & atomic rollback preservation
//! - 1,000-case seeded deterministic property suite

use math3d::{Mat3, Plane, Quat, Vec3};
use rigidbody_6dof::{
    GroundContactParams, IntegratorKind, MassProperties, RigidBodyError, RigidBodyState, EPSILON,
    MAX_CONTACT_POINTS, MAX_COORD, MAX_DT, MAX_MASS, MAX_SUBSTEPS, MIN_DET, MIN_MASS,
};

const TOL: f64 = 1e-9;

fn approx(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() < tol
}

fn vec3_approx(a: Vec3, b: Vec3, tol: f64) -> bool {
    approx(a.x, b.x, tol) && approx(a.y, b.y, tol) && approx(a.z, b.z, tol)
}

fn next_u64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    x.wrapping_mul(0x2545_f491_4f6c_dd1d)
}

fn next_range(state: &mut u64, lo: f64, hi: f64) -> f64 {
    let u = (next_u64(state) >> 11) as f64 / ((1u64 << 53) as f64);
    lo + (hi - lo) * u
}

// --- MassProperties --------------------------------------------------------

#[test]
fn mass_properties_constructors_and_inversion() {
    let box_mp = MassProperties::mass_props_from_solid_box(12.0, Vec3::new(0.5, 1.0, 1.5)).unwrap();
    assert_eq!(box_mp.mass_props_mass(), 12.0);
    assert!(approx(box_mp.mass_props_inv_mass(), 1.0 / 12.0, TOL));

    let i_prod = box_mp
        .mass_props_inertia_body()
        .mat3_mul(&box_mp.mass_props_inv_inertia_body())
        .unwrap();
    let ident = Mat3::mat3_identity();
    for c in 0..3 {
        for r in 0..3 {
            assert!(approx(i_prod.cols[c][r], ident.cols[c][r], 1e-12));
        }
    }

    let sphere_mp = MassProperties::mass_props_from_solid_sphere(5.0, 2.0).unwrap();
    let expected_i = 0.4 * 5.0 * 4.0;
    assert!(approx(
        sphere_mp.mass_props_inertia_body().cols[0][0],
        expected_i,
        1e-12
    ));

    // Symmetric non-diagonal positive-definite tensor
    let sym = Mat3::mat3_from_cols([[4.0, 0.5, -0.2], [0.5, 5.0, 0.3], [-0.2, 0.3, 6.0]]).unwrap();
    let tensor_mp = MassProperties::mass_props_from_tensor(3.0, sym).unwrap();
    let prod = tensor_mp
        .mass_props_inertia_body()
        .mat3_mul(&tensor_mp.mass_props_inv_inertia_body())
        .unwrap();
    for c in 0..3 {
        for r in 0..3 {
            assert!(approx(prod.cols[c][r], ident.cols[c][r], 1e-12));
        }
    }

    // Small physical rigid bodies (scale-relative PD check & normalized inversion)
    let small_sphere = MassProperties::mass_props_from_solid_sphere(1.0, 0.01).unwrap();
    let baseball = MassProperties::mass_props_from_solid_sphere(0.145, 0.037).unwrap();
    let small_box =
        MassProperties::mass_props_from_solid_box(0.2, Vec3::new(0.02, 0.02, 0.02)).unwrap();
    for mp in [small_sphere, baseball, small_box] {
        let p = mp
            .mass_props_inertia_body()
            .mat3_mul(&mp.mass_props_inv_inertia_body())
            .unwrap();
        for c in 0..3 {
            for r in 0..3 {
                assert!(approx(p.cols[c][r], ident.cols[c][r], 1e-10));
            }
        }
    }

    // Large aircraft-scale tensor with 1-ULP off-diagonal rounding difference
    let aircraft = Mat3::mat3_from_cols([
        [1.2e8, 0.0, 4.0e7],
        [0.0, 2.5e8, 0.0],
        [4.0e7 + 7.45e-9, 0.0, 3.1e8],
    ])
    .unwrap();
    let aircraft_mp = MassProperties::mass_props_from_tensor(2.0e5, aircraft).unwrap();
    let ap = aircraft_mp
        .mass_props_inertia_body()
        .mat3_mul(&aircraft_mp.mass_props_inv_inertia_body())
        .unwrap();
    for c in 0..3 {
        for r in 0..3 {
            assert!(approx(ap.cols[c][r], ident.cols[c][r], 1e-10));
        }
    }

    // MAX_MASS body weight stays safely below MAX_COORD
    let mut heavy = RigidBodyState::body_identity();
    heavy
        .apply_world_force(Vec3::new(0.0, -MAX_MASS * 9.81, 0.0))
        .unwrap();
    assert!(approx(heavy.force.y, -MAX_MASS * 9.81, 1.0));
}

// --- Analytic Free Fall ----------------------------------------------------

#[test]
fn analytic_free_fall_verlet_and_rk4() {
    let mass = MassProperties::mass_props_from_solid_sphere(2.5, 1.0).unwrap();
    let g = -9.81;
    let dt = 1.0 / 120.0;
    let steps = 240; // t = 2.0 s
    let t_total = (steps as f64) * dt;
    let v0 = Vec3::new(3.0, 10.0, -2.0);
    let p0 = Vec3::new(1.0, 50.0, 4.0);

    for integrator in [IntegratorKind::VelocityVerlet, IntegratorKind::RK4] {
        let mut body = RigidBodyState::body_new(p0, Quat::quat_identity()).unwrap();
        body.linear_momentum = v0.vec3_scale(mass.mass_props_mass()).unwrap();
        body.apply_world_force(Vec3::new(0.0, mass.mass_props_mass() * g, 0.0))
            .unwrap();

        for _ in 0..steps {
            body.step_in_place(&mass, dt, integrator).unwrap();
        }

        let expected_pos = Vec3::new(
            p0.x + v0.x * t_total,
            p0.y + v0.y * t_total + 0.5 * g * t_total * t_total,
            p0.z + v0.z * t_total,
        );
        let expected_vel = Vec3::new(v0.x, v0.y + g * t_total, v0.z);

        assert!(
            vec3_approx(body.position, expected_pos, 1e-10),
            "pos mismatch for {integrator:?}: {:?} vs {:?}",
            body.position,
            expected_pos
        );
        assert!(
            vec3_approx(body.linear_velocity(&mass).unwrap(), expected_vel, 1e-10),
            "vel mismatch for {integrator:?}"
        );
    }
}

// --- Torque-Free Asymmetric Conservation -----------------------------------

#[test]
fn torque_free_conservation_asymmetric_inertia() {
    // Distinct principal moments so gyroscopic precession & Dzhanibekov nutation occur.
    let mass = MassProperties::mass_props_from_diagonal(2.0, 1.2, 3.1, 4.7).unwrap();
    let init_pos = Vec3::new(0.0, 10.0, 0.0);
    let init_q = Quat::from_euler_yxz(0.4, -0.3, 0.7).unwrap();
    let init_p = Vec3::new(4.0, -2.0, 1.5);
    let init_l = Vec3::new(1.5, 4.2, 0.8);

    for (integrator, tol_ke) in [
        (IntegratorKind::VelocityVerlet, 5e-6),
        (IntegratorKind::RK4, 1e-9),
    ] {
        let mut body = RigidBodyState::body_new(init_pos, init_q).unwrap();
        body.linear_momentum = init_p;
        body.angular_momentum = init_l;

        let ke0 = body.kinetic_energy(&mass).unwrap();
        let rot_ke0 = body.rotational_kinetic_energy(&mass).unwrap();

        let dt = 0.002;
        for _ in 0..2000 {
            body.step_in_place(&mass, dt, integrator).unwrap();
        }

        assert!(vec3_approx(body.linear_momentum, init_p, 1e-11));
        assert!(vec3_approx(body.angular_momentum, init_l, 1e-11));
        assert!(approx(body.orientation.quat_norm_sq().unwrap(), 1.0, 1e-12));
        assert!(
            approx(
                body.rotational_kinetic_energy(&mass).unwrap(),
                rot_ke0,
                tol_ke
            ),
            "rot KE drift for {integrator:?}: {} vs {}",
            body.rotational_kinetic_energy(&mass).unwrap(),
            rot_ke0
        );
        assert!(approx(body.kinetic_energy(&mass).unwrap(), ke0, tol_ke));
    }
}

// --- Constant Torque Spin-Up -----------------------------------------------

#[test]
fn constant_torque_euler_spin_up() {
    let mass = MassProperties::mass_props_from_diagonal(3.0, 2.0, 5.0, 8.0).unwrap();
    let tau = Vec3::new(0.0, 2.5, 0.0);
    let dt = 0.005;
    let steps = 400; // t = 2.0 s
    let t = (steps as f64) * dt;

    for integrator in [IntegratorKind::VelocityVerlet, IntegratorKind::RK4] {
        let mut body = RigidBodyState::body_identity();
        body.apply_world_torque(tau).unwrap();

        for _ in 0..steps {
            body.step_in_place(&mass, dt, integrator).unwrap();
        }

        // L(t) = tau * t, omega_y = L_y / I_yy = (2.5 * 2.0) / 5.0 = 1.0 rad/s
        assert!(vec3_approx(
            body.angular_momentum,
            Vec3::new(0.0, 5.0, 0.0),
            1e-11
        ));
        let omega = body.angular_velocity_world(&mass).unwrap();
        assert!(vec3_approx(omega, Vec3::new(0.0, 1.0, 0.0), 1e-10));
        // theta = 0.5 * alpha * t^2 = 0.5 * (2.5 / 5.0) * 4.0 = 1.0 rad -> rot KE = tau * theta = 2.5 J
        let expected_rot_ke = 0.5 * (tau.y * t) * (tau.y * t) / 5.0;
        assert!(approx(
            body.rotational_kinetic_energy(&mass).unwrap(),
            expected_rot_ke,
            1e-9
        ));
    }
}

// --- Order-of-Accuracy dt-Halving Convergence ------------------------------

#[test]
fn dt_halving_convergence_verlet_vs_rk4() {
    let mass = MassProperties::mass_props_from_diagonal(2.0, 1.3, 2.7, 4.1).unwrap();
    let total_t = 1.0;

    // Harmonic spring force F = -k*x plus weak coupled torque tau = -0.2 * x.cross(e_y)
    let spring_force = |s: &RigidBodyState| -> Result<(Vec3, Vec3), RigidBodyError> {
        let f = s.position.vec3_scale(-4.0)?;
        let t = s.position.cross(Vec3::unit_y())?.vec3_scale(-0.2)?;
        Ok((f, t))
    };

    let init_state = || {
        let mut s = RigidBodyState::body_new(
            Vec3::new(1.0, 0.5, -0.2),
            Quat::from_euler_xyz(0.1, 0.2, 0.3).unwrap(),
        )
        .unwrap();
        s.linear_momentum = Vec3::new(0.0, 1.0, 0.5);
        s.angular_momentum = Vec3::new(0.8, 1.4, -0.5);
        s
    };

    // Ultra-fine RK4 reference solution (N = 4096)
    let mut ref_state = init_state();
    let ref_dt = total_t / 4096.0;
    for _ in 0..4096 {
        ref_state
            .step_with_force_fn_in_place(&mass, ref_dt, IntegratorKind::RK4, spring_force)
            .unwrap();
    }

    let run_with_steps = |n: usize, integrator: IntegratorKind| -> f64 {
        let mut s = init_state();
        let dt = total_t / (n as f64);
        for _ in 0..n {
            s.step_with_force_fn_in_place(&mass, dt, integrator, spring_force)
                .unwrap();
        }
        let pos_err = s.position.distance(ref_state.position).unwrap();
        let dot = s.orientation.x * ref_state.orientation.x
            + s.orientation.y * ref_state.orientation.y
            + s.orientation.z * ref_state.orientation.z
            + s.orientation.w * ref_state.orientation.w;
        let sign = if dot < 0.0 { -1.0 } else { 1.0 };
        let dx = s.orientation.x - sign * ref_state.orientation.x;
        let dy = s.orientation.y - sign * ref_state.orientation.y;
        let dz = s.orientation.z - sign * ref_state.orientation.z;
        let dw = s.orientation.w - sign * ref_state.orientation.w;
        let sx = s.orientation.x + sign * ref_state.orientation.x;
        let sy = s.orientation.y + sign * ref_state.orientation.y;
        let sz = s.orientation.z + sign * ref_state.orientation.z;
        let sw = s.orientation.w + sign * ref_state.orientation.w;
        let chord_norm = trig::sqrt(dx * dx + dy * dy + dz * dz + dw * dw);
        let sum_norm = trig::sqrt(sx * sx + sy * sy + sz * sz + sw * sw);
        let ang_err = 2.0 * trig::atan2(chord_norm, sum_norm);
        pos_err + ang_err
    };

    // Velocity-Verlet: 2nd-order -> ratio ~ 4.0 when halving dt
    let e_verlet_1 = run_with_steps(32, IntegratorKind::VelocityVerlet);
    let e_verlet_2 = run_with_steps(64, IntegratorKind::VelocityVerlet);
    let verlet_ratio = e_verlet_1 / e_verlet_2;
    assert!(
        verlet_ratio > 3.5 && verlet_ratio < 4.5,
        "expected 2nd-order Verlet ratio ~4.0, got {verlet_ratio} (e1={e_verlet_1}, e2={e_verlet_2})"
    );

    // RK4: 4th-order -> ratio ~ 16.0 when halving dt
    let e_rk4_1 = run_with_steps(16, IntegratorKind::RK4);
    let e_rk4_2 = run_with_steps(32, IntegratorKind::RK4);
    let rk4_ratio = e_rk4_1 / e_rk4_2;
    assert!(
        rk4_ratio > 14.0 && rk4_ratio < 18.5,
        "expected 4th-order RK4 ratio ~16.0, got {rk4_ratio} (e1={e_rk4_1}, e2={e_rk4_2})"
    );
}

// --- Ground Contact Models -------------------------------------------------

#[test]
fn ground_contact_penalty_4_probe_settle_and_friction() {
    let mass = MassProperties::mass_props_from_solid_box(10.0, Vec3::new(1.0, 0.5, 1.0)).unwrap();
    let ground = Plane::plane_from_point_normal(Vec3::zero(), Vec3::unit_y()).unwrap();
    let params =
        GroundContactParams::contact_params_penalty(ground, 20_000.0, 900.0, 0.6, 4).unwrap();

    // 4 landing gear probes at the bottom corners of the body (y_local = -0.5)
    let probes = [
        Vec3::new(-1.0, -0.5, -1.0),
        Vec3::new(1.0, -0.5, -1.0),
        Vec3::new(-1.0, -0.5, 1.0),
        Vec3::new(1.0, -0.5, 1.0),
    ];

    let mut body =
        RigidBodyState::body_new(Vec3::new(0.0, 1.2, 0.0), Quat::quat_identity()).unwrap();
    // Give horizontal glide velocity +X so Coulomb friction arrests it on touchdown
    body.linear_momentum = Vec3::new(20.0, 0.0, 0.0);
    body.apply_world_force(Vec3::new(0.0, -10.0 * 9.81, 0.0))
        .unwrap();

    let mut last_report = None;
    for _ in 0..300 {
        last_report = Some(
            body.step_with_contacts_report_in_place(
                &mass,
                1.0 / 60.0,
                IntegratorKind::VelocityVerlet,
                &params,
                &probes,
            )
            .unwrap(),
        );
    }

    let report = last_report.unwrap();
    assert_eq!(report.report_active_contacts(), 4 * 4);
    // Static equilibrium penetration: 4 * k * pen = m * g -> pen = 98.1 / 80_000 = 0.00122625 m
    let expected_y = 0.5 - (10.0 * 9.81) / (4.0 * 20_000.0);
    assert!(
        approx(body.position.y, expected_y, 1e-4),
        "body.position.y = {}, expected {}",
        body.position.y,
        expected_y
    );
    let vel = body.linear_velocity(&mass).unwrap();
    assert!(
        vel.length().unwrap() < 1e-3,
        "body failed to settle: {vel:?}"
    );
    let omega = body.angular_velocity_world(&mass).unwrap();
    assert!(
        omega.length().unwrap() < 1e-3,
        "angular vel non-zero: {omega:?}"
    );
}

#[test]
fn ground_contact_impulse_restitution_bounce_and_rest() {
    let mass = MassProperties::mass_props_from_solid_sphere(2.0, 0.5).unwrap();
    let ground = Plane::plane_from_point_normal(Vec3::zero(), Vec3::unit_y()).unwrap();
    let restitution = 0.8;
    let params =
        GroundContactParams::contact_params_impulse(ground, restitution, 0.0, 0.2, 4).unwrap();
    let probes = [Vec3::new(0.0, -0.5, 0.0)];

    let mut body =
        RigidBodyState::body_new(Vec3::new(0.0, 5.5, 0.0), Quat::quat_identity()).unwrap();
    let g = 9.81;
    body.apply_world_force(Vec3::new(0.0, -mass.mass_props_mass() * g, 0.0))
        .unwrap();

    // Drop from COM y = 5.5 (probe at y = 5.0 -> drop height h0 = 5.0 m).
    // Track peak height after first bounce.
    let dt = 1.0 / 240.0;
    let mut bounced = false;
    let mut max_post_bounce_probe_y = 0.0_f64;
    for _ in 0..600 {
        let rep = body
            .step_with_contacts_report_in_place(
                &mass,
                dt,
                IntegratorKind::VelocityVerlet,
                &params,
                &probes,
            )
            .unwrap();
        if rep.report_active_contacts() > 0 && body.linear_velocity(&mass).unwrap().y > 0.0 {
            bounced = true;
        }
        if bounced {
            let probe_y = body.position.y - 0.5;
            if probe_y > max_post_bounce_probe_y {
                max_post_bounce_probe_y = probe_y;
            }
            if body.linear_velocity(&mass).unwrap().y < 0.0 && max_post_bounce_probe_y > 1.0 {
                break;
            }
        }
    }

    assert!(bounced);
    let expected_h1 = 5.0 * restitution * restitution; // 3.2 m
    assert!(
        approx(max_post_bounce_probe_y, expected_h1, 0.08),
        "bounce height {max_post_bounce_probe_y} vs expected {expected_h1}"
    );

    // Zero-restitution settling on ground
    let settle_params =
        GroundContactParams::contact_params_impulse(ground, 0.0, 0.8, 0.4, 4).unwrap();
    let mut resting =
        RigidBodyState::body_new(Vec3::new(0.0, 1.0, 0.0), Quat::quat_identity()).unwrap();
    resting
        .apply_world_force(Vec3::new(0.0, -mass.mass_props_mass() * g, 0.0))
        .unwrap();
    for _ in 0..240 {
        resting
            .step_with_contacts_in_place(
                &mass,
                1.0 / 60.0,
                IntegratorKind::VelocityVerlet,
                &settle_params,
                &probes,
            )
            .unwrap();
    }
    assert!(approx(resting.position.y, 0.5, 0.01));
    assert!(resting.linear_velocity(&mass).unwrap().length().unwrap() < 0.05);
}

// --- Diagnostics & Canonical Digest ----------------------------------------

#[test]
fn diagnostics_and_digest_canonicalizes_negative_zero() {
    let mut s1 = RigidBodyState::body_identity();
    let mut s2 = RigidBodyState::body_identity();
    s2.position = Vec3::new(-0.0, -0.0, -0.0);
    s2.linear_momentum = Vec3::new(-0.0, 0.0, -0.0);
    s2.angular_momentum = Vec3::new(0.0, -0.0, -0.0);
    s2.force = Vec3::new(-0.0, -0.0, 0.0);
    s2.torque = Vec3::new(0.0, -0.0, 0.0);

    assert_eq!(
        s1.state_digest_fnv1a().unwrap(),
        s2.state_digest_fnv1a().unwrap()
    );

    // Body force/torque & world point helpers
    let q = Quat::from_axis_angle(Vec3::unit_y(), trig::PI / 2.0).unwrap();
    s1 = RigidBodyState::body_new(Vec3::new(1.0, 2.0, 3.0), q).unwrap();
    s1.apply_body_force(Vec3::new(0.0, 0.0, 10.0)).unwrap();
    // +Z rotated +90 deg about +Y becomes +X
    assert!(vec3_approx(s1.force, Vec3::new(10.0, 0.0, 0.0), 1e-12));

    s1.body_clear_accumulators();
    s1.apply_force_at_world_point(Vec3::new(0.0, 10.0, 0.0), Vec3::new(3.0, 2.0, 3.0))
        .unwrap();
    // r = (2, 0, 0), F = (0, 10, 0) => tau = (0, 0, 20)
    assert!(vec3_approx(s1.force, Vec3::new(0.0, 10.0, 0.0), 1e-12));
    assert!(vec3_approx(s1.torque, Vec3::new(0.0, 0.0, 20.0), 1e-12));

    let mass = MassProperties::mass_props_from_diagonal(2.0, 1.0, 2.0, 4.0).unwrap();
    s1.linear_momentum = Vec3::new(2.0, 0.0, 0.0);
    s1.angular_momentum = Vec3::new(0.0, 4.0, 0.0);
    let v_pt = s1
        .world_point_velocity(&mass, Vec3::new(1.0, 2.0, 4.0))
        .unwrap();
    // omega = (0, 2, 0), r = (0, 0, 1) => omega x r = (2, 0, 0), plus v_cm (1, 0, 0) = (3, 0, 0)
    assert!(vec3_approx(v_pt, Vec3::new(3.0, 0.0, 0.0), 1e-12));
    assert!(vec3_approx(
        s1.angular_velocity_body(&mass).unwrap(),
        Vec3::new(0.0, 2.0, 0.0),
        1e-12
    ));
}

// --- Security / CWE Refusal & Atomic State Preservation --------------------

#[test]
fn cwe_guards_and_atomic_error_preservation() {
    // CWE-20 / CWE-369 Mass & Inertia guards
    assert_eq!(
        MassProperties::mass_props_from_solid_sphere(f64::NAN, 1.0),
        Err(RigidBodyError::NonFiniteInput)
    );
    assert_eq!(
        MassProperties::mass_props_from_solid_sphere(0.0, 1.0),
        Err(RigidBodyError::InvalidMass)
    );
    assert_eq!(
        MassProperties::mass_props_from_solid_sphere(MIN_MASS * 0.5, 1.0),
        Err(RigidBodyError::InvalidMass)
    );
    assert_eq!(
        MassProperties::mass_props_from_solid_sphere(MAX_MASS * 2.0, 1.0),
        Err(RigidBodyError::InvalidMass)
    );
    assert_eq!(
        MassProperties::mass_props_from_diagonal(1.0, -1.0, 2.0, 3.0),
        Err(RigidBodyError::InvalidInertiaTensor)
    );
    assert_eq!(
        MassProperties::mass_props_from_diagonal(1.0, 1.0, 1.0, MIN_DET * 0.5),
        Err(RigidBodyError::InvalidInertiaTensor)
    );
    // Asymmetric tensor rejected
    let asym = Mat3::mat3_from_cols([[2.0, 0.5, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 2.0]]).unwrap();
    assert_eq!(
        MassProperties::mass_props_from_tensor(1.0, asym),
        Err(RigidBodyError::InvalidInertiaTensor)
    );
    // Negative 2nd principal minor rejected
    let indefinite =
        Mat3::mat3_from_cols([[1.0, 2.0, 0.0], [2.0, 1.0, 0.0], [0.0, 0.0, 1.0]]).unwrap();
    assert_eq!(
        MassProperties::mass_props_from_tensor(1.0, indefinite),
        Err(RigidBodyError::InvalidInertiaTensor)
    );

    // CWE-369 Zero-length quaternion & ground normal
    assert_eq!(
        RigidBodyState::body_new(
            Vec3::zero(),
            Quat::quat_from_xyzw(0.0, 0.0, 0.0, 0.0).unwrap()
        ),
        Err(RigidBodyError::ZeroLengthNormal)
    );
    let ground = Plane::plane_from_point_normal(Vec3::zero(), Vec3::unit_y()).unwrap();

    // CWE-20 Contact parameters & dt
    assert_eq!(
        GroundContactParams::contact_params_impulse(ground, -0.1, 0.5, 0.2, 1),
        Err(RigidBodyError::InvalidContactParameter)
    );
    assert_eq!(
        GroundContactParams::contact_params_impulse(ground, 1.1, 0.5, 0.2, 1),
        Err(RigidBodyError::InvalidContactParameter)
    );
    assert_eq!(
        GroundContactParams::contact_params_impulse(ground, 0.5, -0.1, 0.2, 1),
        Err(RigidBodyError::InvalidContactParameter)
    );
    assert_eq!(
        GroundContactParams::contact_params_penalty(ground, -1.0, 10.0, 0.5, 1),
        Err(RigidBodyError::InvalidContactParameter)
    );
    assert_eq!(
        GroundContactParams::contact_params_penalty(ground, 10.0, -1.0, 0.5, 1),
        Err(RigidBodyError::InvalidContactParameter)
    );

    // CWE-400 / CWE-835 Substeps & Contact points
    assert_eq!(
        GroundContactParams::contact_params_impulse(ground, 0.5, 0.5, 0.2, 0),
        Err(RigidBodyError::SubstepLimitExceeded)
    );
    assert_eq!(
        GroundContactParams::contact_params_impulse(ground, 0.5, 0.5, 0.2, MAX_SUBSTEPS + 1),
        Err(RigidBodyError::SubstepLimitExceeded)
    );

    let mass = MassProperties::mass_props_from_solid_sphere(2.0, 1.0).unwrap();
    let valid_params =
        GroundContactParams::contact_params_impulse(ground, 0.5, 0.5, 0.2, 2).unwrap();
    let too_many_probes = [Vec3::zero(); MAX_CONTACT_POINTS + 1];

    // Atomic rollback verification across all error paths
    let mut body = RigidBodyState::body_new(
        Vec3::new(1.0, 2.0, 3.0),
        Quat::from_axis_angle(Vec3::unit_y(), 0.5).unwrap(),
    )
    .unwrap();
    body.linear_momentum = Vec3::new(0.4, -0.2, 0.8);
    body.angular_momentum = Vec3::new(0.1, 0.3, -0.2);
    let snap = body;
    let snap_digest = body.state_digest_fnv1a().unwrap();

    assert_eq!(
        body.step_in_place(&mass, 0.0, IntegratorKind::RK4),
        Err(RigidBodyError::InvalidTimeStep)
    );
    assert_eq!(body, snap);

    assert_eq!(
        body.step_in_place(&mass, MAX_DT + 0.01, IntegratorKind::VelocityVerlet),
        Err(RigidBodyError::InvalidTimeStep)
    );
    assert_eq!(body, snap);

    assert_eq!(
        body.step_in_place(&mass, f64::NAN, IntegratorKind::RK4),
        Err(RigidBodyError::NonFiniteInput)
    );
    assert_eq!(body, snap);

    assert_eq!(
        body.step_with_contacts_in_place(
            &mass,
            0.016,
            IntegratorKind::RK4,
            &valid_params,
            &too_many_probes
        ),
        Err(RigidBodyError::TooManyContactPoints)
    );
    assert_eq!(body, snap);

    // Numerical divergence during force evaluation leaves state untouched!
    let mut step_counter = 0usize;
    let diverging_force = |_s: &RigidBodyState| -> Result<(Vec3, Vec3), RigidBodyError> {
        step_counter += 1;
        if step_counter >= 2 {
            return Ok((Vec3::new(MAX_COORD * 10.0, 0.0, 0.0), Vec3::zero()));
        }
        Ok((Vec3::new(1.0, 0.0, 0.0), Vec3::zero()))
    };
    assert_eq!(
        body.step_with_force_fn_in_place(&mass, 0.016, IntegratorKind::RK4, diverging_force),
        Err(RigidBodyError::NumericalDivergence)
    );
    assert_eq!(body, snap);
    assert_eq!(body.state_digest_fnv1a().unwrap(), snap_digest);

    // Overflowing apply_force_at_world_point leaves both force and torque untouched
    body.force = Vec3::new(MAX_COORD - 1.0, 0.0, 0.0);
    body.torque = Vec3::new(1.0, 2.0, 3.0);
    let force_snap = body;
    assert_eq!(
        body.apply_force_at_world_point(Vec3::new(10.0, 0.0, 0.0), Vec3::new(1.0, 5.0, 3.0)),
        Err(RigidBodyError::NumericalDivergence)
    );
    assert_eq!(body, force_snap);
}

// --- 1,000-Case Deterministic Property Suite -----------------------------

#[test]
fn seeded_1000_case_deterministic_invariants() {
    let mut seed: u64 = 0x4741_4d45_3430_646f;
    let dt = 0.01;

    for case in 0..1000 {
        let m = next_range(&mut seed, 0.5, 50.0);
        let ixx = next_range(&mut seed, 0.5, 10.0);
        let iyy = next_range(&mut seed, 0.5, 10.0);
        let izz = next_range(&mut seed, 0.5, 10.0);
        let mass = MassProperties::mass_props_from_diagonal(m, ixx, iyy, izz).unwrap();

        let pos = Vec3::new(
            next_range(&mut seed, -100.0, 100.0),
            next_range(&mut seed, -100.0, 100.0),
            next_range(&mut seed, -100.0, 100.0),
        );
        let axis = Vec3::new(
            next_range(&mut seed, -1.0, 1.0),
            next_range(&mut seed, -1.0, 1.0),
            next_range(&mut seed, -1.0, 1.0) + 2.0,
        );
        let angle = next_range(&mut seed, -trig::PI, trig::PI);
        let q = Quat::from_axis_angle(axis, angle).unwrap();

        let p0 = Vec3::new(
            next_range(&mut seed, -20.0, 20.0),
            next_range(&mut seed, -20.0, 20.0),
            next_range(&mut seed, -20.0, 20.0),
        );
        let l0 = Vec3::new(
            next_range(&mut seed, -5.0, 5.0),
            next_range(&mut seed, -5.0, 5.0),
            next_range(&mut seed, -5.0, 5.0),
        );
        let f = Vec3::new(
            next_range(&mut seed, -10.0, 10.0),
            next_range(&mut seed, -10.0, 10.0),
            next_range(&mut seed, -10.0, 10.0),
        );
        let tau = Vec3::new(
            next_range(&mut seed, -5.0, 5.0),
            next_range(&mut seed, -5.0, 5.0),
            next_range(&mut seed, -5.0, 5.0),
        );

        let integrator = if case % 2 == 0 {
            IntegratorKind::VelocityVerlet
        } else {
            IntegratorKind::RK4
        };

        let mut s1 = RigidBodyState::body_new(pos, q).unwrap();
        s1.linear_momentum = p0;
        s1.angular_momentum = l0;
        s1.apply_world_force(f).unwrap();
        s1.apply_world_torque(tau).unwrap();
        let mut s2 = s1;

        s1.step_in_place(&mass, dt, integrator).unwrap();
        s2.step_in_place(&mass, dt, integrator).unwrap();

        // Bit-exact deterministic replay
        assert_eq!(
            s1.state_digest_fnv1a().unwrap(),
            s2.state_digest_fnv1a().unwrap()
        );
        // Unit quaternion invariant
        assert!(approx(s1.orientation.quat_norm_sq().unwrap(), 1.0, 1e-12));
        // Exact impulse conservation for constant F, tau
        let expected_p = p0.vec3_add(f.vec3_scale(dt).unwrap()).unwrap();
        let expected_l = l0.vec3_add(tau.vec3_scale(dt).unwrap()).unwrap();
        assert!(vec3_approx(s1.linear_momentum, expected_p, 1e-11));
        assert!(vec3_approx(s1.angular_momentum, expected_l, 1e-11));
        // Non-negative kinetic energy
        assert!(s1.kinetic_energy(&mass).unwrap() >= -EPSILON);
    }
}
