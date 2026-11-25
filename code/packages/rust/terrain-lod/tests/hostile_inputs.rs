//! # Hostile-Input CVE/CWE Proof Suite for `terrain-lod` (GAME42)
//!
//! Proves with explicit adversarial vectors and a 1,500-case randomized
//! hostile/malformed fuzz campaign that every applicable CVE/CWE class is
//! impossible without panics, undefined behavior, or partial state corruption:
//!
//! - **CWE-20 / CWE-129 / CWE-190 / CWE-191 / CWE-369 (Numeric & Panic Safety):**
//!   `NaN`, `+/-Inf`, `-0.0`, subnormals (`f64::MIN_POSITIVE * 0.5`, `f64::from_bits(1)`),
//!   `f64::MIN`/`f64::MAX`, `usize::MAX`/`u32::MAX`, degenerate dimensions, steep
//!   `+/-MAX_HEIGHT / MIN_CELL_SIZE` normals, and hostile `Frustum` planes.
//! - **CWE-119 / CWE-120 / CWE-125 / CWE-787 (Memory & Bounds Safety):**
//!   Truncated, empty, oversized, and mismatched slices across `from_heights`,
//!   `from_heights_mut`, `emit_vertices_into`, `emit_mesh_into`, and `mesh_digest_fnv1a`,
//!   plus atomic buffer preservation on every `Err`.
//! - **CWE-400 / CWE-674 / CWE-835 (DoS & Resource Exhaustion):**
//!   Oversized levels, octaves, node/triangle budgets, and atomic scratch invalidation
//!   on `BudgetExceeded`.

#![forbid(unsafe_code)]

use math3d::{Frustum, Math3dError, Plane, Vec3};
use terrain_lod::{
    emit_mesh_into, mesh_digest_fnv1a, refine_into, validate_grid_dimensions, validate_grid_level,
    validate_index_capacity, validate_lod_budget, Heightfield, HeightfieldMut, HeightfieldSpec,
    LodParams, RunwaySpec, TerrainError, TerrainScratch, MAX_CELL_SIZE, MAX_COORD, MAX_HEIGHT,
    MAX_LEVELS, MAX_OCTAVES, MAX_OUTPUT_TRIANGLES, MAX_QUADTREE_NODES, MIN_CELL_SIZE,
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
    let sub_pos = f64::from_bits(1);
    let sub_neg = -f64::from_bits(1);
    let half_min = f64::MIN_POSITIVE * 0.5;
    match next_u64(state) % 20 {
        0 => f64::NAN,
        1 => f64::INFINITY,
        2 => f64::NEG_INFINITY,
        3 => -0.0,
        4 => 0.0,
        5 => sub_pos,
        6 => sub_neg,
        7 => half_min,
        8 => f64::MIN,
        9 => f64::MAX,
        10 => -MAX_HEIGHT * 2.0,
        11 => MAX_HEIGHT * 2.0,
        12 => MAX_COORD * 2.0,
        13 => -MAX_COORD * 2.0,
        14 => MIN_CELL_SIZE * 0.1,
        15 => MAX_CELL_SIZE * 2.0,
        16 => 1e-18,
        17 => -1e-18,
        _ => {
            let u = (next_u64(state) >> 11) as f64 / ((1u64 << 53) as f64);
            -500.0 + u * 1000.0
        }
    }
}

