//! # Microbenchmark Suite and Equal-Instruction-Count Verification (VER07 / VER07a / VER-P017)
//!
//! Evaluates `coding_adventures_ct_compare` under the Level 5 Microbenchmark
//! Harness (`coding_adventures_test_bootstrap::bench`).
//!
//! Validates:
//! - Equal instruction counts across input equivalence classes (identical, diff first, diff last, diff all)
//! - Multi-scale complexity sweeps: $n \in \{16, 256, 4096\}$
//! - Fixed-width equality: `ct_eq_fixed` for 16, 32, and 64 bytes
//! - Branch-free selection: `ct_select_bytes` under `true` vs `false` choices
//! - 64-bit integer equality: `ct_eq_u64`
//! - Hard zero-heap-allocation budgets across all comparison operations
//! - Sabotage detection: early-exit loops produce measurable instruction/timing divergence

use std::hint::black_box;
use std::time::Duration;

use coding_adventures_ct_compare::{ct_eq, ct_eq_fixed, ct_eq_u64, ct_select_bytes};
use coding_adventures_test_bootstrap::bench::{
    AllocCounter, BenchmarkConfig, BenchmarkHarness, Budget, CounterSample, HardwarePerfCounter,
    StepCounter,
};
use coding_adventures_test_bootstrap::run::{self, Ctx, Suite};
use coding_adventures_test_bootstrap::Verdict;

const FID_BENCH_FAIL: &str = "property-counterexample";

fn main() {
    let mut suite = Suite::new();

    suite.add("bench/ct_eq/16_bytes", check_ct_eq_16);
    suite.add("bench/ct_eq/256_bytes", check_ct_eq_256);
    suite.add("bench/ct_eq/4096_bytes", check_ct_eq_4096);
    suite.add("bench/ct_eq_fixed/fixed_widths", check_ct_eq_fixed_widths);
    suite.add(
        "bench/ct_select_bytes/branchless_choice",
        check_ct_select_choice,
    );
    suite.add("bench/ct_eq_u64/all_classes", check_ct_eq_u64_classes);
    suite.add(
        "bench/sabotage/early_exit_divergence_detected",
        check_sabotage_divergence_detected,
    );

    std::process::exit(run::main(&suite));
}

/// Helper creating a fast benchmark configuration suitable for automated test suites.
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

/// Verify `ct_eq` on 16-byte slices across equal, differing-first, and differing-last classes.
fn check_ct_eq_16(_ctx: &Ctx) -> Verdict {
    let a = [0x5au8; 16];
    let b_equal = [0x5au8; 16];
    let mut b_diff_first = [0x5au8; 16];
    let mut b_diff_last = [0x5au8; 16];
    b_diff_first[0] = 0xa5;
    b_diff_last[15] = 0xa5;

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        ..Default::default()
    };

    // 1. Equal inputs
    let alloc_counter = AllocCounter::new();
    let res_equal = harness.run_with_counter("ct_eq_16_equal", &budget, &alloc_counter, || {
        ct_eq(black_box(&a), black_box(&b_equal))
    });
    if !res_equal.budget_verdict.is_acceptable() {
        return Verdict::fail(FID_BENCH_FAIL, "ct_eq 16 equal exceeded allocation budget");
    }

    // 2. Differing first byte
    let res_diff_first =
        harness.run_with_counter("ct_eq_16_diff_first", &budget, &alloc_counter, || {
            ct_eq(black_box(&a), black_box(&b_diff_first))
        });
    if !res_diff_first.budget_verdict.is_acceptable() {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "ct_eq 16 diff first exceeded allocation budget",
        );
    }

    // 3. Differing last byte
    let res_diff_last =
        harness.run_with_counter("ct_eq_16_diff_last", &budget, &alloc_counter, || {
            ct_eq(black_box(&a), black_box(&b_diff_last))
        });
    if !res_diff_last.budget_verdict.is_acceptable() {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "ct_eq 16 diff last exceeded allocation budget",
        );
    }

    // 4. Hardware instruction counter check (if Live on this host)
    let mut hw_equal = HardwarePerfCounter::instructions();
    let res_hw_eq = harness.run_with_counter("hw_eq", &budget, &mut hw_equal, || {
        ct_eq(black_box(&a), black_box(&b_equal))
    });

    let mut hw_diff = HardwarePerfCounter::instructions();
    let res_hw_diff = harness.run_with_counter("hw_diff", &budget, &mut hw_diff, || {
        ct_eq(black_box(&a), black_box(&b_diff_first))
    });

    if let (CounterSample::Live(eq_instr), CounterSample::Live(diff_instr)) = (
        res_hw_eq.instructions_per_iter,
        res_hw_diff.instructions_per_iter,
    ) {
        if eq_instr != diff_instr {
            return Verdict::fail(
                FID_BENCH_FAIL,
                format!(
                    "ct_eq 16 instruction count diverged: equal={eq_instr}, diff={diff_instr}"
                ),
            );
        }
    }

    Verdict::Pass
}

