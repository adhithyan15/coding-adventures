//! # VER07 microbenchmarks: statistics, budgets and honest reporting
//!
//! Wall-clock results cannot be asserted, so this file asserts everything
//! that is *not* wall-clock:
//!
//! * the **statistics functions** against values worked out by hand
//!   (median, MAD, percentiles by linear interpolation, the bootstrap
//!   interval against an independent re-derivation);
//! * the **budget rules** of VER07 §5.1 through deterministic counters,
//!   including the case the audit found: a hard budget that was *declared* but
//!   could not be *checked* must be visible, not a quiet pass;
//! * **output safety**: a benchmark name cannot break the JSON or forge TAP
//!   lines;
//! * the **shape invariants** every real run must satisfy (ordering of
//!   quantiles, batch bounds, sample count).
//!
//! libtest runs these, not the harness under test.

use std::time::Duration;

use coding_adventures_test_bootstrap::bench::{
    bootstrap_median_ci_95, mad_from_samples, median_sorted, percentile_sorted, AllocCounter,
    BenchmarkConfig, BenchmarkHarness, Budget, BudgetVerdict, CostCounter, CounterSample,
    StepCounter, MAD_NORMAL_FACTOR,
};
use coding_adventures_test_bootstrap::prng::SplitMix64;

fn fast(sample_count: usize) -> BenchmarkHarness {
    BenchmarkHarness::new(BenchmarkConfig {
        warmup_iterations: 2,
        warmup_max_duration: Duration::from_micros(200),
        min_batch_duration: Duration::from_micros(20),
        max_batch_size: 4096,
        sample_count,
        seed: 7,
    })
}

// ---------------------------------------------------------------------------
// Hand-computed statistics
// ---------------------------------------------------------------------------

/// REGRESSION: the interpolation used to be `lo * (1 - w) + hi * w`. With
/// `lo == hi` that is not exactly `lo`: the two rounded products can sum one
/// ulp ABOVE it. On a coarse clock many samples are identical, so the 97.5th
/// percentile of the bootstrap medians came out one ulp above the run's own
/// maximum, and `every_run_has_ordered_quantiles_and_honours_its_bounds`
/// failed on a macOS runner. A percentile must lie between its two neighbours
/// exactly, and equal them when they are equal.
#[test]
fn a_percentile_between_equal_neighbours_is_exactly_that_value() {
    // (value, sample count, p): each rounded above `value` under the old
    // formula.
    let cases: [(f64, usize, f64); 5] = [
        (236_322.0, 10, 0.025),
        (118_448.666_666_666_67, 33, 0.025),
        (7_013.058_375_011_717_5, 2, 0.95),
        (123_905.666_666_666_67, 10, 0.025),
        (7_062.909_211_004_13, 33, 0.975),
    ];
    for (value, n, p) in cases {
        let flat = vec![value; n];
        assert_eq!(
            percentile_sorted(&flat, p),
            value,
            "p{p} of {n} copies of {value}"
        );
    }
}

/// Every interpolated percentile stays inside the closed interval of its two
/// neighbouring samples, for awkward magnitudes and weights alike.
#[test]
fn an_interpolated_percentile_never_leaves_its_neighbours() {
    let ps = [0.0, 0.01, 0.025, 0.25, 0.5, 0.75, 0.95, 0.975, 0.99, 1.0];
    let mut x = 0.1_f64;
    for step in 0..2_000u32 {
        // A deterministic spread of values from tiny to large, with near-equal
        // and equal neighbours mixed in.
        x = (x * 1.75 + f64::from(step) * 0.37) % 1.0e6;
        let lo = x;
        let hi = if step % 3 == 0 {
            x
        } else {
            x + (step % 7) as f64 * 1e-9 * x
        };
        for n in [2usize, 3, 10, 33] {
            let mut sorted = vec![lo; n / 2];
            sorted.extend(std::iter::repeat_n(hi, n - n / 2));
            for p in ps {
                let v = percentile_sorted(&sorted, p);
                assert!(
                    lo <= v && v <= hi,
                    "p{p} = {v} outside [{lo}, {hi}] (n={n})"
                );
            }
        }
    }
}

