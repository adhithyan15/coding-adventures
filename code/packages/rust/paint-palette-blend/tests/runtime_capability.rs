//! # VER08b runtime capability conformance for `paint-palette-blend`.
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
use paint_palette_blend::{
    blend_pixel, blend_rgba_scanline, blend_solid_span, blit_indexed_image, blit_indexed_scanline,
    diff_paint_scene_into, BlendMode, IndexedImage, PaintScene, PatchKind, PatchOp, RgbaTarget,
    OPAQUE_BLIT,
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
        let palette = [[0u8, 0, 0, 0], [255, 0, 0, 255], [0, 255, 0, 255]];

        let px = blend_pixel(BlendMode::Add, [10, 10, 10, 255], [1, 2, 3, 255], 255);
        checks.push(check("blend_pixel adds", px == [11, 12, 13, 255]));

        let indices = [1u8, 2, 1, 2];
        let mut row = [0u8; 16];
        let r = blit_indexed_scanline(&mut row, &indices, &palette, &OPAQUE_BLIT);
        checks.push(check("blit_indexed_scanline succeeds", r == Ok(4)));

        let pixels = [1u8, 2, 2, 1];
        let img = IndexedImage {
            width: 2,
            height: 2,
            stride: 2,
            pixels: &pixels,
        };
        let mut fb = [0u8; 16];
        let mut target = RgbaTarget {
            width: 2,
            height: 2,
            stride: 8,
            pixels: &mut fb,
        };
        let r = blit_indexed_image(&mut target, 0, 0, &img, &palette, &OPAQUE_BLIT);
        checks.push(check("blit_indexed_image succeeds", r == Ok(4)));

        let mut span = [5u8; 8];
        checks.push(check(
            "blend_solid_span succeeds",
            blend_solid_span(&mut span, [1, 1, 1, 255], BlendMode::Screen, 200) == Ok(2),
        ));
        let src = [9u8; 8];
        checks.push(check(
            "blend_rgba_scanline succeeds",
            blend_rgba_scanline(&mut span, &src, BlendMode::Multiply, 255) == Ok(2),
        ));

        let scene = PaintScene::new(16.0, 16.0);
        let mut ops = [PatchOp {
            kind: PatchKind::Added,
            old_index: 0,
            new_index: 0,
        }; 2];
        let d = diff_paint_scene_into(&scene, &scene, &mut ops);
        checks.push(check(
            "diff_paint_scene_into succeeds",
            d.map(|s| s.ops_written == 0).unwrap_or(false),
        ));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/paint-palette-blend",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
