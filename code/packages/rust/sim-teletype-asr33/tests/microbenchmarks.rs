//! # Microbenchmark Suite and Zero-Allocation Verification (VER07)
//!
//! Evaluates `sim-teletype-asr33` under the Level 5 Microbenchmark Harness
//! (`coding_adventures_test_bootstrap::bench`).
//!
//! Validates:
//! - Hard zero-heap-allocation budgets across all state transitions on `Asr33FrameState`:
//!   `step_serial_bit`, `tx_load`, `rx_feed_bit`, `kbd_push_byte`, `kbd_read_clear`,
//!   `tp_clear_flag`, as well as `Asr33Functional::step_serial_bit` on the hot path.
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
use sim_teletype_asr33::{Asr33FrameState, Asr33Functional};

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

fn check_step_serial_bit_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut state = Asr33FrameState::new();
    state.tx_load(b'A');

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(32),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("step_serial_bit_zero_alloc", &budget, &alloc, || {
        if state.tx_bits_remaining == 0 {
            state.tx_load(b'A');
        }
        let b = state.step_serial_bit();
        black_box(b);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "step_serial_bit exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_tx_load_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut state = Asr33FrameState::new();

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(64),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("tx_load_zero_alloc", &budget, &alloc, || {
        state.tx_load(black_box(b'X'));
        black_box(&state);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(FID_BENCH_FAIL, "tx_load exceeded zero allocation budget");
    }

    Verdict::Pass
}

fn check_rx_feed_bit_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut state = Asr33FrameState::new();

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(32),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("rx_feed_bit_zero_alloc", &budget, &alloc, || {
        let r = state.rx_feed_bit(black_box(0));
        black_box(r);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "rx_feed_bit exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_kbd_push_byte_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut state = Asr33FrameState::new();

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(32),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("kbd_push_byte_zero_alloc", &budget, &alloc, || {
        state.kbd_push_byte(black_box(b'Q'));
        black_box(&state);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "kbd_push_byte exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_kbd_read_clear_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut state = Asr33FrameState::new();
    state.kbd_push_byte(b'M');

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(32),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("kbd_read_clear_zero_alloc", &budget, &alloc, || {
        let b = state.kbd_read_clear();
        black_box(b);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "kbd_read_clear exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_tp_clear_flag_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut state = Asr33FrameState::new();

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(32),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("tp_clear_flag_zero_alloc", &budget, &alloc, || {
        state.tp_clear_flag();
        black_box(&state);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "tp_clear_flag exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_functional_step_serial_bit_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut func = Asr33Functional::new();
    func.tx_load(b'K');

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(64),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter(
        "functional_step_serial_bit_zero_alloc",
        &budget,
        &alloc,
        || {
            if func.state.tx_bits_remaining == 0 {
                func.tx_load(b'K');
            }
            let b = func.step_serial_bit();
            black_box(b);
        },
    );

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "Asr33Functional::step_serial_bit exceeded zero allocation budget",
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
        ("module", Json::str("sim-teletype-asr33")),
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
        "bench/step_serial_bit/zero_alloc",
        check_step_serial_bit_zero_alloc,
    );
    suite.add("bench/tx_load/zero_alloc", check_tx_load_zero_alloc);
    suite.add("bench/rx_feed_bit/zero_alloc", check_rx_feed_bit_zero_alloc);
    suite.add(
        "bench/kbd_push_byte/zero_alloc",
        check_kbd_push_byte_zero_alloc,
    );
    suite.add(
        "bench/kbd_read_clear/zero_alloc",
        check_kbd_read_clear_zero_alloc,
    );
    suite.add(
        "bench/tp_clear_flag/zero_alloc",
        check_tp_clear_flag_zero_alloc,
    );
    suite.add(
        "bench/functional_step/zero_alloc",
        check_functional_step_serial_bit_zero_alloc,
    );
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
