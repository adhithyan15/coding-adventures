//! The pure logic of `results`: the green predicate clause by clause, the `check` rules each with a
//! negative control, the re-run comparison, and the section 8.3 decision on synthetic rows.

use vos_capacity_probe::exp::{plan, Tier};
use vos_capacity_probe::results::*;
use vos_capacity_probe::row::{Control, Knob, Kv, Row};

fn ok_control() -> Control {
    Control {
        result: "sat".into(),
        model_checks_cnf: true,
        replayed: true,
    }
}

/// A green monolithic row.
fn good(exp: &str, knob: Knob, mode: &str, seed: u64) -> Row {
    let mut r = Row::new(exp, knob, mode, seed, 5);
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

fn five(f: impl FnMut(u64) -> Row) -> Vec<Row> {
    (1..=5).map(f).collect()
}

fn meta() -> Meta {
    Meta {
        name: "vos-capacity-probe".into(),
        version: "0.1.0".into(),
        binary_sha256: "00".into(),
        dirty_tree: false,
        triple: "t".into(),
        rustc: "rustc".into(),
        os: "os".into(),
        revisions: vec![("repo".into(), "abc".into())],
        regeneration: None,
        merges: Vec::new(),
        trees: Vec::new(),
    }
}

#[test]
fn green_requires_every_clause() {
    let base = good("A1", Knob::of(&[("n", Kv::I(8))]), "monolithic", 1);
    assert!(green(&base));
    let mut r = base.clone();
    r.result = "sat".into();
    assert!(!green(&r));
    let mut r = base.clone();
    r.control.replayed = false;
    assert!(!green(&r), "an unreplayed control");
    let mut r = base.clone();
    r.conflicts = 100_001;
    assert!(!green(&r), "over 100,000 conflicts");
    let mut r = base.clone();
    r.worst_step_conflicts = Some(100_001);
    assert!(!green(&r), "a worst step over 100,000 conflicts");
    let mut r = base.clone();
    r.worst_step_solve_ms = 10_001.0;
    assert!(!green(&r), "over 10 s");
    let mut r = base.clone();
    r.drat_bytes = Some(32 * 1024 * 1024 + 1);
    assert!(!green(&r), "DRAT over 32 MB");
    let mut r = base.clone();
    r.drat_bytes = None;
    assert!(!green(&r), "no proof");
    let mut r = base.clone();
    r.pass_default_limits = Some(false);
    assert!(!green(&r), "rejected by the default limits");
    let mut r = base.clone();
    r.peak_rss_kb = GREEN_MAX_RSS_KB + 1;
    assert!(!green(&r), "over 1 GB");
    let mut r = base.clone();
    r.peak_rss_kb = 0;
    assert!(!green(&r), "an unparsed RSS");
    let mut r = base.clone();
    r.cap = "wall".into();
    assert!(!green(&r), "a cap tripped");
    for bad in [
        "collapsed",
        "unknown",
        "killed",
        "oom",
        "crashed",
        "refused",
    ] {
        let mut r = base.clone();
        r.result = bad.into();
        assert!(!green(&r), "{bad}");
    }
}

#[test]
fn an_incremental_row_needs_no_proof_and_an_enumeration_row_needs_its_control() {
    let mut inc = good("B3", Knob::of(&[("length", Kv::I(25))]), "incremental", 1);
    inc.drat_bytes = None;
    inc.pass_default_limits = None;
    assert!(green(&inc));
    let mut e = Row::new("E1", Knob::default(), "enum", 1, 5);
    e.result = "pass".into();
    e.control = Control {
        result: "caught".into(),
        model_checks_cnf: true,
        replayed: true,
    };
    e.peak_rss_kb = 1;
    assert!(green(&e));
    e.control = Control::none("the broken adapter passed");
    assert!(!green(&e));
}

#[test]
fn red_reasons_name_what_failed() {
    let base = good("A1", Knob::of(&[("n", Kv::I(8))]), "monolithic", 1);
    let mut r = base.clone();
    r.conflicts = 200_000;
    r.worst_step_solve_ms = 20_000.0;
    r.pass_default_limits = Some(false);
    let why = red_reason(&r);
    assert!(
        why.contains("conflicts") && why.contains("solve") && why.contains("default limits"),
        "{why}"
    );
    let mut r = base.clone();
    r.result = "unknown".into();
    assert_eq!(red_reason(&r), "unknown");
    let mut r = base.clone();
    r.control = Control::none("no SAT control");
    assert!(red_reason(&r).starts_with("control"));
    let mut r = base;
    r.drat_bytes = Some(40 * 1024 * 1024);
    r.peak_rss_kb = 2 * 1024 * 1024;
    assert!(red_reason(&r).contains("DRAT") && red_reason(&r).contains("RSS"));
}

fn valid_file() -> Results {
    Results {
        meta: meta(),
        rows: five(|s| good("A1", Knob::of(&[("n", Kv::I(8))]), "monolithic", s)),
    }
}

#[test]
fn a_clean_file_validates_and_each_rule_has_a_negative_control() {
    assert_eq!(validate(&valid_file()), Vec::<String>::new());
    let mutate = |f: &dyn Fn(&mut Results)| {
        let mut r = valid_file();
        f(&mut r);
        validate(&r).join(" | ")
    };
    assert!(mutate(&|r| r.meta.dirty_tree = true).contains("dirty_tree"));
    assert!(mutate(&|r| r.rows.clear()).contains("no rows"));
    assert!(mutate(&|r| r.rows[0].result = "weird".into()).contains("closed set"));
    assert!(mutate(&|r| r.rows[0].peak_rss_kb = 0).contains("peak RSS"));
    assert!(mutate(&|r| r.rows[0].green = false).contains("green is false"));
    assert!(mutate(&|r| r.rows[0].control.replayed = false).contains("no replayed SAT control"));
    assert!(
        mutate(&|r| r.rows[0].control_instance_sha256.clear()).contains("control_instance_sha256")
    );
    assert!(mutate(&|r| r.rows[0].drat_sha256 = None).contains("no DRAT digest"));
    assert!(mutate(&|r| r.rows[0].seeds = 3).contains("seeds are recorded"));
    assert!(mutate(&|r| {
        r.rows.truncate(4);
    })
    .contains("distinct seeds"));
    assert!(mutate(&|r| r.rows[0].cap = "disk".into()).contains("cap"));
    // A collapsed row is never green, needs a reason from the enum or a note.
    assert!(mutate(&|r| {
        r.rows[0].result = "collapsed".into();
        r.rows[0].collapsed_reason = Some("made-up".into());
    })
    .contains("section 4.5 enum"));
    assert!(mutate(&|r| {
        r.rows[0].result = "collapsed".into();
        r.rows[0].green = false;
    })
    .contains("without a reason"));
    assert!(mutate(&|r| {
        r.rows[0].result = "unknown".into();
        r.rows[0].green = false;
    })
    .contains("must name the cap"));
    assert!(mutate(&|r| {
        r.rows[0].result = "oom".into();
        r.rows[0].green = false;
        r.rows[0].cap = "none".into();
    })
    .contains("cap rss"));
    // An enumeration row whose control did not pass.
    assert!(mutate(&|r| {
        r.rows[0].mode = "enum".into();
        r.rows[0].result = "pass".into();
        r.rows[0].control = Control::none("x");
        r.rows[0].green = false;
    })
    .contains("enumeration control"));
}

#[test]
fn the_file_round_trips_through_its_text() {
    let mut f = valid_file();
    f.meta.regeneration = Some(("A. Reviewer".into(), "host".into()));
    let text = f.to_text();
    assert!(text.starts_with("{\n  \"schema\": 2"));
    assert_eq!(
        text.lines().filter(|l| l.starts_with("    {")).count(),
        5,
        "one row per line"
    );
    assert_eq!(Results::from_text(&text).unwrap(), f);
    f.meta.regeneration = None;
    assert_eq!(Results::from_text(&f.to_text()).unwrap(), f);
    assert!(Results::from_text("not json").is_err());
    assert!(Results::from_text("[]").is_err());
    assert!(
        Results::from_text(&text.replace("\"schema\": 2", "\"schema\": 1"))
            .unwrap_err()
            .contains("schema")
    );
    assert!(Results::from_text(&text.replace("\"rows\": [", "\"rowz\": [")).is_err());
}

#[test]
fn compare_rerun_flags_each_deterministic_field() {
    let rec = good("A1", Knob::of(&[("n", Kv::I(8))]), "monolithic", 1);
    assert!(compare_rerun(&rec, &rec.clone()).is_empty());
    let diff = |f: &dyn Fn(&mut Row)| {
        let mut now = rec.clone();
        f(&mut now);
        compare_rerun(&rec, &now).join(" | ")
    };
    assert!(diff(&|r| r.instance_sha256 = "00".repeat(32)).contains("instance_sha256"));
    assert!(diff(&|r| r.drat_sha256 = None).contains("drat_sha256"));
    assert!(diff(&|r| r.control_instance_sha256 = "x".into()).contains("control_instance_sha256"));
    assert!(diff(&|r| r.result = "collapsed".into()).contains("result"));
    assert!(diff(&|r| r.control.replayed = false).contains("control"));
    assert!(diff(&|r| r.conflicts += 1).contains("conflicts"));
    assert!(diff(&|r| r.worst_step_conflicts = Some(3)).contains("worst_step_conflicts"));
    assert!(diff(&|r| r.steps = 9).contains("steps"));
    assert!(diff(&|r| r.ands = 9).contains("ands"));
    assert!(diff(&|r| r.pass_default_limits = Some(false)).contains("pass_default_limits"));
    // Times are not compared (they are host-specific); counted metrics are.
    let mut with_metric = rec.clone();
    with_metric.metrics = vec![("cases".into(), 256.0), ("run_ms".into(), 5.0)];
    let mut slower = with_metric.clone();
    slower.metrics = vec![("cases".into(), 256.0), ("run_ms".into(), 50.0)];
    slower.solve_ms = 99.0;
    assert!(compare_rerun(&with_metric, &slower).is_empty());
    slower.metrics = vec![("cases".into(), 255.0), ("run_ms".into(), 5.0)];
    assert!(compare_rerun(&with_metric, &slower)
        .join(" ")
        .contains("metric cases"));
}

#[test]
fn quick_specs_and_plan_coverage() {
    let mut f = valid_file();
    for r in &mut f.rows {
        r.tier = "quick".into();
    }
    assert_eq!(quick_specs(&f).len(), 5);
    // The plan's quick tier has many experiments; a file with only A1 does not cover it.
    let missing = quick_plan_covered(&f);
    assert!(missing.iter().any(|m| m.contains("B1")), "{missing:?}");
    assert!(!missing.iter().any(|m| m.contains("A1")));
    assert_eq!(
        rerun(
            &Results {
                meta: meta(),
                rows: vec![]
            },
            1
        )
        .len(),
        1,
        "a file with no quick row cannot be re-run"
    );
    assert!(plan(Tier::Quick, &[], 5).len() < plan(Tier::Full, &[], 5).len());
}

// ----- the decision -----------------------------------------------------------------------

fn group(exp: &str, knob: Knob, mode: &str, green: bool) -> Vec<Row> {
    five(|s| {
        let mut r = good(exp, knob.clone(), mode, s);
        if !green {
            r.conflicts = 500_000;
            r.worst_step_conflicts = Some(500_000);
            r.green = false;
        }
        r
    })
}

fn kn(pairs: &[(&str, Kv)]) -> Knob {
    Knob::of(pairs)
}

/// Every shape green: row 3.  `bad` lists (experiment, knob key substring) to make non-green.
fn everything(bad: &[&str]) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut add = |exp: &str, knob: Knob, mode: &str| {
        let key = format!("{exp} {}", knob.key());
        let g = !bad.iter().any(|b| key.contains(b));
        rows.extend(group(exp, knob, mode, g));
    };
    for e in ["A1", "A2"] {
        for n in [8, 16, 32, 64] {
            add(e, kn(&[("n", Kv::I(n))]), "monolithic");
        }
    }
    for len in [200, 1000] {
        for cut in ["every1", "every8", "every32"] {
            add(
                "B3",
                kn(&[
                    ("length", Kv::I(len)),
                    ("divergence", Kv::F(0.25)),
                    ("cut", Kv::S(cut.into())),
                ]),
                "certified",
            );
        }
        add(
            "C1",
            kn(&[
                ("base", Kv::S("b".into())),
                ("length", Kv::I(len)),
                ("divergence", Kv::F(0.25)),
            ]),
            "monolithic",
        );
    }
    for t in [8, 16, 64, 256] {
        add("D3", kn(&[("t", Kv::I(t)), ("k", Kv::I(16))]), "split");
    }
    add(
        "D2",
        kn(&[("t", Kv::I(64)), ("k", Kv::I(16))]),
        "monolithic",
    );
    for n in 3..=7 {
        add(
            "B4",
            kn(&[
                ("shape", Kv::S("assoc".into())),
                ("n", Kv::I(n)),
                ("w", Kv::I(32)),
            ]),
            "monolithic",
        );
    }
    rows
}

