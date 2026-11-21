//! Hostile-input security regression and fuzz tests for `raycast-dda` (`GAME31`).
//!
//! Covers:
//! - `NaN`, non-canonical `NaN`, `±Inf`, `-0.0`, subnormals (`f64::from_bits(1)`), `±f64::MAX`, and `> MAX_COORD`
//! - Hand-mutated `Camera2D` fields in `camera_rotate` / `camera_move_by` and finite addition overflow
//! - Atomic byte-for-byte preservation of `framebuffer.data`, `zbuffer`, and `SpriteScratch` across all `Err` paths
//!   (including mid-wall `InvalidMaterialId`)
//! - Grazing open-map rays and `MAX_MAP_DIM` termination bounds
//! - Seeded 2,000-case hostile fuzz campaign

#![forbid(unsafe_code)]

use pixel_container::PixelContainer;
use raycast_dda::{
    cast_dda_ray, render_frame_into, Camera2D, GridMap, PlaneSurface, RaycastError, RenderConfig,
    Sprite, SpriteScratch, TextureRgba, EPSILON, MAX_COORD, MAX_FRAME_WIDTH, MAX_MAP_DIM,
    MAX_SPRITES, MAX_TEXTURE_DIM,
};

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_hostile_f64(&mut self) -> f64 {
        match self.next_u64() % 16 {
            0 => f64::NAN,
            1 => f64::from_bits(0x7ff8_0000_0000_0001),
            2 => f64::INFINITY,
            3 => f64::NEG_INFINITY,
            4 => -0.0,
            5 => 0.0,
            6 => f64::from_bits(1),
            7 => f64::MIN_POSITIVE * 0.5,
            8 => f64::MAX,
            9 => -f64::MAX,
            10 => MAX_COORD + 1.0,
            11 => -(MAX_COORD + 1.0),
            12 => EPSILON * 0.5,
            13 => {
                let u = ((self.next_u64() >> 11) as f64) / ((1u64 << 53) as f64);
                (u * 2.0 - 1.0) * 16.0
            }
            14 => {
                let u = ((self.next_u64() >> 11) as f64) / ((1u64 << 53) as f64);
                u * 8.0
            }
            _ => f64::from_bits(self.next_u64()),
        }
    }
}

fn make_sample_room() -> GridMap {
    #[rustfmt::skip]
    let cells = vec![
        1, 1, 1, 1, 1,
        1, 0, 0, 0, 1,
        1, 0, 0, 0, 1,
        1, 0, 0, 0, 1,
        1, 1, 1, 1, 1,
    ];
    GridMap::grid_new(5, 5, cells).unwrap()
}

