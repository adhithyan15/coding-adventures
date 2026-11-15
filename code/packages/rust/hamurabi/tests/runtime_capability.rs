//! # VER08b runtime capability conformance for `hamurabi`.
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
use hamurabi::{
    format_report, new_game, step_year, validate_order, GameStatus, HamurabiError, TurnOrder,
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

        let state = new_game(1337);
        checks.push(check(
            "new_game initializes playing state",
            state.status == GameStatus::Playing,
        ));

        let order = TurnOrder {
            acres_delta: 0,
            bushels_to_feed: 2000,
            acres_to_plant: 800,
        };
        let val_res = validate_order(&state, &order);
        checks.push(check("validate_order legal order", val_res.is_ok()));

        let step_res = step_year(&state, &order);
        checks.push(check("step_year legal order", step_res.is_ok()));

        if let Ok(next_state) = step_res {
            let report = format_report(&next_state);
            checks.push(check(
                "format_report produces non-empty text",
                !report.is_empty(),
            ));
        }

        let bad_order = TurnOrder {
            acres_delta: 1000,
            bushels_to_feed: 2000,
            acres_to_plant: 800,
        };
        let err_res = step_year(&state, &bad_order);
        checks.push(check(
            "step_year rejects unaffordable land",
            matches!(err_res, Err(HamurabiError::InsufficientBushelsForLand)),
        ));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/hamurabi",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
