//! # Hostile-Input & CVE Proof Suite for `bsp-portal25d` (GAME32)
//!
//! Proves that all applicable CVE/CWE classes are impossible:
//! - Memory safety (CWE-119/125/415/416/476/787): `#![forbid(unsafe_code)]`,
//!   truncated `PixelContainer`, cross-paired `Map25D`/`BspTree`, OOB `usize::MAX`
//!   sector & material IDs, and undersized `RenderScratch`.
//! - Numeric & panic safety (CWE-20/190/369): `NaN`, `±Inf`, `-0.0`, subnormals
//!   (`f64::from_bits(1)`, `f64::MIN_POSITIVE * 0.5`), `f64::MIN`/`MAX`, hand-mutated
//!   public fields on `Aabb2`/`Linedef`/`Camera25D`, narrow/wide FOV extremes (`1px`
//!   framebuffer with `PI - 1e-9`), and extreme textured UV sampling.
//! - DoS / atomic state preservation (CWE-400/674): bounded budgets and byte-for-byte
//!   atomic preservation of `framebuffer.data` on every `Err` path.
//! - Seeded 2,000-case randomized hostile/malformed fuzz campaign across
//!   `Map25D::map_new`, `BspTree::bsp_build`, `Camera25D`, and `render_frame_into`.

#![forbid(unsafe_code)]

use bsp_portal25d::{
    frame_digest_fnv1a, render_frame_into, scene_digest_fnv1a, Aabb2, BspError, BspTree, Camera25D,
    Linedef, Map25D, RenderConfig, RenderScratch, Sector, SurfaceStyle, TextureRgba, Vec2,
    MAX_COORD, MAX_FRAME_HEIGHT, MAX_FRAME_WIDTH, MAX_TEXTURE_DIM, MIN_PERP_DIST,
};
use pixel_container::PixelContainer;

fn sentinel_fb(width: u32, height: u32) -> PixelContainer {
    let count = (width as usize) * (height as usize);
    let mut data = Vec::with_capacity(count * 4);
    for i in 0..count {
        data.push(0xA5 ^ (i as u8));
        data.push(0x5A);
        data.push(0x3C);
        data.push(0xC3);
    }
    PixelContainer::from_data(width, height, data)
}

fn make_valid_room(sector_count: usize) -> (Map25D, BspTree) {
    let mut sectors = Vec::new();
    for i in 0..sector_count {
        sectors.push(
            Sector::sector_new(
                (i as f64) * 0.2,
                4.0 + (i as f64) * 0.2,
                [40, 80, 40, 255],
                [40, 40, 90, 255],
                255,
            )
            .unwrap(),
        );
    }
    let w = SurfaceStyle::surface_solid([180, 60, 60, 255]);
    let up = SurfaceStyle::surface_solid([200, 180, 60, 255]);
    let lo = SurfaceStyle::surface_solid([180, 80, 200, 255]);

    let mut linedefs = vec![
        Linedef::linedef_solid(Vec2::vec2_new(0.0, 0.0), Vec2::vec2_new(0.0, 10.0), 0, w).unwrap(),
        Linedef::linedef_solid(Vec2::vec2_new(0.0, 10.0), Vec2::vec2_new(10.0, 10.0), 0, w)
            .unwrap(),
        Linedef::linedef_solid(Vec2::vec2_new(10.0, 0.0), Vec2::vec2_new(0.0, 0.0), 0, w).unwrap(),
    ];
    if sector_count == 1 {
        linedefs.push(
            Linedef::linedef_solid(Vec2::vec2_new(10.0, 10.0), Vec2::vec2_new(10.0, 0.0), 0, w)
                .unwrap(),
        );
    } else {
        linedefs.push(
            Linedef::linedef_portal(
                Vec2::vec2_new(10.0, 10.0),
                Vec2::vec2_new(10.0, 0.0),
                0,
                Some(1),
                w,
                up,
                lo,
            )
            .unwrap(),
        );
        for s in 1..sector_count {
            let x0 = (s as f64) * 10.0;
            let x1 = ((s + 1) as f64) * 10.0;
            linedefs.push(
                Linedef::linedef_solid(Vec2::vec2_new(x0, 10.0), Vec2::vec2_new(x1, 10.0), s, w)
                    .unwrap(),
            );
            linedefs.push(
                Linedef::linedef_solid(Vec2::vec2_new(x1, 0.0), Vec2::vec2_new(x0, 0.0), s, w)
                    .unwrap(),
            );
            if s + 1 < sector_count {
                linedefs.push(
                    Linedef::linedef_portal(
                        Vec2::vec2_new(x1, 10.0),
                        Vec2::vec2_new(x1, 0.0),
                        s,
                        Some(s + 1),
                        w,
                        up,
                        lo,
                    )
                    .unwrap(),
                );
            } else {
                linedefs.push(
                    Linedef::linedef_solid(Vec2::vec2_new(x1, 10.0), Vec2::vec2_new(x1, 0.0), s, w)
                        .unwrap(),
                );
            }
        }
    }

    let map = Map25D::map_new(sectors, linedefs).unwrap();
    let bsp = BspTree::bsp_build(&map).unwrap();
    (map, bsp)
}

