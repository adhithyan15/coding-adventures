//! The command line end to end: exit codes, the child process, the quick tier, `check`, `table`
//! and `decide`, on the real binary and on real child processes.

use std::process::Command;
use vos_capacity_probe::cli::{dispatch, Failure};
use vos_capacity_probe::exp::RowSpec;
use vos_capacity_probe::row::{Knob, Kv};

const EXE: &str = env!("CARGO_BIN_EXE_vos-capacity-probe");

fn run(args: &[&str]) -> (String, String, Result<(), Failure>) {
    let a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let r = dispatch(&a, &mut out, &mut err);
    (
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
        r,
    )
}

fn spec(exp: &str, knob: Knob, mode: &str, seed: u64) -> RowSpec {
    RowSpec {
        experiment: exp.into(),
        knob,
        mode: mode.into(),
        seed,
        seeds: 5,
        tier: "quick".into(),
    }
}

#[test]
fn list_names_every_experiment() {
    let (out, _, r) = run(&["list"]);
    assert!(r.is_ok());
    for e in [
        "A1", "A2", "B1", "B2", "B3", "B4", "C1", "C2", "D1", "D2", "D3", "D4", "E1", "E2",
    ] {
        assert!(out.contains(e), "{e}");
    }
    assert!(out.contains("seeds per knob: 5"));
}

#[test]
fn a_malformed_command_line_is_a_usage_error_not_a_violation() {
    for args in [
        vec![],
        vec!["bogus"],
        vec!["run"],
        vec!["run", "--tier", "fast"],
        vec!["run", "--tier", "quick", "--seeds", "3"],
        vec!["run", "--tier", "quick", "--only", "ZZ"],
        vec!["check"],
        vec!["check", "/no/such/file.json"],
        vec!["row"],
        vec!["row", "--spec", "nope"],
        vec!["decide", "--results", "/no/such.json"],
    ] {
        let (_, _, r) = run(&args);
        assert!(matches!(r, Err(Failure::Usage(_))), "{args:?}: {r:?}");
    }
    let st = Command::new(EXE).arg("bogus").output().unwrap();
    assert_eq!(st.status.code(), Some(2));
    let st = Command::new(EXE).arg("list").output().unwrap();
    assert_eq!(st.status.code(), Some(0));
}

