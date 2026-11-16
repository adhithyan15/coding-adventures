//! # VER08b runtime capability conformance for `lunar-lander`.
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
use lunar_lander::{
    isqrt_u128, new_flight, render_telemetry_ascii, step_burn, to_paint_scene, validate_burn,
    BurnCommand, FlightStatus, LanderError,
};
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

        let state = new_flight();
        checks.push(check(
            "new_flight initializes in-flight state",
            state.status == FlightStatus::InFlight,
        ));

        let cmd = BurnCommand {
            burn_rate_lbs_s: 100,
            duration_seconds: 5,
        };
        let val_res = validate_burn(&state, &cmd);
        checks.push(check("validate_burn legal command", val_res.is_ok()));

        let step_res = step_burn(&state, &cmd);
        checks.push(check("step_burn legal command", step_res.is_ok()));

        if let Ok(next_state) = step_res {
            let hud = render_telemetry_ascii(&next_state, 100);
            checks.push(check(
                "render_telemetry_ascii produces 5-line HUD",
                hud.lines().count() == 5,
            ));
            let scene = to_paint_scene(&next_state, 100);
            checks.push(check(
                "to_paint_scene produces valid scene",
                scene.width == 76.0 && scene.height == 5.0,
            ));
        }

        let bad_cmd = BurnCommand {
            burn_rate_lbs_s: 250,
            duration_seconds: 5,
        };
        let err_res = step_burn(&state, &bad_cmd);
        checks.push(check(
            "step_burn rejects excessive burn rate",
            matches!(err_res, Err(LanderError::ExcessiveBurnRate)),
        ));

        let sqrt_val = isqrt_u128(10_274_400_000_000);
        checks.push(check(
            "isqrt_u128 computes exact root",
            sqrt_val == 3_205_370,
        ));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/lunar-lander",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