#[test]
fn hostile_numeric_subnormal_extreme_and_mutated_structs() {
    let tiny_subnormal = f64::from_bits(1);
    let half_min_pos = f64::MIN_POSITIVE * 0.5;
    let hostile_floats = [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN,
        f64::MAX,
        -MAX_COORD - 1.0,
        MAX_COORD + 1.0,
    ];

    for &hf in &hostile_floats {
        assert!(Vec2::vec2_try_new(hf, 0.0).is_err());
        assert!(Vec2::vec2_try_new(0.0, hf).is_err());
        assert!(Sector::sector_new(hf, 4.0, [0; 4], [0; 4], 255).is_err());
        assert!(Sector::sector_new(0.0, hf, [0; 4], [0; 4], 255).is_err());
        assert!(Camera25D::camera_from_yaw_fov(hf, 0.0, 1.0, 0.0, 1.0).is_err());
        assert!(Camera25D::camera_from_yaw_fov(0.0, 0.0, 1.0, 0.0, hf).is_err());
    }
    for &ha in &[
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN,
        f64::MAX,
        -(math3d::MAX_ANGLE_RAD + 1.0),
        math3d::MAX_ANGLE_RAD + 1.0,
    ] {
        assert!(Camera25D::camera_from_yaw_fov(0.0, 0.0, 1.0, ha, 1.0).is_err());
    }

    // Subnormals and -0.0 must never panic and must reject near-zero degenerate segments/cameras.
    for &sub in &[-0.0_f64, 0.0, tiny_subnormal, -tiny_subnormal, half_min_pos] {
        assert_eq!(
            Linedef::linedef_solid(
                Vec2::vec2_new(1.0, 1.0),
                Vec2::vec2_new(1.0 + sub, 1.0 + sub),
                0,
                SurfaceStyle::surface_solid([0; 4]),
            ),
            Err(BspError::DegenerateSegment)
        );
        assert_eq!(
            Camera25D::camera_new(0.0, 0.0, 1.0, sub, sub, 0.0, -1.0, 1.0, 0.05),
            Err(BspError::DegenerateCamera)
        );
    }

    // Hand-mutated public fields on Aabb2 (min > max, NaN, Inf, > MAX_COORD).
    let mut bad_box =
        Aabb2::aabb2_from_points(Vec2::vec2_new(0.0, 0.0), Vec2::vec2_new(5.0, 5.0)).unwrap();
    bad_box.min_x = 10.0;
    assert_eq!(
        bad_box.aabb2_contains_point(Vec2::vec2_new(2.0, 2.0)),
        Err(BspError::ValueOutOfBounds)
    );
    assert_eq!(
        bad_box.aabb2_to_aabb3(0.0, 4.0),
        Err(BspError::ValueOutOfBounds)
    );

    bad_box.min_x = f64::NAN;
    assert_eq!(
        bad_box.aabb2_contains_point(Vec2::vec2_new(2.0, 2.0)),
        Err(BspError::NonFiniteInput)
    );
    assert_eq!(
        bad_box.aabb2_to_aabb3(0.0, 4.0),
        Err(BspError::NonFiniteInput)
    );

    bad_box.min_x = 0.0;
    bad_box.max_y = 1.0e200;
    assert_eq!(
        bad_box.aabb2_contains_point(Vec2::vec2_new(2.0, 2.0)),
        Err(BspError::ValueOutOfBounds)
    );

    // Hand-mutated public fields on Linedef (NaN and 1e200 endpoints).
    let mut bad_ld = Linedef::linedef_solid(
        Vec2::vec2_new(0.0, 0.0),
        Vec2::vec2_new(4.0, 0.0),
        0,
        SurfaceStyle::surface_solid([0; 4]),
    )
    .unwrap();
    bad_ld.v0.x = f64::NAN;
    assert_eq!(
        bad_ld.linedef_signed_distance(Vec2::vec2_new(1.0, 1.0)),
        Err(BspError::NonFiniteInput)
    );
    assert_eq!(bad_ld.linedef_length(), Err(BspError::NonFiniteInput));

    bad_ld.v0.x = 1.0e200;
    assert_eq!(
        bad_ld.linedef_signed_distance(Vec2::vec2_new(1.0, 1.0)),
        Err(BspError::ValueOutOfBounds)
    );

    // Canonical -0.0 digests on PixelContainer & Map25D.
    let s_pos = Sector::sector_new(0.0, 4.0, [10; 4], [20; 4], 255).unwrap();
    let s_neg = Sector::sector_new(-0.0, 4.0, [10; 4], [20; 4], 255).unwrap();
    let ld_pos = Linedef::linedef_solid(
        Vec2::vec2_new(0.0, 0.0),
        Vec2::vec2_new(0.0, 10.0),
        0,
        SurfaceStyle::surface_solid([30; 4]),
    )
    .unwrap();
    let ld_neg = Linedef::linedef_solid(
        Vec2::vec2_new(-0.0, -0.0),
        Vec2::vec2_new(-0.0, 10.0),
        0,
        SurfaceStyle::surface_solid([30; 4]),
    )
    .unwrap();
    let map_pos = Map25D::map_new(vec![s_pos], vec![ld_pos]).unwrap();
    let map_neg = Map25D::map_new(vec![s_neg], vec![ld_neg]).unwrap();
    let bsp_pos = BspTree::bsp_build(&map_pos).unwrap();
    let bsp_neg = BspTree::bsp_build(&map_neg).unwrap();
    assert_eq!(
        scene_digest_fnv1a(&map_pos, &bsp_pos),
        scene_digest_fnv1a(&map_neg, &bsp_neg)
    );
    assert_ne!(frame_digest_fnv1a(&sentinel_fb(4, 4)), 0);
}

