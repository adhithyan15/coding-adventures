//! # Microbenchmark Suite and Zero-Allocation Verification (VER07)
//!
//! Evaluates `lunar-lander` under the Level 5 Microbenchmark Harness
//! (`coding_adventures_test_bootstrap::bench`).
//!
//! Validates:
//! - Hard zero-heap-allocation budgets across all flight transitions (`step_burn`),
//!   burn validations (`validate_burn`), and mission initializations (`new_flight`),
//!   measured on the real heap by test-bootstrap's counting global allocator.
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
use lunar_lander::{isqrt_u128, new_flight, step_burn, validate_burn, BurnCommand};

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

fn check_new_flight_zero_alloc(_ctx: &Ctx) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(32),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("new_flight_zero_alloc", &budget, &alloc, || {
        let s = new_flight();
        black_box(s);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(FID_BENCH_FAIL, "new_flight exceeded zero allocation budget");
    }

    Verdict::Pass
}

fn check_validate_burn_zero_alloc(_ctx: &Ctx) -> Verdict {
    let state = new_flight();
    let cmd = BurnCommand {
        burn_rate_lbs_s: 100,
        duration_seconds: 5,
    };

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(64),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("validate_burn_zero_alloc", &budget, &alloc, || {
        let r = validate_burn(black_box(&state), black_box(&cmd));
        black_box(r).expect("validate_burn should succeed");
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "validate_burn exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_step_burn_zero_alloc(_ctx: &Ctx) -> Verdict {
    let state = new_flight();
    let cmd = BurnCommand {
        burn_rate_lbs_s: 100,
        duration_seconds: 5,
    };

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(128),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("step_burn_zero_alloc", &budget, &alloc, || {
        let r = step_burn(black_box(&state), black_box(&cmd));
        black_box(r).expect("step_burn should succeed");
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(FID_BENCH_FAIL, "step_burn exceeded zero allocation budget");
    }

    Verdict::Pass
}

fn check_isqrt_u128_zero_alloc(_ctx: &Ctx) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(64),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("isqrt_u128_zero_alloc", &budget, &alloc, || {
        let r = isqrt_u128(black_box(10_274_400_000_000));
        black_box(r);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(FID_BENCH_FAIL, "isqrt_u128 exceeded zero allocation budget");
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
        ("module", Json::str("lunar-lander")),
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
    suite.add("bench/new_flight/zero_alloc", check_new_flight_zero_alloc);
    suite.add(
        "bench/validate_burn/zero_alloc",
        check_validate_burn_zero_alloc,
    );
    suite.add("bench/step_burn/zero_alloc", check_step_burn_zero_alloc);
    suite.add("bench/isqrt_u128/zero_alloc", check_isqrt_u128_zero_alloc);
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
