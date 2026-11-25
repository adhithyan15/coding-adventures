//! # Conformance & Cross-Backend Parity Suite for `render3d-gpu-bridge` (GAME44)

#![forbid(unsafe_code)]

use math3d::{Mat3, Mat4, Vec3};
use pixel_container::PixelContainer;
use raster3d_soft::{
    clear_depth_f64, clear_framebuffer, depth_digest_f64_fnv1a, draw_indexed_mesh_into,
    frame_digest_fnv1a, RasterConfig, RasterScratch,
};
use render3d_gpu_bridge::{
    bridge_depth_digest_fnv1a, bridge_depth_resolve_shaders, bridge_frame_digest_fnv1a,
    bridge_postprocess_shaders, execute_render_pass_into, execute_render_pass_with_runtime,
    resolve_depth_to_rgba_with_runtime, validate_draw_call_count, validate_framebuffer_dim,
    validate_handle_index, validate_index_range, validate_total_triangles, BackendCapabilities,
    BackendPreference, BridgeError, CameraUniforms, ClipSpace, CullMode, DepthBufferMut, DepthFunc,
    DrawCall, FallbackReason, FrontFace, GpuRuntime, LightRig, MaterialBinding, MeshSlice,
    PostProcessMode, RenderBackendKind, RenderBridgeScratch, RenderPass, RenderTargetSpec,
    ShadingMode, Texture, TextureWrap, Vertex, MAX_DRAW_CALLS, MAX_FRAMEBUFFER_WIDTH,
    MAX_TOTAL_TRIANGLES,
};

fn build_cube_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let corners = [
        Vec3::new(-0.5, -0.5, -0.5),
        Vec3::new(0.5, -0.5, -0.5),
        Vec3::new(0.5, 0.5, -0.5),
        Vec3::new(-0.5, 0.5, -0.5),
        Vec3::new(-0.5, -0.5, 0.5),
        Vec3::new(0.5, -0.5, 0.5),
        Vec3::new(0.5, 0.5, 0.5),
        Vec3::new(-0.5, 0.5, 0.5),
    ];
    let faces: [([usize; 4], Vec3, [f64; 3]); 6] = [
        ([4, 5, 6, 7], Vec3::unit_z(), [0.9, 0.2, 0.2]),
        ([1, 0, 3, 2], Vec3::new(0.0, 0.0, -1.0), [0.2, 0.9, 0.2]),
        ([5, 1, 2, 6], Vec3::unit_x(), [0.2, 0.4, 0.9]),
        ([0, 4, 7, 3], Vec3::new(-1.0, 0.0, 0.0), [0.9, 0.8, 0.2]),
        ([7, 6, 2, 3], Vec3::unit_y(), [0.8, 0.2, 0.9]),
        ([0, 1, 5, 4], Vec3::new(0.0, -1.0, 0.0), [0.2, 0.8, 0.9]),
    ];
    let mut verts = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for (idx, normal, color) in faces {
        let base = verts.len() as u32;
        verts.push(Vertex::vertex_new(corners[idx[0]], normal, color, 0.0, 0.0));
        verts.push(Vertex::vertex_new(corners[idx[1]], normal, color, 1.0, 0.0));
        verts.push(Vertex::vertex_new(corners[idx[2]], normal, color, 1.0, 1.0));
        verts.push(Vertex::vertex_new(corners[idx[3]], normal, color, 0.0, 1.0));
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (verts, indices)
}

fn make_checker_texture() -> PixelContainer {
    let mut pc = PixelContainer::new(8, 8);
    for y in 0..8u32 {
        for x in 0..8u32 {
            let dark = ((x + y) & 1) == 0;
            let c = if dark { 40 } else { 230 };
            pc.set_pixel(x, y, c, 180, 255 - c, 255);
        }
    }
    pc
}