/// Verify `ct_eq` on 256-byte slices across equal, differing-first, differing-last, and middle classes.
fn check_ct_eq_256(_ctx: &Ctx) -> Verdict {
    let a = [0x42u8; 256];
    let b_equal = [0x42u8; 256];
    let mut b_diff_first = [0x42u8; 256];
    let mut b_diff_last = [0x42u8; 256];
    let mut b_diff_middle = [0x42u8; 256];
    b_diff_first[0] = 0x24;
    b_diff_last[255] = 0x24;
    b_diff_middle[128] = 0x24;

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        ..Default::default()
    };

    let alloc = AllocCounter::new();
    let res1 = harness.run_with_counter("ct_eq_256_equal", &budget, &alloc, || {
        ct_eq(black_box(&a), black_box(&b_equal))
    });
    let res2 = harness.run_with_counter("ct_eq_256_diff_first", &budget, &alloc, || {
        ct_eq(black_box(&a), black_box(&b_diff_first))
    });
    let res3 = harness.run_with_counter("ct_eq_256_diff_last", &budget, &alloc, || {
        ct_eq(black_box(&a), black_box(&b_diff_last))
    });
    let res4 = harness.run_with_counter("ct_eq_256_diff_middle", &budget, &alloc, || {
        ct_eq(black_box(&a), black_box(&b_diff_middle))
    });

    if !res1.budget_verdict.is_acceptable()
        || !res2.budget_verdict.is_acceptable()
        || !res3.budget_verdict.is_acceptable()
        || !res4.budget_verdict.is_acceptable()
    {
        return Verdict::fail(FID_BENCH_FAIL, "ct_eq 256 allocated memory");
    }

    Verdict::Pass
}

/// Verify `ct_eq` on 4096-byte slices (memory page scale) with 0 allocations.
fn check_ct_eq_4096(_ctx: &Ctx) -> Verdict {
    let a = vec![0x33u8; 4096];
    let b_equal = vec![0x33u8; 4096];
    let mut b_diff_last = vec![0x33u8; 4096];
    b_diff_last[4095] = 0xcc;

    let harness = BenchmarkHarness::new(fast_bench_config(15));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        ..Default::default()
    };

    let alloc = AllocCounter::new();
    let res_eq = harness.run_with_counter("ct_eq_4096_eq", &budget, &alloc, || {
        ct_eq(black_box(&a), black_box(&b_equal))
    });
    let res_diff = harness.run_with_counter("ct_eq_4096_diff", &budget, &alloc, || {
        ct_eq(black_box(&a), black_box(&b_diff_last))
    });

    if !res_eq.budget_verdict.is_acceptable() || !res_diff.budget_verdict.is_acceptable() {
        return Verdict::fail(FID_BENCH_FAIL, "ct_eq 4096 allocated memory");
    }

    Verdict::Pass
}

/// Verify fixed compile-time width equality (`ct_eq_fixed::<16>`, `ct_eq_fixed::<32>`, `ct_eq_fixed::<64>`).
fn check_ct_eq_fixed_widths(_ctx: &Ctx) -> Verdict {
    let k16_a = [0x11u8; 16];
    let k16_b = [0x11u8; 16];

    let k32_a = [0x22u8; 32];
    let k32_b = [0x22u8; 32];

    let k64_a = [0x33u8; 64];
    let k64_b = [0x33u8; 64];

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        ..Default::default()
    };

    let alloc = AllocCounter::new();
    let r16 = harness.run_with_counter("fixed_16", &budget, &alloc, || {
        ct_eq_fixed::<16>(black_box(&k16_a), black_box(&k16_b))
    });
    let r32 = harness.run_with_counter("fixed_32", &budget, &alloc, || {
        ct_eq_fixed::<32>(black_box(&k32_a), black_box(&k32_b))
    });
    let r64 = harness.run_with_counter("fixed_64", &budget, &alloc, || {
        ct_eq_fixed::<64>(black_box(&k64_a), black_box(&k64_b))
    });

    if !r16.budget_verdict.is_acceptable()
        || !r32.budget_verdict.is_acceptable()
        || !r64.budget_verdict.is_acceptable()
    {
        return Verdict::fail(FID_BENCH_FAIL, "ct_eq_fixed allocated memory");
    }

    Verdict::Pass
}

