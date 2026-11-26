//! VOS-028c2, the security review's SECOND round (VOS30 section 9.12), findings 1 to 8, each with
//! a negative control: a file, row or path that the fixed code refuses (or reports) and the code
//! before the fix accepted.  Tests that start child processes are unix-only.

// Off unix the child-process tests are compiled out, which leaves their helpers unused.
#![cfg_attr(not(unix), allow(unused))]

use std::path::PathBuf;
use vos_capacity_probe::cli::{dispatch, Failure};
use vos_capacity_probe::cost::{is_cheap, CHEAP, STRUCT_HEAVY};
use vos_capacity_probe::results::{
    bounds_violations, compare_rerun, compare_structure, decide, decision_differences,
    decision_groups_not_resolved, decision_read, rerun, rerun_collect, validate, Results,
    NONDETERMINISTIC_METRICS,
};
use vos_capacity_probe::row::{Knob, Kv, Row};
use vos_capacity_probe::spot::{spot_check, structure_comparable, SPOT_FLOOR};

const EXE: &str = env!("CARGO_BIN_EXE_vos-capacity-probe");

fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs")
        .join(name)
}

fn committed_text() -> String {
    std::fs::read_to_string(data("data/vos30-capacity-results.json")).unwrap()
}

fn committed() -> Results {
    Results::from_text(&committed_text()).unwrap()
}

