//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `game-clock`
//!
//! Evaluates `game-clock` under the Level 5 Microbenchmark Harness
//! (`coding_adventures_test_bootstrap::bench`).
//!
//! Validates:
//! - Hard zero-heap-allocation budgets across all frame advances (`advance_frame`),
//!   spiral-of-death clamp routines, configuration validations, and clock initializations,
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
use game_clock::{advance_frame, new_clock, validate_state, GameClockConfig, DEFAULT_60HZ_DT_NS};

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

fn check_advance_frame_60hz_steady_zero_alloc(_ctx: &Ctx) -> Verdict {
    let config = GameClockConfig::standard_60hz();
    let state = new_clock(config).expect("new_clock should succeed");

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(128),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter(
        "advance_frame_60hz_steady_zero_alloc",
        &budget,
        &alloc,
        || {
            let r = advance_frame(black_box(&state), black_box(DEFAULT_60HZ_DT_NS));
            black_box(r).expect("advance_frame should succeed");
        },
    );

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "advance_frame_60hz_steady exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_advance_frame_spiral_clamp_zero_alloc(_ctx: &Ctx) -> Verdict {
    let config = GameClockConfig::standard_60hz();
    let state = new_clock(config).expect("new_clock should succeed");
    let spike_delta_ns = 1_000_000_000u64; // 1-second spike

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(128),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter(
        "advance_frame_spiral_clamp_zero_alloc",
        &budget,
        &alloc,
        || {
            let r = advance_frame(black_box(&state), black_box(spike_delta_ns));
            black_box(r).expect("advance_frame should succeed");
        },
    );

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "advance_frame_spiral_clamp exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_new_clock_zero_alloc(_ctx: &Ctx) -> Verdict {
    let config = GameClockConfig::standard_60hz();

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(64),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("new_clock_zero_alloc", &budget, &alloc, || {
        let s = new_clock(black_box(config));
        black_box(s).expect("new_clock should succeed");
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(FID_BENCH_FAIL, "new_clock exceeded zero allocation budget");
    }

    Verdict::Pass
}

fn check_validate_state_zero_alloc(_ctx: &Ctx) -> Verdict {
    let config = GameClockConfig::standard_60hz();
    let state = new_clock(config).expect("new_clock");

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(64),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("validate_state_zero_alloc", &budget, &alloc, || {
        let v = validate_state(black_box(&state));
        black_box(v).expect("validate_state should succeed");
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "validate_state exceeded zero allocation budget",
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
        ("module", Json::str("game_clock")),
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
        "bench/advance_frame/60hz_steady_zero_alloc",
        check_advance_frame_60hz_steady_zero_alloc,
    );
    suite.add(
        "bench/advance_frame/spiral_clamp_zero_alloc",
        check_advance_frame_spiral_clamp_zero_alloc,
    );
    suite.add("bench/new_clock/zero_alloc", check_new_clock_zero_alloc);
    suite.add(
        "bench/validate_state/zero_alloc",
        check_validate_state_zero_alloc,
    );
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