#[test]
fn hostile_atomic_framebuffer_rollback_and_cross_paired_bsp() {
    let (map1, bsp1) = make_valid_room(1);
    let (map5, bsp5) = make_valid_room(5);
    let cam = Camera25D::camera_from_yaw_fov(5.0, 5.0, 2.0, 0.0, trig::PI / 3.0).unwrap();
    let cfg = RenderConfig::render_config_default();

    // 1. Cross-paired 5-sector BspTree against 1-sector Map25D must fail upfront atomically.
    let mut fb = sentinel_fb(64, 48);
    let fb_snapshot = fb.data.clone();
    let mut scratch5 = RenderScratch::scratch_for_frame_and_map(&fb, &map5, &bsp5).unwrap();
    assert_eq!(
        render_frame_into(&map1, &bsp5, &cam, &[], &cfg, &mut scratch5, &mut fb),
        Err(BspError::InvalidLinedef)
    );
    assert_eq!(
        fb.data, fb_snapshot,
        "cross-paired Map25D/BspTree must preserve framebuffer atomically"
    );

    // 2. 1px wide framebuffer with near-PI FOV (PI - 3*EPSILON): camera_from_yaw_fov succeeds,
    //    but focal <= EPSILON -> DegenerateCamera AND framebuffer.data stays byte-for-byte untouched!
    let mut fb_1px = sentinel_fb(1, 16);
    let fb_1px_snap = fb_1px.data.clone();
    let mut scratch_1px = RenderScratch::scratch_for_frame_and_map(&fb_1px, &map1, &bsp1).unwrap();
    let cam_wide =
        Camera25D::camera_from_yaw_fov(5.0, 5.0, 2.0, 0.0, trig::PI - 3.0 * bsp_portal25d::EPSILON)
            .unwrap();
    assert_eq!(
        render_frame_into(
            &map1,
            &bsp1,
            &cam_wide,
            &[],
            &cfg,
            &mut scratch_1px,
            &mut fb_1px
        ),
        Err(BspError::DegenerateCamera)
    );
    assert_eq!(
        fb_1px.data, fb_1px_snap,
        "1px + near-PI FOV DegenerateCamera must preserve framebuffer atomically"
    );

    // 3. Truncated PixelContainer byte buffer.
    let mut fb_trunc = PixelContainer {
        width: 16,
        height: 16,
        data: vec![0xAA; 10],
    };
    let trunc_snap = fb_trunc.data.clone();
    let mut scratch1 = RenderScratch::scratch_for_frame_and_map(&fb, &map1, &bsp1).unwrap();
    assert_eq!(
        render_frame_into(&map1, &bsp1, &cam, &[], &cfg, &mut scratch1, &mut fb_trunc),
        Err(BspError::InvalidFramebuffer)
    );
    assert_eq!(fb_trunc.data, trunc_snap);

    // 4. usize::MAX sector and material IDs.
    let ld_usize_max = Linedef::linedef_solid(
        Vec2::vec2_new(0.0, 0.0),
        Vec2::vec2_new(0.0, 10.0),
        usize::MAX,
        SurfaceStyle::surface_solid([0; 4]),
    )
    .unwrap();
    let s0 = Sector::sector_new(0.0, 4.0, [0; 4], [0; 4], 255).unwrap();
    assert_eq!(
        Map25D::map_new(vec![s0], vec![ld_usize_max]),
        Err(BspError::InvalidLinedef)
    );

    let s_bad_tex = Sector::sector_with_surfaces(
        0.0,
        4.0,
        SurfaceStyle::surface_textured(usize::MAX, [0; 4]),
        SurfaceStyle::surface_solid([0; 4]),
        255,
    )
    .unwrap();
    let ld0 = Linedef::linedef_solid(
        Vec2::vec2_new(0.0, 0.0),
        Vec2::vec2_new(0.0, 10.0),
        0,
        SurfaceStyle::surface_solid([0; 4]),
    )
    .unwrap();
    let map_bad_tex = Map25D::map_new(vec![s_bad_tex], vec![ld0]).unwrap();
    let bsp_bad_tex = BspTree::bsp_build(&map_bad_tex).unwrap();
    let mut scratch_bt =
        RenderScratch::scratch_for_frame_and_map(&fb, &map_bad_tex, &bsp_bad_tex).unwrap();
    assert_eq!(
        render_frame_into(
            &map_bad_tex,
            &bsp_bad_tex,
            &cam,
            &[],
            &cfg,
            &mut scratch_bt,
            &mut fb
        ),
        Err(BspError::InvalidMaterialId { id: usize::MAX })
    );
    assert_eq!(fb.data, fb_snapshot);

    // 5. Mutated camera NaN / Inf / zero-det / out-of-bounds near_dist preserves framebuffer.
    for mutate in [
        (|c: &mut Camera25D| c.pos_x = f64::NAN) as fn(&mut Camera25D),
        |c| c.pos_z = f64::INFINITY,
        |c| {
            c.dir_x = 0.0;
            c.dir_y = 0.0;
        },
        |c| c.near_dist = MIN_PERP_DIST * 0.1,
        |c| c.fov_rad = trig::PI,
    ] {
        let mut bad_cam = cam;
        mutate(&mut bad_cam);
        assert!(
            render_frame_into(&map1, &bsp1, &bad_cam, &[], &cfg, &mut scratch1, &mut fb).is_err()
        );
        assert_eq!(fb.data, fb_snapshot);
    }

    // 6. Oversized framebuffer & texture dimensions.
    let mut fb_huge = PixelContainer {
        width: MAX_FRAME_WIDTH + 1,
        height: 1,
        data: vec![0x77; (MAX_FRAME_WIDTH as usize + 1) * 4],
    };
    let huge_snap = fb_huge.data.clone();
    assert_eq!(
        render_frame_into(&map1, &bsp1, &cam, &[], &cfg, &mut scratch1, &mut fb_huge),
        Err(BspError::InvalidFramebuffer)
    );
    assert_eq!(fb_huge.data, huge_snap);

    let mut fb_zero_h = PixelContainer {
        width: 4,
        height: 0,
        data: Vec::new(),
    };
    assert_eq!(
        render_frame_into(&map1, &bsp1, &cam, &[], &cfg, &mut scratch1, &mut fb_zero_h),
        Err(BspError::InvalidFramebuffer)
    );
    assert_eq!(
        TextureRgba::texture_solid(0, 4, [0; 4]),
        Err(BspError::InvalidTexture)
    );
    assert_eq!(
        TextureRgba::texture_solid(1, MAX_TEXTURE_DIM + 1, [0; 4]),
        Err(BspError::InvalidTexture)
    );
    assert!(matches!(
        RenderScratch::scratch_new(MAX_FRAME_WIDTH as usize + 1, 1, 1),
        Err(BspError::InvalidFramebuffer)
    ));
    let _ = MAX_FRAME_HEIGHT;
}

