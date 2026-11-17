//! # VER08b runtime capability conformance for `paint-batch-tiles`.
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
use paint_batch_tiles::{
    diff_paint_scenes, diff_sprite_batches_into, lower_sprites_to_paint_scene,
    lower_tile_batch_to_paint_scene, DeltaKind, SpriteDeltaOp, SpriteQuad, TileBatchCell,
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

        let sprites = [
            SpriteQuad::new(1, 10.0, 10.0, 16.0, 16.0),
            SpriteQuad::new(2, 20.0, 20.0, 16.0, 16.0),
        ];

        let lower_res = lower_sprites_to_paint_scene(320.0, 240.0, "#000", &sprites);
        checks.push(check(
            "lower_sprites_to_paint_scene succeeds",
            lower_res.is_ok(),
        ));

        let cells = [TileBatchCell::new(0, 0, 1), TileBatchCell::new(1, 0, 2)];
        let tile_res = lower_tile_batch_to_paint_scene(100.0, 100.0, 16.0, 16.0, "#000", &cells);
        checks.push(check(
            "lower_tile_batch_to_paint_scene succeeds",
            tile_res.is_ok(),
        ));

        let mut ops = [SpriteDeltaOp {
            kind: DeltaKind::Unchanged,
            id: 0,
            old_index: 0,
            new_index: 0,
        }; 4];
        let diff_res = diff_sprite_batches_into(&sprites, &sprites, &mut ops);
        checks.push(check("diff_sprite_batches_into succeeds", diff_res.is_ok()));

        if let Ok(scene1) = &lower_res {
            let scene_diff = diff_paint_scenes(scene1, scene1);
            checks.push(check("diff_paint_scenes succeeds", scene_diff.is_ok()));
        }

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/paint-batch-tiles",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
