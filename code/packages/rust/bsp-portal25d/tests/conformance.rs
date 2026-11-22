//! # Conformance Tests for `bsp-portal25d` (GAME32)
//!
//! Covers:
//! - 2D BSP tree construction, iterative traversal, and `bsp_find_subsector`
//! - 2.5D Doom/Duke3D-style multi-sector portal rendering (variable floor/ceiling
//!   heights, upper/middle/lower walls, sector light levels, textured planes/walls)
//! - 1D column occlusion buffer (`y_top`, `y_bottom`), early occlusion exit,
//!   node culling, and dynamic `scratch_visible_sectors` vis-lists
//! - Non-convex L-shaped room BSP splitting and closed-door portal seamlessness
//! - `math3d` (`GAME30`) interop (`Vec2`/`Vec3`, `Aabb2`/`Aabb3`, `Ray3`, `Mat4`)
//! - Deterministic `frame_digest_fnv1a` and `scene_digest_fnv1a`
//! - Security guards: CWE-20, CWE-125/787, CWE-190, CWE-369, CWE-400/674

use bsp_portal25d::{
    frame_digest_fnv1a, render_frame_into, scene_digest_fnv1a, Aabb2, BspError, BspNodeKind,
    BspTree, Camera25D, Linedef, Map25D, RenderConfig, RenderScratch, Sector, SurfaceStyle,
    TextureRgba, Vec2, EPSILON, MAX_COORD, MAX_FRAME_WIDTH, MAX_LINEDEFS, MAX_SECTORS,
    MAX_TEXTURE_DIM, MIN_PERP_DIST,
};
use coding_adventures_sha256::sha256_hex;
use pixel_container::PixelContainer;

fn v(x: f64, y: f64) -> Vec2 {
    Vec2::vec2_new(x, y)
}

/// Build a two-sector connected room:
/// - Sector 0 (West, x in [0, 10], y in [0, 10]): floor = 0.0, ceil = 4.0, light = 255
/// - Sector 1 (East, x in [10, 20], y in [0, 10]): floor = 1.0, ceil = 3.0, light = 160
/// - Portal linedef at x = 10 from (10, 10) -> (10, 0) (front = Sector 0, back = Sector 1)
fn make_two_sector_portal_map() -> Map25D {
    let s0 = Sector::sector_new(0.0, 4.0, [40, 80, 40, 255], [40, 40, 90, 255], 255).unwrap();
    let s1 = Sector::sector_new(1.0, 3.0, [80, 120, 80, 255], [70, 70, 130, 255], 160).unwrap();

    let wall_w = SurfaceStyle::surface_solid([180, 60, 60, 255]);
    let wall_e = SurfaceStyle::surface_solid([60, 180, 180, 255]);
    let upper = SurfaceStyle::surface_solid([220, 200, 60, 255]);
    let lower = SurfaceStyle::surface_solid([200, 100, 220, 255]);

    // Clockwise around Sector 0 (normal points inward into Sector 0)
    let l0 = Linedef::linedef_solid(v(0.0, 0.0), v(0.0, 10.0), 0, wall_w).unwrap();
    let l1 = Linedef::linedef_solid(v(0.0, 10.0), v(10.0, 10.0), 0, wall_w).unwrap();
    let l2_portal = Linedef::linedef_portal(
        v(10.0, 10.0),
        v(10.0, 0.0),
        0,
        Some(1),
        wall_w,
        upper,
        lower,
    )
    .unwrap();
    let l3 = Linedef::linedef_solid(v(10.0, 0.0), v(0.0, 0.0), 0, wall_w).unwrap();

    // Clockwise around Sector 1 (x in [10, 20], y in [0, 10]), excluding shared portal edge
    let l4 = Linedef::linedef_solid(v(10.0, 10.0), v(20.0, 10.0), 1, wall_e).unwrap();
    let l5 = Linedef::linedef_solid(v(20.0, 10.0), v(20.0, 0.0), 1, wall_e).unwrap();
    let l6 = Linedef::linedef_solid(v(20.0, 0.0), v(10.0, 0.0), 1, wall_e).unwrap();

    Map25D::map_new(vec![s0, s1], vec![l0, l1, l2_portal, l3, l4, l5, l6]).unwrap()
}