#[test]
fn percentiles_by_linear_interpolation_match_hand_values() {
    // VER07 §3.3: rank r = p (K - 1) + 1, linear between neighbours.
    let five = [1.0, 2.0, 3.0, 4.0, 5.0];
    assert_eq!(percentile_sorted(&five, 0.0), 1.0);
    assert_eq!(percentile_sorted(&five, 0.25), 2.0); // r = 2
    assert_eq!(percentile_sorted(&five, 0.5), 3.0); // r = 3
    assert_eq!(percentile_sorted(&five, 0.75), 4.0); // r = 4
    assert!((percentile_sorted(&five, 0.95) - 4.8).abs() < 1e-12); // 4*.2 + 5*.8
    assert!((percentile_sorted(&five, 0.99) - 4.96).abs() < 1e-12);
    assert_eq!(percentile_sorted(&five, 1.0), 5.0);

    let four = [1.0, 2.0, 3.0, 4.0];
    assert!((percentile_sorted(&four, 0.25) - 1.75).abs() < 1e-12);
    assert!((percentile_sorted(&four, 0.5) - 2.5).abs() < 1e-12);
    assert!((percentile_sorted(&four, 0.75) - 3.25).abs() < 1e-12);

    // Two samples: a straight line between them.
    assert!((percentile_sorted(&[10.0, 20.0], 0.3) - 13.0).abs() < 1e-12);
    // One sample: that sample for every p.
    for p in [0.0, 0.3, 1.0] {
        assert_eq!(percentile_sorted(&[42.0], p), 42.0);
    }
}

#[test]
fn percentile_clamps_p_and_never_leaves_the_sample_range() {
    let data = [3.0, 5.0, 9.0, 20.0];
    assert_eq!(percentile_sorted(&data, -1.0), 3.0);
    assert_eq!(percentile_sorted(&data, 7.0), 20.0);
    assert_eq!(percentile_sorted(&data, f64::NEG_INFINITY), 3.0);
    assert_eq!(percentile_sorted(&data, f64::INFINITY), 20.0);
    // Monotone in p over a fine grid, and inside [min, max].
    let mut previous = f64::NEG_INFINITY;
    for step in 0..=1000 {
        let value = percentile_sorted(&data, f64::from(step) / 1000.0);
        assert!(value >= previous && (3.0..=20.0).contains(&value));
        previous = value;
    }
}

#[test]
#[should_panic(expected = "non-empty")]
fn percentile_of_nothing_is_rejected_loudly() {
    let _ = percentile_sorted(&[], 0.5);
}

#[test]
fn median_follows_the_odd_and_even_definitions() {
    assert_eq!(median_sorted(&[7.0]), 7.0);
    assert_eq!(median_sorted(&[1.0, 9.0]), 5.0);
    assert_eq!(median_sorted(&[1.0, 2.0, 100.0]), 2.0);
    assert_eq!(median_sorted(&[1.0, 2.0, 3.0, 4.0]), 2.5);
    assert_eq!(median_sorted(&[-5.0, -1.0, 0.0, 4.0, 4.0, 4.0]), 2.0);
}