#[test]
fn hostile_numeric_subnormal_extreme_and_canonical_zero() {
    let subnormal_a = f64::from_bits(1);
    let subnormal_b = f64::MIN_POSITIVE * 0.5;
    let hostile_scalars = [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN,
        f64::MAX,
        subnormal_a,
        -subnormal_a,
        subnormal_b,
        -0.0,
        0.0,
    ];

    for &val in &hostile_scalars {
        // Spec construction must never panic on any scalar.
        let _ = HeightfieldSpec::spec_new(3, val, 0.0, 0.0);
        let _ = HeightfieldSpec::spec_new(3, 10.0, val, 0.0);
        let _ = HeightfieldSpec::spec_new(3, 10.0, 0.0, val);
        let _ = RunwaySpec::runway_new(val, 0.0, 0.0, 10.0, 10.0, 1.0, 0.0);
        let _ = RunwaySpec::runway_new(0.0, 0.0, val, 10.0, 10.0, 1.0, 0.0);
        let _ = RunwaySpec::runway_new(0.0, 0.0, 0.0, val, 10.0, 1.0, 0.0);
        let _ = RunwaySpec::runway_new(0.0, 0.0, 0.0, 10.0, 10.0, val, 0.0);
        let _ = RunwaySpec::runway_new(0.0, 0.0, 0.0, 10.0, 10.0, 1.0, val);
        let _ = LodParams::lod_params_new(Vec3::new(val, 10.0, 0.0), 0.02, 1.0, 3, 256, 256, None);
        let _ = LodParams::lod_params_new(Vec3::zero(), val, 1.0, 3, 256, 256, None);
        let _ = LodParams::lod_params_new(Vec3::zero(), 0.02, val, 3, 256, 256, None);
    }

    // Subnormal cell_size below MIN_CELL_SIZE must be rejected.
    assert_eq!(
        HeightfieldSpec::spec_new(2, subnormal_a, 0.0, 0.0),
        Err(TerrainError::InvalidParameter)
    );
    assert_eq!(
        HeightfieldSpec::spec_new(2, subnormal_b, 0.0, 0.0),
        Err(TerrainError::InvalidParameter)
    );
    assert_eq!(
        HeightfieldSpec::spec_new(2, -0.0, 0.0, 0.0),
        Err(TerrainError::InvalidParameter)
    );

    // -0.0 and +0.0 heights must canonicalize to the exact same FNV-1a digest.
    let spec = HeightfieldSpec::spec_new(2, 10.0, 0.0, 0.0).unwrap();
    let count = spec.spec_vertex_count().unwrap();
    let pos_zero = vec![0.0_f64; count];
    let neg_zero = vec![-0.0_f64; count];
    let hf_pos = Heightfield::from_heights(spec, &pos_zero).unwrap();
    let hf_neg = Heightfield::from_heights(spec, &neg_zero).unwrap();
    let mut scratch = TerrainScratch::scratch_with_capacity(256).unwrap();
    let params =
        LodParams::lod_params_new(Vec3::new(0.0, 50.0, 0.0), 0.05, 1.0, 2, 256, 256, None).unwrap();
    let stats = refine_into(&hf_pos, &params, &mut scratch).unwrap();
    let mut idx = vec![0u32; stats.index_count];
    emit_mesh_into(&hf_pos, &scratch, &mut idx, 256).unwrap();
    assert_eq!(
        mesh_digest_fnv1a(&hf_pos, &idx).unwrap(),
        mesh_digest_fnv1a(&hf_neg, &idx).unwrap()
    );

    // Steep cliff (+/-MAX_HEIGHT across MIN_CELL_SIZE) must compute finite unit normals without panic.
    let cliff_spec = HeightfieldSpec::spec_new(1, MIN_CELL_SIZE, 0.0, 0.0).unwrap();
    let mut cliff_h = vec![-MAX_HEIGHT; cliff_spec.spec_vertex_count().unwrap()];
    cliff_h[1] = MAX_HEIGHT;
    cliff_h[4] = MAX_HEIGHT;
    let cliff_hf = Heightfield::from_heights(cliff_spec, &cliff_h).unwrap();
    let n = cliff_hf
        .normal_at(MIN_CELL_SIZE * 0.5, MIN_CELL_SIZE * 0.5)
        .unwrap();
    assert!(n.vec3_is_finite());
    let len = n.length().unwrap();
    assert!((len - 1.0).abs() < 1e-9);
}

