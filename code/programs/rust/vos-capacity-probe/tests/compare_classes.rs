//! VOS-028c3c-2a (VOS-D124, VOS-D128, VOS-D131): the comparison classes of `compare_rerun`.
//!
//! A row stopped by the wall clock, or ended `killed`, `oom` or `crashed`, has counters that are
//! whatever was done at that moment, so an honest re-run never reproduces them.  Each test below
//! pairs a recorded row with a re-run that differs ONLY in what such a row may not be held to
//! (or only in what it still must be), and states which side of the line the difference is on.
//! Everything here is pure (no child process), so it runs on every platform.

use std::path::PathBuf;
use std::time::Duration;
use vos_capacity_probe::results::{
    compare_class, compare_rerun, compare_timing, rerun_wall, CompareClass, Results,
};
use vos_capacity_probe::row::{Knob, Kv, Row};

/// A row that stopped on the wall: `unknown`, cap `wall`, with counters the clock decided.
fn walled(exp: &str, seed: u64) -> Row {
    let mut r = Row::new(
        exp,
        Knob::of(&[("length", Kv::I(500))]),
        "monolithic",
        seed,
        5,
    );
    r.result = "unknown".into();
    r.cap = "wall".into();
    r.instance_sha256 = "aa".repeat(32);
    r.control_instance_sha256 = "cc".repeat(32);
    r.ands = 100;
    r.vars = 200;
    r.clauses = 300;
    r.entropy_bits = 8;
    r.declared_bits = 16;
    r.conflicts = 860_024;
    r.propagations = 9_000_000;
    r.steps = 3;
    r.note = "partial".into();
    r.solve_ms = 120_000.0;
    r.worst_step_solve_ms = 120_000.0;
    r.metrics = vec![("cases".into(), 7.0)];
    r
}

/// Make `now` differ from `rec` in everything a wall-capped re-run may legitimately differ in.
fn clock_noise(now: &mut Row) {
    now.conflicts = 740_026;
    now.propagations = 7_000_000;
    now.steps = 2;
    now.note = "partial, differently".into();
    now.solve_ms = 119_000.0;
    now.worst_step_conflicts = Some(5);
    now.drat_bytes = Some(10);
    now.check_work = Some(11);
    now.pass_default_limits = Some(false);
    now.collapsed_reason = Some("whatever".into());
    now.control_instance_sha256 = "dd".repeat(32);
    now.drat_sha256 = Some("ee".repeat(32));
    now.metrics = vec![("other".into(), 1.0)];
}

#[test]
fn a_wall_capped_single_instance_row_ignores_counters_but_not_its_digest() {
    let rec = walled("B2", 1);
    assert_eq!(compare_class(&rec, &rec), CompareClass::WallWhole);
    let mut now = rec.clone();
    clock_noise(&mut now);
    assert!(compare_rerun(&rec, &now).is_empty());
    // Each structure field is still held: an edited instance cannot hide behind a timeout.
    for (field, edit) in [
        ("instance_sha256", 0usize),
        ("ands", 1),
        ("vars", 2),
        ("clauses", 3),
        ("entropy_bits", 4),
        ("declared_bits", 5),
        ("cap", 6),
        ("wall_s", 7),
    ] {
        let mut n = rec.clone();
        clock_noise(&mut n);
        match edit {
            0 => n.instance_sha256 = "00".repeat(32),
            1 => n.ands += 1,
            2 => n.vars += 1,
            3 => n.clauses += 1,
            4 => n.entropy_bits += 1,
            5 => n.declared_bits += 1,
            // `unknown`/`wall` against `unknown`/`conflicts` is a finding.
            6 => n.cap = "conflicts".into(),
            _ => n.wall_s = Some(600),
        }
        let v = compare_rerun(&rec, &n).join(" | ");
        assert!(v.contains(&format!("{field} differs")), "{field}: {v}");
    }
    // The identity is held too.
    let mut n = rec.clone();
    n.seed = 2;
    assert!(compare_rerun(&rec, &n).join("|").contains("seed differs"));
}