#[test]
fn bsp_build_and_portal_rendering_upper_lower_far_wall() {
    let map = make_two_sector_portal_map();
    let bsp = BspTree::bsp_build(&map).unwrap();
    assert!(!bsp.bsp_nodes().is_empty());
    assert!(bsp.bsp_max_depth() <= bsp_portal25d::MAX_BSP_DEPTH);

    // Camera in Sector 0 at (5, 5, eye=2.0) looking East (+X) through portal at x=10 to far wall at x=20.
    let cam = Camera25D::camera_from_yaw_fov(5.0, 5.0, 2.0, 0.0, trig::PI / 3.0).unwrap();
    let mut fb = PixelContainer::new(160, 120);
    let mut scratch = RenderScratch::scratch_for_frame_and_map(&fb, &map, &bsp).unwrap();
    let cfg = RenderConfig::render_config_default();

    let stats = render_frame_into(&map, &bsp, &cam, &[], &cfg, &mut scratch, &mut fb).unwrap();
    assert_eq!(stats.columns_drawn, 160);
    assert!(stats.portals_traversed >= 1);
    assert!(stats.subsectors_visited >= 2);

    // Front-to-back traversal must visit Sector 0 before Sector 1.
    let vis = scratch.scratch_visible_sectors();
    assert_eq!(vis, &[0, 1]);

    // Check center column (x = 80) top-to-bottom pixel regions:
    // - Top edge (y=0): Sector 0 ceiling [40, 40, 90, 255]
    // - Upper portal wall (front ceil 4.0 -> back ceil 3.0 at dist=5): [220, 200, 60, 255]
    // - Center horizon (y=60): Sector 1 far East wall [60, 180, 180, 255] shaded by light 160!
    // - Lower portal wall (front floor 0.0 -> back floor 1.0 at dist=5): [200, 100, 220, 255]
    // - Bottom edge (y=119): Sector 0 floor [40, 80, 40, 255]
    assert_eq!(fb.pixel_at(80, 0), (40, 40, 90, 255));
    assert_eq!(fb.pixel_at(80, 119), (40, 80, 40, 255));

    // At x=80, dist to portal = 5.0, focal = 80 / tan(30 deg) ~= 138.564.
    // y_ceil(s0=4.0) = 60 - 2.0 * 138.564 / 5.0 ~= 4.57 -> open_top = 5
    // b_ceil(s1=3.0) = 60 - 1.0 * 138.564 / 5.0 ~= 32.28 -> portal_top = 33
    // So y=15 is squarely inside the upper portal wall!
    assert_eq!(fb.pixel_at(80, 15), (220, 200, 60, 255));

    // b_floor(s1=1.0) = 60 - (-1.0) * 138.564 / 5.0 ~= 87.71 -> portal_bot = 87
    // y_floor(s0=0.0) = 60 - (-2.0) * 138.564 / 5.0 ~= 115.42 -> open_bot = 115
    // So y=100 is squarely inside the lower portal wall!
    assert_eq!(fb.pixel_at(80, 100), (200, 100, 220, 255));

    // Center pixel (80, 60) looks through the portal onto Sector 1's East wall [60, 180, 180, 255]
    // shaded by Sector 1 light_level = 160:
    // r = (60*160 + 127)/255 = 38, g = (180*160 + 127)/255 = 113, b = 113
    assert_eq!(fb.pixel_at(80, 60), (38, 113, 113, 255));

    // Walking into Sector 1 and looking West (+PI) must see Sector 1 first, then Sector 0!
    let cam_s1 = Camera25D::camera_from_yaw_fov(15.0, 5.0, 2.0, trig::PI, trig::PI / 3.0).unwrap();
    render_frame_into(&map, &bsp, &cam_s1, &[], &cfg, &mut scratch, &mut fb).unwrap();
    assert_eq!(scratch.scratch_visible_sectors(), &[1, 0]);
}