#[test]
fn hostile_scalars_and_camera_field_mutations() {
    let tiny_subnormal = f64::from_bits(1);
    let half_min = f64::MIN_POSITIVE * 0.5;
    let hostile_vals = [
        f64::NAN,
        f64::from_bits(0x7ff8_0000_0000_0001),
        f64::INFINITY,
        f64::NEG_INFINITY,
        -0.0,
        tiny_subnormal,
        -tiny_subnormal,
        half_min,
        f64::MAX,
        -f64::MAX,
        MAX_COORD + 1.0,
        -(MAX_COORD + 1.0),
    ];

    let grid = make_sample_room();
    for &v in &hostile_vals {
        let _ = Camera2D::camera_new(v, 2.5, 1.0, 0.0, 0.0, 0.66);
        let _ = Camera2D::camera_new(2.5, 2.5, v, 0.0, 0.0, 0.66);
        let _ = Camera2D::camera_from_yaw_fov(2.5, 2.5, v, 1.0);
        let _ = Camera2D::camera_from_yaw_fov(2.5, 2.5, 0.0, v);
        let _ = Sprite::sprite_with_scale(v, 2.5, 0, 1.0, 1.0, 0.0);
        let _ = Sprite::sprite_with_scale(2.5, 2.5, 0, v, 1.0, 0.0);
        let _ = cast_dda_ray(&grid, v, 2.5, 1.0, 0.0);
        let _ = cast_dda_ray(&grid, 2.5, 2.5, v, 0.0);
    }

    // Hand-mutated Camera2D fields must be caught by camera_rotate and camera_move_by.
    let mut mutated = Camera2D::camera_new(2.5, 2.5, 1.0, 0.0, 0.0, 0.66).unwrap();
    mutated.pos_x = 1e308;
    assert_eq!(
        mutated.camera_rotate(0.1),
        Err(RaycastError::ValueOutOfBounds)
    );
    assert_eq!(
        mutated.camera_move_by(1e308, 0.0),
        Err(RaycastError::ValueOutOfBounds)
    );

    let mut mutated_nan = Camera2D::camera_new(2.5, 2.5, 1.0, 0.0, 0.0, 0.66).unwrap();
    mutated_nan.dir_x = f64::NAN;
    assert_eq!(
        mutated_nan.camera_rotate(0.1),
        Err(RaycastError::NonFiniteInput)
    );
    assert_eq!(
        mutated_nan.camera_move_by(1.0, 0.0),
        Err(RaycastError::NonFiniteInput)
    );

    let mut mutated_collinear = Camera2D::camera_new(2.5, 2.5, 1.0, 0.0, 0.0, 0.66).unwrap();
    mutated_collinear.plane_y = 0.0;
    mutated_collinear.plane_x = 1.0;
    assert_eq!(
        mutated_collinear.camera_rotate(0.1),
        Err(RaycastError::DegenerateCamera)
    );
    assert_eq!(
        mutated_collinear.camera_move_by(0.1, 0.1),
        Err(RaycastError::DegenerateCamera)
    );

    // Near-boundary coordinate addition overflow maps to ValueOutOfBounds.
    let edge_cam = Camera2D::camera_new(MAX_COORD, 0.0, 1.0, 0.0, 0.0, 0.66).unwrap();
    assert_eq!(
        edge_cam.camera_move_by(1.0, 0.0),
        Err(RaycastError::ValueOutOfBounds)
    );
}