#[test]
fn all_green_is_feasible_as_planned() {
    let d = decide(&everything(&[]));
    assert_eq!((d.row, d.title), (3, TITLES[2]), "{:?}", d.because);
}

#[test]
fn a_required_shape_that_is_not_green_is_infeasible() {
    for bad in [
        "A1 {\"n\":32}",
        "D3 {\"t\":8,\"k\":16}",
        "B3 {\"length\":200,\"divergence\":0.25,\"cut\":\"every1\"}",
        "C1 {\"base\":\"b\",\"length\":200",
    ] {
        let d = decide(&everything(&[bad]));
        assert_eq!(d.row, 1, "{bad}: {:?}", d.because);
        assert!(!d.because.is_empty());
    }
}

#[test]
fn a_missing_shape_is_red_not_green() {
    let rows: Vec<Row> = everything(&[])
        .into_iter()
        .filter(|r| r.experiment != "C1")
        .collect();
    let d = decide(&rows);
    assert_eq!(d.row, 1);
    assert!(
        d.because.iter().any(|b| b.contains("no rows")),
        "{:?}",
        d.because
    );
    assert_eq!(decide(&[]).row, 1);
}

#[test]
fn reduced_and_mitigated_when_only_part_of_the_range_is_green() {
    // B3 green at 200 instructions but not at 1000 (every instruction): "only up to about 200".
    let d = decide(&everything(&[
        "B3 {\"length\":1000,\"divergence\":0.25,\"cut\":\"every1\"}",
    ]));
    assert_eq!(d.row, 2, "{:?}", d.because);
    // D3 not green above T = 16.
    let d = decide(&everything(&["D3 {\"t\":64", "D3 {\"t\":256"]));
    assert_eq!(d.row, 2, "{:?}", d.because);
    // B4 re-association green only up to n = 4.
    let d = decide(&everything(&[
        "B4 {\"shape\":\"assoc\",\"n\":5",
        "B4 {\"shape\":\"assoc\",\"n\":6",
        "B4 {\"shape\":\"assoc\",\"n\":7",
    ]));
    assert_eq!(d.row, 2, "{:?}", d.because);
    // Cuts of 8 or fewer green, 32 not green, at 1000 instructions.
    let d = decide(&everything(&[
        "B3 {\"length\":1000,\"divergence\":0.25,\"cut\":\"every32\"",
    ]));
    assert_eq!(d.row, 2, "{:?}", d.because);
}