#[test]
fn solid_wall_occlusion_culls_far_sector_from_vis_list() {
    // Sector 0: [0, 10] x [0, 10], completely closed 1-sided room.
    // Sector 1: [20, 30] x [0, 10], behind Sector 0's East wall.
    let s0 = Sector::sector_new(0.0, 4.0, [40, 40, 40, 255], [60, 60, 60, 255], 255).unwrap();
    let s1 = Sector::sector_new(0.0, 4.0, [90, 90, 90, 255], [90, 90, 90, 255], 255).unwrap();
    let wall0 = SurfaceStyle::surface_solid([200, 50, 50, 255]);
    let wall1 = SurfaceStyle::surface_solid([50, 200, 50, 255]);

    let linedefs = vec![
        Linedef::linedef_solid(v(0.0, 0.0), v(0.0, 10.0), 0, wall0).unwrap(),
        Linedef::linedef_solid(v(0.0, 10.0), v(10.0, 10.0), 0, wall0).unwrap(),
        Linedef::linedef_solid(v(10.0, 10.0), v(10.0, 0.0), 0, wall0).unwrap(),
        Linedef::linedef_solid(v(10.0, 0.0), v(0.0, 0.0), 0, wall0).unwrap(),
        Linedef::linedef_solid(v(20.0, 0.0), v(20.0, 10.0), 1, wall1).unwrap(),
        Linedef::linedef_solid(v(20.0, 10.0), v(30.0, 10.0), 1, wall1).unwrap(),
        Linedef::linedef_solid(v(30.0, 10.0), v(30.0, 0.0), 1, wall1).unwrap(),
        Linedef::linedef_solid(v(30.0, 0.0), v(20.0, 0.0), 1, wall1).unwrap(),
    ];

    let map = Map25D::map_new(vec![s0, s1], linedefs).unwrap();
    let bsp = BspTree::bsp_build(&map).unwrap();
    let cam = Camera25D::camera_from_yaw_fov(5.0, 5.0, 2.0, 0.0, trig::PI / 3.0).unwrap();
    let mut fb = PixelContainer::new(128, 96);
    let mut scratch = RenderScratch::scratch_for_frame_and_map(&fb, &map, &bsp).unwrap();
    let cfg = RenderConfig::render_config_default();

    let stats = render_frame_into(&map, &bsp, &cam, &[], &cfg, &mut scratch, &mut fb).unwrap();
    assert!(stats.early_occlusion_exits);
    // Sector 0 completely closes all 128 columns before traversal reaches Sector 1.
    assert_eq!(scratch.scratch_visible_sectors(), &[0]);
    assert_eq!(fb.pixel_at(64, 48), (200, 50, 50, 255));
}

#[test]
fn non_convex_l_shaped_room_and_closed_door_portal() {
    // Non-convex L-shaped Sector 0 with a closed-door portal (Sector 1 has floor=2.0, ceil=2.0).
    let s0 = Sector::sector_new(0.0, 4.0, [30, 60, 30, 255], [30, 30, 80, 255], 255).unwrap();
    let s1_closed =
        Sector::sector_new(2.0, 2.0, [10, 10, 10, 255], [10, 10, 10, 255], 255).unwrap();

    let wall = SurfaceStyle::surface_solid([150, 150, 170, 255]);
    let upper = SurfaceStyle::surface_solid([210, 180, 40, 255]);
    let lower = SurfaceStyle::surface_solid([40, 180, 210, 255]);

    let linedefs = vec![
        Linedef::linedef_solid(v(0.0, 0.0), v(0.0, 10.0), 0, wall).unwrap(),
        Linedef::linedef_solid(v(0.0, 10.0), v(10.0, 10.0), 0, wall).unwrap(),
        Linedef::linedef_solid(v(10.0, 10.0), v(10.0, 6.0), 0, wall).unwrap(),
        Linedef::linedef_solid(v(10.0, 6.0), v(6.0, 6.0), 0, wall).unwrap(),
        Linedef::linedef_portal(v(6.0, 6.0), v(6.0, 0.0), 0, Some(1), wall, upper, lower).unwrap(),
        Linedef::linedef_solid(v(6.0, 0.0), v(0.0, 0.0), 0, wall).unwrap(),
    ];

    let map = Map25D::map_new(vec![s0, s1_closed], linedefs).unwrap();
    let bsp = BspTree::bsp_build(&map).unwrap();
    // Non-convex L-shape must produce at least one internal splitting node.
    assert!(bsp
        .bsp_nodes()
        .iter()
        .any(|n| n.node_kind() == BspNodeKind::Internal));

    let cam = Camera25D::camera_from_yaw_fov(2.0, 3.0, 2.0, 0.0, trig::PI / 3.0).unwrap();
    // Pre-fill framebuffer with sentinel to verify closed door leaves zero 1px background/sentinel seam.
    let mut fb = PixelContainer::from_data(64, 61, [9, 9, 9, 255].repeat(64 * 61));
    let mut scratch = RenderScratch::scratch_for_frame_and_map(&fb, &map, &bsp).unwrap();
    let cfg = RenderConfig::render_config_with_background([1, 2, 3, 255], true);

    render_frame_into(&map, &bsp, &cam, &[], &cfg, &mut scratch, &mut fb).unwrap();
    // Center row y=30 on closed door at eye=2.0 must be painted with upper or lower (never background [1,2,3,255]).
    for x in 20..44 {
        let px = fb.pixel_at(x, 30);
        assert!(
            px == (210, 180, 40, 255) || px == (40, 180, 210, 255),
            "unexpected seam pixel at x={x}: {px:?}"
        );
    }
}

