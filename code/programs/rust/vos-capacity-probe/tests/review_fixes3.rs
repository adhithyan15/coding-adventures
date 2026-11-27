//! VOS-028c2, the security review's THIRD round (VOS30 section 9.12, "Hardening, round 3"): each
//! finding with a negative control that the fixed code refuses and the code before it accepted.
//! Tests that start child processes are unix-only.

#![cfg_attr(not(unix), allow(unused))]

use std::path::PathBuf;
use vos_capacity_probe::cli::{dispatch, Failure};
use vos_capacity_probe::cost::is_cheap;
use vos_capacity_probe::results::{
    collapse_violations, compare_structure, decision_groups_not_resolved, decision_read, validate,
    Results,
};
use vos_capacity_probe::row::Row;
use vos_capacity_probe::spot::structure_comparable;

const EXE: &str = env!("CARGO_BIN_EXE_vos-capacity-probe");

fn committed() -> Results {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/vos30-capacity-results.json");
    Results::from_text(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn b3_every8_seed2(f: &Results) -> usize {
    f.rows
        .iter()
        .position(|r| {
            r.experiment == "B3"
                && r.mode == "certified"
                && r.seed == 2
                && r.knob.int("length") == 1000
                && r.knob.s("cut") == "every8"
                && (r.knob.f("divergence") - 0.25).abs() < 1e-9
        })
        .unwrap()
}

fn relabel(r: &mut Row, reason: &str) {
    r.result = "collapsed".into();
    r.collapsed_reason = Some(reason.into());
    r.green = false;
}

// ---- 1. the collapse relabel ---------------------------------------------------------------------

#[test]
fn the_reviewers_relabel_of_a_red_chain_row_is_refused_by_validate() {
    let mut f = committed();
    assert!(validate(&f).is_empty(), "the committed file is clean");
    let i = b3_every8_seed2(&f);
    assert_eq!(f.rows[i].result, "unsat");
    relabel(&mut f.rows[i], "propagation-only");
    let v = validate(&f);
    assert!(
        v.iter()
            .any(|m| m.contains("cut\":\"every8") && m.contains("relabelled")),
        "{v:?}"
    );
    assert!(v.iter().any(|m| m.contains("chain row collapses only")));
}

#[test]
fn every_genuine_collapsed_row_of_the_committed_file_passes_the_rules() {
    let f = committed();
    let mut n = 0;
    for r in f.rows.iter().filter(|r| r.result == "collapsed") {
        assert!(collapse_violations("id", r).is_empty(), "{r:?}");
        n += 1;
    }
    assert_eq!(n, 19);
}

#[test]
fn a_forged_zero_conflict_relabel_still_fails_on_the_other_rules() {
    // The forger zeroes the conflicts too: a chain row with ANDs cannot be identical-structure.
    let mut f = committed();
    let i = b3_every8_seed2(&f);
    relabel(&mut f.rows[i], "identical-structure");
    f.rows[i].conflicts = 0;
    f.rows[i].worst_step_conflicts = Some(0);
    let v = validate(&f);
    assert!(v.iter().any(|m| m.contains("empty miter cone")), "{v:?}");
}

#[test]
fn reasonless_collapses_must_carry_a_g2_note_that_matches_the_row() {
    let f = committed();
    let i = b3_every8_seed2(&f);
    let base = f.rows[i].clone();
    let mut r = base.clone();
    r.result = "collapsed".into();
    r.collapsed_reason = None;
    // No note, an unrelated note, and an entropy note that is not below half: all refused.
    for note in [
        "".to_string(),
        "looks collapsed".to_string(),
        format!(
            "G2: {} of {} declared bits are sensitive",
            r.entropy_bits, r.declared_bits
        ),
        "G2: 1 of 100 declared bits are sensitive".to_string(),
    ] {
        r.note = note.clone();
        assert!(
            !collapse_violations("id", &r).is_empty(),
            "accepted note {note:?}"
        );
    }
    // The honest entropy collapse: numbers equal the row's fields and are below half.
    r.entropy_bits = 10;
    r.declared_bits = 100;
    r.note = "G2: 10 of 100 declared bits are sensitive".into();
    assert!(collapse_violations("id", &r).is_empty());
    // The 25% AND note is for a single instance only, with the row's own AND count.
    let a1 = f.rows.iter().find(|r| r.experiment == "A1").unwrap();
    let mut m = a1.clone();
    m.result = "collapsed".into();
    m.collapsed_reason = None;
    m.ands = 10;
    m.note = "G2: 10 ANDs is under 25% of 100 at divergence 100%".into();
    assert!(collapse_violations("id", &m).is_empty());
    m.note = "G2: 10 ANDs is under 25% of 30 at divergence 100%".into();
    assert!(!collapse_violations("id", &m).is_empty());
    m.note = "G2: 11 ANDs is under 25% of 100 at divergence 100%".into();
    assert!(!collapse_violations("id", &m).is_empty());
    let mut c = r.clone();
    c.ands = 10;
    c.note = "G2: 10 ANDs is under 25% of 100 at divergence 100%".into();
    assert!(
        !collapse_violations("id", &c).is_empty(),
        "chain rows have no AND rule"
    );
}

#[test]
fn a_collapse_reason_outside_the_enum_stops_at_the_enum_check() {
    let f = committed();
    let mut r = f.rows[b3_every8_seed2(&f)].clone();
    relabel(&mut r, "because-i-said-so");
    let v = collapse_violations("id", &r);
    assert_eq!(v.len(), 1);
    assert!(v[0].contains("not in the section 4.5 enum"));
}

fn built(result: &str, reason: Option<&str>, note: &str) -> Row {
    let f = committed();
    let mut r = f.rows[0].clone();
    r.result = result.into();
    r.collapsed_reason = reason.map(str::to_string);
    r.note = note.into();
    r
}

#[test]
fn the_rebuild_holds_unsat_and_collapsed_rows_to_its_own_verdicts() {
    let rec_unsat = built("unsat", None, "");
    let none = built("unknown", None, "");
    let ident = built("collapsed", Some("identical-structure"), "");
    // A recorded unsat row against a rebuild that collapses: a forged refutation.
    assert!(compare_structure(&rec_unsat, &none).is_empty());
    let v = compare_structure(&rec_unsat, &ident);
    assert!(v.iter().any(|m| m.contains("recorded unsat")), "{v:?}");
    // A recorded collapse the rebuild does not see.
    let rec_prop = built("collapsed", Some("propagation-only"), "");
    let v = compare_structure(&rec_prop, &none);
    assert!(v.iter().any(|m| m.contains("recorded collapsed")), "{v:?}");
    // A recorded collapse with the other reason.
    let v = compare_structure(&rec_prop, &ident);
    assert!(v.iter().any(|m| m.contains("recorded collapsed")), "{v:?}");
    assert!(compare_structure(&ident, &ident).is_empty());
    // A reasonless collapse must carry the rebuild's note; a rebuild with a reason is a mismatch.
    let note = "G2: 1 of 9 declared bits are sensitive";
    let rec_g2 = built("collapsed", None, note);
    let now_g2 = built("collapsed", None, note);
    assert!(compare_structure(&rec_g2, &now_g2).is_empty());
    assert!(!compare_structure(&rec_g2, &ident).is_empty());
    assert!(!compare_structure(&rec_g2, &none).is_empty());
    let other = built("collapsed", None, "G2: 2 of 9 declared bits are sensitive");
    assert!(!compare_structure(&other, &now_g2).is_empty());
}

#[cfg(unix)]
#[test]
fn the_structure_child_reports_the_collapse_the_rebuilt_instance_gives() {
    use vos_capacity_probe::exp::{plan, structure_of, Tier};
    let mut saw_reason = false;
    let mut saw_none = false;
    for spec in plan(Tier::Quick, &[], 1) {
        if spec.experiment.starts_with('E') {
            continue;
        }
        let s = structure_of(&spec).unwrap();
        if spec.experiment == "B1" {
            assert_eq!(s.collapse_reason.as_deref(), Some("identical-structure"));
            saw_reason = true;
        } else if s.collapse_reason.is_none() && s.collapse_note.is_empty() {
            saw_none = true;
        }
    }
    assert!(saw_reason && saw_none);
}

#[cfg(unix)]
#[test]
fn plain_check_refuses_the_relabelled_file_at_once() {
    let mut f = committed();
    let i = b3_every8_seed2(&f);
    relabel(&mut f.rows[i], "propagation-only");
    let d = std::env::temp_dir().join(format!("vos-probe-rev3-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let p = d.join("forged.json");
    std::fs::write(&p, f.to_text()).unwrap();
    let (mut o, mut e) = (Vec::new(), Vec::new());
    let args = vec![
        "check".to_string(),
        p.display().to_string(),
        "--exe".into(),
        EXE.into(),
    ];
    let r = dispatch(&args, &mut o, &mut e);
    assert!(matches!(r, Err(Failure::Violations(n)) if n >= 2), "{r:?}");
    let e = String::from_utf8(e).unwrap();
    assert!(e.contains("relabelled"), "{e}");
    let _ = std::fs::remove_dir_all(&d);
}

// ---- 2. unreached decision rows ------------------------------------------------------------------

fn c1_b200_file() -> Results {
    let mut f = committed();
    f.meta.merges.clear();
    f.rows.retain(|r| {
        r.experiment == "C1"
            && r.knob.s("base") == "b"
            && r.knob.int("length") == 200
            && (r.knob.f("divergence") - 0.25).abs() < 1e-9
    });
    f
}

#[cfg(unix)]
#[test]
fn a_decision_row_the_rebuild_does_not_reach_is_a_violation_naming_it() {
    use std::time::Duration;
    use vos_capacity_probe::spot::structure_check_with;
    let f = c1_b200_file();
    assert_eq!(f.rows.len(), 5);
    assert!(f.rows.iter().all(decision_read));
    // A zero decision budget reaches nothing: five violations, one per row, each with its identity.
    let rep = structure_check_with(
        &f,
        std::path::Path::new(EXE),
        2,
        Duration::ZERO,
        Duration::from_secs(5),
        Duration::from_secs(5),
        &[],
    );
    let named: Vec<_> = rep
        .violations
        .iter()
        .filter(|v| v.contains("decision row not verified"))
        .collect();
    assert_eq!(named.len(), 5, "{:?}", rep.violations);
    assert!(named.iter().all(|v| v.starts_with("C1 ")));
    assert_eq!(rep.verified, 0);
    // An ample budget verifies them all and reports no violation.
    let rep = structure_check_with(
        &f,
        std::path::Path::new(EXE),
        4,
        Duration::from_secs(120),
        Duration::from_secs(5),
        Duration::from_secs(30),
        &[],
    );
    assert!(rep.violations.is_empty(), "{:?}", rep.violations);
    assert_eq!(rep.critical_verified, 5);
    assert!(rep.line().contains("decision rows verified 5 of 5"));
}

#[cfg(unix)]
#[test]
fn a_decision_row_covered_by_the_spot_check_needs_no_rebuild() {
    use std::time::Duration;
    use vos_capacity_probe::spot::structure_check_with;
    let f = c1_b200_file();
    let rep = structure_check_with(
        &f,
        std::path::Path::new(EXE),
        1,
        Duration::ZERO,
        Duration::ZERO,
        Duration::from_secs(5),
        &[0, 1, 2, 3, 4],
    );
    assert!(rep.violations.is_empty(), "{:?}", rep.violations);
    assert_eq!(rep.covered_by_spot, 5);
    assert_eq!(rep.critical_verified, 5);
}

// ---- 3. the count follows the run ----------------------------------------------------------------

#[test]
fn the_unresolved_count_follows_what_the_run_re_solved() {
    let f = committed();
    let none = decision_groups_not_resolved(&f.rows, &[]);
    let cheap: Vec<usize> = f
        .rows
        .iter()
        .enumerate()
        .filter(|(_, r)| decision_read(r) && is_cheap(&r.experiment, &r.mode, &r.knob.key()))
        .map(|(i, _)| i)
        .collect();
    let all_cheap = decision_groups_not_resolved(&f.rows, &cheap);
    assert_eq!(all_cheap.len(), 12);
    assert!(none.len() > all_cheap.len(), "{none:?}");
    // Re-solving one row of a group removes exactly that group.
    let one = f
        .rows
        .iter()
        .position(|r| {
            decision_read(r)
                && r.tier != "quick"
                && none.contains(&vos_capacity_probe::results::group_key(r))
        })
        .unwrap();
    let after = decision_groups_not_resolved(&f.rows, &[one]);
    assert_eq!(after.len(), none.len() - 1);
}

#[test]
fn the_spec_no_longer_claims_what_a_slow_host_cannot_deliver() {
    let s = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../specs/VOS30-proof-route-and-capacity.md"),
    )
    .unwrap();
    let i = s.find("### 9.12 ").unwrap();
    let rest = &s[i..];
    let t = rest[..rest.find("\n## 10.").unwrap()]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for gone in [
        "so the gap is never silent",
        "sample probability that is 1 for the decision rows",
        "(80 of 85 on the committed file). A plain",
        "(N = 11 on the committed file)",
    ] {
        assert!(!t.contains(gone), "{gone}");
    }
    assert!(t.contains("Owner decision, 2026-11-20"));
    assert!(t.contains("not re-solved by this run"));
}

// ---- 4. a chain row that ended sat is partial ----------------------------------------------------

#[test]
fn a_chain_row_that_ended_sat_is_not_structurally_comparable() {
    let f = committed();
    let mut chain = f.rows[b3_every8_seed2(&f)].clone();
    chain.result = "sat".into();
    assert!(!structure_comparable(&chain));
    let mut single = f
        .rows
        .iter()
        .find(|r| r.experiment == "A1")
        .unwrap()
        .clone();
    single.result = "sat".into();
    assert!(structure_comparable(&single));
}

#[cfg(unix)]
#[test]
fn a_zero_conflict_propagation_only_relabel_of_a_single_instance_fails_the_rebuild() {
    use std::time::Duration;
    use vos_capacity_probe::spot::structure_check_with;
    // `validate` accepts this row (conflicts 0 satisfies the rule); only the rebuild, which sees
    // an instance of many clauses that does not collapse by itself, refuses it.
    let mut f = c1_b200_file();
    relabel(&mut f.rows[2], "propagation-only");
    f.rows[2].conflicts = 0;
    f.rows[2].worst_step_conflicts = Some(0);
    assert!(collapse_violations("id", &f.rows[2]).is_empty());
    let rep = structure_check_with(
        &f,
        std::path::Path::new(EXE),
        4,
        Duration::from_secs(120),
        Duration::from_secs(5),
        Duration::from_secs(30),
        &[],
    );
    assert!(
        rep.violations
            .iter()
            .any(|v| v.contains("recorded collapsed")),
        "{:?}",
        rep.violations
    );
    assert_eq!(rep.violations.len(), 1, "{:?}", rep.violations);
}

// ---- in-process coverage of the structure child and the plan refusals --------------------------

#[test]
fn the_structure_command_prints_a_collapsed_row_for_an_identical_pair() {
    use vos_capacity_probe::exp::{plan, Tier};
    use vos_capacity_probe::runner::spec_to_json;
    let b1 = plan(Tier::Quick, &[], 1)
        .into_iter()
        .find(|s| s.experiment == "B1")
        .unwrap();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let args = vec!["structure".to_string(), "--spec".into(), spec_to_json(&b1)];
    dispatch(&args, &mut out, &mut err).unwrap();
    let line = String::from_utf8(out).unwrap();
    assert!(line.contains("\"result\":\"collapsed\""), "{line}");
    assert!(line.contains("identical-structure"), "{line}");
    // A non-collapsing instance reports the neutral word `unknown` (built), never a verdict.
    let a1 = plan(Tier::Quick, &[], 1)
        .into_iter()
        .find(|s| s.experiment == "A1")
        .unwrap();
    let mut out = Vec::new();
    let args = vec!["structure".to_string(), "--spec".into(), spec_to_json(&a1)];
    dispatch(&args, &mut out, &mut Vec::new()).unwrap();
    assert!(String::from_utf8(out)
        .unwrap()
        .contains("\"result\":\"unknown\""));
}

#[cfg(unix)]
#[test]
fn a_row_outside_the_plan_is_refused_by_the_rebuild_and_the_rerun_without_running() {
    use vos_capacity_probe::results::planned_identities;
    use vos_capacity_probe::spot::{rerun_one, structure_one};
    let f = committed();
    let mut r = f.rows[b3_every8_seed2(&f)].clone();
    r.knob = vos_capacity_probe::row::Knob::default();
    let planned = planned_identities();
    let path = std::path::Path::new(EXE);
    let v = structure_one(path, &r, &planned, None);
    assert!(v.iter().any(|m| m.contains("not in the plan")), "{v:?}");
    let v = rerun_one(path, &r, &planned, None, false);
    assert!(v.iter().any(|m| m.contains("not in the plan")), "{v:?}");
}