#[test]
fn a_collapsed_seed_is_excluded_but_a_fully_collapsed_group_is_red() {
    let mut rows = everything(&[]);
    // One collapsed seed of A1 at 32: the other four are green, so the group is green.
    let a1 = rows
        .iter()
        .position(|r| r.experiment == "A1" && r.knob.int("n") == 32)
        .unwrap();
    rows[a1].result = "collapsed".into();
    rows[a1].green = false;
    assert_eq!(decide(&rows).row, 3);
    // All five collapsed: unclassified is red.
    for r in rows
        .iter_mut()
        .filter(|r| r.experiment == "A1" && r.knob.int("n") == 32)
    {
        r.result = "collapsed".into();
        r.green = false;
    }
    assert_eq!(decide(&rows).row, 1);
}

#[test]
fn row_three_needs_the_b3_step_and_time_budget() {
    // Green but with a worst step over 10k conflicts at 1000 instructions: not "as planned".
    let mut rows = everything(&[]);
    for r in rows
        .iter_mut()
        .filter(|r| r.experiment == "B3" && r.knob.int("length") == 1000)
    {
        r.worst_step_conflicts = Some(50_000);
    }
    let d = decide(&rows);
    assert_ne!(d.row, 3, "{:?}", d.because);
    assert!(
        d.because.iter().any(|b| b.contains("B3")),
        "{:?}",
        d.because
    );
    // Total over 60 s.
    let mut rows = everything(&[]);
    for r in rows
        .iter_mut()
        .filter(|r| r.experiment == "B3" && r.knob.int("length") == 1000)
    {
        r.solve_ms = 70_000.0;
    }
    assert_ne!(decide(&rows).row, 3);
}