#[test]
fn textured_surfaces_and_deterministic_digests() {
    let floor_tex =
        TextureRgba::texture_checkerboard(16, 16, 4, [60, 140, 60, 255], [30, 80, 30, 255])
            .unwrap();
    let ceil_tex =
        TextureRgba::texture_checkerboard(16, 16, 4, [40, 60, 160, 255], [20, 30, 100, 255])
            .unwrap();
    let wall_tex =
        TextureRgba::texture_checkerboard(16, 16, 4, [220, 120, 80, 255], [140, 60, 40, 255])
            .unwrap();
    let textures = [wall_tex, floor_tex, ceil_tex];

    let s0 = Sector::sector_with_surfaces(
        0.0,
        4.0,
        SurfaceStyle::surface_textured(1, [40, 100, 40, 255]),
        SurfaceStyle::surface_textured(2, [30, 40, 120, 255]),
        200,
    )
    .unwrap();
    let w = SurfaceStyle::surface_textured(0, [200, 100, 80, 255]);
    let linedefs = vec![
        Linedef::linedef_solid(v(0.0, 0.0), v(0.0, 8.0), 0, w).unwrap(),
        Linedef::linedef_solid(v(0.0, 8.0), v(8.0, 8.0), 0, w).unwrap(),
        Linedef::linedef_solid(v(8.0, 8.0), v(8.0, 0.0), 0, w).unwrap(),
        Linedef::linedef_solid(v(8.0, 0.0), v(0.0, 0.0), 0, w).unwrap(),
    ];

    let map = Map25D::map_new(vec![s0], linedefs).unwrap();
    let bsp1 = BspTree::bsp_build(&map).unwrap();
    let bsp2 = BspTree::bsp_build(&map).unwrap();
    assert_eq!(
        scene_digest_fnv1a(&map, &bsp1),
        scene_digest_fnv1a(&map, &bsp2)
    );

    let cam = Camera25D::camera_from_yaw_fov(2.5, 4.0, 1.8, 0.3, trig::PI / 3.0).unwrap();
    let mut fb1 = PixelContainer::new(96, 72);
    let mut fb2 = PixelContainer::new(96, 72);
    let mut scratch = RenderScratch::scratch_for_frame_and_map(&fb1, &map, &bsp1).unwrap();
    let cfg = RenderConfig::render_config_default();

    render_frame_into(&map, &bsp1, &cam, &textures, &cfg, &mut scratch, &mut fb1).unwrap();
    render_frame_into(&map, &bsp2, &cam, &textures, &cfg, &mut scratch, &mut fb2).unwrap();

    assert_eq!(frame_digest_fnv1a(&fb1), frame_digest_fnv1a(&fb2));
    assert_eq!(sha256_hex(&fb1.data), sha256_hex(&fb2.data));

    // Textured checkerboard should produce more than 1 distinct pixel color across wall and floor.
    let unique_colors: std::collections::BTreeSet<(u8, u8, u8, u8)> = (0..72)
        .flat_map(|y| (0..96).map(move |x| (x, y)))
        .map(|(x, y)| fb1.pixel_at(x, y))
        .collect();
    assert!(unique_colors.len() >= 4);

    // Subsector lookup inside room succeeds.
    let leaf = bsp1.bsp_find_subsector(v(4.0, 4.0)).unwrap();
    assert_eq!(bsp1.bsp_nodes()[leaf].node_kind(), BspNodeKind::Leaf);
}

