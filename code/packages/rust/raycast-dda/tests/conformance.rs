//! # Conformance Tests for `raycast-dda` (GAME31)
//!
//! Covers:
//! - Orthogonal 2D DDA wall raycasting (fisheye-free perpendicular distance,
//!   E/W vs N/S wall side detection and shading, wall U/V texture sampling)
//! - Perspective floor & ceiling plane casting (solid & textured)
//! - Z-buffer column occlusion (full and partial wall occlusion)
//! - Depth-sorted billboard sprite rendering (painter's order invariance, alpha cutout/blend)
//! - `math3d` & `trig` deterministic camera construction and rotation
//! - Security guards: CWE-20 (NaN/Inf), CWE-125/787 (bounds), CWE-369 (near-zero
//!   ray/camera/perp_dist), CWE-400/835 (dimension caps & `width + height` open-map cap)

use coding_adventures_sha256::sha256_hex;
use pixel_container::PixelContainer;
use raycast_dda::{
    cast_dda_ray, render_frame_into, Camera2D, GridMap, PlaneSurface, RaycastError, RenderConfig,
    Sprite, SpriteScratch, TextureRgba, WallSide, EPSILON, MAX_COORD, MAX_FRAME_WIDTH, MAX_MAP_DIM,
    MAX_SPRITES, MAX_TEXTURE_DIM, MIN_PERP_DIST,
};

fn make_box_map() -> GridMap {
    // 8x8 enclosed room with a central pillar at (4, 3)
    #[rustfmt::skip]
    let cells = vec![
        1, 1, 1, 1, 1, 1, 1, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 0, 0, 0, 2, 0, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 1,
        1, 1, 1, 1, 1, 1, 1, 1,
    ];
    GridMap::grid_new(8, 8, cells).expect("valid 8x8 box map")
}

#[test]
fn dda_fisheye_free_perpendicular_distance() {
    let map = make_box_map();
    // Camera at (2.5, 5.5) facing +X toward the flat East wall at x = 7 (clear of pillar at (4, 3)).
    let cam = Camera2D::camera_new(2.5, 5.5, 1.0, 0.0, 0.0, 0.66).unwrap();

    // All rays hitting the vertical wall at x = 7 (between y = 1..7) must have
    // the exact same perpendicular distance: 7.0 - 2.5 = 4.5.
    for camera_x in [-0.5_f64, -0.25, 0.0, 0.25, 0.5] {
        let rx = cam.dir_x + cam.plane_x * camera_x;
        let ry = cam.dir_y + cam.plane_y * camera_x;
        let hit = cast_dda_ray(&map, cam.pos_x, cam.pos_y, rx, ry).unwrap();
        assert!(hit.hit);
        assert_eq!(hit.map_x, 7);
        assert_eq!(hit.side, WallSide::EastWest);
        assert!(
            (hit.perp_wall_dist - 4.5).abs() < 1e-10,
            "expected perp_wall_dist=4.5, got {}",
            hit.perp_wall_dist
        );
    }
}

