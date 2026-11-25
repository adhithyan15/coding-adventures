//! VOS-028c2: the chain budget (VOS-D095), the hardening of the probe (VOS-D097) and `merge
//! --add`, each clause with a negative control.  The tests that start child processes are
//! unix-only, as the runner is (VOS30 section 9.12).

mod common;

use std::path::PathBuf;
use std::time::{Duration, Instant};
use vos_capacity_probe::cli::{add_results, dispatch, full_refusals, merge_results, Failure};
use vos_capacity_probe::exp::{
    obligation_deadline, row_wall_s, run_row, wall_field, RowSpec, CHAIN_WALL_S, OBLIGATION_WALL_S,
};
use vos_capacity_probe::meta::stale_lines;
use vos_capacity_probe::results::{validate, Merge, Results, TIMEOUT_S, TREE_NAMES};
use vos_capacity_probe::row::{Control, Knob, Kv, Row};
use vos_capacity_probe::runner::{clamp_jobs, hard_limit, hard_limit_for, KILL_SLACK};
use vos_capacity_probe::spot::{population, sample_order, sample_seed};

const EXE: &str = env!("CARGO_BIN_EXE_vos-capacity-probe");

fn committed_text() -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/vos30-capacity-results.json");
    std::fs::read_to_string(p).unwrap()
}

/// The committed file completed with fixture rows for the plan rows it lacks (VOS-028c3c commit A
/// registered B3 every2/every4, D5 and B5 before the recording): "a complete file".
fn committed() -> Results {
    common::complete_plan(Results::from_text(&committed_text()).unwrap())
}

fn ok_control() -> Control {
    Control {
        result: "sat".into(),
        model_checks_cnf: true,
        replayed: true,
    }
}

/// A green monolithic row of experiment `exp`.
fn good(exp: &str, knob: Knob, mode: &str, seed: u64) -> Row {
    let mut r = Row::new(exp, knob.clone(), mode, seed, 5);
    // The tier is the plan's for the knob (finding H1).
    let quick = vos_capacity_probe::results::planned_tiers()
        .get(&(exp.to_string(), mode.to_string(), knob.key()))
        .copied()
        .unwrap_or(false);
    r.tier = if quick { "quick" } else { "full" }.into();
    r.result = "unsat".into();
    r.instance_sha256 = "aa".repeat(32);
    r.drat_sha256 = Some("bb".repeat(32));
    r.control_instance_sha256 = "cc".repeat(32);
    r.control = ok_control();
    r.conflicts = 1000;
    r.worst_step_solve_ms = 5.0;
    r.solve_ms = 5.0;
    r.drat_bytes = Some(1000);
    r.check_work = Some(1_000_000);
    r.pass_default_limits = Some(true);
    r.peak_rss_kb = 10_000;
    r.green = true;
    r
}

fn b4(shape: &str) -> Knob {
    match shape {
        "assoc" => Knob::of(&[
            ("shape", Kv::S(shape.into())),
            ("n", Kv::I(3)),
            ("w", Kv::I(32)),
        ]),
        _ => Knob::of(&[("shape", Kv::S(shape.into())), ("w", Kv::I(8))]),
    }
}

fn group(shape: &str, digest: &str) -> Vec<Row> {
    (1..=5)
        .map(|s| {
            let mut r = good("B4", b4(shape), "monolithic", s);
            r.instance_sha256 = digest.repeat(32);
            r
        })
        .collect()
}

fn file(groups: Vec<Vec<Row>>, rev: &str) -> Results {
    let mut m = committed().meta;
    m.merges.clear();
    m.trees.clear();
    m.binary_sha256 = format!("{rev:0<64}").chars().take(64).collect();
    m.revisions = vec![("repo".into(), rev.into())];
    Results {
        meta: m,
        rows: groups.into_iter().flatten().collect(),
    }
}

const REV_OLD: &str = "1111111111111111111111111111111111111111";
const REV_NEW: &str = "2222222222222222222222222222222222222222";
const AT: &str = "2026-11-19T00:00:00Z";
fn yes(_: &str) -> Result<bool, String> {
    Ok(true)
}

fn bogus_trees(c: char) -> Vec<(String, String)> {
    TREE_NAMES
        .iter()
        .map(|n| (n.to_string(), c.to_string().repeat(40)))
        .collect()
}

// ---- VOS-D095: the chain budget ----------------------------------------------------------------

#[test]
fn a_chain_has_a_600_s_row_budget_and_every_other_row_keeps_120_s() {
    assert_eq!(OBLIGATION_WALL_S, TIMEOUT_S);
    for e in ["B3", "D3", "D4", "D5"] {
        assert_eq!(row_wall_s(e), 600, "{e}");
        assert_eq!(wall_field(e), Some(600), "{e}");
    }
    for e in [
        "A1", "A2", "B1", "B2", "B4", "C1", "C2", "D1", "D2", "E1", "E2",
    ] {
        assert_eq!(row_wall_s(e), 120, "{e}");
        assert_eq!(
            wall_field(e),
            None,
            "{e} omits the field so its bytes do not change"
        );
    }
    // The hard kill moves with the budget: 600 s + 30 s of control + 150 s of slack.
    assert_eq!(row_wall_s("B3"), CHAIN_WALL_S);
    assert_eq!(hard_limit_for("B3"), Duration::from_secs(780));
    assert_eq!(hard_limit(), hard_limit_for("A1"));
    assert!(KILL_SLACK >= Duration::from_secs(150));
}