#[test]
fn math3d_vec2_vec3_aabb_and_camera_interop() {
    let p2 = Vec2::vec2_try_new(3.0, 4.0).unwrap();
    assert!((p2.vec2_length().unwrap() - 5.0).abs() < 1e-12);
    let p3 = p2.vec2_to_vec3(2.5).unwrap();
    assert_eq!(p3, math3d::Vec3::new(3.0, 2.5, 4.0));
    assert_eq!(Vec2::vec2_from_vec3_xz(p3).unwrap(), p2);

    let box2 = Aabb2::aabb2_from_points(v(1.0, 2.0), v(5.0, 6.0)).unwrap();
    assert!(box2.aabb2_contains_point(v(3.0, 4.0)).unwrap());
    let box3 = box2.aabb2_to_aabb3(0.0, 4.0).unwrap();
    assert_eq!(box3.min, math3d::Vec3::new(1.0, 0.0, 2.0));
    assert_eq!(box3.max, math3d::Vec3::new(5.0, 4.0, 6.0));

    let cam = Camera25D::camera_from_yaw_fov(1.0, 2.0, 1.5, 0.0, trig::PI / 3.0).unwrap();
    let fwd = cam.camera_forward_vec3().unwrap();
    assert!((fwd.x - 1.0).abs() < 1e-12 && fwd.y.abs() < 1e-12 && fwd.z.abs() < 1e-12);
    let ray = cam.camera_world_ray().unwrap();
    assert_eq!(ray.origin, math3d::Vec3::new(1.0, 1.5, 2.0));
    assert!(cam.camera_view_matrix().is_ok());

    let rot = cam.camera_rotate(trig::PI * 0.5).unwrap();
    let moved = rot.camera_move_by(1.0, -0.5, 0.25).unwrap();
    assert!((moved.camera_eye_height() - 1.75).abs() < 1e-12);
}

