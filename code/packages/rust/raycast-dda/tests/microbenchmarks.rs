//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `raycast-dda`
//!
//! Validates, on the real heap via `coding_adventures_test_bootstrap`'s
//! `CountingAllocator`:
//! - zero heap allocation in `cast_dda_ray`
//! - zero heap allocation in steady-state `render_frame_into` (solid planes + walls + sprites)
//! - zero heap allocation in steady-state `render_frame_into` (textured planes + walls + sprites)
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
use pixel_container::PixelContainer;
use raycast_dda::{
    cast_dda_ray, render_frame_into, Camera2D, GridMap, PlaneSurface, RenderConfig, Sprite,
    SpriteScratch, TextureRgba,
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
        max_batch_size: 10_000,
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

fn build_test_map() -> GridMap {
    let mut cells = vec![0u16; 16 * 16];
    for i in 0..16 {
        cells[i] = 1;
        cells[15 * 16 + i] = 1;
        cells[i * 16] = 1;
        cells[i * 16 + 15] = 1;
    }
    cells[8 * 16 + 8] = 1;
    GridMap::grid_new(16, 16, cells).unwrap()
}

fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

fn check_cast_dda_ray_zero_alloc(_ctx: &Ctx) -> Verdict {
    let map = build_test_map();
    bench_zero_alloc("cast_dda_ray_zero_alloc", || {
        let hit = cast_dda_ray(
            black_box(&map),
            black_box(3.5),
            black_box(3.5),
            black_box(0.8),
            black_box(0.6),
        )
        .expect("cast_dda_ray should succeed");
        black_box(hit);
    })
}

fn check_render_frame_solid_planes_zero_alloc(_ctx: &Ctx) -> Verdict {
    let map = build_test_map();
    let cam = Camera2D::camera_from_yaw_fov(3.5, 3.5, 0.4, trig::PI / 3.0).unwrap();
    let wall =
        TextureRgba::texture_checkerboard(16, 16, 4, [200, 120, 80, 255], [120, 60, 40, 255])
            .unwrap();
    let sprite_tex =
        TextureRgba::texture_checkerboard(16, 16, 8, [255, 255, 0, 255], [0, 0, 0, 0]).unwrap();
    let textures = [wall, sprite_tex];
    let sprites = [
        Sprite::sprite_new(6.5, 4.5, 1).unwrap(),
        Sprite::sprite_new(5.5, 4.2, 1).unwrap(),
        Sprite::sprite_new(7.5, 5.0, 1).unwrap(),
    ];
    let config = RenderConfig::render_config_default();
    let mut scratch = SpriteScratch::scratch_with_capacity(8).unwrap();
    let mut zbuf = vec![0.0; 160];
    let mut fb = PixelContainer::new(160, 120);

    bench_zero_alloc("render_frame_solid_planes_zero_alloc", || {
        let stats = render_frame_into(
            black_box(&map),
            black_box(&cam),
            black_box(&textures),
            black_box(&sprites),
            black_box(&config),
            &mut scratch,
            &mut zbuf,
            &mut fb,
        )
        .expect("render_frame_into should succeed");
        black_box(stats);
    })
}

fn check_render_frame_textured_planes_zero_alloc(_ctx: &Ctx) -> Verdict {
    let map = build_test_map();
    let cam = Camera2D::camera_from_yaw_fov(4.5, 4.5, 0.7, trig::PI / 3.0).unwrap();
    let wall = TextureRgba::texture_solid(16, 16, [180, 80, 80, 255]).unwrap();
    let floor = TextureRgba::texture_checkerboard(16, 16, 4, [60, 140, 60, 255], [40, 90, 40, 255])
        .unwrap();
    let ceil = TextureRgba::texture_checkerboard(16, 16, 4, [40, 40, 160, 255], [20, 20, 90, 255])
        .unwrap();
    let sprite_tex = TextureRgba::texture_solid(8, 8, [255, 200, 50, 255]).unwrap();
    let textures = [wall, floor, ceil, sprite_tex];
    let sprites = [
        Sprite::sprite_new(6.0, 6.0, 3).unwrap(),
        Sprite::sprite_new(5.5, 5.5, 3).unwrap(),
    ];
    let config = RenderConfig::render_config_with_planes(
        PlaneSurface::Textured { texture_index: 2 },
        PlaneSurface::Textured { texture_index: 1 },
        128,
    );
    let mut scratch = SpriteScratch::scratch_with_capacity(8).unwrap();
    let mut zbuf = vec![0.0; 128];
    let mut fb = PixelContainer::new(128, 96);

    bench_zero_alloc("render_frame_textured_planes_zero_alloc", || {
        let stats = render_frame_into(
            black_box(&map),
            black_box(&cam),
            black_box(&textures),
            black_box(&sprites),
            black_box(&config),
            &mut scratch,
            &mut zbuf,
            &mut fb,
        )
        .expect("render_frame_into should succeed");
        black_box(stats);
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
        ("module", Json::str("raycast-dda")),
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
        "bench/cast_dda_ray/zero_alloc",
        check_cast_dda_ray_zero_alloc,
    );
    suite.add(
        "bench/render_frame_solid_planes/zero_alloc",
        check_render_frame_solid_planes_zero_alloc,
    );
    suite.add(
        "bench/render_frame_textured_planes/zero_alloc",
        check_render_frame_textured_planes_zero_alloc,
    );
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
