//! # Hostile-Input CVE Proof & Atomic Rollback Suite for `raster3d-soft` (GAME33)
//!
//! Proves that CWE-20, CWE-125, CWE-190, CWE-369, CWE-400, and CWE-787
//! cannot trigger panics, out-of-bounds memory access, integer overflows, or
//! partial framebuffer/Z-buffer corruption under hostile or malformed inputs.

#![forbid(unsafe_code)]

use math3d::{Mat3, Mat4, Vec3};
use pixel_container::PixelContainer;
use raster3d_soft::{
    clear_depth_f64, clear_framebuffer, depth_digest_f32_fnv1a, depth_digest_f64_fnv1a,
    draw_indexed_mesh_into, draw_triangle_into, ClipSpace, CullMode, DepthBufferMut, DepthFunc,
    DirectionalLight, FrontFace, LightRig, RasterConfig, RasterError, RasterScratch, ShadingMode,
    Texture, TextureWrap, Vertex, MAX_COORD, MAX_VERTICES,
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
        1 => f64::from_bits(0x7ff8_0000_0000_1234), // non-canonical NaN payload
        2 => f64::INFINITY,
        3 => f64::NEG_INFINITY,
        4 => -0.0,
        5 => 0.0,
        6 => f64::from_bits(1), // smallest positive subnormal
        7 => f64::MIN_POSITIVE * 0.5,
        8 => f64::MIN,
        9 => f64::MAX,
        10 => 1.0e305,
        11 => -1.0e300,
        12 => 1.0e9 + 1.0,
        13 => -1.0e-17,
        _ => {
            let u = (next_u64(state) >> 11) as f64 / ((1u64 << 53) as f64);
            (u - 0.5) * 4.0
        }
    }
}

fn sentinel_fb(w: u32, h: u32) -> PixelContainer {
    let mut fb = PixelContainer::new(w, h);
    for (i, b) in fb.data.iter_mut().enumerate() {
        *b = ((i * 37 + 13) & 0xff) as u8;
    }
    fb
}

fn sentinel_zb_f64(len: usize) -> Vec<f64> {
    (0..len).map(|i| 0.25 + (i as f64) * 1e-4).collect()
}

fn sentinel_zb_f32(len: usize) -> Vec<f32> {
    (0..len).map(|i| 0.25_f32 + (i as f32) * 1e-4).collect()
}

