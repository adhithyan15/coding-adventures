//! # VER08b runtime capability conformance for `tilemap2d`.
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
use std::path::PathBuf;
use tilemap2d::{
    aabb_attrs, aabb_blocked, aabb_first_hit, aabb_has_attr, attr_at_point, cull_layer,
    layer_scroll, lower_layer_cells_into, lower_layers_to_paint_scene, render_layer_scanline,
    render_layers_to_container, screen_to_layer, Aabb, BitDepth, Camera, MotionHint, Palette,
    PixelContainer, TileBatchCell, TileCell, TileLayer, Tileset, Viewport, ATTR_SOLID,
};

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

        let cells = [
            TileCell::tile(1).with_attr(ATTR_SOLID),
            TileCell::tile(2).with_flip(true, false).with_bank(1),
            TileCell::tile(0),
            TileCell::tile(1).with_flip(false, true),
        ];
        let layer = TileLayer::attach(&cells, 2, 2, 4, 4);
        checks.push(check("TileLayer::attach succeeds", layer.is_ok()));
        let Ok(layer) = layer else {
            return checks;
        };
        let view = Viewport {
            width: 8,
            height: 8,
        };
        let cam = Camera { x: 3, y: 1 };

        checks.push(check(
            "layer_scroll succeeds",
            layer_scroll(&layer, view, cam).is_ok(),
        ));
        let rect = cull_layer(&layer, view, cam);
        checks.push(check("cull_layer succeeds", rect.is_ok()));
        checks.push(check(
            "screen_to_layer succeeds",
            screen_to_layer(&layer, view, cam, 1, 1).is_ok(),
        ));

        checks.push(check(
            "attr_at_point reports solid",
            attr_at_point(&layer, 0, 0) == ATTR_SOLID,
        ));
        let b = Aabb {
            x: 0,
            y: 0,
            w: 6,
            h: 6,
        };
        checks.push(check("aabb_attrs succeeds", aabb_attrs(&layer, b).is_ok()));
        checks.push(check(
            "aabb_has_attr succeeds",
            aabb_has_attr(&layer, b, ATTR_SOLID) == Ok(true),
        ));
        checks.push(check(
            "aabb_first_hit succeeds",
            matches!(aabb_first_hit(&layer, b, ATTR_SOLID), Ok(Some(_))),
        ));
        checks.push(check(
            "aabb_blocked succeeds",
            aabb_blocked(
                &layer,
                b,
                MotionHint {
                    moving_down: true,
                    prev_bottom: 0,
                },
            ) == Ok(true),
        ));

        if let Ok(rect) = rect {
            let mut out = [TileBatchCell::new(0, 0, 0); 16];
            let tints = [[255u8, 0, 0], [0, 255, 0]];
            checks.push(check(
                "lower_layer_cells_into succeeds",
                lower_layer_cells_into(&layer, &rect, &tints, &mut out).is_ok(),
            ));
        }
        checks.push(check(
            "lower_layers_to_paint_scene succeeds",
            lower_layers_to_paint_scene(
                &[layer],
                view,
                cam,
                &[[255, 0, 0], [0, 255, 0]],
                "#000000",
            )
            .is_ok(),
        ));

        let tile_data = [0x12u8; 4 * 2 * 3];
        let tileset = Tileset::attach_tiles(BitDepth::Bpp4, &tile_data, 4, 4, 3);
        checks.push(check("Tileset::attach_tiles succeeds", tileset.is_ok()));
        if let Ok(tileset) = tileset {
            let mut bank = Palette::blank(BitDepth::Bpp4);
            for i in 0..16u8 {
                let _ = bank.set_entry(i, [i * 16, 0, 0, 255]);
            }
            let banks = [bank, bank];
            let mut dst = PixelContainer::new(8, 8);
            checks.push(check(
                "render_layers_to_container succeeds",
                render_layers_to_container(&[layer], &tileset, &banks, cam, None, &mut dst).is_ok(),
            ));
            let mut line = [0u8; 32];
            checks.push(check(
                "render_layer_scanline succeeds",
                render_layer_scanline(&layer, &tileset, &banks, view, cam, 2, &mut line).is_ok(),
            ));
        }

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/tilemap2d",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