#[test]
fn hostile_atomic_framebuffer_zbuffer_and_scratch_preservation_on_err() {
    let tex = TextureRgba::texture_solid(8, 8, [10, 20, 30, 255]).unwrap();
    let textures = [tex];
    let cam = Camera2D::camera_new(2.5, 2.5, 1.0, 0.0, 0.0, 0.66).unwrap();

    // Helper asserting byte-for-byte preservation of framebuffer, zbuffer, and scratch.
    let assert_atomic_err = |grid: &GridMap,
                             cam: &Camera2D,
                             sprites: &[Sprite],
                             config: &RenderConfig,
                             expected_err: RaycastError| {
        let mut fb = PixelContainer::new(32, 32);
        for (i, b) in fb.data.iter_mut().enumerate() {
            *b = ((i * 37 + 13) & 0xFF) as u8;
        }
        let fb_before = fb.data.clone();

        let mut zbuf: Vec<f64> = (0..32).map(|i| 123.456 + i as f64).collect();
        let zbuf_before: Vec<u64> = zbuf.iter().map(|v| v.to_bits()).collect();

        // Populate scratch once via a valid render so scratch_len() > 0 before the failing call.
        let valid_grid = make_sample_room();
        let valid_cam = Camera2D::camera_new(2.5, 2.5, 1.0, 0.0, 0.0, 0.66).unwrap();
        let valid_sp = [Sprite::sprite_new(3.5, 2.5, 0).unwrap()];
        let mut scratch = SpriteScratch::scratch_with_capacity(4).unwrap();
        let mut tmp_fb = PixelContainer::new(16, 16);
        let mut tmp_z = vec![0.0; 16];
        render_frame_into(
            &valid_grid,
            &valid_cam,
            &textures,
            &valid_sp,
            &RenderConfig::render_config_default(),
            &mut scratch,
            &mut tmp_z,
            &mut tmp_fb,
        )
        .unwrap();
        let scratch_len_before = scratch.scratch_len();
        assert_eq!(scratch_len_before, 1);

        let err = render_frame_into(
            grid,
            cam,
            &textures,
            sprites,
            config,
            &mut scratch,
            &mut zbuf,
            &mut fb,
        )
        .expect_err("expected atomic render error");
        assert_eq!(err, expected_err);
        assert_eq!(
            fb.data, fb_before,
            "framebuffer.data was mutated on Err({expected_err:?})"
        );
        let zbuf_after: Vec<u64> = zbuf.iter().map(|v| v.to_bits()).collect();
        assert_eq!(
            zbuf_after, zbuf_before,
            "zbuffer was mutated on Err({expected_err:?})"
        );
        assert_eq!(
            scratch.scratch_len(),
            scratch_len_before,
            "SpriteScratch was mutated on Err({expected_err:?})"
        );
    };

    // 1. Mid-wall InvalidMaterialId: tile 2 at (4, 2) only hit on center columns while columns 0..x hit tile 1!
    #[rustfmt::skip]
    let mid_wall_cells = vec![
        1, 1, 1, 1, 1,
        1, 0, 0, 0, 1,
        1, 0, 0, 0, 2,
        1, 0, 0, 0, 1,
        1, 1, 1, 1, 1,
    ];
    let mid_wall_grid = GridMap::grid_new(5, 5, mid_wall_cells).unwrap();
    assert_atomic_err(
        &mid_wall_grid,
        &cam,
        &[],
        &RenderConfig::render_config_default(),
        RaycastError::InvalidMaterialId { id: 2 },
    );

    // 2. Invalid floor/ceiling texture index.
    let valid_grid = make_sample_room();
    let bad_floor_cfg = RenderConfig::render_config_with_planes(
        PlaneSurface::Solid([0, 0, 0, 255]),
        PlaneSurface::Textured { texture_index: 9 },
        128,
    );
    assert_atomic_err(
        &valid_grid,
        &cam,
        &[],
        &bad_floor_cfg,
        RaycastError::InvalidMaterialId { id: 9 },
    );

    // 3. Invalid sprite texture index and NaN hand-mutated sprite fields.
    let bad_sprite_tex = [Sprite {
        x: 3.0,
        y: 2.5,
        texture_index: 4,
        scale_u: 1.0,
        scale_v: 1.0,
        z_offset: 0.0,
    }];
    assert_atomic_err(
        &valid_grid,
        &cam,
        &bad_sprite_tex,
        &RenderConfig::render_config_default(),
        RaycastError::InvalidMaterialId { id: 4 },
    );

    let bad_sprite_nan = [Sprite {
        x: f64::NAN,
        y: 2.5,
        texture_index: 0,
        scale_u: 1.0,
        scale_v: 1.0,
        z_offset: 0.0,
    }];
    assert_atomic_err(
        &valid_grid,
        &cam,
        &bad_sprite_nan,
        &RenderConfig::render_config_default(),
        RaycastError::NonFiniteInput,
    );

    // 4. Camera whose edge column ray overflows MAX_COORD.
    let big_ray_cam = Camera2D::camera_new(2.5, 2.5, 900_000.0, 0.0, 900_000.0, 1.0).unwrap();
    assert_atomic_err(
        &valid_grid,
        &big_ray_cam,
        &[],
        &RenderConfig::render_config_default(),
        RaycastError::ValueOutOfBounds,
    );
}

