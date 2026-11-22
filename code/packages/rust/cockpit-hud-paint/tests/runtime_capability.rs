//! # VER08b runtime capability conformance for `cockpit-hud-paint`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation). This test proves it at run time inside a sandboxed process.
//! Kept compact so no large heap reallocations (`mremap`) occur after arming.

#![deny(unsafe_code)]

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use cockpit_hud_paint::{
    build_hud_frame_into, paint_scene_digest_fnv1a, prepare_paint_scene, update_paint_scene_into,
    verify_paint_scene_finite, FlightTelemetry, HudConfig, HudScratch,
};
use math3d::{Quat, Vec3};
use paint_vm_runtime::analyze_scene;
use rigidbody_6dof::{MassProperties, RigidBodyState};
use std::path::PathBuf;

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

fn prepare() -> Exercise {
    std::panic::set_hook(Box::new(|_| {}));

    // Pre-allocate small scratch & retained scene before arming so the
    // exercised hot path executes with zero large reallocations / mremap.
    let cfg = HudConfig::hud_config_default(800.0, 600.0).expect("default config");
    let mut scratch = HudScratch::hud_scratch_with_capacity(256);
    let mut scene = prepare_paint_scene(&cfg).expect("prepare scene");

    Box::new(move || {
        let mut checks = Vec::with_capacity(6);

        let tel = FlightTelemetry::telemetry_level_flight();
        let build_ok = build_hud_frame_into(&tel, &cfg, &mut scratch).is_ok();
        checks.push(check("build_hud_frame_into", build_ok));

        let update_ok = update_paint_scene_into(&tel, &cfg, &mut scene).is_ok();
        checks.push(check("update_paint_scene_into", update_ok));

        let finite_ok = verify_paint_scene_finite(&scene).is_ok();
        checks.push(check("verify_paint_scene_finite", finite_ok));

        let reqs = analyze_scene(&scene);
        checks.push(check(
            "analyze_scene requirements",
            reqs.uses_rect && reqs.uses_line && reqs.uses_path && reqs.uses_text,
        ));

        let digest_ok =
            paint_scene_digest_fnv1a(&scene).is_ok() && scratch.hud_scratch_digest_fnv1a().is_ok();
        checks.push(check("canonical fnv1a digests", digest_ok));

        let rb_ok = MassProperties::mass_props_from_solid_box(500.0, Vec3::new(1.0, 0.5, 1.0))
            .ok()
            .and_then(|mass| {
                RigidBodyState::body_new(Vec3::new(0.0, 1200.0, 0.0), Quat::quat_identity())
                    .ok()
                    .and_then(|body| {
                        FlightTelemetry::telemetry_from_rigid_body(&body, &mass, 340.0, 100.0).ok()
                    })
            })
            .is_some();
        checks.push(check("rigidbody telemetry extraction", rb_ok));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/cockpit-hud-paint",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