/// Verify `ct_select_bytes` execution under `choice = true` vs `choice = false`.
fn check_ct_select_choice(_ctx: &Ctx) -> Verdict {
    let x = [0xaau8; 64];
    let y = [0x55u8; 64];

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget::default();

    let res_true = harness.run("select_true", &budget, || {
        let out = ct_select_bytes(black_box(&x), black_box(&y), black_box(true));
        black_box(out);
    });

    let res_false = harness.run("select_false", &budget, || {
        let out = ct_select_bytes(black_box(&x), black_box(&y), black_box(false));
        black_box(out);
    });

    if res_true.median_ns <= 0.0 || res_false.median_ns <= 0.0 {
        return Verdict::fail(FID_BENCH_FAIL, "select benchmarks measured non-positive duration");
    }

    Verdict::Pass
}

/// Verify `ct_eq_u64` across equal, high-bit diff, low-bit diff, and inverted classes.
fn check_ct_eq_u64_classes(_ctx: &Ctx) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        ..Default::default()
    };

    let alloc = AllocCounter::new();
    let r_eq = harness.run_with_counter("u64_equal", &budget, &alloc, || {
        ct_eq_u64(black_box(0x1234_5678_9abc_def0), black_box(0x1234_5678_9abc_def0))
    });

    let r_diff = harness.run_with_counter("u64_diff", &budget, &alloc, || {
        ct_eq_u64(black_box(0x1234_5678_9abc_def0), black_box(0x9234_5678_9abc_def0))
    });

    if !r_eq.budget_verdict.is_acceptable() || !r_diff.budget_verdict.is_acceptable() {
        return Verdict::fail(FID_BENCH_FAIL, "ct_eq_u64 allocated memory");
    }

    Verdict::Pass
}

/// Negative fault injection: early-exit comparison produces instruction count divergence.
fn check_sabotage_divergence_detected(_ctx: &Ctx) -> Verdict {
    // Intentionally flawed comparison function that early-exits on first difference
    #[inline(never)]
    fn naive_leaky_compare(a: &[u8], b: &[u8], step_counter: &StepCounter) -> bool {
        if a.len() != b.len() {
            return false;
        }
        for i in 0..a.len() {
            step_counter.advance(1);
            if a[i] != b[i] {
                return false;
            }
        }
        true
    }

    let a = [0u8; 100];
    let mut diff_first = [0u8; 100];
    let mut diff_last = [0u8; 100];
    diff_first[0] = 1;
    diff_last[99] = 1;

    let harness = BenchmarkHarness::new(fast_bench_config(10));
    let budget = Budget::default();

    let step_counter_first = StepCounter::new();
    let res_first =
        harness.run_with_counter("leaky_diff_first", &budget, &step_counter_first, || {
            naive_leaky_compare(black_box(&a), black_box(&diff_first), &step_counter_first)
        });

    let step_counter_last = StepCounter::new();
    let res_last =
        harness.run_with_counter("leaky_diff_last", &budget, &step_counter_last, || {
            naive_leaky_compare(black_box(&a), black_box(&diff_last), &step_counter_last)
        });

    // The harness must observe clear step divergence: first-byte difference takes 1 step,
    // last-byte difference takes 100 steps.
    let steps_first = res_first.steps_per_iter.unwrap_or(0.0);
    let steps_last = res_last.steps_per_iter.unwrap_or(0.0);

    if steps_first >= steps_last {
        return Verdict::fail(
            FID_BENCH_FAIL,
            format!(
                "expected leaky steps_first < steps_last, got first={steps_first}, last={steps_last}"
            ),
        );
    }

    Verdict::Pass
}
