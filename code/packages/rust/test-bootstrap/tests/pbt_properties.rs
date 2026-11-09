//! # VER05 property engine: generators, shrinking guarantees, verdict honesty
//!
//! `pbt_corpus.rs` pins the engine to the pbt-v1 conformance vectors. This
//! file asks the questions a corpus cannot:
//!
//! * **Does a falsified property always come back as a failure?** (and a
//!   property that was never evaluated never as a pass)
//! * **Is the reported witness really a counterexample?** A shrinker that
//!   returns a value that no longer fails is a lie about the bug.
//! * **Do shrinkers keep their promises exhaustively?** Over every list in a
//!   small domain and every predicate shape, not over three examples.
//! * **Do the generators produce what their names claim?** Uniform draws,
//!   inclusive bounds, extremes of the range, and, the audit's headline,
//!   whether independent `Bytes` draws can ever *collide* (they almost never
//!   do, which is why an "equal inputs" branch needs a correlated generator).
//!
//! libtest, not the harness under test, runs these.

use std::collections::{BTreeMap, BTreeSet};

use coding_adventures_test_bootstrap::pbt::{
    check_property, frequency, gen_bounded_int, gen_bytes_slice, list, one_of, shrink_bytes,
    shrink_int, shrink_list, Gen, PbtOptions, FID_COUNTEREXAMPLE, FID_EXHAUSTED, FID_PANIC,
};
use coding_adventures_test_bootstrap::prng::SplitMix64;
use coding_adventures_test_bootstrap::Verdict;

fn options(num_tests: usize) -> PbtOptions {
    PbtOptions {
        num_tests,
        max_retries: 100,
        max_shrink_steps: 1000,
        is_ci: false,
    }
}

fn failure(verdict: Verdict) -> (&'static str, String) {
    match verdict {
        Verdict::Fail {
            failure_id,
            message,
        } => (failure_id, message),
        Verdict::Pass => panic!("expected a failure, got Pass"),
    }
}

// ---------------------------------------------------------------------------
// Verdict honesty
// ---------------------------------------------------------------------------

#[test]
fn a_false_property_is_always_a_counterexample_failure() {
    // Whatever the seed, a property that is false everywhere must fail at
    // iteration 1 and never report Pass.
    for seed in 0..200u64 {
        let verdict = check_property(
            seed,
            &options(10),
            |r| Ok(gen_bounded_int(r, 0, 1000)),
            |_| false,
            |c, _| (c, 0),
        );
        let (id, message) = failure(verdict);
        assert_eq!(id, FID_COUNTEREXAMPLE);
        assert!(message.contains("iteration 1"), "{message}");
    }
}

#[test]
fn a_rarely_false_property_is_found_for_every_seed() {
    // False only at 7 out of 0..=10: 100 draws miss it with probability
    // (10/11)^100 ~ 7e-5 per seed, so 200 deterministic seeds all find it
    // (checked here, not assumed).
    for seed in 0..200u64 {
        let verdict = check_property(
            seed,
            &options(100),
            |r| Ok(gen_bounded_int(r, 0, 10)),
            |&x| x != 7,
            |c, _| (c, 0),
        );
        let (id, message) = failure(verdict);
        assert_eq!(id, FID_COUNTEREXAMPLE, "seed {seed}");
        assert!(message.contains("initial=7"), "{message}");
    }
}

#[test]
fn a_true_property_passes_and_evaluates_every_case() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let calls = AtomicUsize::new(0);
    let verdict = check_property(
        1,
        &options(77),
        |r| Ok(gen_bounded_int(r, 0, 5)),
        |_| {
            calls.fetch_add(1, Ordering::Relaxed);
            true
        },
        |c, _| (c, 0),
    );
    assert_eq!(verdict, Verdict::Pass);
    assert_eq!(calls.load(Ordering::Relaxed), 77);
}

#[test]
fn zero_evaluated_cases_is_not_a_pass() {
    // REGRESSION: num_tests == 0 used to return Pass having evaluated
    // nothing: a property test with no tests that reports success.
    let verdict = check_property(
        1,
        &options(0),
        |r| Ok(gen_bounded_int(r, 0, 5)),
        |_| false, // would fail if it were ever run
        |c, _| (c, 0),
    );
    let (id, message) = failure(verdict);
    assert_eq!(id, FID_EXHAUSTED);
    assert!(message.contains("no cases"), "{message}");
}

