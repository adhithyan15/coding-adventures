//! # VER08b runtime capability conformance for `render3d-gpu-bridge` (GAME44).
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation).
//!
//! Critical seccomp `RCG001` invariants:
//! - **NEVER** calls `gpu_runtime::Runtime::detect()` or `Runtime::global()`
//!   inside `prepare()` or the sandboxed `Exercise` closure (no `dlopen` of
//!   `libcuda.so` or Metal IOKit probes). Uses `GpuRuntime::cpu_only()` only.
//! - Keeps Phase A framebuffers `<= 64x64` (`< 1 MiB`) and pre-allocates
//!   buffers before arming so glibc does not invoke `mremap`.

#![forbid(unsafe_code)]

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use math3d::{Mat4, Vec3};
use pixel_container::PixelContainer;
use render3d_gpu_bridge::{
    bridge_depth_resolve_shaders, bridge_postprocess_shaders, execute_render_pass_into,
    execute_render_pass_with_runtime, resolve_depth_to_rgba_with_runtime, validate_draw_call_count,
    validate_framebuffer_dim, validate_handle_index, validate_index_range,
    validate_total_triangles, BackendCapabilities, BackendPreference, CameraUniforms,
    DepthBufferMut, DrawCall, GpuRuntime, LightRig, MaterialBinding, MeshSlice, PostProcessMode,
    RenderBackendKind, RenderBridgeScratch, RenderPass, RenderTargetSpec, ShadingMode, Vertex,
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

    // Pre-create compact <= 64x64 buffers and cpu_only runtime prior to arming.
    let cpu_rt = GpuRuntime::cpu_only();
    let mut fb_sw = PixelContainer::new(32, 32);
    let mut fb_rt = PixelContainer::new(32, 32);
    let mut fb_depth = PixelContainer::new(32, 32);
    let mut zb64 = vec![1.0_f64; 32 * 32];
    let mut zb32 = vec![1.0_f32; 32 * 32];
    let mut scratch_sw =
        RenderBridgeScratch::bridge_scratch_with_capacity(8, 32, 32).expect("scratch sw");
    let mut scratch_rt =
        RenderBridgeScratch::bridge_scratch_with_capacity(8, 32, 32).expect("scratch rt");

    Box::new(move || {
        let mut checks = Vec::with_capacity(6);

        let validators_ok = validate_framebuffer_dim(32).is_ok()
            && validate_draw_call_count(4).is_ok()
            && validate_total_triangles(2).is_ok()
            && validate_handle_index(0, 1).is_ok()
            && validate_index_range(0, 3, 3).is_ok();
        checks.push(check("warrant contract validators", validators_ok));

        let caps = BackendCapabilities::backend_caps_detect_safe(Some(&cpu_rt));
        let caps_ok = caps.cap_set_get(RenderBackendKind::Software).available
            && caps.cap_set_get(RenderBackendKind::GpuRuntimeCpu).available
            && bridge_postprocess_shaders().cpu.is_some()
            && bridge_depth_resolve_shaders().cpu.is_some();
        checks.push(check("safe capability descriptors and shaders", caps_ok));

        let verts = [
            Vertex::vertex_new(
                Vec3::new(-0.5, -0.5, 0.0),
                Vec3::unit_z(),
                [1.0, 0.2, 0.2],
                0.0,
                0.0,
            ),
            Vertex::vertex_new(
                Vec3::new(0.5, -0.5, 0.0),
                Vec3::unit_z(),
                [0.2, 1.0, 0.2],
                1.0,
                0.0,
            ),
            Vertex::vertex_new(
                Vec3::new(0.0, 0.5, 0.0),
                Vec3::unit_z(),
                [0.2, 0.2, 1.0],
                0.5,
                1.0,
            ),
        ];
        let indices = [0u32, 1, 2];
        let meshes = [MeshSlice::mesh_slice_new(&verts, &indices).expect("valid mesh")];
        let materials = [MaterialBinding::material_new(ShadingMode::Gouraud)];
        let dcs = [DrawCall::draw_call_new(0, 0, Mat4::mat4_identity(), 0, 3).expect("valid dc")];
        let target = RenderTargetSpec {
            width: 32,
            height: 32,
            clear_color: [0, 0, 0, 255],
            clear_depth: 1.0,
            clip_space: render3d_gpu_bridge::ClipSpace::OpenGl,
            post_process: PostProcessMode::Identity,
        };
        let camera =
            CameraUniforms::camera_from_combined(Mat4::mat4_identity(), Vec3::zero()).expect("cam");
        let pass = RenderPass {
            target,
            camera,
            light: LightRig::light_rig_default(),
            meshes: &meshes,
            materials: &materials,
            draw_calls: &dcs,
        };

        let rep_sw = execute_render_pass_into(
            &pass,
            BackendPreference::AutoHardwareWithSoftwareFallback,
            &mut fb_sw,
            DepthBufferMut::F64(&mut zb64),
            &mut scratch_sw,
        );
        checks.push(check("execute_render_pass_into fallback", rep_sw.is_ok()));

        let rep_rt = execute_render_pass_with_runtime(
            &pass,
            BackendPreference::ForceGpuRuntimeCpu,
            Some(&cpu_rt),
            &mut fb_rt,
            DepthBufferMut::F32(&mut zb32),
            &mut scratch_rt,
        );
        let parity_ok = match (rep_sw, rep_rt) {
            (Ok(a), Ok(b)) => a.frame_digest == b.frame_digest && a.frame_digest != 0,
            _ => false,
        };
        checks.push(check("gpu_runtime cpu_only digest parity", parity_ok));

        let depth_res_ok =
            resolve_depth_to_rgba_with_runtime(&cpu_rt, 32, 32, &zb32, &mut fb_depth).is_ok();
        checks.push(check(
            "resolve_depth_to_rgba_with_runtime cpu_only",
            depth_res_ok,
        ));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/render3d-gpu-bridge",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
