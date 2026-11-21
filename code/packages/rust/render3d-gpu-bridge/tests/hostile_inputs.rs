//! # Hostile-Input CVE Proof & Multi-Draw Atomic Rollback Suite for `render3d-gpu-bridge` (GAME44)
//!
//! Proves that CWE-20, CWE-125, CWE-190, CWE-369, CWE-400, and CWE-787
//! cannot panic, corrupt memory, or partially mutate `framebuffer`, `depth`, or
//! `scratch` on error across multi-draw `RenderPass` execution.

#![forbid(unsafe_code)]

use math3d::{Mat3, Mat4, Vec3};
use pixel_container::PixelContainer;
use render3d_gpu_bridge::{
    execute_render_pass_into, execute_render_pass_with_runtime, resolve_depth_to_rgba_with_runtime,
    BackendPreference, BridgeError, CameraUniforms, ClipSpace, CullMode, DepthBufferMut, DepthFunc,
    DrawCall, FrontFace, GpuRuntime, LightRig, MaterialBinding, MeshSlice, PostProcessMode,
    RenderBackendKind, RenderBridgeScratch, RenderPass, RenderTargetSpec, ShadingMode, Texture,
    TextureWrap, Vertex, MAX_COORD, MAX_DRAW_CALLS, MAX_FRAMEBUFFER_WIDTH,
};

fn next_u64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    x.wrapping_mul(0x2545_f491_4f6c_dd1d)
}

fn pick_hostile_f64(state: &mut u64) -> f64 {
    match next_u64(state) % 16 {
        0 => f64::NAN,
        1 => f64::from_bits(0x7ff8_0000_0000_1234),
        2 => f64::from_bits(0x7ff0_0000_0000_0001),
        3 => f64::INFINITY,
        4 => f64::NEG_INFINITY,
        5 => -0.0,
        6 => 0.0,
        7 => f64::from_bits(1),
        8 => f64::MIN_POSITIVE * 0.5,
        9 => f64::MIN,
        10 => f64::MAX,
        11 => 1.0e305,
        12 => -1.0e300,
        13 => MAX_COORD + 1.0,
        _ => {
            let u = (next_u64(state) >> 11) as f64 / ((1u64 << 53) as f64);
            (u - 0.5) * 4.0
        }
    }
}

fn sentinel_fb(w: u32, h: u32) -> PixelContainer {
    let mut fb = PixelContainer::new(w, h);
    for (i, b) in fb.data.iter_mut().enumerate() {
        *b = ((i.wrapping_mul(41).wrapping_add(19)) & 0xff) as u8;
    }
    fb
}

fn sentinel_zb_f64(len: usize) -> Vec<f64> {
    (0..len).map(|i| 0.125 + (i as f64) * 1e-4).collect()
}

fn sentinel_zb_f32(len: usize) -> Vec<f32> {
    (0..len).map(|i| 0.125_f32 + (i as f32) * 1e-4).collect()
}

fn valid_triangle() -> ([Vertex; 3], [u32; 3]) {
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
    (verts, [0, 1, 2])
}