#[test]
fn a_panicking_property_is_property_panic() {
    for threshold in [0i128, 3, 9] {
        let verdict = check_property(
            5,
            &options(200),
            |r| Ok(gen_bounded_int(r, 0, 9)),
            move |&x| {
                assert!(x < threshold || threshold > 9, "boom");
                true
            },
            |c, _| (c, 0),
        );
        let (id, _) = failure(verdict);
        assert_eq!(id, FID_PANIC, "threshold {threshold}");
    }
}

#[test]
fn a_shrinker_that_returns_a_passing_value_cannot_forge_the_witness() {
    // REGRESSION: the engine trusted the shrinker. A shrinker returning a
    // value that does NOT falsify the property produced "minimal=0" for a
    // property that is true at 0.
    let verdict = check_property(
        3,
        &options(50),
        |r| Ok(gen_bounded_int(r, 100, 200)),
        |&x| x < 100,      // false everywhere in 100..=200
        |_, _| (0i128, 5), // 0 satisfies the property: not a counterexample
    );
    let (id, message) = failure(verdict);
    assert_eq!(id, FID_COUNTEREXAMPLE);
    assert!(
        !message.contains("minimal=0"),
        "reported a non-counterexample as minimal: {message}"
    );
    assert!(message.contains("minimal="), "{message}");
}

#[test]
fn a_property_that_panics_while_shrinking_is_still_a_failure() {
    // The property is false for x >= 50 and *panics* for x < 10; the shrinker
    // walks into the panicking region. The verdict must stay a failure, not
    // crash the engine.
    let verdict = check_property(
        2,
        &options(200),
        |r| Ok(gen_bounded_int(r, 50, 100)),
        |&x| {
            assert!(x >= 10, "panics below 10");
            x < 50
        },
        |c, prop| shrink_int(c, 0, 0, 100, |x| prop(&x), 1000),
    );
    let (id, _) = failure(verdict);
    assert!(id == FID_COUNTEREXAMPLE || id == FID_PANIC, "{id}");
}

#[test]
fn generator_errors_are_failures_not_passes() {
    for error in [FID_EXHAUSTED.to_string(), "anything else".to_string()] {
        let verdict = check_property(
            0,
            &options(5),
            move |_| Err::<u8, _>(error.clone()),
            |_| true,
            |c, _| (c, 0),
        );
        assert_eq!(failure(verdict).0, FID_EXHAUSTED);
    }
}

#[test]
fn ci_mode_redacts_the_witness_and_keeps_the_same_failure_id() {
    let ci = PbtOptions {
        is_ci: true,
        ..options(50)
    };
    let verdict = check_property(
        42,
        &ci,
        |r| Ok(gen_bounded_int(r, 1_000_000, 2_000_000)),
        |&x| x < 1_000_000,
        |c, prop| shrink_int(c, 1_000_000, 1_000_000, 2_000_000, |x| prop(&x), 1000),
    );
    let (id, message) = failure(verdict);
    assert_eq!(id, FID_COUNTEREXAMPLE);
    for forbidden in ["initial=", "minimal=", "1000000", "42"] {
        assert!(
            !message.contains(forbidden),
            "CI message leaks {forbidden:?}: {message}"
        );
    }
    assert!(message.contains("repro-fnv: "), "{message}");
}

#[test]
fn replay_is_deterministic_by_seed() {
    let run = |seed| {
        check_property(
            seed,
            &options(100),
            |r| Ok(gen_bounded_int(r, 0, 10_000)),
            |&x| x % 997 != 0 || x == 0,
            |c, prop| shrink_int(c, 0, 0, 10_000, |x| prop(&x), 1000),
        )
    };
    for seed in 0..20 {
        assert_eq!(run(seed), run(seed));
    }
    let distinct: BTreeSet<String> = (0..20).map(|s| format!("{:?}", run(s))).collect();
    assert!(distinct.len() > 1, "all seeds produced the same run");
}

// ---------------------------------------------------------------------------
// The audit's headline: independent Bytes pairs and the "equal" branch
// ---------------------------------------------------------------------------