#[test]
fn groups_keep_file_order_and_share_a_key() {
    let rows = everything(&[]);
    let gs = groups(&rows);
    assert!(gs.iter().all(|g| g.rows.len() == 5));
    assert_eq!(gs[0].first().experiment, "A1");
    assert!(gs[0].max(|r| r.conflicts as f64) >= 1000.0);
    assert_eq!(gs[0].counted().len(), 5);
}

// ---- `merge`: refreshing a subset of the committed rows through the tool (VOS30 section 9.2) ----

use vos_capacity_probe::cli::merge_results;

const REV_OLD: &str = "1111111111111111111111111111111111111111";
const REV_NEW: &str = "2222222222222222222222222222222222222222";

/// The planned B4 knob of `shape` (the tests select a group of rows by its string field, as the
/// real refresh selects E1's `fixture`; `validate` holds every knob to the plan).
fn b4_knob(shape: &str) -> Knob {
    match shape {
        "assoc" => Knob::of(&[
            ("shape", Kv::S(shape.into())),
            ("n", Kv::I(3)),
            ("w", Kv::I(32)),
        ]),
        _ => Knob::of(&[("shape", Kv::S(shape.into())), ("w", Kv::I(8))]),
    }
}

/// Five valid B4 rows of `shape` ("assoc", "mul5" or "mul12"), with instance digest `digest`.
fn grp(shape: &str, digest: &str) -> Vec<Row> {
    five(|s| {
        let mut r = good("B4", b4_knob(shape), "monolithic", s);
        r.instance_sha256 = digest.repeat(32);
        r
    })
}

