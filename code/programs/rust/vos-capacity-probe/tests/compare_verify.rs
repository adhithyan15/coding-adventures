//! VOS-028c3c-3b-1a (VOS-D141): `results::compare_verify` is the one per-row rule of a
//! verification, factored out of the two sites that used to spell it out (the in-process quick
//! rows of `results::rerun_collect_budgeted` and the full-tier children of `spot::rerun_one_row`).
//!
//! The refactor must not change what any row is judged by.  Before it, a verifying site computed
//!
//! ```text
//!   compare_rerun(rec, now)  ++  compare_timing(rec, now)
//! ```
//!
//! and nothing else, so that is the OLD behaviour, spelled out here as `old_rule`; each test pairs
//! a recorded row with a re-run of one comparison class (Exact, WallWhole, Minimal) and asserts
//! `compare_verify` returns exactly what `old_rule` does.  The classes themselves are pinned by
//! `compare_classes.rs`; what is pinned here is that composing them changed nothing, in
//! particular that the timing band still applies to an Exact pair and to nothing else.
//!
//! Pure, so it runs on every platform.

use vos_capacity_probe::results::{
    compare_class, compare_rerun, compare_timing, compare_verify, green, CompareClass,
};
use vos_capacity_probe::row::{Knob, Kv, Row};

/// What every verifying site computed before `compare_verify` existed.
fn old_rule(rec: &Row, now: &Row) -> Vec<String> {
    let mut v = compare_rerun(rec, now);
    v.extend(compare_timing(rec, now));
    v
}

/// A completed (`unsat`, no cap), exactly-comparable row.
fn exact(exp: &str, seed: u64) -> Row {
    let mut r = Row::new(
        exp,
        Knob::of(&[("length", Kv::I(50))]),
        "monolithic",
        seed,
        5,
    );
    r.result = "unsat".into();
    r.cap = "none".into();
    r.instance_sha256 = "aa".repeat(32);
    r.control_instance_sha256 = "cc".repeat(32);
    r.ands = 100;
    r.vars = 200;
    r.clauses = 300;
    r.entropy_bits = 8;
    r.declared_bits = 16;
    r.conflicts = 1_000;
    r.propagations = 90_000;
    r.steps = 3;
    r.solve_ms = 400.0;
    r.worst_step_solve_ms = 400.0;
    r.peak_rss_kb = 100_000;
    // `compare_timing` recomputes `green` from the row's own figures and compares it with the
    // recorded flag, so a row that is meant to be clean must carry the flag it would compute.
    r.green = green(&r);
    r
}

/// A row stopped by the wall clock.
fn walled(exp: &str, seed: u64) -> Row {
    let mut r = exact(exp, seed);
    r.result = "unknown".into();
    r.cap = "wall".into();
    r.conflicts = 860_024;
    r.solve_ms = 120_000.0;
    r.worst_step_solve_ms = 120_000.0;
    r
}

#[test]
fn an_exact_pair_gets_the_counter_comparison_and_the_timing_band() {
    let rec = exact("B2", 1);
    assert_eq!(compare_class(&rec, &rec), CompareClass::Exact);
    assert!(compare_verify(&rec, &rec.clone()).is_empty());

    // A counter differing exactly: found by compare_rerun, reported through compare_verify.
    let mut counter = rec.clone();
    counter.conflicts += 1;
    let v = compare_verify(&rec, &counter);
    assert_eq!(v, old_rule(&rec, &counter));
    assert!(v.join("|").contains("conflicts differs"), "{v:?}");

    // A time outside the two-sided band: found by compare_timing only, and still found.
    let mut slow = rec.clone();
    slow.solve_ms = 400.0 * 10.0 + 1_000.0;
    let v = compare_verify(&rec, &slow);
    assert_eq!(v, old_rule(&rec, &slow));
    assert!(compare_rerun(&rec, &slow).is_empty(), "no counter moved");
    assert!(v.join("|").contains("solve_ms is outside"), "{v:?}");

    // The resident set is banded as well, and a changed `green` is a finding.
    let mut fat = rec.clone();
    fat.peak_rss_kb = 100_000 * 8;
    let v = compare_verify(&rec, &fat);
    assert_eq!(v, old_rule(&rec, &fat));
    assert!(v.join("|").contains("peak_rss_kb is outside"), "{v:?}");

    // Both kinds at once: the counters first, then the band, as before.
    let mut both = counter.clone();
    both.solve_ms = slow.solve_ms;
    let v = compare_verify(&rec, &both);
    assert_eq!(v, old_rule(&rec, &both));
    let at_counter = v.iter().position(|m| m.contains("conflicts differs"));
    let at_band = v.iter().position(|m| m.contains("solve_ms is outside"));
    assert!(at_counter < at_band, "{v:?}");
}