#[test]
fn dda_ns_vs_ew_side_shading_and_texture_uv() {
    // 5x5 enclosed room of wall tile 1.
    #[rustfmt::skip]
    let cells = vec![
        1, 1, 1, 1, 1,
        1, 0, 0, 0, 1,
        1, 0, 0, 0, 1,
        1, 0, 0, 0, 1,
        1, 1, 1, 1, 1,
    ];
    let map = GridMap::grid_new(5, 5, cells).unwrap();
    let solid_white = TextureRgba::texture_solid(16, 16, [200, 160, 120, 255]).unwrap();
    let textures = [solid_white];
    let config = RenderConfig::render_config_default();

    // Looking East (+X) hits EastWest wall -> full brightness [200, 160, 120, 255].
    let cam_east = Camera2D::camera_from_yaw_fov(2.5, 2.5, 0.0, trig::PI / 3.0).unwrap();
    let mut fb = PixelContainer::new(64, 64);
    let mut zbuf = vec![0.0; 64];
    let mut scratch = SpriteScratch::scratch_with_capacity(16).unwrap();

    render_frame_into(
        &map,
        &cam_east,
        &textures,
        &[],
        &config,
        &mut scratch,
        &mut zbuf,
        &mut fb,
    )
    .unwrap();
    assert_eq!(fb.pixel_at(32, 32), (200, 160, 120, 255));

    // Looking South (+Y = yaw PI/2) hits NorthSouth wall -> 50% shaded [100, 80, 60, 255].
    let cam_south =
        Camera2D::camera_from_yaw_fov(2.5, 2.5, trig::PI * 0.5, trig::PI / 3.0).unwrap();
    render_frame_into(
        &map,
        &cam_south,
        &textures,
        &[],
        &config,
        &mut scratch,
        &mut zbuf,
        &mut fb,
    )
    .unwrap();
    assert_eq!(fb.pixel_at(32, 32), (100, 80, 60, 255));

    // Verify checkerboard wall texture actually samples multiple U/V texels.
    let checker =
        TextureRgba::texture_checkerboard(16, 16, 4, [240, 240, 240, 255], [40, 40, 40, 255])
            .unwrap();
    render_frame_into(
        &map,
        &cam_east,
        &[checker],
        &[],
        &config,
        &mut scratch,
        &mut zbuf,
        &mut fb,
    )
    .unwrap();
    let mut saw_light = false;
    let mut saw_dark = false;
    for y in 20..44 {
        for x in 16..48 {
            match fb.pixel_at(x, y) {
                (240, 240, 240, 255) => saw_light = true,
                (40, 40, 40, 255) => saw_dark = true,
                _ => {}
            }
        }
    }
    assert!(
        saw_light && saw_dark,
        "expected checkerboard texels on wall"
    );
}

#[test]
fn floor_and_ceiling_plane_casting_textured() {
    // Open-ish 16x16 room so lots of floor & ceiling rows are visible.
    let mut cells = vec![0u16; 16 * 16];
    for i in 0..16 {
        cells[i] = 1;
        cells[15 * 16 + i] = 1;
        cells[i * 16] = 1;
        cells[i * 16 + 15] = 1;
    }
    let map = GridMap::grid_new(16, 16, cells).unwrap();

    let wall_tex = TextureRgba::texture_solid(16, 16, [180, 60, 60, 255]).unwrap();
    let floor_tex =
        TextureRgba::texture_checkerboard(16, 16, 8, [30, 180, 30, 255], [10, 60, 10, 255])
            .unwrap();
    let ceil_tex =
        TextureRgba::texture_checkerboard(16, 16, 8, [30, 30, 200, 255], [10, 10, 80, 255])
            .unwrap();
    let textures = [wall_tex, floor_tex, ceil_tex];

    let config = RenderConfig::render_config_with_planes(
        PlaneSurface::Textured { texture_index: 2 },
        PlaneSurface::Textured { texture_index: 1 },
        128,
    );
    let cam = Camera2D::camera_from_yaw_fov(8.5, 8.5, 0.3, trig::PI / 3.0).unwrap();
    let mut fb = PixelContainer::new(80, 60);
    let mut zbuf = vec![0.0; 80];
    let mut scratch = SpriteScratch::scratch_with_capacity(8).unwrap();

    render_frame_into(
        &map,
        &cam,
        &textures,
        &[],
        &config,
        &mut scratch,
        &mut zbuf,
        &mut fb,
    )
    .unwrap();

    // Top rows should contain both checkerboard ceiling colors.
    let mut ceil_colors = std::collections::BTreeSet::new();
    for y in 0..15 {
        for x in 0..80 {
            ceil_colors.insert(fb.pixel_at(x, y));
        }
    }
    assert!(ceil_colors.contains(&(30, 30, 200, 255)));
    assert!(ceil_colors.contains(&(10, 10, 80, 255)));

    // Bottom rows should contain both checkerboard floor colors.
    let mut floor_colors = std::collections::BTreeSet::new();
    for y in 45..60 {
        for x in 0..80 {
            floor_colors.insert(fb.pixel_at(x, y));
        }
    }
    assert!(floor_colors.contains(&(30, 180, 30, 255)));
    assert!(floor_colors.contains(&(10, 60, 10, 255)));
}

