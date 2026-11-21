//! # Conformance Tests for `raster3d-soft` (GAME33)
//!
//! Covers:
//! - Warrant contract validation helpers (`validate_*`)
//! - MVP vertex transformation, OpenGL `[-1, 1]` and `ZeroToOne` `[0, 1]` clip spaces
//! - Homogeneous 4D Sutherland-Hodgman near-plane (`w >= MIN_CLIP_W`) & frustum clipping
//! - Perspective-correct barycentric attribute interpolation (`1/w`, RGB, U/V, normals)
//! - Watertight edge-function half-space rasterization (top-left tie-breaking)
//! - Shading modes: `Flat`, `Gouraud`, `Phong`, `TexturedNearest`, `TexturedBilinear`
//! - `TextureWrap::Repeat` and `TextureWrap::Clamp`, directional + ambient lighting
//! - Backface / frontface culling (`CounterClockwise` / `Clockwise`)
//! - `f64` and `f32` Z-buffer depth ordering invariance & `frame_digest_fnv1a` determinism
//! - Security & CWE-20 / CWE-125 / CWE-190 / CWE-369 / CWE-400 / CWE-787 guards
//! - 1,000-case deterministic pseudo-random robustness property suite

use math3d::{Mat4, Quat, Vec3};
use pixel_container::PixelContainer;
use raster3d_soft::{
    clear_depth_f32, clear_depth_f64, clear_framebuffer, depth_digest_f32_fnv1a,
    depth_digest_f64_fnv1a, draw_indexed_mesh_into, draw_triangle_into, frame_digest_fnv1a,
    validate_clip_w, validate_framebuffer_dim, validate_texture_dim, validate_triangle_count,
    validate_vertex_index, ClipSpace, CullMode, DepthBufferMut, DepthFunc, DirectionalLight,
    FrontFace, LightRig, RasterConfig, RasterError, RasterScratch, ShadingMode, Texture,
    TextureWrap, Vertex, MAX_COORD, MAX_FRAMEBUFFER_HEIGHT, MAX_FRAMEBUFFER_WIDTH, MAX_TEXTURE_DIM,
    MAX_TRIANGLES, MAX_VERTICES, MIN_CLIP_W,
};

fn next_u64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    x.wrapping_mul(0x2545_f491_4f6c_dd1d)
}

fn next_range(state: &mut u64, lo: f64, hi: f64) -> f64 {
    let u = (next_u64(state) >> 11) as f64 / ((1u64 << 53) as f64);
    lo + (hi - lo) * u
}

fn px(fb: &PixelContainer, x: u32, y: u32) -> [u8; 4] {
    let (r, g, b, a) = fb.pixel_at(x, y);
    [r, g, b, a]
}

fn make_checker_texture(size: u32) -> PixelContainer {
    let mut pc = PixelContainer::new(size, size);
    for y in 0..size {
        for x in 0..size {
            let even = ((x / (size / 2).max(1)) + (y / (size / 2).max(1))).is_multiple_of(2);
            if even {
                pc.set_pixel(x, y, 255, 255, 255, 255);
            } else {
                pc.set_pixel(x, y, 20, 40, 220, 255);
            }
        }
    }
    pc
}