fn section_9_12() -> String {
    let s = std::fs::read_to_string(data("VOS30-proof-route-and-capacity.md")).unwrap();
    let i = s.find("### 9.12 ").unwrap();
    let rest = &s[i..];
    let j = rest.find("\n## 10.").unwrap_or(rest.len());
    rest[..j].split_whitespace().collect::<Vec<_>>().join(" ")
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vos-probe-rev2-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn argv(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

fn small_file(rows: Vec<Row>) -> Results {
    let mut f = committed();
    f.meta.merges.clear();
    f.rows = rows;
    f
}

fn rows_of(exp: &str, pred: impl Fn(&Row) -> bool) -> Vec<Row> {
    committed()
        .rows
        .into_iter()
        .filter(|r| r.experiment == exp && pred(r))
        .collect()
}

fn c1_b200() -> Vec<Row> {
    rows_of("C1", |r| {
        r.knob.s("base") == "b"
            && r.knob.int("length") == 200
            && (r.knob.f("divergence") - 0.25).abs() < 1e-9
    })
}

fn d3_t8_k16() -> Vec<Row> {
    rows_of("D3", |r| r.knob.int("t") == 8 && r.knob.int("k") == 16)
}

// ---- 1. expensive rows: the structural rebuild, and the printed gap ----------------------------

#[test]
fn the_committed_file_has_decision_groups_that_a_plain_check_does_not_re_solve() {
    let f = committed();
    // Nothing re-run by the spot check: the quick groups only (round 3: the count follows the run).
    let cheap_rows: Vec<usize> = f
        .rows
        .iter()
        .enumerate()
        .filter(|(_, r)| decision_read(r) && is_cheap(&r.experiment, &r.mode, &r.knob.key()))
        .map(|(i, _)| i)
        .collect();
    // The spot check that re-ran every cheap decision row leaves the 12 groups (11 of round 2, plus D2 at T = 64, k = 16 once VOS-D143 re-derived the table from the CI recording).
    let unsolved = decision_groups_not_resolved(&f.rows, &cheap_rows);
    assert_eq!(unsolved.len(), 12, "{unsolved:?}");
    // The reviewer's two targets are among them.
    assert!(unsolved
        .iter()
        .any(|g| g.starts_with("D3|split|{\"t\":8,\"k\":16}")));
    assert!(unsolved
        .iter()
        .any(|g| g.starts_with("C1|monolithic|") && g.contains("\"length\":200")));
    // Every one is a group that `decide` reads and the plan does not class cheap.
    for g in &unsolved {
        let r = f
            .rows
            .iter()
            .find(|r| &vos_capacity_probe::results::group_key(r) == g)
            .unwrap();
        assert!(decision_read(r) && !is_cheap(&r.experiment, &r.mode, &r.knob.key()));
    }
}

/// Soundness of `decision_read`: on an all-green synthetic file, turning any one group red changes
/// the decision only for groups the predicate names.
#[test]
fn decision_read_covers_every_group_whose_redness_changes_the_decision() {
    let mut rows = committed().rows;
    for r in &mut rows {
        // An all-green synthetic recording: every group passes section 8.3.
        r.result = "unsat".into();
        r.control.result = "sat".into();
        r.control.model_checks_cnf = true;
        r.control.replayed = true;
        r.conflicts = 10;
        r.worst_step_conflicts = Some(10);
        r.solve_ms = 10.0;
        r.worst_step_solve_ms = 10.0;
        r.check_ms = Some(1.0);
        r.drat_bytes = Some(10);
        r.pass_default_limits = Some(true);
        r.peak_rss_kb = 1000;
        r.cap = "none".into();
        r.green = vos_capacity_probe::results::green(r);
    }
    let base = decide(&rows);
    assert_eq!(base.row, 3, "{base:?}");
    let mut groups: Vec<String> = rows
        .iter()
        .map(vos_capacity_probe::results::group_key)
        .collect();
    groups.sort();
    groups.dedup();
    let mut changed = 0;
    for g in &groups {
        let mut red = rows.clone();
        for r in red
            .iter_mut()
            .filter(|r| &vos_capacity_probe::results::group_key(r) == g)
        {
            r.result = "unknown".into();
            r.cap = "wall".into();
            r.green = false;
        }
        let d = decide(&red);
        if d.row != base.row || d.because != base.because {
            changed += 1;
            let sample = rows
                .iter()
                .find(|r| &vos_capacity_probe::results::group_key(r) == g)
                .unwrap();
            assert!(
                decision_read(sample),
                "{g} changes the decision but is not decision_read"
            );
        }
    }
    assert!(
        changed >= 8,
        "the probe did not exercise the decision: {changed}"
    );
}

#[test]
fn the_expensive_row_edit_the_reviewer_made_is_caught_when_it_touches_the_structure() {
    // C1 base b at 200 instructions and 25% divergence is an EXPENSIVE group: a plain check never
    // re-solves it.  The reviewer's edit (result unsat, fabricated control and digests) now meets
    // the structural rebuild, which compares the digest, size and entropy.
    let rows = c1_b200();
    assert_eq!(rows.len(), 5);
    assert!(!is_cheap("C1", "monolithic", &rows[0].knob.key()));
    let clean = small_file(rows.clone());
    #[cfg(unix)]
    {
        use std::time::Duration;
        use vos_capacity_probe::spot::structure_check;
        let rep = structure_check(
            &clean,
            std::path::Path::new(EXE),
            4,
            Duration::from_secs(120),
            Duration::from_secs(30),
            &[],
        );
        assert!(rep.violations.is_empty(), "{:?}", rep.violations);
        assert_eq!(rep.verified, 5, "{rep:?}");
        assert_eq!(rep.critical_verified, 5);
        for field in [
            "instance_sha256",
            "ands",
            "vars",
            "clauses",
            "entropy_bits",
            "declared_bits",
        ] {
            let mut forged = clean.clone();
            let r = &mut forged.rows[2];
            match field {
                "instance_sha256" => r.instance_sha256 = "ab".repeat(32),
                "ands" => r.ands += 1,
                "vars" => r.vars += 1,
                "clauses" => r.clauses += 1,
                "entropy_bits" => r.entropy_bits = r.entropy_bits.saturating_sub(1),
                _ => r.declared_bits += 1,
            }
            let rep = structure_check(
                &forged,
                std::path::Path::new(EXE),
                4,
                Duration::from_secs(120),
                Duration::from_secs(30),
                &[],
            );
            assert!(
                rep.violations
                    .iter()
                    .any(|v| v.contains(field) && v.contains("structural rebuild")),
                "{field}: {:?}",
                rep.violations
            );
            assert_eq!(rep.verified, 5);
        }
    }
}

#[test]
fn an_edit_that_touches_only_timing_fields_of_a_decision_group_is_not_structural_and_is_named() {
    // The reviewer's D3 t=8 k=16 edit (worst conflicts 90000, worst solve 9000 ms, DRAT 20 MB,
    // green true) changes no structural field.  A plain check cannot solve this group (minutes);
    // what it does is name the gap: the group is in `decision_groups_not_resolved` and only
    // `check --full` verifies it.  This test pins that this is the documented limit.
    let mut rows = d3_t8_k16();
    assert_eq!(rows.len(), 5);
    let honest = rows.clone();
    for r in &mut rows {
        r.worst_step_conflicts = Some(90_000);
        r.worst_step_solve_ms = 9_000.0;
        r.drat_bytes = Some(20 * 1024 * 1024);
        r.green = true;
        for (a, b) in honest.iter().filter(|h| h.seed == r.seed).map(|h| (h, &*r)) {
            assert!(compare_structure(a, b).is_empty());
        }
    }
    let f = small_file(rows);
    assert!(decision_groups_not_resolved(&f.rows, &[])
        .iter()
        .any(|g| g.starts_with("D3|split|{\"t\":8,\"k\":16}")));
    // And the spec says so in plain words.
    let spec = section_9_12();
    assert!(spec.contains("re-solves only cheap-class rows"));
    assert!(spec.contains("re-solves it is **0**"));
}

#[test]
fn structure_of_agrees_with_run_row_on_every_shape_of_the_quick_tier() {
    use std::time::Duration;
    use vos_capacity_probe::exp::{plan, run_row, structure_of, Tier};
    let mut n = 0;
    for spec in plan(Tier::Quick, &[], 5)
        .into_iter()
        .filter(|s| s.seed == 1)
    {
        if spec.experiment.starts_with('E') {
            assert!(structure_of(&spec).is_err());
            continue;
        }
        let row = run_row(&spec, Duration::from_secs(120), &mut |_| {});
        let s = structure_of(&spec).unwrap();
        let tag = format!("{} {}", spec.experiment, spec.knob.key());
        assert_eq!(s.instance_sha256, row.instance_sha256, "{tag}");
        assert_eq!(
            (s.ands, s.vars, s.clauses, s.entropy_bits, s.declared_bits),
            (
                row.ands,
                row.vars,
                row.clauses,
                row.entropy_bits,
                row.declared_bits
            ),
            "{tag}"
        );
        n += 1;
    }
    assert!(n >= 10, "{n}");
}

#[test]
fn a_partial_record_is_not_structurally_comparable_and_a_complete_one_is() {
    let mut r = committed()
        .rows
        .into_iter()
        .find(|r| r.experiment == "D3")
        .unwrap();
    for (res, want) in [
        ("unsat", true),
        ("collapsed", true),
        // Round 3, finding 4: a chain that stopped on a SAT segment is a partial record.
        ("sat", false),
        ("unknown", false),
        ("killed", false),
        ("oom", false),
        ("crashed", false),
    ] {
        r.result = res.into();
        assert_eq!(structure_comparable(&r), want, "chain {res}");
    }
    let mut a = committed()
        .rows
        .into_iter()
        .find(|r| r.experiment == "A1")
        .unwrap();
    a.result = "unknown".into();
    assert!(
        structure_comparable(&a),
        "a single instance built whole before the solve"
    );
    a.result = "killed".into();
    assert!(!structure_comparable(&a));
    let e = committed()
        .rows
        .into_iter()
        .find(|r| r.experiment == "E2")
        .unwrap();
    assert!(structure_comparable(&e));
}

#[test]
fn the_heavy_class_is_the_plans_not_the_files() {
    // Editing a heavy row's recorded fields cannot get it rebuilt or skipped differently.
    assert_eq!(STRUCT_HEAVY.len(), 5);
    let mut r = committed()
        .rows
        .into_iter()
        .find(|r| r.experiment == "D3" && r.knob.int("t") == 256)
        .unwrap();
    assert!(vos_capacity_probe::cost::structure_heavy(
        &r.experiment,
        &r.mode,
        &r.knob.key()
    ));
    r.solve_ms = 0.0;
    r.result = "unsat".into();
    assert!(vos_capacity_probe::cost::structure_heavy(
        &r.experiment,
        &r.mode,
        &r.knob.key()
    ));
}

// ---- 2. compare_rerun compares every deterministic field ----------------------------------------

fn rec() -> Row {
    let mut r = committed()
        .rows
        .into_iter()
        .find(|r| r.experiment == "A1")
        .unwrap();
    r.metrics = vec![("cases".into(), 256.0), ("run_ms".into(), 5.0)];
    r
}

#[test]
fn compare_rerun_flags_every_deterministic_field_and_ignores_only_the_nondeterministic_ones() {
    let rec = rec();
    assert!(compare_rerun(&rec, &rec.clone()).is_empty());
    let diff = |f: &dyn Fn(&mut Row)| {
        let mut now = rec.clone();
        f(&mut now);
        compare_rerun(&rec, &now).join(" | ")
    };
    assert!(diff(&|r| r.vars += 1).contains("vars"));
    assert!(diff(&|r| r.clauses += 1).contains("clauses"));
    assert!(diff(&|r| r.entropy_bits += 1).contains("entropy_bits"));
    assert!(diff(&|r| r.declared_bits += 1).contains("declared_bits"));
    assert!(diff(&|r| r.propagations += 1).contains("propagations"));
    assert!(diff(&|r| r.check_work = Some(7)).contains("check_work"));
    assert!(diff(&|r| r.drat_bytes = Some(7)).contains("drat_bytes"));
    assert!(diff(&|r| r.cap = "wall".into()).contains("cap"));
    assert!(diff(&|r| r.note = "edited".into()).contains("note"));
    assert!(diff(&|r| r.wall_s = Some(600)).contains("wall_s"));
    assert!(diff(&|r| r.seeds = 9).contains("seeds"));
    assert!(
        diff(&|r| r.tier = if r.tier == "quick" { "full" } else { "quick" }.into())
            .contains("tier")
    );
    assert!(diff(&|r| r.check_ms = None).contains("check_ms present"));
    // Metrics: a symmetric key set, and exact values for every metric that is not a timing.
    assert!(diff(&|r| r.metrics.push(("extra".into(), 1.0))).contains("metric keys"));
    assert!(diff(&|r| r.metrics.retain(|(k, _)| k != "cases")).contains("metric keys"));
    assert!(diff(&|r| r.metrics[0].1 = 255.0).contains("metric cases"));
    // The key set is symmetric: an EXTRA recorded key (the old code never looked at them) is a
    // finding when the re-run lacks it.
    let mut extra_recorded = rec.clone();
    extra_recorded.metrics.push(("planted".into(), 1.0));
    assert!(compare_rerun(&extra_recorded, &rec)
        .join(" ")
        .contains("metric keys"));
    // Not compared: times, the resident set, the values of the timing metrics.
    let mut slower = rec.clone();
    slower.solve_ms += 500.0;
    slower.worst_step_solve_ms += 500.0;
    slower.check_ms = Some(slower.check_ms.unwrap_or(0.0) + 500.0);
    slower.peak_rss_kb += 99_999;
    slower.metrics[1].1 = 5_000.0;
    assert!(
        compare_rerun(&rec, &slower).is_empty(),
        "{:?}",
        compare_rerun(&rec, &slower)
    );
    assert_eq!(
        NONDETERMINISTIC_METRICS,
        ["run_ms", "cases_per_s", "sweep_ms"]
    );
}

#[test]
fn the_deterministic_counters_really_are_deterministic_across_two_runs_of_one_row() {
    use std::time::Duration;
    use vos_capacity_probe::exp::{plan, run_row, Tier};
    for spec in plan(
        Tier::Quick,
        &["A1".into(), "B3".into(), "D4".into(), "E2".into()],
        5,
    )
    .into_iter()
    .filter(|s| s.seed == 2)
    {
        let a = run_row(&spec, Duration::from_secs(120), &mut |_| {});
        let b = run_row(&spec, Duration::from_secs(120), &mut |_| {});
        let d = compare_rerun(&a, &b);
        assert!(
            d.is_empty(),
            "{} {}: {d:?}",
            spec.experiment,
            spec.knob.key()
        );
    }
}

#[test]
fn check_full_compares_timing_and_recomputes_green_on_the_in_process_quick_rows() {
    // Five quick A1 n = 8 rows.  Flip a recorded `green`: the plain re-run does not look (it
    // compares only what is deterministic), `check --full`'s in-process pass does.
    let rows: Vec<Row> = rows_of("A1", |r| r.tier == "quick");
    assert_eq!(rows.len(), 5);
    let mut f = small_file(rows);
    assert!(rerun(&f, 2).is_empty());
    let (d, map) = rerun_collect(&f, 2, true);
    assert!(d.is_empty(), "{d:?}");
    assert_eq!(map.len(), 5);
    f.rows[0].green = !f.rows[0].green;
    assert!(rerun(&f, 2).is_empty(), "the plain re-run ignores green");
    let (d, _) = rerun_collect(&f, 2, true);
    assert!(d.iter().any(|m| m.contains("green is")), "{d:?}");
    // And a recorded time far outside the band is caught in process too.
    let mut g = small_file(rows_of("A1", |r| r.tier == "quick"));
    g.rows[1].solve_ms = 1.0e6;
    let (d, _) = rerun_collect(&g, 2, true);
    assert!(d.iter().any(|m| m.contains("solve_ms is outside")), "{d:?}");
}

// ---- 3. --full: the decision the re-run gives is the recorded one --------------------------------

#[test]
fn the_decision_on_the_rerun_rows_must_equal_the_recorded_one() {
    let rows = committed().rows;
    assert!(decision_differences(&rows, &rows).is_empty());
    // A re-run in which the four required groups come out green: row 1 becomes something else.
    let mut greener = rows.clone();
    for r in greener.iter_mut().filter(|r| decision_read(r)) {
        r.result = "unsat".into();
        r.control.result = "sat".into();
        r.control.model_checks_cnf = true;
        r.control.replayed = true;
        r.worst_step_conflicts = Some(5);
        r.conflicts = 5;
        r.worst_step_solve_ms = 5.0;
        r.drat_bytes = Some(5);
        r.pass_default_limits = Some(true);
        r.peak_rss_kb = 1000;
        r.cap = "none".into();
        r.green = vos_capacity_probe::results::green(r);
    }
    let d = decision_differences(&rows, &greener);
    assert!(
        d.iter()
            .any(|m| m.contains("is row 1 on the recorded rows but row")),
        "{d:?}"
    );
    // The `because` text is compared too, with the worst-solve time (nondeterministic) cut out.
    let mut slower = rows.clone();
    for r in slower.iter_mut() {
        r.worst_step_solve_ms *= 1.5;
    }
    assert!(decision_differences(&rows, &slower).is_empty());
    let mut more_conflicts = rows.clone();
    for r in more_conflicts.iter_mut().filter(|r| r.experiment == "D3") {
        r.worst_step_conflicts = r.worst_step_conflicts.map(|c| c + 1_000_000);
    }
    let d = decision_differences(&rows, &more_conflicts);
    assert!(d.iter().any(|m| m.contains("reasons differ")), "{d:?}");
}

// ---- 4. canonical input, and no silent narrowing ------------------------------------------------

#[test]
fn the_committed_file_is_canonical() {
    let text = committed_text();
    assert!(Results::from_canonical_text(&text).is_ok());
}

#[test]
fn a_non_canonical_file_is_refused_by_check_merge_table_and_decide() {
    let text = committed_text();
    // Two spellings of a duplicate key: the parser keeps the first, so a reader sees one value
    // and a human reading to the end sees another.
    let dup = text.replacen("\"schema\": 2,", "\"schema\": 2,\n  \"schema\": 1,", 1);
    assert!(
        Results::from_text(&dup).is_ok(),
        "the lenient reader accepts it (the old behaviour)"
    );
    assert!(Results::from_canonical_text(&dup)
        .unwrap_err()
        .contains("canonical"));
    // An unknown key, a re-spaced file and a trailing blank line are all non-canonical.
    let unknown = text.replacen("\"schema\": 2,", "\"schema\": 2,\n  \"extra\": 1,", 1);
    let spaced = text.replacen("\"schema\": 2,", "\"schema\":   2,", 1);
    let blank = format!("{text}\n");
    let d = scratch("canon");
    let good = d.join("good.json");
    std::fs::write(&good, &text).unwrap();
    for (name, bad) in [
        ("dup", dup),
        ("unknown", unknown),
        ("spaced", spaced),
        ("blank", blank),
    ] {
        let p = d.join(format!("{name}.json"));
        std::fs::write(&p, &bad).unwrap();
        let ps = p.to_str().unwrap();
        let mut o = Vec::new();
        let mut e = Vec::new();
        for cmd in [
            vec!["check", ps],
            vec!["decide", "--results", ps],
            vec!["table", "--check", "--results", ps],
        ] {
            let r = dispatch(&argv(&cmd), &mut o, &mut e);
            assert!(
                matches!(&r, Err(Failure::Usage(m)) if m.contains("canonical")),
                "{name} {cmd:?}: {r:?}"
            );
        }
        // merge refuses a non-canonical file on EITHER input.
        let out = d.join("out.json");
        for (base, fresh) in [(ps, good.to_str().unwrap()), (good.to_str().unwrap(), ps)] {
            let r = dispatch(
                &argv(&[
                    "merge",
                    "--base",
                    base,
                    "--fresh",
                    fresh,
                    "--experiment",
                    "E2",
                    "--out",
                    out.to_str().unwrap(),
                ]),
                &mut o,
                &mut e,
            );
            assert!(
                matches!(&r, Err(Failure::Usage(m)) if m.contains("canonical")),
                "{name}: {r:?}"
            );
            assert!(!out.exists());
        }
    }
}

#[test]
fn a_seed_count_that_does_not_fit_in_32_bits_is_refused_not_truncated() {
    // 2^32 + 5 used to be read as 5.
    let text = committed_text();
    let bad = text.replacen(
        "\"seeds\":5,\"instance_sha256\"",
        "\"seeds\":4294967301,\"instance_sha256\"",
        1,
    );
    assert_ne!(bad, text);
    let e = Results::from_text(&bad).unwrap_err();
    assert!(e.contains("32 bits"), "{e}");
    let mut spec = vos_capacity_probe::runner::spec_to_json(&vos_capacity_probe::exp::RowSpec {
        experiment: "A1".into(),
        knob: Knob::of(&[("n", Kv::I(8))]),
        mode: "monolithic".into(),
        seed: 1,
        seeds: 5,
        tier: "full".into(),
    });
    spec = spec.replace("\"seeds\":5", "\"seeds\":4294967301");
    assert!(vos_capacity_probe::runner::spec_from_json(&spec)
        .unwrap_err()
        .contains("32 bits"));
}

// ---- 5. numeric bounds ---------------------------------------------------------------------------

#[test]
fn numeric_fields_are_bounded() {
    let base = committed();
    assert!(validate(&base).is_empty());
    let check = |label: &str, f: &dyn Fn(&mut Row), want: &str| {
        let mut x = base.clone();
        f(&mut x.rows[10]);
        let v = validate(&x).join(" | ");
        assert!(v.contains(want), "{label}: {v}");
    };
    check(
        "negative solve",
        &|r| r.solve_ms = -1.0,
        "not a finite time",
    );
    check("huge solve", &|r| r.solve_ms = 1e300, "not a finite time");
    check(
        "negative worst",
        &|r| r.worst_step_solve_ms = -0.5,
        "not a finite time",
    );
    check(
        "negative check",
        &|r| r.check_ms = Some(-3.0),
        "not a finite time",
    );
    check(
        "nan check",
        &|r| r.check_ms = Some(f64::NAN),
        "not a finite time",
    );
    check(
        "inf solve",
        &|r| r.solve_ms = f64::INFINITY,
        "not a finite time",
    );
    check(
        "conflicts",
        &|r| r.conflicts = i64::MAX as u64,
        "over the bound",
    );
    check(
        "propagations",
        &|r| r.propagations = i64::MAX as u64,
        "over the bound",
    );
    check("ands", &|r| r.ands = i64::MAX as u64, "over the bound");
    check("vars", &|r| r.vars = 1 << 40, "over the bound");
    check("clauses", &|r| r.clauses = 1 << 40, "over the bound");
    check("drat", &|r| r.drat_bytes = Some(1 << 41), "over the bound");
    check("work", &|r| r.check_work = Some(1 << 41), "over the bound");
    check("steps", &|r| r.steps = 1 << 20, "over the bound");
    check("rss", &|r| r.peak_rss_kb = 1 << 40, "over the bound");
    check("seeds", &|r| r.seeds = 1_000_000, "over the bound");
    check(
        "entropy",
        &|r| r.entropy_bits = r.declared_bits + 1,
        "exceeds declared_bits",
    );
    check(
        "worst > total",
        &|r| {
            r.worst_step_conflicts = Some(r.conflicts + 1);
        },
        "exceeds the total",
    );
    check("seed 0", &|r| r.seed = 0, "is not in 1..=");
    check(
        "metric",
        &|r| r.metrics = vec![("cases".into(), f64::INFINITY)],
        "metric cases",
    );
    // The committed file satisfies every bound with room (the largest recorded time is 10 minutes).
    for r in &base.rows {
        assert!(bounds_violations("x", r).is_empty());
    }
}

// ---- 6. the cheap table cannot shrink quietly -----------------------------------------------------

#[test]
fn the_cheap_table_and_its_floor_are_pinned() {
    assert!(CHEAP.len() >= 122, "{}", CHEAP.len());
    let f = committed();
    let eligible = f
        .rows
        .iter()
        .filter(|r| {
            r.tier == "full"
                && !r.experiment.starts_with('E')
                && is_cheap(&r.experiment, &r.mode, &r.knob.key())
        })
        .count();
    assert!(eligible >= 610, "{eligible}");
    assert_eq!(SPOT_FLOOR, 16);
    let spec = section_9_12();
    assert!(spec.contains("`CHEAP` has at least 122 groups and at least 610 eligible rows"));
    assert!(spec.contains("`SPOT_FLOOR` = 16"));
}

#[cfg(unix)]
#[test]
fn an_eligible_count_below_the_floor_is_a_failure_of_its_own() {
    // Fifteen cheap rows: above the old `min(16, eligible)` floor (which was 15), below 16.
    let mut rows = Vec::new();
    for n in [16, 32, 64] {
        rows.extend(rows_of("A1", |r| r.knob.int("n") == n));
    }
    assert_eq!(rows.len(), 15);
    for r in &mut rows {
        r.tier = "full".into();
    }
    let f = small_file(rows);
    let rep = spot_check(
        &f,
        std::path::Path::new(EXE),
        4,
        std::time::Duration::from_secs(120),
    );
    assert_eq!(rep.rerun, 15, "{rep:?}");
    assert!(
        rep.violations
            .iter()
            .any(|v| v.contains("only 15 rows are eligible") && v.contains("floor of 16")),
        "{:?}",
        rep.violations
    );
}

// ---- 7. a reader thread cannot hold the parent past the deadline ---------------------------------

#[cfg(unix)]
#[test]
fn an_orphan_holding_the_pipes_open_cannot_hold_the_parent_past_the_reader_deadline() {
    use std::os::unix::fs::PermissionsExt;
    use std::time::{Duration, Instant};
    use vos_capacity_probe::exp::RowSpec;
    use vos_capacity_probe::runner::{run_child_sub, End};
    let d = scratch("orphan");
    let exe = d.join("orphan.sh");
    // The script leaves a background process that inherits stdout and stderr and lives 4 s, then
    // exits at once: the pipes stay open after the child is gone, as after a failed group kill.
    std::fs::write(&exe, "#!/bin/sh\nsleep 4 &\nexit 0\n").unwrap();
    std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
    let spec = RowSpec {
        experiment: "A1".into(),
        knob: Knob::of(&[("n", Kv::I(8))]),
        mode: "monolithic".into(),
        seed: 1,
        seeds: 5,
        tier: "full".into(),
    };
    let t0 = Instant::now();
    let ch = run_child_sub(
        &exe,
        "row",
        &spec,
        120,
        4 * 1024 * 1024,
        Duration::from_secs(60),
        Duration::from_millis(300),
    )
    .unwrap();
    assert!(t0.elapsed() < Duration::from_secs(3), "{:?}", t0.elapsed());
    assert_eq!(ch.end, End::Killed, "unverified, never a clean exit");
    // Control: without an orphan the same call ends normally and keeps its output.
    std::fs::write(&exe, "#!/bin/sh\necho hello\nexit 0\n").unwrap();
    let ch = run_child_sub(
        &exe,
        "row",
        &spec,
        120,
        4 * 1024 * 1024,
        Duration::from_secs(60),
        Duration::from_secs(5),
    )
    .unwrap();
    assert!(matches!(ch.end, End::Exited(_)));
    assert!(ch.stdout.contains("hello"));
}

// ---- 8. the atomic writer -------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn write_atomic_drops_setuid_setgid_sticky_and_survives_a_long_target_name() {
    use std::os::unix::fs::PermissionsExt;
    use vos_capacity_probe::cli::{tmp_name_stem, write_atomic, TMP_STEM_MAX};
    let d = scratch("atomic");
    let p = d.join("target.json");
    std::fs::write(&p, "old").unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o4755)).unwrap();
    write_atomic(&p, "new").unwrap();
    assert_eq!(std::fs::read_to_string(&p).unwrap(), "new");
    assert_eq!(
        std::fs::metadata(&p).unwrap().permissions().mode() & 0o7777,
        0o755
    );
    // A target name of 240 bytes: the temporary `.NAME.PID.N.NANOS.tmp` would be over 255 bytes
    // and could not be created before the stem was cut.
    let long = format!("{}.json", "a".repeat(235));
    let lp = d.join(&long);
    write_atomic(&lp, "x").unwrap();
    assert_eq!(std::fs::read_to_string(&lp).unwrap(), "x");
    assert!(std::fs::read_dir(&d).unwrap().all(|e| !e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .ends_with(".tmp")));
    // The stem is cut on a character boundary.
    let multi = "é".repeat(200);
    let s = tmp_name_stem(&multi);
    assert!(s.len() <= TMP_STEM_MAX && multi.starts_with(&s));
    assert_eq!(tmp_name_stem("short"), "short");
}
