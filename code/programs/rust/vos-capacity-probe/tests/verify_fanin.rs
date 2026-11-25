//! VOS-028c3c-2b-ii: the verify fan-in (`check --full --shard`, `--rerun-out`, `join` of verify
//! shards, `decision-diff`), VOS-D132 (the re-plan decision and `K` in `decision_differences`) and
//! VOS-D134 (the guards that run before any re-run).
//!
//! Everything here runs on every platform: the only re-run is one in-process quick row, and every
//! refusal is shown with an executable that does not exist, which proves it happened BEFORE any
//! re-run (a re-run would have had to spawn it, or would have taken a visible time).

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;
use vos_capacity_probe::cli::{dispatch, Failure};
use vos_capacity_probe::exp::{full_knob_list, plan, plan_shard, Tier};
use vos_capacity_probe::meta;
use vos_capacity_probe::results::{
    self, check_guards, cut_density_k, decide, decide_replan, decision_differences, green,
    identity_of, quick_row_cap, quick_specs, quick_specs_in, rerun_collect_in, shard_identities,
    Recording, Results, MIN_SEEDS, TREE_NAMES,
};
use vos_capacity_probe::row::Row;

const NO_SUCH_EXE: &str = "/no/such/vos-capacity-probe-exe";

fn committed_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/vos30-capacity-results.json")
}

