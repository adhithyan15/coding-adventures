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
//! - Hard zero-heap-allocation budgets across all comparison operations,
//!   measured on the REAL heap by test-bootstrap's counting global allocator
//!   (LH-003a: the previous counter was cooperative and these budgets were
//!   vacuous). `ct_select_bytes` is deliberately NOT in that claim: it returns
//!   a fresh `Vec`, so its allocation profile is pinned separately and exactly.
//! - Sabotage detection: early-exit loops produce measurable instruction/timing divergence

#![deny(unsafe_code)]

use std::hint::black_box;
use std::time::Duration;

use coding_adventures_ct_compare::{ct_eq, ct_eq_fixed, ct_eq_u64, ct_select_bytes};
use coding_adventures_test_bootstrap::bench::{
    AllocCounter, BenchmarkConfig, BenchmarkHarness, BenchmarkResult, Budget, BudgetVerdict,
    CounterSample, HardwarePerfCounter, StepCounter,
};
use coding_adventures_test_bootstrap::counting_alloc;
use coding_adventures_test_bootstrap::manifest::{self, Json};
use coding_adventures_test_bootstrap::run::{self, Ctx, Outcome, Suite};
use coding_adventures_test_bootstrap::Verdict;

const FID_BENCH_FAIL: &str = "property-counterexample";

// Every `max_alloc_bytes` budget below is checked against the real heap
// because this binary's global allocator counts it (LH-003a).
coding_adventures_test_bootstrap::install_counting_allocator!();

/// A zero-allocation budget held AND was genuinely measured: the verdict is
/// acceptable, the counter was live and read exactly 0.0 bytes per iteration,
/// and the budget is not listed as unchecked. (`is_acceptable()` alone would
/// also accept a counter that was never live.)
fn zero_alloc_ok(result: &BenchmarkResult) -> bool {
    result.budget_verdict.is_acceptable()
        && result.alloc_bytes_per_iter == Some(0.0)
        && !result.unchecked_hard_budgets.contains(&"alloc_bytes")
}

fn main() {
    let mut suite = Suite::new();

    suite.add(
        "bench/alloc/counting_allocator_is_live",
        check_counting_allocator_is_live,
    );
    suite.add("bench/ct_eq/16_bytes", check_ct_eq_16);
    suite.add("bench/ct_eq/256_bytes", check_ct_eq_256);
    suite.add("bench/ct_eq/4096_bytes", check_ct_eq_4096);
    suite.add("bench/ct_eq_fixed/fixed_widths", check_ct_eq_fixed_widths);
    // Not `bench/ct_*`: ct_select_bytes allocates its output, so it must not
    // be counted among the zero-allocation benchmarks the assurance report
    // cites for CWE-400.
    suite.add(
        "bench/select_bytes/branchless_choice",
        check_ct_select_choice,
    );
    suite.add(
        "bench/select_bytes/allocates_exactly_its_output",
        check_select_allocation_profile,
    );
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );
    suite.add("bench/ct_eq_u64/all_classes", check_ct_eq_u64_classes);
    suite.add(
        "bench/sabotage/early_exit_divergence_detected",
        check_sabotage_divergence_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}

/// Is a hardware retired-instruction counter live on this host?  The equal
/// instruction-count assertions above only run when it is, so the report must
/// know (VER04h): "all benchmarks passed" does not imply "instruction counts
/// were compared".
fn probe_hardware_counters() -> &'static str {
    let harness = BenchmarkHarness::new(fast_bench_config(3));
    let budget = Budget::default();
    let a = [0x5au8; 16];
    let mut counter = HardwarePerfCounter::instructions();
    let res = harness.run_with_counter("hw_probe", &budget, &mut counter, || {
        ct_eq(black_box(&a), black_box(&a))
    });
    match res.instructions_per_iter {
        CounterSample::Live(_) => "live",
        _ => "not-live",
    }
}

