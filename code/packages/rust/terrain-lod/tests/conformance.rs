//! # Conformance Tests for `terrain-lod` (GAME42)
//!
//! Covers:
//! - Deterministic heightfield synthesis (diamond-square & value-noise fBm via `rng::Pcg32`)
//!   and golden digest stability
//! - Bilinear `height_at(x, z)` and central-difference `normal_at(x, z)` / `vertex_normal`
//! - Rotated runway placement with linear shoulder blending and atomic out-of-bounds refusal
//! - Restricted quadtree LOD (`|level_a - level_b| <= 1`), camera-distance responsiveness,
//!   and `math3d::Frustum` culling
//! - Watertight crack-free mesh invariant (every non-boundary directed edge `(u, v)` has
//!   exact twin `(v, u)`, consistent CCW winding)
//! - Atomic `CapacityExceeded` / `BudgetExceeded` refusal leaving caller buffers unchanged
//! - CWE-20, CWE-125, CWE-190, CWE-400, CWE-787, CWE-835 security guards
//! - 1,000-case deterministic seeded invariant campaign

use std::collections::HashMap;

use math3d::{Frustum, Mat4, Vec3};
use terrain_lod::{
    emit_mesh_into, indices_digest_fnv1a, mesh_digest_fnv1a, refine_into, validate_grid_dimensions,
    validate_grid_level, validate_index_capacity, validate_lod_budget, Heightfield, HeightfieldMut,
    HeightfieldSpec, LodParams, RunwaySpec, TerrainError, TerrainScratch, MAX_LEVELS,
    MAX_QUADTREE_NODES,
};

