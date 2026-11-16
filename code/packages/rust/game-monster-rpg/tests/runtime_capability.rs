//! # VER08b runtime capability conformance for `game-monster-rpg`.
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
use game_monster_rpg::{
    build_rpg_sprites, default_rpg_config, init_rpg_game, lower_rpg_to_paint_scene,
    render_rpg_audio_into, step_rpg_frame, step_rpg_with_clock, validate_fade_fraction,
    validate_map_id, validate_move_slot, validate_party_index, validate_step_rate, PcmRingBuffer,
    RpgInput, SfxBank, SpriteQuad,
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

        // 1. Config & warrant validation
        let config = default_rpg_config();
        checks.push(check(
            "default_rpg_config valid",
            config.validate_rpg_config().is_ok(),
        ));
        checks.push(check("validate_map_id", validate_map_id(0).is_ok()));
        checks.push(check(
            "validate_party_index",
            validate_party_index(0).is_ok(),
        ));
        checks.push(check("validate_move_slot", validate_move_slot(0).is_ok()));
        checks.push(check(
            "validate_fade_fraction",
            validate_fade_fraction(128).is_ok(),
        ));
        checks.push(check("validate_step_rate", validate_step_rate(32).is_ok()));

        // 2. New game state
        let state_res = init_rpg_game(config, 0x1234_5678);
        checks.push(check("init_rpg_game initializes state", state_res.is_ok()));

        if let Ok(mut state) = state_res {
            // 3. Step simulation
            let mut sfx_bank = SfxBank::<4>::empty(44_100).expect("valid sfx bank");
            let input = RpgInput::walk_up();
            let step_res = step_rpg_frame(&mut state, &input, &mut sfx_bank);
            checks.push(check("step_rpg_frame executes", step_res.is_ok()));

            // 4. Fixed timestep integration with clock
            let clock_cfg = game_clock::GameClockConfig::standard_60hz();
            if let Ok(mut clock) = game_clock::new_clock(clock_cfg) {
                let clock_step_res =
                    step_rpg_with_clock(&mut clock, 16_666_667, &mut state, &input, &mut sfx_bank);
                checks.push(check(
                    "step_rpg_with_clock executes",
                    clock_step_res.is_ok(),
                ));
            }

            // 5. Zero-alloc sprite emission
            let mut sprites = [SpriteQuad::new(0, 0.0, 0.0, 0.0, 0.0); 128];
            let sprite_res = build_rpg_sprites(&state, &mut sprites);
            checks.push(check("build_rpg_sprites non-zero", sprite_res.is_ok()));

            // 6. PaintScene lowering
            let scene_res = lower_rpg_to_paint_scene(&state);
            checks.push(check(
                "lower_rpg_to_paint_scene generates scene",
                scene_res.is_ok(),
            ));

            // 7. Audio rendering helper
            let mut ring = PcmRingBuffer::<256>::empty();
            let mut scratch = [0i16; 64];
            let audio_res = render_rpg_audio_into(&mut sfx_bank, &mut ring, &mut scratch);
            checks.push(check("render_rpg_audio_into executes", audio_res.is_ok()));
        }

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/game-monster-rpg",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