#[test]
fn security_cwe_20_125_190_369_400_674_guards() {
    // CWE-20: NaN / Inf / out-of-bounds coordinates and heights
    assert_eq!(
        Vec2::vec2_try_new(f64::NAN, 0.0),
        Err(BspError::NonFiniteInput)
    );
    assert_eq!(
        Vec2::vec2_try_new(MAX_COORD + 1.0, 0.0),
        Err(BspError::ValueOutOfBounds)
    );
    assert_eq!(
        Sector::sector_new(5.0, 2.0, [0; 4], [0; 4], 255),
        Err(BspError::InvalidSector)
    );
    assert_eq!(
        Camera25D::camera_from_yaw_fov(0.0, 0.0, 1.0, 0.0, 0.0),
        Err(BspError::ValueOutOfBounds)
    );
    assert_eq!(
        Camera25D::camera_from_yaw_fov(0.0, 0.0, 1.0, 0.0, trig::PI),
        Err(BspError::ValueOutOfBounds)
    );

    // CWE-369: zero-length linedef & degenerate/collinear camera
    assert_eq!(
        Linedef::linedef_solid(
            v(1.0, 1.0),
            v(1.0, 1.0),
            0,
            SurfaceStyle::surface_solid([0; 4])
        ),
        Err(BspError::DegenerateSegment)
    );
    assert_eq!(
        Camera25D::camera_new(0.0, 0.0, 1.0, 1.0, 0.0, 2.0, 0.0, 1.0, 0.05),
        Err(BspError::DegenerateCamera)
    );
    assert_eq!(
        Camera25D::camera_new(0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.05),
        Err(BspError::DegenerateCamera)
    );
    assert_eq!(
        Camera25D::camera_new(0.0, 0.0, 1.0, 1.0, 0.0, 0.0, -1.0, 1.0, MIN_PERP_DIST * 0.1),
        Err(BspError::ValueOutOfBounds)
    );

    // CWE-400: oversized sector / linedef / texture / framebuffer budgets
    assert_eq!(
        Map25D::map_new(vec![], vec![]),
        Err(BspError::InvalidSector)
    );
    let s_ok = Sector::sector_new(0.0, 4.0, [0; 4], [0; 4], 255).unwrap();
    let ld_ok = Linedef::linedef_solid(
        v(0.0, 0.0),
        v(1.0, 0.0),
        0,
        SurfaceStyle::surface_solid([0; 4]),
    )
    .unwrap();
    assert_eq!(
        Map25D::map_new(vec![s_ok; MAX_SECTORS + 1], vec![ld_ok]),
        Err(BspError::InvalidSector)
    );
    assert_eq!(
        Map25D::map_new(vec![s_ok], vec![ld_ok; MAX_LINEDEFS + 1]),
        Err(BspError::InvalidLinedef)
    );
    assert_eq!(
        TextureRgba::texture_solid(MAX_TEXTURE_DIM + 1, 1, [255; 4]),
        Err(BspError::InvalidTexture)
    );

    // CWE-125 / CWE-787: invalid sector index & missing material ID & scratch too small
    let ld_bad_sec = Linedef::linedef_solid(
        v(0.0, 0.0),
        v(1.0, 0.0),
        1,
        SurfaceStyle::surface_solid([0; 4]),
    )
    .unwrap();
    assert_eq!(
        Map25D::map_new(vec![s_ok], vec![ld_bad_sec]),
        Err(BspError::InvalidLinedef)
    );

    let map = make_two_sector_portal_map();
    let bsp = BspTree::bsp_build(&map).unwrap();
    let cam = Camera25D::camera_from_yaw_fov(5.0, 5.0, 2.0, 0.0, trig::PI / 3.0).unwrap();
    let cfg = RenderConfig::render_config_default();

    let mut fb = PixelContainer::new(64, 48);
    let mut small_scratch = RenderScratch::scratch_new(32, 2, bsp.bsp_nodes().len()).unwrap();
    assert_eq!(
        render_frame_into(&map, &bsp, &cam, &[], &cfg, &mut small_scratch, &mut fb),
        Err(BspError::ScratchTooSmall {
            required: 64,
            actual: 32
        })
    );

    let s_tex = Sector::sector_with_surfaces(
        0.0,
        4.0,
        SurfaceStyle::surface_textured(99, [0; 4]),
        SurfaceStyle::surface_solid([0; 4]),
        255,
    )
    .unwrap();
    let map_bad_tex = Map25D::map_new(vec![s_tex], vec![ld_ok]).unwrap();
    let bsp_bad_tex = BspTree::bsp_build(&map_bad_tex).unwrap();
    let mut scratch =
        RenderScratch::scratch_for_frame_and_map(&fb, &map_bad_tex, &bsp_bad_tex).unwrap();
    assert_eq!(
        render_frame_into(
            &map_bad_tex,
            &bsp_bad_tex,
            &cam,
            &[],
            &cfg,
            &mut scratch,
            &mut fb
        ),
        Err(BspError::InvalidMaterialId { id: 99 })
    );

    // Oversized framebuffer rejection
    let mut oversized_fb = PixelContainer {
        width: MAX_FRAME_WIDTH + 1,
        height: 1,
        data: vec![0; (MAX_FRAME_WIDTH as usize + 1) * 4],
    };
    assert_eq!(
        render_frame_into(&map, &bsp, &cam, &[], &cfg, &mut scratch, &mut oversized_fb),
        Err(BspError::InvalidFramebuffer)
    );

    // CWE-190: extreme near-plane & narrow-FOV projection must not overflow or panic
    let cam_extreme =
        Camera25D::camera_from_yaw_fov(9.999999, 5.0, 2.0, 0.0, EPSILON * 1000.0).unwrap();
    let mut scratch_ok = RenderScratch::scratch_for_frame_and_map(&fb, &map, &bsp).unwrap();
    assert!(render_frame_into(
        &map,
        &bsp,
        &cam_extreme,
        &[],
        &cfg,
        &mut scratch_ok,
        &mut fb
    )
    .is_ok());

    // Mutated NaN camera field caught at render entry
    let mut bad_cam = cam;
    bad_cam.pos_x = f64::NAN;
    assert_eq!(
        render_frame_into(&map, &bsp, &bad_cam, &[], &cfg, &mut scratch_ok, &mut fb),
        Err(BspError::NonFiniteInput)
    );
}