/// A deliberately broken constant-time-style comparison: it forgets the last byte.
fn buggy_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).rev().skip(1).all(|(x, y)| x == y)
}

#[test]
fn independent_fixed_length_bytes_pairs_never_collide() {
    // Measured: for 16-byte draws the collision probability is 2^-128. In
    // 200,000 pairs the "equal inputs" branch is never exercised.
    let mut rng = SplitMix64::new(1);
    let mut equal = 0;
    for _ in 0..200_000 {
        let a = gen_bytes_slice(&mut rng, 16, 16);
        let b = gen_bytes_slice(&mut rng, 16, 16);
        if a == b {
            equal += 1;
        }
    }
    assert_eq!(equal, 0);
}

#[test]
fn independent_short_bytes_pairs_collide_only_through_the_empty_string() {
    // For lengths 0..=8 (9 lengths) the collision probability is
    // sum_L (1/9)^2 * 256^-L = 1/81 * (1 + 1/256 + ...) ~ 0.0124, almost all
    // of it from the single pair (empty, empty). Measure, and check the
    // non-empty share is negligible.
    let mut rng = SplitMix64::new(2);
    let (mut equal, mut equal_nonempty) = (0u32, 0u32);
    let draws = 400_000;
    for _ in 0..draws {
        let a = gen_bytes_slice(&mut rng, 0, 8);
        let b = gen_bytes_slice(&mut rng, 0, 8);
        if a == b {
            equal += 1;
            if !a.is_empty() {
                equal_nonempty += 1;
            }
        }
    }
    let rate = f64::from(equal) / f64::from(draws);
    assert!((0.011..0.014).contains(&rate), "equal-pair rate {rate}");
    assert!(
        equal_nonempty < 100,
        "non-empty collisions: {equal_nonempty} of {equal}"
    );
}

#[test]
fn an_equality_property_over_independent_pairs_cannot_see_a_last_byte_bug_but_a_correlated_generator_can(
) {
    // Property: buggy_eq(a, b) == (a == b). The bug only shows on pairs that
    // differ in the LAST byte alone. Independent 16-byte draws never
    // produce such a pair, so the property "passes" against broken code.
    let independent = check_property(
        7,
        &options(1000),
        |r| Ok((gen_bytes_slice(r, 16, 16), gen_bytes_slice(r, 16, 16))),
        |(a, b)| buggy_eq(a, b) == (a == b),
        |c, _| (c, 0),
    );
    assert_eq!(
        independent,
        Verdict::Pass,
        "independent pairs detected the bug; the audit claim would be wrong"
    );

    // A correlated generator built from the public combinators finds it:
    // half the time b is a copy of a, half the time a copy with the last
    // byte flipped.
    let correlated = Gen::bytes_gen(16, 16).map(|a| {
        let mut b = a.clone();
        let last = b.len() - 1;
        b[last] ^= 0x01;
        (a, b)
    });
    let copy = Gen::bytes_gen(16, 16).map(|a| (a.clone(), a));
    let pair_gen = one_of(vec![correlated, copy]);
    let verdict = check_property(
        7,
        &options(1000),
        |r| pair_gen.sample(r),
        |(a, b)| buggy_eq(a, b) == (a == b),
        |c, _| (c, 0),
    );
    assert_eq!(failure(verdict).0, FID_COUNTEREXAMPLE);
}

// ---------------------------------------------------------------------------
// Generator distributions
// ---------------------------------------------------------------------------

#[test]
fn bounded_int_hits_both_ends_and_stays_inside_for_every_small_range() {
    for lo in -4i128..=4 {
        for hi in lo..=lo + 6 {
            let mut rng = SplitMix64::new((lo * 31 + hi + 1000) as u64);
            let mut seen = BTreeSet::new();
            for _ in 0..2000 {
                let v = gen_bounded_int(&mut rng, lo, hi);
                assert!((lo..=hi).contains(&v), "{v} outside {lo}..={hi}");
                seen.insert(v);
            }
            assert_eq!(seen.len() as i128, hi - lo + 1, "{lo}..={hi} not covered");
        }
    }
}

