//! # VER08b runtime capability conformance for `game-clock`.
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
use game_clock::{
    advance_frame, new_clock, validate_state, ClockError, GameClockConfig, DEFAULT_60HZ_DT_NS,
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

        let cfg = GameClockConfig::standard_60hz();
        checks.push(check("config validate", cfg.validate().is_ok()));

        let state_res = new_clock(cfg);
        checks.push(check("new_clock initializes state", state_res.is_ok()));

        if let Ok(state) = state_res {
            let val_res = validate_state(&state);
            checks.push(check("validate_state initial state", val_res.is_ok()));

            let adv_res = advance_frame(&state, DEFAULT_60HZ_DT_NS);
            checks.push(check("advance_frame steady 60hz", adv_res.is_ok()));

            if let Ok(res) = adv_res {
                checks.push(check("steps_to_run is 1", res.steps_to_run == 1));
                checks.push(check("alpha_f64 is 0.0", res.alpha_f64() == 0.0));
            }

            let spike_res = advance_frame(&state, 1_000_000_000);
            checks.push(check("advance_frame spike delta", spike_res.is_ok()));
            if let Ok(res) = spike_res {
                checks.push(check("spiral clamp fired", res.spiral_of_death_clamped));
            }

            let err_res = advance_frame(&state, u64::MAX);
            checks.push(check(
                "advance_frame u64::MAX succeeds with clamp",
                err_res.is_ok(),
            ));
        }

        let bad_cfg = GameClockConfig::from_hz(0, 8);
        checks.push(check(
            "from_hz rejects 0",
            matches!(bad_cfg, Err(ClockError::ZeroFixedTimestep)),
        ));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/game-clock",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