#[test]
fn mad_matches_hand_values() {
    // [1,2,3,4,100]: median 3, deviations [2,1,0,1,97], sorted [0,1,1,2,97], median 1.
    assert_eq!(mad_from_samples(&[1.0, 2.0, 3.0, 4.0, 100.0], 3.0), 1.0);
    // [1,2,3,4]: median 2.5, deviations [1.5,.5,.5,1.5], median 1.0.
    assert_eq!(mad_from_samples(&[1.0, 2.0, 3.0, 4.0], 2.5), 1.0);
    // Unsorted input is fine: MAD sorts its own deviations.
    assert_eq!(mad_from_samples(&[100.0, 3.0, 1.0, 4.0, 2.0], 3.0), 1.0);
    // Constant sample and empty sample.
    assert_eq!(mad_from_samples(&[5.0; 9], 5.0), 0.0);
    assert_eq!(mad_from_samples(&[], 0.0), 0.0);
    // Robustness: one wild outlier does not move the MAD.
    let mut clean: Vec<f64> = (1..=21).map(f64::from).collect();
    let mad_clean = mad_from_samples(&clean, 11.0);
    clean[20] = 1e12;
    assert_eq!(mad_from_samples(&clean, 11.0), mad_clean);
}

#[test]
fn nmad_factor_is_the_reciprocal_of_the_normal_75th_percentile() {
    // 1 / Phi^-1(0.75) with Phi^-1(0.75) = 0.6744897501960817.
    assert!((MAD_NORMAL_FACTOR - 1.0 / 0.674_489_750_196_081_7).abs() < 1e-12);
    // VER07 §3.2 writes 1.4826; the constant agrees to the digits quoted.
    assert!((MAD_NORMAL_FACTOR - 1.4826).abs() < 1e-4);
}

/// An independent bootstrap CI, written from VER07 §3.4 with the percentile
/// rule the module documents (linear interpolation, same as §3.3).
fn reference_ci(samples: &[f64], resamples: usize, seed: u64) -> (f64, f64) {
    let mut rng = SplitMix64::new(seed);
    let n = samples.len();
    let mut medians = Vec::new();
    for _ in 0..resamples {
        let mut draw: Vec<f64> = (0..n)
            .map(|_| samples[rng.bounded(n as u64) as usize])
            .collect();
        draw.sort_by(f64::total_cmp);
        let median = if n % 2 == 1 {
            draw[n / 2]
        } else {
            (draw[n / 2 - 1] + draw[n / 2]) / 2.0
        };
        medians.push(median);
    }
    medians.sort_by(f64::total_cmp);
    let at = |p: f64| {
        let rank = p * (medians.len() - 1) as f64;
        let lo = rank.floor() as usize;
        let hi = (lo + 1).min(medians.len() - 1);
        let w = rank - lo as f64;
        medians[lo] * (1.0 - w) + medians[hi] * w
    };
    (at(0.025), at(0.975))
}

#[test]
fn bootstrap_interval_matches_an_independent_derivation() {
    let samples = [12.0, 15.0, 11.0, 14.0, 90.0, 13.0, 12.5, 16.0, 11.5, 13.5];
    for seed in [0u64, 1, 7, 12345] {
        for resamples in [1usize, 2, 10, 1000] {
            let got = bootstrap_median_ci_95(&samples, resamples, seed);
            let want = reference_ci(&samples, resamples, seed);
            assert!(
                (got.0 - want.0).abs() < 1e-9 && (got.1 - want.1).abs() < 1e-9,
                "{got:?} vs {want:?} (seed {seed}, M {resamples})"
            );
        }
    }
}