#[test]
fn bounded_int_is_uniform_on_a_non_power_of_two_span() {
    // chi-square on span 7 with 70,000 draws: expected 10,000 per bucket.
    let mut rng = SplitMix64::new(11);
    let mut counts = [0u32; 7];
    for _ in 0..70_000 {
        counts[gen_bounded_int(&mut rng, 100, 106) as usize - 100] += 1;
    }
    let chi: f64 = counts
        .iter()
        .map(|&c| (f64::from(c) - 10_000.0).powi(2) / 10_000.0)
        .sum();
    // 6 degrees of freedom: the 99.99th percentile is about 27.9.
    assert!(chi < 27.9, "chi-square {chi} for {counts:?}");
}

#[test]
fn bounded_int_survives_every_extreme_span() {
    // REGRESSION: (max - min) overflowed i128 for any span above 2^127 and
    // panicked in debug builds (wrapped to a zero modulus in release).
    let cases: [(i128, i128); 12] = [
        (i128::MIN, i128::MAX),
        (0, i128::MAX),
        (i128::MIN, 0),
        (-1, i128::MAX),
        (i128::MIN, 1),
        (i128::MIN, i128::MIN),
        (i128::MAX, i128::MAX),
        (0, (1i128 << 64) - 1),
        (0, 1i128 << 64),
        (-(1i128 << 63), (1i128 << 63) - 1),
        (-(1i128 << 100), 1i128 << 100),
        (i128::MIN + 1, i128::MAX - 1),
    ];
    for (lo, hi) in cases {
        let mut rng = SplitMix64::new(1);
        for _ in 0..200 {
            let v = gen_bounded_int(&mut rng, lo, hi);
            assert!(lo <= v && v <= hi, "{v} outside {lo}..={hi}");
        }
    }
    // Singleton ranges are exact.
    let mut rng = SplitMix64::new(5);
    assert_eq!(gen_bounded_int(&mut rng, i128::MIN, i128::MIN), i128::MIN);
    assert_eq!(gen_bounded_int(&mut rng, i128::MAX, i128::MAX), i128::MAX);
}

#[test]
fn bounded_int_is_unbiased_for_spans_above_two_to_the_64() {
    // REGRESSION: spans above 2^64 used `raw % span` over a 128-bit word,
    // which VER05 §3.1 forbids ("rejection sampling"). For span = 3 * 2^126
    // the residues below 2^126 were drawn with probability 1/2 instead of
    // the uniform 1/3.
    // The range starts at i128::MIN so that a span of 3 * 2^126 fits.
    let min = i128::MIN;
    let max = (1i128 << 126) - 1; // span = max - min + 1 = 2^126 + 2^127 = 3 * 2^126
    let low_edge = min + (1i128 << 126);
    let mut rng = SplitMix64::new(99);
    let draws = 30_000;
    let mut low = 0u32;
    for _ in 0..draws {
        if gen_bounded_int(&mut rng, min, max) < low_edge {
            low += 1;
        }
    }
    let share = f64::from(low) / f64::from(draws);
    assert!(
        (0.32..0.347).contains(&share),
        "P(r < 2^126) = {share}, want 1/3"
    );
}

#[test]
fn bytes_lengths_and_values_are_uniform_and_inclusive() {
    let mut rng = SplitMix64::new(21);
    let mut lengths: BTreeMap<usize, u32> = BTreeMap::new();
    let mut values = [0u32; 256];
    let draws = 40_000;
    for _ in 0..draws {
        let bytes = gen_bytes_slice(&mut rng, 2, 5);
        *lengths.entry(bytes.len()).or_default() += 1;
        for b in bytes {
            values[b as usize] += 1;
        }
    }
    assert_eq!(lengths.keys().copied().collect::<Vec<_>>(), [2, 3, 4, 5]);
    for (len, count) in &lengths {
        assert!((9_000..=11_000).contains(count), "length {len}: {count}");
    }
    let total: u32 = values.iter().sum();
    let expected = f64::from(total) / 256.0;
    let chi: f64 = values
        .iter()
        .map(|&c| (f64::from(c) - expected).powi(2) / expected)
        .sum();
    // 255 degrees of freedom: 99.99th percentile about 342.
    assert!(chi < 342.0, "byte-value chi-square {chi}");
    // Degenerate and empty ranges.
    for _ in 0..50 {
        assert!(gen_bytes_slice(&mut rng, 0, 0).is_empty());
        assert_eq!(gen_bytes_slice(&mut rng, 7, 7).len(), 7);
    }
}