#[test]
fn multi_draw_atomicity_preserves_buffers_byte_for_byte_on_second_draw_error() {
    let (verts, indices) = valid_triangle();
    let meshes = [MeshSlice::mesh_slice_new(&verts, &indices).expect("valid mesh")];
    let tex_pc = PixelContainer::new(4, 4);
    let tex = Texture::texture_from_container(&tex_pc).expect("valid tex");
    let materials = [
        MaterialBinding::material_new(ShadingMode::Gouraud),
        MaterialBinding::material_new(ShadingMode::TexturedBilinear), // MissingTexture!
        MaterialBinding::material_with_texture(ShadingMode::TexturedNearest, tex),
    ];
    let target = RenderTargetSpec::target_spec_new(16, 16, [0, 0, 0, 255], 1.0).expect("target");
    let camera =
        CameraUniforms::camera_from_combined(Mat4::mat4_identity(), Vec3::zero()).expect("cam");

    let valid_dc0 = DrawCall::draw_call_new(0, 0, Mat4::mat4_identity(), 0, 3).expect("dc0");
    let mut nan_model = Mat4::mat4_identity();
    nan_model.cols[3][0] = f64::NAN;
    let mut inf_normal = Mat3::mat3_identity();
    inf_normal.cols[1][1] = f64::INFINITY;

    let bad_draw_calls = [
        // 1. OOB mesh_index on draw #1
        DrawCall {
            mesh_index: 1,
            ..valid_dc0
        },
        // 2. OOB material_index on draw #1
        DrawCall {
            material_index: 99,
            ..valid_dc0
        },
        // 3. index_offset + index_count overflow usize::MAX on draw #1
        DrawCall {
            index_offset: usize::MAX - 2,
            index_count: 3,
            ..valid_dc0
        },
        // 4. Non-multiple-of-3 index_count on draw #1
        DrawCall {
            index_count: 2,
            ..valid_dc0
        },
        // 5. MissingTexture material referenced on draw #1
        DrawCall {
            material_index: 1,
            ..valid_dc0
        },
        // 6. Non-finite model matrix on draw #1
        DrawCall {
            model: nan_model,
            ..valid_dc0
        },
        // 7. Non-finite normal matrix on draw #1
        DrawCall {
            normal_matrix: inf_normal,
            ..valid_dc0
        },
    ];

    let fb_orig = sentinel_fb(16, 16);
    let zb64_orig = sentinel_zb_f64(16 * 16);
    let zb32_orig = sentinel_zb_f32(16 * 16);
    let cpu_rt = GpuRuntime::cpu_only();

    for (case_idx, &bad_dc1) in bad_draw_calls.iter().enumerate() {
        let draw_calls = [valid_dc0, bad_dc1];
        let pass = RenderPass {
            target,
            camera,
            light: LightRig::light_rig_default(),
            meshes: &meshes,
            materials: &materials,
            draw_calls: &draw_calls,
        };

        let mut fb = fb_orig.clone();
        let mut zb64 = zb64_orig.clone();
        let mut scratch =
            RenderBridgeScratch::bridge_scratch_with_capacity(3, 16, 16).expect("scratch");
        let cap_v = scratch.bridge_scratch_vertex_capacity();
        let cap_p = scratch.bridge_scratch_pixel_byte_capacity();

        let res = execute_render_pass_into(
            &pass,
            BackendPreference::ForceSoftware,
            &mut fb,
            DepthBufferMut::F64(&mut zb64),
            &mut scratch,
        );
        assert!(res.is_err(), "case {case_idx} must fail atomically");
        assert_eq!(
            fb.data, fb_orig.data,
            "case {case_idx}: framebuffer must remain byte-for-byte untouched (not cleared)"
        );
        assert_eq!(
            zb64, zb64_orig,
            "case {case_idx}: f64 depth buffer must remain byte-for-byte untouched"
        );
        assert_eq!(scratch.bridge_scratch_vertex_capacity(), cap_v);
        assert_eq!(scratch.bridge_scratch_pixel_byte_capacity(), cap_p);

        // Also test execute_render_pass_with_runtime + f32 Z-buffer
        let mut fb_rt = fb_orig.clone();
        let mut zb32 = zb32_orig.clone();
        let mut scratch_rt =
            RenderBridgeScratch::bridge_scratch_with_capacity(3, 16, 16).expect("scratch rt");
        let res_rt = execute_render_pass_with_runtime(
            &pass,
            BackendPreference::ForceGpuRuntimeCpu,
            Some(&cpu_rt),
            &mut fb_rt,
            DepthBufferMut::F32(&mut zb32),
            &mut scratch_rt,
        );
        assert!(res_rt.is_err(), "rt case {case_idx} must fail atomically");
        assert_eq!(fb_rt.data, fb_orig.data);
        assert_eq!(zb32, zb32_orig);
    }

    // RequireHardware rejection must also preserve buffers untouched
    let good_draw_calls = [valid_dc0];
    let good_pass = RenderPass {
        target,
        camera,
        light: LightRig::light_rig_default(),
        meshes: &meshes,
        materials: &materials[..1],
        draw_calls: &good_draw_calls,
    };
    for hw_kind in [
        RenderBackendKind::Software,
        RenderBackendKind::GpuRuntimeCpu,
        RenderBackendKind::GpuRuntimeMetal,
        RenderBackendKind::GpuRuntimeCuda,
        RenderBackendKind::Wgpu,
        RenderBackendKind::Metal,
        RenderBackendKind::Vulkan,
    ] {
        let mut fb = fb_orig.clone();
        let mut zb64 = zb64_orig.clone();
        let mut scratch =
            RenderBridgeScratch::bridge_scratch_with_capacity(3, 16, 16).expect("scratch");
        let res = execute_render_pass_with_runtime(
            &good_pass,
            BackendPreference::RequireHardware(hw_kind),
            Some(&cpu_rt),
            &mut fb,
            DepthBufferMut::F64(&mut zb64),
            &mut scratch,
        );
        assert_eq!(res, Err(BridgeError::RequestedBackendUnavailable(hw_kind)));
        assert_eq!(fb.data, fb_orig.data);
        assert_eq!(zb64, zb64_orig);
    }
}