/// The committed file completed with fixture rows for the plan rows it lacks (see
/// `common::complete_plan`; VOS-028c3c commit A).
fn committed() -> Results {
    common::complete_plan(
        Results::from_canonical_text(&std::fs::read_to_string(committed_path()).unwrap()).unwrap(),
    )
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vos-probe-fanin-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
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

fn write(dir: &std::path::Path, name: &str, res: &Results) -> String {
    let p = dir.join(name);
    std::fs::write(&p, res.to_text()).unwrap();
    p.to_string_lossy().into_owned()
}

fn is_usage<T: std::fmt::Debug>(r: &Result<T, Failure>, needle: &str) {
    match r {
        Err(Failure::Usage(m)) => assert!(m.contains(needle), "wanted {needle:?} in {m:?}"),
        other => panic!("wanted a usage error with {needle:?}, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------------------
// Shard filtering
// ---------------------------------------------------------------------------------------

#[test]
fn the_shards_select_every_quick_row_exactly_once() {
    let res = committed();
    let all: BTreeSet<_> = quick_specs(&res)
        .iter()
        .map(|(_, r)| identity_of(r))
        .collect();
    assert_eq!(all.len(), 105);
    for n in [1u32, 7, 64] {
        let mut seen: Vec<results::Identity> = Vec::new();
        for k in 0..n {
            let ids = shard_identities(k, n);
            for (spec, r) in quick_specs_in(&res, Some(&ids)) {
                assert_eq!(spec.experiment, r.experiment);
                assert!(ids.contains(&identity_of(r)), "a row outside shard {k}/{n}");
                seen.push(identity_of(r));
            }
        }
        let got: BTreeSet<_> = seen.iter().cloned().collect();
        assert_eq!(seen.len(), 105, "N = {n}: each quick row once");
        assert_eq!(got, all, "N = {n}: the union is the quick tier");
    }
}

#[test]
fn a_shard_identity_set_is_the_plan_slice() {
    for (k, n) in [(0u32, 1u32), (3, 7), (63, 64)] {
        let want: BTreeSet<_> = plan_shard(MIN_SEEDS, k, n)
            .into_iter()
            .map(|s| (s.experiment, s.mode, s.knob.key(), s.seed))
            .collect();
        assert_eq!(shard_identities(k, n), want);
    }
}

#[test]
fn a_shard_with_no_quick_row_is_not_a_finding_but_an_empty_file_is() {
    let res = committed();
    // N = 1024 > 445 knobs: shard 1000 holds nothing.
    let none = shard_identities(1000, 1024);
    assert!(none.is_empty());
    let (found, rows) = rerun_collect_in(&res, 2, true, Some(&none));
    assert!(found.is_empty() && rows.is_empty(), "{found:?}");
    // The whole-file run on a file with no quick row still says so.
    let mut f = res;
    f.rows.retain(|r| r.tier != "quick");
    let (found, _) = rerun_collect_in(&f, 2, true, None);
    assert_eq!(found, vec!["the file has no quick-tier row to re-run"]);
}

// ---------------------------------------------------------------------------------------
// Flags: refused before the file is even read
// ---------------------------------------------------------------------------------------

#[test]
fn the_shard_flags_are_refused_before_anything_is_read() {
    let f = "/no/such/results.json";
    let cases: Vec<(Vec<&str>, &str)> = vec![
        (vec!["check", f, "--shard", "0/4"], "--shard needs --full"),
        (vec!["check", f, "--full", "--shard", "4/4"], "shard"),
        (
            vec!["check", f, "--full", "--shard", "0/4", "--shard", "1/4"],
            "more than once",
        ),
        (
            vec!["check", f, "--full", "--rerun-out", "x.json"],
            "need --shard",
        ),
        (
            vec!["check", f, "--full", "--shard", "0/4", "--run-id", "1"],
            "go together",
        ),
        (
            vec![
                "check",
                f,
                "--full",
                "--shard",
                "0/4",
                "--run-id",
                "1",
                "--run-attempt",
                "1",
            ],
            "need it",
        ),
        (
            vec!["check", f, "--full", "--shard", "0/4", "--rerun-out"],
            "needs a value",
        ),
    ];
    for (args, needle) in cases {
        let (r, _, _) = run(&args);
        is_usage(&r, needle);
    }
}

#[test]
fn a_rerun_out_that_exists_is_refused_before_any_re_run() {
    let dir = scratch("exists");
    let src = write(&dir, "r.json", &committed());
    let out = dir.join("v.json");
    std::fs::write(&out, "keep me").unwrap();
    let (r, _, _) = run(&[
        "check",
        &src,
        "--full",
        "--shard",
        "0/64",
        "--rerun-out",
        out.to_str().unwrap(),
        "--exe",
        NO_SUCH_EXE,
    ]);
    is_usage(&r, "already exists");
    assert_eq!(std::fs::read_to_string(&out).unwrap(), "keep me");
}

// ---------------------------------------------------------------------------------------
// VOS-D134: the guards, each refused before any re-run (an executable that does not exist)
// ---------------------------------------------------------------------------------------

#[test]
fn the_committed_file_passes_the_guards_and_the_cap_is_the_plan() {
    assert!(check_guards(&committed()).is_empty());
    assert_eq!(quick_row_cap(), 105);
    assert_eq!(quick_row_cap(), plan(Tier::Quick, &[], MIN_SEEDS).len());
}

#[test]
fn a_structural_finding_stops_every_check_mode_with_exit_2_and_no_re_run() {
    let dir = scratch("guard-a");
    let mut f = committed();
    f.meta.dirty_tree = true;
    let p = write(&dir, "dirty.json", &f);
    assert!(!check_guards(&f).is_empty());
    for extra in [&[][..], &["--full"][..], &["--full", "--shard", "0/64"][..]] {
        let mut args = vec!["check", p.as_str(), "--exe", NO_SUCH_EXE];
        args.extend_from_slice(extra);
        let (r, _, e) = run(&args);
        is_usage(&r, "nothing was re-run");
        assert!(e.contains("structural"), "{e}");
    }
}

#[test]
fn a_duplicate_row_identity_is_structural_and_stops_before_any_re_run() {
    let dir = scratch("guard-bc");
    let mut f = committed();
    let q = f.rows.iter().find(|r| r.tier == "quick").unwrap().clone();
    f.rows.push(q);
    let g = check_guards(&f);
    // (b): the repeated identity.  (c): the quick specs now outnumber the plan's quick rows.
    assert!(
        g.iter().any(|m| m.contains("appears more than once")),
        "{g:?}"
    );
    assert!(
        g.iter()
            .any(|m| m.contains("quick-tier rows to re-run, but the plan has 105")),
        "{g:?}"
    );
    let p = write(&dir, "dup.json", &f);
    for extra in [&[][..], &["--full"][..]] {
        let mut args = vec!["check", p.as_str(), "--exe", NO_SUCH_EXE];
        args.extend_from_slice(extra);
        let (r, _, e) = run(&args);
        is_usage(&r, "nothing was re-run");
        assert!(e.contains("appears more than once"), "{e}");
        assert!(e.contains("but the plan has 105"), "{e}");
    }
}

// ---------------------------------------------------------------------------------------
// The shard re-run, end to end, on one in-process quick row
// ---------------------------------------------------------------------------------------

/// The shard (K of 1024: one knob each) of the first quick A1 knob: five cheap in-process rows.
fn one_quick_shard() -> (u32, u32) {
    let i = full_knob_list()
        .iter()
        .position(|(e, p)| *e == "A1" && p.quick)
        .expect("a quick A1 knob");
    (u32::try_from(i).unwrap(), 1024)
}

#[test]
fn check_full_shard_reruns_only_its_rows_and_writes_a_verify_file() {
    let (k, n) = one_quick_shard();
    let ids = shard_identities(k, n);
    assert_eq!(ids.len() as u32, MIN_SEEDS);
    let dir = scratch("e2e");
    let mut f = committed();
    // `check --full` refuses a file that does not describe this tree; describe it, as the
    // regeneration the refusal asks for would.
    f.meta.merges.clear();
    f.meta.trees = meta::head_trees();
    if meta::working_trees() != f.meta.trees {
        // A developer's uncommitted edit under a recorded tree makes `check --full` refuse the
        // file (a `+uncommitted` id is not one a file can record); CI's tree is clean.
        eprintln!("skipped: the working tree has uncommitted changes under a recorded tree");
        return;
    }
    let src = write(&dir, "recorded.json", &f);
    let out = dir.join("verify.json");
    let shard = format!("{k}/{n}");
    let args = [
        "check",
        &src,
        "--full",
        "--shard",
        &shard,
        "--rerun-out",
        out.to_str().unwrap(),
        "--run-id",
        "42",
        "--run-attempt",
        "3",
        "--jobs",
        "2",
    ];
    let (r, o, e) = run(&args);
    assert!(r.is_ok(), "{r:?}\n{o}\n{e}");
    assert!(o.contains(&format!("shard {k}/{n}: 5 of ")), "{o}");

    let v = Results::from_canonical_text(&std::fs::read_to_string(&out).unwrap()).unwrap();
    let rec = v.meta.recording.as_ref().expect("a recording block");
    assert_eq!(rec.mode, "verify");
    assert_eq!(rec.shard, Some((k, n)));
    assert_eq!(rec.shards, None);
    assert_eq!(rec.jobs, 2);
    assert_eq!(rec.run_id.as_deref(), Some("42"));
    assert_eq!(rec.run_attempt.as_deref(), Some("3"));
    let got: BTreeSet<_> = v.rows.iter().map(identity_of).collect();
    assert_eq!(got, ids, "exactly the shard's rows");
    assert_eq!(
        v.meta.binary_sha256.len(),
        64,
        "the verifying binary's own digest"
    );
    assert!(v.meta.merges.is_empty() && v.meta.regeneration.is_none());

    // The file is never overwritten.
    let (r, _, _) = run(&args);
    is_usage(&r, "already exists");
    // The shard file `check` wrote is one shard, not the fan-in: `decision-diff` refuses it.
    let (r, _, _) = run(&[
        "decision-diff",
        "--recorded",
        &src,
        "--rerun",
        out.to_str().unwrap(),
    ]);
    is_usage(&r, "one verify shard");
}

#[test]
fn an_empty_slice_is_refused_before_anything_is_read_or_re_run() {
    // N = 1024 > the knob count: shard 1000 holds nothing, which would be an empty "verification".
    let (r, _, _) = run(&[
        "check",
        "/no/such/results.json",
        "--full",
        "--shard",
        "1000/1024",
    ]);
    is_usage(&r, "holds no row of the plan");
}

#[test]
fn rerun_out_needs_the_running_binary_as_exe() {
    let dir = scratch("exe");
    let src = write(&dir, "r.json", &committed());
    let out = dir.join("v.json");
    // Any other existing file (here, the recorded file itself) cannot stand behind the digest.
    let (r, _, _) = run(&[
        "check",
        &src,
        "--full",
        "--shard",
        "0/64",
        "--rerun-out",
        out.to_str().unwrap(),
        "--exe",
        &src,
    ]);
    is_usage(&r, "must be this executable");
    assert!(!out.exists());
    // A path that does not exist is refused the same way.
    let (r, _, _) = run(&[
        "check",
        &src,
        "--full",
        "--shard",
        "0/64",
        "--rerun-out",
        out.to_str().unwrap(),
        "--exe",
        NO_SUCH_EXE,
    ]);
    is_usage(&r, "must be this executable");
}

#[test]
fn check_refuses_a_mistyped_flag() {
    let (r, _, _) = run(&[
        "check",
        "/no/such/results.json",
        "--full",
        "--rerun-ot",
        "x",
    ]);
    is_usage(&r, "unknown flag --rerun-ot");
    let (r, _, _) = run(&["check", "/no/such/results.json", "--bogus"]);
    is_usage(&r, "unknown flag --bogus");
}

#[test]
fn write_new_never_replaces_and_leaves_no_temporary_file() {
    let dir = scratch("write-new");
    let target = dir.join("evidence.json");
    // The target appears AFTER any early `exists()` check and BEFORE the publish: refused.
    std::fs::write(&target, "first").unwrap();
    let r = vos_capacity_probe::cli::write_new(&target, "second");
    is_usage(&r, "already exists");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "first");
    let left: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left.len(), 1, "no temporary file is left behind: {left:?}");
    // An absent target is published.
    let fresh = dir.join("fresh.json");
    vos_capacity_probe::cli::write_new(&fresh, "hello").unwrap();
    assert_eq!(std::fs::read_to_string(&fresh).unwrap(), "hello");
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
}

// ---------------------------------------------------------------------------------------
// join accepts verify shards; decision-diff
// ---------------------------------------------------------------------------------------

fn trees() -> Vec<(String, String)> {
    TREE_NAMES
        .iter()
        .enumerate()
        .map(|(i, n)| (n.to_string(), format!("{:040x}", i + 1)))
        .collect()
}

/// The committed rows cut into `n` verify-mode shard files.
fn verify_shards(n: u32) -> Vec<(String, Results)> {
    // What every shard of a verification of `recorded_with_trees()` names (VOS-D141).
    let verifies = recorded_with_trees().unsigned_sha256();
    (0..n)
        .map(|k| {
            let ids = shard_identities(k, n);
            let mut r = committed();
            r.meta.merges.clear();
            r.meta.regeneration = None;
            r.meta.trees = trees();
            r.rows.retain(|row| ids.contains(&identity_of(row)));
            r.meta.recording = Some(Recording {
                mode: "verify".into(),
                run_id: Some("99".into()),
                run_attempt: Some("1".into()),
                shard: Some((k, n)),
                shards: None,
                jobs: 2,
                verifies_sha256: Some(verifies.clone()),
            });
            (format!("v{k}.json"), r)
        })
        .collect()
}

/// The committed rows as a recording that names its trees (today's committed file records none,
/// and `decision-diff` refuses a pair that does not say which tree it describes).
fn recorded_with_trees() -> Results {
    let mut r = committed();
    r.meta.merges.clear();
    r.meta.regeneration = None;
    r.meta.trees = trees();
    r
}

/// `rec` as the JOINED verify file `join` would build from `n` shards of a verification.
fn joined_of(rec: &Results, n: u32) -> Results {
    let mut r = rec.clone();
    let verifies = rec.unsigned_sha256();
    r.meta.recording = Some(Recording {
        mode: "verify".into(),
        run_id: Some("99".into()),
        run_attempt: Some("1".into()),
        shard: None,
        shards: Some(n),
        jobs: 2,
        verifies_sha256: Some(verifies),
    });
    r
}

/// Fan `verify_shards(5)` in through the real `join`; returns `(dir, recorded path, joined path)`.
fn fan_in(name: &str) -> (PathBuf, String, String) {
    let dir = scratch(name);
    let files: Vec<String> = verify_shards(5)
        .into_iter()
        .map(|(name, r)| write(&dir, &name, &r))
        .collect();
    let joined = dir.join("joined.json").to_string_lossy().into_owned();
    let mut args = vec![
        "join",
        "--out",
        joined.as_str(),
        "--expect-run-id",
        "99",
        "--expect-shards",
        "5",
    ];
    args.extend(files.iter().map(String::as_str));
    let (r, o, e) = run(&args);
    assert!(r.is_ok(), "{r:?}\n{o}\n{e}");
    let recorded = write(&dir, "recorded.json", &recorded_with_trees());
    (dir, recorded, joined)
}

#[test]
fn join_fans_verify_shards_in_and_decision_diff_agrees_with_the_recording() {
    let (dir, recorded, j) = fan_in("fanin");
    let back = Results::from_canonical_text(&std::fs::read_to_string(&j).unwrap()).unwrap();
    let rc = back.meta.recording.as_ref().unwrap();
    assert_eq!((rc.mode.as_str(), rc.shards), ("verify", Some(5)));

    // join refuses to overwrite the fan-in file, as it does any output.
    let shard0 = dir.join("v0.json");
    let (r, _, _) = run(&["join", "--out", &j, &shard0.to_string_lossy()]);
    is_usage(&r, "already exists");

    // The joined re-run rows are the recorded rows: no difference, exit 0; with the shard count
    // pinned too.
    let (r, o, e) = run(&["decision-diff", "--recorded", &recorded, "--rerun", &j]);
    assert!(r.is_ok(), "{r:?}\n{o}\n{e}");
    assert!(o.starts_with("decision-diff: ok ("), "{o}");
    let (r, _, _) = run(&[
        "decision-diff",
        "--recorded",
        &recorded,
        "--rerun",
        &j,
        "--expect-shards",
        "5",
    ]);
    assert!(r.is_ok(), "{r:?}");
    let (r, _, _) = run(&[
        "decision-diff",
        "--recorded",
        &recorded,
        "--rerun",
        &j,
        "--expect-shards",
        "6",
    ]);
    is_usage(&r, "--expect-shards is 6");
    let (r, _, _) = run(&[
        "decision-diff",
        "--recorded",
        &recorded,
        "--rerun",
        &j,
        "--expect-shards",
        "x",
    ]);
    is_usage(&r, "small decimal integer");
}

#[test]
fn decision_diff_exits_0_equal_1_different_and_2_on_other_rows() {
    let dir = scratch("dd");
    let rec = recorded_with_trees();
    let recorded = write(&dir, "rec.json", &rec);
    let joined = joined_of(&rec, 5);
    let same = write(&dir, "same.json", &joined);
    // An authentic re-run of equal rows: exit 0.
    let (r, _, _) = run(&["decision-diff", "--recorded", &recorded, "--rerun", &same]);
    assert!(r.is_ok(), "{r:?}");

    // An edited decision-driving field: exit 1, the difference printed.
    let mut edited = joined.clone();
    for r in edited.rows.iter_mut().filter(|r| r.experiment == "D3") {
        r.worst_step_conflicts = r.worst_step_conflicts.map(|c| c + 1_000_000);
    }
    let p = write(&dir, "edited.json", &edited);
    let (r, _, e) = run(&["decision-diff", "--recorded", &recorded, "--rerun", &p]);
    assert!(matches!(r, Err(Failure::Violations(n)) if n >= 1), "{r:?}");
    assert!(e.contains("decision-diff:"), "{e}");

    // Different identity sets: exit 2 (it is not a re-run of the recording).
    // (A whole knob's seeds: one missing seed is itself a structural finding.)
    let mut fewer = joined.clone();
    let (e0, k0) = (rec.rows[0].experiment.clone(), rec.rows[0].knob.key());
    fewer
        .rows
        .retain(|r| !(r.experiment == e0 && r.knob.key() == k0));
    let p = write(&dir, "fewer.json", &fewer);
    let (r, _, _) = run(&["decision-diff", "--recorded", &recorded, "--rerun", &p]);
    is_usage(&r, "different rows");
    let mut more = joined.clone();
    more.rows.push(rec.rows[0].clone());
    let p = write(&dir, "more.json", &more);
    let (r, _, _) = run(&["decision-diff", "--recorded", &recorded, "--rerun", &p]);
    // The repeated identity is a structural finding of the file, which is refused first.
    is_usage(&r, "structural finding");

    // Usage: a missing flag, a repeated flag, an unknown flag, a stray positional.
    for args in [
        vec!["decision-diff", "--recorded", recorded.as_str()],
        vec!["decision-diff", "--rerun", recorded.as_str()],
        vec![
            "decision-diff",
            "--recorded",
            recorded.as_str(),
            "--recorded",
            recorded.as_str(),
            "--rerun",
            same.as_str(),
        ],
        vec![
            "decision-diff",
            "--recorded",
            &recorded,
            "--rerun",
            &same,
            "--bogus",
        ],
        vec![
            "decision-diff",
            "--recorded",
            &recorded,
            "--rerun",
            &same,
            "stray",
        ],
    ] {
        let (r, _, _) = run(&args);
        assert!(matches!(r, Err(Failure::Usage(_))), "{args:?}: {r:?}");
    }
}

/// The security review's MEDIUM: `decision-diff` of a file against ITSELF (or against any file
/// that merely holds the same rows) used to exit 0 and read as a verification.
#[test]
fn decision_diff_authenticates_the_rerun_before_comparing() {
    let dir = scratch("dd-auth");
    let rec = recorded_with_trees();
    let recorded = write(&dir, "rec.json", &rec);
    let good = joined_of(&rec, 5);
    let dd = |recorded: &str, rerun: &str| {
        run(&["decision-diff", "--recorded", recorded, "--rerun", rerun])
    };

    // The file against itself, and today's committed file (no recording, no trees) against itself.
    let (r, _, _) = dd(&recorded, &recorded);
    is_usage(&r, "not a verify file");
    let committed_file = committed_path().to_string_lossy().into_owned();
    let (r, _, _) = dd(&committed_file, &committed_file);
    is_usage(&r, "not a verify file");

    // A mode `record` file, joined or not, is not a verification either.
    let mut as_record = good.clone();
    as_record.meta.recording.as_mut().unwrap().mode = "record".into();
    // (A record-mode file cannot carry `verifies_sha256`; it is refused when parsed.)
    as_record.meta.recording.as_mut().unwrap().verifies_sha256 = None;
    let p = write(&dir, "as-record.json", &as_record);
    let (r, _, _) = dd(&recorded, &p);
    is_usage(&r, "not a verify file");

    // One verify shard is not the fan-in.
    let one = write(&dir, "one.json", &verify_shards(5).remove(0).1);
    let (r, _, _) = dd(&recorded, &one);
    is_usage(&r, "one verify shard");

    // The recording must not itself be a verify file.
    let verify_as_recorded = write(&dir, "good.json", &good);
    let (r, _, _) = dd(&verify_as_recorded, &verify_as_recorded);
    is_usage(&r, "itself a verify file");

    // Tree ids: a verify at ANOTHER tree is no longer refused (VOS-D141; it is what a verify at
    // the checkout is), but a file that records none, on either side, still is.
    let mut other_trees = good.clone();
    other_trees.meta.trees[0].1 = format!("{:040x}", 0xdead);
    let p = write(&dir, "other-trees.json", &other_trees);
    let (r, o, e) = dd(&recorded, &p);
    assert!(
        r.is_ok(),
        "{r:?}
{o}
{e}"
    );
    let mut no_trees = good.clone();
    no_trees.meta.trees.clear();
    let p = write(&dir, "no-trees.json", &no_trees);
    let (r, _, _) = dd(&recorded, &p);
    is_usage(&r, "no tree ids");
    let mut rec_no_trees = rec.clone();
    rec_no_trees.meta.trees.clear();
    let p = write(&dir, "rec-no-trees.json", &rec_no_trees);
    let good_path = write(&dir, "good2.json", &good);
    let (r, _, _) = dd(&p, &good_path);
    is_usage(&r, "no tree ids");

    // A structural finding on either side stops the comparison.
    let mut dirty = good.clone();
    dirty.meta.dirty_tree = true;
    let p = write(&dir, "dirty.json", &dirty);
    let (r, _, _) = dd(&recorded, &p);
    is_usage(&r, "--rerun has");
    let mut dirty_rec = rec.clone();
    dirty_rec.meta.dirty_tree = true;
    let p = write(&dir, "dirty-rec.json", &dirty_rec);
    let (r, _, _) = dd(&p, &good_path);
    is_usage(&r, "--recorded has");

    // And the authentic pair passes.
    let (r, _, _) = dd(&recorded, &good_path);
    assert!(r.is_ok(), "{r:?}");
}

// ---------------------------------------------------------------------------------------
// VOS-D132: decision_differences sees the re-plan decision and K
// ---------------------------------------------------------------------------------------

/// Push the rows matching `pick` over the route-green conflict bound (50,000) but under the
/// green one (100,000), so `green` (what `decide` reads) is unchanged and `route_green` (what
/// `decide_replan` and `cut_density_k` read) is not.
fn over_route_bound(rows: &[Row], pick: impl Fn(&Row) -> bool) -> Vec<Row> {
    let mut v = rows.to_vec();
    let mut n = 0;
    for r in v.iter_mut().filter(|r| pick(r)) {
        r.conflicts = 60_000;
        r.worst_step_conflicts = Some(60_000);
        assert!(green(r), "the edit must leave the row green");
        n += 1;
    }
    assert!(n > 0, "the pick matched no row");
    v
}

#[test]
fn the_committed_rows_have_no_decision_difference_against_themselves() {
    let rows = committed().rows;
    assert!(decision_differences(&rows, &rows).is_empty());
}

#[test]
fn a_replan_only_change_is_a_difference() {
    let rows = committed().rows;
    let changed = over_route_bound(&rows, |r| {
        r.experiment == "A1" && r.mode == "monolithic" && r.knob.int("n") == 8
    });
    // Precondition: the section 8.3 decision and K do not see it, so before VOS-D132 this
    // verification passed.
    let (a, b) = (decide(&rows), decide(&changed));
    assert_eq!(a.row, b.row);
    assert_eq!(cut_density_k(&rows), cut_density_k(&changed));
    assert_ne!(decide_replan(&rows).row, decide_replan(&changed).row);
    let d = decision_differences(&rows, &changed);
    assert!(
        d.iter().any(|m| m.contains("re-plan decision is R")),
        "{d:?}"
    );
    assert!(!d.iter().any(|m| m.contains("cut density")), "{d:?}");
}

#[test]
fn a_cut_density_only_change_is_a_difference() {
    let rows = committed().rows;
    // Break the spacing K itself (not a finer one: K is the largest route-green spacing, so
    // breaking a finer spacing leaves it unchanged), whatever the recording's K is.
    let k = cut_density_k(&rows);
    assert!(k >= 1, "this test needs a recording with a defined K");
    let spacing = format!("every{k}");
    let changed = over_route_bound(&rows, |r| {
        r.experiment == "B3"
            && r.mode == "certified"
            && r.knob.s("cut") == spacing
            && r.knob.int("length") == 25
            && (r.knob.f("divergence") - 0.05).abs() < 1e-9
    });
    let (a, b) = (decide(&rows), decide(&changed));
    assert_eq!(a.row, b.row);
    assert_ne!(cut_density_k(&rows), cut_density_k(&changed));
    let d = decision_differences(&rows, &changed);
    assert!(d.iter().any(|m| m.contains("cut density K is")), "{d:?}");
}

#[test]
fn replan_counters_are_noise_but_its_verdicts_are_not() {
    // A wall-capped row varies its counters from run to run: not a difference while the route
    // verdict (and so the row) stays.
    let rows = committed().rows;
    let mut noisy = rows.clone();
    for r in noisy.iter_mut().filter(|r| r.experiment == "A2") {
        r.check_work = r.check_work.map(|w| w + 1);
    }
    assert!(decision_differences(&rows, &noisy).is_empty());
}

// ---------------------------------------------------------------------------------------
// VOS-028c3c-3b-1a (VOS-D141): verifies_sha256, and decision-diff by data
// ---------------------------------------------------------------------------------------

/// Join `shards` through the real `join`.
fn join_files(name: &str, shards: &[(String, Results)]) -> (PathBuf, Result<(), Failure>, String) {
    let dir = scratch(name);
    let files: Vec<String> = shards.iter().map(|(n, r)| write(&dir, n, r)).collect();
    let joined = dir.join("joined.json").to_string_lossy().into_owned();
    let mut args = vec!["join", "--out", joined.as_str()];
    args.extend(files.iter().map(String::as_str));
    let (r, _, e) = run(&args);
    (dir, r, e)
}

#[test]
fn join_requires_verifies_sha256_equal_across_shards() {
    // One shard names another recorded file: refused, naming the field and the shard.
    let mut shards = verify_shards(5);
    shards[2].1.meta.recording.as_mut().unwrap().verifies_sha256 = Some("ab".repeat(32));
    let (dir, r, _) = join_files("join-vs-differ", &shards);
    is_usage(&r, "verifies_sha256");
    is_usage(&r, "shard 2/5");
    assert!(!dir.join("joined.json").exists(), "nothing is written");

    // One shard names none among shards that do: refused the same way.
    let mut shards = verify_shards(5);
    shards[4].1.meta.recording.as_mut().unwrap().verifies_sha256 = None;
    let (_, r, _) = join_files("join-vs-absent", &shards);
    is_usage(&r, "verifies_sha256");
}

#[test]
fn join_carries_verifies_sha256_into_the_joined_file() {
    let (_dir, recorded, j) = fan_in("join-carry");
    let back = Results::from_canonical_text(&std::fs::read_to_string(&j).unwrap()).unwrap();
    let want = Results::from_canonical_text(&std::fs::read_to_string(&recorded).unwrap())
        .unwrap()
        .unsigned_sha256();
    assert_eq!(
        back.meta.recording.unwrap().verifies_sha256.as_deref(),
        Some(want.as_str())
    );
}

#[test]
fn join_still_accepts_shards_that_name_no_recorded_file() {
    // Handled as record-mode files are: every shard agrees (all absent), and the joined file has
    // none.  `decision-diff` is where its absence is refused (below).
    let mut shards = verify_shards(5);
    for (_, s) in &mut shards {
        s.meta.recording.as_mut().unwrap().verifies_sha256 = None;
    }
    let (dir, r, e) = join_files("join-vs-none", &shards);
    assert!(r.is_ok(), "{r:?}\n{e}");
    let back =
        Results::from_canonical_text(&std::fs::read_to_string(dir.join("joined.json")).unwrap())
            .unwrap();
    assert_eq!(back.meta.recording.unwrap().verifies_sha256, None);
}

#[test]
fn the_recording_block_rejects_a_misplaced_or_malformed_verifies_sha256() {
    let good = recorded_with_trees();
    let digest = good.unsigned_sha256();
    let mut v = joined_of(&good, 5);
    // Round trip, in the canonical position (after `jobs`).
    let text = v.to_text();
    assert!(
        text.contains(&format!("\"verifies_sha256\":\"{digest}\"")),
        "{text:.600}"
    );
    assert_eq!(
        Results::from_canonical_text(&text).unwrap().meta.recording,
        v.meta.recording
    );
    // Not 64 hex digits.
    v.meta.recording.as_mut().unwrap().verifies_sha256 = Some("xyz".into());
    let e = Results::from_canonical_text(&v.to_text()).unwrap_err();
    assert!(e.contains("verifies_sha256 is not 64 hex digits"), "{e}");
    // A record-mode file cannot name a file it verified.
    let mut r = joined_of(&good, 5);
    r.meta.recording.as_mut().unwrap().mode = "record".into();
    let e = Results::from_canonical_text(&r.to_text()).unwrap_err();
    assert!(e.contains("only for mode verify"), "{e}");
}

#[test]
fn decision_diff_binds_the_rerun_to_the_recording_by_digest() {
    let dir = scratch("dd-digest");
    let rec = recorded_with_trees();
    let recorded = write(&dir, "rec.json", &rec);
    let dd = |recorded: &str, rerun: &str| {
        run(&["decision-diff", "--recorded", recorded, "--rerun", rerun])
    };

    // A re-run that names a DIFFERENT file: refused (exit 2), naming both digests.
    let mut wrong = joined_of(&rec, 5);
    wrong.meta.recording.as_mut().unwrap().verifies_sha256 = Some("cd".repeat(32));
    let p = write(&dir, "wrong.json", &wrong);
    let (r, _, _) = dd(&recorded, &p);
    is_usage(&r, "verified a different file");
    is_usage(&r, &"cd".repeat(32));
    is_usage(&r, &rec.unsigned_sha256());

    // A re-run that names none: refused, not waved through.
    let mut none = joined_of(&rec, 5);
    none.meta.recording.as_mut().unwrap().verifies_sha256 = None;
    let p = write(&dir, "none.json", &none);
    let (r, _, _) = dd(&recorded, &p);
    is_usage(&r, "names no recorded file");

    // The same rows verified at ANOTHER tree pass: the digest, not the tree ids, is the binding.
    let mut elsewhere = joined_of(&rec, 5);
    for (_, id) in &mut elsewhere.meta.trees {
        *id = format!("{:040x}", 0x77);
    }
    let p = write(&dir, "elsewhere.json", &elsewhere);
    let (r, o, e) = dd(&recorded, &p);
    assert!(r.is_ok(), "{r:?}\n{o}\n{e}");
}

#[test]
fn decision_diff_reports_row_and_decision_differences_in_separate_sections() {
    let dir = scratch("dd-sections");
    let rec = recorded_with_trees();
    let recorded = write(&dir, "rec.json", &rec);
    let joined = joined_of(&rec, 5);

    // 1. A row difference that moves no decision: only the "row differences" section.
    let mut row_only = joined.clone();
    let victim = row_only
        .rows
        .iter_mut()
        .find(|r| r.experiment == "A1")
        .expect("an A1 row");
    victim.propagations += 1;
    let id = results::row_text_id(victim);
    let p = write(&dir, "row-only.json", &row_only);
    let (r, o, e) = run(&["decision-diff", "--recorded", &recorded, "--rerun", &p]);
    assert!(matches!(r, Err(Failure::Violations(1))), "{r:?}\n{o}\n{e}");
    assert!(o.is_empty(), "no ok line on a difference: {o}");
    assert!(e.contains("decision-diff: row differences (1):"), "{e}");
    assert!(
        e.contains(&format!("decision-diff:   row: {id}: propagations differs")),
        "{e}"
    );
    assert!(!e.contains("decision differences"), "{e}");

    // 2. A decision-driving edit: both sections, rows first, then the decision.
    let mut both = joined.clone();
    for r in both.rows.iter_mut().filter(|r| r.experiment == "D3") {
        r.worst_step_conflicts = r.worst_step_conflicts.map(|c| c + 1_000_000);
    }
    let p = write(&dir, "both.json", &both);
    let (r, _, e) = run(&["decision-diff", "--recorded", &recorded, "--rerun", &p]);
    let Err(Failure::Violations(n)) = r else {
        panic!("{r:?}\n{e}")
    };
    assert!(n >= 2, "rows plus decision: {n}");
    let rows_at = e
        .find("decision-diff: row differences (")
        .expect("rows section");
    let dec_at = e
        .find("decision-diff: decision differences (")
        .expect("decision section");
    assert!(rows_at < dec_at, "{e}");
    assert!(e.contains("decision-diff:   decision: "), "{e}");

    // 3. Equal rows: exit 0, the ok line, nothing on stderr.
    let same = write(&dir, "same.json", &joined);
    let (r, o, e) = run(&["decision-diff", "--recorded", &recorded, "--rerun", &same]);
    assert!(r.is_ok(), "{r:?}\n{e}");
    assert!(
        o.contains("every row reproduces under the 9.12.1 classes"),
        "{o}"
    );
    assert!(e.is_empty(), "{e}");
}

#[test]
fn decision_diff_applies_compare_verify_to_every_identity() {
    // A time beyond the band on ONE exact row: the shard that produced it could have exited 0,
    // but the joined data is judged here, row by row, by the rule `check --full` applied.
    let dir = scratch("dd-every");
    let rec = recorded_with_trees();
    let recorded = write(&dir, "rec.json", &rec);
    let mut joined = joined_of(&rec, 5);
    let victim = joined.rows.iter_mut().find(|r| {
        r.result == "unsat" && r.cap != "wall" && r.solve_ms > 1.0 && r.solve_ms < 1000.0
    });
    let Some(victim) = victim else {
        eprintln!("skipped: no short exact row in the committed file");
        return;
    };
    victim.solve_ms = victim.solve_ms * 50.0 + 10_000.0;
    let id = results::row_text_id(victim);
    let p = write(&dir, "slow.json", &joined);
    let (r, _, e) = run(&["decision-diff", "--recorded", &recorded, "--rerun", &p]);
    assert!(
        matches!(r, Err(Failure::Violations(n)) if n >= 1),
        "{r:?}\n{e}"
    );
    assert!(e.contains(&format!("{id}: solve_ms is outside the")), "{e}");
}

// ---------------------------------------------------------------------------------------
// decision-diff accepts only a verify that ran at a committed, well-formed tree (VOS-D141)
// ---------------------------------------------------------------------------------------

#[test]
fn decision_diff_refuses_a_rerun_that_was_not_at_a_committed_tree() {
    let dir = scratch("dd-committed");
    let rec = recorded_with_trees();
    let recorded = write(&dir, "rec.json", &rec);
    let dd = |p: &str| run(&["decision-diff", "--recorded", &recorded, "--rerun", p]);

    // Positive control: a clean re-run passes.
    let good = joined_of(&rec, 5);
    let p = write(&dir, "good.json", &good);
    let (r, o, e) = dd(&p);
    assert!(r.is_ok(), "{r:?}\n{o}\n{e}");

    // Each bad tool-block shape exits 2.  The structural guards may name it first (a junk or
    // legacy tool id is structural); either way the file is refused before any comparison.
    let mut cases: Vec<(&str, Results)> = Vec::new();
    let mut dirty = good.clone();
    dirty.meta.trees[3].1.push_str("+uncommitted");
    cases.push(("uncommitted", dirty));
    let mut legacy = good.clone();
    legacy.meta.trees = results::LEGACY_TREE_NAMES
        .iter()
        .map(|n| (n.to_string(), "a".repeat(40)))
        .collect();
    cases.push(("legacy", legacy));
    let mut junk = good.clone();
    junk.meta.trees[0].1 = "not-a-tree-id".into();
    cases.push(("junk", junk));
    for (name, f) in cases {
        let p = write(&dir, &format!("{name}.json"), &f);
        let (r, _, _) = dd(&p);
        match r {
            Err(Failure::Usage(m)) => assert!(
                m.contains("not verified at a committed tree") || m.contains("structural finding"),
                "{name}: {m}"
            ),
            other => panic!("{name}: {other:?}"),
        }
    }

    // Non-empty merges are refused.
    let mut base = good.clone();
    base.meta.recording = None;
    let mut fresh = base.clone();
    fresh.meta.trees = TREE_NAMES
        .iter()
        .map(|n| (n.to_string(), "b".repeat(40)))
        .collect();
    let mut merged = vos_capacity_probe::cli::merge_results(
        &base,
        &fresh,
        "B3",
        None,
        true,
        "2026-11-19T00:00:00Z",
        &|_| Ok(true),
    )
    .unwrap();
    merged.meta.recording = good.meta.recording.clone();
    let p = write(&dir, "merges.json", &merged);
    let (r, _, _) = dd(&p);
    match r {
        Err(Failure::Usage(m)) => assert!(
            m.contains("records merges") || m.contains("structural finding"),
            "{m}"
        ),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_committed_tree_predicate_names_each_defect() {
    use vos_capacity_probe::meta::committed_tree_refusals as refusals;
    let ok = trees();
    assert!(refusals(&ok).is_empty());
    assert!(refusals(&[])[0].contains("no tree ids"));
    let legacy: Vec<_> = results::LEGACY_TREE_NAMES
        .iter()
        .map(|n| (n.to_string(), "a".repeat(40)))
        .collect();
    assert!(refusals(&legacy)[0].contains("legacy"));
    let mut v = ok.clone();
    v[1].1.push_str("+uncommitted");
    assert!(refusals(&v).iter().any(|m| m.contains("+uncommitted")));
    let mut v = ok.clone();
    v[2].1 = "zz".into();
    assert!(refusals(&v)
        .iter()
        .any(|m| m.contains("not a hex object id")));
    let mut v = ok;
    v.pop();
    assert!(refusals(&v)[0].contains("do not name exactly"));
}