#[test]
fn hostile_truncated_mismatched_slices_and_usize_u32_bounds() {
    let spec = HeightfieldSpec::spec_new(2, 10.0, 0.0, 0.0).unwrap(); // 5x5 = 25 vertices
    let count = spec.spec_vertex_count().unwrap();

    // Truncated, empty, and oversized heightfield slices.
    assert_eq!(
        Heightfield::from_heights(spec, &[]).unwrap_err(),
        TerrainError::InvalidGridSize
    );
    assert_eq!(
        Heightfield::from_heights(spec, &vec![0.0; count - 1]).unwrap_err(),
        TerrainError::InvalidGridSize
    );
    assert_eq!(
        Heightfield::from_heights(spec, &vec![0.0; count + 1]).unwrap_err(),
        TerrainError::InvalidGridSize
    );
    let mut short_mut = vec![0.0; count - 1];
    assert_eq!(
        HeightfieldMut::from_heights_mut(spec, &mut short_mut).unwrap_err(),
        TerrainError::InvalidGridSize
    );

    // Non-finite / out-of-range sample inside valid-length slice.
    let mut poisoned = vec![0.0; count];
    poisoned[count - 1] = f64::NAN;
    assert_eq!(
        Heightfield::from_heights(spec, &poisoned).unwrap_err(),
        TerrainError::NonFiniteInput
    );
    poisoned[count - 1] = MAX_HEIGHT + 1.0;
    assert_eq!(
        Heightfield::from_heights(spec, &poisoned).unwrap_err(),
        TerrainError::InvalidParameter
    );

    // emit_vertices_into atomic refusal leaves both output buffers untouched.
    let valid = vec![5.0; count];
    let hf = Heightfield::from_heights(spec, &valid).unwrap();
    let sentinel_v = Vec3::new(-99.0, -88.0, -77.0);
    let mut pos_ok = vec![sentinel_v; count];
    let mut nrm_short = vec![sentinel_v; count - 1];
    assert_eq!(
        hf.emit_vertices_into(&mut pos_ok, &mut nrm_short),
        Err(TerrainError::CapacityExceeded)
    );
    assert!(pos_ok.iter().all(|&v| v == sentinel_v));
    assert!(nrm_short.iter().all(|&v| v == sentinel_v));

    // usize::MAX and u32::MAX boundary guards.
    assert_eq!(
        validate_grid_level(u32::MAX),
        Err(TerrainError::InvalidParameter)
    );
    assert_eq!(
        validate_grid_dimensions(usize::MAX, usize::MAX),
        Err(TerrainError::InvalidGridSize)
    );
    assert_eq!(
        validate_lod_budget(usize::MAX, usize::MAX, usize::MAX, usize::MAX),
        Err(TerrainError::BudgetExceeded)
    );
    assert_eq!(
        validate_index_capacity(usize::MAX, 0),
        Err(TerrainError::CapacityExceeded)
    );
    assert_eq!(
        TerrainScratch::scratch_with_capacity(0).unwrap_err(),
        TerrainError::BudgetExceeded
    );
    assert_eq!(
        TerrainScratch::scratch_with_capacity(usize::MAX).unwrap_err(),
        TerrainError::BudgetExceeded
    );
    assert_eq!(
        hf.height_at_grid(usize::MAX, 0),
        Err(TerrainError::OutOfBounds)
    );
    assert_eq!(
        hf.vertex_position(0, usize::MAX),
        Err(TerrainError::OutOfBounds)
    );
    assert_eq!(
        hf.vertex_normal(usize::MAX, usize::MAX),
        Err(TerrainError::OutOfBounds)
    );
    assert_eq!(
        mesh_digest_fnv1a(&hf, &[0, 1]),
        Err(TerrainError::InvalidParameter)
    );
    assert_eq!(
        mesh_digest_fnv1a(&hf, &[0, 1, u32::MAX]),
        Err(TerrainError::OutOfBounds)
    );

    // Resource caps: octaves > MAX_OCTAVES and max_triangles > MAX_OUTPUT_TRIANGLES.
    let mut valid_mut = vec![0.0; count];
    assert_eq!(
        HeightfieldMut::from_heights_mut(spec, &mut valid_mut)
            .unwrap()
            .synthesize_fbm(1, 0.0, 10.0, 2.0, MAX_OCTAVES + 1, 0.5, 2.0),
        Err(TerrainError::InvalidParameter)
    );
    assert_eq!(
        LodParams::lod_params_new(
            Vec3::zero(),
            0.02,
            1.0,
            2,
            16,
            MAX_OUTPUT_TRIANGLES + 1,
            None,
        ),
        Err(TerrainError::BudgetExceeded)
    );
    let mut scratch = TerrainScratch::scratch_with_capacity(64).unwrap();
    let params_cap = LodParams::lod_params_new(Vec3::zero(), 0.02, 1.0, 2, 128, 64, None).unwrap();
    assert_eq!(
        refine_into(&hf, &params_cap, &mut scratch),
        Err(TerrainError::CapacityExceeded)
    );
    assert_eq!(
        emit_mesh_into(&hf, &scratch, &mut [], MAX_OUTPUT_TRIANGLES + 1),
        Err(TerrainError::InvalidParameter)
    );
}