#[test]
fn warrant_validators_and_capability_descriptors() {
    assert_eq!(validate_framebuffer_dim(1), Ok(true));
    assert_eq!(validate_framebuffer_dim(MAX_FRAMEBUFFER_WIDTH), Ok(true));
    assert_eq!(
        validate_framebuffer_dim(0),
        Err(BridgeError::InvalidDimensions)
    );
    assert_eq!(
        validate_framebuffer_dim(MAX_FRAMEBUFFER_WIDTH + 1),
        Err(BridgeError::InvalidDimensions)
    );

    assert_eq!(validate_draw_call_count(MAX_DRAW_CALLS), Ok(true));
    assert_eq!(
        validate_draw_call_count(MAX_DRAW_CALLS + 1),
        Err(BridgeError::BudgetExceeded)
    );

    assert_eq!(validate_total_triangles(MAX_TOTAL_TRIANGLES), Ok(true));
    assert_eq!(
        validate_total_triangles(MAX_TOTAL_TRIANGLES + 1),
        Err(BridgeError::BudgetExceeded)
    );

    assert_eq!(validate_handle_index(0, 1), Ok(true));
    assert_eq!(
        validate_handle_index(1, 1),
        Err(BridgeError::InvalidHandleIndex)
    );

    assert_eq!(validate_index_range(3, 6, 9), Ok(true));
    assert_eq!(
        validate_index_range(0, 4, 9),
        Err(BridgeError::InvalidIndexRange)
    );
    assert_eq!(
        validate_index_range(6, 6, 9),
        Err(BridgeError::InvalidIndexRange)
    );

    let cpu_rt = GpuRuntime::cpu_only();
    let table_none = BackendCapabilities::backend_caps_detect_safe(None);
    let table_cpu = BackendCapabilities::backend_caps_detect_safe(Some(&cpu_rt));
    assert!(
        table_none
            .cap_set_get(RenderBackendKind::Software)
            .available
    );
    assert!(
        table_none
            .cap_set_get(RenderBackendKind::Software)
            .deterministic_software_parity
    );
    assert!(
        !table_none
            .cap_set_get(RenderBackendKind::Software)
            .hardware_accelerated
    );
    assert!(
        table_cpu
            .cap_set_get(RenderBackendKind::GpuRuntimeCpu)
            .available
    );
    assert!(
        !table_cpu
            .cap_set_get(RenderBackendKind::GpuRuntimeCuda)
            .available
    );
    assert!(!table_cpu.cap_set_get(RenderBackendKind::Wgpu).available);
    assert_eq!(
        RenderBackendKind::Software.backend_name(),
        "software-raster3d-soft"
    );
    assert!(!RenderBackendKind::Software.backend_is_hardware());
    assert!(!RenderBackendKind::GpuRuntimeCpu.backend_is_hardware());
    assert!(RenderBackendKind::GpuRuntimeMetal.backend_is_hardware());
    assert!(RenderBackendKind::GpuRuntimeCuda.backend_is_hardware());
    assert!(RenderBackendKind::Wgpu.backend_is_hardware());
    assert!(RenderBackendKind::Metal.backend_is_hardware());
    assert!(RenderBackendKind::Vulkan.backend_is_hardware());
}