#[test]
fn bootstrap_interval_is_ordered_inside_the_sample_and_deterministic() {
    let mut rng = SplitMix64::new(3);
    for len in 2..40usize {
        let samples: Vec<f64> = (0..len).map(|_| rng.bounded(1000) as f64).collect();
        let (lo, hi) = bootstrap_median_ci_95(&samples, 200, 9);
        let min = samples.iter().copied().fold(f64::INFINITY, f64::min);
        let max = samples.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        assert!(lo <= hi, "{lo} > {hi}");
        assert!(
            min <= lo && hi <= max,
            "[{lo}, {hi}] escapes [{min}, {max}]"
        );
        assert_eq!(bootstrap_median_ci_95(&samples, 200, 9), (lo, hi));
    }
    // Degenerate samples give a degenerate interval.
    assert_eq!(bootstrap_median_ci_95(&[4.0; 25], 300, 1), (4.0, 4.0));
    assert_eq!(bootstrap_median_ci_95(&[], 300, 1), (0.0, 0.0));
    assert_eq!(bootstrap_median_ci_95(&[8.0], 300, 1), (8.0, 8.0));
    // Two samples: every resample median is a, (a+b)/2 or b.
    let (lo, hi) = bootstrap_median_ci_95(&[2.0, 10.0], 1000, 5);
    assert!(2.0 <= lo && hi <= 10.0);
    // A tight sample has a tight interval; a wild outlier does not widen it.
    let mut tight: Vec<f64> = (0..51).map(|i| 100.0 + f64::from(i % 3)).collect();
    tight.push(1e9);
    let (lo, hi) = bootstrap_median_ci_95(&tight, 500, 2);
    assert!(
        hi - lo <= 2.0,
        "outlier leaked into the median CI: [{lo}, {hi}]"
    );
}

// ---------------------------------------------------------------------------
// Budgets (VER07 §5.1) through deterministic counters
// ---------------------------------------------------------------------------

#[test]
fn a_step_budget_is_exact_at_the_limit() {
    let harness = fast(10);
    let steps = StepCounter::new();
    let ten = Budget {
        max_steps: Some(10),
        ..Default::default()
    };
    let result = harness.run_with_counter("steps/at-limit", &ten, &steps, || steps.advance(10));
    assert_eq!(result.steps_per_iter, Some(10.0));
    assert_eq!(result.budget_verdict, BudgetVerdict::Pass);
    assert!(result.unchecked_hard_budgets.is_empty());

    let nine = Budget {
        max_steps: Some(9),
        ..Default::default()
    };
    let steps = StepCounter::new();
    let result = harness.run_with_counter("steps/over", &nine, &steps, || steps.advance(10));
    match result.budget_verdict {
        BudgetVerdict::Failed {
            observed, limit, ..
        } => assert_eq!((observed, limit), (10, 9)),
        other => panic!("expected Failed, got {other:?}"),
    }
    assert!(!result.budget_verdict.is_acceptable());
}

#[test]
fn an_allocation_budget_cannot_be_checked_without_the_counting_allocator() {
    // This binary deliberately does NOT call install_counting_allocator!, so
    // the real allocation counter has nothing to read. It must say NotLive and
    // leave the declared budget visibly unchecked; a bare zero would be the
    // vacuous pass of audit finding LH-003a. (Real measurement is tested in
    // alloc_budget.rs, which installs the allocator.)
    let harness = fast(10);
    let zero = Budget {
        max_alloc_bytes: Some(0),
        ..Default::default()
    };
    let counter = AllocCounter::new();
    assert_eq!(counter.stats(), None);
    let result = harness.run_with_counter("alloc/uninstalled", &zero, &counter, || {
        std::hint::black_box(vec![0u8; 4096])
    });
    assert_eq!(result.alloc_bytes_per_iter, None);
    assert_eq!(result.unchecked_hard_budgets, ["alloc_bytes"]);
    let mut by_ref = &counter;
    by_ref.start();
    assert_eq!(by_ref.sample(), CounterSample::NotLive);
}

#[test]
fn counters_measure_only_their_own_window() {
    // Work done before start() is invisible: the instrumented window is the
    // last batch, and per-iteration cost is the window total over the batch.
    let steps = StepCounter::new();
    steps.advance(1_000_000);
    let mut counter = &steps;
    counter.start();
    steps.advance(37);
    assert_eq!(counter.sample(), CounterSample::Live(37));
    assert_eq!(steps.total_steps(), 1_000_037);
}

/// A fake counter with a scripted sample, to reach the NotLive/Unsupported rows.
struct Scripted {
    name: &'static str,
    sample: CounterSample,
}

impl CostCounter for Scripted {
    fn name(&self) -> &'static str {
        self.name
    }
    fn start(&mut self) {}
    fn sample(&mut self) -> CounterSample {
        self.sample
    }
}