#[test]
fn hostile_frustum_planes_rejected_atomically() {
    let spec = HeightfieldSpec::spec_new(3, 10.0, 0.0, 0.0).unwrap();
    let mut heights = vec![0.0; spec.spec_vertex_count().unwrap()];
    HeightfieldMut::from_heights_mut(spec, &mut heights)
        .unwrap()
        .synthesize_diamond_square(7, 10.0, 5.0, 0.5)
        .unwrap();
    let hf = Heightfield::from_heights(spec, &heights).unwrap();

    // Prime scratch with a valid refinement first to ensure hostile frustum cleanly invalidates it.
    let mut scratch = TerrainScratch::scratch_with_capacity(512).unwrap();
    let ok_params =
        LodParams::lod_params_new(Vec3::new(20.0, 30.0, 20.0), 0.05, 1.0, 3, 512, 512, None)
            .unwrap();
    let ok_stats = refine_into(&hf, &ok_params, &mut scratch).unwrap();
    assert!(ok_stats.node_count > 0);

    let good_plane = Plane {
        normal: Vec3::unit_y(),
        d: 0.0,
    };
    let hostile_planes = [
        Plane {
            normal: Vec3::new(f64::NAN, 1.0, 0.0),
            d: 0.0,
        },
        Plane {
            normal: Vec3::new(0.0, f64::INFINITY, 0.0),
            d: 0.0,
        },
        Plane {
            normal: Vec3::unit_y(),
            d: f64::NEG_INFINITY,
        },
        Plane {
            normal: Vec3::zero(),
            d: 0.0,
        },
    ];

    for bad_plane in hostile_planes {
        let mut planes = [good_plane; 6];
        planes[2] = bad_plane;
        let hostile_frustum = Frustum { planes };
        let params = LodParams::lod_params_new(
            Vec3::new(20.0, 30.0, 20.0),
            0.05,
            1.0,
            3,
            512,
            512,
            Some(hostile_frustum),
        )
        .unwrap();
        let res = refine_into(&hf, &params, &mut scratch);
        assert!(
            matches!(res, Err(TerrainError::MathError(_))),
            "expected MathError for hostile frustum, got {res:?}"
        );
        assert_eq!(scratch.scratch_node_count(), 0);
        let mut out = [0u32; 16];
        assert_eq!(
            emit_mesh_into(&hf, &scratch, &mut out, 512),
            Err(TerrainError::InvalidParameter)
        );
    }

    // Unused Math3dError import sanity check.
    let _ = TerrainError::MathError(Math3dError::NonFiniteInput);
    let _ = MAX_LEVELS;
    let _ = MAX_QUADTREE_NODES;
}