#[test]
fn frequency_follows_its_weights_and_never_picks_a_zero_weight() {
    let gen = frequency(vec![
        (1, Gen::bounded(1, 1)),
        (0, Gen::bounded(2, 2)),
        (3, Gen::bounded(3, 3)),
    ]);
    let mut rng = SplitMix64::new(4);
    let mut counts: BTreeMap<i128, u32> = BTreeMap::new();
    for _ in 0..40_000 {
        *counts.entry(gen.sample(&mut rng).unwrap()).or_default() += 1;
    }
    assert!(!counts.contains_key(&2), "zero-weight arm was chosen");
    let share = f64::from(counts[&3]) / 40_000.0;
    assert!((0.73..0.77).contains(&share), "weight 3 of 4 gave {share}");
}

#[test]
fn one_of_is_uniform_over_its_arms() {
    let gen = one_of((0..4).map(|k| Gen::bounded(k, k)).collect());
    let mut rng = SplitMix64::new(8);
    let mut counts = [0u32; 4];
    for _ in 0..40_000 {
        counts[gen.sample(&mut rng).unwrap() as usize] += 1;
    }
    for count in counts {
        assert!((9_400..=10_600).contains(&count), "{counts:?}");
    }
}

#[test]
#[should_panic(expected = "total weight")]
fn frequency_rejects_all_zero_weights() {
    let _ = frequency(vec![(0, Gen::bounded(0, 0))]);
}

#[test]
fn filter_retries_exactly_max_retries_times_then_reports_exhaustion() {
    use std::cell::Cell;
    use std::rc::Rc;
    let seen = Rc::new(Cell::new(0usize));
    let counter = seen.clone();
    let gen = Gen::bounded(0, 9).filter(
        move |_| {
            counter.set(counter.get() + 1);
            false
        },
        13,
    );
    let mut rng = SplitMix64::new(0);
    assert_eq!(gen.sample(&mut rng), Err(FID_EXHAUSTED.to_string()));
    assert_eq!(seen.get(), 13);
    // And the verdict path: an unsatisfiable precondition is a failure.
    let verdict = check_property(
        0,
        &options(5),
        |r| Gen::bounded(0, 9).filter(|_| false, 3).sample(r),
        |_| true,
        |c, _| (c, 0),
    );
    assert_eq!(failure(verdict).0, FID_EXHAUSTED);
}

#[test]
fn filter_returns_only_values_satisfying_the_predicate() {
    let gen = Gen::bounded(0, 99).filter(|x| x % 7 == 0, 1000);
    let mut rng = SplitMix64::new(3);
    for _ in 0..500 {
        assert_eq!(gen.sample(&mut rng).unwrap() % 7, 0);
    }
}

#[test]
fn list_lengths_are_inclusive_and_element_errors_propagate() {
    let gen = list(Gen::bounded(0, 1), 0, 3);
    let mut rng = SplitMix64::new(6);
    let lengths: BTreeSet<usize> = (0..400)
        .map(|_| gen.sample(&mut rng).unwrap().len())
        .collect();
    assert_eq!(lengths, BTreeSet::from([0, 1, 2, 3]));
    let failing = list(Gen::bounded(0, 1).filter(|_| false, 1), 2, 2);
    assert_eq!(failing.sample(&mut rng), Err(FID_EXHAUSTED.to_string()));
}

#[test]
fn generators_are_pure_functions_of_the_seed() {
    let gen = list(Gen::bytes_gen(0, 6), 0, 5);
    let a: Vec<_> = {
        let mut rng = SplitMix64::new(77);
        (0..50).map(|_| gen.sample(&mut rng).unwrap()).collect()
    };
    let b: Vec<_> = {
        let mut rng = SplitMix64::new(77);
        (0..50).map(|_| gen.sample(&mut rng).unwrap()).collect()
    };
    assert_eq!(a, b);
}

// ---------------------------------------------------------------------------
// Shrinking guarantees
// ---------------------------------------------------------------------------