#[test]
fn hostile_numeric_index_budget_and_runtime_bounds() {
    let hostile_floats = [
        f64::NAN,
        f64::from_bits(0x7ff8_0000_0000_9999),
        f64::from_bits(0x7ff0_0000_0000_0001),
        f64::INFINITY,
        f64::NEG_INFINITY,
        -0.0,
        f64::from_bits(1),
        f64::MIN_POSITIVE * 0.5,
        f64::MIN,
        f64::MAX,
    ];

    for &val in &hostile_floats {
        let spec_res = RenderTargetSpec::target_spec_new(8, 8, [0, 0, 0, 255], val);
        if !val.is_finite() {
            assert_eq!(spec_res, Err(BridgeError::NonFiniteInput));
        } else {
            assert!(spec_res.is_ok());
        }

        let mut view = Mat4::mat4_identity();
        view.cols[0][0] = val;
        let cam_res = CameraUniforms::camera_from_view_proj(
            view,
            Mat4::mat4_identity(),
            Vec3::new(0.0, 0.0, val),
        );
        if !val.is_finite() {
            assert_eq!(cam_res, Err(BridgeError::NonFiniteInput));
        }
    }

    // Singular model matrix (zero scale) -> Err(SingularTransform)
    let singular_model = Mat4::mat4_from_scale(Vec3::new(0.0, 1.0, 1.0)).expect("scale matrix");
    assert_eq!(
        DrawCall::draw_call_new(0, 0, singular_model, 0, 3),
        Err(BridgeError::SingularTransform)
    );

    // Budget bombs: MAX_DRAW_CALLS + 1 and cumulative MAX_TOTAL_TRIANGLES + 1
    let (verts, indices) = valid_triangle();
    let mut big_indices = Vec::with_capacity(3 * 600_000);
    for _ in 0..600_000 {
        big_indices.extend_from_slice(&[0, 1, 2]);
    }
    let meshes = [MeshSlice {
        vertices: &verts,
        indices: &big_indices,
    }];
    let materials = [MaterialBinding::material_new(ShadingMode::Flat)];
    let target = RenderTargetSpec::target_spec_new(8, 8, [0, 0, 0, 255], 1.0).expect("target");
    let camera =
        CameraUniforms::camera_from_combined(Mat4::mat4_identity(), Vec3::zero()).expect("cam");

    // 2 draw calls * 600_000 tris = 1_200_000 > MAX_TOTAL_TRIANGLES (1_000_000)
    let dc_600k = DrawCall::draw_call_new(0, 0, Mat4::mat4_identity(), 0, 600_000 * 3).expect("dc");
    let over_tri_calls = [dc_600k, dc_600k];
    let pass_tri_bomb = RenderPass {
        target,
        camera,
        light: LightRig::light_rig_default(),
        meshes: &meshes,
        materials: &materials,
        draw_calls: &over_tri_calls,
    };
    let fb_orig = sentinel_fb(8, 8);
    let zb_orig = sentinel_zb_f64(64);
    let mut fb = fb_orig.clone();
    let mut zb = zb_orig.clone();
    let mut scratch = RenderBridgeScratch::bridge_scratch_new();
    assert_eq!(
        execute_render_pass_into(
            &pass_tri_bomb,
            BackendPreference::ForceSoftware,
            &mut fb,
            DepthBufferMut::F64(&mut zb),
            &mut scratch,
        ),
        Err(BridgeError::BudgetExceeded)
    );
    assert_eq!(fb.data, fb_orig.data);
    assert_eq!(zb, zb_orig);

    // MAX_DRAW_CALLS + 1 draw calls
    let one_tri_dc = DrawCall::draw_call_new(0, 0, Mat4::mat4_identity(), 0, 3).expect("dc");
    let many_dcs = vec![one_tri_dc; MAX_DRAW_CALLS + 1];
    let pass_dc_bomb = RenderPass {
        target,
        camera,
        light: LightRig::light_rig_default(),
        meshes: &[MeshSlice::mesh_slice_new(&verts, &indices).expect("mesh")],
        materials: &materials,
        draw_calls: &many_dcs,
    };
    assert_eq!(
        execute_render_pass_into(
            &pass_dc_bomb,
            BackendPreference::ForceSoftware,
            &mut fb,
            DepthBufferMut::F64(&mut zb),
            &mut scratch,
        ),
        Err(BridgeError::BudgetExceeded)
    );
    assert_eq!(fb.data, fb_orig.data);

    // Oversized / zero dimension checks
    assert_eq!(
        RenderTargetSpec::target_spec_new(0, 8, [0, 0, 0, 255], 1.0),
        Err(BridgeError::InvalidDimensions)
    );
    assert_eq!(
        RenderTargetSpec::target_spec_new(MAX_FRAMEBUFFER_WIDTH + 1, 8, [0, 0, 0, 255], 1.0),
        Err(BridgeError::InvalidDimensions)
    );
    assert_eq!(
        RenderBridgeScratch::bridge_scratch_with_capacity(3, 0, 8).err(),
        Some(BridgeError::InvalidDimensions)
    );

    // OOB vertex index inside MeshSlice
    let bad_mesh = MeshSlice {
        vertices: &verts,
        indices: &[0, 1, u32::MAX],
    };
    assert_eq!(
        bad_mesh.mesh_slice_validate(),
        Err(BridgeError::VertexIndexOutOfBounds)
    );

    // resolve_depth_to_rgba_with_runtime bounds & NaN depth handling
    let cpu_rt = GpuRuntime::cpu_only();
    let mut bad_pc = PixelContainer::new(4, 4);
    assert_eq!(
        resolve_depth_to_rgba_with_runtime(&cpu_rt, 0, 4, &[], &mut bad_pc),
        Err(BridgeError::InvalidDimensions)
    );
    assert_eq!(
        resolve_depth_to_rgba_with_runtime(&cpu_rt, 4, 4, &[0.5_f32; 8], &mut bad_pc),
        Err(BridgeError::InvalidDimensions)
    );
    let hostile_depth_f32 = [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        -0.0,
        0.0,
        0.5,
        1.0,
        f32::MAX,
        f32::MIN,
        f32::from_bits(1),
        0.25,
        0.75,
        -1.0,
        2.0,
        0.1,
        0.9,
    ];
    assert_eq!(
        resolve_depth_to_rgba_with_runtime(&cpu_rt, 4, 4, &hostile_depth_f32, &mut bad_pc),
        Ok(())
    );
}

