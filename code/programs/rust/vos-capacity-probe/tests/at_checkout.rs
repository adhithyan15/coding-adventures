//! VOS-028c3c-3b-1a (VOS-D141): `check --full --at-checkout`, and the split of `full_refusals`.
//!
//! The behaviour under test, in one table:
//!
//! ```text
//!                                    check --full      check --full --at-checkout
//!   an input id differs              REFUSED           `note:` line, rows re-run
//!   no tree ids / legacy crate ids   REFUSED           REFUSED
//!   unreadable checkout              REFUSED           REFUSED
//!   uncommitted input                REFUSED           REFUSED
//!   a plan-coverage gap              REFUSED           REFUSED
//!   --at-checkout without --full     (usage error, exit 2)
//! ```
//!
//! Every refusal is shown with an executable that does not exist (`NO_SUCH_EXE`), which proves it
//! happened BEFORE any re-run.  The one test that proceeds re-runs five cheap in-process quick
//! rows (one A1 knob), as `verify_fanin.rs` does, and runs on every platform.

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;
use vos_capacity_probe::cli::{
    dispatch, full_refusals, merge_results, missing_tree_refusals, stale_refusals, Failure,
};
use vos_capacity_probe::exp::full_knob_list;
use vos_capacity_probe::meta;
use vos_capacity_probe::results::{
    identity_of, Merge, Regeneration, Results, LEGACY_TREE_NAMES, TREE_NAMES,
};

const NO_SUCH_EXE: &str = "/no/such/vos-capacity-probe-exe";