#[test]
fn an_obligation_never_outlives_its_own_wall_nor_the_row_budget() {
    let now = Instant::now();
    let row = now + Duration::from_secs(600);
    let ob = Duration::from_secs(120);
    assert_eq!(obligation_deadline(row, now, ob), now + ob);
    // Late in the row the row's deadline wins.
    let late = now + Duration::from_secs(550);
    assert_eq!(obligation_deadline(row, late, ob), row);
}

#[test]
fn the_chain_timeout_path_ends_the_row_unknown_on_the_wall_cap_with_its_budget_recorded() {
    // A tiny budget instead of a real 600 s wait: the row's deadline is already past, so the
    // chain stops before its first obligation, exactly the path a spent 600 s budget takes.
    for mode in ["certified", "incremental"] {
        let spec = RowSpec {
            experiment: "B3".into(),
            knob: Knob::of(&[
                ("length", Kv::I(25)),
                ("divergence", Kv::F(0.05)),
                ("cut", Kv::S("every1".into())),
            ]),
            mode: mode.into(),
            seed: 1,
            seeds: 5,
            tier: "quick".into(),
        };
        let row = run_row(&spec, Duration::ZERO, &mut |_| {});
        assert_eq!(
            (row.result.as_str(), row.cap.as_str()),
            ("unknown", "wall"),
            "{mode}"
        );
        assert!(
            row.note.contains("wall clock ended the row"),
            "{}",
            row.note
        );
        assert_eq!(row.wall_s, Some(600), "the budget is recorded");
        assert!(!row.green);
        assert!(row.to_line().contains("\"wall_s\":600"));
    }
}

#[test]
fn a_non_chain_row_records_no_budget_and_validate_holds_wall_s_to_the_chain_budget() {
    let a1 = RowSpec {
        experiment: "A1".into(),
        knob: Knob::of(&[("n", Kv::I(8))]),
        mode: "monolithic".into(),
        seed: 1,
        seeds: 5,
        tier: "quick".into(),
    };
    let row = run_row(&a1, Duration::from_secs(30), &mut |_| {});
    assert_eq!(row.wall_s, None);
    assert!(!row.to_line().contains("wall_s"));
    // validate: a 120 s row names none, a chain row names 600 or (recorded before) nothing.
    let mut f = committed();
    assert!(validate(&f).is_empty());
    let a = f.rows.iter().position(|r| r.experiment == "A1").unwrap();
    f.rows[a].wall_s = Some(600);
    assert!(validate(&f).join("|").contains("wall_s 600"));
    f.rows[a].wall_s = None;
    let b = f.rows.iter().position(|r| r.experiment == "B3").unwrap();
    f.rows[b].wall_s = Some(600);
    assert!(
        validate(&f).is_empty(),
        "the chain budget is the one valid value"
    );
    f.rows[b].wall_s = Some(900);
    assert!(validate(&f).join("|").contains("wall_s 900"));
}

// ---- VOS-D097 (4): knob values are a closed set ------------------------------------------------

#[test]
fn an_unplanned_knob_value_or_mode_is_refused_by_validate() {
    let mut f = committed();
    assert!(validate(&f).is_empty(), "every recorded row is in the plan");
    let i = f.rows.iter().position(|r| r.experiment == "B2").unwrap();
    let orig = f.rows[i].clone();
    // A hand-edited length of 10^9 (a float divergence off the grid, a new field, a new mode).
    f.rows[i].knob = Knob::of(&[
        ("length", Kv::I(1_000_000_000)),
        ("divergence", Kv::F(0.25)),
    ]);
    assert!(validate(&f)
        .join("|")
        .contains("not one of the plan's knobs"));
    f.rows[i] = orig.clone();
    f.rows[i].knob = Knob::of(&[("length", Kv::I(25)), ("divergence", Kv::F(0.2500001))]);
    assert!(!validate(&f).is_empty());
    f.rows[i] = orig.clone();
    f.rows[i].mode = "certified".into();
    assert!(!validate(&f).is_empty());
    f.rows[i] = orig;
    assert!(validate(&f).is_empty());
}

// ---- VOS-D097 (1): trees, stale and the sample --------------------------------------------------

#[test]
fn the_committed_file_is_canonical_so_its_bytes_seed_the_sample() {
    let text = committed_text();
    let res = Results::from_text(&text).unwrap();
    assert_eq!(
        res.to_text(),
        text,
        "reading and writing the file changes no byte"
    );
    assert_eq!(
        sample_seed(&res),
        coding_adventures_sha256::sha256_hex(text.as_bytes()),
        "the seed is the SHA-256 of the file's own bytes"
    );
}

#[test]
fn the_sample_is_every_full_non_e_row_in_a_seeded_order_and_any_edit_reshuffles_it() {
    let res = committed();
    let pop = population(&res);
    assert!(pop
        .iter()
        .all(|&i| res.rows[i].tier == "full" && !res.rows[i].experiment.starts_with('E')));
    assert_eq!(
        pop.len(),
        res.rows
            .iter()
            .filter(|r| r.tier == "full" && !r.experiment.starts_with('E'))
            .count(),
        "no slow row is excluded from the population"
    );
    let order = sample_order(&res);
    assert_eq!(order, sample_order(&res), "deterministic for one file");
    let mut sorted = order.clone();
    sorted.sort();
    assert_eq!(sorted, pop, "a permutation of the population");
    assert_ne!(order, pop, "not file order");
    // A hand edit of a single row's measurement changes the file's bytes and so the order.
    let mut edited = res.clone();
    let i = pop[0];
    edited.rows[i].conflicts += 1;
    let other = sample_order(&edited);
    assert_ne!(other, order, "an edit reshuffles the sample");
    let head = |o: &[usize]| o.iter().take(30).copied().collect::<Vec<_>>();
    assert_ne!(
        head(&other),
        head(&order),
        "including its first rows, which are what run"
    );
}