#[test]
fn hostile_grazing_open_map_rays_and_dimension_limits() {
    // Open map with zero walls: rays at grazing angles must terminate within width + height steps.
    let open = GridMap::grid_new(64, 64, vec![0; 64 * 64]).unwrap();
    let grazing_dirs = [
        (1.0, 1e-12),
        (1.0, -1e-12),
        (1e-12, 1.0),
        (-1e-12, -1.0),
        (1.0, 0.0),
        (-1.0, -0.0),
    ];
    for &(dx, dy) in &grazing_dirs {
        let hit = cast_dda_ray(&open, 32.0, 32.0, dx, dy).unwrap();
        assert!(!hit.hit);
        assert!(hit.steps_taken <= 128);
    }

    assert_eq!(
        GridMap::grid_new(0, 4, vec![]),
        Err(RaycastError::InvalidGrid)
    );
    assert_eq!(
        GridMap::grid_new(MAX_MAP_DIM + 1, 1, vec![0; MAX_MAP_DIM + 1]),
        Err(RaycastError::InvalidGrid)
    );
    assert_eq!(
        TextureRgba::texture_solid(MAX_TEXTURE_DIM + 1, 1, [0; 4]),
        Err(RaycastError::InvalidTexture)
    );
    assert!(matches!(
        SpriteScratch::scratch_with_capacity(MAX_SPRITES + 1),
        Err(RaycastError::TooManySprites {
            count,
            max
        }) if count == MAX_SPRITES + 1 && max == MAX_SPRITES
    ));

    let cam = Camera2D::camera_new(2.5, 2.5, 1.0, 0.0, 0.0, 0.66).unwrap();
    let mut bad_fb = PixelContainer::new(MAX_FRAME_WIDTH + 1, 1);
    let mut zbuf = vec![0.0; (MAX_FRAME_WIDTH + 1) as usize];
    let mut scratch = SpriteScratch::scratch_with_capacity(0).unwrap();
    assert_eq!(
        render_frame_into(
            &open,
            &cam,
            &[],
            &[],
            &RenderConfig::render_config_default(),
            &mut scratch,
            &mut zbuf,
            &mut bad_fb,
        ),
        Err(RaycastError::InvalidFramebuffer)
    );
}

#[test]
fn hostile_fuzz_2000_cases_zero_panics() {
    let mut rng = SplitMix64(0x1337_5EED_3131_2026);
    let tex0 = TextureRgba::texture_solid(4, 4, [200, 100, 50, 255]).unwrap();
    let textures = [tex0];
    let grid = make_sample_room();

    for i in 0..2_000 {
        let ox = rng.next_hostile_f64();
        let oy = rng.next_hostile_f64();
        let dx = rng.next_hostile_f64();
        let dy = rng.next_hostile_f64();
        let _ = cast_dda_ray(&grid, ox, oy, dx, dy);

        let px = rng.next_hostile_f64();
        let py = rng.next_hostile_f64();
        let _ = Camera2D::camera_new(ox, oy, dx, dy, px, py);
        let _ = Camera2D::camera_from_yaw_fov(ox, oy, dx, dy);

        let mut cam = Camera2D {
            pos_x: ox,
            pos_y: oy,
            dir_x: dx,
            dir_y: dy,
            plane_x: px,
            plane_y: py,
        };
        let _ = cam.camera_rotate(px);
        let _ = cam.camera_move_by(px, py);
        let _ = cam.camera_forward_vec3();
        let _ = cam.camera_world_ray();

        let sp = Sprite {
            x: ox,
            y: oy,
            texture_index: (rng.next_u64() as usize) % 4,
            scale_u: dx,
            scale_v: dy,
            z_offset: px,
        };
        let cfg = RenderConfig::render_config_with_planes(
            if i % 3 == 0 {
                PlaneSurface::Textured {
                    texture_index: (rng.next_u64() as usize) % 4,
                }
            } else {
                PlaneSurface::Solid([1, 2, 3, 255])
            },
            PlaneSurface::Solid([4, 5, 6, 255]),
            (rng.next_u64() & 0xFF) as u8,
        );
        let mut fb = PixelContainer::new(8, 8);
        let mut zbuf = vec![0.0; 8];
        let mut scratch = SpriteScratch::scratch_with_capacity(2).unwrap();
        if i % 5 == 0 {
            cam.pos_x = 2.5;
            cam.pos_y = 2.5;
            cam.dir_x = 1.0;
            cam.dir_y = 0.0;
            cam.plane_x = 0.0;
            cam.plane_y = 0.66;
        }
        let _ = render_frame_into(
            &grid,
            &cam,
            &textures,
            &[sp],
            &cfg,
            &mut scratch,
            &mut zbuf,
            &mut fb,
        );
    }
}