#[test]
fn zbuffer_occlusion_and_depth_sorted_billboard_sprites() {
    // 10x10 room with a wall pillar at (4, 5)
    let mut cells = vec![0u16; 10 * 10];
    for i in 0..10 {
        cells[i] = 1;
        cells[9 * 10 + i] = 1;
        cells[i * 10] = 1;
        cells[i * 10 + 9] = 1;
    }
    cells[5 * 10 + 4] = 1; // pillar directly between x=2 and x=7 along y=5.5
    let map = GridMap::grid_new(10, 10, cells).unwrap();

    let wall_tex = TextureRgba::texture_solid(8, 8, [100, 100, 100, 255]).unwrap();
    let near_tex = TextureRgba::texture_solid(8, 8, [0, 255, 0, 255]).unwrap();
    let far_tex = TextureRgba::texture_solid(8, 8, [255, 0, 0, 255]).unwrap();
    let hidden_tex = TextureRgba::texture_solid(8, 8, [0, 0, 255, 255]).unwrap();
    let textures = [wall_tex, near_tex, far_tex, hidden_tex];

    let cam = Camera2D::camera_new(2.0, 2.5, 1.0, 0.0, 0.0, 0.66).unwrap();
    let config = RenderConfig::render_config_default();

    // Two overlapping sprites along y=2.5 (clear line of sight): far at x=6.0, near at x=4.0.
    // Plus a hidden sprite behind the pillar when camera looks through pillar at (4, 5):
    let s_far = Sprite::sprite_new(6.0, 2.5, 2).unwrap();
    let s_near = Sprite::sprite_new(4.0, 2.5, 1).unwrap();

    let mut fb_a = PixelContainer::new(80, 60);
    let mut fb_b = PixelContainer::new(80, 60);
    let mut zbuf = vec![0.0; 80];
    let mut scratch = SpriteScratch::scratch_with_capacity(8).unwrap();

    // Order A: [far, near]
    render_frame_into(
        &map,
        &cam,
        &textures,
        &[s_far, s_near],
        &config,
        &mut scratch,
        &mut zbuf,
        &mut fb_a,
    )
    .unwrap();

    // Order B: [near, far] — depth sorting must produce identical framebuffer!
    render_frame_into(
        &map,
        &cam,
        &textures,
        &[s_near, s_far],
        &config,
        &mut scratch,
        &mut zbuf,
        &mut fb_b,
    )
    .unwrap();

    assert_eq!(
        fb_a.data, fb_b.data,
        "sprite order must not affect sorted output"
    );
    // Center pixel must show near green sprite, NOT far red sprite.
    assert_eq!(fb_a.pixel_at(40, 30), (0, 255, 0, 255));

    // Now place camera at (2.0, 5.5) looking +X straight into pillar at (4, 5).
    // A sprite at (7.0, 5.5) behind the pillar must be occluded by Z-buffer!
    let cam_occluded = Camera2D::camera_new(2.0, 5.5, 1.0, 0.0, 0.0, 0.66).unwrap();
    let s_hidden = Sprite::sprite_new(7.0, 5.5, 3).unwrap();
    render_frame_into(
        &map,
        &cam_occluded,
        &textures,
        &[s_hidden],
        &config,
        &mut scratch,
        &mut zbuf,
        &mut fb_a,
    )
    .unwrap();
    // Center column hits the pillar at (4, 5) with perp_wall_dist = 2.0 < sprite depth 5.0.
    assert!(zbuf[40] < 2.1);
    assert_eq!(
        fb_a.pixel_at(40, 30),
        (100, 100, 100, 255),
        "pillar must occlude hidden blue sprite"
    );

    // Partial column occlusion: place a wide sprite right at the corner edge of pillar (4, 5)
    // at y = 5.0 with scale_u = 2.0 from camera at (2.0, 5.0).
    // Columns with y >= 5.0 hit pillar at x=4 (z ~ 2.0), columns with y < 5.0 pass beside it (z ~ 7.0).
    // So the right half of the sprite (y >= 5.0) is occluded by the pillar while the left half (y < 5.0) is visible!
    let cam_edge = Camera2D::camera_new(2.0, 5.0, 1.0, 0.0, 0.0, 0.66).unwrap();
    let s_partial = Sprite::sprite_with_scale(6.0, 5.0, 3, 2.0, 1.0, 0.0).unwrap();
    render_frame_into(
        &map,
        &cam_edge,
        &textures,
        &[s_partial],
        &config,
        &mut scratch,
        &mut zbuf,
        &mut fb_a,
    )
    .unwrap();
    // Left of center (unoccluded) should show blue sprite; right of center (occluded) shows wall.
    assert_eq!(fb_a.pixel_at(34, 30), (0, 0, 255, 255));
    assert_ne!(fb_a.pixel_at(44, 30), (0, 0, 255, 255));
}