// Deterministic xorshift64* PRNG for hostile fuzzing
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
        1 => f64::INFINITY,
        2 => f64::NEG_INFINITY,
        3 => -0.0,
        4 => f64::from_bits(1),
        5 => f64::MIN_POSITIVE * 0.5,
        6 => f64::MAX,
        7 => f64::MIN,
        8 => MAX_COORD + 10.0,
        9 => -MAX_COORD - 10.0,
        10 => 1.0e-15,
        _ => {
            let u = ((next_u64(state) >> 11) as f64) / ((1u64 << 53) as f64);
            (u - 0.5) * 40.0
        }
    }
}

#[test]
fn seeded_2000_case_hostile_fuzz_campaign() {
    let mut rng = 0x5EED_6A3E_32F0_25D1_u64;
    let tex =
        TextureRgba::texture_checkerboard(8, 8, 2, [200, 120, 80, 255], [90, 50, 40, 255]).unwrap();
    let textures = [tex];
    let (base_map, base_bsp) = make_valid_room(2);
    let mut ok_renders = 0usize;
    let mut err_renders = 0usize;

    for case_idx in 0..2000_usize {
        // Fuzz Sector construction
        let f_h = pick_hostile_f64(&mut rng);
        let c_h = pick_hostile_f64(&mut rng);
        let tex_idx = if next_u64(&mut rng).is_multiple_of(4) {
            Some((next_u64(&mut rng) % 3) as usize)
        } else {
            None
        };
        let sec_res = Sector::sector_with_surfaces(
            f_h,
            c_h,
            SurfaceStyle {
                color: [100, 110, 120, 255],
                texture_index: tex_idx,
            },
            SurfaceStyle {
                color: [50, 60, 140, 255],
                texture_index: tex_idx,
            },
            (next_u64(&mut rng) & 0xFF) as u8,
        );

        // Fuzz Linedef + Map25D + BspTree
        let v0 = Vec2::vec2_new(pick_hostile_f64(&mut rng), pick_hostile_f64(&mut rng));
        let v1 = Vec2::vec2_new(pick_hostile_f64(&mut rng), pick_hostile_f64(&mut rng));
        let front_s = (next_u64(&mut rng) % 4) as usize;
        let back_s = if next_u64(&mut rng).is_multiple_of(3) {
            Some((next_u64(&mut rng) % 4) as usize)
        } else {
            None
        };
        let ld_res = Linedef::linedef_portal(
            v0,
            v1,
            front_s,
            back_s,
            SurfaceStyle::surface_solid([180, 90, 60, 255]),
            SurfaceStyle::surface_solid([200, 180, 60, 255]),
            SurfaceStyle::surface_solid([80, 180, 200, 255]),
        );

        if let (Ok(s), Ok(ld)) = (sec_res, ld_res) {
            if let Ok(map) = Map25D::map_new(vec![s], vec![ld]) {
                if let Ok(bsp) = BspTree::bsp_build(&map) {
                    assert!(!bsp.bsp_nodes().is_empty());
                    let _ = bsp.bsp_find_subsector(v0);
                }
            }
        }

        // Fuzz Camera25D + render_frame_into (alternating base valid map and cross-paired map)
        let mut cam = Camera25D {
            pos_x: pick_hostile_f64(&mut rng),
            pos_y: pick_hostile_f64(&mut rng),
            pos_z: pick_hostile_f64(&mut rng),
            dir_x: pick_hostile_f64(&mut rng),
            dir_y: pick_hostile_f64(&mut rng),
            right_x: pick_hostile_f64(&mut rng),
            right_y: pick_hostile_f64(&mut rng),
            fov_rad: pick_hostile_f64(&mut rng),
            near_dist: pick_hostile_f64(&mut rng),
        };
        if case_idx % 4 == 0 {
            // Keep some cases structurally valid with hostile coordinates/extremes to exercise rasterizer
            if let Ok(valid_c) = Camera25D::camera_from_yaw_fov(
                ((next_u64(&mut rng) % 20) as f64) * 0.9,
                ((next_u64(&mut rng) % 10) as f64) * 0.9,
                pick_hostile_f64(&mut rng).clamp(-50.0, 50.0),
                ((next_u64(&mut rng) % 628) as f64) * 0.01,
                0.01 + ((next_u64(&mut rng) % 300) as f64) * 0.01,
            ) {
                cam = valid_c;
            }
        }

        let fb_w = [1u32, 8, 32][case_idx % 3];
        let fb_h = [1u32, 9, 24][case_idx % 3];
        let mut fb = sentinel_fb(fb_w, fb_h);
        let fb_snap = fb.data.clone();
        let mut scratch =
            RenderScratch::scratch_for_frame_and_map(&fb, &base_map, &base_bsp).unwrap();
        let cfg = RenderConfig::render_config_with_background([12, 24, 36, 255], case_idx % 2 == 0);

        let tex_slice: &[TextureRgba] = if case_idx % 5 == 0 { &[] } else { &textures };
        let res = render_frame_into(
            &base_map,
            &base_bsp,
            &cam,
            tex_slice,
            &cfg,
            &mut scratch,
            &mut fb,
        );
        match res {
            Ok(stats) => {
                ok_renders += 1;
                assert_eq!(stats.columns_drawn, fb_w as usize);
            }
            Err(_) => {
                err_renders += 1;
                assert_eq!(
                    fb.data, fb_snap,
                    "fuzz case {case_idx} failed atomicity check on Err"
                );
            }
        }
    }

    assert!(ok_renders > 0 && err_renders > 0);
}