#[test]
fn shrink_int_reaches_the_exact_threshold_for_every_monotone_property() {
    // "x < t holds" is falsified by every x >= t. For every threshold t in
    // 1..=120 and every failing start in t..=120 the minimal counterexample
    // toward target 0 is exactly t.
    let mut cases = 0u32;
    for t in 1..=120i128 {
        for start in t..=120 {
            let (min, steps) = shrink_int(start, 0, 0, 120, |x| x < t, 1000);
            assert_eq!(min, t, "start {start}, threshold {t}");
            assert!(steps <= 1000);
            cases += 1;
        }
    }
    // The mirror image: "x > -t holds", failing at x <= -t, target 0.
    for t in 1..=120i128 {
        for start in (-120..=-t).rev() {
            let (min, _) = shrink_int(start, 0, -120, 0, |x| x > -t, 1000);
            assert_eq!(min, -t, "start {start}, threshold -{t}");
            cases += 1;
        }
    }
    assert_eq!(cases, 2 * (120 * 121 / 2));
}

#[test]
fn shrink_int_never_returns_a_value_that_passes_for_any_failing_set() {
    // Exhaustive over every failing set F of the domain 0..=7 (256 sets) and
    // every start in F: the result is in F, in bounds, and no farther from
    // the target than the start. (Local minimum, not global: see VER05 §8.)
    let mut cases = 0u32;
    for mask in 1u32..256 {
        let fails = |x: i128| mask & (1 << x) != 0;
        for start in (0..8i128).filter(|&x| fails(x)) {
            let (min, steps) = shrink_int(start, 0, 0, 7, |x| !fails(x), 1000);
            assert!(fails(min), "mask {mask:08b}, start {start}: {min} passes");
            assert!((0..=7).contains(&min));
            assert!(min <= start, "moved away from the target");
            assert!(steps <= 1000);
            cases += 1;
        }
    }
    assert_eq!(cases, 8 * 128);
}

#[test]
fn shrink_int_respects_refinement_bounds() {
    // Integrated shrinking: a refined type x in 10..=50 must never be
    // shrunk to a value outside it, even though 0 is the default target.
    for start in 10..=50i128 {
        let (min, _) = shrink_int(start, 0, 10, 50, |x| x < 10, 1000);
        assert!((10..=50).contains(&min));
        assert_eq!(min, 10);
    }
    // A target outside the range is ignored rather than escaped to.
    let (min, _) = shrink_int(40, 99, 10, 50, |x| x < 10, 1000);
    assert!((10..=50).contains(&min));
}

#[test]
fn shrink_int_survives_extreme_values() {
    // REGRESSION: `diff.abs()` panics for i128::MIN, and `current - target`
    // overflowed when the target and start were opposite extremes.
    let (min, _) = shrink_int(i128::MIN, 0, i128::MIN, 0, |x| x > -1000, 1000);
    assert!(min <= -1000);
    let (min, _) = shrink_int(i128::MAX, 0, 0, i128::MAX, |x| x < 5, 1000);
    assert!(min >= 5);
    let (min, _) = shrink_int(i128::MAX, i128::MIN, i128::MIN, i128::MAX, |x| x < 5, 1000);
    assert!(min >= 5);
}

#[test]
fn shrink_int_honours_a_zero_step_budget() {
    // With no budget the start comes back unchanged and unshrunk, even when
    // the target itself would have been accepted by the one-step shortcut.
    let (min, steps) = shrink_int(100, 0, 0, 200, |x| x < 0, 0);
    assert_eq!((min, steps), (100, 0));
}

#[test]
fn shrink_int_step_budget_still_returns_a_counterexample() {
    for budget in 0..10 {
        let (min, steps) = shrink_int(5000, 0, 0, 10_000, |x| x < 17, budget);
        assert!(min >= 17, "budget {budget}: {min} passes");
        assert!(steps <= budget.max(1));
    }
}

/// Every list over {0,1,2,3} of length 0..=4: 1 + 4 + 16 + 64 + 256 = 341.
fn small_lists() -> Vec<Vec<i128>> {
    let mut all: Vec<Vec<i128>> = vec![vec![]];
    let mut frontier: Vec<Vec<i128>> = vec![vec![]];
    for _ in 0..4 {
        let mut next = Vec::new();
        for prefix in &frontier {
            for v in 0..4 {
                let mut l = prefix.clone();
                l.push(v);
                next.push(l);
            }
        }
        all.extend(next.iter().cloned());
        frontier = next;
    }
    all
}