#[test]
fn bit_exact_parity_across_software_auto_fallback_and_gpu_runtime_cpu() {
    let (verts, indices) = build_cube_mesh();
    let tex_pc = make_checker_texture();
    let tex = Texture::texture_from_container(&tex_pc).expect("valid checker texture");

    let view =
        Mat4::look_at(Vec3::new(1.6, 1.4, 2.8), Vec3::zero(), Vec3::unit_y()).expect("valid view");
    let proj = Mat4::perspective(std::f64::consts::FRAC_PI_3, 1.0, 0.25, 32.0).expect("valid proj");
    let camera = CameraUniforms::camera_from_view_proj(view, proj, Vec3::new(1.6, 1.4, 2.8))
        .expect("valid camera");

    let target = RenderTargetSpec {
        width: 64,
        height: 64,
        clear_color: [12, 18, 28, 255],
        clear_depth: 1.0,
        clip_space: ClipSpace::OpenGl,
        post_process: PostProcessMode::Identity,
    };

    let meshes = [MeshSlice::mesh_slice_new(&verts, &indices).expect("valid mesh")];
    let mut mat_textured =
        MaterialBinding::material_with_texture(ShadingMode::TexturedBilinear, tex);
    mat_textured.texture_wrap = TextureWrap::Repeat;
    let mat_phong = MaterialBinding {
        texture: None,
        shading: ShadingMode::Phong,
        cull: CullMode::Back,
        front_face: FrontFace::CounterClockwise,
        texture_wrap: TextureWrap::Clamp,
        depth_test: true,
        depth_write: true,
        depth_func: DepthFunc::Less,
    };
    let materials = [mat_textured, mat_phong];

    let dc0 = DrawCall::draw_call_new(
        0,
        0,
        Mat4::mat4_from_translation(Vec3::new(-0.55, 0.0, 0.0)).expect("dc0 model"),
        0,
        indices.len(),
    )
    .expect("valid dc0");
    let dc1 = DrawCall::draw_call_new(
        0,
        1,
        Mat4::mat4_from_translation(Vec3::new(0.65, 0.1, -0.4)).expect("dc1 model"),
        0,
        indices.len(),
    )
    .expect("valid dc1");
    let draw_calls = [dc0, dc1];

    let pass = RenderPass {
        target,
        camera,
        light: LightRig::light_rig_default(),
        meshes: &meshes,
        materials: &materials,
        draw_calls: &draw_calls,
    };

    // 1. Direct raster3d-soft reference
    let mut fb_ref = PixelContainer::new(64, 64);
    let mut zb_ref = vec![1.0_f64; 64 * 64];
    clear_framebuffer(&mut fb_ref, target.clear_color).expect("clear fb");
    clear_depth_f64(&mut zb_ref, target.clear_depth).expect("clear depth");
    let mut ref_scratch = RasterScratch::scratch_with_capacity(verts.len()).expect("ref scratch");
    for dc in &draw_calls {
        let mvp = camera.view_projection.mul(&dc.model).expect("mvp");
        let mat = &materials[dc.material_index];
        let cfg = RasterConfig {
            mvp,
            normal_matrix: dc.normal_matrix,
            shading: mat.shading,
            cull: mat.cull,
            front_face: mat.front_face,
            texture_wrap: mat.texture_wrap,
            clip_space: target.clip_space,
            depth_test: mat.depth_test,
            depth_write: mat.depth_write,
            depth_func: mat.depth_func,
            light: pass.light,
        };
        draw_indexed_mesh_into(
            &mut fb_ref,
            DepthBufferMut::F64(&mut zb_ref),
            &cfg,
            mat.texture.as_ref(),
            &verts,
            &indices,
            &mut ref_scratch,
        )
        .expect("ref draw");
    }
    let ref_frame_digest = frame_digest_fnv1a(&fb_ref);
    let ref_depth_digest = depth_digest_f64_fnv1a(&zb_ref);

    // 2. ForceSoftware (zero-alloc)
    let mut fb_sw = PixelContainer::new(64, 64);
    let mut zb_sw = vec![0.0_f64; 64 * 64];
    let mut scratch_sw =
        RenderBridgeScratch::bridge_scratch_with_capacity(verts.len(), 64, 64).expect("scratch");
    let rep_sw = execute_render_pass_into(
        &pass,
        BackendPreference::ForceSoftware,
        &mut fb_sw,
        DepthBufferMut::F64(&mut zb_sw),
        &mut scratch_sw,
    )
    .expect("ForceSoftware should succeed");
    assert_eq!(rep_sw.active_backend, RenderBackendKind::Software);
    assert!(!rep_sw.fell_back);
    assert_eq!(rep_sw.fallback_reason, FallbackReason::NoFallback);
    assert_eq!(rep_sw.frame_digest, ref_frame_digest);
    assert_eq!(rep_sw.depth_digest, ref_depth_digest);
    assert!(rep_sw.stats.pixels_written > 0);

    // 3. AutoHardwareWithSoftwareFallback (no HW runtime -> falls back to Software)
    let mut fb_auto = PixelContainer::new(64, 64);
    let mut zb_auto = vec![0.0_f64; 64 * 64];
    let mut scratch_auto =
        RenderBridgeScratch::bridge_scratch_with_capacity(verts.len(), 64, 64).expect("scratch");
    let rep_auto = execute_render_pass_into(
        &pass,
        BackendPreference::AutoHardwareWithSoftwareFallback,
        &mut fb_auto,
        DepthBufferMut::F64(&mut zb_auto),
        &mut scratch_auto,
    )
    .expect("AutoHardwareWithSoftwareFallback should succeed");
    assert_eq!(rep_auto.active_backend, RenderBackendKind::Software);
    assert!(rep_auto.fell_back);
    assert_eq!(
        rep_auto.fallback_reason,
        FallbackReason::NoHardwareRuntimeProvided
    );
    assert_eq!(rep_auto.frame_digest, ref_frame_digest);
    assert_eq!(rep_auto.depth_digest, ref_depth_digest);

    // 4. ForceGpuRuntimeCpu via execute_render_pass_into
    let mut fb_gr_into = PixelContainer::new(64, 64);
    let mut zb_gr_into = vec![0.0_f64; 64 * 64];
    let mut scratch_gr_into =
        RenderBridgeScratch::bridge_scratch_with_capacity(verts.len(), 64, 64).expect("scratch");
    let rep_gr_into = execute_render_pass_into(
        &pass,
        BackendPreference::ForceGpuRuntimeCpu,
        &mut fb_gr_into,
        DepthBufferMut::F64(&mut zb_gr_into),
        &mut scratch_gr_into,
    )
    .expect("ForceGpuRuntimeCpu into should succeed");
    assert_eq!(rep_gr_into.active_backend, RenderBackendKind::GpuRuntimeCpu);
    assert!(!rep_gr_into.fell_back);
    assert_eq!(rep_gr_into.frame_digest, ref_frame_digest);
    assert_eq!(rep_gr_into.depth_digest, ref_depth_digest);

    // 5. ForceGpuRuntimeCpu via gpu_runtime::Runtime::cpu_only()
    let cpu_rt = GpuRuntime::cpu_only();
    let mut fb_gr_rt = PixelContainer::new(64, 64);
    let mut zb_gr_rt = vec![0.0_f64; 64 * 64];
    let mut scratch_gr_rt =
        RenderBridgeScratch::bridge_scratch_with_capacity(verts.len(), 64, 64).expect("scratch");
    let rep_gr_rt = execute_render_pass_with_runtime(
        &pass,
        BackendPreference::ForceGpuRuntimeCpu,
        Some(&cpu_rt),
        &mut fb_gr_rt,
        DepthBufferMut::F64(&mut zb_gr_rt),
        &mut scratch_gr_rt,
    )
    .expect("ForceGpuRuntimeCpu with runtime should succeed");
    assert_eq!(rep_gr_rt.active_backend, RenderBackendKind::GpuRuntimeCpu);
    assert!(!rep_gr_rt.fell_back);
    assert_eq!(rep_gr_rt.frame_digest, ref_frame_digest);
    assert_eq!(rep_gr_rt.depth_digest, ref_depth_digest);

    // 6. AutoHardwareWithSoftwareFallback with Runtime::cpu_only()
    let mut fb_auto_rt = PixelContainer::new(64, 64);
    let mut zb_auto_rt = vec![0.0_f64; 64 * 64];
    let mut scratch_auto_rt =
        RenderBridgeScratch::bridge_scratch_with_capacity(verts.len(), 64, 64).expect("scratch");
    let rep_auto_rt = execute_render_pass_with_runtime(
        &pass,
        BackendPreference::AutoHardwareWithSoftwareFallback,
        Some(&cpu_rt),
        &mut fb_auto_rt,
        DepthBufferMut::F64(&mut zb_auto_rt),
        &mut scratch_auto_rt,
    )
    .expect("Auto with cpu_only runtime should fall back cleanly");
    assert_eq!(rep_auto_rt.active_backend, RenderBackendKind::Software);
    assert!(rep_auto_rt.fell_back);
    assert_eq!(
        rep_auto_rt.fallback_reason,
        FallbackReason::HardwareDriverUnavailable
    );
    assert_eq!(rep_auto_rt.frame_digest, ref_frame_digest);
    assert_eq!(rep_auto_rt.depth_digest, ref_depth_digest);
}

