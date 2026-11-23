//! VOS-028c3c-2a (VOS-D127 L3, VOS-D129, VOS-D130): `validate` told apart into structural findings
//! and row findings, and the `validate FILE` subcommand with its three exit statuses.
//!
//! The rule (VOS30 9.12.1): a structural finding discards a recording, a row finding (a failed
//! control, say) keeps it and blocks the commit, a clean file is committable.  Everything here is
//! pure or reads a temp file; nothing starts a child process, so it runs on every platform.

use std::path::PathBuf;
use vos_capacity_probe::cli::{dispatch, Failure};
use vos_capacity_probe::results::{validate, validate_classes, Results};

fn committed() -> Results {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/vos30-capacity-results.json");
    Results::from_text(&std::fs::read_to_string(p).unwrap()).unwrap()
}

/// The index of the first row with a replayed SAT control (an `unsat` row outside `enum`).
fn controlled_row(res: &Results) -> usize {
    res.rows
        .iter()
        .position(|r| r.mode != "enum" && r.result == "unsat")
        .unwrap()
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vos-probe-validate-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Run `validate` on `res` through the real dispatcher, returning the failure (if any) and stdout.
fn run_validate(res: &Results, name: &str) -> (Result<(), Failure>, String) {
    let dir = scratch(name);
    let f = dir.join("r.json");
    std::fs::write(&f, res.to_text()).unwrap();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let r = dispatch(
        &["validate".to_string(), f.to_string_lossy().into_owned()],
        &mut out,
        &mut err,
    );
    (r, String::from_utf8(out).unwrap())
}

#[test]
fn the_committed_file_is_clean() {
    let res = committed();
    let v = validate_classes(&res);
    assert!(v.structural.is_empty() && v.rows.is_empty(), "{v:?}");
    assert!(validate(&res).is_empty());
}

#[test]
fn a_failed_control_is_a_row_finding_not_structural() {
    let mut res = committed();
    let i = controlled_row(&res);
    // The control did not end SAT: the row keeps `unsat` (that is what `mono_row` writes).
    res.rows[i].control.result = "unsat".into();
    res.rows[i].control.replayed = false;
    let v = validate_classes(&res);
    assert!(v.structural.is_empty(), "{:?}", v.structural);
    assert!(
        v.rows
            .iter()
            .any(|m| m.contains("an UNSAT row has no replayed SAT control")),
        "{:?}",
        v.rows
    );
    // A red `green` and a missing RSS are row findings too.
    let mut res = committed();
    res.rows[0].green = !res.rows[0].green;
    res.rows[1].peak_rss_kb = 0;
    let v = validate_classes(&res);
    assert!(v.structural.is_empty(), "{:?}", v.structural);
    assert!(v.rows.len() >= 2, "{:?}", v.rows);
}

#[test]
fn validate_exits_2_on_structure_and_1_on_rows() {
    // Clean: exit 0 and a line on stdout.
    let (r, out) = run_validate(&committed(), "clean");
    assert!(r.is_ok());
    assert!(out.starts_with("validate: ok ("), "{out}");

    // Row findings only: exit 1 (`Failure::Violations`).
    let mut res = committed();
    let i = controlled_row(&res);
    res.rows[i].control.result = "unsat".into();
    let (r, _) = run_validate(&res, "rows");
    assert!(matches!(r, Err(Failure::Violations(n)) if n >= 1), "{r:?}");

    // A structural finding: exit 2 (`Failure::Usage`), even when row findings are present too.
    let mut res = committed();
    res.meta.dirty_tree = true;
    let (r, _) = run_validate(&res, "structural");
    assert!(matches!(r, Err(Failure::Usage(_))), "{r:?}");
    let i = controlled_row(&res);
    res.rows[i].control.result = "unsat".into();
    let (r, _) = run_validate(&res, "both");
    assert!(matches!(r, Err(Failure::Usage(_))), "{r:?}");

    // A tier outside the closed set is structural on its own.
    let mut res = committed();
    res.rows[0].tier = "bogus".into();
    let (r, _) = run_validate(&res, "tier");
    assert!(matches!(r, Err(Failure::Usage(_))), "{r:?}");

    // An unreadable or non-canonical file, and a missing argument, are exit 2.
    let dir = scratch("junk");
    let f = dir.join("junk.json");
    std::fs::write(&f, "{\"schema\": 2").unwrap();
    let (mut o, mut e) = (Vec::new(), Vec::new());
    let r = dispatch(
        &["validate".into(), f.to_string_lossy().into_owned()],
        &mut o,
        &mut e,
    );
    assert!(matches!(r, Err(Failure::Usage(_))), "{r:?}");
    let r = dispatch(&["validate".into()], &mut o, &mut e);
    assert!(matches!(r, Err(Failure::Usage(_))), "{r:?}");
    let r = dispatch(
        &[
            "validate".into(),
            dir.join("missing.json").to_string_lossy().into_owned(),
        ],
        &mut o,
        &mut e,
    );
    assert!(matches!(r, Err(Failure::Usage(_))), "{r:?}");
}

#[test]
fn validate_is_the_union_in_the_old_order() {
    // Findings of both classes, interleaved the way the old single pass found them: the file's
    // dirty tree (structural, first), then row 0's failed control (row), then row 1's tier
    // (structural, found AFTER a row finding).  `validate` must keep exactly that order, so
    // `check` and `merge` print what they always printed; the classes are a partition of it.
    let mut res = committed();
    res.meta.dirty_tree = true;
    let i = controlled_row(&res);
    let j = res.rows.iter().position(|r| r.experiment != "E1").unwrap();
    let j = if j == i { j + 1 } else { j };
    assert!(i < j || j < i);
    let (first, second) = (i.min(j), i.max(j));
    // The earlier row takes the row finding, the later row the structural one.
    res.rows[first].green = !res.rows[first].green;
    res.rows[second].tier = "bogus".into();

    let all = validate(&res);
    let v = validate_classes(&res);
    assert!(all[0].starts_with("dirty_tree is true"), "{all:?}");
    assert_eq!(all.len(), v.structural.len() + v.rows.len());
    // Partition: every finding is in exactly one class, and each class keeps its relative order.
    let pick = |class: &[String]| -> Vec<String> {
        all.iter().filter(|m| class.contains(m)).cloned().collect()
    };
    assert_eq!(pick(&v.structural), v.structural);
    assert_eq!(pick(&v.rows), v.rows);
    // The interleaving is the old one: the row finding of the earlier row precedes the
    // structural finding of the later row (a "structural first" concatenation would reverse them).
    let pos = |needle: &str| all.iter().position(|m| m.contains(needle)).unwrap();
    assert!(pos("green is") < pos("tier \"bogus\""), "{all:?}");
    assert!(v.structural.iter().any(|m| m.contains("tier")));
    assert!(v.rows.iter().any(|m| m.contains("green is")));
}

#[test]
fn a_bad_seed_range_is_structural() {
    // Seed 0 and seed above the seed count are identity findings: the row is not a row of the
    // plan.  They are structural (exit 2), not row findings (exit 1).
    for (name, bad_seed) in [("seed0", Some(0u64)), ("seedhi", None)] {
        let mut res = committed();
        let seeds = u64::from(res.rows[0].seeds);
        res.rows[0].seed = bad_seed.unwrap_or(seeds + 1);
        let v = validate_classes(&res);
        assert!(
            v.structural.iter().any(|m| m.contains("is not in 1..=")),
            "{name}: {v:?}"
        );
        assert!(
            !v.rows.iter().any(|m| m.contains("is not in 1..=")),
            "{name}: {v:?}"
        );
        let (r, _) = run_validate(&res, name);
        assert!(matches!(r, Err(Failure::Usage(_))), "{name}: {r:?}");
    }
    // An over-large seed count is structural too.
    let mut res = committed();
    res.rows[0].seeds = 1001;
    let v = validate_classes(&res);
    assert!(
        v.structural
            .iter()
            .any(|m| m.contains("seeds 1001 is over")),
        "{v:?}"
    );
}

#[test]
fn validate_takes_exactly_one_file() {
    let dir = scratch("arity");
    let f = dir.join("r.json");
    std::fs::write(&f, committed().to_text()).unwrap();
    let a = f.to_string_lossy().into_owned();
    let (mut o, mut e) = (Vec::new(), Vec::new());
    let r = dispatch(&["validate".into(), a.clone(), a], &mut o, &mut e);
    assert!(
        matches!(&r, Err(Failure::Usage(m)) if m == "validate takes exactly one results file"),
        "{r:?}"
    );
}