#[test]
fn hostile_numeric_and_flat_normal_overflow_safety() {
    let subnormal_min = f64::from_bits(1);
    let subnormal_half = f64::MIN_POSITIVE * 0.5;
    let hostile_scalars = [
        f64::NAN,
        f64::from_bits(0x7ff8_0000_0000_5678),
        f64::INFINITY,
        f64::NEG_INFINITY,
        -0.0,
        subnormal_min,
        subnormal_half,
        f64::MIN,
        f64::MAX,
        1.0e308,
        -1.0e308,
    ];

    let mut fb = PixelContainer::new(16, 16);
    let mut zb = vec![1.0_f64; 16 * 16];
    let base_cfg = RasterConfig::raster_config_new(Mat4::mat4_identity()).unwrap();

    for &val in &hostile_scalars {
        // Hostile MVP / normal matrix entries must never panic
        let mut mvp = Mat4::mat4_identity();
        mvp.cols[0][0] = val;
        let cfg_res = RasterConfig::raster_config_new(mvp);
        if !val.is_finite() {
            assert_eq!(cfg_res, Err(RasterError::NonFiniteInput));
        } else if let Ok(cfg) = cfg_res {
            let v0 = Vertex::vertex_new(
                Vec3::new(-0.5, -0.5, 0.0),
                Vec3::unit_z(),
                [1.0, 1.0, 1.0],
                0.0,
                0.0,
            );
            let v1 = Vertex::vertex_new(
                Vec3::new(0.5, -0.5, 0.0),
                Vec3::unit_z(),
                [1.0, 1.0, 1.0],
                1.0,
                0.0,
            );
            let v2 = Vertex::vertex_new(
                Vec3::new(0.0, 0.5, 0.0),
                Vec3::unit_z(),
                [1.0, 1.0, 1.0],
                0.5,
                1.0,
            );
            let _ = draw_triangle_into(
                &mut fb,
                DepthBufferMut::F64(&mut zb),
                &cfg,
                None,
                &v0,
                &v1,
                &v2,
            );
        }

        // Hostile vertex positions
        let hv = Vertex::vertex_new(
            Vec3::new(val, -0.0, subnormal_half),
            Vec3::new(subnormal_min, 0.0, 1.0),
            [0.5, 0.5, 0.5],
            val,
            -0.0,
        );
        let res = draw_triangle_into(
            &mut fb,
            DepthBufferMut::F64(&mut zb),
            &base_cfg,
            None,
            &hv,
            &hv,
            &hv,
        );
        if !val.is_finite() {
            assert_eq!(res, Err(RasterError::NonFiniteInput));
        } else if val.abs() > MAX_COORD {
            assert_eq!(res, Err(RasterError::ValueOutOfBounds));
        }
    }

    // Multi-triangle Flat shading with 1e305 normal_matrix fails atomically in pre-flight (no partial fb/zb write)
    let mut fb_atomic = sentinel_fb(16, 16);
    let orig_fb_atomic = fb_atomic.data.clone();
    let mut zb_atomic = sentinel_zb_f64(16 * 16);
    let orig_zb_atomic = zb_atomic.clone();
    let mut flat_cfg = RasterConfig::raster_config_new(Mat4::mat4_identity()).unwrap();
    flat_cfg.shading = ShadingMode::Flat;
    flat_cfg.cull = CullMode::None;
    flat_cfg.normal_matrix = Mat3::mat3_from_cols([
        [1.0e305, 0.0, 0.0],
        [0.0, 1.0e305, 0.0],
        [0.0, 0.0, 1.0e305],
    ])
    .unwrap();

    let verts = [
        Vertex::vertex_new(
            Vec3::new(-0.8, -0.8, 0.0),
            Vec3::unit_z(),
            [1.0, 0.2, 0.2],
            0.0,
            0.0,
        ),
        Vertex::vertex_new(
            Vec3::new(0.0, -0.8, 0.0),
            Vec3::unit_z(),
            [1.0, 0.2, 0.2],
            1.0,
            0.0,
        ),
        Vertex::vertex_new(
            Vec3::new(-0.4, 0.2, 0.0),
            Vec3::unit_z(),
            [1.0, 0.2, 0.2],
            0.5,
            1.0,
        ),
        Vertex::vertex_new(
            Vec3::new(0.0, -0.8, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            [0.2, 1.0, 0.2],
            0.0,
            0.0,
        ),
        Vertex::vertex_new(
            Vec3::new(0.8, -0.8, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            [0.2, 1.0, 0.2],
            1.0,
            0.0,
        ),
        Vertex::vertex_new(
            Vec3::new(0.4, 0.6, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            [0.2, 1.0, 0.2],
            0.5,
            1.0,
        ),
    ];
    let mut scratch = RasterScratch::scratch_with_capacity(6).unwrap();
    let res_305 = draw_indexed_mesh_into(
        &mut fb_atomic,
        DepthBufferMut::F64(&mut zb_atomic),
        &flat_cfg,
        None,
        &verts,
        &[0, 1, 2, 3, 4, 5],
        &mut scratch,
    );
    assert!(res_305.is_err());
    assert_eq!(fb_atomic.data, orig_fb_atomic);
    assert_eq!(zb_atomic, orig_zb_atomic);

    // Multi-triangle Flat shading with 1e150 normal_matrix (passes pre-flight) succeeds cleanly
    clear_framebuffer(&mut fb, [0, 0, 0, 255]).unwrap();
    clear_depth_f64(&mut zb, 1.0).unwrap();
    flat_cfg.normal_matrix = Mat3::mat3_from_cols([
        [1.0e150, 0.0, 0.0],
        [0.0, 1.0e150, 0.0],
        [0.0, 0.0, 1.0e150],
    ])
    .unwrap();
    let stats = draw_indexed_mesh_into(
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &flat_cfg,
        None,
        &verts,
        &[0, 1, 2, 3, 4, 5],
        &mut scratch,
    )
    .unwrap();
    assert_eq!(stats.triangles_submitted, 2);
    assert!(stats.pixels_written > 0);
}

#[test]
fn hostile_textures_slices_and_canonical_depth_digests() {
    let mut tex_pc = PixelContainer::new(4, 4);
    tex_pc.fill(100, 150, 200, 255);
    let tex = Texture::texture_from_container(&tex_pc).unwrap();

    // Extreme UV coordinates under both Repeat and Clamp
    for wrap in [TextureWrap::Repeat, TextureWrap::Clamp] {
        for &(u, v) in &[
            (1.0e300, -1.0e300),
            (-1.0e-17, 1.0 + 1.0e-15),
            (-0.0, 0.0),
            (f64::from_bits(1), f64::MIN_POSITIVE * 0.5),
            (f64::MAX, f64::MIN),
        ] {
            assert!(tex.texture_sample_nearest(u, v, wrap).is_ok());
            assert!(tex.texture_sample_bilinear(u, v, wrap).is_ok());
        }
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                tex.texture_sample_nearest(bad, 0.5, wrap),
                Err(RasterError::NonFiniteInput)
            );
            assert_eq!(
                tex.texture_sample_bilinear(0.5, bad, wrap),
                Err(RasterError::NonFiniteInput)
            );
        }
    }

    // Truncated PixelContainer byte buffer
    let mut truncated_fb = PixelContainer::new(8, 8);
    truncated_fb.data.pop();
    assert_eq!(
        Texture::texture_from_container(&truncated_fb),
        Err(RasterError::InvalidDimensions)
    );
    assert_eq!(
        clear_framebuffer(&mut truncated_fb, [0, 0, 0, 255]),
        Err(RasterError::InvalidDimensions)
    );

    // Canonical NaN payload & -0.0 hashing in depth digests
    let nan_a = f64::from_bits(0x7ff8_0000_0000_0001);
    let nan_b = f64::from_bits(0x7ff8_1234_5678_9abc);
    assert_eq!(
        depth_digest_f64_fnv1a(&[nan_a, -0.0, 1.0]),
        depth_digest_f64_fnv1a(&[nan_b, 0.0, 1.0])
    );

    let nan32_a = f32::from_bits(0x7fc0_0001);
    let nan32_b = f32::from_bits(0x7fc1_abcd);
    assert_eq!(
        depth_digest_f32_fnv1a(&[nan32_a, -0.0_f32, 1.0_f32]),
        depth_digest_f32_fnv1a(&[nan32_b, 0.0_f32, 1.0_f32])
    );
}

#[test]
fn atomic_rollback_on_all_error_paths() {
    let valid = [
        Vertex::vertex_new(
            Vec3::new(-0.5, -0.5, 0.0),
            Vec3::unit_z(),
            [1.0, 0.0, 0.0],
            0.0,
            0.0,
        ),
        Vertex::vertex_new(
            Vec3::new(0.5, -0.5, 0.0),
            Vec3::unit_z(),
            [0.0, 1.0, 0.0],
            1.0,
            0.0,
        ),
        Vertex::vertex_new(
            Vec3::new(0.0, 0.5, 0.0),
            Vec3::unit_z(),
            [0.0, 0.0, 1.0],
            0.5,
            1.0,
        ),
        Vertex::vertex_new(
            Vec3::new(-0.3, -0.3, -0.2),
            Vec3::unit_z(),
            [1.0, 1.0, 0.0],
            0.0,
            0.0,
        ),
        Vertex::vertex_new(
            Vec3::new(f64::NAN, 0.0, 0.0),
            Vec3::unit_z(),
            [1.0, 1.0, 1.0],
            0.0,
            0.0,
        ),
        Vertex::vertex_new(
            Vec3::new(0.3, 0.3, -0.2),
            Vec3::unit_z(),
            [1.0, 0.0, 1.0],
            1.0,
            1.0,
        ),
    ];

    let mut fb = sentinel_fb(16, 16);
    let orig_fb = fb.data.clone();
    let mut zb64 = sentinel_zb_f64(16 * 16);
    let orig_zb64 = zb64.clone();
    let mut zb32 = sentinel_zb_f32(16 * 16);
    let orig_zb32 = zb32.clone();

    let cfg = RasterConfig::raster_config_new(Mat4::mat4_identity()).unwrap();
    let mut scratch = RasterScratch::scratch_with_capacity(6).unwrap();

    // 1. Mid-mesh NaN vertex (#4) must not mutate fb, zbuffer, or leave partial scratch
    assert_eq!(
        draw_indexed_mesh_into(
            &mut fb,
            DepthBufferMut::F64(&mut zb64),
            &cfg,
            None,
            &valid,
            &[0, 1, 2, 3, 4, 5],
            &mut scratch,
        ),
        Err(RasterError::NonFiniteInput)
    );
    assert_eq!(fb.data, orig_fb);
    assert_eq!(zb64, orig_zb64);

    // 2. OOB u32::MAX index (#5) must not mutate fb or f32 zbuffer
    assert_eq!(
        draw_indexed_mesh_into(
            &mut fb,
            DepthBufferMut::F32(&mut zb32),
            &cfg,
            None,
            &valid[..4],
            &[0, 1, 2, 0, 1, u32::MAX],
            &mut scratch,
        ),
        Err(RasterError::IndexOutOfBounds)
    );
    assert_eq!(fb.data, orig_fb);
    assert_eq!(zb32, orig_zb32);

    // 3. Non-multiple-of-3 index slice
    assert_eq!(
        draw_indexed_mesh_into(
            &mut fb,
            DepthBufferMut::F64(&mut zb64),
            &cfg,
            None,
            &valid[..3],
            &[0, 1, 2, 0],
            &mut scratch,
        ),
        Err(RasterError::InvalidIndexCount)
    );
    assert_eq!(fb.data, orig_fb);
    assert_eq!(zb64, orig_zb64);

    // 4. MissingTexture
    let mut tex_cfg = cfg;
    tex_cfg.shading = ShadingMode::TexturedBilinear;
    assert_eq!(
        draw_indexed_mesh_into(
            &mut fb,
            DepthBufferMut::F64(&mut zb64),
            &tex_cfg,
            None,
            &valid[..3],
            &[0, 1, 2],
            &mut scratch,
        ),
        Err(RasterError::MissingTexture)
    );
    assert_eq!(fb.data, orig_fb);
    assert_eq!(zb64, orig_zb64);

    // 5. InvalidLight inside RasterConfig
    let mut bad_light_cfg = cfg;
    bad_light_cfg.light.ambient_intensity = -1.0;
    assert_eq!(
        draw_triangle_into(
            &mut fb,
            DepthBufferMut::F64(&mut zb64),
            &bad_light_cfg,
            None,
            &valid[0],
            &valid[1],
            &valid[2],
        ),
        Err(RasterError::InvalidLight)
    );
    assert_eq!(fb.data, orig_fb);
    assert_eq!(zb64, orig_zb64);

    // 6. Truncated framebuffer / mismatched Z-buffer length
    let mut short_zb = vec![1.0_f64; 10];
    assert_eq!(
        draw_triangle_into(
            &mut fb,
            DepthBufferMut::F64(&mut short_zb),
            &cfg,
            None,
            &valid[0],
            &valid[1],
            &valid[2],
        ),
        Err(RasterError::InvalidDimensions)
    );
    assert_eq!(fb.data, orig_fb);

    // 7. Excessive scratch capacity request
    assert!(matches!(
        RasterScratch::scratch_with_capacity(usize::MAX),
        Err(RasterError::MeshTooLarge)
    ));
    assert!(matches!(
        RasterScratch::scratch_with_capacity(MAX_VERTICES + 1),
        Err(RasterError::MeshTooLarge)
    ));
}

#[test]
fn seeded_2000_case_hostile_input_fuzz_campaign() {
    let mut rng = 0xbaad_f00d_cafe_4321_u64;
    let mut tex_pc = PixelContainer::new(4, 4);
    tex_pc.fill(200, 120, 80, 255);
    let tex = Texture::texture_from_container(&tex_pc).unwrap();
    let mut scratch = RasterScratch::scratch_with_capacity(16).unwrap();

    for case in 0..2000u64 {
        let mut fb = sentinel_fb(16, 16);
        let orig_fb = fb.data.clone();
        let mut zb64 = sentinel_zb_f64(16 * 16);
        let orig_zb64 = zb64.clone();
        let mut zb32 = sentinel_zb_f32(16 * 16);
        let orig_zb32 = zb32.clone();

        let mut mvp = Mat4::mat4_identity();
        if next_u64(&mut rng).is_multiple_of(4) {
            mvp.cols[(next_u64(&mut rng) % 4) as usize][(next_u64(&mut rng) % 4) as usize] =
                pick_hostile_f64(&mut rng);
        }
        let mut nmat = Mat3::mat3_identity();
        if next_u64(&mut rng).is_multiple_of(5) {
            nmat.cols[(next_u64(&mut rng) % 3) as usize][(next_u64(&mut rng) % 3) as usize] =
                pick_hostile_f64(&mut rng);
        }

        let light_dir = Vec3::new(
            pick_hostile_f64(&mut rng),
            pick_hostile_f64(&mut rng),
            pick_hostile_f64(&mut rng),
        );
        let dir_res = DirectionalLight::dir_light_new(
            light_dir,
            [
                pick_hostile_f64(&mut rng),
                pick_hostile_f64(&mut rng),
                pick_hostile_f64(&mut rng),
            ],
            pick_hostile_f64(&mut rng),
        );
        let dir = dir_res.unwrap_or(DirectionalLight {
            direction_to_light: light_dir,
            color: [1.0, 1.0, 1.0],
            intensity: pick_hostile_f64(&mut rng),
        });
        let light = LightRig {
            ambient_color: [1.0, 1.0, 1.0],
            ambient_intensity: pick_hostile_f64(&mut rng),
            directional: dir,
        };

        let cfg = RasterConfig {
            mvp,
            normal_matrix: nmat,
            shading: match case % 5 {
                0 => ShadingMode::Flat,
                1 => ShadingMode::Gouraud,
                2 => ShadingMode::Phong,
                3 => ShadingMode::TexturedNearest,
                _ => ShadingMode::TexturedBilinear,
            },
            cull: match case % 3 {
                0 => CullMode::None,
                1 => CullMode::Back,
                _ => CullMode::Front,
            },
            front_face: if case.is_multiple_of(2) {
                FrontFace::CounterClockwise
            } else {
                FrontFace::Clockwise
            },
            texture_wrap: if case.is_multiple_of(2) {
                TextureWrap::Repeat
            } else {
                TextureWrap::Clamp
            },
            clip_space: if case.is_multiple_of(2) {
                ClipSpace::OpenGl
            } else {
                ClipSpace::ZeroToOne
            },
            depth_test: !case.is_multiple_of(7),
            depth_write: !case.is_multiple_of(11),
            depth_func: if case.is_multiple_of(2) {
                DepthFunc::Less
            } else {
                DepthFunc::LessEqual
            },
            light,
        };

        let mut verts =
            [Vertex::vertex_new(Vec3::zero(), Vec3::unit_z(), [1.0, 1.0, 1.0], 0.0, 0.0); 4];
        for v in &mut verts {
            *v = Vertex::vertex_new(
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
                [
                    pick_hostile_f64(&mut rng),
                    pick_hostile_f64(&mut rng),
                    pick_hostile_f64(&mut rng),
                ],
                pick_hostile_f64(&mut rng),
                pick_hostile_f64(&mut rng),
            );
        }

        let use_tex = if next_u64(&mut rng).is_multiple_of(4) {
            None
        } else {
            Some(&tex)
        };

        if case.is_multiple_of(2) {
            let res = draw_triangle_into(
                &mut fb,
                DepthBufferMut::F64(&mut zb64),
                &cfg,
                use_tex,
                &verts[0],
                &verts[1],
                &verts[2],
            );
            if res.is_err() {
                assert_eq!(fb.data, orig_fb);
                assert_eq!(zb64, orig_zb64);
            }
        } else {
            let indices = [
                (next_u64(&mut rng) % 6) as u32,
                (next_u64(&mut rng) % 6) as u32,
                (next_u64(&mut rng) % 6) as u32,
                (next_u64(&mut rng) % 6) as u32,
                (next_u64(&mut rng) % 6) as u32,
                (next_u64(&mut rng) % 6) as u32,
            ];
            let res = draw_indexed_mesh_into(
                &mut fb,
                DepthBufferMut::F32(&mut zb32),
                &cfg,
                use_tex,
                &verts,
                &indices,
                &mut scratch,
            );
            if res.is_err() {
                assert_eq!(fb.data, orig_fb);
                assert_eq!(zb32, orig_zb32);
            }
        }
    }
}