#[test]
fn deterministic_scene_digest_and_math3d_interop() {
    let map = make_box_map();
    let wall1 =
        TextureRgba::texture_checkerboard(16, 16, 4, [200, 80, 60, 255], [140, 50, 40, 255])
            .unwrap();
    let wall2 = TextureRgba::texture_solid(16, 16, [60, 140, 210, 255]).unwrap();
    let sprite_tex =
        TextureRgba::texture_checkerboard(16, 16, 8, [255, 220, 0, 255], [0, 0, 0, 0]).unwrap();
    let textures = [wall1, wall2, sprite_tex];

    let cam = Camera2D::camera_from_yaw_fov(1.5, 1.5, 0.45, trig::PI / 3.0)
        .unwrap()
        .camera_rotate(0.1)
        .unwrap();
    let fwd = cam.camera_forward_vec3().unwrap();
    assert!((fwd.length().unwrap() - 1.0).abs() < 1e-12);
    let world_ray = cam.camera_world_ray().unwrap();
    assert!((world_ray.origin.x - cam.pos_x).abs() < 1e-12);

    let sprites = [Sprite::sprite_new(3.5, 2.5, 2).unwrap()];
    let config = RenderConfig::render_config_default();
    let mut fb1 = PixelContainer::new(120, 90);
    let mut fb2 = PixelContainer::new(120, 90);
    let mut zbuf = vec![0.0; 120];
    let mut scratch = SpriteScratch::scratch_with_capacity(4).unwrap();

    let stats1 = render_frame_into(
        &map,
        &cam,
        &textures,
        &sprites,
        &config,
        &mut scratch,
        &mut zbuf,
        &mut fb1,
    )
    .unwrap();
    let stats2 = render_frame_into(
        &map,
        &cam,
        &textures,
        &sprites,
        &config,
        &mut scratch,
        &mut zbuf,
        &mut fb2,
    )
    .unwrap();

    assert_eq!(stats1, stats2);
    assert_eq!(stats1.columns_cast, 120);
    assert_eq!(stats1.wall_hits, 120);
    assert_eq!(stats1.sprites_projected, 1);
    assert_eq!(sha256_hex(&fb1.data), sha256_hex(&fb2.data));
}