#[test]
fn postprocess_shaders_and_depth_resolve_parity() {
    let post_bundle = bridge_postprocess_shaders();
    let depth_bundle = bridge_depth_resolve_shaders();
    assert!(post_bundle.cpu.is_some());
    assert!(post_bundle.metal.is_some());
    assert!(post_bundle.cuda.is_some());
    assert!(depth_bundle.cpu.is_some());
    assert!(depth_bundle.metal.is_some());
    assert!(depth_bundle.cuda.is_some());

    let (verts, indices) = build_cube_mesh();
    let view =
        Mat4::look_at(Vec3::new(0.0, 0.5, 2.5), Vec3::zero(), Vec3::unit_y()).expect("valid view");
    let proj =
        Mat4::perspective_zo(std::f64::consts::FRAC_PI_3, 1.0, 0.1, 20.0).expect("valid zo proj");
    let camera = CameraUniforms::camera_from_view_proj(view, proj, Vec3::new(0.0, 0.5, 2.5))
        .expect("valid camera");
    let meshes = [MeshSlice::mesh_slice_new(&verts, &indices).expect("valid mesh")];
    let materials = [MaterialBinding::material_new(ShadingMode::Gouraud)];
    let dc = DrawCall::draw_call_with_normal_matrix(
        0,
        0,
        Mat4::mat4_identity(),
        Mat3::mat3_identity(),
        0,
        indices.len(),
    )
    .expect("valid dc");
    let draw_calls = [dc];

    let cpu_rt = GpuRuntime::cpu_only();
    for mode in [
        PostProcessMode::ExposureGainBias {
            gain_q8: 320,
            bias: 8,
        },
        PostProcessMode::ChannelTint {
            tint: [220, 180, 140, 255],
        },
    ] {
        let target = RenderTargetSpec {
            width: 32,
            height: 32,
            clear_color: [20, 30, 40, 255],
            clear_depth: 1.0,
            clip_space: ClipSpace::ZeroToOne,
            post_process: mode,
        };
        let pass = RenderPass {
            target,
            camera,
            light: LightRig::light_rig_default(),
            meshes: &meshes,
            materials: &materials,
            draw_calls: &draw_calls,
        };

        let mut fb_a = PixelContainer::new(32, 32);
        let mut zb_a = vec![1.0_f32; 32 * 32];
        let mut scratch_a = RenderBridgeScratch::bridge_scratch_with_capacity(verts.len(), 32, 32)
            .expect("scratch a");
        let rep_a = execute_render_pass_into(
            &pass,
            BackendPreference::ForceSoftware,
            &mut fb_a,
            DepthBufferMut::F32(&mut zb_a),
            &mut scratch_a,
        )
        .expect("software postprocess");

        let mut fb_b = PixelContainer::new(32, 32);
        let mut zb_b = vec![1.0_f32; 32 * 32];
        let mut scratch_b = RenderBridgeScratch::bridge_scratch_with_capacity(verts.len(), 32, 32)
            .expect("scratch b");
        let rep_b = execute_render_pass_with_runtime(
            &pass,
            BackendPreference::ForceGpuRuntimeCpu,
            Some(&cpu_rt),
            &mut fb_b,
            DepthBufferMut::F32(&mut zb_b),
            &mut scratch_b,
        )
        .expect("gpu_runtime cpu postprocess");

        assert_eq!(rep_a.frame_digest, rep_b.frame_digest);
        assert_eq!(rep_a.depth_digest, rep_b.depth_digest);
        assert_eq!(bridge_frame_digest_fnv1a(&fb_a), rep_a.frame_digest);
        assert_eq!(
            bridge_depth_digest_fnv1a(&DepthBufferMut::F32(&mut zb_a)),
            rep_a.depth_digest
        );

        let mut depth_rgba = PixelContainer::new(32, 32);
        resolve_depth_to_rgba_with_runtime(&cpu_rt, 32, 32, &zb_b, &mut depth_rgba)
            .expect("depth resolve should succeed");
        assert_ne!(bridge_frame_digest_fnv1a(&depth_rgba), 0);
    }

    let uniform_bytes = camera
        .camera_to_uniform_bytes(&Mat4::mat4_identity())
        .expect("uniform bytes");
    assert_eq!(uniform_bytes.len(), 128);
}
