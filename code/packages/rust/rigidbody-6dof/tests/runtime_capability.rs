//! # VER08b runtime capability conformance for `rigidbody-6dof`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation). This test proves it at run time: the full public API is
//! exercised inside a process that has been sandboxed, mechanically from that
//! manifest, to the minimal armed baseline (allocator + exit + TAP output).
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports
//! a pass it did not earn. Set `RCG_STRICT=1` to turn not-live into failure.

#![deny(unsafe_code)]

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use math3d::{Plane, Quat, Vec3};
use rigidbody_6dof::{GroundContactParams, IntegratorKind, MassProperties, RigidBodyState};
use std::path::PathBuf;

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

fn prepare() -> Exercise {
    std::panic::set_hook(Box::new(|_| {}));

    Box::new(move || {
        let mut checks = Vec::new();

        let mass = MassProperties::mass_props_from_solid_box(5.0, Vec3::new(1.0, 0.5, 1.0));
        checks.push(check("mass properties construction", mass.is_ok()));

        let body_res = RigidBodyState::body_new(Vec3::new(0.0, 2.0, 0.0), Quat::quat_identity());
        checks.push(check("body state construction", body_res.is_ok()));

        if let (Ok(m), Ok(mut b)) = (mass, body_res) {
            let f_ok = b.apply_world_force(Vec3::new(0.0, -49.05, 0.0)).is_ok()
                && b.apply_body_torque(Vec3::new(0.2, 0.4, 0.0)).is_ok();
            checks.push(check("force and torque application", f_ok));

            let step_ok = b.step_in_place(&m, 1.0 / 60.0, IntegratorKind::RK4).is_ok()
                && b.step_in_place(&m, 1.0 / 60.0, IntegratorKind::VelocityVerlet)
                    .is_ok();
            checks.push(check("verlet and rk4 step_in_place", step_ok));

            let ground = Plane::plane_from_point_normal(Vec3::zero(), Vec3::unit_y());
            let contact_ok = ground
                .ok()
                .and_then(|p| {
                    GroundContactParams::contact_params_penalty(p, 5000.0, 200.0, 0.5, 2).ok()
                })
                .map(|params| {
                    let probes = [Vec3::new(0.0, -0.5, 0.0)];
                    b.step_with_contacts_report_in_place(
                        &m,
                        1.0 / 60.0,
                        IntegratorKind::VelocityVerlet,
                        &params,
                        &probes,
                    )
                    .is_ok()
                })
                .unwrap_or(false);
            checks.push(check("ground contact resolution", contact_ok));

            let diag_ok = b.kinetic_energy(&m).is_ok()
                && b.angular_velocity_body(&m).is_ok()
                && b.state_digest_fnv1a().is_ok();
            checks.push(check("diagnostics and canonical digest", diag_ok));
        }

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/rigidbody-6dof",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