#[test]
fn security_guards_cwe20_cwe125_cwe369_cwe400_cwe835() {
    // CWE-400: bounded grid, texture, framebuffer, sprite scratch
    assert_eq!(
        GridMap::grid_new(0, 4, vec![]),
        Err(RaycastError::InvalidGrid)
    );
    assert_eq!(
        GridMap::grid_new(MAX_MAP_DIM + 1, 1, vec![0; MAX_MAP_DIM + 1]),
        Err(RaycastError::InvalidGrid)
    );
    assert_eq!(
        GridMap::grid_new(2, 2, vec![0; 3]),
        Err(RaycastError::InvalidGrid)
    );
    assert_eq!(
        TextureRgba::texture_solid(0, 8, [0, 0, 0, 0]),
        Err(RaycastError::InvalidTexture)
    );
    assert_eq!(
        TextureRgba::texture_solid(MAX_TEXTURE_DIM + 1, 1, [0, 0, 0, 0]),
        Err(RaycastError::InvalidTexture)
    );
    assert_eq!(
        SpriteScratch::scratch_with_capacity(MAX_SPRITES + 1).unwrap_err(),
        RaycastError::TooManySprites {
            count: MAX_SPRITES + 1,
            max: MAX_SPRITES
        }
    );

    // CWE-20: non-finite & out-of-bounds camera / ray / sprite inputs
    assert_eq!(
        Camera2D::camera_new(f64::NAN, 1.0, 1.0, 0.0, 0.0, 0.66),
        Err(RaycastError::NonFiniteInput)
    );
    assert_eq!(
        Camera2D::camera_new(1.0, f64::INFINITY, 1.0, 0.0, 0.0, 0.66),
        Err(RaycastError::NonFiniteInput)
    );
    assert_eq!(
        Camera2D::camera_new(MAX_COORD + 1.0, 1.0, 1.0, 0.0, 0.0, 0.66),
        Err(RaycastError::ValueOutOfBounds)
    );
    assert_eq!(
        Camera2D::camera_from_yaw_fov(1.0, 1.0, 0.0, 0.0),
        Err(RaycastError::ValueOutOfBounds)
    );
    assert_eq!(
        Camera2D::camera_from_yaw_fov(1.0, 1.0, 0.0, trig::PI),
        Err(RaycastError::ValueOutOfBounds)
    );
    assert_eq!(
        Sprite::sprite_new(f64::NEG_INFINITY, 1.0, 0),
        Err(RaycastError::NonFiniteInput)
    );
    assert_eq!(
        Sprite::sprite_with_scale(1.0, 1.0, 0, 0.0, 1.0, 0.0),
        Err(RaycastError::ValueOutOfBounds)
    );

    // CWE-369: degenerate camera & near-zero ray components
    assert_eq!(
        Camera2D::camera_new(1.0, 1.0, 0.0, 0.0, 0.0, 0.66),
        Err(RaycastError::DegenerateCamera)
    );
    // Collinear dir and plane (zero 2x2 determinant)
    assert_eq!(
        Camera2D::camera_new(1.0, 1.0, 1.0, 0.0, 0.5, 0.0),
        Err(RaycastError::DegenerateCamera)
    );

    let open_map = GridMap::grid_new(32, 32, vec![0; 32 * 32]).unwrap();
    // Near-zero ray_dir_x and ray_dir_y -> DegenerateRay
    assert_eq!(
        cast_dda_ray(&open_map, 16.5, 16.5, EPSILON * 0.1, 0.0),
        Err(RaycastError::DegenerateRay)
    );
    // Axis-aligned rays with exact 0.0 in one component must NOT divide by zero
    let miss_x = cast_dda_ray(&open_map, 16.5, 16.5, 1.0, 0.0).unwrap();
    assert!(!miss_x.hit);
    assert!(miss_x.steps_taken <= open_map.grid_width() + open_map.grid_height());

    let miss_y = cast_dda_ray(&open_map, 16.5, 16.5, 0.0, -1.0).unwrap();
    assert!(!miss_y.hit);
    assert!(miss_y.steps_taken <= open_map.grid_width() + open_map.grid_height());

    // Camera standing inside a wall cell clamps perp_wall_dist >= MIN_PERP_DIST
    let solid_map = GridMap::grid_new(4, 4, vec![1; 16]).unwrap();
    let inside_hit = cast_dda_ray(&solid_map, 1.5, 1.5, 1.0, 0.0).unwrap();
    assert!(inside_hit.hit);
    assert!(inside_hit.perp_wall_dist >= MIN_PERP_DIST);

    // CWE-125/787: Z-buffer / scratch / framebuffer / material validation
    let cam = Camera2D::camera_new(1.5, 1.5, 1.0, 0.0, 0.0, 0.66).unwrap();
    let tex = TextureRgba::texture_solid(4, 4, [255, 255, 255, 255]).unwrap();
    let cfg = RenderConfig::render_config_default();
    let mut fb = PixelContainer::new(16, 16);
    let mut zbuf_small = vec![0.0; 15];
    let mut scratch = SpriteScratch::scratch_with_capacity(0).unwrap();
    assert_eq!(
        render_frame_into(
            &solid_map,
            &cam,
            std::slice::from_ref(&tex),
            &[],
            &cfg,
            &mut scratch,
            &mut zbuf_small,
            &mut fb
        ),
        Err(RaycastError::ZBufferTooSmall {
            required: 16,
            actual: 15
        })
    );

    let mut zbuf = vec![0.0; 16];
    let sp = [Sprite::sprite_new(2.0, 1.5, 0).unwrap()];
    assert_eq!(
        render_frame_into(
            &solid_map,
            &cam,
            std::slice::from_ref(&tex),
            &sp,
            &cfg,
            &mut scratch,
            &mut zbuf,
            &mut fb
        ),
        Err(RaycastError::ScratchTooSmall {
            required: 1,
            actual: 0
        })
    );

    // InvalidMaterialId for missing wall texture and missing sprite texture
    assert_eq!(
        render_frame_into(
            &solid_map,
            &cam,
            &[],
            &[],
            &cfg,
            &mut scratch,
            &mut zbuf,
            &mut fb
        ),
        Err(RaycastError::InvalidMaterialId { id: 1 })
    );
    let sp_bad_tex = [Sprite::sprite_new(2.0, 1.5, 3).unwrap()];
    let mut scratch_1 = SpriteScratch::scratch_with_capacity(1).unwrap();
    assert_eq!(
        render_frame_into(
            &solid_map,
            &cam,
            std::slice::from_ref(&tex),
            &sp_bad_tex,
            &cfg,
            &mut scratch_1,
            &mut zbuf,
            &mut fb
        ),
        Err(RaycastError::InvalidMaterialId { id: 3 })
    );

    // Framebuffer dimension cap
    let mut oversized_fb = PixelContainer {
        width: MAX_FRAME_WIDTH + 1,
        height: 1,
        data: vec![0; (MAX_FRAME_WIDTH as usize + 1) * 4],
    };
    let mut big_zbuf = vec![0.0; MAX_FRAME_WIDTH as usize + 1];
    assert_eq!(
        render_frame_into(
            &open_map,
            &cam,
            &[],
            &[],
            &cfg,
            &mut scratch,
            &mut big_zbuf,
            &mut oversized_fb
        ),
        Err(RaycastError::InvalidFramebuffer)
    );
}