#[test]
fn a_counter_passed_by_mutable_reference_is_forwarded_faithfully() {
    // ct-compare hands `&mut HardwarePerfCounter` to run_with_counter; the
    // `&mut C` impl must forward name, start and sample.
    let harness = fast(10);
    let budget = Budget {
        max_steps: Some(3),
        ..Default::default()
    };
    let steps = StepCounter::new();
    let mut counter = &steps;
    let result = harness.run_with_counter("by-mut-ref", &budget, &mut counter, || steps.advance(3));
    assert_eq!(result.steps_per_iter, Some(3.0));
    assert_eq!(result.budget_verdict, BudgetVerdict::Pass);

    let mut scripted = Scripted {
        name: "steps",
        sample: CounterSample::Live(0),
    };
    let result = harness.run_with_counter("by-mut-ref-scripted", &budget, &mut scripted, || 1 + 1);
    assert_eq!(result.steps_per_iter, Some(0.0));
}

#[test]
fn instruction_budget_rules_for_live_not_live_and_unsupported() {
    // The batch size is pinned to exactly 1 (max_batch_size: 1 means the
    // calibration loop never scales), so Live(n) means n instructions/iter.
    // It used to rely on one `1 + 1` meeting a 1 ns min_batch_duration, but a
    // coarse clock (Windows) can read 0 ns for that, which doubled the batch
    // and turned Live(101) into 51 per iteration: a spurious Pass.
    let harness = BenchmarkHarness::new(BenchmarkConfig {
        warmup_iterations: 1,
        warmup_max_duration: Duration::from_micros(50),
        min_batch_duration: Duration::from_nanos(1),
        max_batch_size: 1,
        sample_count: 3,
        seed: 1,
    });
    let budget = Budget {
        max_instructions: Some(100),
        ..Default::default()
    };
    let run = |sample| {
        let counter = Scripted {
            name: "instructions",
            sample,
        };
        harness.run_with_counter("hw", &budget, counter, || 1 + 1)
    };
    assert_eq!(run(CounterSample::Live(0)).batch_size, 1);
    assert_eq!(
        run(CounterSample::Live(0)).budget_verdict,
        BudgetVerdict::Pass
    );
    let at_limit = run(CounterSample::Live(100));
    assert_eq!(at_limit.budget_verdict, BudgetVerdict::Pass);
    assert!(at_limit.unchecked_hard_budgets.is_empty());
    let over = run(CounterSample::Live(101));
    assert!(matches!(
        over.budget_verdict,
        BudgetVerdict::Failed { limit: 100, .. }
    ));

    // §5.1 rule 3: not live / unsupported is a PASS, annotated, never a failure...
    for sample in [CounterSample::NotLive, CounterSample::Unsupported] {
        let result = run(sample);
        assert_eq!(result.budget_verdict, BudgetVerdict::Pass);
        assert_eq!(result.instructions_per_iter, sample);
        // ...and the report says the budget went unchecked.
        assert_eq!(result.unchecked_hard_budgets, ["instructions"]);
        assert!(result
            .to_tap_yaml()
            .contains("unchecked_hard_budgets: [instructions]"));
        assert!(result
            .to_json()
            .contains("\"unchecked_hard_budgets\": [\"instructions\"]"));
    }
}

