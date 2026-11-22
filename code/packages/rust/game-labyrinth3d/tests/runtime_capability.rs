//! # VER08b runtime capability conformance for `game-labyrinth3d`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation). This test exercises the state machine and both renderers
//! inside the capability-runtime-conformance sandbox.
//!
//! Phase A framebuffers are kept at `32x32` (`< 1 MiB`) so glibc never issues
//! `mremap` inside seccomp `RCG001`.

#![deny(unsafe_code)]

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use game_labyrinth3d::{
    frame_digest_fnv1a, render_frame_into, state_digest_fnv1a, step_in_place, validate_door_count,
    validate_entity_count, validate_frame_dim, validate_grid_dim, validate_key_id,
    validate_pickup_count, validate_player_radius_micro, LabyrinthAssets, LabyrinthInput,
    LabyrinthRenderScratch, LabyrinthState, PixelContainer, RenderMode,
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

        let validators_ok = validate_grid_dim(11, 11).is_ok()
            && validate_frame_dim(32, 32).is_ok()
            && validate_entity_count(2).is_ok()
            && validate_door_count(1).is_ok()
            && validate_pickup_count(3).is_ok()
            && validate_key_id(0).is_ok()
            && validate_player_radius_micro(250_000).is_ok();
        checks.push(check("warrant contract validators", validators_ok));

        let state_res = LabyrinthState::state_build_demo(2026);
        let assets_res = LabyrinthAssets::assets_default();
        let mut fb = PixelContainer::new(32, 32);
        let mut zb = vec![1.0_f64; 32 * 32];

        let (step_ok, r25_ok, r3d_ok, digests_ok) = match (state_res, assets_res) {
            (Ok(mut st), Ok(assets)) => {
                let scratch_res = LabyrinthRenderScratch::rscratch_for_state(&st);
                let inp = LabyrinthInput::input_new(0.5, 0.0, 0.2, false, 16_666_667);
                let step_ok = inp
                    .map(|i| step_in_place(&mut st, i).is_ok())
                    .unwrap_or(false);
                match scratch_res {
                    Ok(mut scratch) => {
                        let d25 = render_frame_into(
                            &st,
                            &assets,
                            RenderMode::Raycast25D,
                            &mut scratch,
                            &mut zb,
                            &mut fb,
                        )
                        .map(|r| r.frame_digest)
                        .unwrap_or(0);
                        let d3d = render_frame_into(
                            &st,
                            &assets,
                            RenderMode::Software3D,
                            &mut scratch,
                            &mut zb,
                            &mut fb,
                        )
                        .map(|r| r.frame_digest)
                        .unwrap_or(0);
                        let sd = state_digest_fnv1a(&st);
                        let fd = frame_digest_fnv1a(&fb);
                        (step_ok, d25 != 0, d3d != 0, sd != 0 && fd != 0)
                    }
                    Err(_) => (step_ok, false, false, false),
                }
            }
            _ => (false, false, false, false),
        };

        checks.push(check("step_in_place simulation", step_ok));
        checks.push(check("render_frame_into Raycast25D", r25_ok));
        checks.push(check("render_frame_into Software3D", r3d_ok));
        checks.push(check("canonical FNV-1a digests", digests_ok));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/game-labyrinth3d",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
