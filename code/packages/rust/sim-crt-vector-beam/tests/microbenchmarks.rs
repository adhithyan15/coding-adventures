//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `sim-crt-vector-beam`
//!
//! Evaluates `sim-crt-vector-beam` under the Level 5 Microbenchmark Harness
//! (`coding_adventures_test_bootstrap::bench`).
//!
//! Validates:
//! - Hard zero-heap-allocation budgets across raster CRT beam stepping (`step_raster_beam`),
//!   vector deflection amplifier stepping (`step_vector_beam`), phosphor buffer decay (`decay_phosphor_buffer`),
//!   and stroke rasterization (`rasterize_vector_stroke_into`).
//! - Counting allocator liveness and active heap metering.
//! - Sabotage detection: intentional dynamic memory allocation is detected and rejected.

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
use sim_crt_vector_beam::{
    decay_phosphor_buffer, CrtRasterBeam, PhosphorType, RasterTiming, VectorBeamDisplay,
    VectorStroke,
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

fn check_step_raster_beam_zero_alloc(_ctx: &Ctx) -> Verdict {
    let timing = RasterTiming::standard_arcade();
    let mut crt = CrtRasterBeam::new_raster_beam(timing, PhosphorType::P4White)
        .expect("raster beam creation should succeed");
    let mut phosphor = vec![0u16; timing.total_raster_pixels()];

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(128),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter(
        "step_raster_beam_zero_alloc",
        &budget,
        &alloc,
        || {
            let r = crt.step_raster_beam(black_box(200), black_box(&mut phosphor));
            black_box(r).expect("step_raster_beam should succeed");
        },
    );

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "step_raster_beam exceeded zero allocation budget",
        );
    }
    Verdict::Pass
}

fn check_step_vector_beam_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut disp =
        VectorBeamDisplay::new_vector_display(1024.0, 768.0, 25.0, PhosphorType::P31Green)
            .expect("vector display creation should succeed");

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(128),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter(
        "step_vector_beam_zero_alloc",
        &budget,
        &alloc,
        || {
            let r = disp.step_vector_beam(
                black_box(500.0),
                black_box(400.0),
                black_box(false),
                black_box(255),
            );
            black_box(r).expect("step_vector_beam should succeed");
        },
    );

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "step_vector_beam exceeded zero allocation budget",
        );
    }
    Verdict::Pass
}

fn check_rasterize_vector_stroke_into_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut disp =
        VectorBeamDisplay::new_vector_display(100.0, 100.0, 20.0, PhosphorType::P31Green)
            .expect("vector display creation should succeed");
    let mut phosphor = vec![0u16; 100 * 100];
    let stroke = VectorStroke::new_vector_stroke(10.0, 10.0, 50.0, 50.0, 200);

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(2048),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter(
        "rasterize_vector_stroke_into_zero_alloc",
        &budget,
        &alloc,
        || {
            let r = disp.rasterize_vector_stroke_into(
                black_box(&stroke),
                black_box(&mut phosphor),
                black_box(100),
                black_box(100),
            );
            black_box(r).expect("rasterize_vector_stroke_into should succeed");
        },
    );

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "rasterize_vector_stroke_into exceeded zero allocation budget",
        );
    }
    Verdict::Pass
}

fn check_decay_phosphor_buffer_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut phosphor = vec![30000u16; 1024];

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(2048),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter(
        "decay_phosphor_buffer_zero_alloc",
        &budget,
        &alloc,
        || {
            let r = decay_phosphor_buffer(black_box(&mut phosphor), black_box(45056));
            black_box(r).expect("decay_phosphor_buffer should succeed");
        },
    );

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "decay_phosphor_buffer exceeded zero allocation budget",
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
        ("module", Json::str("sim_crt_vector_beam")),
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
        "bench/step_raster_beam/zero_alloc",
        check_step_raster_beam_zero_alloc,
    );
    suite.add(
        "bench/step_vector_beam/zero_alloc",
        check_step_vector_beam_zero_alloc,
    );
    suite.add(
        "bench/rasterize_vector_stroke_into/zero_alloc",
        check_rasterize_vector_stroke_into_zero_alloc,
    );
    suite.add(
        "bench/decay_phosphor_buffer/zero_alloc",
        check_decay_phosphor_buffer_zero_alloc,
    );
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