#[test]
fn stale_lines_name_each_changed_tree_and_say_nothing_for_a_file_without_trees() {
    let rec = bogus_trees('a');
    assert!(stale_lines("tool", &rec, &rec).is_empty());
    assert!(
        stale_lines("tool", &[], &rec).is_empty(),
        "nothing is claimed"
    );
    let mut cur = rec.clone();
    cur[1].1 = "b".repeat(40);
    let v = stale_lines("tool", &rec, &cur);
    assert_eq!(v.len(), 1);
    assert!(
        v[0].starts_with(&format!("stale: tool: {} was recorded at tree", rec[1].0)),
        "{}",
        v[0]
    );
    assert_eq!(stale_lines("tool", &rec, &[]).len(), 1, "git cannot say");
}

#[test]
fn trees_validate_and_round_trip_and_an_absent_object_keeps_the_bytes() {
    let mut f = committed();
    // The committed merges carry trees of their own; this test is about the file's.
    f.meta.merges.clear();
    f.meta.trees = bogus_trees('a');
    assert!(validate(&f).is_empty());
    let text = f.to_text();
    assert!(text.contains("\"trees\":{\"vos-capacity-probe/src\":"));
    assert_eq!(Results::from_text(&text).unwrap(), f);
    f.meta.trees[0].1 = "xyz".into();
    assert!(validate(&f).join("|").contains("not a hex object id"));
    f.meta.trees = bogus_trees('a');
    f.meta.trees.pop();
    assert!(validate(&f).join("|").contains("must name exactly"));
    f.meta.trees.clear();
    assert!(!f.to_text().contains("\"trees\""));
    // `f` has no trees now, so its tool block ends at `dirty_tree`: make `trees` a number.
    let bad = f
        .to_text()
        .replace("\"dirty_tree\":false}", "\"dirty_tree\":false,\"trees\":3}");
    assert!(bad.contains("\"trees\":3"), "the edit must apply");
    assert!(Results::from_text(&bad).is_err());
}