#[test]
fn a_wall_capped_chain_row_is_compared_on_identity_result_and_cap() {
    // D4 is segmented: its digest and structure fields are partial, so they are not compared.
    let rec = walled("D4", 1);
    assert_eq!(compare_class(&rec, &rec), CompareClass::Minimal);
    let mut now = rec.clone();
    clock_noise(&mut now);
    now.instance_sha256 = "00".repeat(32);
    now.ands += 5;
    now.vars += 5;
    now.clauses += 5;
    now.entropy_bits += 1;
    now.declared_bits += 1;
    assert!(compare_rerun(&rec, &now).is_empty());
    // Identity, cap and the budget are held.
    let mut n = rec.clone();
    n.mode = "certified".into();
    assert!(compare_rerun(&rec, &n).join("|").contains("mode differs"));
    let mut n = rec.clone();
    n.cap = "conflicts".into();
    assert!(compare_rerun(&rec, &n).join("|").contains("cap differs"));
    let mut n = rec.clone();
    n.wall_s = Some(600);
    assert!(compare_rerun(&rec, &n).join("|").contains("wall_s differs"));
    // The other segmented experiments are in the same class.
    for e in ["B3", "D3"] {
        let r = walled(e, 1);
        assert_eq!(compare_class(&r, &r), CompareClass::Minimal, "{e}");
    }
}

#[test]
fn a_flip_between_unsat_and_unknown_wall_is_one_finding() {
    let mut unsat = walled("B2", 1);
    unsat.result = "unsat".into();
    unsat.cap = "none".into();
    unsat.drat_sha256 = Some("bb".repeat(32));
    let wall = walled("B2", 1);
    for (rec, now) in [(&unsat, &wall), (&wall, &unsat)] {
        let v = compare_rerun(rec, now);
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(v[0].contains("result differs"), "{v:?}");
        assert_eq!(compare_class(rec, now), CompareClass::Minimal);
    }
    // The same for a segmented row.
    let mut u4 = walled("D4", 1);
    u4.result = "unsat".into();
    u4.cap = "none".into();
    let v = compare_rerun(&u4, &walled("D4", 1));
    assert_eq!(v.len(), 1, "{v:?}");
}

#[test]
fn an_unknown_conflicts_row_is_still_compared_exactly() {
    let mut rec = walled("B2", 1);
    rec.cap = "conflicts".into();
    assert_eq!(compare_class(&rec, &rec), CompareClass::Exact);
    assert!(compare_rerun(&rec, &rec.clone()).is_empty());
    let mut now = rec.clone();
    now.conflicts += 1;
    assert!(compare_rerun(&rec, &now)
        .join("|")
        .contains("conflicts differs"));
    let mut now = rec.clone();
    now.note = "x".into();
    assert!(compare_rerun(&rec, &now).join("|").contains("note differs"));
}

#[test]
fn timing_is_not_banded_for_a_wall_capped_rerun() {
    let rec = walled("B2", 1);
    let mut now = rec.clone();
    now.solve_ms = 1.0;
    now.worst_step_solve_ms = 1.0;
    now.peak_rss_kb = 9_000_000;
    // Either side being wall-capped is enough, and so is the re-run alone.
    assert!(compare_timing(&rec, &now).is_empty());
    assert!(compare_timing(&now, &rec).is_empty());
    let mut exact = rec.clone();
    exact.cap = "none".into();
    exact.result = "unsat".into();
    assert!(
        compare_timing(&exact, &rec).is_empty(),
        "the re-run is wall-capped"
    );
    // Control: two exact rows with the same gap ARE banded.
    let mut slow = exact.clone();
    slow.solve_ms = 120_000.0;
    let mut fast = exact.clone();
    fast.solve_ms = 1.0;
    assert!(!compare_timing(&slow, &fast).is_empty());
}

