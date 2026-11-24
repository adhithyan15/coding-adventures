//! VOS-028c3c-2b-ii: the verify fan-in (`check --full --shard`, `--rerun-out`, `join` of verify
//! shards, `decision-diff`), VOS-D132 (the re-plan decision and `K` in `decision_differences`) and
//! VOS-D134 (the guards that run before any re-run).
//!
//! Everything here runs on every platform: the only re-run is one in-process quick row, and every
//! refusal is shown with an executable that does not exist, which proves it happened BEFORE any
//! re-run (a re-run would have had to spawn it, or would have taken a visible time).

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

fn committed() -> Results {
    Results::from_canonical_text(&std::fs::read_to_string(committed_path()).unwrap()).unwrap()
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
    assert_eq!(all.len(), 85);
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
        assert_eq!(seen.len(), 85, "N = {n}: each quick row once");
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
    assert_eq!(quick_row_cap(), 85);
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
            .any(|m| m.contains("quick-tier rows to re-run, but the plan has 85")),
        "{g:?}"
    );
    let p = write(&dir, "dup.json", &f);
    for extra in [&[][..], &["--full"][..]] {
        let mut args = vec!["check", p.as_str(), "--exe", NO_SUCH_EXE];
        args.extend_from_slice(extra);
        let (r, _, e) = run(&args);
        is_usage(&r, "nothing was re-run");
        assert!(e.contains("appears more than once"), "{e}");
        assert!(e.contains("but the plan has 85"), "{e}");
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

    // A shard that holds no row is not a "no quick-tier row" finding either.
    let (r, o, e) = run(&["check", &src, "--full", "--shard", "1000/1024"]);
    assert!(r.is_ok(), "{r:?}\n{o}\n{e}");
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
            });
            (format!("v{k}.json"), r)
        })
        .collect()
}

#[test]
fn join_fans_verify_shards_in_and_decision_diff_agrees_with_the_recording() {
    let dir = scratch("fanin");
    let files: Vec<String> = verify_shards(5)
        .into_iter()
        .map(|(name, r)| write(&dir, &name, &r))
        .collect();
    let joined = dir.join("joined.json");
    let j = joined.to_str().unwrap();
    let mut args = vec![
        "join",
        "--out",
        j,
        "--expect-run-id",
        "99",
        "--expect-shards",
        "5",
    ];
    args.extend(files.iter().map(String::as_str));
    let (r, o, e) = run(&args);
    assert!(r.is_ok(), "{r:?}\n{o}\n{e}");
    let back = Results::from_canonical_text(&std::fs::read_to_string(&joined).unwrap()).unwrap();
    let rc = back.meta.recording.as_ref().unwrap();
    assert_eq!((rc.mode.as_str(), rc.shards), ("verify", Some(5)));

    // join refuses to overwrite the fan-in file, as it does any output.
    let (r, _, _) = run(&args);
    is_usage(&r, "already exists");

    // The joined re-run rows are the recorded rows: no difference, exit 0.
    let recorded = write(&dir, "recorded.json", &committed());
    let (r, o, e) = run(&["decision-diff", "--recorded", &recorded, "--rerun", j]);
    assert!(r.is_ok(), "{r:?}\n{o}\n{e}");
    assert!(o.starts_with("decision-diff: ok ("), "{o}");
}

#[test]
fn decision_diff_exits_0_equal_1_different_and_2_on_other_rows() {
    let dir = scratch("dd");
    let rec = committed();
    let recorded = write(&dir, "rec.json", &rec);
    // The committed file against itself: exit 0.
    let (r, _, _) = run(&[
        "decision-diff",
        "--recorded",
        &recorded,
        "--rerun",
        &recorded,
    ]);
    assert!(r.is_ok(), "{r:?}");

    // An edited decision-driving field: exit 1, the difference printed.
    let mut edited = rec.clone();
    for r in edited.rows.iter_mut().filter(|r| r.experiment == "D3") {
        r.worst_step_conflicts = r.worst_step_conflicts.map(|c| c + 1_000_000);
    }
    let p = write(&dir, "edited.json", &edited);
    let (r, _, e) = run(&["decision-diff", "--recorded", &recorded, "--rerun", &p]);
    assert!(matches!(r, Err(Failure::Violations(n)) if n >= 1), "{r:?}");
    assert!(e.contains("decision-diff:"), "{e}");

    // Different identity sets: exit 2 (it is not a re-run of the recording).
    let mut fewer = rec.clone();
    fewer.rows.pop();
    let p = write(&dir, "fewer.json", &fewer);
    let (r, _, _) = run(&["decision-diff", "--recorded", &recorded, "--rerun", &p]);
    is_usage(&r, "different rows");
    let mut more = rec.clone();
    more.rows.push(rec.rows[0].clone());
    let p = write(&dir, "more.json", &more);
    let (r, _, _) = run(&["decision-diff", "--recorded", &recorded, "--rerun", &p]);
    is_usage(&r, "twice");

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
            recorded.as_str(),
        ],
        vec![
            "decision-diff",
            "--recorded",
            &recorded,
            "--rerun",
            &recorded,
            "--bogus",
        ],
        vec![
            "decision-diff",
            "--recorded",
            &recorded,
            "--rerun",
            &recorded,
            "stray",
        ],
    ] {
        let (r, _, _) = run(&args);
        assert!(matches!(r, Err(Failure::Usage(_))), "{args:?}: {r:?}");
    }
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
    let changed = over_route_bound(&rows, |r| {
        r.experiment == "B3"
            && r.mode == "certified"
            && r.knob.s("cut") == "every1"
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