fn file_of(groups: Vec<Vec<Row>>, rev: &str) -> Results {
    let mut m = meta();
    m.binary_sha256 = format!("{rev:0<64}").chars().take(64).collect();
    m.revisions = vec![("repo".into(), rev.into())];
    Results {
        meta: m,
        rows: groups.into_iter().flatten().collect(),
    }
}

const SEL: Option<(&str, &str)> = Some(("shape", "assoc"));
const AT: &str = "2026-11-18T00:00:00Z";
fn yes(_: &str) -> Result<bool, String> {
    Ok(true)
}
fn go(
    b: &Results,
    f: &Results,
    sel: Option<(&str, &str)>,
    outside: bool,
) -> Result<Results, String> {
    merge_results(b, f, "B4", sel, outside, AT, &yes)
}

fn base3() -> Results {
    file_of(
        vec![grp("assoc", "aa"), grp("mul5", "bb"), grp("mul12", "cc")],
        REV_OLD,
    )
}

#[test]
fn merge_replaces_only_the_subset_keeps_the_original_provenance_and_records_the_merge() {
    let base = base3();
    let fresh = file_of(vec![grp("assoc", "11")], REV_NEW);
    let m = go(&base, &fresh, SEL, false).unwrap();
    assert_eq!(m.rows.len(), 15);
    assert_eq!(m.rows[..5], fresh.rows[..], "the subset is the fresh rows");
    assert_eq!(m.rows[5..], base.rows[5..], "every other row is identical");
    // The file's own provenance is the base's, not the fresh run's.
    let mut want = base.meta.clone();
    want.merges = m.meta.merges.clone();
    assert_eq!(m.meta, want);
    assert_eq!(m.meta.merges.len(), 1);
    let g = &m.meta.merges[0];
    assert_eq!(
        (g.rev.as_str(), g.where_.as_str(), g.at.as_str()),
        (REV_NEW, "shape=assoc", AT)
    );
    assert_eq!(g.binary_sha256, fresh.meta.binary_sha256);
    assert_eq!(g.rows.len(), 5);
    // The output passes validate, and survives the text round trip with its merges.
    assert_eq!(validate(&m), Vec::<String>::new());
    assert_eq!(Results::from_text(&m.to_text()).unwrap(), m);
    // A second merge appends; the first entry stays.
    let again = go(&m, &file_of(vec![grp("assoc", "33")], REV_NEW), SEL, false).unwrap();
    assert_eq!(again.meta.merges.len(), 2);
    assert_eq!(validate(&again), Vec::<String>::new());
}

