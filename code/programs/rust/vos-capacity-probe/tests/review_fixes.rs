//! VOS-028c2, the security review's findings H1, H2, M1 to M5 and L1 to L3 (VOS30 section 9.12),
//! each with a negative control: a row, file or path that the fixed code refuses and the code
//! before the fix accepted.  Tests that start child processes are unix-only.

mod common;

#[cfg(unix)]
use std::path::Path;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use vos_capacity_probe::cli::{dispatch, Failure};
use vos_capacity_probe::cost::{derive_cheap, CHEAP, CHEAP_MAX_MS};
use vos_capacity_probe::results::{
    compare_timing, full_plan_covered, full_plan_missing, quick_plan_covered, quick_specs,
    validate, within_band, Results, FULL_PLAN_EXEMPT, RSS_SLACK_KB, TIMING_BAND, TIMING_SLACK_MS,
};
use vos_capacity_probe::row::{Knob, Kv};
use vos_capacity_probe::runner::ULIMIT_SCRIPT;
use vos_capacity_probe::spot::{SPOT_BUDGET, SPOT_FLOOR, SPOT_SLACK};

const EXE: &str = env!("CARGO_BIN_EXE_vos-capacity-probe");

fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs")
        .join(name)
}

fn committed_raw() -> Results {
    Results::from_text(&std::fs::read_to_string(data("data/vos30-capacity-results.json")).unwrap())
        .unwrap()
}

/// The committed file completed with fixture rows for the plan rows it lacks (VOS-028c3c commit A
/// registered B3 every2/every4, D5 and B5 before the recording; `common::complete_plan`).
fn committed() -> Results {
    common::complete_plan(committed_raw())
}

fn spec_text() -> String {
    std::fs::read_to_string(data("VOS30-proof-route-and-capacity.md")).unwrap()
}