#[test]
fn fuzz_2000_hostile_render_passes_zero_panics_and_atomic_on_error() {
    let mut rng: u64 = 0x51a7_c0de_4444_9876;
    let mut scratch =
        RenderBridgeScratch::bridge_scratch_with_capacity(16, 16, 16).expect("scratch");
    let cpu_rt = GpuRuntime::cpu_only();

    for case in 0..2000usize {
        let w = match next_u64(&mut rng) % 5 {
            0 => 0,
            1 => MAX_FRAMEBUFFER_WIDTH + 1,
            _ => 8,
        };
        let h = 8u32;
        let clear_depth = pick_hostile_f64(&mut rng);
        let clip_space = if (next_u64(&mut rng) & 1) == 0 {
            ClipSpace::OpenGl
        } else {
            ClipSpace::ZeroToOne
        };
        let post_process = match next_u64(&mut rng) % 3 {
            0 => PostProcessMode::Identity,
            1 => PostProcessMode::ExposureGainBias {
                gain_q8: (next_u64(&mut rng) & 0x3ff) as u16,
                bias: ((next_u64(&mut rng) & 0xff) as i16) - 128,
            },
            _ => PostProcessMode::ChannelTint {
                tint: [255, 128, 64, 255],
            },
        };

        let target = RenderTargetSpec {
            width: w,
            height: h,
            clear_color: [10, 20, 30, 255],
            clear_depth,
            clip_space,
            post_process,
        };

        let mut vp = Mat4::mat4_identity();
        if next_u64(&mut rng).is_multiple_of(4) {
            vp.cols[(next_u64(&mut rng) % 4) as usize][(next_u64(&mut rng) % 4) as usize] =
                pick_hostile_f64(&mut rng);
        }
        let camera = CameraUniforms {
            view: Mat4::mat4_identity(),
            projection: vp,
            view_projection: vp,
            camera_position: Vec3::new(
                pick_hostile_f64(&mut rng),
                pick_hostile_f64(&mut rng),
                pick_hostile_f64(&mut rng),
            ),
        };

        let mut verts = [
            Vertex::vertex_new(
                Vec3::new(
                    pick_hostile_f64(&mut rng),
                    pick_hostile_f64(&mut rng),
                    pick_hostile_f64(&mut rng),
                ),
                Vec3::new(
                    pick_hostile_f64(&mut rng),
                    pick_hostile_f64(&mut rng),
                    pick_hostile_f64(&mut rng),
                ),
                [0.5, 0.5, 0.5],
                pick_hostile_f64(&mut rng),
                pick_hostile_f64(&mut rng),
            ),
            Vertex::vertex_new(
                Vec3::new(0.5, -0.5, 0.0),
                Vec3::unit_z(),
                [1.0, 0.0, 0.0],
                1.0,
                0.0,
            ),
            Vertex::vertex_new(
                Vec3::new(0.0, 0.5, 0.0),
                Vec3::unit_z(),
                [0.0, 1.0, 0.0],
                0.5,
                1.0,
            ),
        ];
        if next_u64(&mut rng).is_multiple_of(3) {
            verts[1].color[0] = pick_hostile_f64(&mut rng);
        }
        let i0 = (next_u64(&mut rng) % 5) as u32;
        let indices = [i0, 1, 2, 0];
        let slice_len = match next_u64(&mut rng) % 4 {
            0 => 3,
            1 => 4, // invalid modulo 3
            _ => 3,
        };
        let meshes = [MeshSlice {
            vertices: &verts,
            indices: &indices[..slice_len],
        }];

        let shading = match next_u64(&mut rng) % 5 {
            0 => ShadingMode::Flat,
            1 => ShadingMode::Gouraud,
            2 => ShadingMode::Phong,
            3 => ShadingMode::TexturedNearest,
            _ => ShadingMode::TexturedBilinear,
        };
        let materials = [MaterialBinding {
            texture: None, // triggers MissingTexture when textured
            shading,
            cull: CullMode::Back,
            front_face: FrontFace::CounterClockwise,
            texture_wrap: TextureWrap::Repeat,
            depth_test: true,
            depth_write: true,
            depth_func: DepthFunc::Less,
        }];

        let dc0 = DrawCall {
            mesh_index: (next_u64(&mut rng) % 3) as usize,
            material_index: (next_u64(&mut rng) % 2) as usize,
            model: Mat4::mat4_identity(),
            normal_matrix: Mat3::mat3_identity(),
            index_offset: if next_u64(&mut rng).is_multiple_of(6) {
                usize::MAX
            } else {
                0
            },
            index_count: if next_u64(&mut rng).is_multiple_of(5) {
                2
            } else {
                3
            },
        };
        let draw_calls = [dc0];
        let pass = RenderPass {
            target,
            camera,
            light: LightRig::light_rig_default(),
            meshes: &meshes,
            materials: &materials,
            draw_calls: &draw_calls,
        };

        let fb_orig = sentinel_fb(8, 8);
        let zb_orig = sentinel_zb_f64(64);
        let mut fb = fb_orig.clone();
        let mut zb = zb_orig.clone();

        let pref = match case % 4 {
            0 => BackendPreference::ForceSoftware,
            1 => BackendPreference::AutoHardwareWithSoftwareFallback,
            2 => BackendPreference::ForceGpuRuntimeCpu,
            _ => BackendPreference::RequireHardware(RenderBackendKind::Vulkan),
        };
        let res = if (case & 1) == 0 {
            execute_render_pass_into(
                &pass,
                pref,
                &mut fb,
                DepthBufferMut::F64(&mut zb),
                &mut scratch,
            )
        } else {
            execute_render_pass_with_runtime(
                &pass,
                pref,
                Some(&cpu_rt),
                &mut fb,
                DepthBufferMut::F64(&mut zb),
                &mut scratch,
            )
        };
        if res.is_err() {
            assert_eq!(fb.data, fb_orig.data);
            assert_eq!(zb, zb_orig);
        }
    }
}
