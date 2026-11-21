//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `raster3d-soft`
//!
//! Validates on the real heap via `coding_adventures_test_bootstrap`'s
//! `CountingAllocator`:
//! - zero heap allocation in steady-state `draw_triangle_into`
//! - zero heap allocation in steady-state `draw_indexed_mesh_into` (`f64` Z-buffer)
//! - zero heap allocation in steady-state `draw_indexed_mesh_into` (`f32` Z-buffer + bilinear texture)
//! - zero heap allocation in `clear_framebuffer`, `clear_depth_f64`, and `frame_digest_fnv1a`
//! - sabotage detection: an intentional allocation is caught and rejected.

#![deny(unsafe_code)]

use std::hint::black_box;
use std::time::Duration;

use coding_adventures_test_bootstrap::bench::{
    AllocCounter, BenchmarkConfig, BenchmarkHarness, BenchmarkResult, Budget, BudgetVerdict,
};
use coding_adventures_test_bootstrap::counting_alloc;
use coding_adventures_test_bootstrap::manifest::{self, Json};
use coding_adventures_test_bootstrap::run::{self, Ctx, Outcome, Suite};
use coding_adventures_test_bootstrap::Verdict;
use math3d::{Mat4, Quat, Vec3};
use pixel_container::PixelContainer;
use raster3d_soft::{
    clear_depth_f32, clear_depth_f64, clear_framebuffer, draw_indexed_mesh_into,
    draw_triangle_into, frame_digest_fnv1a, DepthBufferMut, RasterConfig, RasterScratch,
    ShadingMode, Texture, Vertex,
};

const FID_BENCH_FAIL: &str = "property-counterexample";

coding_adventures_test_bootstrap::install_counting_allocator!();

fn zero_alloc_ok(result: &BenchmarkResult) -> bool {
    result.budget_verdict.is_acceptable()
        && result.alloc_bytes_per_iter == Some(0.0)
        && !result.unchecked_hard_budgets.contains(&"alloc_bytes")
}

fn fast_bench_config(samples: usize) -> BenchmarkConfig {
    BenchmarkConfig {
        warmup_iterations: 5,
        warmup_max_duration: Duration::from_millis(1),
        min_batch_duration: Duration::from_micros(10),
        max_batch_size: 1000,
        sample_count: samples,
        seed: 0x9e37_79b9_7f4a_7c15,
    }
}

fn zero_budget() -> Budget {
    Budget {
        max_alloc_bytes: Some(0),
        ..Default::default()
    }
}

fn bench_zero_alloc(name: &str, f: impl FnMut()) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(15));
    let budget = zero_budget();
    let alloc = AllocCounter::new();
    let res = harness.run_with_counter(name, &budget, &alloc, f);
    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            format!("{name} exceeded zero allocation budget"),
        );
    }
    Verdict::Pass
}

fn build_cube_mesh() -> (Vec<Vertex>, Vec<u32>) {
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
    let mut verts = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    let uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
    for (quad, n, col) in faces {
        let base = verts.len() as u32;
        for (i, &vi) in quad.iter().enumerate() {
            verts.push(Vertex::vertex_new(p[vi], n, col, uvs[i][0], uvs[i][1]));
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (verts, indices)
}

fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

fn check_draw_triangle_into_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut fb = PixelContainer::new(64, 64);
    let mut zb = vec![1.0_f64; 64 * 64];
    let cfg = RasterConfig::raster_config_new(Mat4::mat4_identity()).unwrap();
    let v0 = Vertex::vertex_new(
        Vec3::new(-0.6, -0.6, 0.0),
        Vec3::unit_z(),
        [1.0, 0.0, 0.0],
        0.0,
        0.0,
    );
    let v1 = Vertex::vertex_new(
        Vec3::new(0.6, -0.6, 0.0),
        Vec3::unit_z(),
        [0.0, 1.0, 0.0],
        1.0,
        0.0,
    );
    let v2 = Vertex::vertex_new(
        Vec3::new(0.0, 0.6, 0.0),
        Vec3::unit_z(),
        [0.0, 0.0, 1.0],
        0.5,
        1.0,
    );

    bench_zero_alloc("draw_triangle_into_zero_alloc", || {
        clear_depth_f64(black_box(&mut zb), 1.0).expect("clear_depth_f64 should succeed");
        let stats = draw_triangle_into(
            black_box(&mut fb),
            DepthBufferMut::F64(&mut zb),
            black_box(&cfg),
            None,
            black_box(&v0),
            black_box(&v1),
            black_box(&v2),
        )
        .expect("draw_triangle_into should succeed");
        black_box(stats);
    })
}

fn check_draw_indexed_mesh_f64_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut fb = PixelContainer::new(64, 64);
    let mut zb = vec![1.0_f64; 64 * 64];
    let (verts, indices) = build_cube_mesh();
    let mut scratch = RasterScratch::scratch_with_capacity(verts.len()).unwrap();

    let model = Mat4::mat4_from_trs(
        Vec3::zero(),
        Quat::from_euler_yxz(0.6, 0.3, 0.1).unwrap(),
        Vec3::new(1.0, 1.0, 1.0),
    )
    .unwrap();
    let view = Mat4::look_at(Vec3::new(0.0, 0.5, 4.0), Vec3::zero(), Vec3::unit_y()).unwrap();
    let proj = Mat4::perspective(std::f64::consts::FRAC_PI_3, 1.0, 0.5, 20.0).unwrap();
    let mut cfg =
        RasterConfig::raster_config_new(proj.mul(&view).unwrap().mul(&model).unwrap()).unwrap();
    cfg.normal_matrix = model.upper_mat3().unwrap();

    bench_zero_alloc("draw_indexed_mesh_f64_zero_alloc", || {
        clear_framebuffer(black_box(&mut fb), [0, 0, 0, 255]).expect("clear_fb should succeed");
        clear_depth_f64(black_box(&mut zb), 1.0).expect("clear_depth should succeed");
        let stats = draw_indexed_mesh_into(
            black_box(&mut fb),
            DepthBufferMut::F64(&mut zb),
            black_box(&cfg),
            None,
            black_box(&verts),
            black_box(&indices),
            black_box(&mut scratch),
        )
        .expect("draw_indexed_mesh_into should succeed");
        black_box(stats);
    })
}