fn committed() -> Results {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/vos30-capacity-results.json");
    common::complete_plan(
        Results::from_canonical_text(&std::fs::read_to_string(p).unwrap()).unwrap(),
    )
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vos-probe-atco-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn write(dir: &std::path::Path, name: &str, res: &Results) -> String {
    let p = dir.join(name);
    std::fs::write(&p, res.to_text()).unwrap();
    p.to_string_lossy().into_owned()
}

fn run(args: &[&str]) -> (Result<(), Failure>, String, String) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let r = dispatch(&args, &mut out, &mut err);
    (
        r,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn ids(prefix: &str) -> Vec<(String, String)> {
    TREE_NAMES
        .iter()
        .enumerate()
        .map(|(i, n)| (n.to_string(), format!("{prefix}{:040x}", i + 1)))
        .collect()
}

/// The working tree's ids with the FIRST entry's id altered: a recording whose first input
/// differs from the checkout and whose other 45 equal it.
fn recorded_with_one_stale_input() -> Results {
    let mut f = committed();
    f.meta.merges.clear();
    f.meta.trees = meta::head_trees();
    assert_eq!(f.meta.trees.len(), TREE_NAMES.len(), "git can say");
    f.meta.trees[0].1 = format!("{:040x}", 0xbad);
    f
}

fn is_usage(r: &Result<(), Failure>, needle: &str) {
    match r {
        Err(Failure::Usage(m)) => assert!(m.contains(needle), "wanted {needle:?} in {m:?}"),
        other => panic!("wanted a usage error with {needle:?}, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------------------
// The flag
// ---------------------------------------------------------------------------------------

#[test]
fn at_checkout_is_valid_only_with_full() {
    // Refused before the file is even read.
    let (r, _, _) = run(&["check", "/no/such/results.json", "--at-checkout"]);
    is_usage(&r, "check --at-checkout needs --full");
    // And it is a known flag with --full (the unknown-flag refusal does not catch it).
    let (r, _, _) = run(&[
        "check",
        "/no/such/results.json",
        "--full",
        "--at-checkout",
        "--exe",
        NO_SUCH_EXE,
    ]);
    assert!(
        !matches!(&r, Err(Failure::Usage(m)) if m.contains("unknown flag")),
        "{r:?}"
    );
}

#[test]
fn a_stale_input_still_refuses_without_the_flag() {
    let dir = scratch("stale-no-flag");
    let p = write(&dir, "r.json", &recorded_with_one_stale_input());
    let (r, o, e) = run(&["check", &p, "--full", "--exe", NO_SUCH_EXE]);
    assert!(matches!(r, Err(Failure::Violations(n)) if n >= 1), "{r:?}");
    let name = TREE_NAMES[0];
    // Byte for byte what it printed before: the `stale:` line on stdout, the refusal on stderr.
    assert!(
        o.contains(&format!("stale: tool: {name} was recorded at tree ")),
        "{o}"
    );
    assert!(!o.contains("note:"), "{o}");
    assert!(
        e.contains(&format!(
            "check: check --full refuses (tool: {name} was recorded at tree "
        )) && e.contains("regenerate the rows"),
        "{e}"
    );
}

#[test]
fn a_stale_input_is_a_note_with_the_flag_and_the_rows_are_re_run() {
    let f = recorded_with_one_stale_input();
    if !meta::uncommitted_inputs(&meta::working_trees()).is_empty() {
        // A developer's uncommitted edit under an input makes a verify at the checkout refuse
        // (a verify runs at a COMMITTED tree); CI's tree is clean.
        eprintln!("skipped: the working tree has uncommitted changes under an input");
        return;
    }
    let dir = scratch("stale-flag");
    let src = write(&dir, "recorded.json", &f);
    let i = full_knob_list()
        .iter()
        .position(|(e, p)| *e == "A1" && p.quick)
        .expect("a quick A1 knob");
    let shard = format!("{i}/1024");
    let out = dir.join("verify.json");
    let (r, o, e) = run(&[
        "check",
        &src,
        "--full",
        "--at-checkout",
        "--shard",
        &shard,
        "--rerun-out",
        out.to_str().unwrap(),
        "--run-id",
        "7",
        "--run-attempt",
        "1",
        "--jobs",
        "2",
    ]);
    assert!(r.is_ok(), "{r:?}\n{o}\n{e}");
    let name = TREE_NAMES[0];
    assert!(
        o.contains(&format!("check: note: tool: {name} was recorded at tree ")),
        "{o}"
    );
    // The note replaces the `stale:` line, it is not printed beside it.
    assert!(!o.contains("stale:"), "{o}");
    // The ok tail names the count and the names of the differing inputs.
    assert!(
        o.contains(&format!(
            "at the checkout 1 input(s) differ from the recorded ones: {name}"
        )),
        "{o}"
    );
    assert!(o.contains("check: ok ("), "{o}");

    // The rows were re-run, and the verify file names the recording it verified, whose own
    // trees are untouched.
    let v = Results::from_canonical_text(&std::fs::read_to_string(&out).unwrap()).unwrap();
    let got: BTreeSet<_> = v.rows.iter().map(identity_of).collect();
    assert_eq!(got.len(), 5);
    assert_eq!(
        v.meta
            .recording
            .as_ref()
            .unwrap()
            .verifies_sha256
            .as_deref(),
        Some(f.unsigned_sha256().as_str())
    );
    assert_ne!(
        v.meta.trees, f.meta.trees,
        "the verify records ITS tree, not the recorded one"
    );
    assert_eq!(v.meta.trees, meta::head_trees());
}

#[test]
fn nothing_stale_says_so_with_the_flag() {
    if !meta::uncommitted_inputs(&meta::working_trees()).is_empty() {
        eprintln!("skipped: the working tree has uncommitted changes under an input");
        return;
    }
    let mut f = committed();
    f.meta.merges.clear();
    f.meta.trees = meta::head_trees();
    let dir = scratch("equal");
    let src = write(&dir, "recorded.json", &f);
    let i = full_knob_list()
        .iter()
        .position(|(e, p)| *e == "A1" && p.quick)
        .unwrap();
    let shard = format!("{i}/1024");
    let (r, o, e) = run(&[
        "check",
        &src,
        "--full",
        "--at-checkout",
        "--shard",
        &shard,
        "--jobs",
        "2",
    ]);
    assert!(r.is_ok(), "{r:?}\n{o}\n{e}");
    assert!(
        o.contains("at the checkout every input equals the recorded one"),
        "{o}"
    );
    assert!(!o.contains("note:"), "{o}");
}

// ---------------------------------------------------------------------------------------
// What the flag does NOT relax
// ---------------------------------------------------------------------------------------

#[test]
fn a_file_with_no_tree_ids_still_refuses_with_the_flag() {
    let dir = scratch("no-trees");
    let mut f = committed();
    f.meta.merges.clear();
    f.meta.trees.clear();
    let p = write(&dir, "r.json", &f);
    let (r, _, e) = run(&["check", &p, "--full", "--at-checkout", "--exe", NO_SUCH_EXE]);
    assert!(matches!(r, Err(Failure::Violations(n)) if n >= 1), "{r:?}");
    assert!(e.contains("records no tree ids"), "{e}");
}

#[test]
fn legacy_whole_crate_ids_still_refuse_with_the_flag() {
    let dir = scratch("legacy");
    // The legacy names are history that only a MERGE record may keep (the tool block must name
    // the entries, which the structural guard refuses before anything runs).  Build a file whose
    // tool block is current and whose one merge record carries them.
    let mut f = committed();
    f.meta.merges.clear();
    f.meta.trees = meta::head_trees();
    let mut fresh = f.clone();
    fresh.meta.trees = TREE_NAMES
        .iter()
        .map(|n| (n.to_string(), "b".repeat(40)))
        .collect();
    let mut merged = merge_results(
        &f,
        &fresh,
        "B3",
        None,
        true,
        "2026-11-19T00:00:00Z",
        &|_| Ok(true),
    )
    .unwrap();
    merged.meta.merges.last_mut().unwrap().trees = LEGACY_TREE_NAMES
        .iter()
        .map(|n| (n.to_string(), "a".repeat(40)))
        .collect();
    let p = write(&dir, "r.json", &merged);
    let (r, o, e) = run(&["check", &p, "--full", "--at-checkout", "--exe", NO_SUCH_EXE]);
    assert!(
        matches!(r, Err(Failure::Violations(n)) if n >= 1),
        "{r:?}
{o}
{e}"
    );
    assert!(e.contains("whole-crate tree ids"), "{e}");
    // Not a note: it is a refusal, and its line keeps the `stale:` form.
    assert!(
        o.contains("stale: merges[0]: recorded at whole-crate tree ids"),
        "{o}"
    );
    assert!(!o.contains("note: merges[0]"), "{o}");

    // In the tool block they are refused structurally, flag or no flag: exit 2, nothing re-run.
    let mut g = committed();
    g.meta.merges.clear();
    g.meta.trees = LEGACY_TREE_NAMES
        .iter()
        .map(|n| (n.to_string(), "a".repeat(40)))
        .collect();
    let p = write(&dir, "tool-legacy.json", &g);
    let (r, _, _) = run(&["check", &p, "--full", "--at-checkout", "--exe", NO_SUCH_EXE]);
    is_usage(&r, "nothing was re-run");
}

#[test]
fn a_plan_coverage_gap_still_refuses_with_the_flag() {
    let dir = scratch("gap");
    let mut f = recorded_with_one_stale_input();
    // Drop a whole knob: a recording that no longer covers the plan.
    let (e0, k0) = (f.rows[0].experiment.clone(), f.rows[0].knob.key());
    f.rows
        .retain(|r| !(r.experiment == e0 && r.knob.key() == k0));
    let p = write(&dir, "r.json", &f);
    let (r, _, e) = run(&["check", &p, "--full", "--at-checkout", "--exe", NO_SUCH_EXE]);
    assert!(r.is_err(), "{r:?}");
    assert!(
        e.contains("is not in the results file") || matches!(&r, Err(Failure::Usage(_))),
        "{r:?}\n{e}"
    );
}

// ---------------------------------------------------------------------------------------
// The split of full_refusals, as pure functions
// ---------------------------------------------------------------------------------------

fn merge_with(trees: Vec<(String, String)>) -> Merge {
    Merge {
        rev: "1".repeat(40),
        binary_sha256: "ab".repeat(32),
        dirty_tree: false,
        experiment: "B2".into(),
        where_: String::new(),
        rows: Vec::new(),
        at: "2026-01-01T00:00:00Z".into(),
        kind: None,
        trees,
    }
}

#[test]
fn full_refusals_is_missing_and_stale_in_the_historical_order() {
    let mut res = committed();
    let current = ids("");
    // The tool record: two inputs stale.  Merge 0: no tree ids at all.  Merge 1: one stale.
    res.meta.trees = current.clone();
    res.meta.trees[1].1 = "0".repeat(40);
    res.meta.trees[2].1 = "1".repeat(40);
    res.meta.merges = vec![
        merge_with(Vec::new()),
        merge_with({
            let mut t = current.clone();
            t[3].1 = "2".repeat(40);
            t
        }),
    ];
    let all = full_refusals(&res, &current);
    let missing = missing_tree_refusals(&res, &current);
    let stale = stale_refusals(&res, &current);
    assert_eq!(missing.len(), 1, "{missing:?}");
    assert!(
        missing[0].contains("merges[0] records no tree ids"),
        "{missing:?}"
    );
    assert_eq!(stale.len(), 3, "{stale:?}");
    assert!(stale[0].starts_with(&format!("tool: {} was recorded at tree ", TREE_NAMES[1])));
    assert!(stale[2].starts_with(&format!("merges[1]: {} was recorded", TREE_NAMES[3])));
    // The historical order: the tool's stale lines, then merge 0's message, then merge 1's line.
    assert_eq!(all.len(), 4, "{all:?}");
    assert!(all[0].contains(TREE_NAMES[1]) && all[0].starts_with("check --full refuses (tool:"));
    assert!(all[1].contains(TREE_NAMES[2]));
    assert!(all[2].contains("merges[0] records no tree ids"));
    assert!(all[3].contains("merges[1]"));
    // Together they are exactly the same findings: nothing is lost or invented by the split.
    let mut union: Vec<String> = missing.clone();
    union.extend(
        stale
            .iter()
            .map(|b| format!("check --full refuses ({b}); regenerate the rows")),
    );
    let (mut a, mut b) = (all.clone(), union);
    a.sort();
    b.sort();
    assert_eq!(a, b);
}

#[test]
fn an_equal_recording_has_no_refusal_of_either_kind() {
    let mut res = committed();
    res.meta.merges.clear();
    let current = ids("");
    res.meta.trees = current.clone();
    assert!(full_refusals(&res, &current).is_empty());
    assert!(missing_tree_refusals(&res, &current).is_empty());
    assert!(stale_refusals(&res, &current).is_empty());
}

#[test]
fn a_missing_record_is_never_a_stale_one() {
    let current = ids("");
    // No tool trees.
    let mut res = committed();
    res.meta.merges.clear();
    res.meta.trees.clear();
    assert_eq!(missing_tree_refusals(&res, &current).len(), 1);
    assert!(stale_refusals(&res, &current).is_empty());
    // Legacy whole-crate record: one line, and it is the fatal kind.
    res.meta.trees = LEGACY_TREE_NAMES
        .iter()
        .map(|n| (n.to_string(), "3".repeat(40)))
        .collect();
    assert_eq!(missing_tree_refusals(&res, &current).len(), 1);
    assert!(stale_refusals(&res, &current).is_empty());
    // A checkout whose ids git cannot read: also fatal, never a note.
    res.meta.trees = current.clone();
    let none: Vec<(String, String)> = Vec::new();
    assert_eq!(missing_tree_refusals(&res, &none).len(), 1);
    assert!(stale_refusals(&res, &none).is_empty());
}

#[test]
fn an_uncommitted_input_is_always_fatal_even_when_the_recording_is_otherwise_fine() {
    let mut res = committed();
    res.meta.merges.clear();
    let mut current = ids("");
    res.meta.trees = current.clone();
    assert!(missing_tree_refusals(&res, &current).is_empty());
    // Two inputs edited in the working tree: their ids carry `+uncommitted`.
    current[4].1.push_str("+uncommitted");
    current[9].1.push_str("+uncommitted");
    let m = missing_tree_refusals(&res, &current);
    assert_eq!(m.len(), 1, "{m:?}");
    assert!(m[0].contains("uncommitted changes in 2 input(s)"), "{m:?}");
    assert!(
        m[0].contains(TREE_NAMES[4]) && m[0].contains(TREE_NAMES[9]),
        "{m:?}"
    );
    // Without the flag the same checkout is refused too (the id differs from every recorded one),
    // as before: the stale list names them.
    let s = stale_refusals(&res, &current);
    assert_eq!(s.len(), 2, "{s:?}");
}

// ---------------------------------------------------------------------------------------
// verifies_sha256 is the digest without the banner
// ---------------------------------------------------------------------------------------

#[test]
fn the_unsigned_digest_is_the_same_before_and_after_a_banner() {
    let mut res = committed();
    res.meta.merges.clear();
    res.meta.regeneration = None;
    let before = res.unsigned_sha256();
    assert_eq!(before.len(), 64);
    // It is the digest of the canonical text, the very bytes `sign` hashes.
    assert_eq!(
        before,
        coding_adventures_sha256::sha256_hex(res.to_text().as_bytes())
    );
    let mut signed = res.clone();
    signed.meta.regeneration = Some(Regeneration {
        reviewer: "A Reviewer".into(),
        host: "host".into(),
        run_id: "1".into(),
        run_attempt: "1".into(),
        head_sha: "a".repeat(40),
        unsigned_sha256: before.clone(),
        verified: vos_capacity_probe::results::Verified {
            run_id: "2".into(),
            run_attempt: "1".into(),
            head_sha: "c".repeat(40),
            verify_sha256: "d".repeat(64),
            trees: Vec::new(),
        },
    });
    assert_ne!(
        coding_adventures_sha256::sha256_hex(signed.to_text().as_bytes()),
        before,
        "the banner does change the file's own bytes"
    );
    assert_eq!(
        signed.unsigned_sha256(),
        before,
        "and not the unsigned digest"
    );
    // A row edit does change it.
    let mut edited = signed.clone();
    edited.rows[0].conflicts += 1;
    assert_ne!(edited.unsigned_sha256(), before);
}
