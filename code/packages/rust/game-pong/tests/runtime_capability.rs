//! # VER08b runtime capability conformance for `game-pong`.
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
use game_pong::{
    build_pong_sprites, default_config, deterministic_prng_next, deterministic_serve_angle,
    lower_pong_to_paint_scene, new_game, render_pong_audio_into, step_pong, step_pong_with_clock,
    validate_config, validate_state, PaddleAction, PcmRingBuffer, PongInput, SfxBank, SpriteItem,
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

        // 1. Config validation
        let config = default_config();
        checks.push(check("default_config valid", validate_config(&config).is_ok()));

        // 2. New game state
        let state_res = new_game(config, 0x1234_5678);
        checks.push(check("new_game initializes state", state_res.is_ok()));

        if let Ok(mut state) = state_res {
            checks.push(check("validate_state initial", validate_state(&state).is_ok()));

            // 3. Step pong simulation
            let mut sfx_bank = SfxBank::<4>::empty(44_100).expect("valid sfx bank");
            let input = PongInput::player_vs_ai(PaddleAction::Up);
            let step_res = step_pong(&mut state, &input, &mut sfx_bank);
            checks.push(check("step_pong executes", step_res.is_ok()));

            // 4. Fixed timestep integration with clock
            let clock_cfg = game_clock::GameClockConfig::standard_60hz();
            if let Ok(mut clock) = game_clock::new_clock(clock_cfg) {
                let clock_step_res = step_pong_with_clock(
                    &mut clock,
                    16_666_667,
                    &mut state,
                    &input,
                    &mut sfx_bank,
                );
                checks.push(check("step_pong_with_clock executes", clock_step_res.is_ok()));
            }

            // 5. Zero-alloc sprite emission
            let mut sprites = [SpriteItem::new(0, 0.0, 0.0, 0.0, 0.0); 64];
            let sprite_count = build_pong_sprites(&state, &mut sprites);
            checks.push(check("build_pong_sprites non-zero", sprite_count > 0));

            // 6. PaintScene lowering
            let scene_res = lower_pong_to_paint_scene(&state);
            checks.push(check("lower_pong_to_paint_scene generates scene", scene_res.is_ok()));

            // 7. Audio rendering helper
            let mut ring = PcmRingBuffer::<256>::empty();
            let mut scratch = [0i16; 64];
            let audio_res = render_pong_audio_into(&mut sfx_bank, &mut ring, &mut scratch);
            checks.push(check("render_pong_audio_into executes", audio_res.is_ok()));
        }

        // 8. Deterministic PRNG
        let mut prng = 42u64;
        let r1 = deterministic_prng_next(&mut prng);
        let r2 = deterministic_prng_next(&mut prng);
        checks.push(check("prng changes state", r1 != r2));
        let angle = deterministic_serve_angle(&mut prng, 0.8);
        checks.push(check("serve angle bounded", (-0.8..=0.8).contains(&angle)));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/game-pong",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
