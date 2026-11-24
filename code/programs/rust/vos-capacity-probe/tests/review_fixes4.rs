//! VOS-028c3b security review: each finding with a negative control that the fixed code refuses
//! and the code before it accepted (VOS-D119 and the B5 hardening).  The missing-proof control is
//! a unit test in `src/sweep.rs`.

use std::time::Duration;
use vos_capacity_probe::exp::{run_row, shape_for, RowSpec};
use vos_capacity_probe::results::{b5_fuse1_violations, validate, Results};
use vos_capacity_probe::row::{Knob, Kv, Row};

fn committed() -> Results {
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/vos30-capacity-results.json");
    Results::from_text(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn b5_knob(len: i64, div: f64, fuse: i64) -> Knob {
    Knob::of(&[
        ("length", Kv::I(len)),
        ("divergence", Kv::F(div)),
        ("fuse", Kv::I(fuse)),
    ])
}

fn b2_knob() -> Knob {
    Knob::of(&[("length", Kv::I(25)), ("divergence", Kv::F(0.25))])
}

fn row(exp: &str, knob: Knob, sha: &str) -> Row {
    let mut r = Row::new(exp, knob, "certified", 1, 5);
    r.instance_sha256 = sha.to_string();
    r
}

// ---- 1. validate refuses an unregistered experiment --------------------------------------------

#[test]
fn an_out_of_plan_b5_row_is_refused_by_validate() {
    let mut f = committed();
    assert!(validate(&f).is_empty());
    let mut r = f.rows[0].clone();
    r.experiment = "B5".into();
    // B5 is registered since VOS-028c3c commit A; a fuse outside the plan's {1, 2, 4, 8} is not.
    r.knob = b5_knob(25, 0.25, 3);
    f.rows.push(r);
    let v = validate(&f);
    assert!(
        v.iter()
            .any(|m| m.contains("is not one of the plan's knobs")),
        "{v:?}"
    );
}

#[test]
fn a_row_named_zz_is_refused_by_validate() {
    let mut f = committed();
    let mut r = f.rows[0].clone();
    r.experiment = "ZZ".into();
    f.rows.push(r);
    let v = validate(&f);
    assert!(
        v.iter()
            .any(|m| m.contains("experiment \"ZZ\" is not one of the plan's experiments")),
        "{v:?}"
    );
}

// ---- 3. the fuse = 1 cross-rule ----------------------------------------------------------------

#[test]
fn an_empty_b5_digest_is_reported_even_against_an_empty_b2_twin() {
    let b2 = row("B2", b2_knob(), "");
    let b5 = row("B5", b5_knob(25, 0.25, 1), "");
    let v = b5_fuse1_violations(&[b2, b5]);
    assert!(
        v.iter().any(|m| m.contains("instance_sha256 is empty")),
        "{v:?}"
    );
}

#[test]
fn every_b2_twin_must_match_not_just_the_first() {
    let a = "aa".repeat(32);
    let good = row("B2", b2_knob(), &a);
    let evil = row("B2", b2_knob(), &"bb".repeat(32));
    let b5 = row("B5", b5_knob(25, 0.25, 1), &a);
    assert!(b5_fuse1_violations(&[good.clone(), b5.clone()]).is_empty());
    let v = b5_fuse1_violations(&[good, evil, b5]);
    assert_eq!(v.len(), 1, "{v:?}");
}

/// VOS-D138: a missing B2 twin is not a `b5_fuse1` finding (a quick-only file or one record shard
/// legitimately lacks it), but on a complete file removing the twin is still caught, by
/// `full_plan_covered`; and a PRESENT twin that differs is still a violation.
#[test]
fn a_missing_b2_twin_is_full_plan_coverage_s_finding_not_the_cross_rules() {
    use vos_capacity_probe::results::full_plan_missing;
    let a = "aa".repeat(32);
    let b5 = row("B5", b5_knob(25, 0.25, 1), &a);
    // (b) alone in the file: no b5_fuse1 finding.
    assert!(b5_fuse1_violations(std::slice::from_ref(&b5)).is_empty());
    // (a) a present twin with a different digest is still a violation.
    let evil = row("B2", b2_knob(), &"bb".repeat(32));
    assert_eq!(b5_fuse1_violations(&[evil, b5.clone()]).len(), 1);
    // (c) removing the twin from the real file is caught by full_plan_missing / full_plan_covered
    // even though b5_fuse1_violations stays silent.
    let mut f = committed();
    let at = f
        .rows
        .iter()
        .position(|r| {
            r.experiment == "B2"
                && r.tier == "full"
                && r.seed == 1
                && r.knob.int("length") == 25
                && (r.knob.f("divergence") - 0.25).abs() < 1e-9
        })
        .expect("a B2 length-25 divergence-0.25 seed-1 row");
    let before = full_plan_missing(&f).len();
    let mut twin_b5 = f.rows[at].clone();
    twin_b5.experiment = "B5".into();
    twin_b5.knob = b5_knob(25, 0.25, 1);
    f.rows.remove(at);
    f.rows.push(twin_b5);
    assert!(b5_fuse1_violations(&f.rows).is_empty());
    assert_eq!(full_plan_missing(&f).len(), before + 1);
    assert!(full_plan_missing(&f).iter().any(|m| m.contains("B2")));
}

// ---- 4. B5 knob bounds --------------------------------------------------------------------------

fn spec(len: i64, fuse: i64) -> RowSpec {
    RowSpec {
        experiment: "B5".into(),
        knob: b5_knob(len, 0.25, fuse),
        mode: "certified".into(),
        seed: 1,
        seeds: 5,
        tier: "quick".into(),
    }
}

#[test]
fn out_of_range_b5_knobs_are_errors_not_panics() {
    for (len, fuse) in [(1_000_000_000_000, 2), (-1, 2), (25, 0), (25, 1000)] {
        let s = spec(len, fuse);
        assert!(shape_for(&s).is_err(), "{len} {fuse}");
        let r = run_row(&s, Duration::from_secs(5), &mut |_| {});
        assert_eq!(r.result, "crashed", "{len} {fuse}");
    }
    let mut s = spec(25, 2);
    s.knob = b5_knob(25, 2.0, 2);
    assert!(shape_for(&s).is_err());
    assert!(shape_for(&spec(25, 2)).is_ok());
}