#[test]
fn a_killed_or_oom_row_is_compared_on_identity_result_cap_and_wall() {
    // A killed row has no digests and partial counters; a re-run that was killed too (or an oom
    // pair, whose cap is `rss`) must not be held to them.
    let mut killed = Row::new("B2", Knob::of(&[("length", Kv::I(50))]), "monolithic", 1, 5);
    killed.result = "killed".into();
    killed.instance_sha256 = "pending".into();
    killed.conflicts = 10;
    let mut again = killed.clone();
    again.instance_sha256 = "unavailable".into();
    again.conflicts = 99;
    again.ands = 4;
    again.note = "other".into();
    assert_eq!(compare_class(&killed, &again), CompareClass::Minimal);
    assert!(compare_rerun(&killed, &again).is_empty());
    let mut oom = killed.clone();
    oom.result = "oom".into();
    oom.cap = "rss".into();
    let mut oom2 = oom.clone();
    oom2.conflicts = 500;
    oom2.steps = 9;
    assert!(compare_rerun(&oom, &oom2).is_empty());
    assert!(compare_timing(&oom, &oom2).is_empty());
    let mut crashed = killed.clone();
    crashed.result = "crashed".into();
    assert_eq!(compare_class(&crashed, &crashed), CompareClass::Minimal);
    // The verdict is held: a recorded `unsat` that re-runs `killed` (and the reverse) is ONE finding.
    let mut unsat = killed.clone();
    unsat.result = "unsat".into();
    for (a, b) in [(&unsat, &killed), (&killed, &unsat), (&oom, &unsat)] {
        let v = compare_rerun(a, b);
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(v[0].contains("result differs"), "{v:?}");
    }
    // `oom` against `killed` differ in the verdict, and only there.
    assert_eq!(compare_rerun(&oom, &killed).len(), 1);
    // The budget is held.
    let mut walled_killed = killed.clone();
    walled_killed.wall_s = Some(600);
    assert!(compare_rerun(&killed, &walled_killed)
        .join("|")
        .contains("wall_s differs"));
}

#[test]
fn the_committed_b2_length_500_seed_1_and_2_rows_compare_clean() {
    // The motivating pair (VOS30 9.12.1): the committed B2 {length: 500, divergence: 1.0} rows
    // for seeds 1 and 2 are `unknown` on the wall at different conflict counts (which counts
    // depends on the recording's host, so none is asserted).  Re-running seed 1 under a slightly
    // different clock gives seed 2's counters; that must not be a finding.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/vos30-capacity-results.json");
    let res = Results::from_text(&std::fs::read_to_string(path).unwrap()).unwrap();
    let find = |seed: u64| {
        res.rows
            .iter()
            .find(|r| {
                r.experiment == "B2"
                    && r.seed == seed
                    && r.knob.int("length") == 500
                    && r.knob.key().contains("1.0")
                    && r.cap == "wall"
            })
            .unwrap()
            .clone()
    };
    let (one, two) = (find(1), find(2));
    assert_ne!(
        one.conflicts, two.conflicts,
        "the pair needs different counters"
    );
    let mut now = one.clone();
    now.conflicts = two.conflicts;
    now.propagations = two.propagations;
    now.steps = two.steps;
    now.solve_ms = two.solve_ms;
    now.worst_step_solve_ms = two.worst_step_solve_ms;
    now.note = two.note.clone();
    now.metrics = two.metrics.clone();
    now.drat_bytes = two.drat_bytes;
    assert_eq!(compare_class(&one, &now), CompareClass::WallWhole);
    assert!(compare_rerun(&one, &now).is_empty());
    assert!(compare_timing(&one, &now).is_empty());
    // Control: the digest still binds (the two seeds are different instances).
    let mut other = now.clone();
    other.instance_sha256 = two.instance_sha256.clone();
    assert_ne!(one.instance_sha256, two.instance_sha256);
    assert!(compare_rerun(&one, &other)
        .join("|")
        .contains("instance_sha256 differs"));
}

#[test]
fn the_rerun_wall_is_the_rows_own_experiment_wall() {
    // VOS-D131: a chain quick row was recorded under 600 s and B5 under 1,800 s, so an in-process
    // re-run must use the same clock; the old flat 120 s could flip a slow honest row.
    assert_eq!(rerun_wall("D4"), Duration::from_secs(600));
    assert_eq!(rerun_wall("B3"), Duration::from_secs(600));
    assert_eq!(rerun_wall("B5"), Duration::from_secs(1800));
    assert_eq!(rerun_wall("A1"), Duration::from_secs(120));
}