#[test]
fn merge_without_a_where_refreshes_the_whole_experiment() {
    let base = base3();
    let fresh = file_of(
        vec![grp("assoc", "11"), grp("mul5", "22"), grp("mul12", "33")],
        REV_NEW,
    );
    assert!(go(&base, &fresh, None, false).is_ok());
}

#[test]
fn an_invalid_or_dirty_fresh_run_is_refused() {
    let base = base3();
    let mut bad = file_of(vec![grp("assoc", "11")], REV_NEW);
    bad.rows[0].result = "sat".into(); // an UNSAT shape that is not, but still marked green
    assert!(go(&base, &bad, SEL, false).unwrap_err().contains("invalid"));
    let mut dirty = file_of(vec![grp("assoc", "11")], REV_NEW);
    dirty.meta.dirty_tree = true;
    assert!(go(&base, &dirty, SEL, false)
        .unwrap_err()
        .contains("dirty_tree"));
}

#[test]
fn a_dirty_tree_that_is_not_a_json_boolean_does_not_parse() {
    let text = base3().to_text();
    assert!(Results::from_text(&text).is_ok());
    for bad in ["\"false\"", "null", "0", "\"true\""] {
        let t = text.replacen("\"dirty_tree\":false", &format!("\"dirty_tree\":{bad}"), 1);
        assert_ne!(t, text, "the field was not found");
        assert!(
            Results::from_text(&t).unwrap_err().contains("dirty_tree"),
            "{bad}"
        );
    }
}