fn make_unit_cube() -> (Vec<Vertex>, Vec<u32>) {
    let p = [
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(-1.0, 1.0, 1.0),
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(1.0, 1.0, -1.0),
        Vec3::new(-1.0, 1.0, -1.0),
    ];
    let faces: [([usize; 4], Vec3, [f64; 3]); 6] = [
        ([0, 1, 2, 3], Vec3::new(0.0, 0.0, 1.0), [1.0, 0.2, 0.2]),
        ([5, 4, 7, 6], Vec3::new(0.0, 0.0, -1.0), [0.2, 1.0, 0.2]),
        ([1, 5, 6, 2], Vec3::new(1.0, 0.0, 0.0), [0.2, 0.2, 1.0]),
        ([4, 0, 3, 7], Vec3::new(-1.0, 0.0, 0.0), [1.0, 1.0, 0.2]),
        ([3, 2, 6, 7], Vec3::new(0.0, 1.0, 0.0), [0.2, 1.0, 1.0]),
        ([4, 5, 1, 0], Vec3::new(0.0, -1.0, 0.0), [1.0, 0.2, 1.0]),
    ];
    let uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
    let mut verts = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for (quad, n, col) in faces {
        let base = verts.len() as u32;
        for (i, &vi) in quad.iter().enumerate() {
            verts.push(Vertex::vertex_new(p[vi], n, col, uvs[i][0], uvs[i][1]));
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (verts, indices)
}

#[test]
fn warrant_contract_validators() {
    assert_eq!(validate_framebuffer_dim(1), Ok(true));
    assert_eq!(validate_framebuffer_dim(MAX_FRAMEBUFFER_WIDTH), Ok(true));
    assert_eq!(
        validate_framebuffer_dim(0),
        Err(RasterError::InvalidDimensions)
    );
    assert_eq!(
        validate_framebuffer_dim(MAX_FRAMEBUFFER_WIDTH + 1),
        Err(RasterError::InvalidDimensions)
    );
    assert_eq!(validate_texture_dim(MAX_TEXTURE_DIM), Ok(true));
    assert_eq!(validate_texture_dim(0), Err(RasterError::InvalidDimensions));
    assert_eq!(validate_triangle_count(MAX_TRIANGLES), Ok(true));
    assert_eq!(
        validate_triangle_count(MAX_TRIANGLES + 1),
        Err(RasterError::MeshTooLarge)
    );
    assert_eq!(validate_vertex_index(2, 3), Ok(true));
    assert_eq!(
        validate_vertex_index(3, 3),
        Err(RasterError::IndexOutOfBounds)
    );
    assert_eq!(validate_clip_w(MIN_CLIP_W), Ok(true));
    assert_eq!(validate_clip_w(0.0), Err(RasterError::NearClipRejected));
    assert_eq!(validate_clip_w(f64::NAN), Err(RasterError::NonFiniteInput));
}

#[test]
fn single_triangle_gouraud_and_flat_shading() {
    let mut fb = PixelContainer::new(64, 64);
    clear_framebuffer(&mut fb, [0, 0, 0, 255]).unwrap();
    let mut zb = vec![1.0_f64; 64 * 64];

    let mut cfg = RasterConfig::raster_config_new(Mat4::mat4_identity()).unwrap();
    cfg.light = LightRig::light_rig_unlit();
    cfg.shading = ShadingMode::Gouraud;

    // CCW triangle in NDC covering center of 64x64 viewport
    let v0 = Vertex::vertex_new(
        Vec3::new(-0.5, -0.5, 0.0),
        Vec3::unit_z(),
        [1.0, 0.0, 0.0],
        0.0,
        0.0,
    );
    let v1 = Vertex::vertex_new(
        Vec3::new(0.5, -0.5, 0.0),
        Vec3::unit_z(),
        [0.0, 1.0, 0.0],
        1.0,
        0.0,
    );
    let v2 = Vertex::vertex_new(
        Vec3::new(0.0, 0.5, 0.0),
        Vec3::unit_z(),
        [0.0, 0.0, 1.0],
        0.5,
        1.0,
    );

    let stats = draw_triangle_into(
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &cfg,
        None,
        &v0,
        &v1,
        &v2,
    )
    .unwrap();
    assert_eq!(stats.triangles_submitted, 1);
    assert_eq!(stats.triangles_rasterized, 1);
    assert!(stats.pixels_written > 400);

    // Corner outside triangle stays black
    assert_eq!(px(&fb, 0, 0), [0, 0, 0, 255]);
    // Center pixel has blended RGB (~1/3 each)
    let center = px(&fb, 32, 32);
    assert!(center[0] > 40 && center[1] > 40 && center[2] > 60);

    // Flat mode renders uniform color over all covered pixels
    clear_framebuffer(&mut fb, [0, 0, 0, 255]).unwrap();
    clear_depth_f64(&mut zb, 1.0).unwrap();
    cfg.shading = ShadingMode::Flat;
    draw_triangle_into(
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &cfg,
        None,
        &v0,
        &v1,
        &v2,
    )
    .unwrap();
    let c1 = px(&fb, 32, 32);
    let c2 = px(&fb, 28, 40);
    assert_eq!(c1, c2);
    assert!(c1[0] > 70 && c1[1] > 70 && c1[2] > 70);
}

#[test]
fn zbuffer_order_invariance_f64_and_f32() {
    let mut cfg = RasterConfig::raster_config_new(Mat4::mat4_identity()).unwrap();
    cfg.light = LightRig::light_rig_unlit();
    cfg.shading = ShadingMode::Flat;

    let near_red = [
        Vertex::vertex_new(
            Vec3::new(-0.5, -0.5, -0.5),
            Vec3::unit_z(),
            [1.0, 0.0, 0.0],
            0.0,
            0.0,
        ),
        Vertex::vertex_new(
            Vec3::new(0.5, -0.5, -0.5),
            Vec3::unit_z(),
            [1.0, 0.0, 0.0],
            0.0,
            0.0,
        ),
        Vertex::vertex_new(
            Vec3::new(0.0, 0.5, -0.5),
            Vec3::unit_z(),
            [1.0, 0.0, 0.0],
            0.0,
            0.0,
        ),
    ];
    let far_blue = [
        Vertex::vertex_new(
            Vec3::new(-0.6, -0.6, 0.4),
            Vec3::unit_z(),
            [0.0, 0.0, 1.0],
            0.0,
            0.0,
        ),
        Vertex::vertex_new(
            Vec3::new(0.6, -0.6, 0.4),
            Vec3::unit_z(),
            [0.0, 0.0, 1.0],
            0.0,
            0.0,
        ),
        Vertex::vertex_new(
            Vec3::new(0.0, 0.6, 0.4),
            Vec3::unit_z(),
            [0.0, 0.0, 1.0],
            0.0,
            0.0,
        ),
    ];

    // Order A: near then far (f64)
    let mut fb_a = PixelContainer::new(48, 48);
    clear_framebuffer(&mut fb_a, [0, 0, 0, 255]).unwrap();
    let mut zb_a = vec![1.0_f64; 48 * 48];
    draw_triangle_into(
        &mut fb_a,
        DepthBufferMut::F64(&mut zb_a),
        &cfg,
        None,
        &near_red[0],
        &near_red[1],
        &near_red[2],
    )
    .unwrap();
    draw_triangle_into(
        &mut fb_a,
        DepthBufferMut::F64(&mut zb_a),
        &cfg,
        None,
        &far_blue[0],
        &far_blue[1],
        &far_blue[2],
    )
    .unwrap();

    // Order B: far then near (f64)
    let mut fb_b = PixelContainer::new(48, 48);
    clear_framebuffer(&mut fb_b, [0, 0, 0, 255]).unwrap();
    let mut zb_b = vec![1.0_f64; 48 * 48];
    draw_triangle_into(
        &mut fb_b,
        DepthBufferMut::F64(&mut zb_b),
        &cfg,
        None,
        &far_blue[0],
        &far_blue[1],
        &far_blue[2],
    )
    .unwrap();
    draw_triangle_into(
        &mut fb_b,
        DepthBufferMut::F64(&mut zb_b),
        &cfg,
        None,
        &near_red[0],
        &near_red[1],
        &near_red[2],
    )
    .unwrap();

    assert_eq!(frame_digest_fnv1a(&fb_a), frame_digest_fnv1a(&fb_b));
    assert_eq!(depth_digest_f64_fnv1a(&zb_a), depth_digest_f64_fnv1a(&zb_b));
    assert_eq!(px(&fb_a, 24, 24), [255, 0, 0, 255]);

    // Order C: far then near (f32)
    let mut fb_c = PixelContainer::new(48, 48);
    clear_framebuffer(&mut fb_c, [0, 0, 0, 255]).unwrap();
    let mut zb_c = vec![1.0_f32; 48 * 48];
    draw_triangle_into(
        &mut fb_c,
        DepthBufferMut::F32(&mut zb_c),
        &cfg,
        None,
        &far_blue[0],
        &far_blue[1],
        &far_blue[2],
    )
    .unwrap();
    draw_triangle_into(
        &mut fb_c,
        DepthBufferMut::F32(&mut zb_c),
        &cfg,
        None,
        &near_red[0],
        &near_red[1],
        &near_red[2],
    )
    .unwrap();
    assert_eq!(frame_digest_fnv1a(&fb_a), frame_digest_fnv1a(&fb_c));
    assert!(depth_digest_f32_fnv1a(&zb_c) != 0);
}

#[test]
fn backface_and_frontface_culling() {
    let mut fb = PixelContainer::new(32, 32);
    let mut zb = vec![1.0_f64; 32 * 32];
    let mut cfg = RasterConfig::raster_config_new(Mat4::mat4_identity()).unwrap();
    cfg.light = LightRig::light_rig_unlit();

    // CW triangle in NDC (back-facing when FrontFace::CounterClockwise)
    let v0 = Vertex::vertex_new(
        Vec3::new(-0.5, -0.5, 0.0),
        Vec3::unit_z(),
        [1.0, 1.0, 1.0],
        0.0,
        0.0,
    );
    let v1 = Vertex::vertex_new(
        Vec3::new(0.0, 0.5, 0.0),
        Vec3::unit_z(),
        [1.0, 1.0, 1.0],
        0.0,
        0.0,
    );
    let v2 = Vertex::vertex_new(
        Vec3::new(0.5, -0.5, 0.0),
        Vec3::unit_z(),
        [1.0, 1.0, 1.0],
        0.0,
        0.0,
    );

    clear_framebuffer(&mut fb, [0, 0, 0, 255]).unwrap();
    cfg.cull = CullMode::Back;
    cfg.front_face = FrontFace::CounterClockwise;
    let s1 = draw_triangle_into(
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &cfg,
        None,
        &v0,
        &v1,
        &v2,
    )
    .unwrap();
    assert_eq!(s1.triangles_culled, 1);
    assert_eq!(s1.pixels_written, 0);

    // Same triangle is front-facing under FrontFace::Clockwise
    clear_depth_f64(&mut zb, 1.0).unwrap();
    cfg.front_face = FrontFace::Clockwise;
    let s2 = draw_triangle_into(
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &cfg,
        None,
        &v0,
        &v1,
        &v2,
    )
    .unwrap();
    assert_eq!(s2.triangles_culled, 0);
    assert!(s2.pixels_written > 100);

    // CullMode::Front culls it again
    clear_depth_f64(&mut zb, 1.0).unwrap();
    cfg.cull = CullMode::Front;
    let s3 = draw_triangle_into(
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &cfg,
        None,
        &v0,
        &v1,
        &v2,
    )
    .unwrap();
    assert_eq!(s3.triangles_culled, 1);
    assert_eq!(s3.pixels_written, 0);

    // CullMode::None renders regardless
    clear_depth_f64(&mut zb, 1.0).unwrap();
    cfg.cull = CullMode::None;
    let s4 = draw_triangle_into(
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &cfg,
        None,
        &v0,
        &v1,
        &v2,
    )
    .unwrap();
    assert_eq!(s4.triangles_rasterized, 1);
    assert!(s4.pixels_written > 100);
}

#[test]
fn watertight_shared_edge_rasterization() {
    // Full-quad covering [8, 24) x [8, 24) in a 32x32 framebuffer (exact 16x16 = 256 pixels)
    // NDC coords for x in [8, 24] out of 32: [-0.5, +0.5]; y in [8, 24]: [+0.5, -0.5]
    let mut fb = PixelContainer::new(32, 32);
    clear_framebuffer(&mut fb, [0, 0, 0, 255]).unwrap();
    let mut zb = vec![1.0_f64; 32 * 32];

    let mut cfg = RasterConfig::raster_config_new(Mat4::mat4_identity()).unwrap();
    cfg.light = LightRig::light_rig_unlit();
    cfg.shading = ShadingMode::Flat;
    cfg.depth_func = DepthFunc::Less; // Strict `<` rejects any double-written shared-edge pixel

    let verts = [
        Vertex::vertex_new(
            Vec3::new(-0.5, -0.5, 0.0),
            Vec3::unit_z(),
            [1.0, 1.0, 1.0],
            0.0,
            0.0,
        ),
        Vertex::vertex_new(
            Vec3::new(0.5, -0.5, 0.0),
            Vec3::unit_z(),
            [1.0, 1.0, 1.0],
            1.0,
            0.0,
        ),
        Vertex::vertex_new(
            Vec3::new(0.5, 0.5, 0.0),
            Vec3::unit_z(),
            [1.0, 1.0, 1.0],
            1.0,
            1.0,
        ),
        Vertex::vertex_new(
            Vec3::new(-0.5, 0.5, 0.0),
            Vec3::unit_z(),
            [1.0, 1.0, 1.0],
            0.0,
            1.0,
        ),
    ];
    let indices = [0u32, 1, 2, 0, 2, 3];
    let mut scratch = RasterScratch::scratch_with_capacity(4).unwrap();
    let stats = draw_indexed_mesh_into(
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &cfg,
        None,
        &verts,
        &indices,
        &mut scratch,
    )
    .unwrap();

    // Every pixel in [8, 24) x [8, 24) written exactly once with no gaps or overlaps
    assert_eq!(stats.pixels_written, 256);
    for y in 0..32 {
        for x in 0..32 {
            let inside = (8..24).contains(&x) && (8..24).contains(&y);
            let expected = if inside {
                [255, 255, 255, 255]
            } else {
                [0, 0, 0, 255]
            };
            assert_eq!(px(&fb, x, y), expected, "mismatch at ({x}, {y})");
        }
    }
}

#[test]
fn sutherland_hodgman_near_plane_and_frustum_clipping() {
    let view = Mat4::look_at(Vec3::new(0.0, 0.0, 2.0), Vec3::zero(), Vec3::unit_y()).unwrap();
    let proj = Mat4::perspective(std::f64::consts::FRAC_PI_2, 1.0, 1.0, 20.0).unwrap();
    let mvp = proj.mul(&view).unwrap();

    let mut cfg = RasterConfig::raster_config_new(mvp).unwrap();
    cfg.light = LightRig::light_rig_unlit();
    cfg.cull = CullMode::None;

    let mut fb = PixelContainer::new(64, 64);
    clear_framebuffer(&mut fb, [0, 0, 0, 255]).unwrap();
    let mut zb = vec![1.0_f64; 64 * 64];

    // Triangle spans from behind camera (z = 4.0, view z = +2 -> w = -2) to in front (z = -2.0, w = 4)
    let v0 = Vertex::vertex_new(
        Vec3::new(-1.0, -1.0, -2.0),
        Vec3::unit_z(),
        [0.0, 1.0, 0.0],
        0.0,
        0.0,
    );
    let v1 = Vertex::vertex_new(
        Vec3::new(1.0, -1.0, -2.0),
        Vec3::unit_z(),
        [0.0, 1.0, 0.0],
        1.0,
        0.0,
    );
    let v2 = Vertex::vertex_new(
        Vec3::new(0.0, 1.0, 4.0),
        Vec3::unit_z(),
        [0.0, 1.0, 0.0],
        0.5,
        1.0,
    );

    let stats = draw_triangle_into(
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &cfg,
        None,
        &v0,
        &v1,
        &v2,
    )
    .unwrap();
    assert!(stats.triangles_rasterized >= 1);
    assert!(stats.pixels_written > 0);

    // Triangle completely behind camera must be cleanly clipped out
    clear_framebuffer(&mut fb, [0, 0, 0, 255]).unwrap();
    clear_depth_f64(&mut zb, 1.0).unwrap();
    let behind0 = Vertex::vertex_new(
        Vec3::new(-1.0, -1.0, 5.0),
        Vec3::unit_z(),
        [1.0, 0.0, 0.0],
        0.0,
        0.0,
    );
    let behind1 = Vertex::vertex_new(
        Vec3::new(1.0, -1.0, 5.0),
        Vec3::unit_z(),
        [1.0, 0.0, 0.0],
        0.0,
        0.0,
    );
    let behind2 = Vertex::vertex_new(
        Vec3::new(0.0, 1.0, 5.0),
        Vec3::unit_z(),
        [1.0, 0.0, 0.0],
        0.0,
        0.0,
    );
    let stats_behind = draw_triangle_into(
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &cfg,
        None,
        &behind0,
        &behind1,
        &behind2,
    )
    .unwrap();
    assert_eq!(stats_behind.triangles_clipped_out, 1);
    assert_eq!(stats_behind.pixels_written, 0);

    // ZeroToOne clip space matches Mat4::perspective_zo
    let proj_zo = Mat4::perspective_zo(std::f64::consts::FRAC_PI_2, 1.0, 1.0, 20.0).unwrap();
    cfg.mvp = proj_zo.mul(&view).unwrap();
    cfg.clip_space = ClipSpace::ZeroToOne;
    clear_depth_f64(&mut zb, 1.0).unwrap();
    let stats_zo = draw_triangle_into(
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &cfg,
        None,
        &v0,
        &v1,
        &v2,
    )
    .unwrap();
    assert!(stats_zo.pixels_written > 0);
}

#[test]
fn perspective_correct_uv_and_textures_and_lighting() {
    // Verify perspective-correct UV interpolation on a sharply foreshortened quad:
    // Bottom edge at z = -1 (w = 1), top edge at z = -9 (w = 9).
    let view = Mat4::mat4_identity();
    let proj = Mat4::perspective(std::f64::consts::FRAC_PI_2, 1.0, 0.5, 20.0).unwrap();
    let mvp = proj.mul(&view).unwrap();

    // 2x1 texture: left texel red, right texel blue; or 16x16 horizontal gradient in red
    let mut tex_pc = PixelContainer::new(16, 16);
    for y in 0..16 {
        for x in 0..16 {
            let v_byte = (x * 16) as u8;
            tex_pc.set_pixel(x, y, v_byte, 0, 255 - v_byte, 255);
        }
    }
    let tex = Texture::texture_from_container(&tex_pc).unwrap();

    let mut cfg = RasterConfig::raster_config_new(mvp).unwrap();
    cfg.light = LightRig::light_rig_unlit();
    cfg.cull = CullMode::None;
    cfg.shading = ShadingMode::TexturedBilinear;
    cfg.texture_wrap = TextureWrap::Clamp;

    // Foreshortened triangle spanning x = [-1, 1] at z = -1 (w = 1) to x = [-1, 1] at z = -9 (w = 9)
    // Wait: in clip space, left-to-right has constant w on horizontal scanlines, whereas
    // vertically bottom has w = 1 (v = 0) and top has w = 9 (v = 1).
    let mut vtex_pc = PixelContainer::new(2, 2);
    // Top row (v = 0) = green, Bottom row (v = 1) = blue
    vtex_pc.set_pixel(0, 0, 0, 255, 0, 255);
    vtex_pc.set_pixel(1, 0, 0, 255, 0, 255);
    vtex_pc.set_pixel(0, 1, 0, 0, 255, 255);
    vtex_pc.set_pixel(1, 1, 0, 0, 255, 255);
    let vtex = Texture::texture_from_container(&vtex_pc).unwrap();

    let mut fb = PixelContainer::new(64, 64);
    clear_framebuffer(&mut fb, [0, 0, 0, 255]).unwrap();
    let mut zb = vec![1.0_f64; 64 * 64];

    // Quad in XZ plane at y = -0.5, from z = -1 (bottom of screen) to z = -9 (near horizon)
    // At y = -0.5, z = -1 -> ndc_y = -0.5 (sy = 48); at y = -0.5, z = -9 -> ndc_y = -0.0555 (sy = 33.8)
    let verts = [
        Vertex::vertex_new(
            Vec3::new(-0.9, -0.5, -1.0),
            Vec3::unit_y(),
            [1.0, 1.0, 1.0],
            0.0,
            0.0,
        ),
        Vertex::vertex_new(
            Vec3::new(0.9, -0.5, -1.0),
            Vec3::unit_y(),
            [1.0, 1.0, 1.0],
            1.0,
            0.0,
        ),
        Vertex::vertex_new(
            Vec3::new(4.0, -0.5, -9.0),
            Vec3::unit_y(),
            [1.0, 1.0, 1.0],
            1.0,
            1.0,
        ),
        Vertex::vertex_new(
            Vec3::new(-4.0, -0.5, -9.0),
            Vec3::unit_y(),
            [1.0, 1.0, 1.0],
            0.0,
            1.0,
        ),
    ];
    let indices = [0u32, 1, 2, 0, 2, 3];
    let mut scratch = RasterScratch::scratch_with_capacity(4).unwrap();
    cfg.shading = ShadingMode::TexturedNearest;
    draw_indexed_mesh_into(
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &cfg,
        Some(&vtex),
        &verts,
        &indices,
        &mut scratch,
    )
    .unwrap();

    // Near bottom (sy = 46, close to z = -1, v ~ 0 -> green)
    let near_px = px(&fb, 32, 46);
    assert_eq!(near_px, [0, 255, 0, 255]);
    // Screen midpoint between sy=48 and sy=33.78 is sy=41 (perspective v ~ 0.10 < 0.5 -> green).
    // Even at sy=35 (py=35.5), perspective v ~ 0.446 < 0.5 -> still green (affine would be 0.88 -> blue).
    let mid_px = px(&fb, 32, 41);
    assert_eq!(mid_px, [0, 255, 0, 255]);
    assert_eq!(px(&fb, 32, 35), [0, 255, 0, 255]);
    // At sy=34 (py=34.5), world z ~ -7.1 -> perspective v ~ 0.675 > 0.5 -> blue!
    let far_px = px(&fb, 32, 34);
    assert_eq!(far_px, [0, 0, 255, 255]);

    // Also test TexturedBilinear + Repeat wrap + Phong shading on cube
    let (cube_v, cube_i) = make_unit_cube();
    let rot = Quat::from_euler_yxz(0.6, 0.4, 0.1).unwrap();
    let model = Mat4::mat4_from_trs(Vec3::zero(), rot, Vec3::new(1.0, 1.0, 1.0)).unwrap();
    let view_c = Mat4::look_at(Vec3::new(0.0, 0.0, 4.0), Vec3::zero(), Vec3::unit_y()).unwrap();
    let proj_c = Mat4::perspective(std::f64::consts::FRAC_PI_3, 1.0, 0.5, 20.0).unwrap();
    let mut cube_cfg =
        RasterConfig::raster_config_new(proj_c.mul(&view_c).unwrap().mul(&model).unwrap()).unwrap();
    cube_cfg.normal_matrix = model.upper_mat3().unwrap();
    cube_cfg.shading = ShadingMode::TexturedBilinear;
    cube_cfg.texture_wrap = TextureWrap::Repeat;
    cube_cfg.light = LightRig::light_rig_new(
        [1.0, 1.0, 1.0],
        0.2,
        DirectionalLight::dir_light_new(Vec3::new(0.5, 1.0, 1.0), [1.0, 1.0, 1.0], 0.8).unwrap(),
    )
    .unwrap();

    clear_framebuffer(&mut fb, [10, 10, 15, 255]).unwrap();
    clear_depth_f64(&mut zb, 1.0).unwrap();
    let mut cube_scratch = RasterScratch::scratch_with_capacity(cube_v.len()).unwrap();
    let stats_bilin = draw_indexed_mesh_into(
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &cube_cfg,
        Some(&tex),
        &cube_v,
        &cube_i,
        &mut cube_scratch,
    )
    .unwrap();
    assert!(stats_bilin.pixels_written > 500);
    let digest_1 = frame_digest_fnv1a(&fb);

    // Phong shading mode produces deterministic non-trivial output too
    cube_cfg.shading = ShadingMode::Phong;
    clear_framebuffer(&mut fb, [10, 10, 15, 255]).unwrap();
    clear_depth_f64(&mut zb, 1.0).unwrap();
    let stats_phong = draw_indexed_mesh_into(
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &cube_cfg,
        None,
        &cube_v,
        &cube_i,
        &mut cube_scratch,
    )
    .unwrap();
    assert_eq!(stats_phong.pixels_written, stats_bilin.pixels_written);

    // Re-rendering TexturedBilinear produces bit-exact digest
    cube_cfg.shading = ShadingMode::TexturedBilinear;
    clear_framebuffer(&mut fb, [10, 10, 15, 255]).unwrap();
    clear_depth_f64(&mut zb, 1.0).unwrap();
    draw_indexed_mesh_into(
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &cube_cfg,
        Some(&tex),
        &cube_v,
        &cube_i,
        &mut cube_scratch,
    )
    .unwrap();
    assert_eq!(frame_digest_fnv1a(&fb), digest_1);
    assert_eq!(make_checker_texture(8).width, 8);
}

#[test]
fn security_and_cwe_guards() {
    let mut fb = PixelContainer::new(16, 16);
    let mut zb = vec![1.0_f64; 16 * 16];
    let cfg = RasterConfig::raster_config_new(Mat4::mat4_identity()).unwrap();
    let mut scratch = RasterScratch::scratch_new();

    let valid_v = Vertex::vertex_new(Vec3::zero(), Vec3::unit_z(), [1.0, 1.0, 1.0], 0.0, 0.0);

    // CWE-20: Non-finite vertex, matrix, light, clear depth
    let nan_v = Vertex::vertex_new(
        Vec3::new(f64::NAN, 0.0, 0.0),
        Vec3::unit_z(),
        [1.0, 1.0, 1.0],
        0.0,
        0.0,
    );
    assert_eq!(
        draw_triangle_into(
            &mut fb,
            DepthBufferMut::F64(&mut zb),
            &cfg,
            None,
            &nan_v,
            &valid_v,
            &valid_v
        ),
        Err(RasterError::NonFiniteInput)
    );
    assert_eq!(
        Vertex::vertex_try_new(
            Vec3::new(MAX_COORD + 1.0, 0.0, 0.0),
            Vec3::unit_z(),
            [1.0, 1.0, 1.0],
            0.0,
            0.0
        ),
        Err(RasterError::ValueOutOfBounds)
    );
    assert_eq!(
        clear_depth_f64(&mut zb, f64::INFINITY),
        Err(RasterError::NonFiniteInput)
    );
    assert_eq!(
        clear_depth_f32(&mut [1.0_f32; 4], f32::NAN),
        Err(RasterError::NonFiniteInput)
    );

    let mut bad_mat = Mat4::mat4_identity();
    bad_mat.cols[0][0] = f64::INFINITY;
    assert_eq!(
        RasterConfig::raster_config_new(bad_mat),
        Err(RasterError::NonFiniteInput)
    );

    // CWE-369: Invalid light direction, negative intensity, degenerate screen area
    assert_eq!(
        DirectionalLight::dir_light_new(Vec3::zero(), [1.0, 1.0, 1.0], 1.0),
        Err(RasterError::InvalidLight)
    );
    assert_eq!(
        DirectionalLight::dir_light_new(Vec3::unit_y(), [1.0, 1.0, 1.0], -0.1),
        Err(RasterError::InvalidLight)
    );
    let col_a = Vertex::vertex_new(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::unit_z(),
        [1.0, 1.0, 1.0],
        0.0,
        0.0,
    );
    let col_b = Vertex::vertex_new(
        Vec3::new(0.5, 0.0, 0.0),
        Vec3::unit_z(),
        [1.0, 1.0, 1.0],
        0.0,
        0.0,
    );
    let col_c = Vertex::vertex_new(
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::unit_z(),
        [1.0, 1.0, 1.0],
        0.0,
        0.0,
    );
    let col_stats = draw_triangle_into(
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &cfg,
        None,
        &col_a,
        &col_b,
        &col_c,
    )
    .unwrap();
    assert_eq!(col_stats.triangles_culled, 1);
    assert_eq!(col_stats.pixels_written, 0);

    // CWE-400 / CWE-125 / CWE-787: Buffer dimension mismatches, OOB indices, missing texture
    let mut wrong_zb = vec![1.0_f64; 10];
    assert_eq!(
        draw_triangle_into(
            &mut fb,
            DepthBufferMut::F64(&mut wrong_zb),
            &cfg,
            None,
            &valid_v,
            &valid_v,
            &valid_v
        ),
        Err(RasterError::InvalidDimensions)
    );
    let zero_fb = PixelContainer::new(0, 16);
    assert_eq!(
        clear_framebuffer(&mut zero_fb.clone(), [0, 0, 0, 255]),
        Err(RasterError::InvalidDimensions)
    );
    assert_eq!(
        Texture::texture_from_rgba8(4, 4, &[0u8; 8]),
        Err(RasterError::InvalidDimensions)
    );
    assert_eq!(
        Texture::texture_from_rgba8(MAX_TEXTURE_DIM + 1, 1, &[]),
        Err(RasterError::InvalidDimensions)
    );
    assert!(matches!(
        RasterScratch::scratch_with_capacity(MAX_VERTICES + 1),
        Err(RasterError::MeshTooLarge)
    ));
    assert_eq!(
        draw_indexed_mesh_into(
            &mut fb,
            DepthBufferMut::F64(&mut zb),
            &cfg,
            None,
            &[valid_v],
            &[0, 1],
            &mut scratch
        ),
        Err(RasterError::InvalidIndexCount)
    );
    assert_eq!(
        draw_indexed_mesh_into(
            &mut fb,
            DepthBufferMut::F64(&mut zb),
            &cfg,
            None,
            &[valid_v],
            &[0, 0, 5],
            &mut scratch
        ),
        Err(RasterError::IndexOutOfBounds)
    );
    let mut tex_cfg = cfg;
    tex_cfg.shading = ShadingMode::TexturedNearest;
    assert_eq!(
        draw_triangle_into(
            &mut fb,
            DepthBufferMut::F64(&mut zb),
            &tex_cfg,
            None,
            &valid_v,
            &valid_v,
            &valid_v
        ),
        Err(RasterError::MissingTexture)
    );

    // CWE-190: Huge offscreen triangle clamps without integer overflow or panic
    let huge0 = Vertex::vertex_new(
        Vec3::new(-1.0e8, -1.0e8, 0.0),
        Vec3::unit_z(),
        [0.5, 0.5, 0.5],
        0.0,
        0.0,
    );
    let huge1 = Vertex::vertex_new(
        Vec3::new(1.0e8, -1.0e8, 0.0),
        Vec3::unit_z(),
        [0.5, 0.5, 0.5],
        0.0,
        0.0,
    );
    let huge2 = Vertex::vertex_new(
        Vec3::new(0.0, 1.0e8, 0.0),
        Vec3::unit_z(),
        [0.5, 0.5, 0.5],
        0.0,
        0.0,
    );
    let huge_stats = draw_triangle_into(
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &cfg,
        None,
        &huge0,
        &huge1,
        &huge2,
    )
    .unwrap();
    assert_eq!(huge_stats.pixels_written, 256);

    let mut bad_fb = PixelContainer::new(MAX_FRAMEBUFFER_WIDTH, 1);
    bad_fb.height = MAX_FRAMEBUFFER_HEIGHT + 1;
    assert_eq!(
        clear_framebuffer(&mut bad_fb, [0, 0, 0, 0]),
        Err(RasterError::InvalidDimensions)
    );
}

#[test]
fn deterministic_1000_case_randomized_raster_suite() {
    let mut rng = 0x5eed_cafe_1234_5678_u64;
    let mut fb = PixelContainer::new(32, 32);
    let mut zb = vec![1.0_f64; 32 * 32];
    let tex_pc = make_checker_texture(8);
    let tex = Texture::texture_from_container(&tex_pc).unwrap();

    let proj = Mat4::perspective(std::f64::consts::FRAC_PI_3, 1.0, 0.2, 50.0).unwrap();
    let view = Mat4::look_at(Vec3::new(0.0, 0.5, 3.0), Vec3::zero(), Vec3::unit_y()).unwrap();
    let vp = proj.mul(&view).unwrap();
    let mut cfg = RasterConfig::raster_config_new(vp).unwrap();
    cfg.cull = CullMode::None;

    for i in 0..1000 {
        cfg.shading = match i % 5 {
            0 => ShadingMode::Flat,
            1 => ShadingMode::Gouraud,
            2 => ShadingMode::Phong,
            3 => ShadingMode::TexturedNearest,
            _ => ShadingMode::TexturedBilinear,
        };
        clear_framebuffer(&mut fb, [0, 0, 0, 255]).unwrap();
        clear_depth_f64(&mut zb, 1.0).unwrap();

        let mut make_v = || {
            Vertex::vertex_new(
                Vec3::new(
                    next_range(&mut rng, -3.0, 3.0),
                    next_range(&mut rng, -3.0, 3.0),
                    next_range(&mut rng, -4.0, 4.0),
                ),
                Vec3::new(
                    next_range(&mut rng, -1.0, 1.0),
                    next_range(&mut rng, -1.0, 1.0),
                    next_range(&mut rng, -1.0, 1.0),
                ),
                [
                    next_range(&mut rng, 0.0, 1.0),
                    next_range(&mut rng, 0.0, 1.0),
                    next_range(&mut rng, 0.0, 1.0),
                ],
                next_range(&mut rng, -2.0, 2.0),
                next_range(&mut rng, -2.0, 2.0),
            )
        };
        let v0 = make_v();
        let v1 = make_v();
        let v2 = make_v();
        let stats = draw_triangle_into(
            &mut fb,
            DepthBufferMut::F64(&mut zb),
            &cfg,
            Some(&tex),
            &v0,
            &v1,
            &v2,
        )
        .unwrap();
        assert_eq!(stats.triangles_submitted, 1);
        assert!(stats.pixels_written <= 1024);
    }
}

#[test]
fn scratch_growth_and_huge_w_clip_safety() {
    let mut fb = PixelContainer::new(32, 32);
    let mut zb = vec![1.0_f64; 32 * 32];
    let mut scratch = RasterScratch::scratch_with_capacity(3).unwrap();

    // First draw with 3 vertices populates scratch (len=3, cap>=3)
    let cfg = RasterConfig::raster_config_new(Mat4::mat4_identity()).unwrap();
    let tri = [
        Vertex::vertex_new(
            Vec3::new(-0.5, -0.5, 0.0),
            Vec3::unit_z(),
            [1.0, 1.0, 1.0],
            0.0,
            0.0,
        ),
        Vertex::vertex_new(
            Vec3::new(0.5, -0.5, 0.0),
            Vec3::unit_z(),
            [1.0, 1.0, 1.0],
            1.0,
            0.0,
        ),
        Vertex::vertex_new(
            Vec3::new(0.0, 0.5, 0.0),
            Vec3::unit_z(),
            [1.0, 1.0, 1.0],
            0.5,
            1.0,
        ),
    ];
    draw_indexed_mesh_into(
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &cfg,
        None,
        &tri,
        &[0, 1, 2],
        &mut scratch,
    )
    .unwrap();

    // Second draw with 24-vertex cube must clear first and reserve >= 24
    let (cube_v, cube_i) = make_unit_cube();
    clear_depth_f64(&mut zb, 1.0).unwrap();
    draw_indexed_mesh_into(
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &cfg,
        None,
        &cube_v,
        &cube_i,
        &mut scratch,
    )
    .unwrap();
    assert!(scratch.scratch_capacity() >= cube_v.len());

    // Extreme homogeneous w / huge scale triggering Inf clip plane distance must not panic/NaN
    let mut huge_mvp = Mat4::mat4_identity();
    huge_mvp.cols[3][3] = 1.0e308;
    let huge_cfg = RasterConfig::raster_config_new(huge_mvp).unwrap();
    clear_depth_f64(&mut zb, 1.0).unwrap();
    let v_behind = Vertex::vertex_new(
        Vec3::new(0.0, 0.0, 10.0),
        Vec3::unit_z(),
        [1.0, 0.0, 0.0],
        0.0,
        0.0,
    );
    let _ = draw_triangle_into(
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &huge_cfg,
        None,
        &tri[0],
        &tri[1],
        &v_behind,
    )
    .unwrap();
}
