//! # VER08b runtime capability conformance for `terrain-lod`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation). This test proves it at run time: the full public API is
//! exercised inside a process that has been sandboxed, mechanically from that
//! manifest, to the minimal armed baseline (allocator + exit + TAP output).
//!
//! Buffers are kept small (`33x33` heightfield, `2048`-node scratch, `8192`-u32
//! index buffer) so heap allocations stay well under glibc's `mremap` threshold
//! inside the armed seccomp sandbox (`RCG001`).

#![deny(unsafe_code)]

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use math3d::{Frustum, Mat4, Vec3};
use std::path::PathBuf;
use terrain_lod::{
    emit_mesh_into, mesh_digest_fnv1a, refine_into, Heightfield, HeightfieldMut, HeightfieldSpec,
    LodParams, RunwaySpec, TerrainScratch,
};

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

fn prepare() -> Exercise {
    std::panic::set_hook(Box::new(|_| {}));

    // Pre-size modest Phase A buffers well below 1 MiB to avoid Linux glibc mremap (`RCG001`).
    let spec = HeightfieldSpec::spec_new(5, 10.0, -160.0, -160.0).expect("valid spec"); // 33x33
    let vert_count = spec.spec_vertex_count().expect("vertex count");
    let mut heights = vec![0.0_f64; vert_count];
    let mut scratch = TerrainScratch::scratch_with_capacity(2048).expect("scratch");
    let mut indices = vec![0_u32; 8192];

    Box::new(move || {
        let mut checks = Vec::new();

        let synth_ok = {
            let mut hfm = HeightfieldMut::from_heights_mut(spec, &mut heights).expect("hfm");
            let ds = hfm.synthesize_diamond_square(42, 50.0, 20.0, 0.6).is_ok();
            let rw = RunwaySpec::runway_new(0.0, 0.0, 0.25, 100.0, 30.0, 15.0, 52.0)
                .and_then(|r| hfm.place_runway(&r))
                .is_ok();
            ds && rw
        };
        checks.push(check(
            "synthesize_diamond_square and place_runway",
            synth_ok,
        ));

        let hf = Heightfield::from_heights(spec, &heights).expect("hf");
        let query_ok = hf.height_at(0.0, 0.0).is_ok()
            && hf.normal_at(0.0, 0.0).is_ok()
            && hf.world_bounds().is_ok();
        checks.push(check("height_at, normal_at, and world_bounds", query_ok));

        let view = Mat4::look_at(
            Vec3::new(0.0, 90.0, 120.0),
            Vec3::new(0.0, 50.0, 0.0),
            Vec3::unit_y(),
        )
        .expect("view");
        let proj = Mat4::perspective(1.0, 1.33, 1.0, 1000.0).expect("proj");
        let vp = proj.mul(&view).expect("vp");
        let frustum = Frustum::from_view_proj(&vp).expect("frustum");

        let params = LodParams::lod_params_new(
            Vec3::new(0.0, 90.0, 120.0),
            0.02,
            1.0,
            5,
            2048,
            4096,
            Some(frustum),
        )
        .expect("params");

        let lod_ok = match refine_into(&hf, &params, &mut scratch) {
            Ok(stats) => {
                scratch.scratch_is_restricted()
                    && emit_mesh_into(&hf, &scratch, &mut indices, 4096)
                        .and_then(|n| mesh_digest_fnv1a(&hf, &indices[..n]))
                        .map(|_| stats.node_count > 0)
                        .unwrap_or(false)
            }
            Err(_) => false,
        };
        checks.push(check(
            "restricted quadtree refine_into, emit_mesh_into, and digest",
            lod_ok,
        ));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/terrain-lod",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