#[test]
fn a_hard_budget_without_a_matching_counter_is_reported_unchecked() {
    // REGRESSION: `bench()` with `max_steps: Some(1)` reported a clean Pass
    // because nothing measured steps; the declared budget just vanished.
    let harness = fast(10);
    let all_three = Budget {
        max_steps: Some(1),
        max_alloc_bytes: Some(0),
        max_instructions: Some(1),
        ..Default::default()
    };
    let result = harness.run("no-counter", &all_three, || 1 + 1);
    assert_eq!(result.budget_verdict, BudgetVerdict::Pass);
    assert_eq!(
        result.unchecked_hard_budgets,
        ["steps", "alloc_bytes", "instructions"]
    );

    // A steps counter checks steps and nothing else.
    let steps = StepCounter::new();
    let result = harness.run_with_counter("steps-only", &all_three, &steps, || steps.advance(1));
    assert_eq!(
        result.unchecked_hard_budgets,
        ["alloc_bytes", "instructions"]
    );

    // An unknown counter name checks nothing.
    let result = harness.run_with_counter(
        "mystery",
        &all_three,
        Scripted {
            name: "mystery",
            sample: CounterSample::Live(5),
        },
        || 1 + 1,
    );
    assert_eq!(result.unchecked_hard_budgets.len(), 3);

    // No hard budget declared: nothing to be unchecked.
    let result = harness.run("none", &Budget::default(), || 1 + 1);
    assert!(result.unchecked_hard_budgets.is_empty());
    assert!(!result.to_tap_yaml().contains("unchecked_hard_budgets"));
}

#[test]
fn soft_timing_budget_warns_but_never_fails() {
    let harness = fast(10);
    let impossible = Budget {
        max_median_duration_ns: Some(0),
        ..Default::default()
    };
    let result = harness.run("timing", &impossible, || {
        std::hint::black_box(3u64).wrapping_mul(7)
    });
    assert!(matches!(
        result.budget_verdict,
        BudgetVerdict::PassWithWarning { limit_ns: 0, .. }
    ));
    assert!(result.budget_verdict.is_acceptable());
    let generous = Budget {
        max_median_duration_ns: Some(u64::MAX),
        ..Default::default()
    };
    assert_eq!(
        harness.run("timing", &generous, || 1 + 1).budget_verdict,
        BudgetVerdict::Pass
    );
}

// ---------------------------------------------------------------------------
// Shape invariants of a real run
// ---------------------------------------------------------------------------

#[test]
fn every_run_has_ordered_quantiles_and_honours_its_bounds() {
    for sample_count in [1usize, 2, 10, 33] {
        let result = fast(sample_count).run("shape", &Budget::default(), || {
            std::hint::black_box((1..20u64).sum::<u64>())
        });
        assert_eq!(result.sample_count, sample_count);
        assert!(result.batch_size >= 1 && result.batch_size <= 4096);
        let chain = [
            result.min_ns,
            result.p25_ns,
            result.median_ns,
            result.p75_ns,
            result.p95_ns,
            result.p99_ns,
            result.max_ns,
        ];
        assert!(
            chain.windows(2).all(|w| w[0] <= w[1]),
            "unordered: {chain:?}"
        );
        assert!(result.mad_ns >= 0.0);
        assert!((result.nmad_ns - result.mad_ns * MAD_NORMAL_FACTOR).abs() < 1e-9);
        assert!(result.ci_95_ns.0 <= result.ci_95_ns.1);
        assert!(result.min_ns <= result.ci_95_ns.0 && result.ci_95_ns.1 <= result.max_ns);
        assert!(result.min_ns >= 0.0);
    }
}

#[test]
fn a_zero_sample_run_is_rejected_loudly() {
    let result = std::panic::catch_unwind(|| fast(0).run("zero", &Budget::default(), || 1 + 1));
    assert!(
        result.is_err(),
        "sample_count 0 must not produce statistics from nothing"
    );
}

