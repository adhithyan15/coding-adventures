//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `bsp-portal25d`
//!
//! Validates, on the real heap via `coding_adventures_test_bootstrap`'s
//! `CountingAllocator`:
//! - zero heap allocation in `BspTree::bsp_find_subsector`
//! - zero heap allocation in steady-state `render_frame_into` (solid multi-sector portal scene)
//! - zero heap allocation in steady-state `render_frame_into` (textured multi-sector portal scene)
//! - sabotage detection: an intentional allocation is caught and rejected.

#![deny(unsafe_code)]

use std::hint::black_box;
use std::time::Duration;

use bsp_portal25d::{
    render_frame_into, BspTree, Camera25D, Linedef, Map25D, RenderConfig, RenderScratch, Sector,
    SurfaceStyle, TextureRgba, Vec2,
};
use coding_adventures_test_bootstrap::bench::{
    AllocCounter, BenchmarkConfig, BenchmarkHarness, BenchmarkResult, Budget, BudgetVerdict,
};
use coding_adventures_test_bootstrap::counting_alloc;
use coding_adventures_test_bootstrap::manifest::{self, Json};
use coding_adventures_test_bootstrap::run::{self, Ctx, Outcome, Suite};
use coding_adventures_test_bootstrap::Verdict;
use pixel_container::PixelContainer;

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

fn v(x: f64, y: f64) -> Vec2 {
    Vec2::vec2_new(x, y)
}

fn build_portal_map(textured: bool) -> Map25D {
    let (f0, c0, f1, c1, w0, w1, up, lo) = if textured {
        (
            SurfaceStyle::surface_textured(1, [40, 80, 40, 255]),
            SurfaceStyle::surface_textured(2, [40, 40, 90, 255]),
            SurfaceStyle::surface_textured(1, [70, 110, 70, 255]),
            SurfaceStyle::surface_textured(2, [70, 70, 120, 255]),
            SurfaceStyle::surface_textured(0, [180, 60, 60, 255]),
            SurfaceStyle::surface_textured(0, [60, 180, 180, 255]),
            SurfaceStyle::surface_textured(0, [220, 200, 60, 255]),
            SurfaceStyle::surface_textured(0, [200, 100, 220, 255]),
        )
    } else {
        (
            SurfaceStyle::surface_solid([40, 80, 40, 255]),
            SurfaceStyle::surface_solid([40, 40, 90, 255]),
            SurfaceStyle::surface_solid([70, 110, 70, 255]),
            SurfaceStyle::surface_solid([70, 70, 120, 255]),
            SurfaceStyle::surface_solid([180, 60, 60, 255]),
            SurfaceStyle::surface_solid([60, 180, 180, 255]),
            SurfaceStyle::surface_solid([220, 200, 60, 255]),
            SurfaceStyle::surface_solid([200, 100, 220, 255]),
        )
    };

    let s0 = Sector::sector_with_surfaces(0.0, 4.0, f0, c0, 255).unwrap();
    let s1 = Sector::sector_with_surfaces(1.0, 3.0, f1, c1, 180).unwrap();

    let linedefs = vec![
        Linedef::linedef_solid(v(0.0, 0.0), v(0.0, 10.0), 0, w0).unwrap(),
        Linedef::linedef_solid(v(0.0, 10.0), v(10.0, 10.0), 0, w0).unwrap(),
        Linedef::linedef_portal(v(10.0, 10.0), v(10.0, 0.0), 0, Some(1), w0, up, lo).unwrap(),
        Linedef::linedef_solid(v(10.0, 0.0), v(0.0, 0.0), 0, w0).unwrap(),
        Linedef::linedef_solid(v(10.0, 10.0), v(20.0, 10.0), 1, w1).unwrap(),
        Linedef::linedef_solid(v(20.0, 10.0), v(20.0, 0.0), 1, w1).unwrap(),
        Linedef::linedef_solid(v(20.0, 0.0), v(10.0, 0.0), 1, w1).unwrap(),
    ];
    Map25D::map_new(vec![s0, s1], linedefs).unwrap()
}

fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

fn check_bsp_find_subsector_zero_alloc(_ctx: &Ctx) -> Verdict {
    let map = build_portal_map(false);
    let bsp = BspTree::bsp_build(&map).unwrap();
    bench_zero_alloc("bsp_find_subsector_zero_alloc", || {
        let leaf = bsp
            .bsp_find_subsector(black_box(v(4.0, 5.0)))
            .expect("find_subsector should succeed");
        black_box(leaf);
    })
}

fn check_render_frame_solid_portal_zero_alloc(_ctx: &Ctx) -> Verdict {
    let map = build_portal_map(false);
    let bsp = BspTree::bsp_build(&map).unwrap();
    let cam = Camera25D::camera_from_yaw_fov(5.0, 5.0, 2.0, 0.0, trig::PI / 3.0).unwrap();
    let cfg = RenderConfig::render_config_default();
    let mut fb = PixelContainer::new(160, 120);
    let mut scratch = RenderScratch::scratch_for_frame_and_map(&fb, &map, &bsp).unwrap();

    bench_zero_alloc("render_frame_solid_portal_zero_alloc", || {
        let stats = render_frame_into(
            black_box(&map),
            black_box(&bsp),
            black_box(&cam),
            black_box(&[]),
            black_box(&cfg),
            &mut scratch,
            &mut fb,
        )
        .expect("render_frame_into should succeed");
        black_box(stats);
    })
}

fn check_render_frame_textured_portal_zero_alloc(_ctx: &Ctx) -> Verdict {
    let map = build_portal_map(true);
    let bsp = BspTree::bsp_build(&map).unwrap();
    let cam = Camera25D::camera_from_yaw_fov(5.0, 5.0, 2.0, 0.15, trig::PI / 3.0).unwrap();
    let cfg = RenderConfig::render_config_default();
    let textures = [
        TextureRgba::texture_checkerboard(16, 16, 4, [200, 100, 60, 255], [120, 60, 40, 255])
            .unwrap(),
        TextureRgba::texture_checkerboard(16, 16, 4, [60, 140, 60, 255], [40, 90, 40, 255])
            .unwrap(),
        TextureRgba::texture_checkerboard(16, 16, 4, [40, 40, 160, 255], [20, 20, 90, 255])
            .unwrap(),
    ];
    let mut fb = PixelContainer::new(160, 120);
    let mut scratch = RenderScratch::scratch_for_frame_and_map(&fb, &map, &bsp).unwrap();

    bench_zero_alloc("render_frame_textured_portal_zero_alloc", || {
        let stats = render_frame_into(
            black_box(&map),
            black_box(&bsp),
            black_box(&cam),
            black_box(&textures),
            black_box(&cfg),
            &mut scratch,
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
        ("module", Json::str("bsp-portal25d")),
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
        "bench/bsp_find_subsector/zero_alloc",
        check_bsp_find_subsector_zero_alloc,
    );
    suite.add(
        "bench/render_frame_solid_portal_scene/zero_alloc",
        check_render_frame_solid_portal_zero_alloc,
    );
    suite.add(
        "bench/render_frame_textured_portal_scene/zero_alloc",
        check_render_frame_textured_portal_zero_alloc,
    );
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