fn approx(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

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

fn make_grid(level: u32) -> (HeightfieldSpec, Vec<f64>) {
    let spec = HeightfieldSpec::spec_new(level, 10.0, -320.0, -320.0).unwrap();
    let count = spec.spec_vertex_count().unwrap();
    (spec, vec![0.0; count])
}

#[test]
fn deterministic_synthesis_and_golden_digest() {
    let (spec, mut buf_a) = make_grid(5); // 33x33
    let (_, mut buf_b) = make_grid(5);

    {
        let mut hfm = HeightfieldMut::from_heights_mut(spec, &mut buf_a).unwrap();
        hfm.synthesize_diamond_square(0x4242_1337, 100.0, 45.0, 0.55)
            .unwrap();
    }
    {
        let mut hfm = HeightfieldMut::from_heights_mut(spec, &mut buf_b).unwrap();
        hfm.synthesize_diamond_square(0x4242_1337, 100.0, 45.0, 0.55)
            .unwrap();
    }
    assert_eq!(buf_a, buf_b);

    let hf_a = Heightfield::from_heights(spec, &buf_a).unwrap();
    let mut scratch = TerrainScratch::scratch_with_capacity(4096).unwrap();
    let params =
        LodParams::lod_params_new(Vec3::new(0.0, 80.0, 0.0), 0.02, 1.0, 5, 4096, 65_536, None)
            .unwrap();
    let stats = refine_into(&hf_a, &params, &mut scratch).unwrap();
    let mut indices = vec![0u32; stats.index_count];
    let emitted = emit_mesh_into(&hf_a, &scratch, &mut indices, 65_536).unwrap();
    assert_eq!(emitted, stats.index_count);

    let d1 = mesh_digest_fnv1a(&hf_a, &indices).unwrap();
    let d2 = mesh_digest_fnv1a(&hf_a, &indices).unwrap();
    assert_eq!(d1, d2);
    // Golden digest check: must remain bit-exact across platforms and runs.
    assert_eq!(d1, 0xba20_d084_44df_ec73);

    // fBm synthesis determinism
    let mut fbm_a = vec![0.0; buf_a.len()];
    let mut fbm_b = vec![0.0; buf_a.len()];
    HeightfieldMut::from_heights_mut(spec, &mut fbm_a)
        .unwrap()
        .synthesize_fbm(0x9999_0001, 50.0, 30.0, 4.0, 5, 0.5, 2.0)
        .unwrap();
    HeightfieldMut::from_heights_mut(spec, &mut fbm_b)
        .unwrap()
        .synthesize_fbm(0x9999_0001, 50.0, 30.0, 4.0, 5, 0.5, 2.0)
        .unwrap();
    assert_eq!(fbm_a, fbm_b);
    assert_ne!(fbm_a, buf_a);
}

#[test]
fn bilinear_height_and_central_difference_normals() {
    let spec = HeightfieldSpec::spec_new(3, 5.0, 10.0, 20.0).unwrap(); // 9x9, x in [10..50], z in [20..60]
    let side = spec.spec_side();
    let mut heights = vec![0.0; side * side];

    // Linear ramp: h(x, z) = 0.5 * x - 0.25 * z + 12.0
    for iz in 0..side {
        for ix in 0..side {
            let x = spec.origin_x + (ix as f64) * spec.cell_size;
            let z = spec.origin_z + (iz as f64) * spec.cell_size;
            heights[iz * side + ix] = 0.5 * x - 0.25 * z + 12.0;
        }
    }

    let hf = Heightfield::from_heights(spec, &heights).unwrap();

    // Check exact bilinear interpolation at interior and boundary points.
    for &(x, z) in &[
        (10.0, 20.0),
        (50.0, 60.0),
        (22.75, 34.3),
        (10.0, 41.25),
        (49.9, 20.0),
    ] {
        let expected = 0.5 * x - 0.25 * z + 12.0;
        let got = hf.height_at(x, z).unwrap();
        assert!(
            approx(got, expected, 1e-11),
            "got={got}, expected={expected}"
        );
    }

    // Normal of h = 0.5*x - 0.25*z + 12 is normalize(-0.5, 1.0, 0.25).
    let expected_n = Vec3::new(-0.5, 1.0, 0.25).normalize().unwrap();
    for &(x, z) in &[(25.0, 35.0), (10.0, 20.0), (50.0, 60.0)] {
        let n = hf.normal_at(x, z).unwrap();
        assert!(approx(n.x, expected_n.x, 1e-11));
        assert!(approx(n.y, expected_n.y, 1e-11));
        assert!(approx(n.z, expected_n.z, 1e-11));
    }

    let vn = hf.vertex_normal(4, 4).unwrap();
    assert!(approx(vn.x, expected_n.x, 1e-11));
    assert!(approx(vn.y, expected_n.y, 1e-11));
    assert!(approx(vn.z, expected_n.z, 1e-11));
}

#[test]
fn rotated_runway_flatness_shoulder_and_bounds_refusal() {
    let spec = HeightfieldSpec::spec_new(6, 10.0, 0.0, 0.0).unwrap(); // 65x65, [0..640]^2
    let mut heights = vec![0.0; spec.spec_vertex_count().unwrap()];
    {
        let mut hfm = HeightfieldMut::from_heights_mut(spec, &mut heights).unwrap();
        hfm.synthesize_diamond_square(777, 150.0, 60.0, 0.6)
            .unwrap();

        let runway = RunwaySpec::runway_new(
            320.0, 320.0, 0.52, // ~30 deg rotated
            200.0, 60.0, 40.0, 125.0,
        )
        .unwrap();
        hfm.place_runway(&runway).unwrap();
    }

    let hf = Heightfield::from_heights(spec, &heights).unwrap();

    // Sample points well inside the rotated core rectangle: must be flat at 125.0.
    let fwd_x = trig::cos(0.52);
    let fwd_z = trig::sin(0.52);
    let right_x = -fwd_z;
    let right_z = fwd_x;

    for u in [-60.0, -30.0, 0.0, 30.0, 60.0] {
        for v in [-12.0, 0.0, 12.0] {
            let x = 320.0 + u * fwd_x + v * right_x;
            let z = 320.0 + u * fwd_z + v * right_z;
            let h = hf.height_at(x, z).unwrap();
            assert!(
                approx(h, 125.0, 1e-11),
                "runway core not flat at ({x}, {z}): h={h}"
            );
            let n = hf.normal_at(x, z).unwrap();
            assert!(approx(n.y, 1.0, 1e-11));
        }
    }

    // Out-of-bounds runway must refuse atomically without modifying `heights`.
    let before = heights.clone();
    let bad_runway = RunwaySpec::runway_new(50.0, 50.0, 0.25, 200.0, 60.0, 40.0, 10.0).unwrap();
    let res = HeightfieldMut::from_heights_mut(spec, &mut heights)
        .unwrap()
        .place_runway(&bad_runway);
    assert_eq!(res, Err(TerrainError::OutOfBounds));
    assert_eq!(heights, before);
}

/// Verify watertight crack-free topology and consistent CCW +Y winding.
fn assert_watertight_and_ccw(hf: &Heightfield<'_>, indices: &[u32]) {
    let side = hf.grid_side() as u32;
    let (tris, rem) = indices.as_chunks::<3>();
    assert!(rem.is_empty(), "index count not divisible by 3");
    assert!(!tris.is_empty(), "expected at least one triangle");

    let mut edges: HashMap<(u32, u32), usize> = HashMap::new();
    for &[i0, i1, i2] in tris {
        assert_ne!(i0, i1);
        assert_ne!(i1, i2);
        assert_ne!(i2, i0);

        // Check CCW winding in XZ plane from +Y looking down.
        let (x0, z0) = ((i0 % side) as i64, (i0 / side) as i64);
        let (x1, z1) = ((i1 % side) as i64, (i1 / side) as i64);
        let (x2, z2) = ((i2 % side) as i64, (i2 / side) as i64);
        let area2 = (x1 - x0) * (z2 - z0) - (z1 - z0) * (x2 - x0);
        // Right-handed +X right, +Z down: CW in 2D screen (x, z) corresponds to +Y up normal!
        assert!(
            area2 < 0,
            "triangle ({i0}, {i1}, {i2}) has inverted winding"
        );

        for (u, v) in [(i0, i1), (i1, i2), (i2, i0)] {
            *edges.entry((u, v)).or_insert(0) += 1;
        }
    }

    let is_boundary_vertex = |idx: u32| -> (bool, bool, bool, bool) {
        let ix = idx % side;
        let iz = idx / side;
        (ix == 0, ix == side - 1, iz == 0, iz == side - 1)
    };

    for (&(u, v), &count) in &edges {
        assert_eq!(count, 1, "duplicate directed edge ({u}, {v})");
        if edges.contains_key(&(v, u)) {
            continue;
        }
        let bu = is_boundary_vertex(u);
        let bv = is_boundary_vertex(v);
        let shared_border = (bu.0 && bv.0) || (bu.1 && bv.1) || (bu.2 && bv.2) || (bu.3 && bv.3);
        assert!(
            shared_border,
            "crack / T-junction detected: interior edge ({u}, {v}) has no opposite twin"
        );
    }
}

#[test]
fn lod_restricted_crack_free_distance_and_frustum() {
    let (spec, mut buf) = make_grid(6); // 65x65, [-320..320]^2
    HeightfieldMut::from_heights_mut(spec, &mut buf)
        .unwrap()
        .synthesize_diamond_square(12345, 50.0, 40.0, 0.65)
        .unwrap();
    let hf = Heightfield::from_heights(spec, &buf).unwrap();
    let mut scratch = TerrainScratch::scratch_with_capacity(8192).unwrap();

    // Near camera -> more triangles than far camera.
    let near_params = LodParams::lod_params_new(
        Vec3::new(-100.0, 70.0, -100.0),
        0.02,
        1.0,
        6,
        8192,
        65_536,
        None,
    )
    .unwrap();
    let near_stats = refine_into(&hf, &near_params, &mut scratch).unwrap();
    assert!(scratch.scratch_is_restricted());

    let mut near_indices = vec![0u32; near_stats.index_count];
    emit_mesh_into(&hf, &scratch, &mut near_indices, 65_536).unwrap();
    assert_watertight_and_ccw(&hf, &near_indices);

    let far_params = LodParams::lod_params_new(
        Vec3::new(0.0, 5000.0, 0.0),
        0.02,
        1.0,
        6,
        8192,
        65_536,
        None,
    )
    .unwrap();
    let far_stats = refine_into(&hf, &far_params, &mut scratch).unwrap();
    assert!(scratch.scratch_is_restricted());
    assert!(
        far_stats.triangle_count < near_stats.triangle_count,
        "expected far ({}) < near ({})",
        far_stats.triangle_count,
        near_stats.triangle_count
    );

    let mut far_indices = vec![0u32; far_stats.index_count];
    emit_mesh_into(&hf, &scratch, &mut far_indices, 65_536).unwrap();
    assert_watertight_and_ccw(&hf, &far_indices);

    // Optional frustum culling: looking away from terrain culls all nodes.
    let view_away = Mat4::look_at(
        Vec3::new(0.0, 100.0, 1000.0),
        Vec3::new(0.0, 100.0, 2000.0),
        Vec3::unit_y(),
    )
    .unwrap();
    let proj = Mat4::perspective(trig::PI / 3.0, 1.0, 1.0, 500.0).unwrap();
    let frustum_away = Frustum::from_view_proj(&proj.mul(&view_away).unwrap()).unwrap();
    let culled_params = LodParams::lod_params_new(
        Vec3::new(0.0, 100.0, 1000.0),
        0.02,
        1.0,
        6,
        8192,
        65_536,
        Some(frustum_away),
    )
    .unwrap();
    let culled_stats = refine_into(&hf, &culled_params, &mut scratch).unwrap();
    assert_eq!(culled_stats.visible_leaf_count, 0);
    assert_eq!(culled_stats.triangle_count, 0);

    // Frustum looking across part of the terrain reduces visible leaves vs unculled.
    let view_part = Mat4::look_at(
        Vec3::new(0.0, 150.0, 0.0),
        Vec3::new(200.0, 40.0, 200.0),
        Vec3::unit_y(),
    )
    .unwrap();
    let frustum_part = Frustum::from_view_proj(&proj.mul(&view_part).unwrap()).unwrap();
    let part_params = LodParams::lod_params_new(
        Vec3::new(0.0, 150.0, 0.0),
        0.02,
        1.0,
        6,
        8192,
        65_536,
        Some(frustum_part),
    )
    .unwrap();
    let part_stats = refine_into(&hf, &part_params, &mut scratch).unwrap();
    assert!(scratch.scratch_is_restricted());
    assert!(part_stats.visible_leaf_count > 0);
    assert!(part_stats.visible_leaf_count < part_stats.leaf_count);
}

#[test]
fn capacity_refusal_leaves_buffers_unchanged_and_cwe_guards() {
    let (spec, mut buf) = make_grid(4); // 17x17
    HeightfieldMut::from_heights_mut(spec, &mut buf)
        .unwrap()
        .synthesize_diamond_square(99, 20.0, 15.0, 0.6)
        .unwrap();
    let hf = Heightfield::from_heights(spec, &buf).unwrap();
    let mut scratch = TerrainScratch::scratch_with_capacity(1024).unwrap();
    let params = LodParams::lod_params_new(
        Vec3::new(-100.0, 30.0, -100.0),
        0.01,
        1.0,
        4,
        1024,
        65_536,
        None,
    )
    .unwrap();
    let stats = refine_into(&hf, &params, &mut scratch).unwrap();
    assert!(stats.index_count > 6);

    // Too-small index buffer must refuse atomically without writing any sentinel slot.
    let mut small = vec![0xDEAD_BEEF_u32; stats.index_count - 1];
    assert_eq!(
        emit_mesh_into(&hf, &scratch, &mut small, 65_536),
        Err(TerrainError::CapacityExceeded)
    );
    assert!(small.iter().all(|&v| v == 0xDEAD_BEEF));

    // Too-small max_triangles must return BudgetExceeded without touching buffer.
    let mut exact = vec![0xCAFE_BABE_u32; stats.index_count];
    assert_eq!(
        emit_mesh_into(&hf, &scratch, &mut exact, stats.triangle_count - 1),
        Err(TerrainError::BudgetExceeded)
    );
    assert!(exact.iter().all(|&v| v == 0xCAFE_BABE));

    // CWE-190 / CWE-20 / CWE-125 / CWE-400 boundary guards.
    assert_eq!(validate_grid_level(0), Err(TerrainError::InvalidParameter));
    assert_eq!(
        validate_grid_level(MAX_LEVELS + 1),
        Err(TerrainError::InvalidParameter)
    );
    assert_eq!(
        validate_grid_dimensions(65, 4224),
        Err(TerrainError::InvalidGridSize)
    );
    assert_eq!(
        validate_grid_dimensions(usize::MAX, 1),
        Err(TerrainError::InvalidGridSize)
    );
    assert_eq!(
        validate_lod_budget(10, 5, 0, 100),
        Err(TerrainError::BudgetExceeded)
    );
    assert_eq!(
        validate_index_capacity(100, 99),
        Err(TerrainError::CapacityExceeded)
    );
    assert_eq!(
        TerrainScratch::scratch_with_capacity(MAX_QUADTREE_NODES + 1).unwrap_err(),
        TerrainError::BudgetExceeded
    );

    assert_eq!(
        LodParams::lod_params_new(
            Vec3::new(f64::NAN, 0.0, 0.0),
            0.02,
            1.0,
            4,
            1024,
            1024,
            None
        ),
        Err(TerrainError::NonFiniteInput)
    );
    assert_eq!(
        LodParams::lod_params_new(Vec3::zero(), 0.0, 1.0, 4, 1024, 1024, None),
        Err(TerrainError::InvalidParameter)
    );
    assert_eq!(hf.height_at(-1000.0, 0.0), Err(TerrainError::OutOfBounds));
    assert_eq!(
        hf.normal_at(f64::INFINITY, 0.0),
        Err(TerrainError::NonFiniteInput)
    );
    assert_eq!(
        mesh_digest_fnv1a(&hf, &[0, 1, 999_999]),
        Err(TerrainError::OutOfBounds)
    );
    assert_eq!(
        indices_digest_fnv1a(&[0, 1, 2]),
        indices_digest_fnv1a(&[0, 1, 2])
    );

    // Scale-relative runway dimension guard: length < cell_size * 1e-3 is rejected.
    let tiny_runway = RunwaySpec::runway_new(0.0, 0.0, 0.0, 1e-4, 5.0, 1.0, 0.0).unwrap();
    assert_eq!(
        tiny_runway.runway_validate(&spec),
        Err(TerrainError::InvalidRunway)
    );
}

#[test]
fn seeded_1000_case_campaign() {
    let mut prng = 0x5eed_cafe_1234_5678_u64;
    let mut scratch = TerrainScratch::scratch_with_capacity(4096).unwrap();
    let mut indices = vec![0u32; 16_384];
    let mut pos_buf = vec![Vec3::zero(); 1089];
    let mut nrm_buf = vec![Vec3::zero(); 1089];

    for case_idx in 0..1000u64 {
        let level = ((next_u64(&mut prng) % 4) + 2) as u32; // levels 2..=5 (5x5 .. 33x33)
        let cell_size = next_range(&mut prng, 1.0, 25.0);
        let origin_x = next_range(&mut prng, -500.0, 500.0);
        let origin_z = next_range(&mut prng, -500.0, 500.0);
        let spec = HeightfieldSpec::spec_new(level, cell_size, origin_x, origin_z).unwrap();

        let count = spec.spec_vertex_count().unwrap();
        let mut heights = vec![0.0; count];
        {
            let mut hfm = HeightfieldMut::from_heights_mut(spec, &mut heights).unwrap();
            if case_idx % 2 == 0 {
                hfm.synthesize_diamond_square(
                    next_u64(&mut prng),
                    next_range(&mut prng, -100.0, 100.0),
                    next_range(&mut prng, 1.0, 50.0),
                    next_range(&mut prng, 0.3, 0.8),
                )
                .unwrap();
            } else {
                hfm.synthesize_fbm(
                    next_u64(&mut prng),
                    next_range(&mut prng, -100.0, 100.0),
                    next_range(&mut prng, 1.0, 50.0),
                    next_range(&mut prng, 1.0, 6.0),
                    ((next_u64(&mut prng) % 5) + 1) as u32,
                    next_range(&mut prng, 0.3, 0.8),
                    next_range(&mut prng, 1.5, 2.5),
                )
                .unwrap();
            }
        }

        let hf = Heightfield::from_heights(spec, &heights).unwrap();
        let span = spec.spec_world_span();
        let cam = Vec3::new(
            origin_x + next_range(&mut prng, -span * 0.5, span * 1.5),
            next_range(&mut prng, 10.0, 500.0),
            origin_z + next_range(&mut prng, -span * 0.5, span * 1.5),
        );
        let params = LodParams::lod_params_new(
            cam,
            next_range(&mut prng, 0.01, 0.25),
            next_range(&mut prng, 0.5, 500.0),
            level,
            4096,
            5_400,
            None,
        )
        .unwrap();

        let stats = refine_into(&hf, &params, &mut scratch).unwrap();
        assert!(scratch.scratch_is_restricted());
        assert_eq!(scratch.scratch_node_count(), stats.node_count);
        assert_eq!(scratch.scratch_leaf_count(), stats.leaf_count);
        assert_eq!(scratch.scratch_max_leaf_depth(), stats.max_leaf_depth);

        let emitted = emit_mesh_into(&hf, &scratch, &mut indices, 5_400).unwrap();
        assert_eq!(emitted, stats.index_count);
        assert_watertight_and_ccw(&hf, &indices[..emitted]);

        let qx = origin_x + next_range(&mut prng, 0.001, 0.999) * span;
        let qz = origin_z + next_range(&mut prng, 0.001, 0.999) * span;
        assert!(hf.height_at(qx, qz).unwrap().is_finite());
        let n = hf.normal_at(qx, qz).unwrap();
        assert!(approx(n.length().unwrap(), 1.0, 1e-9));

        let v_count = hf
            .emit_vertices_into(&mut pos_buf[..count], &mut nrm_buf[..count])
            .unwrap();
        assert_eq!(v_count, count);
    }
}