#[test]
fn check_full_refuses_a_file_that_does_not_describe_the_tree() {
    let cur = bogus_trees('a');
    let mut f = committed();
    // A file with no recorded trees cannot say which tree it describes (the committed recording
    // has them, so strip them).
    f.meta.trees.clear();
    assert!(full_refusals(&f, &cur)
        .join("|")
        .contains("records no tree ids"));
    // Recorded and equal: nothing to refuse.
    f.meta.trees = cur.clone();
    f.meta.merges.clear();
    assert!(full_refusals(&f, &cur).is_empty());
    // A differing recorded tree: refused until the rows are regenerated.
    f.meta.trees = bogus_trees('b');
    let r = full_refusals(&f, &cur);
    assert_eq!(r.len(), TREE_NAMES.len(), "{r:?}");
    assert!(r[0].contains("check --full refuses") && r[0].contains("regenerate"));
    // A merges entry at another tree, or without any, refuses too.  The committed recording has
    // no merges, so give it one that predates trees (a refresh whose record has no trees).
    let mut f = committed();
    let mut fresh = f.clone();
    fresh.meta.trees = bogus_trees('d');
    f = merge_results(&f, &fresh, "B3", None, true, AT, &yes).unwrap();
    f.meta.merges[0].trees.clear();
    f.meta.trees = cur.clone();
    f.meta.merges.truncate(1);
    assert!(full_refusals(&f, &cur)
        .join("|")
        .contains("merges[0] records no tree ids"));
    f.meta.merges[0].trees = bogus_trees('c');
    assert_eq!(full_refusals(&f, &cur).len(), TREE_NAMES.len());
    // And through the command: exit status 1, nothing re-run (this returns at once).
    let dir = scratch("full-refuse");
    let p = dir.join("results.json");
    // The committed recording describes this very tree, so `--full` would not refuse it (and
    // would start the full re-run); record trees that differ from the tree's own.
    let mut stale = Results::from_text(&committed_text()).unwrap();
    stale.meta.trees = bogus_trees('a');
    std::fs::write(&p, stale.to_text()).unwrap();
    let (mut o, mut e) = (Vec::new(), Vec::new());
    let args: Vec<String> = ["check", p.to_str().unwrap(), "--full", "--exe", EXE]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let r = dispatch(&args, &mut o, &mut e);
    assert!(matches!(r, Err(Failure::Violations(n)) if n >= 1), "{r:?}");
    assert!(String::from_utf8(e)
        .unwrap()
        .contains("check --full refuses"));
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vos-probe-hard-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The committed file's quick rows only, with the given recorded trees.  It is NOT a complete
/// file: `check` refuses it for the missing full-plan rows (finding H2) before re-running anything.
fn quick_only(trees: Vec<(String, String)>) -> Results {
    let mut f = committed();
    f.rows.retain(|r| r.tier == "quick");
    f.meta.merges.clear();
    f.meta.trees = trees;
    f
}

fn check_args(p: &std::path::Path, extra: &[&str]) -> Vec<String> {
    let mut a: Vec<String> = ["check", p.to_str().unwrap(), "--exe", EXE]
        .iter()
        .map(|s| s.to_string())
        .collect();
    a.extend(extra.iter().map(|s| s.to_string()));
    a
}

#[test]
fn check_prints_a_non_fatal_stale_line_when_a_recorded_tree_differs() {
    let dir = scratch("stale");
    let p = dir.join("results.json");
    // A complete file (the committed one) recorded at another tree: stale, and still ok.  The
    // spot check has a short budget here; its floor of 16 verified rows still applies.
    let mut f = committed();
    f.meta.merges.clear();
    f.meta.trees = bogus_trees('a');
    std::fs::write(&p, f.to_text()).unwrap();
    let (mut o, mut e) = (Vec::new(), Vec::new());
    let r = dispatch(
        &check_args(
            &p,
            &["--jobs", "4", "--spot-budget", "15", "--struct-budget", "5"],
        ),
        &mut o,
        &mut e,
    );
    let (o, e) = (String::from_utf8(o).unwrap(), String::from_utf8(e).unwrap());
    // Stale is not fatal: no violation mentions it.  (Under a very slow build, such as the
    // opt-level 0 coverage build, the time-budgeted re-runs can end "not verified"; that is a
    // budget artifact and the only kind of violation tolerated here.)
    // Some rows of this file are fixtures (`common::complete_plan_ids`): the plan rows the
    // committed file lacks until commit B of VOS-028c3c records them, each a clone of a sibling.
    // A clone cannot reproduce, so `check` reports it (quick re-run, spot sample or structural
    // rebuild alike).  Findings about EXACTLY those rows are dropped; findings about every other
    // row are asserted as before.  Once commit B lands the set is empty and this is a no-op.
    let raw = Results::from_text(&committed_text()).unwrap();
    let (_, fabricated) = common::complete_plan_ids(raw.clone());
    assert_eq!(
        fabricated.len(),
        common::plan_rows_missing(&raw),
        "the filter covers exactly the plan rows the committed file lacks, no more"
    );
    let real: Vec<&str> = e
        .lines()
        .filter(|l| !common::is_about(l, &fabricated))
        .collect();
    assert!(
        real.iter().all(|l| {
            !l.contains("stale") && (l.contains("not verified") || l.contains("below the floor"))
                || l.is_empty()
        }),
        "stale is not fatal: {e}"
    );
    if e.is_empty() {
        assert!(r.is_ok());
        assert!(o.contains("check: ok"));
    } else if real.is_empty() {
        // Only fixture findings: the check still fails on them, and says so.
        assert!(matches!(r, Err(Failure::Violations(_))), "{r:?}");
    }
    assert!(o.contains("stale: tool: "), "{o}");
    #[cfg(unix)]
    {
        // 1,420 on the committed file; the fixture rows of `common::complete_plan` add the new
        // full-tier rows, so the population is computed from the file that was checked.
        let want = format!("spot check: population {}", population(&f).len());
        assert!(o.contains(&want), "{o}");
        assert!(o.contains("structural rebuild: population"), "{o}");
    }
    #[cfg(not(unix))]
    {
        assert!(
            o.contains("spot check: skipped (the process limits are unix-only)"),
            "{o}"
        );
        assert!(
            o.contains("structural rebuild: skipped (the process limits are unix-only)"),
            "{o}"
        );
    }
    assert!(
        o.contains("decision-critical groups not re-solved by this run: "),
        "{o}"
    );
    // The same recorded trees on a file with only its quick rows: the stale line still prints,
    // and the file is refused for what it lacks, not for being stale.
    std::fs::write(&p, quick_only(bogus_trees('a')).to_text()).unwrap();
    let (mut o, mut e) = (Vec::new(), Vec::new());
    let r = dispatch(&check_args(&p, &["--jobs", "4"]), &mut o, &mut e);
    let (o, e) = (String::from_utf8(o).unwrap(), String::from_utf8(e).unwrap());
    assert!(matches!(r, Err(Failure::Violations(_))), "{r:?}");
    assert!(o.contains("stale: tool: "), "{o}");
    assert!(e.contains("is not in the results file"), "{e}");
    assert!(
        !e.contains("stale"),
        "stale itself is never a violation: {e}"
    );
}

// ---- merge --add ---------------------------------------------------------------------------------

#[test]
fn merge_add_appends_new_rows_and_records_kind_add() {
    let base = file(vec![group("assoc", "aa"), group("mul5", "bb")], REV_OLD);
    let mut fresh = file(vec![group("mul12", "cc")], REV_NEW);
    fresh.meta.trees = bogus_trees('d');
    let m = add_results(&base, &fresh, "B4", None, false, AT, &yes).unwrap();
    assert_eq!(m.rows.len(), 15);
    assert_eq!(
        m.rows[..10],
        base.rows[..],
        "the base rows are untouched, in order"
    );
    assert_eq!(m.rows[10..], fresh.rows[..], "the new rows are appended");
    let g = &m.meta.merges[0];
    assert_eq!(g.kind.as_deref(), Some("add"));
    assert_eq!(g.rows.len(), 5);
    assert_eq!(g.trees, fresh.meta.trees);
    assert_eq!(
        m.meta.trees, base.meta.trees,
        "the file's own provenance stays the base's"
    );
    assert!(validate(&m).is_empty(), "{:?}", validate(&m));
    let text = m.to_text();
    assert!(text.contains("\"kind\":\"add\""));
    assert_eq!(Results::from_text(&text).unwrap(), m);
    // A `where` that no base row has is fine for an add (the experiment may be new).
    assert!(add_results(
        &base,
        &fresh,
        "B4",
        Some(("shape", "mul12")),
        false,
        AT,
        &yes
    )
    .is_ok());
}

#[test]
fn merge_add_refuses_any_overlap_with_the_base_and_empty_or_foreign_input() {
    let base = file(vec![group("assoc", "aa"), group("mul5", "bb")], REV_OLD);
    // One identity of the fresh run is already in the base: the whole add is refused.
    let overlap = file(vec![group("mul12", "cc"), group("assoc", "11")], REV_NEW);
    let e = add_results(&base, &overlap, "B4", None, false, AT, &yes).unwrap_err();
    assert!(e.contains("overlap"), "{e}");
    // The same rows as the base is a refresh, not an add.
    let same = file(vec![group("assoc", "11")], REV_NEW);
    assert!(add_results(&base, &same, "B4", None, false, AT, &yes)
        .unwrap_err()
        .contains("overlap"));
    // Every other refusal applies unchanged: dirty, ancestry, outside rows, an empty subset.
    let mut dirty = file(vec![group("mul12", "cc")], REV_NEW);
    dirty.meta.dirty_tree = true;
    assert!(add_results(&base, &dirty, "B4", None, false, AT, &yes)
        .unwrap_err()
        .contains("dirty_tree"));
    let fresh = file(vec![group("mul12", "cc")], REV_NEW);
    let no = |_: &str| Ok(false);
    assert!(add_results(&base, &fresh, "B4", None, false, AT, &no)
        .unwrap_err()
        .contains("ancestor"));
    let mut mixed = file(vec![group("mul12", "cc")], REV_NEW);
    for seed in 1..=5 {
        mixed.rows.push(good(
            "B2",
            Knob::of(&[("length", Kv::I(25)), ("divergence", Kv::F(0.05))]),
            "monolithic",
            seed,
        ));
    }
    assert!(add_results(&base, &mixed, "B4", None, false, AT, &yes)
        .unwrap_err()
        .contains("outside"));
    assert!(add_results(&base, &mixed, "B4", None, true, AT, &yes).is_ok());
    let e = add_results(&base, &fresh, "A1", None, false, AT, &yes).unwrap_err();
    assert!(e.contains("outside") || e.contains("nothing to add"), "{e}");
    let e = add_results(&base, &fresh, "A1", None, true, AT, &yes).unwrap_err();
    assert!(e.contains("nothing to add"), "{e}");
}

#[test]
fn validate_rejects_an_unknown_merge_kind() {
    let base = file(vec![group("assoc", "aa")], REV_OLD);
    let fresh = file(vec![group("mul5", "bb")], REV_NEW);
    let mut m = add_results(&base, &fresh, "B4", None, false, AT, &yes).unwrap();
    m.meta.merges[0].kind = Some("replace".into());
    assert!(validate(&m).join("|").contains("is not \"add\""));
    m.meta.merges[0].trees = vec![("x".into(), "y".into())];
    assert!(validate(&m).join("|").contains("merges[0]: trees"));
}

#[test]
fn a_refresh_keeps_every_other_row_and_the_existing_merges_entry_byte_identical() {
    // An early merges entry (no kind, no trees, as recorded before 0.2.0).  The committed recording
    // has no merges, so make one by a refresh and strip its trees.
    let (base_text, base) = {
        let raw = Results::from_text(&committed_text()).unwrap();
        let mut seed = raw.clone();
        seed.meta.trees = bogus_trees('d');
        let mut first = merge_results(&raw, &seed, "B3", None, true, AT, &yes).unwrap();
        first.meta.merges[0].trees.clear();
        let text = first.to_text();
        let parsed = Results::from_text(&text).unwrap();
        (text, parsed)
    };
    let old_entry = {
        let g: &Merge = &base.meta.merges[0];
        assert!(g.kind.is_none() && g.trees.is_empty());
        coding_adventures_bounded_json::serialize(
            &coding_adventures_bounded_json::JsonValue::Object(vec![(
                "rev".into(),
                coding_adventures_bounded_json::JsonValue::String(g.rev.clone()),
            )]),
        )
        .unwrap();
        // The old entry's own text, as it sits in the file.
        let start = base_text.find("\"merges\": [").unwrap() + "\"merges\": [".len();
        // The only entry ends where the array does.
        let end = base_text[start..].find("}]").unwrap() + start + 1;
        base_text[start..end].to_string()
    };
    // Refresh every B3 row with identical rows from a "fresh run" (a second merge entry appears).
    let mut fresh = base.clone();
    fresh.meta.merges.clear();
    fresh.meta.trees = bogus_trees('e');
    let m = merge_results(&base, &fresh, "B3", None, true, AT, &yes).unwrap();
    let new_text = m.to_text();
    assert_eq!(m.meta.merges.len(), base.meta.merges.len() + 1);
    assert!(
        m.meta.merges.last().unwrap().kind.is_none(),
        "a refresh omits the key"
    );
    assert!(
        new_text.contains(&old_entry),
        "the existing merges entry keeps its bytes"
    );
    let (old_rows, new_rows): (Vec<&str>, Vec<&str>) = (
        base_text
            .lines()
            .filter(|l| l.starts_with("    {"))
            .collect(),
        new_text
            .lines()
            .filter(|l| l.starts_with("    {"))
            .collect(),
    );
    assert_eq!(old_rows, new_rows, "every row line is byte-identical");
    // Nothing but the merges line (and the new entry's trees) differs.
    let diff =
        |a: &str, b: &str| -> usize { a.lines().zip(b.lines()).filter(|(x, y)| x != y).count() };
    assert_eq!(
        diff(&base_text, &new_text),
        1,
        "only the merges line changes"
    );
}

// ---- --jobs ---------------------------------------------------------------------------------------

#[test]
fn jobs_are_clamped_to_one_through_sixty_four() {
    assert_eq!(clamp_jobs(0), 1);
    assert_eq!(clamp_jobs(1), 1);
    assert_eq!(clamp_jobs(8), 8);
    assert_eq!(clamp_jobs(64), 64);
    assert_eq!(clamp_jobs(65), 64);
    assert_eq!(clamp_jobs(usize::MAX), 64);
}

#[cfg(unix)]
mod unix_children {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use vos_capacity_probe::runner::{
        launch_command, run_child, run_one, CHILD_OUTPUT_CAP, ULIMIT_SCRIPT,
    };
    use vos_capacity_probe::spot::{full_rerun, rerun_one, spot_check};

    fn a1_16(seed: u64) -> Row {
        let mut r = committed()
            .rows
            .into_iter()
            .find(|r| r.experiment == "A1" && r.knob.int("n") == 16 && r.seed == seed)
            .unwrap();
        r.tier = "full".into();
        r
    }

    fn small_file(rows: Vec<Row>) -> Results {
        let mut f = committed();
        f.meta.merges.clear();
        f.rows = rows;
        f
    }

    #[test]
    fn a_clean_sampled_full_row_reproduces_and_a_hand_edited_one_fails_check() {
        let rows: Vec<Row> = (1..=5).map(a1_16).collect();
        let f = small_file(rows);
        let rep = spot_check(&f, std::path::Path::new(EXE), 4, Duration::from_secs(120));
        assert_eq!(rep.population, 5);
        assert_eq!(
            (rep.rerun, rep.skipped_expensive, rep.not_reached),
            (5, 0, 0),
            "{rep:?}"
        );
        // A 5-row file is below the (plain-constant) floor of 16 eligible rows, and says so; that
        // is the only violation (round 2, finding 6).
        assert!(
            rep.violations.iter().all(|v| v.contains("floor of 16")),
            "{:?}",
            rep.violations
        );
        assert!(rep.line().contains("re-run 5"));
        // Hand-edit one row's recorded measurement (and its digest, as a forger would).
        for edit in [0usize, 1, 2] {
            let mut g = f.clone();
            match edit {
                0 => g.rows[2].conflicts += 7,
                1 => g.rows[2].instance_sha256 = "00".repeat(32),
                _ => g.rows[2].result = "unknown".into(),
            }
            let rep = spot_check(&g, std::path::Path::new(EXE), 4, Duration::from_secs(120));
            assert!(
                rep.violations.iter().any(|v| !v.contains("floor of 16")),
                "edit {edit} slipped through"
            );
        }
    }

    #[test]
    fn a_spent_budget_stops_the_sample_and_the_floor_makes_that_a_failure() {
        let f = small_file((1..=5).map(a1_16).collect());
        let rep = spot_check(&f, std::path::Path::new(EXE), 2, Duration::ZERO);
        assert_eq!((rep.rerun, rep.not_reached), (0, 5), "{rep:?}");
        // Finding M4: a check that verified fewer than the floor fails (here 0 of 5 eligible rows).
        assert!(
            rep.violations
                .iter()
                .any(|v| v.contains("below the floor of 16")),
            "{:?}",
            rep.violations
        );
    }

    /// Finding M4, both directions: the cost class is the PLAN's, so editing the recorded times
    /// or result of a row can neither get a cheap row skipped nor make an expensive one run.
    #[test]
    fn the_cost_class_is_the_plans_and_recorded_times_and_results_are_never_read() {
        let mut rows: Vec<Row> = (1..=5).map(a1_16).collect();
        // A cheap row edited to look enormous and killed: it is still re-run (and caught).
        rows[0].solve_ms = 1.0e9;
        rows[0].result = "killed".into();
        // An expensive row (D1 k=1 t=256 took minutes) edited to look instant: still skipped.
        let mut hard = committed()
            .rows
            .into_iter()
            .find(|r| r.experiment == "D1" && r.knob.int("t") == 256 && r.knob.int("k") == 1)
            .unwrap();
        hard.solve_ms = 0.0;
        hard.check_ms = Some(0.0);
        hard.result = "unsat".into();
        rows.push(hard);
        let f = small_file(rows);
        let rep = spot_check(&f, std::path::Path::new(EXE), 4, Duration::from_secs(120));
        assert_eq!(rep.skipped_expensive, 1, "{rep:?}");
        assert_eq!(rep.rerun, 5, "{rep:?}");
        assert!(
            rep.violations.iter().any(|v| v.contains("result differs")),
            "the edited cheap row was caught: {:?}",
            rep.violations
        );
        assert!(rep
            .line()
            .contains("skipped (plan cost class expensive): 1"));
    }

    /// Finding M5: the hard limit of a sampled child is what is left of the budget plus the
    /// slack, and a child the bound kills is NOT verified, which fails the check.
    #[test]
    fn a_sampled_child_that_outlives_the_budget_is_killed_and_not_counted_as_verified() {
        // D1 t=8 k=2 is in the cheap class and takes about two seconds: its FASTEST seed in the
        // CI recording is 1.98 s of solve plus check (seeds 1.98 to 2.26 s), ten times the 200 ms
        // budget below, so the first child is killed on any host that is not ten times faster.  (It was D1 t=4 k=4 until VOS-D143 re-derived the cheap table from the
        // CI recording, where that group's slowest seed is over the 3 s line.)
        let rows: Vec<Row> = committed()
            .rows
            .into_iter()
            .filter(|r| r.experiment == "D1" && r.knob.int("t") == 8 && r.knob.int("k") == 2)
            .collect();
        assert_eq!(rows.len(), 5);
        assert!(vos_capacity_probe::cost::is_cheap(
            "D1",
            "monolithic",
            &rows[0].knob.key()
        ));
        let f = small_file(rows.clone());
        let t0 = Instant::now();
        let rep = vos_capacity_probe::spot::spot_check_in(
            &f,
            std::path::Path::new(EXE),
            5,
            Duration::from_millis(200),
            Duration::ZERO,
        );
        assert!(t0.elapsed() < Duration::from_secs(30), "{:?}", t0.elapsed());
        assert!(rep.unverified >= 1, "{rep:?}");
        assert!(
            rep.violations.iter().any(|v| v.contains("not verified")),
            "{:?}",
            rep.violations
        );
        assert!(
            rep.violations.iter().any(|v| v.contains("below the floor")),
            "an unverified row does not count toward the floor: {:?}",
            rep.violations
        );
        // The single-row function says the same, with an explicit bound.
        let planned = vos_capacity_probe::results::planned_identities();
        let v = rerun_one(
            std::path::Path::new(EXE),
            &rows[0],
            &planned,
            Some(Duration::from_millis(1)),
            false,
        );
        assert!(v[0].contains("not verified"), "{v:?}");
        // Security review of VOS-028c3c-2a: a bound-killed re-run is not verified even when the
        // recorded row is itself `killed` (a `killed` record must not vouch for a killed re-run).
        let mut killed = rows[0].clone();
        killed.result = "killed".into();
        let v = rerun_one(
            std::path::Path::new(EXE),
            &killed,
            &planned,
            Some(Duration::from_millis(1)),
            false,
        );
        assert!(v[0].contains("not verified"), "{v:?}");
    }

    #[test]
    fn a_row_outside_the_plan_is_refused_without_being_run() {
        let mut r = a1_16(1);
        // The size of a hand-edited knob: if this ran it would try to build 10^9-wide adders.
        r.knob = Knob::of(&[("n", Kv::I(1_000_000_000))]);
        let t0 = Instant::now();
        let planned = vos_capacity_probe::results::planned_identities();
        let v = rerun_one(std::path::Path::new(EXE), &r, &planned, None, false);
        assert!(v[0].contains("not in the plan"), "{v:?}");
        assert!(t0.elapsed() < Duration::from_secs(5), "nothing was started");
        let f = small_file(vec![r]);
        let rep = spot_check(&f, std::path::Path::new(EXE), 1, Duration::from_secs(60));
        assert!(
            rep.violations.iter().any(|v| v.contains("not in the plan")),
            "{:?}",
            rep.violations
        );
        // validate refuses it too, so `check` never reaches the sample.
        assert!(validate(&f)
            .join("|")
            .contains("not one of the plan's knobs"));
    }

    #[test]
    fn check_full_reruns_every_full_row_and_catches_an_edit() {
        let f = small_file((1..=3).map(a1_16).collect());
        let seen = std::sync::atomic::AtomicUsize::new(0);
        let v = full_rerun(&f, std::path::Path::new(EXE), 3, &|_, total| {
            assert_eq!(total, 3);
            seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        assert!(v.is_empty(), "{v:?}");
        assert_eq!(seen.load(std::sync::atomic::Ordering::SeqCst), 3);
        let mut g = f.clone();
        g.rows[1].conflicts += 1;
        let v = full_rerun(&g, std::path::Path::new(EXE), 3, &|_, _| {});
        assert!(v.iter().any(|x| x.contains("conflicts differs")), "{v:?}");
        // A quick-tier row is not a full row: it is not re-run here (check re-runs it in process).
        let mut q = f.clone();
        q.rows.iter_mut().for_each(|r| r.tier = "quick".into());
        assert!(full_rerun(&q, std::path::Path::new(EXE), 1, &|_, _| {}).is_empty());
    }

    // ---- the launch command and what reaches the shell ------------------------------------------

    fn spec_with(experiment: &str, knob: Knob, mode: &str) -> RowSpec {
        RowSpec {
            experiment: experiment.into(),
            knob,
            mode: mode.into(),
            seed: 1,
            seeds: 5,
            tier: "full".into(),
        }
    }

    const EVIL: [&str; 4] = [
        "; touch PWNED #",
        "$(touch PWNED)",
        "`touch PWNED`",
        "' ; touch PWNED ; '",
    ];

    #[test]
    fn the_shell_text_is_the_fixed_literal_and_every_variable_part_is_a_separate_argv_entry() {
        assert_eq!(
            ULIMIT_SCRIPT,
            "ulimit -t \"$1\" || exit 126; shift; exec \"$@\""
        );
        for evil in EVIL {
            let spec = spec_with(evil, Knob::of(&[("n", Kv::S(evil.into()))]), evil);
            let cmd = launch_command(std::path::Path::new(EXE), &spec, 600);
            assert_eq!(cmd.get_program(), "/bin/sh");
            let args: Vec<String> = cmd
                .get_args()
                .map(|a| a.to_string_lossy().into_owned())
                .collect();
            // sh -c SCRIPT sh N /usr/bin/time FLAG EXE row --spec JSON --wall 600
            assert_eq!(args[0], "-c");
            assert_eq!(args[1], ULIMIT_SCRIPT, "the script never contains the data");
            assert_eq!(args[2], "sh");
            // N is keyed on the EXPERIMENT (VOS-D123), not on the `wall_s` argument: these specs
            // name a hostile, unknown experiment, which gets the 120 s row budget, the 30 s
            // control stage and the 150 s slack, 300 s.  (In production `wall_s` is always
            // `row_wall_s` of the same experiment, so the two never disagree.)
            assert_eq!(
                args[3], "300",
                "N is the number formatted from the budget constants of the experiment"
            );
            assert!(args[3].bytes().all(|b| b.is_ascii_digit()));
            assert_eq!(args[4], "/usr/bin/time");
            let json = &args[args.iter().position(|a| a == "--spec").unwrap() + 1];
            assert!(
                json.contains("touch PWNED"),
                "the data reaches the program as one inert argument"
            );
            assert_eq!(args.iter().filter(|a| a.contains("touch")).count(), 1);
        }
    }

    #[test]
    fn a_malicious_knob_and_budget_string_creates_no_file_and_never_runs() {
        let d = scratch("evil");
        for evil in EVIL {
            // The shell would create the file in its working directory; the path is absolute so
            // a creation anywhere in this scratch directory or the CWD is found.
            let target = d.join("PWNED");
            let payload = evil.replace("PWNED", target.to_str().unwrap());
            let spec = spec_with(
                &format!("A1{payload}"),
                Knob::of(&[("n", Kv::S(payload.clone()))]),
                &payload,
            );
            let row = run_one(std::path::Path::new(EXE), &spec);
            assert_ne!(row.result, "unsat", "{payload}");
            assert!(!target.exists(), "{payload} reached a shell");
            assert!(!std::path::Path::new("PWNED").exists());
            // And as a results file, `validate` refuses the knob before anything runs.
            let mut f = committed();
            let i = f.rows.iter().position(|r| r.experiment == "A1").unwrap();
            f.rows[i].knob = Knob::of(&[("n", Kv::S(payload.clone()))]);
            assert!(!validate(&f).is_empty());
            // A budget that is not an integer does not even parse.
            let bad = committed_text().replacen(
                "\"note\":",
                &format!("\"wall_s\":\"{payload}\",\"note\":"),
                1,
            );
            assert!(Results::from_text(&bad).is_err());
        }
        // The real command line also rejects a non-numeric --wall by using the default.
        let out = std::process::Command::new(EXE)
            .args(["row", "--spec", "{}", "--wall", "; touch X"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
    }

    // ---- child output is capped, a child that ignores the kill cannot hang the parent ----------

    fn stub(dir: &std::path::Path, name: &str, body: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p
    }

    #[test]
    fn a_child_that_prints_without_end_is_read_through_a_one_mebibyte_cap() {
        let d = scratch("noisy");
        // 6 MiB on stdout and 6 MiB on stderr, then exits.
        let exe = stub(
            &d,
            "noisy.sh",
            "head -c 6291456 /dev/zero | tr '\\0' 'x'; head -c 6291456 /dev/zero | tr '\\0' 'y' >&2",
        );
        let spec = spec_with("A1", Knob::of(&[("n", Kv::I(8))]), "monolithic");
        let ch = run_child(&exe, &spec, 120, 4 * 1024 * 1024, hard_limit()).unwrap();
        assert!(
            ch.stdout.len() as u64 <= CHILD_OUTPUT_CAP,
            "{}",
            ch.stdout.len()
        );
        assert!(
            ch.stderr.len() as u64 <= CHILD_OUTPUT_CAP,
            "{}",
            ch.stderr.len()
        );
        assert!(ch.stdout.starts_with("xxx") && ch.stderr.contains('y'));
        assert!(matches!(ch.end, vos_capacity_probe::runner::End::Exited(_)));
    }

    #[test]
    fn a_runaway_child_is_killed_and_the_parent_returns_within_the_bounded_wait() {
        let d = scratch("runaway");
        let exe = stub(&d, "spin.sh", "while :; do :; done");
        let spec = spec_with("A1", Knob::of(&[("n", Kv::I(8))]), "monolithic");
        let t0 = Instant::now();
        let ch = run_child(
            &exe,
            &spec,
            120,
            4 * 1024 * 1024,
            Duration::from_millis(300),
        )
        .unwrap();
        assert_eq!(ch.end, vos_capacity_probe::runner::End::Killed);
        assert!(t0.elapsed() < Duration::from_secs(20), "{:?}", t0.elapsed());
    }
}

// ---- VOS-D140: input-level entries; the early merge records keep their whole-crate names -----

#[test]
fn legacy_whole_crate_names_are_history_in_a_merge_record_only() {
    use vos_capacity_probe::results::LEGACY_TREE_NAMES;
    let legacy: Vec<(String, String)> = LEGACY_TREE_NAMES
        .iter()
        .map(|n| (n.to_string(), "a".repeat(40)))
        .collect();
    // An early merge record carries them and the file validates (the committed recording has no
    // merges, so one is built: a refresh whose record is then given the legacy names).
    let mut with_legacy = committed();
    let mut fresh = with_legacy.clone();
    fresh.meta.trees = bogus_trees('d');
    with_legacy = merge_results(&with_legacy, &fresh, "B3", None, true, AT, &yes).unwrap();
    with_legacy.meta.merges.last_mut().unwrap().trees = legacy.clone();
    assert!(with_legacy.meta.merges.iter().any(|g| g.trees == legacy));
    assert!(
        validate(&with_legacy).is_empty(),
        "{:?}",
        validate(&with_legacy)
    );
    // In the `tool` block they are refused: that block must name the entries.
    let mut f = committed();
    f.meta.trees = legacy.clone();
    assert!(validate(&f).join("|").contains("trees must name exactly"));
    // A legacy record is never compared name by name: one advisory line, and `check --full`
    // refuses it.
    let cur = bogus_trees('b');
    let v = stale_lines("merges[0]", &legacy, &cur);
    assert_eq!(v.len(), 1, "{v:?}");
    assert!(v[0].contains("whole-crate"), "{}", v[0]);
}
