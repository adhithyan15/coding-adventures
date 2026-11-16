//! # Microbenchmark Suite and Zero-Allocation Verification for `swf-shape-to-paint`
//!
//! Validates via `test-bootstrap`'s counting global allocator:
//! - zero heap allocation in twips coordinate conversions and checked math,
//! - zero heap allocation in quadratic Bézier curve evaluation and bounding box calculation,
//! - zero heap allocation in edge reversal and coordinate projection,
//! - sabotage detection: an intentional allocation is caught and rejected.

#![deny(unsafe_code)]

use std::hint::black_box;
use std::time::Duration;

use coding_adventures_test_bootstrap::bench::{
    AllocCounter, BenchmarkConfig, BenchmarkHarness, BenchmarkResult, Budget,
};
use coding_adventures_test_bootstrap::counting_alloc;
use coding_adventures_test_bootstrap::manifest::{self, Json};
use coding_adventures_test_bootstrap::run::{self, Ctx, Outcome, Suite};
use coding_adventures_test_bootstrap::Verdict;

use swf_shape_to_paint::{
    checked_twips_add, checked_twips_point_add, evaluate_quad_edge, pixels_to_twips_i32,
    quad_edge_bounding_box, twips_point_to_pixels, twips_to_pixels_f64, ShapeEdge,
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

fn zero_budget() -> Budget {
    Budget {
        max_alloc_bytes: Some(0),
        ..Default::default()
    }
}

fn bench_zero_alloc(name: &str, f: impl FnMut()) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(20));
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

fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

fn check_twips_zero_alloc(_ctx: &Ctx) -> Verdict {
    bench_zero_alloc("twips_zero_alloc", || {
        let px = twips_to_pixels_f64(black_box(1440));
        let tw = pixels_to_twips_i32(black_box(px));
        let pt = twips_point_to_pixels(black_box(tw), black_box(tw));
        let added = checked_twips_add(black_box(tw), black_box(100)).unwrap_or(0);
        let pt_added =
            checked_twips_point_add(black_box(tw), black_box(tw), black_box(10), black_box(20))
                .unwrap_or((0, 0));
        black_box((px, tw, pt, added, pt_added));
    })
}

fn check_bezier_eval_zero_alloc(_ctx: &Ctx) -> Verdict {
    let edge = ShapeEdge {
        start: (100, 200),
        end: (500, 600),
        control: Some((300, 800)),
        fill_style0: 1,
        fill_style1: 2,
        line_style: 1,
    };

    bench_zero_alloc("bezier_eval_zero_alloc", || {
        let p_start = evaluate_quad_edge(black_box(&edge), black_box(0.0));
        let p_mid = evaluate_quad_edge(black_box(&edge), black_box(0.5));
        let p_end = evaluate_quad_edge(black_box(&edge), black_box(1.0));
        let bbox = quad_edge_bounding_box(black_box(&edge));
        black_box((p_start, p_mid, p_end, bbox));
    })
}

fn check_edge_helpers_zero_alloc(_ctx: &Ctx) -> Verdict {
    let edge = ShapeEdge {
        start: (0, 0),
        end: (200, 400),
        control: Some((100, 200)),
        fill_style0: 1,
        fill_style1: 2,
        line_style: 3,
    };

    bench_zero_alloc("edge_helpers_zero_alloc", || {
        let rev = black_box(&edge).reverse_edge_direction();
        let s = rev.edge_start_pixels();
        let e = rev.edge_end_pixels();
        let c = rev.edge_control_pixels();
        let curved = rev.is_curved_edge();
        black_box((s, e, c, curved));
    })
}

fn check_sabotage_allocation_detected(_ctx: &Ctx) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(5));
    let budget = zero_budget();
    let alloc = AllocCounter::new();
    let res = harness.run_with_counter("sabotage_allocation", &budget, &alloc, || {
        let v: Vec<u8> = vec![1, 2, 3, 4];
        black_box(v);
    });

    if zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "sabotage allocation was not detected by harness",
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
        ("module", Json::str("swf_shape_to_paint")),
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
    suite.add("bench/twips/zero_alloc", check_twips_zero_alloc);
    suite.add("bench/bezier/zero_alloc", check_bezier_eval_zero_alloc);
    suite.add("bench/edge/zero_alloc", check_edge_helpers_zero_alloc);
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
