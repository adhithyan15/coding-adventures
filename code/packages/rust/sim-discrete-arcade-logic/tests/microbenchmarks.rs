//! # Microbenchmark Suite and Zero-Allocation Verification (VER07)
//!
//! Evaluates `sim-discrete-arcade-logic` under the Level 5 Microbenchmark Harness
//! (`coding_adventures_test_bootstrap::bench`).
//!
//! Validates:
//! - Hard zero-heap-allocation budgets across all hot-path operations:
//!   `Ttl7400QuadNand`, `Ttl7402QuadNor`, `Ttl7474DualDFlipFlop`, `Ttl7490DecadeCounter`,
//!   `Ttl7493BinaryCounter`, `Ttl74153DualMux4to1`, `ArcadeSyncGenerator`, and `PaddlePotAdc`.
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
use sim_discrete_arcade_logic::{
    ArcadeSyncGenerator, PaddlePotAdc, QuadLogicLevels, Ttl7400QuadNand, Ttl7402QuadNor,
    Ttl7474DualDFlipFlop, Ttl7490DecadeCounter, Ttl7493BinaryCounter, Ttl74153DualMux4to1,
    MASTER_CLOCK_HZ, PIXEL_CLOCK_HZ,
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

fn check_7400_nand_zero_alloc(_ctx: &Ctx) -> Verdict {
    let nand = Ttl7400QuadNand::new_7400();
    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(32),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("7400_nand_zero_alloc", &budget, &alloc, || {
        let r = nand.nand_eval_gate(0, black_box(1), black_box(1)).unwrap();
        black_box(r);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(FID_BENCH_FAIL, "7400 nand exceeded zero allocation budget");
    }
    Verdict::Pass
}

fn check_7402_nor_zero_alloc(_ctx: &Ctx) -> Verdict {
    let nor = Ttl7402QuadNor::new_7402();
    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(32),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("7402_nor_zero_alloc", &budget, &alloc, || {
        let r = nor.nor_eval_gate(0, black_box(0), black_box(0)).unwrap();
        black_box(r);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(FID_BENCH_FAIL, "7402 nor exceeded zero allocation budget");
    }
    Verdict::Pass
}

fn check_7474_dff_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut dff = Ttl7474DualDFlipFlop::new_7474();
    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(64),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("7474_dff_zero_alloc", &budget, &alloc, || {
        let r = dff
            .dff_step_channel(0, black_box(1), black_box(1), 1, 1)
            .unwrap();
        dff.dff_step_channel(0, 0, 1, 1, 1).unwrap();
        black_box(r);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(FID_BENCH_FAIL, "7474 dff exceeded zero allocation budget");
    }
    Verdict::Pass
}

fn check_7490_decade_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut cnt = Ttl7490DecadeCounter::new_7490();
    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(64),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("7490_decade_zero_alloc", &budget, &alloc, || {
        let r = cnt.decade_step_bcd(black_box(0)).unwrap();
        cnt.decade_step_bcd(1).unwrap();
        black_box(r);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(FID_BENCH_FAIL, "7490 decade exceeded zero allocation budget");
    }
    Verdict::Pass
}

fn check_7493_binary_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut cnt = Ttl7493BinaryCounter::new_7493();
    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(64),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("7493_binary_zero_alloc", &budget, &alloc, || {
        let r = cnt.binary_step_4bit(black_box(0)).unwrap();
        cnt.binary_step_4bit(1).unwrap();
        black_box(r);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(FID_BENCH_FAIL, "7493 binary exceeded zero allocation budget");
    }
    Verdict::Pass
}

fn check_74153_mux_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mux = Ttl74153DualMux4to1::new_74153();
    let in1 = QuadLogicLevels {
        y0: 1,
        y1: 0,
        y2: 1,
        y3: 0,
    };
    let in2 = QuadLogicLevels {
        y0: 0,
        y1: 1,
        y2: 0,
        y3: 1,
    };
    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(64),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("74153_mux_zero_alloc", &budget, &alloc, || {
        let r = mux
            .mux_eval_dual(black_box(0), black_box(1), 0, in1, 0, in2)
            .unwrap();
        black_box(r);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(FID_BENCH_FAIL, "74153 mux exceeded zero allocation budget");
    }
    Verdict::Pass
}

fn check_arcade_sync_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut sync = ArcadeSyncGenerator::new_arcade_sync(MASTER_CLOCK_HZ).unwrap();
    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(64),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("arcade_sync_zero_alloc", &budget, &alloc, || {
        let pos = sync.sync_step_master_clock();
        black_box(pos);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(FID_BENCH_FAIL, "arcade sync exceeded zero allocation budget");
    }
    Verdict::Pass
}

fn check_paddle_pot_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut adc = PaddlePotAdc::new_paddle_pot_adc(PIXEL_CLOCK_HZ, 100_000, 100_000).unwrap();
    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(64),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("paddle_pot_zero_alloc", &budget, &alloc, || {
        let st = adc.paddle_step_clock(black_box(32768)).unwrap();
        black_box(st);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(FID_BENCH_FAIL, "paddle pot exceeded zero allocation budget");
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
        ("module", Json::str("sim-discrete-arcade-logic")),
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
    suite.add("bench/7400_nand/zero_alloc", check_7400_nand_zero_alloc);
    suite.add("bench/7402_nor/zero_alloc", check_7402_nor_zero_alloc);
    suite.add("bench/7474_dff/zero_alloc", check_7474_dff_zero_alloc);
    suite.add(
        "bench/7490_decade/zero_alloc",
        check_7490_decade_zero_alloc,
    );
    suite.add(
        "bench/7493_binary/zero_alloc",
        check_7493_binary_zero_alloc,
    );
    suite.add("bench/74153_mux/zero_alloc", check_74153_mux_zero_alloc);
    suite.add(
        "bench/arcade_sync/zero_alloc",
        check_arcade_sync_zero_alloc,
    );
    suite.add(
        "bench/paddle_pot/zero_alloc",
        check_paddle_pot_zero_alloc,
    );
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
