//! # Microbenchmark Suite & Zero-Allocation Verification (VER07) for `collision2d`
//!
//! Validates:
//! - Hard zero-heap-allocation budgets across all hot collision queries:
//!   `aabb_vs_aabb`, `circle_vs_aabb`, `obb_vs_obb_sat`, `sweep_circle_aabb`.
//! - Verified on the real heap via `CountingAllocator`.
//! - Sabotage detection: intentional dynamic memory allocation is caught and rejected.

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
use collision2d::{
    aabb_vs_aabb, circle_vs_aabb, obb_vs_obb_sat, sweep_circle_aabb, Circle, Obb2D, Point, Rect,
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
        warmup_iterations: 10,
        warmup_max_duration: Duration::from_millis(1),
        min_batch_duration: Duration::from_micros(10),
        max_batch_size: 100_000,
        sample_count: samples,
        seed: 0x9e37_79b9_7f4a_7c15,
    }
}

fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

fn check_aabb_vs_aabb_zero_alloc(_ctx: &Ctx) -> Verdict {
    let a = Rect::new(0.0, 0.0, 10.0, 10.0);
    let b = Rect::new(8.0, 2.0, 10.0, 10.0);

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(64),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("aabb_vs_aabb_zero_alloc", &budget, &alloc, || {
        let r = aabb_vs_aabb(black_box(a), black_box(b));
        black_box(r).expect("query should succeed");
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "aabb_vs_aabb exceeded zero allocation budget",
        );
    }
    Verdict::Pass
}

fn check_circle_vs_aabb_zero_alloc(_ctx: &Ctx) -> Verdict {
    let circle = Circle::new(Point::new(-2.0, 5.0), 4.0).unwrap();
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(64),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("circle_vs_aabb_zero_alloc", &budget, &alloc, || {
        let r = circle_vs_aabb(black_box(circle), black_box(rect));
        black_box(r).expect("query should succeed");
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "circle_vs_aabb exceeded zero allocation budget",
        );
    }
    Verdict::Pass
}

fn check_obb_vs_obb_sat_zero_alloc(_ctx: &Ctx) -> Verdict {
    let a = Obb2D::new(Point::new(0.0, 0.0), Point::new(5.0, 5.0), 0.3).unwrap();
    let b = Obb2D::new(Point::new(6.0, 2.0), Point::new(4.0, 4.0), 0.7).unwrap();

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(256),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("obb_vs_obb_sat_zero_alloc", &budget, &alloc, || {
        let r = obb_vs_obb_sat(black_box(a), black_box(b));
        black_box(r).expect("query should succeed");
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "obb_vs_obb_sat exceeded zero allocation budget",
        );
    }
    Verdict::Pass
}

fn check_sweep_circle_aabb_zero_alloc(_ctx: &Ctx) -> Verdict {
    let circle = Circle::new(Point::new(0.0, 100.0), 4.0).unwrap();
    let velocity = Point::new(500.0, 0.0);
    let paddle = Rect::new(200.0, 80.0, 8.0, 40.0);

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(256),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("sweep_circle_aabb_zero_alloc", &budget, &alloc, || {
        let r = sweep_circle_aabb(black_box(circle), black_box(velocity), black_box(paddle));
        black_box(r).expect("query should succeed");
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "sweep_circle_aabb exceeded zero allocation budget",
        );
    }
    Verdict::Pass
}

fn check_sabotage_allocation_detected(_ctx: &Ctx) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(5));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        ..Default::default()
    };
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
        ("module", Json::str("collision2d")),
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
        "bench/aabb_vs_aabb/zero_alloc",
        check_aabb_vs_aabb_zero_alloc,
    );
    suite.add(
        "bench/circle_vs_aabb/zero_alloc",
        check_circle_vs_aabb_zero_alloc,
    );
    suite.add(
        "bench/obb_vs_obb_sat/zero_alloc",
        check_obb_vs_obb_sat_zero_alloc,
    );
    suite.add(
        "bench/sweep_circle_aabb/zero_alloc",
        check_sweep_circle_aabb_zero_alloc,
    );
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