/// The text of section 9.12 only.
fn section_9_12() -> String {
    let s = spec_text();
    let i = s.find("### 9.12 ").unwrap();
    let rest = &s[i..];
    let j = rest.find("\n## 10.").unwrap_or(rest.len());
    // Whitespace-normalised, so a phrase wrapped over two lines still matches.
    rest[..j].split_whitespace().collect::<Vec<_>>().join(" ")
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vos-probe-rev-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn argv(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

// ---- H1: the tier is a closed set and the plan's ------------------------------------------------

#[test]
fn h1_a_relabelled_tier_is_refused_for_every_wrong_value() {
    assert!(validate(&committed()).is_empty());
    for wrong in ["x", "Full", "QUICK", ""] {
        let mut f = committed();
        let r = f.rows.iter_mut().find(|r| r.tier == "full").unwrap();
        r.tier = wrong.into();
        let v = validate(&f).join("|");
        assert!(v.contains("is not quick or full"), "{wrong:?}: {v}");
    }
    // A full-plan row labelled quick, and a quick-plan row labelled full, are refused as well:
    // `tier` decides what `check` re-runs in process and which rows the sample draws from.
    let mut f = committed();
    f.rows.iter_mut().find(|r| r.tier == "full").unwrap().tier = "quick".into();
    assert!(validate(&f)
        .join("|")
        .contains("gives this knob the full tier"));
    let mut f = committed();
    f.rows.iter_mut().find(|r| r.tier == "quick").unwrap().tier = "full".into();
    assert!(validate(&f)
        .join("|")
        .contains("gives this knob the quick tier"));
}

#[test]
fn h1_a_relabelled_row_never_reaches_the_in_process_quick_re_run() {
    let mut f = committed();
    let n = quick_specs(&f).len();
    // A slow full-tier row relabelled quick would be re-run in process by `check`.
    let r = f
        .rows
        .iter_mut()
        .find(|r| r.experiment == "B2" && r.tier == "full")
        .unwrap();
    r.tier = "quick".into();
    assert_eq!(
        quick_specs(&f).len(),
        n,
        "the plan says B2 length 50 is not quick"
    );
}

// ---- H2: the file is exactly the plan -----------------------------------------------------------

#[test]
fn h2_the_committed_file_is_exactly_the_full_plan_and_the_exemption_list_is_the_missing_set() {
    let f = committed();
    assert!(
        full_plan_covered(&f).is_empty(),
        "{:?}",
        full_plan_covered(&f)
    );
    assert!(quick_plan_covered(&f).is_empty());
    let mut missing = full_plan_missing(&f);
    missing.sort();
    let mut exempt: Vec<String> = FULL_PLAN_EXEMPT
        .iter()
        .map(|(id, _)| id.to_string())
        .collect();
    exempt.sort();
    assert_eq!(
        exempt, missing,
        "the reviewed exemption list must equal the missing rows exactly"
    );
    // Every exemption has a reason, and section 9.12 lists it with that reason.
    let spec = section_9_12();
    for (id, why) in FULL_PLAN_EXEMPT {
        assert!(!why.trim().is_empty());
        assert!(spec.contains(id) && spec.contains(why), "9.12 lacks {id}");
    }
    assert!(
        FULL_PLAN_EXEMPT.is_empty() == spec.contains("`FULL_PLAN_EXEMPT` is empty"),
        "9.12 says what the list is"
    );
}

#[test]
fn h2_a_missing_extra_or_duplicated_row_and_a_seed_count_mismatch_are_each_refused() {
    // A row deleted from the file.
    let mut f = committed();
    let at = f
        .rows
        .iter()
        .position(|r| r.experiment == "D2" && r.tier == "full")
        .unwrap();
    let gone = f.rows.remove(at);
    let v = full_plan_covered(&f).join("|");
    assert!(
        v.contains("is not in the results file") && v.contains("D2"),
        "{v}"
    );
    assert!(full_plan_missing(&f).iter().any(|m| m.contains("D2")));
    // A whole knob group deleted.
    let mut f = committed();
    f.rows
        .retain(|r| !(r.experiment == "D2" && r.knob == gone.knob));
    assert_eq!(full_plan_missing(&f).len(), 5);
    // A duplicated row.
    let mut f = committed();
    let dup = f.rows[at].clone();
    f.rows.push(dup);
    assert!(full_plan_covered(&f).join("|").contains("more than once"));
    // An extra row: a knob the plan does not have.
    let mut f = committed();
    let mut extra = f.rows[at].clone();
    extra.knob = Knob::of(&[("t", Kv::I(7)), ("k", Kv::I(7))]);
    f.rows.push(extra);
    assert!(full_plan_covered(&f)
        .join("|")
        .contains("does not contain the file's row"));
    // Recorded seeds that disagree with the seeds present.
    let mut f = committed();
    f.rows[at].seeds = 6;
    assert!(full_plan_covered(&f)
        .join("|")
        .contains("rows record seeds = 6 but 5 seeds"));
    // The same for the quick tier, which is the mirror image.
    let mut f = committed();
    let q = f.rows.iter().position(|r| r.tier == "quick").unwrap();
    let dup = f.rows[q].clone();
    f.rows.push(dup);
    assert!(quick_plan_covered(&f).join("|").contains("more than once"));
}

#[test]
fn h2_check_refuses_a_file_with_a_row_deleted_before_it_re_runs_anything() {
    let d = scratch("h2");
    let p = d.join("results.json");
    let mut f = committed();
    f.meta.merges.clear();
    let at = f
        .rows
        .iter()
        .position(|r| r.experiment == "D2" && r.tier == "full")
        .unwrap();
    f.rows.remove(at);
    std::fs::write(&p, f.to_text()).unwrap();
    let (mut o, mut e) = (Vec::new(), Vec::new());
    let t0 = Instant::now();
    let r = dispatch(
        &argv(&["check", p.to_str().unwrap(), "--exe", EXE]),
        &mut o,
        &mut e,
    );
    // VOS-D134: a deleted row leaves a knob with too few seeds, which is STRUCTURAL, so the
    // refusal is exit 2 (Usage) and comes before plan coverage is even reported.
    assert!(
        matches!(&r, Err(Failure::Usage(m)) if m.contains("nothing was re-run")),
        "{r:?}"
    );
    assert!(String::from_utf8(e)
        .unwrap()
        .contains("distinct seeds (at least 5 per knob)"));
    assert!(t0.elapsed() < Duration::from_secs(10), "nothing was re-run");
}

// ---- M1: timing-derived fields, in `check --full` only ------------------------------------------

#[test]
fn m1_the_band_is_two_sided_and_the_recorded_green_is_recomputed_from_the_rerun() {
    assert!(within_band(100.0, 100.0, 0.0));
    assert!(within_band(399.0, 100.0, 0.0) && within_band(26.0, 100.0, 0.0));
    // Raising AND lowering a recorded time past the band is refused.
    assert!(!within_band(401.0, 100.0, 0.0) && !within_band(24.0, 100.0, 0.0));
    // The slack keeps two tiny values from being a finding.
    assert!(within_band(0.0, 40.0, TIMING_SLACK_MS) && within_band(40.0, 0.0, TIMING_SLACK_MS));
    let f = committed();
    let rec = f
        .rows
        .iter()
        .find(|r| r.experiment == "A1" && r.tier == "full")
        .unwrap()
        .clone();
    assert!(compare_timing(&rec, &rec).is_empty());
    // An editor LOWERS a recorded solve time of a slow row to turn it green: refused.
    let mut slow = f
        .rows
        .iter()
        .find(|r| r.solve_ms > 60_000.0 && r.mode != "enum")
        .unwrap()
        .clone();
    let honest = slow.clone();
    slow.solve_ms = 1.0;
    slow.worst_step_solve_ms = 1.0;
    let v = compare_timing(&slow, &honest).join("|");
    assert!(v.contains("solve_ms is outside"), "{v}");
    assert!(v.contains("worst_step_solve_ms is outside"), "{v}");
    // Each timing field has its own refusal.
    for (field, edit) in [("check_ms", 0usize), ("peak_rss_kb", 1), ("solve_ms", 2)] {
        let mut r = rec.clone();
        match edit {
            0 => r.check_ms = Some(r.check_ms.unwrap_or(0.0) + 100_000.0),
            1 => r.peak_rss_kb = r.peak_rss_kb * 100 + 1_000_000,
            _ => r.solve_ms += 100_000.0,
        }
        let v = compare_timing(&r, &rec).join("|");
        assert!(v.contains(&format!("{field} is outside")), "{field}: {v}");
    }
    // `green` is recomputed from the re-run's measurements: a recorded green that the re-run's own
    // numbers do not give is refused even when every time is inside the band.
    let mut red_now = rec.clone();
    red_now.conflicts = 5_000_000;
    let v = compare_timing(&rec, &red_now).join("|");
    assert!(
        v.contains("green is true but the re-run's own measurements give false"),
        "{v}"
    );
    // A row that ended killed or oom has no reproducible time: only its result is compared.
    let mut killed = rec.clone();
    killed.result = "killed".into();
    killed.solve_ms = 9.0e9;
    assert!(compare_timing(&killed, &rec).is_empty());
    assert!((TIMING_BAND, TIMING_SLACK_MS, RSS_SLACK_KB) == (4.0, 50.0, 8192.0));
}

// ---- M2: no instance is built from an unvalidated file ------------------------------------------

#[test]
fn m2_a_quick_row_with_a_hostile_knob_is_refused_without_building_its_instance() {
    let d = scratch("m2");
    let p = d.join("results.json");
    let mut f = committed();
    f.meta.merges.clear();
    // B1 is a quick-tier experiment whose knob is a length: 10^9 instructions.
    let r = f
        .rows
        .iter_mut()
        .find(|r| r.experiment == "B1" && r.tier == "quick")
        .unwrap();
    r.knob = Knob::of(&[("length", Kv::I(1_000_000_000)), ("divergence", Kv::F(0.0))]);
    assert!(quick_specs(&f).iter().all(|(s, _)| s.knob != r_knob(&f)));
    std::fs::write(&p, f.to_text()).unwrap();
    let (mut o, mut e) = (Vec::new(), Vec::new());
    let t0 = Instant::now();
    let r = dispatch(
        &argv(&["check", p.to_str().unwrap(), "--exe", EXE]),
        &mut o,
        &mut e,
    );
    let e = String::from_utf8(e).unwrap();
    // VOS-D134: a knob outside the plan is a structural finding: exit 2, nothing re-run.
    assert!(
        matches!(&r, Err(Failure::Usage(m)) if m.contains("nothing was re-run")),
        "{r:?} {e}"
    );
    assert!(e.contains("not one of the plan's knobs"), "{e}");
    assert!(
        t0.elapsed() < Duration::from_secs(10),
        "an instance was built: {:?}",
        t0.elapsed()
    );
}

fn r_knob(f: &Results) -> Knob {
    f.rows
        .iter()
        .find(|r| r.experiment == "B1" && r.tier == "quick")
        .unwrap()
        .knob
        .clone()
}

// ---- M3 and the constants: the spec says what the code does ------------------------------------

#[test]
fn m3_the_spec_no_longer_claims_the_sample_cannot_be_predicted_and_says_the_gate() {
    let spec = section_9_12();
    assert!(
        !spec.contains("cannot be predicted from the repository alone"),
        "the old claim is false: the seed is deterministic"
    );
    assert!(
        spec.contains("grind"),
        "the spec says an editor can grind the file"
    );
    assert!(spec.contains("`check --full` is the release gate"));
    assert!(spec.contains("timing-derived verdicts are verified only by `check --full`"));
}

#[test]
fn the_constants_the_spec_quotes_are_the_ones_in_the_code() {
    let spec = section_9_12();
    let must = [
        format!("`TIMING_BAND` = {TIMING_BAND}"),
        format!("`TIMING_SLACK_MS` = {TIMING_SLACK_MS}"),
        format!("`RSS_SLACK_KB` = {RSS_SLACK_KB}"),
        format!("`SPOT_BUDGET` = {} s", SPOT_BUDGET.as_secs()),
        format!("`SPOT_SLACK` = {} s", SPOT_SLACK.as_secs()),
        format!("`SPOT_FLOOR` = {SPOT_FLOOR}"),
        format!("`CHEAP_MAX_MS` = {CHEAP_MAX_MS}"),
        format!("`{ULIMIT_SCRIPT}`"),
    ];
    for m in must {
        assert!(spec.contains(&m), "9.12 does not quote {m}");
    }
    assert!(
        !spec.contains("`ulimit -t \"$1\"; shift; exec \"$@\"`"),
        "the old literal is gone"
    );
}

// ---- M4: the cost table agrees with the recording it was derived from -------------------------

#[test]
fn cost_table_is_exactly_the_committed_recordings_cheap_set() {
    // The REAL committed recording (CI run 45486250172), not the fixture-completed file: the cost
    // table describes recorded rows only.  The rule is `cost::derive_cheap` (slowest seed's solve
    // plus check at most `CHEAP_MAX_MS`; a killed or oom seed is infinitely slow), and the table
    // must equal what it derives: no group missing, none extra (VOS-D143; this replaces the
    // "at most 4x the line" test commit B loosened when the recording moved from a Mac to CI).
    let derived: std::collections::BTreeSet<(String, String, String)> =
        derive_cheap(&committed_raw().rows)
            .into_iter()
            .map(|(k, _)| k)
            .collect();
    let listed: std::collections::BTreeSet<(String, String, String)> = CHEAP
        .iter()
        .map(|(e, m, k)| (e.to_string(), m.to_string(), k.to_string()))
        .collect();
    assert_eq!(listed.len(), CHEAP.len(), "a group is listed twice");
    let missing: Vec<_> = derived.difference(&listed).collect();
    let extra: Vec<_> = listed.difference(&derived).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "CHEAP is not the derived set; derived but unlisted: {missing:?}; listed but not derived: {extra:?}"
    );
    // Negative control: a recording in which one cheap group's slowest seed is over the line, or
    // killed, no longer derives that group.
    let mut rows = committed_raw().rows.clone();
    let (e, m, k) = CHEAP[0];
    rows.iter_mut()
        .find(|r| r.experiment == e && r.mode == m && r.knob.key() == k && r.tier == "full")
        .unwrap()
        .solve_ms = CHEAP_MAX_MS + 1.0;
    assert!(!derive_cheap(&rows)
        .iter()
        .any(|((a, b, c), _)| (a.as_str(), b.as_str(), c.as_str()) == (e, m, k)));
    for r in rows
        .iter_mut()
        .filter(|r| r.experiment == "A1" && r.tier == "full")
    {
        r.result = "killed".into();
    }
    assert!(!derive_cheap(&rows).iter().any(|((a, _, _), _)| a == "A1"));
    // So do a crashed seed and a time that is not a number: neither is a finished run.
    for bad in ["crashed", "nan"] {
        let mut rows = committed_raw().rows.clone();
        let (e, m, k) = CHEAP[0];
        let r = rows
            .iter_mut()
            .find(|r| r.experiment == e && r.mode == m && r.knob.key() == k && r.tier == "full")
            .unwrap();
        if bad == "crashed" {
            r.result = "crashed".into();
        } else {
            r.solve_ms = f64::NAN;
        }
        let derived = derive_cheap(&rows);
        assert!(
            !derived
                .iter()
                .any(|((a, b, c), _)| (a.as_str(), b.as_str(), c.as_str()) == (e, m, k)),
            "{bad}"
        );
        assert!(derived.iter().all(|(_, t)| t.is_finite()), "{bad}");
    }
}

// ---- L1: every reader is capped ------------------------------------------------------------------

#[test]
fn l1_check_table_and_decide_refuse_a_file_over_the_cap() {
    let d = scratch("l1");
    let big = d.join("big.json");
    // A sparse file one byte over the cap: no disk is used and nothing is read.
    let f = std::fs::File::create(&big).unwrap();
    f.set_len(vos_capacity_probe::results::MAX_FILE_BYTES + 1)
        .unwrap();
    drop(f);
    let b = big.to_str().unwrap();
    let ok_spec = data("VOS30-proof-route-and-capacity.md");
    let ok_res = data("data/vos30-capacity-results.json");
    let cases: Vec<Vec<String>> = vec![
        argv(&["check", b]),
        argv(&["decide", "--results", b]),
        argv(&["table", "--results", b, "--spec", ok_spec.to_str().unwrap()]),
        argv(&[
            "table",
            "--check",
            "--results",
            ok_res.to_str().unwrap(),
            "--spec",
            b,
        ]),
        argv(&[
            "merge",
            "--base",
            b,
            "--fresh",
            ok_res.to_str().unwrap(),
            "--experiment",
            "A1",
            "--out",
            d.join("o.json").to_str().unwrap(),
        ]),
    ];
    for args in cases {
        let (mut o, mut e) = (Vec::new(), Vec::new());
        match dispatch(&args, &mut o, &mut e) {
            Err(Failure::Usage(m)) => assert!(m.contains("byte cap"), "{args:?}: {m}"),
            other => panic!("{args:?} was not refused: {other:?}"),
        }
    }
}

// ---- L2: atomic, symlink-safe output ------------------------------------------------------------

#[cfg(unix)]
const SPEC_WITH_MARKERS: &str = "### 8.5 Current decision state\n\nnothing\n\n## 9. Probe\n\n<!-- vos30-table:begin -->\n<!-- vos30-table:end -->\n";

#[cfg(unix)]
/// `table --write` into `spec` (it goes through the same atomic writer as `run --out` and
/// `merge --out`).
fn write_args(spec: &Path) -> Vec<String> {
    argv(&[
        "table",
        "--write",
        "--results",
        data("data/vos30-capacity-results.json").to_str().unwrap(),
        "--spec",
        spec.to_str().unwrap(),
    ])
}

#[cfg(unix)]
#[test]
fn l2_a_symlink_out_is_refused_and_its_target_is_untouched() {
    let d = scratch("l2-link");
    let victim = d.join("victim.txt");
    std::fs::write(&victim, SPEC_WITH_MARKERS).unwrap();
    let link = d.join("spec.md");
    std::os::unix::fs::symlink(&victim, &link).unwrap();
    let r = dispatch(&write_args(&link), &mut Vec::new(), &mut Vec::new());
    match r {
        Err(Failure::Usage(m)) => assert!(m.contains("symbolic link"), "{m}"),
        other => panic!("{other:?}"),
    }
    assert_eq!(std::fs::read_to_string(&victim).unwrap(), SPEC_WITH_MARKERS);
    assert!(std::fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());
    // A dangling link is refused too (it cannot even be read as a spec), and nothing is created.
    let dangling = d.join("dangling.md");
    std::os::unix::fs::symlink(d.join("nowhere.txt"), &dangling).unwrap();
    assert!(dispatch(&write_args(&dangling), &mut Vec::new(), &mut Vec::new()).is_err());
    assert!(!d.join("nowhere.txt").exists());
}

#[cfg(unix)]
#[test]
fn l2_the_write_is_atomic_in_the_same_directory_and_keeps_the_target_mode() {
    use std::os::unix::fs::PermissionsExt;
    let d = scratch("l2-mode");
    let spec = d.join("spec.md");
    std::fs::write(&spec, SPEC_WITH_MARKERS).unwrap();
    std::fs::set_permissions(&spec, std::fs::Permissions::from_mode(0o640)).unwrap();
    // The target keeps its own mode, the table is written, and no temporary file is left.
    dispatch(&write_args(&spec), &mut Vec::new(), &mut Vec::new()).unwrap();
    assert_eq!(
        std::fs::metadata(&spec).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert!(std::fs::read_to_string(&spec).unwrap().contains("| A1 "));
    let names = |d: &Path| -> Vec<String> {
        std::fs::read_dir(d)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect()
    };
    assert_eq!(names(&d), vec!["spec.md".to_string()]);
    // A pre-planted file under any guessable temporary name is neither followed nor overwritten.
    for planted in [".spec.md.tmp", "spec.tmp", "spec.md.tmp1"] {
        std::fs::write(d.join(planted), "planted").unwrap();
    }
    dispatch(&write_args(&spec), &mut Vec::new(), &mut Vec::new()).unwrap();
    for planted in [".spec.md.tmp", "spec.tmp", "spec.md.tmp1"] {
        assert_eq!(std::fs::read_to_string(d.join(planted)).unwrap(), "planted");
    }
    // A directory that cannot be written to: the command fails and leaves no temporary file.
    let ro = scratch("l2-ro");
    let spec = ro.join("spec.md");
    std::fs::write(&spec, SPEC_WITH_MARKERS).unwrap();
    std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o555)).unwrap();
    // (A superuser can write anyway; the assertion is only for an ordinary user.)
    if std::fs::File::create(ro.join("probe")).is_err() {
        assert!(matches!(
            dispatch(&write_args(&spec), &mut Vec::new(), &mut Vec::new()),
            Err(Failure::Usage(_))
        ));
        assert_eq!(names(&ro), vec!["spec.md".to_string()]);
    }
    std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o755)).unwrap();
}