#[test]
fn a_revision_that_is_not_head_or_an_ancestor_is_refused() {
    let base = base3();
    let fresh = file_of(vec![grp("assoc", "11")], REV_NEW);
    let no = |_: &str| Ok(false);
    let e = merge_results(&base, &fresh, "B4", SEL, false, AT, &no).unwrap_err();
    assert!(e.contains("ancestor"), "{e}");
    let broken = |_: &str| Err("git cannot say".to_string());
    assert!(merge_results(&base, &fresh, "B4", SEL, false, AT, &broken).is_err());
}

#[test]
fn duplicate_ids_in_either_file_are_refused() {
    let base = base3();
    let mut fresh = file_of(vec![grp("assoc", "11")], REV_NEW);
    fresh.rows[1] = fresh.rows[0].clone();
    assert!(go(&base, &fresh, SEL, false).is_err());
    let mut dup_base = base3();
    dup_base.rows[6] = dup_base.rows[5].clone();
    let fresh = file_of(vec![grp("assoc", "11")], REV_NEW);
    assert!(go(&dup_base, &fresh, SEL, false)
        .unwrap_err()
        .contains("duplicate"));
}

#[test]
fn fresh_rows_outside_the_subset_are_refused_unless_ignored() {
    let base = base3();
    let fresh = file_of(vec![grp("assoc", "11"), grp("mul5", "22")], REV_NEW);
    assert!(go(&base, &fresh, SEL, false)
        .unwrap_err()
        .contains("outside"));
    let m = go(&base, &fresh, SEL, true).unwrap();
    assert_eq!(
        m.rows[5..],
        base.rows[5..],
        "the ignored rows do not leak in"
    );
}

#[test]
fn a_where_field_no_row_has_is_refused() {
    let base = base3();
    let fresh = file_of(vec![grp("assoc", "11")], REV_NEW);
    let e = go(&base, &fresh, Some(("shap", "assoc")), false).unwrap_err();
    assert!(e.contains("shap"), "{e}");
    // A value that matches nothing selects no row.
    assert!(go(&base, &fresh, Some(("shape", "zz")), false).is_err());
}

#[test]
fn a_changed_row_identity_is_refused() {
    let base = base3();
    let renamed = file_of(
        vec![grp("assoc", "11").into_iter().take(4).collect()],
        REV_NEW,
    );
    assert!(go(&base, &renamed, SEL, false).is_err());
}

#[test]
fn validate_checks_the_merges_list() {
    let m = go(
        &base3(),
        &file_of(vec![grp("assoc", "11")], REV_NEW),
        SEL,
        false,
    )
    .unwrap();
    let mutate = |f: &dyn Fn(&mut Results)| {
        let mut r = m.clone();
        f(&mut r);
        validate(&r)
    };
    assert!(!mutate(&|r| r.meta.merges[0].rev = "abc".into()).is_empty());
    assert!(!mutate(&|r| r.meta.merges[0].binary_sha256 = "zz".into()).is_empty());
    assert!(!mutate(&|r| r.meta.merges[0].dirty_tree = true).is_empty());
    assert!(!mutate(&|r| r.meta.merges[0].experiment = "ZZ".into()).is_empty());
    assert!(!mutate(&|r| r.meta.merges[0].rows.push("A1 nope".into())).is_empty());
    assert!(!mutate(&|r| {
        let x = r.meta.merges[0].rows[0].clone();
        r.meta.merges[0].rows.push(x)
    })
    .is_empty());
}

#[test]
fn the_table_banner_names_the_refreshed_rows_and_their_revision() {
    let plain = vos_capacity_probe::table::render_block(&base3());
    assert!(!plain.contains("refreshed at rev"));
    let m = go(
        &base3(),
        &file_of(vec![grp("assoc", "11")], REV_NEW),
        SEL,
        false,
    )
    .unwrap();
    let t = vos_capacity_probe::table::render_block(&m);
    assert!(
        t.contains(&format!("5 rows refreshed at rev {REV_NEW}")),
        "{t}"
    );
}
