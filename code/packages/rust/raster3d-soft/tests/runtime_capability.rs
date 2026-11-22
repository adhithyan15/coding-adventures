//! # VER08b runtime capability conformance for `raster3d-soft`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation). This test exercises the public rasterizer API inside the
//! mechanical capability-runtime-conformance sandbox.
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); set `RCG_STRICT=1`
//! on Linux to enforce live seccomp/Landlock sandboxing.

#![deny(unsafe_code)]

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use math3d::{Mat4, Vec3};
use pixel_container::PixelContainer;
use raster3d_soft::{
    clear_depth_f32, clear_depth_f64, clear_framebuffer, draw_indexed_mesh_into,
    draw_triangle_into, frame_digest_fnv1a, validate_clip_w, validate_framebuffer_dim,
    validate_texture_dim, validate_triangle_count, validate_vertex_index, DepthBufferMut,
    RasterConfig, RasterScratch, ShadingMode, Texture, TextureWrap, Vertex,
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

        let validators_ok = validate_framebuffer_dim(32).is_ok()
            && validate_texture_dim(16).is_ok()
            && validate_triangle_count(2).is_ok()
            && validate_vertex_index(0, 3).is_ok()
            && validate_clip_w(1.0).is_ok();
        checks.push(check("warrant contract validators", validators_ok));

        let mut fb = PixelContainer::new(32, 32);
        let mut zb64 = vec![1.0_f64; 32 * 32];
        let mut zb32 = vec![1.0_f32; 32 * 32];
        let clear_ok = clear_framebuffer(&mut fb, [0, 0, 0, 255]).is_ok()
            && clear_depth_f64(&mut zb64, 1.0).is_ok()
            && clear_depth_f32(&mut zb32, 1.0).is_ok();
        checks.push(check("framebuffer and zbuffer clear", clear_ok));

        let mut tex_pc = PixelContainer::new(4, 4);
        tex_pc.fill(180, 120, 60, 255);
        let tex = Texture::texture_from_container(&tex_pc);
        let tex_ok = tex
            .as_ref()
            .map(|t| {
                t.texture_sample_nearest(0.5, 0.5, TextureWrap::Repeat)
                    .is_ok()
                    && t.texture_sample_bilinear(0.5, 0.5, TextureWrap::Clamp)
                        .is_ok()
            })
            .unwrap_or(false);
        checks.push(check("texture creation and sampling", tex_ok));

        let v0 = Vertex::vertex_new(
            Vec3::new(-0.5, -0.5, 0.0),
            Vec3::unit_z(),
            [1.0, 0.2, 0.2],
            0.0,
            0.0,
        );
        let v1 = Vertex::vertex_new(
            Vec3::new(0.5, -0.5, 0.0),
            Vec3::unit_z(),
            [0.2, 1.0, 0.2],
            1.0,
            0.0,
        );
        let v2 = Vertex::vertex_new(
            Vec3::new(0.0, 0.5, 0.0),
            Vec3::unit_z(),
            [0.2, 0.2, 1.0],
            0.5,
            1.0,
        );
        let cfg = RasterConfig::raster_config_new(Mat4::mat4_identity());
        let tri_ok = cfg
            .as_ref()
            .map(|c| {
                draw_triangle_into(
                    &mut fb,
                    DepthBufferMut::F64(&mut zb64),
                    c,
                    None,
                    &v0,
                    &v1,
                    &v2,
                )
                .is_ok()
            })
            .unwrap_or(false);
        checks.push(check("draw_triangle_into gouraud", tri_ok));

        let mesh_ok = match (cfg, tex, RasterScratch::scratch_with_capacity(3)) {
            (Ok(mut c), Ok(t), Ok(mut s)) => {
                c.shading = ShadingMode::TexturedBilinear;
                draw_indexed_mesh_into(
                    &mut fb,
                    DepthBufferMut::F32(&mut zb32),
                    &c,
                    Some(&t),
                    &[v0, v1, v2],
                    &[0, 1, 2],
                    &mut s,
                )
                .is_ok()
            }
            _ => false,
        };
        checks.push(check("draw_indexed_mesh_into textured", mesh_ok));
        checks.push(check(
            "frame_digest_fnv1a non-zero",
            frame_digest_fnv1a(&fb) != 0,
        ));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/raster3d-soft",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