// ---- L3: the ulimit wrapper fails closed ---------------------------------------------------------

#[cfg(unix)]
#[test]
fn l3_a_limit_that_cannot_be_set_stops_the_row_instead_of_running_it_unbounded() {
    let d = scratch("l3");
    let witness = d.join("ran");
    let run = |limit: &str| {
        std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(ULIMIT_SCRIPT)
            .arg("sh")
            .arg(limit)
            .arg("/usr/bin/touch")
            .arg(&witness)
            .status()
            .unwrap()
    };
    // Not a number: `ulimit` fails, the script exits 126 and the program never starts.
    assert_eq!(run("not-a-number").code(), Some(126));
    assert!(!witness.exists());
    // A valid limit runs the program.
    assert!(run("100").success());
    assert!(witness.exists());
}

/// A failed rename (the target is a directory) reports the error and leaves no temporary file.
#[cfg(unix)]
#[test]
fn l2_a_failed_write_reports_an_error_and_removes_its_temporary_file() {
    let d = scratch("l2-dir");
    let target = d.join("out.json");
    std::fs::create_dir(&target).unwrap();
    let r = dispatch(
        &argv(&[
            "run",
            "--tier",
            "quick",
            "--only",
            "A1",
            "--exe",
            EXE,
            "--out",
            target.to_str().unwrap(),
        ]),
        &mut Vec::new(),
        &mut Vec::new(),
    );
    match r {
        Err(Failure::Usage(m)) => assert!(m.contains("cannot write"), "{m}"),
        other => panic!("{other:?}"),
    }
    let left: Vec<String> = std::fs::read_dir(&d)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, vec!["out.json".to_string()], "no .tmp file is left");
}

