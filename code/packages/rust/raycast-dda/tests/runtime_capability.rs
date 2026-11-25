//! # VER08b runtime capability conformance for `raycast-dda`.
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
use pixel_container::PixelContainer;
use raycast_dda::{
    cast_dda_ray, render_frame_into, Camera2D, GridMap, PlaneSurface, RenderConfig, Sprite,
    SpriteScratch, TextureRgba,
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

    // Allocate caller buffers during Phase A setup before sandboxing.
    let mut cells = vec![0u16; 8 * 8];
    for i in 0..8 {
        cells[i] = 1;
        cells[7 * 8 + i] = 1;
        cells[i * 8] = 1;
        cells[i * 8 + 7] = 1;
    }
    let grid = GridMap::grid_new(8, 8, cells).expect("valid 8x8 grid");
    let wall_tex =
        TextureRgba::texture_checkerboard(8, 8, 2, [220, 120, 80, 255], [100, 50, 30, 255])
            .expect("valid wall texture");
    let floor_tex = TextureRgba::texture_solid(8, 8, [60, 90, 60, 255]).expect("floor texture");
    let sprite_tex = TextureRgba::texture_solid(8, 8, [255, 255, 0, 255]).expect("sprite texture");
    let textures = vec![wall_tex, floor_tex, sprite_tex];
    let sprites = vec![Sprite::sprite_new(4.5, 3.5, 2).expect("valid sprite")];
    let mut scratch = SpriteScratch::scratch_with_capacity(4).expect("valid scratch");
    let mut zbuffer = vec![0.0; 64];
    let mut framebuffer = PixelContainer::new(64, 48);

    Box::new(move || {
        let mut checks = Vec::new();

        let cam = Camera2D::camera_from_yaw_fov(2.5, 3.5, 0.1, trig::PI / 3.0);
        checks.push(check("camera_from_yaw_fov", cam.is_ok()));

        let dda = cast_dda_ray(&grid, 2.5, 3.5, 1.0, 0.0);
        checks.push(check(
            "cast_dda_ray hits wall",
            dda.map(|h| h.hit && h.tile_id == 1).unwrap_or(false),
        ));

        let render_ok = match cam {
            Ok(c) => {
                let config = RenderConfig::render_config_with_planes(
                    PlaneSurface::Solid([20, 30, 50, 255]),
                    PlaneSurface::Textured { texture_index: 1 },
                    128,
                );
                render_frame_into(
                    &grid,
                    &c,
                    &textures,
                    &sprites,
                    &config,
                    &mut scratch,
                    &mut zbuffer,
                    &mut framebuffer,
                )
                .map(|stats| stats.columns_cast == 64 && stats.wall_hits == 64)
                .unwrap_or(false)
            }
            Err(_) => false,
        };
        checks.push(check("render_frame_into full pipeline", render_ok));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/raycast-dda",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