#[test]
fn regression_far_offscreen_sprite_no_overflow_and_odd_horizon_no_stale_pixels() {
    // 1. Far-offscreen sprite with extreme transform_x / transform_y ratio must not
    // overflow/panic in debug or wrap in release (CWE-190).
    let open_map = GridMap::grid_new(16, 16, vec![0; 16 * 16]).unwrap();
    let cam_narrow = Camera2D::camera_new(0.0, 0.0, 1.0, 0.0, 0.0, 1e-6).unwrap();
    let sp_far_side = Sprite::sprite_new(1e-5, 1e6, 0).unwrap();
    let tex = TextureRgba::texture_solid(16, 16, [200, 100, 50, 255]).unwrap();
    let cfg = RenderConfig::render_config_default();
    let mut scratch = SpriteScratch::scratch_with_capacity(4).unwrap();
    let mut zbuf = vec![0.0; 256];
    let mut fb = PixelContainer::new(256, 64);

    let stats = render_frame_into(
        &open_map,
        &cam_narrow,
        std::slice::from_ref(&tex),
        &[sp_far_side],
        &cfg,
        &mut scratch,
        &mut zbuf,
        &mut fb,
    )
    .expect("far-offscreen sprite must not overflow");
    assert_eq!(stats.columns_cast, 256);

    // 2. Odd framebuffer height with textured floor on an open map must overwrite
    // the center horizon row (y = 61 / 2 = 30) with zero stale sentinel pixels.
    let mut fb_odd = PixelContainer::from_data(32, 61, [1, 2, 3, 255].repeat(32 * 61));
    let mut zbuf_odd = vec![0.0; 32];
    let floor_tex = TextureRgba::texture_solid(8, 8, [40, 120, 40, 255]).unwrap();
    let ceil_tex = TextureRgba::texture_solid(8, 8, [40, 40, 120, 255]).unwrap();
    let cfg_textured = RenderConfig::render_config_with_planes(
        PlaneSurface::Textured { texture_index: 1 },
        PlaneSurface::Textured { texture_index: 0 },
        128,
    );
    let cam_open = Camera2D::camera_from_yaw_fov(8.5, 8.5, 0.0, trig::PI / 3.0).unwrap();
    render_frame_into(
        &open_map,
        &cam_open,
        &[floor_tex, ceil_tex],
        &[],
        &cfg_textured,
        &mut scratch,
        &mut zbuf_odd,
        &mut fb_odd,
    )
    .unwrap();
    for x in 0..32 {
        assert_eq!(
            fb_odd.pixel_at(x, 30),
            (40, 120, 40, 255),
            "center horizon row must not retain stale sentinel pixel at x={x}"
        );
    }

    // 3. Mutated NaN camera fields map to NonFiniteInput in camera_forward_vec3 / camera_world_ray.
    let mut bad_dir = cam_open;
    bad_dir.dir_x = f64::NAN;
    assert_eq!(
        bad_dir.camera_forward_vec3(),
        Err(RaycastError::NonFiniteInput)
    );
    let mut bad_pos = cam_open;
    bad_pos.pos_y = f64::INFINITY;
    assert_eq!(
        bad_pos.camera_world_ray(),
        Err(RaycastError::NonFiniteInput)
    );
}