/// The `merge` command end to end through the atomic writer: a refresh with the fresh run's
/// revision at HEAD succeeds, and the output is exactly what `merge_results` produces.
#[cfg(unix)]
#[test]
fn l2_merge_writes_its_output_atomically_through_the_command() {
    let head = String::from_utf8(
        std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();
    assert_eq!(head.len(), 40);
    let d = scratch("l2-merge");
    let base = committed();
    let mut fresh = committed();
    fresh.meta.merges.clear();
    fresh.meta.revisions = vec![("repo".into(), head)];
    fresh.meta.trees = base.meta.trees.clone();
    let (b, f, out) = (
        d.join("base.json"),
        d.join("fresh.json"),
        d.join("out.json"),
    );
    std::fs::write(&b, base.to_text()).unwrap();
    std::fs::write(&f, fresh.to_text()).unwrap();
    let args = argv(&[
        "merge",
        "--base",
        b.to_str().unwrap(),
        "--fresh",
        f.to_str().unwrap(),
        "--experiment",
        "A1",
        "--ignore-outside",
        "--out",
        out.to_str().unwrap(),
    ]);
    let mut o = Vec::new();
    dispatch(&args, &mut o, &mut Vec::new()).unwrap();
    assert!(String::from_utf8(o)
        .unwrap()
        .starts_with("merged 20 rows of A1"));
    let merged = Results::from_text(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(
        merged.rows, base.rows,
        "an identical refresh changes no row"
    );
    assert_eq!(merged.meta.merges.len(), base.meta.merges.len() + 1);
    assert!(validate(&merged).is_empty());
}