#[test]
fn a_wall_whole_pair_ignores_counters_and_is_not_banded() {
    let rec = walled("B2", 1);
    assert_eq!(compare_class(&rec, &rec), CompareClass::WallWhole);
    // Counters, time and resident set all differ, as an honest re-run's would: no finding.
    let mut now = rec.clone();
    now.conflicts = 740_026;
    now.propagations = 7;
    now.solve_ms = 1.0;
    now.worst_step_solve_ms = 1.0;
    now.peak_rss_kb = 9_000_000;
    assert_eq!(compare_class(&rec, &now), CompareClass::WallWhole);
    assert!(compare_verify(&rec, &now).is_empty());
    assert_eq!(compare_verify(&rec, &now), old_rule(&rec, &now));

    // What the class still holds is still held: the instance digest.
    let mut edited = now.clone();
    edited.instance_sha256 = "00".repeat(32);
    let v = compare_verify(&rec, &edited);
    assert_eq!(v, old_rule(&rec, &edited));
    assert!(v.join("|").contains("instance_sha256 differs"), "{v:?}");
}

#[test]
fn a_minimal_pair_is_compared_on_identity_result_and_cap_alone() {
    // A segmented (chain) wall-capped row: D4.
    let rec = walled("D4", 1);
    assert_eq!(compare_class(&rec, &rec), CompareClass::Minimal);
    let mut now = rec.clone();
    now.instance_sha256 = "00".repeat(32);
    now.ands += 5;
    now.conflicts = 3;
    now.solve_ms = 1.0;
    assert!(compare_verify(&rec, &now).is_empty());
    assert_eq!(compare_verify(&rec, &now), old_rule(&rec, &now));
    // The result flipping is one finding, and it is reported identically.
    let mut flip = rec.clone();
    flip.result = "unsat".into();
    flip.cap = "none".into();
    let v = compare_verify(&rec, &flip);
    assert_eq!(v, old_rule(&rec, &flip));
    assert!(!v.is_empty());

    // A killed re-run of an exact recording is Minimal too: unmeasurable, not banded.
    let rec = exact("B2", 1);
    let mut killed = rec.clone();
    killed.result = "killed".into();
    killed.solve_ms = 1.0;
    assert_eq!(compare_class(&rec, &killed), CompareClass::Minimal);
    let v = compare_verify(&rec, &killed);
    assert_eq!(v, old_rule(&rec, &killed));
    assert!(v.join("|").contains("result differs"), "{v:?}");
    assert!(!v.join("|").contains("band"), "{v:?}");
}

#[test]
fn compare_verify_is_the_old_rule_over_a_grid_of_pairs() {
    // Every combination of recorded and re-run kind, with and without a counter and a time gap:
    // the composition is the old rule, row for row.
    let kinds: Vec<(&str, Row)> = vec![
        ("exact", exact("B2", 1)),
        ("wall-whole", walled("B2", 1)),
        ("wall-chain", walled("D4", 1)),
        ("killed", {
            let mut r = exact("B2", 1);
            r.result = "killed".into();
            r
        }),
        ("oom", {
            let mut r = exact("B2", 1);
            r.result = "oom".into();
            r
        }),
        ("crashed", {
            let mut r = exact("B2", 1);
            r.result = "crashed".into();
            r
        }),
    ];
    let mut seen_classes = std::collections::BTreeSet::new();
    for (_, rec) in &kinds {
        for (_, now) in &kinds {
            for tweak in 0..4 {
                let mut n = now.clone();
                match tweak {
                    1 => n.conflicts += 7,
                    2 => n.solve_ms *= 20.0,
                    3 => n.peak_rss_kb *= 9,
                    _ => {}
                }
                seen_classes.insert(format!("{:?}", compare_class(rec, &n)));
                assert_eq!(
                    compare_verify(rec, &n),
                    old_rule(rec, &n),
                    "{} / {} / tweak {tweak}",
                    rec.result,
                    n.result
                );
            }
        }
    }
    assert_eq!(seen_classes.len(), 3, "the grid reaches all three classes");
}
