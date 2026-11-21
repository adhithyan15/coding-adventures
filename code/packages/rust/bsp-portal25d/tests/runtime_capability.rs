//! # VER08b runtime capability conformance for `bsp-portal25d`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation). This test proves it at run time: the full public API is
//! exercised inside a process that has been sandboxed, mechanically from that
//! manifest, to the minimal armed baseline (allocator + exit + TAP output).
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports
//! a pass it did not earn. Set `RCG_STRICT=1` to turn not-live into failure.

#![deny(unsafe_code)]

use bsp_portal25d::{
    frame_digest_fnv1a, render_frame_into, scene_digest_fnv1a, BspTree, Camera25D, Linedef, Map25D,
    RenderConfig, RenderScratch, Sector, SurfaceStyle, TextureRgba, Vec2,
};
use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use pixel_container::PixelContainer;
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
    let s0 = Sector::sector_with_surfaces(
        0.0,
        4.0,
        SurfaceStyle::surface_textured(1, [40, 80, 40, 255]),
        SurfaceStyle::surface_solid([40, 40, 90, 255]),
        255,
    )
    .expect("valid sector 0");
    let s1 = Sector::sector_new(1.0, 3.0, [70, 110, 70, 255], [70, 70, 120, 255], 180)
        .expect("valid sector 1");
    let w = SurfaceStyle::surface_textured(0, [180, 80, 60, 255]);
    let up = SurfaceStyle::surface_solid([220, 200, 60, 255]);
    let lo = SurfaceStyle::surface_solid([200, 100, 220, 255]);

    let linedefs = vec![
        Linedef::linedef_solid(Vec2::vec2_new(0.0, 0.0), Vec2::vec2_new(0.0, 10.0), 0, w)
            .expect("l0"),
        Linedef::linedef_solid(Vec2::vec2_new(0.0, 10.0), Vec2::vec2_new(10.0, 10.0), 0, w)
            .expect("l1"),
        Linedef::linedef_portal(
            Vec2::vec2_new(10.0, 10.0),
            Vec2::vec2_new(10.0, 0.0),
            0,
            Some(1),
            w,
            up,
            lo,
        )
        .expect("l2"),
        Linedef::linedef_solid(Vec2::vec2_new(10.0, 0.0), Vec2::vec2_new(0.0, 0.0), 0, w)
            .expect("l3"),
        Linedef::linedef_solid(Vec2::vec2_new(10.0, 10.0), Vec2::vec2_new(20.0, 10.0), 1, w)
            .expect("l4"),
        Linedef::linedef_solid(Vec2::vec2_new(20.0, 10.0), Vec2::vec2_new(20.0, 0.0), 1, w)
            .expect("l5"),
        Linedef::linedef_solid(Vec2::vec2_new(20.0, 0.0), Vec2::vec2_new(10.0, 0.0), 1, w)
            .expect("l6"),
    ];

    let map = Map25D::map_new(vec![s0, s1], linedefs).expect("valid map");
    let bsp = BspTree::bsp_build(&map).expect("valid bsp");
    let textures = vec![
        TextureRgba::texture_checkerboard(8, 8, 2, [220, 120, 80, 255], [100, 50, 30, 255])
            .expect("wall tex"),
        TextureRgba::texture_solid(8, 8, [60, 90, 60, 255]).expect("floor tex"),
    ];
    let mut framebuffer = PixelContainer::new(64, 48);
    let mut scratch =
        RenderScratch::scratch_for_frame_and_map(&framebuffer, &map, &bsp).expect("scratch");

    Box::new(move || {
        let mut checks = Vec::new();

        let cam = Camera25D::camera_from_yaw_fov(5.0, 5.0, 2.0, 0.0, trig::PI / 3.0);
        checks.push(check("camera_from_yaw_fov", cam.is_ok()));

        let leaf = bsp.bsp_find_subsector(Vec2::vec2_new(4.0, 5.0));
        checks.push(check("bsp_find_subsector", leaf.is_ok()));

        let render_ok = match cam {
            Ok(c) => {
                let config = RenderConfig::render_config_default();
                render_frame_into(
                    &map,
                    &bsp,
                    &c,
                    &textures,
                    &config,
                    &mut scratch,
                    &mut framebuffer,
                )
                .map(|stats| stats.columns_drawn == 64 && stats.portals_traversed >= 1)
                .unwrap_or(false)
            }
            Err(_) => false,
        };
        checks.push(check("render_frame_into portal pipeline", render_ok));
        checks.push(check(
            "digests non-zero",
            frame_digest_fnv1a(&framebuffer) != 0 && scene_digest_fnv1a(&map, &bsp) != 0,
        ));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/bsp-portal25d",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