#[test]
fn the_row_child_prints_a_progress_line_then_its_answer() {
    let s = spec("A1", Knob::of(&[("n", Kv::I(8))]), "monolithic", 1);
    let out = Command::new(EXE)
        .args([
            "row",
            "--spec",
            &vos_capacity_probe::runner::spec_to_json(&s),
            "--wall",
            "30",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert!(lines[0].starts_with("{\"progress\":") && lines[1].starts_with("{\"row\":"));
    let parsed = vos_capacity_probe::runner::parse_child_out(&text);
    let row = parsed.row.unwrap();
    assert_eq!(row.result, "unsat");
    assert!(row.control.ok());
    assert_eq!(
        parsed.progress.unwrap().instance_sha256,
        row.instance_sha256
    );
}

// Unix-only: these tests run rows as child processes, and the runner starts each child under
// `/usr/bin/time` and polls it with `ps`, neither of which exists on Windows.  The in-process
// `check` (which re-runs every quick row without children) is what BUILD_windows runs instead.
// Everything only they use (imports, helpers) lives here so Windows builds no dead code.
#[cfg(unix)]
mod unix_children {
    use super::*;
    use std::path::{Path, PathBuf};
    use vos_capacity_probe::results::{Results, RSS_CAP_KB};
    use vos_capacity_probe::runner::{finish_row, hard_limit, run_child, run_rows, End};

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("vos-probe-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn rows_run_as_children_and_come_back_green_with_a_parsed_peak_rss() {
        let specs: Vec<RowSpec> = (1..=3)
            .map(|s| spec("A1", Knob::of(&[("n", Kv::I(8))]), "monolithic", s))
            .collect();
        let rows = run_rows(Path::new(EXE), &specs, 2, &|_, _| {});
        assert_eq!(rows.len(), 3);
        for r in &rows {
            assert_eq!(r.result, "unsat", "{}", r.note);
            assert!(r.peak_rss_kb > 0, "never defaulted to 0");
            assert!(r.green);
        }
        assert_eq!(
            rows.iter().map(|r| r.seed).collect::<Vec<_>>(),
            vec![1, 2, 3],
            "in spec order"
        );
    }

    #[test]
    fn a_child_over_the_memory_cap_is_killed_as_oom_and_a_slow_one_as_killed() {
        // D1 at T = 8, k = 2 runs for a few hundred milliseconds: long enough to be polled.
        let s = spec(
            "D1",
            Knob::of(&[("t", Kv::I(8)), ("k", Kv::I(2))]),
            "monolithic",
            1,
        );
        let ch = run_child(Path::new(EXE), &s, 120, 1_000, hard_limit()).unwrap();
        assert_eq!(ch.end, End::Oom);
        let row = finish_row(&s, &ch);
        assert_eq!((row.result.as_str(), row.cap.as_str()), ("oom", "rss"));
        assert!(row.peak_rss_kb >= RSS_CAP_KB && !row.green);
        let ch = run_child(
            Path::new(EXE),
            &s,
            120,
            RSS_CAP_KB,
            std::time::Duration::from_millis(60),
        )
        .unwrap();
        assert_eq!(ch.end, End::Killed);
        let row = finish_row(&s, &ch);
        assert_eq!((row.result.as_str(), row.cap.as_str()), ("killed", "wall"));
        assert!(!row.green);
    }

    /// The whole quick tier once, then everything that reads a results file.
    #[test]
    fn quick_tier_check_table_and_decide() {
        use vos_capacity_probe::results;
        let d = scratch("quick");
        let raw = d.join("raw.json");
        let (out, err, r) = run(&[
            "run",
            "--tier",
            "quick",
            "--exe",
            EXE,
            "--jobs",
            "4",
            "--out",
            raw.to_str().unwrap(),
        ]);
        assert!(r.is_ok(), "{err}");
        assert!(out.contains("ran 105 rows"), "{out}");
        let mut res = Results::from_text(&std::fs::read_to_string(&raw).unwrap()).unwrap();
        // A run on a development tree is dirty; the committed file may not be.
        res.meta.dirty_tree = false;
        let results = d.join("results.json");
        std::fs::write(&results, res.to_text()).unwrap();

        // `check` on a quick-only file: the file is not the full plan (finding H2), so it is
        // refused at once with nothing but "missing full row" violations, and nothing is re-run.
        let (_, err, r) = run(&["check", results.to_str().unwrap(), "--jobs", "4"]);
        assert!(matches!(r, Err(Failure::Violations(n)) if n > 0), "{r:?}");
        assert!(
            err.lines()
                .all(|l| l.contains("is not in the results file")),
            "{err}"
        );

        // The quick re-run itself (what `check` runs on a complete file) reproduces, and a
        // tampered digest, a tampered conflict count, a dirty tree and a dropped group are each
        // caught.
        assert!(results::validate(&res).is_empty());
        assert!(results::quick_plan_covered(&res).is_empty());
        assert!(results::rerun(&res, 4).is_empty());
        let caught = |f: &dyn Fn(&mut Results), rerun: bool| -> String {
            let mut x = res.clone();
            f(&mut x);
            let mut v = results::validate(&x);
            v.extend(results::quick_plan_covered(&x));
            if rerun {
                // Only a digest or a count needs the (slow) in-process re-run to be seen.
                v.extend(results::rerun(&x, 4));
            }
            assert!(!v.is_empty());
            v.join("\n")
        };
        assert!(
            caught(&|x| x.rows[0].instance_sha256 = "00".repeat(32), true)
                .contains("instance_sha256 differs")
        );
        assert!(caught(&|x| x.rows[0].conflicts += 1, true).contains("conflicts differs"));
        assert!(caught(&|x| x.meta.dirty_tree = true, false).contains("dirty_tree"));
        assert!(caught(
            &|x| {
                x.rows.retain(|r| r.experiment != "A2");
            },
            false
        )
        .contains("is not in the results file"));

        // table: prints a block; --write needs the markers; --check passes then fails on drift.
        let spec_path = d.join("spec.md");
        std::fs::write(&spec_path, concat!(
                "### 8.5 Current decision state\n\nnothing stated\n\n",
                "### 8.6 The re-plan\n\n",
                "**Re-plan decision (VOS-028c): R1, no lane**, **cut density K = 0**, memory **T at most 16**.\n\n",
                "<!-- vos30-replan:begin -->\n<!-- vos30-replan:end -->\n\n",
                "## 9. Probe\n\n<!-- vos30-table:begin -->\n<!-- vos30-table:end -->\n"
            )).unwrap();
        let args = |extra: &[&'static str]| -> Vec<String> {
            let mut v: Vec<String> = vec![
                "table".into(),
                "--results".into(),
                results.to_str().unwrap().into(),
                "--spec".into(),
                spec_path.to_str().unwrap().into(),
            ];
            v.extend(extra.iter().map(|s| s.to_string()));
            v
        };
        let go = |a: Vec<String>| {
            let (mut o, mut e) = (Vec::new(), Vec::new());
            let r = dispatch(&a, &mut o, &mut e);
            (
                String::from_utf8(o).unwrap(),
                String::from_utf8(e).unwrap(),
                r,
            )
        };
        let (block, _, r) = go(args(&[]));
        assert!(
            r.is_ok() && block.contains("PROXY:") && block.contains("| A1 "),
            "{block}"
        );
        let (_, e, r) = go(args(&["--check"]));
        assert!(
            matches!(r, Err(Failure::Violations(_))),
            "an empty block is drift: {e}"
        );
        let (_, _, r) = go(args(&["--write"]));
        assert!(r.is_ok());
        let (_, e, r) = go(args(&["--check"]));
        assert!(r.is_ok(), "{e}");
        let written = std::fs::read_to_string(&spec_path).unwrap();
        std::fs::write(&spec_path, written.replace("GREEN", "green")).unwrap();
        let (_, e, r) = go(args(&["--check"]));
        assert!(
            matches!(r, Err(Failure::Violations(_))) && e.contains("drift"),
            "{e}"
        );
        // --write with no markers is a usage error.
        std::fs::write(&spec_path, "no markers\n").unwrap();
        assert!(matches!(go(args(&["--write"])).2, Err(Failure::Usage(_))));
        // a missing results file with --check and no block is fine; with a printed table it is not.
        let mut none = args(&["--check"]);
        none[2] = d.join("missing.json").to_str().unwrap().into();
        assert!(go(none.clone()).2.is_ok());
        none.pop();
        assert!(matches!(go(none).2, Err(Failure::Usage(_))));

        // decide: the quick file lacks the required shapes, so "unclassified is red": row 1.
        let (out, _, r) = run(&["decide", "--results", results.to_str().unwrap()]);
        assert!(r.is_ok());
        assert!(out.starts_with("row 1: Infeasible: re-plan X3"), "{out}");
    }

    #[test]
    fn a_partial_run_is_reported_and_the_quick_gate_passes_on_what_ran() {
        let d = scratch("partial");
        let out_file = d.join("partial.json");
        let (out, err, r) = run(&[
            "run",
            "--tier",
            "quick",
            "--only",
            "A1,E1,B1",
            "--exe",
            EXE,
            "--jobs",
            "3",
            "--out",
            out_file.to_str().unwrap(),
        ]);
        assert!(r.is_ok(), "{err}");
        assert!(out.contains("ran 20 rows"), "{out}");
        let res = Results::from_text(&std::fs::read_to_string(&out_file).unwrap()).unwrap();
        assert!(res
            .rows
            .iter()
            .any(|r| r.experiment == "E1" && r.result == "refused"));
        assert!(res
            .rows
            .iter()
            .filter(|r| r.experiment == "B1")
            .all(|r| r.result == "collapsed"));
    }
}
