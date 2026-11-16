//! # Microbenchmark Suite and Zero-Allocation Verification (VER07)
//!
//! Evaluates `sim-pdp8-ge225-io-bridge` under the Level 5 Microbenchmark Harness
//! (`coding_adventures_test_bootstrap::bench`).
//!
//! Validates:
//! - Hard zero-heap-allocation budgets across all state transitions and hot dispatch paths:
//!   `pdp8_dispatch_iot_functional`, `GateLevelIotDecoder::decode`, `ge225_bcd_to_ascii`,
//!   `ascii_to_ge225_bcd`, `Pdp8TeletypeBridge::handle_iot`, and `Pdp8TeletypeGateLevelBridge::handle_iot`.
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
use pdp8_simulator::IotEvent;
use sim_pdp8_ge225_io_bridge::{
    ascii_to_ge225_bcd, ge225_bcd_to_ascii, pdp8_dispatch_iot_functional, GateLevelIotDecoder,
    Pdp8TeletypeBridge, Pdp8TeletypeGateLevelBridge, PDP8_DEV_KBD, PDP8_DEV_TPR, PDP8_IOP1,
    PDP8_IOP2, PDP8_IOP4,
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

fn check_pdp8_dispatch_iot_functional_zero_alloc(_ctx: &Ctx) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(32),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter(
        "pdp8_dispatch_iot_functional_zero_alloc",
        &budget,
        &alloc,
        || {
            let r = pdp8_dispatch_iot_functional(
                black_box(PDP8_DEV_KBD),
                black_box(PDP8_IOP2 | PDP8_IOP4),
                black_box(0o1234),
                black_box(true),
                black_box(false),
                black_box(0x41),
            );
            black_box(r);
        },
    );

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "pdp8_dispatch_iot_functional exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_gate_level_iot_decoder_zero_alloc(_ctx: &Ctx) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(64),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("gate_level_iot_decoder_zero_alloc", &budget, &alloc, || {
        let r = GateLevelIotDecoder::decode(
            black_box(PDP8_DEV_TPR),
            black_box(PDP8_IOP1 | PDP8_IOP2 | PDP8_IOP4),
            black_box(0o4321),
            black_box(false),
            black_box(true),
            black_box(0x5A),
        );
        black_box(r);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "GateLevelIotDecoder::decode exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_ge225_bcd_to_ascii_zero_alloc(_ctx: &Ctx) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(16),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("ge225_bcd_to_ascii_zero_alloc", &budget, &alloc, || {
        let r = ge225_bcd_to_ascii(black_box(0o21));
        black_box(r);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "ge225_bcd_to_ascii exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_ascii_to_ge225_bcd_zero_alloc(_ctx: &Ctx) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(16),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("ascii_to_ge225_bcd_zero_alloc", &budget, &alloc, || {
        let r = ascii_to_ge225_bcd(black_box(b'G'));
        black_box(r);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "ascii_to_ge225_bcd exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_pdp8_bridge_handle_iot_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut bridge = Pdp8TeletypeBridge::new();
    let iot = IotEvent {
        device: PDP8_DEV_TPR,
        pulses: PDP8_IOP1,
    };

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(64),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter(
        "pdp8_bridge_handle_iot_zero_alloc",
        &budget,
        &alloc,
        || {
            let r = bridge.handle_iot(black_box(iot), black_box(0o0101));
            black_box(r);
        },
    );

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "Pdp8TeletypeBridge::handle_iot exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_pdp8_gate_bridge_handle_iot_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut bridge = Pdp8TeletypeGateLevelBridge::new();
    let iot = IotEvent {
        device: PDP8_DEV_KBD,
        pulses: PDP8_IOP1,
    };

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(64),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter(
        "pdp8_gate_bridge_handle_iot_zero_alloc",
        &budget,
        &alloc,
        || {
            let r = bridge.handle_iot(black_box(iot), black_box(0o0000));
            black_box(r);
        },
    );

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "Pdp8TeletypeGateLevelBridge::handle_iot exceeded zero allocation budget",
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
        ("module", Json::str("sim-pdp8-ge225-io-bridge")),
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
        "bench/pdp8_dispatch_iot_functional/zero_alloc",
        check_pdp8_dispatch_iot_functional_zero_alloc,
    );
    suite.add(
        "bench/gate_level_iot_decoder/zero_alloc",
        check_gate_level_iot_decoder_zero_alloc,
    );
    suite.add(
        "bench/ge225_bcd_to_ascii/zero_alloc",
        check_ge225_bcd_to_ascii_zero_alloc,
    );
    suite.add(
        "bench/ascii_to_ge225_bcd/zero_alloc",
        check_ascii_to_ge225_bcd_zero_alloc,
    );
    suite.add(
        "bench/pdp8_bridge_handle_iot/zero_alloc",
        check_pdp8_bridge_handle_iot_zero_alloc,
    );
    suite.add(
        "bench/pdp8_gate_bridge_handle_iot/zero_alloc",
        check_pdp8_gate_bridge_handle_iot_zero_alloc,
    );
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