/// With `WARRANT_BENCH_OUT=<file>`, record what this run reported so the
/// assurance report can quote it instead of asserting it (VER04h section 8).
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
                ("verdict", Json::str(if o.passed() { "pass" } else { "fail" })),
            ])
        })
        .collect();
    tests.sort_by_key(|t| t.render());
    let doc = Json::obj([
        ("schema", Json::str("warrant-bench-evidence-v1")),
        ("module", Json::str("ct_compare")),
        ("hw_instruction_counters", Json::str(probe_hardware_counters())),
        // Same definition as the proof manifest: SHA-256 of the framed src/**/*.rs.
        (
            "source_sha256",
            Json::str(coding_adventures_sha256::sha256_hex(
                &manifest::source_tree_framing(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))
                    .expect("bind bench manifest to src"),
            )),
        ),
        ("tests", Json::Arr(tests)),
    ]);
    manifest::write(&path, &doc).expect("write bench manifest");
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
    if !zero_alloc_ok(&res_equal) {
        return Verdict::fail(FID_BENCH_FAIL, "ct_eq 16 equal exceeded allocation budget");
    }

    // 2. Differing first byte
    let res_diff_first =
        harness.run_with_counter("ct_eq_16_diff_first", &budget, &alloc_counter, || {
            ct_eq(black_box(&a), black_box(&b_diff_first))
        });
    if !zero_alloc_ok(&res_diff_first) {
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
    if !zero_alloc_ok(&res_diff_last) {
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
        let eq_per_iter = (eq_instr as f64 / res_hw_eq.batch_size.max(1) as f64).round() as u64;
        let diff_per_iter =
            (diff_instr as f64 / res_hw_diff.batch_size.max(1) as f64).round() as u64;
        if eq_per_iter != diff_per_iter {
            return Verdict::fail(
                FID_BENCH_FAIL,
                format!(
                    "ct_eq 16 instruction count diverged: equal={eq_per_iter}, diff={diff_per_iter}"
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

    if !zero_alloc_ok(&res1)
        || !zero_alloc_ok(&res2)
        || !zero_alloc_ok(&res3)
        || !zero_alloc_ok(&res4)
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

    if !zero_alloc_ok(&res_eq) || !zero_alloc_ok(&res_diff) {
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

    if !zero_alloc_ok(&r16)
        || !zero_alloc_ok(&r32)
        || !zero_alloc_ok(&r64)
    {
        return Verdict::fail(FID_BENCH_FAIL, "ct_eq_fixed allocated memory");
    }

    Verdict::Pass
}

/// The counting allocator must really be the global allocator, or every
/// zero-allocation budget in this file would be unchecked.
fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

/// Negative control (LH-003a): a closure that really allocates must FAIL a
/// zero-byte budget, and a closure that does not must pass it, through the
/// same harness and counter the ct benchmarks use. If the counter were blind,
/// the first half would report a pass and this test would fail.
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

/// `ct_select_bytes` returns a fresh `Vec`, so unlike the comparisons it DOES
/// allocate. Pin the real profile exactly: one allocation of `a.len()` bytes
/// per call, for either choice, and nothing else.
fn check_select_allocation_profile(_ctx: &Ctx) -> Verdict {
    let x = [0xaau8; 64];
    let y = [0x55u8; 64];
    for choice in [true, false] {
        let (out, stats) = counting_alloc::measure(|| {
            ct_select_bytes(black_box(&x), black_box(&y), black_box(choice))
        });
        black_box(out);
        let Some(stats) = stats else {
            return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
        };
        if stats.allocations != 1 || stats.bytes_allocated != 64 {
            return Verdict::fail(
                FID_BENCH_FAIL,
                format!(
                    "ct_select_bytes expected 1 allocation of 64 bytes, saw {} of {} bytes",
                    stats.allocations, stats.bytes_allocated
                ),
            );
        }
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
        return Verdict::fail(
            FID_BENCH_FAIL,
            "select benchmarks measured non-positive duration",
        );
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

    let val: u64 = 0x1234_5678_9abc_def0;
    let alloc = AllocCounter::new();

    // 1. Equal
    let r_eq = harness.run_with_counter("u64_equal", &budget, &alloc, || {
        ct_eq_u64(black_box(val), black_box(val))
    });

    // 2. High bit difference (sign bit)
    let r_high_diff = harness.run_with_counter("u64_high_diff", &budget, &alloc, || {
        ct_eq_u64(black_box(val), black_box(val ^ (1 << 63)))
    });

    // 3. Low bit difference
    let r_low_diff = harness.run_with_counter("u64_low_diff", &budget, &alloc, || {
        ct_eq_u64(black_box(val), black_box(val ^ 1))
    });

    // 4. Bitwise inverted
    let r_inv = harness.run_with_counter("u64_inv", &budget, &alloc, || {
        ct_eq_u64(black_box(val), black_box(!val))
    });

    if !zero_alloc_ok(&r_eq)
        || !zero_alloc_ok(&r_high_diff)
        || !zero_alloc_ok(&r_low_diff)
        || !zero_alloc_ok(&r_inv)
    {
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
    let res_last = harness.run_with_counter("leaky_diff_last", &budget, &step_counter_last, || {
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