#[test]
fn seeded_1500_case_hostile_fuzz_campaign() {
    let mut prng = 0xbadc_0ffe_e0dd_f00d_u64;
    let mut scratch = TerrainScratch::scratch_with_capacity(2048).unwrap();
    let mut pos_out = vec![Vec3::new(-1.0, -2.0, -3.0); 1089];
    let mut nrm_out = vec![Vec3::new(-4.0, -5.0, -6.0); 1089];
    let mut idx_out = vec![0xDEAD_BEEF_u32; 8192];

    for case in 0..1500u64 {
        let level = match next_u64(&mut prng) % 6 {
            0 => 0,
            1 => MAX_LEVELS + 1,
            2 => u32::MAX,
            _ => ((next_u64(&mut prng) % 4) + 1) as u32,
        };
        let cell_size = pick_hostile_f64(&mut prng);
        let origin_x = pick_hostile_f64(&mut prng);
        let origin_z = pick_hostile_f64(&mut prng);

        let Ok(spec) = HeightfieldSpec::spec_new(level, cell_size, origin_x, origin_z) else {
            continue;
        };
        let count = spec.spec_vertex_count().unwrap();

        // Mix truncated, oversized, and valid-sized buffers.
        let buf_len = match next_u64(&mut prng) % 5 {
            0 => 0,
            1 => count - 1,
            2 => count + 1,
            _ => count,
        };
        let mut raw = vec![0.0_f64; buf_len];
        for slot in &mut raw {
            if next_u64(&mut prng).is_multiple_of(8) {
                *slot = pick_hostile_f64(&mut prng);
            } else {
                *slot = ((next_u64(&mut prng) % 200) as f64) - 100.0;
            }
        }

        let mut raw_mut = raw.clone();
        if let Ok(mut hfm) = HeightfieldMut::from_heights_mut(spec, &mut raw_mut) {
            let before_synth = hfm.as_heightfield().unwrap().heights().to_vec();
            let synth_res = if case % 2 == 0 {
                hfm.synthesize_diamond_square(
                    next_u64(&mut prng),
                    pick_hostile_f64(&mut prng),
                    pick_hostile_f64(&mut prng),
                    pick_hostile_f64(&mut prng),
                )
            } else {
                hfm.synthesize_fbm(
                    next_u64(&mut prng),
                    pick_hostile_f64(&mut prng),
                    pick_hostile_f64(&mut prng),
                    pick_hostile_f64(&mut prng),
                    (next_u64(&mut prng) % 16) as u32,
                    pick_hostile_f64(&mut prng),
                    pick_hostile_f64(&mut prng),
                )
            };
            if synth_res.is_err() {
                assert_eq!(
                    hfm.as_heightfield().unwrap().heights(),
                    before_synth.as_slice()
                );
            }

            let before_rw = hfm.as_heightfield().unwrap().heights().to_vec();
            if let Ok(rw) = RunwaySpec::runway_new(
                pick_hostile_f64(&mut prng),
                pick_hostile_f64(&mut prng),
                pick_hostile_f64(&mut prng),
                pick_hostile_f64(&mut prng),
                pick_hostile_f64(&mut prng),
                pick_hostile_f64(&mut prng),
                pick_hostile_f64(&mut prng),
            ) {
                if hfm.place_runway(&rw).is_err() {
                    assert_eq!(
                        hfm.as_heightfield().unwrap().heights(),
                        before_rw.as_slice()
                    );
                }
            }
        }

        let Ok(hf) = Heightfield::from_heights(spec, &raw_mut) else {
            continue;
        };

        let qx = pick_hostile_f64(&mut prng);
        let qz = pick_hostile_f64(&mut prng);
        if let Ok(h) = hf.height_at(qx, qz) {
            assert!(h.is_finite());
        }
        if let Ok(n) = hf.normal_at(qx, qz) {
            assert!(n.vec3_is_finite());
        }
        let _ = hf.vertex_normal(
            (next_u64(&mut prng) as usize) % (spec.spec_side() + 2),
            (next_u64(&mut prng) as usize) % (spec.spec_side() + 2),
        );

        let v_cap = match next_u64(&mut prng) % 3 {
            0 => count.saturating_sub(1),
            _ => count,
        };
        let pos_snap = pos_out[..v_cap].to_vec();
        let nrm_snap = nrm_out[..v_cap].to_vec();
        if hf
            .emit_vertices_into(&mut pos_out[..v_cap], &mut nrm_out[..v_cap])
            .is_err()
        {
            assert_eq!(pos_out[..v_cap], pos_snap[..]);
            assert_eq!(nrm_out[..v_cap], nrm_snap[..]);
        }

        let hostile_frustum = if next_u64(&mut prng).is_multiple_of(4) {
            Some(Frustum {
                planes: [Plane {
                    normal: Vec3::new(
                        pick_hostile_f64(&mut prng),
                        pick_hostile_f64(&mut prng),
                        pick_hostile_f64(&mut prng),
                    ),
                    d: pick_hostile_f64(&mut prng),
                }; 6],
            })
        } else {
            None
        };

        let max_nodes = match next_u64(&mut prng) % 5 {
            0 => 0,
            1 => 5,
            2 => usize::MAX,
            _ => 2048,
        };
        let max_tris = match next_u64(&mut prng) % 4 {
            0 => 0,
            1 => usize::MAX,
            _ => 4096,
        };

        if let Ok(params) = LodParams::lod_params_new(
            Vec3::new(
                pick_hostile_f64(&mut prng),
                pick_hostile_f64(&mut prng),
                pick_hostile_f64(&mut prng),
            ),
            pick_hostile_f64(&mut prng),
            pick_hostile_f64(&mut prng),
            ((next_u64(&mut prng) % 12) as u32).max(1),
            max_nodes,
            max_tris,
            hostile_frustum,
        ) {
            match refine_into(&hf, &params, &mut scratch) {
                Ok(stats) => {
                    assert!(scratch.scratch_is_restricted());
                    let idx_cap = match next_u64(&mut prng) % 3 {
                        0 => stats.index_count.saturating_sub(1),
                        _ => idx_out.len(),
                    };
                    let idx_snap = idx_out[..idx_cap].to_vec();
                    match emit_mesh_into(&hf, &scratch, &mut idx_out[..idx_cap], max_tris) {
                        Ok(written) => {
                            assert_eq!(written, stats.index_count);
                            assert!(mesh_digest_fnv1a(&hf, &idx_out[..written]).is_ok());
                        }
                        Err(_) => {
                            assert_eq!(idx_out[..idx_cap], idx_snap[..]);
                        }
                    }
                }
                Err(TerrainError::CapacityExceeded) => {}
                Err(_) => {
                    assert_eq!(scratch.scratch_node_count(), 0);
                }
            }
        }
    }
}