type ListProperty = (&'static str, fn(&[i128]) -> bool);

const LIST_PROPERTIES: [ListProperty; 6] = [
    ("no element >= 2", |l| l.iter().all(|&x| x < 2)),
    ("no adjacent equal pair", |l| {
        l.windows(2).all(|w| w[0] != w[1])
    }),
    ("sum < 5", |l| l.iter().sum::<i128>() < 5),
    ("length < 3", |l| l.len() < 3),
    ("sorted ascending", |l| l.windows(2).all(|w| w[0] <= w[1])),
    ("fewer than two 3s", |l| {
        l.iter().filter(|&&x| x == 3).count() < 2
    }),
];

#[test]
fn shrink_list_result_always_falsifies_and_is_no_larger_over_every_small_list() {
    let lists = small_lists();
    assert_eq!(lists.len(), 341);
    let mut cases = 0;
    for (name, holds) in LIST_PROPERTIES {
        for list in lists.iter().filter(|l| !holds(l)) {
            let (min, steps) = shrink_list(list, 0, holds, 1000);
            assert!(
                !holds(&min),
                "{name}: {list:?} -> {min:?} satisfies the property"
            );
            assert!(min.len() <= list.len(), "{name}: grew {list:?} -> {min:?}");
            assert!(steps <= 1000);
            // Elements never leave the original alphabet {0..3}.
            assert!(min.iter().all(|&x| (0..4).contains(&x)));
            cases += 1;
        }
    }
    assert!(cases > 600, "only {cases} failing cases");
}

#[test]
fn shrink_list_reaches_the_known_minimal_witness_for_length_and_membership() {
    // For "length < 3" the minimal witness is three zeros; for
    // "no element >= 2" it is [2]. Both from every failing start.
    let lists = small_lists();
    for list in lists.iter().filter(|l| l.len() >= 3) {
        let (min, _) = shrink_list(list, 0, |l| l.len() < 3, 1000);
        assert_eq!(min, [0, 0, 0], "from {list:?}");
    }
    for list in lists.iter().filter(|l| l.iter().any(|&x| x >= 2)) {
        let (min, _) = shrink_list(list, 0, |l| l.iter().all(|&x| x < 2), 1000);
        assert_eq!(min, [2], "from {list:?}");
    }
}

#[test]
fn shrink_list_result_is_one_minimal_by_deletion_for_every_small_list() {
    // The contract of VER05 §4.3 phase 1 as the implementation delivers it:
    // no single element of the result can be deleted while still failing.
    let lists = small_lists();
    for (name, holds) in LIST_PROPERTIES {
        for list in lists.iter().filter(|l| !holds(l)) {
            let (min, _) = shrink_list(list, 0, holds, 1000);
            for i in 0..min.len() {
                let mut without = min.clone();
                without.remove(i);
                assert!(
                    holds(&without),
                    "{name}: {list:?} -> {min:?} but deleting index {i} still fails"
                );
            }
        }
    }
}

#[test]
fn shrink_list_and_bytes_budgets_never_return_a_passing_value() {
    for budget in 0..8 {
        let (min, _) = shrink_list(&[3, 3, 3, 3], 0, |l| l.len() < 2, budget);
        assert!(min.len() >= 2, "budget {budget}: {min:?}");
        let (min, _) = shrink_bytes(&[9, 9, 9, 9], |b| b.len() < 2, budget);
        assert!(min.len() >= 2, "budget {budget}: {min:?}");
    }
}

#[test]
fn shrink_bytes_finds_the_shortest_zero_filled_witness() {
    for len in 5..=12usize {
        let start: Vec<u8> = (1..=len as u8).collect();
        let (min, _) = shrink_bytes(&start, |b| b.len() < 5, 1000);
        assert_eq!(min, vec![0u8; 5], "from length {len}");
    }
    // A content-dependent property: "no byte equals 0xAB".
    let (min, _) = shrink_bytes(&[1, 0xAB, 3], |b| !b.contains(&0xAB), 1000);
    assert_eq!(min, [0xAB]);
}

#[test]
#[should_panic(expected = "must falsify")]
fn shrinking_a_passing_start_is_rejected_loudly() {
    let _ = shrink_int(5, 0, 0, 10, |_| true, 10);
}