#[test]
fn batch_calibration_grows_for_cheap_work_and_respects_the_ceiling() {
    let harness = BenchmarkHarness::new(BenchmarkConfig {
        warmup_iterations: 1,
        warmup_max_duration: Duration::from_micros(100),
        min_batch_duration: Duration::from_millis(2),
        max_batch_size: 64,
        sample_count: 3,
        seed: 1,
    });
    let result = harness.run("cheap", &Budget::default(), || std::hint::black_box(1u8));
    assert_eq!(
        result.batch_size, 64,
        "a ~nanosecond body must hit the ceiling"
    );
    let harness = BenchmarkHarness::new(BenchmarkConfig {
        warmup_iterations: 1,
        warmup_max_duration: Duration::from_micros(100),
        min_batch_duration: Duration::from_nanos(1),
        max_batch_size: 64,
        sample_count: 3,
        seed: 1,
    });
    // The body spins until the clock visibly advances, so one iteration is
    // at least one clock tick (>= 1 ns) on every platform. A bare
    // `black_box(1u8)` can measure 0 ns on a coarse clock (Windows), which
    // would make the harness scale the batch and the test flaky.
    let result = harness.run("already-long-enough", &Budget::default(), || {
        let start = std::time::Instant::now();
        while start.elapsed().is_zero() {
            std::hint::spin_loop();
        }
    });
    assert_eq!(
        result.batch_size, 1,
        "no scaling when one iteration already meets the target"
    );
}

#[test]
fn same_seed_gives_the_same_interval_for_the_same_samples() {
    // The interval is a function of (samples, seed) only; timing noise changes
    // the samples, never the resampling. Check the seed is what drives it.
    let samples: Vec<f64> = (0..40).map(|i| f64::from((i * 37) % 101)).collect();
    let a = bootstrap_median_ci_95(&samples, 1000, 1);
    assert_eq!(a, bootstrap_median_ci_95(&samples, 1000, 1));
    let distinct: std::collections::BTreeSet<String> = (1..=10u64)
        .map(|seed| format!("{:?}", bootstrap_median_ci_95(&samples, 1000, seed)))
        .collect();
    assert!(distinct.len() > 1, "ten seeds resampled identically");
}

// ---------------------------------------------------------------------------
// Output safety
// ---------------------------------------------------------------------------

#[test]
fn a_hostile_benchmark_name_cannot_break_json_or_forge_tap() {
    let harness = fast(3);
    for name in [
        "plain/name",
        "quote\"inside",
        "back\\slash",
        "new\nline",
        "forged\n  ...\nok 99 - x",
        "tab\tand\rcr",
        "\u{0}nul\u{7f}del",
        "line\u{2028}sep",
    ] {
        let result = harness.run(name, &Budget::default(), || 1 + 1);
        let yaml = result.to_tap_yaml();
        // Exactly one block: one opener and one closer, one `benchmark:` line.
        assert_eq!(yaml.matches("\n").count(), yaml.lines().count(), "{name:?}");
        assert_eq!(
            yaml.lines().filter(|l| *l == "  ---").count(),
            1,
            "{name:?}"
        );
        assert_eq!(
            yaml.lines().filter(|l| *l == "  ...").count(),
            1,
            "{name:?}"
        );
        assert_eq!(
            yaml.lines()
                .filter(|l| l.starts_with("  benchmark: "))
                .count(),
            1,
            "{name:?}"
        );
        assert!(!yaml
            .lines()
            .any(|l| l.starts_with("ok ") || l.starts_with("not ok ")));
        let json = result.to_json();
        let name_line = json
            .lines()
            .find(|l| l.contains("\"name\""))
            .expect("name line");
        assert!(!name_line.contains('\n'));
        // The JSON string, un-escaped by a minimal reader, round-trips the name.
        let body = name_line
            .trim()
            .trim_start_matches("\"name\": \"")
            .trim_end_matches("\",");
        assert_eq!(
            unescape(body),
            name,
            "JSON name did not round-trip for {name:?}"
        );
        assert_eq!(
            json.lines()
                .filter(|l| l.contains("\"batch_size\""))
                .count(),
            1
        );
    }
}

/// Undo `escape_quoted`: `\" \\ \n \r \t \uXXXX`.
fn unescape(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                out.push(char::from_u32(u32::from_str_radix(&hex, 16).unwrap()).unwrap());
            }
            other => panic!("bad escape {other:?}"),
        }
    }
    out
}