fn check_draw_indexed_mesh_f32_textured_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut fb = PixelContainer::new(64, 64);
    let mut zb = vec![1.0_f32; 64 * 64];
    let (verts, indices) = build_cube_mesh();
    let mut scratch = RasterScratch::scratch_with_capacity(verts.len()).unwrap();

    let mut tex_pc = PixelContainer::new(16, 16);
    tex_pc.fill(200, 150, 100, 255);
    let tex = Texture::texture_from_container(&tex_pc).unwrap();

    let view = Mat4::look_at(Vec3::new(1.0, 1.0, 4.0), Vec3::zero(), Vec3::unit_y()).unwrap();
    let proj = Mat4::perspective(std::f64::consts::FRAC_PI_3, 1.0, 0.5, 20.0).unwrap();
    let mut cfg = RasterConfig::raster_config_new(proj.mul(&view).unwrap()).unwrap();
    cfg.shading = ShadingMode::TexturedBilinear;

    bench_zero_alloc("draw_indexed_mesh_f32_textured_zero_alloc", || {
        clear_depth_f32(black_box(&mut zb), 1.0).expect("clear_depth_f32 should succeed");
        let stats = draw_indexed_mesh_into(
            black_box(&mut fb),
            DepthBufferMut::F32(&mut zb),
            black_box(&cfg),
            Some(black_box(&tex)),
            black_box(&verts),
            black_box(&indices),
            black_box(&mut scratch),
        )
        .expect("draw_indexed_mesh_into textured should succeed");
        black_box(stats);
    })
}

fn check_frame_digest_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut fb = PixelContainer::new(64, 64);
    fb.fill(12, 34, 56, 255);
    bench_zero_alloc("frame_digest_fnv1a_zero_alloc", || {
        let d = frame_digest_fnv1a(black_box(&fb));
        black_box(d);
    })
}

fn check_sabotage_allocation_detected(_ctx: &Ctx) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(5));
    let budget = zero_budget();
    let counter = AllocCounter::new();
    let leaky = harness.run_with_counter("sabotage_alloc", &budget, &counter, || {
        black_box(Vec::<u8>::with_capacity(1))
    });
    if !matches!(
        leaky.budget_verdict,
        BudgetVerdict::Failed {
            observed: 1,
            limit: 0,
            ..
        }
    ) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "a closure allocating one byte did not fail the zero-allocation budget",
        );
    }
    let clean = harness.run_with_counter("clean_alloc", &budget, &counter, || {
        black_box(1u64) ^ black_box(2u64)
    });
    if !zero_alloc_ok(&clean) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "a non-allocating closure did not pass the zero-allocation budget",
        );
    }
    Verdict::Pass
}

fn write_bench_manifest(outcomes: &[Outcome]) {
    let Some(path) = manifest::output_path("WARRANT_BENCH_OUT") else {
        return;
    };
    let mut tests: Vec<Json> = outcomes
        .iter()
        .map(|o| {
            Json::obj([
                ("id", Json::str(o.id.clone())),
                ("kind", Json::str("bench")),
                (
                    "verdict",
                    Json::str(if o.passed() { "pass" } else { "fail" }),
                ),
            ])
        })
        .collect();
    tests.sort_by_key(|t| t.render());
    let doc = Json::obj([
        ("schema", Json::str("warrant-bench-evidence-v1")),
        ("module", Json::str("raster3d-soft")),
        ("hw_instruction_counters", Json::str("not-live")),
        (
            "source_sha256",
            Json::str(coding_adventures_sha256::sha256_hex(
                &manifest::source_tree_framing(
                    &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
                )
                .expect("bind bench manifest to src"),
            )),
        ),
        ("tests", Json::Arr(tests)),
    ]);
    manifest::write(&path, &doc).expect("write bench manifest");
}

fn main() {
    let mut suite = Suite::new();

    suite.add(
        "bench/alloc/counting_allocator_is_live",
        check_counting_allocator_is_live,
    );
    suite.add(
        "bench/draw_triangle_into/zero_alloc",
        check_draw_triangle_into_zero_alloc,
    );
    suite.add(
        "bench/draw_indexed_mesh_f64/zero_alloc",
        check_draw_indexed_mesh_f64_zero_alloc,
    );
    suite.add(
        "bench/draw_indexed_mesh_f32_textured/zero_alloc",
        check_draw_indexed_mesh_f32_textured_zero_alloc,
    );
    suite.add(
        "bench/frame_digest_fnv1a/zero_alloc",
        check_frame_digest_zero_alloc,
    );
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
