//! # `cli`: the commands behind `main`
//!
//! Kept in the library so that every command is a function with explicit inputs and writers,
//! which the tests call directly.  `dispatch` returns [`Failure::Usage`] for a malformed command
//! line (exit status 2) and [`Failure::Violations`] when a check found something wrong (exit
//! status 1): the two are never confused, because a gate that exits 1 for a typo would pass a
//! broken build as "the tool is strict".

use crate::exp::{knobs, plan, run_row, Tier, EXPERIMENTS};
use crate::meta;
use crate::results::{self, decide, Results, MIN_SEEDS, TIMEOUT_S};
use crate::row::Row;
use crate::runner;
use crate::table;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Why a command failed.
#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    /// The command line is malformed or a file could not be read.
    Usage(String),
    /// The command ran and found this many violations.
    Violations(usize),
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn has(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn jobs(args: &[String]) -> usize {
    flag(args, "--jobs")
        .and_then(|j| j.parse().ok())
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get().min(4))
                .unwrap_or(2)
        })
}

fn read(path: &Path) -> Result<String, Failure> {
    std::fs::read_to_string(path)
        .map_err(|e| Failure::Usage(format!("cannot read {}: {e}", path.display())))
}

/// Run the command named by `args`.
pub fn dispatch(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> Result<(), Failure> {
    match args.first().map(String::as_str) {
        Some("list") => cmd_list(out),
        Some("run") => cmd_run(&args[1..], out, err),
        Some("check") => cmd_check(&args[1..], out, err),
        Some("table") => cmd_table(&args[1..], out, err),
        Some("decide") => cmd_decide(&args[1..], out),
        Some("row") => cmd_row(&args[1..], out),
        Some(other) => Err(Failure::Usage(format!(
            "unknown command {other}; try list, run, check, table, decide"
        ))),
        None => Err(Failure::Usage(
            "no command; try list, run, check, table, decide".into(),
        )),
    }
}

/// `list`: every experiment, its knobs per tier and its guards.
pub fn cmd_list(out: &mut dyn Write) -> Result<(), Failure> {
    let w = |e: std::io::Error| Failure::Usage(e.to_string());
    writeln!(out, "experiment  knobs(full)  knobs(quick)  guards").map_err(w)?;
    for exp in EXPERIMENTS {
        let ks = knobs(exp);
        let quick = ks.iter().filter(|k| k.quick).count();
        let guards = match exp {
            "E1" | "E2" => "G3 (control: a broken adapter / a refused width)",
            "B1" => "G1 (must collapse), G2, G3, G4",
            _ => "G1, G2, G3, G4",
        };
        writeln!(out, "{exp:<10}  {:>11}  {:>12}  {guards}", ks.len(), quick).map_err(w)?;
    }
    writeln!(
        out,
        "seeds per knob: {MIN_SEEDS}; solver timeout {TIMEOUT_S} s; see VOS30 section 9"
    )
    .map_err(w)?;
    Ok(())
}

/// The BUILD gate of VOS30 section 9.3 applied to the quick tier's rows.
pub fn quick_gate(rows: &[Row]) -> Vec<String> {
    let mut v = Vec::new();
    for r in rows {
        let id = format!(
            "{} {} {} seed {}",
            r.experiment,
            r.knob.key(),
            r.mode,
            r.seed
        );
        let n11 = r.experiment == "E1" && r.knob.s("claim") == "insert_n11";
        match r.result.as_str() {
            "refused" if n11 => {}
            "refused" => v.push(format!("{id}: refused")),
            "unknown" | "killed" | "oom" | "crashed" | "sat" | "fail" => {
                v.push(format!("{id}: {} ({})", r.result, r.note))
            }
            _ => {}
        }
        if n11 && r.result != "refused" {
            v.push(format!(
                "{id}: the N = 11 row must be refused with W0706, not {}",
                r.result
            ));
        }
        if r.experiment == "B1" && r.result != "collapsed" {
            v.push(format!(
                "{id}: an identical sequence must report collapsed (G1), not {}",
                r.result
            ));
        }
        if matches!(
            r.result.as_str(),
            "unsat" | "collapsed" | "pass" | "refused"
        ) && !r.control.ok()
        {
            v.push(format!(
                "{id}: the G3 control did not pass ({:?})",
                r.control
            ));
        }
    }
    v
}

/// `run`.
pub fn cmd_run(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> Result<(), Failure> {
    let tier = match flag(args, "--tier").as_deref() {
        Some("quick") => Tier::Quick,
        Some("full") => Tier::Full,
        _ => {
            return Err(Failure::Usage(
                "run needs --tier quick or --tier full".into(),
            ))
        }
    };
    let only: Vec<String> = flag(args, "--only")
        .map(|s| s.split(',').map(str::to_string).collect())
        .unwrap_or_default();
    let seeds: u32 = flag(args, "--seeds")
        .and_then(|s| s.parse().ok())
        .unwrap_or(MIN_SEEDS);
    if seeds < MIN_SEEDS {
        return Err(Failure::Usage(format!(
            "at least {MIN_SEEDS} seeds per knob (VOS30 section 8.3)"
        )));
    }
    for o in &only {
        if !EXPERIMENTS.contains(&o.as_str()) {
            return Err(Failure::Usage(format!("unknown experiment {o} in --only")));
        }
    }
    let specs = plan(tier, &only, seeds);
    // Each row is a child process of this same executable (a test passes `--exe` because its own
    // executable is the test harness).
    let exe = match flag(args, "--exe") {
        Some(p) => PathBuf::from(p),
        None => std::env::current_exe().map_err(|e| Failure::Usage(e.to_string()))?,
    };
    let total = specs.len();
    let done = std::sync::atomic::AtomicUsize::new(0);
    let progress = |_: usize, r: &Row| {
        let n = done.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        eprintln!(
            "[{n}/{total}] {} {} {} seed {}: {}",
            r.experiment,
            r.knob.key(),
            r.mode,
            r.seed,
            r.result
        );
    };
    let rows = runner::run_rows(&exe, &specs, jobs(args), &progress);
    let results = Results {
        meta: meta::gather(&exe),
        rows,
    };
    if let Some(path) = flag(args, "--out") {
        std::fs::write(&path, results.to_text())
            .map_err(|e| Failure::Usage(format!("cannot write {path}: {e}")))?;
    }
    let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
    for r in &results.rows {
        *counts.entry(r.result.clone()).or_default() += 1;
    }
    writeln!(out, "ran {total} rows: {counts:?}").map_err(|e| Failure::Usage(e.to_string()))?;
    if tier == Tier::Quick {
        let bad = quick_gate(&results.rows);
        for b in &bad {
            let _ = writeln!(err, "gate: {b}");
        }
        if !bad.is_empty() {
            return Err(Failure::Violations(bad.len()));
        }
    }
    Ok(())
}

/// `check`.
pub fn cmd_check(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> Result<(), Failure> {
    let path = args
        .iter()
        .find(|a| {
            !a.starts_with("--")
                && args
                    .iter()
                    .position(|b| b == *a)
                    .map(|i| i == 0 || args[i - 1] != "--jobs")
                    .unwrap_or(true)
        })
        .ok_or_else(|| Failure::Usage("check needs a results file".into()))?;
    let res = Results::from_text(&read(Path::new(path))?).map_err(Failure::Usage)?;
    let mut bad = results::validate(&res);
    bad.extend(results::quick_plan_covered(&res));
    let quick = results::quick_specs(&res).len();
    bad.extend(results::rerun(&res, jobs(args)));
    for b in &bad {
        let _ = writeln!(err, "check: {b}");
    }
    if bad.is_empty() {
        writeln!(
            out,
            "check: ok ({} rows, {quick} quick-tier rows re-run and reproduced)",
            res.rows.len()
        )
        .map_err(|e| Failure::Usage(e.to_string()))?;
        Ok(())
    } else {
        Err(Failure::Violations(bad.len()))
    }
}

fn paths(args: &[String]) -> (PathBuf, PathBuf) {
    (
        flag(args, "--results")
            .map(PathBuf::from)
            .unwrap_or_else(meta::default_results),
        flag(args, "--spec")
            .map(PathBuf::from)
            .unwrap_or_else(meta::default_spec),
    )
}

/// `table`.
pub fn cmd_table(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> Result<(), Failure> {
    let (rp, sp) = paths(args);
    let spec = read(&sp)?;
    let res = if rp.exists() {
        Some(Results::from_text(&read(&rp)?).map_err(Failure::Usage)?)
    } else {
        None
    };
    if has(args, "--check") {
        let bad = table::check_spec(&spec, res.as_ref());
        for b in &bad {
            let _ = writeln!(err, "table: {b}");
        }
        return if bad.is_empty() {
            Ok(())
        } else {
            Err(Failure::Violations(bad.len()))
        };
    }
    let res = res.ok_or_else(|| Failure::Usage(format!("no results file at {}", rp.display())))?;
    let block = table::render_block(&res);
    if has(args, "--write") {
        let new = table::splice(&spec, &block)
            .ok_or_else(|| Failure::Usage("the spec has no marker pair to write between".into()))?;
        std::fs::write(&sp, new).map_err(|e| Failure::Usage(e.to_string()))?;
    } else {
        write!(out, "{block}").map_err(|e| Failure::Usage(e.to_string()))?;
    }
    Ok(())
}

/// `decide`.
pub fn cmd_decide(args: &[String], out: &mut dyn Write) -> Result<(), Failure> {
    let (rp, _) = paths(args);
    let res = Results::from_text(&read(&rp)?).map_err(Failure::Usage)?;
    let d = decide(&res.rows);
    let w = |e: std::io::Error| Failure::Usage(e.to_string());
    writeln!(out, "row {}: {}", d.row, d.title).map_err(w)?;
    for b in &d.because {
        writeln!(out, "  {b}").map_err(w)?;
    }
    Ok(())
}

/// `row`: the child process of `run`.  Prints a progress line after the build and the answer.
pub fn cmd_row(args: &[String], out: &mut dyn Write) -> Result<(), Failure> {
    let spec = runner::spec_from_json(
        &flag(args, "--spec").ok_or_else(|| Failure::Usage("row needs --spec".into()))?,
    )
    .map_err(Failure::Usage)?;
    let wall: u64 = flag(args, "--wall")
        .and_then(|s| s.parse().ok())
        .unwrap_or(TIMEOUT_S);
    let mut progress = |r: &Row| {
        let _ = writeln!(out_sink(), "{}", runner::progress_line(r));
    };
    let row = run_row(&spec, Duration::from_secs(wall), &mut progress);
    writeln!(out, "{}", runner::row_line(&row)).map_err(|e| Failure::Usage(e.to_string()))?;
    out.flush().map_err(|e| Failure::Usage(e.to_string()))?;
    Ok(())
}

/// A fresh handle on stdout, flushed per line so a killed child still leaves its progress.
fn out_sink() -> impl Write {
    struct Flushing(std::io::Stdout);
    impl Write for Flushing {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            let n = self.0.write(b)?;
            self.0.flush()?;
            Ok(n)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.0.flush()
        }
    }
    Flushing(std::io::stdout())
}
