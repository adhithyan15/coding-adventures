//! # `cli`: the commands behind `main`
//!
//! Kept in the library so that every command is a function with explicit inputs and writers,
//! which the tests call directly.  `dispatch` returns [`Failure::Usage`] for a malformed command
//! line (exit status 2) and [`Failure::Violations`] when a check found something wrong (exit
//! status 1): the two are never confused, because a gate that exits 1 for a typo would pass a
//! broken build as "the tool is strict".

use crate::exp::{knobs, plan, run_row, Tier, EXPERIMENTS};
use crate::meta;
use crate::results::{self, decide, Merge, Results, MIN_SEEDS, TIMEOUT_S};
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
        Some("merge") => cmd_merge(&args[1..], out),
        Some("row") => cmd_row(&args[1..], out),
        Some(other) => Err(Failure::Usage(format!(
            "unknown command {other}; try list, run, check, table, decide, merge"
        ))),
        None => Err(Failure::Usage(
            "no command; try list, run, check, table, decide, merge".into(),
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
    // The metadata is gathered BEFORE the rows run: `dirty_tree` says whether the tree was clean
    // when the run started (the binary under test was built from it), not whether somebody
    // edited a file while the run was going.
    let meta = meta::gather(&exe);
    let rows = runner::run_rows(&exe, &specs, jobs(args), &progress);
    let results = Results { meta, rows };
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

/// Does this row belong to the refreshed subset: its experiment is `experiment` and, when a
/// `field=value` selector is given, its knob's string field `field` equals `value`?
fn in_subset(r: &Row, experiment: &str, sel: Option<(&str, &str)>) -> bool {
    r.experiment == experiment && sel.is_none_or(|(f, v)| r.knob.s(f) == v)
}

/// The identity of a row across runs: everything but its measurements.
fn row_id(r: &Row) -> (String, String, String, u64) {
    (r.experiment.clone(), r.knob.key(), r.mode.clone(), r.seed)
}

/// The pure part of `merge`: replace, in `base`, exactly the rows of the subset by the matching
/// rows of `fresh`, and record the refresh in `meta.merges`.  Every other row is left untouched
/// (so it stays byte-identical), the order is preserved, and the base `tool`, `host` and
/// `revisions` stay the file's original provenance.  Refused (nothing is produced): a fresh run or
/// a merged result that `validate` rejects; duplicate row ids in either file; fresh rows outside
/// the subset unless `ignore_outside`; a `where` field no base row of the experiment has; subset
/// identities that differ between the files; a fresh `repo` revision that `is_ancestor` does not
/// accept (HEAD or an ancestor of it).
pub fn merge_results(
    base: &Results,
    fresh: &Results,
    experiment: &str,
    sel: Option<(&str, &str)>,
    ignore_outside: bool,
    at: &str,
    is_ancestor: &dyn Fn(&str) -> Result<bool, String>,
) -> Result<Results, String> {
    if fresh.meta.dirty_tree {
        return Err("the fresh run has dirty_tree = true; commit, then run again".into());
    }
    let bad = results::validate(fresh);
    if !bad.is_empty() {
        return Err(format!(
            "the fresh run is invalid ({} violations), first: {}",
            bad.len(),
            bad[0]
        ));
    }
    for (name, f) in [("base", base), ("fresh", fresh)] {
        let mut seen = std::collections::BTreeSet::new();
        for r in &f.rows {
            if !seen.insert(row_id(r)) {
                return Err(format!(
                    "duplicate row id in the {name} file: {}",
                    results::row_text_id(r)
                ));
            }
        }
    }
    if let Some((f, _)) = sel {
        let exists = base
            .rows
            .iter()
            .filter(|r| r.experiment == experiment)
            .any(|r| r.knob.0.iter().any(|(k, _)| k == f));
        if !exists {
            return Err(format!(
                "no base row of {experiment} has a knob field {f} (a typo would select nothing)"
            ));
        }
    }
    let fresh_rows: Vec<&Row> = fresh
        .rows
        .iter()
        .filter(|r| in_subset(r, experiment, sel))
        .collect();
    if !ignore_outside && fresh_rows.len() != fresh.rows.len() {
        return Err(format!(
            "the fresh run has {} rows outside the subset (pass --ignore-outside to drop them)",
            fresh.rows.len() - fresh_rows.len()
        ));
    }
    let mut sorted_base: Vec<_> = base
        .rows
        .iter()
        .filter(|r| in_subset(r, experiment, sel))
        .map(row_id)
        .collect();
    let mut fresh_ids: Vec<_> = fresh_rows.iter().map(|r| row_id(r)).collect();
    sorted_base.sort();
    fresh_ids.sort();
    if sorted_base.is_empty() {
        return Err("the subset selects no row of the base file".into());
    }
    if sorted_base != fresh_ids {
        return Err(format!(
            "the subset has {} rows in the base and {} in the fresh run, or their identities \
             differ: a merge never adds, drops or renames a row",
            sorted_base.len(),
            fresh_ids.len()
        ));
    }
    let rev = fresh
        .meta
        .revisions
        .iter()
        .find(|(k, _)| k == "repo")
        .map(|(_, v)| v.clone())
        .ok_or("the fresh run records no repo revision")?;
    if !is_ancestor(&rev)? {
        return Err(format!(
            "the fresh run's revision {rev} is neither HEAD nor an ancestor of it"
        ));
    }
    let mut rows = Vec::with_capacity(base.rows.len());
    let mut replaced = Vec::new();
    for r in &base.rows {
        if in_subset(r, experiment, sel) {
            let id = row_id(r);
            let f = fresh_rows
                .iter()
                .find(|f| row_id(f) == id)
                .ok_or("internal: a matched identity vanished")?;
            replaced.push(results::row_text_id(r));
            rows.push((*f).clone());
        } else {
            rows.push(r.clone());
        }
    }
    let mut meta = base.meta.clone();
    meta.merges.push(Merge {
        rev,
        binary_sha256: fresh.meta.binary_sha256.clone(),
        dirty_tree: fresh.meta.dirty_tree,
        experiment: experiment.to_string(),
        where_: sel.map(|(f, v)| format!("{f}={v}")).unwrap_or_default(),
        rows: replaced,
        at: at.to_string(),
    });
    let merged = Results { meta, rows };
    let bad = results::validate(&merged);
    if !bad.is_empty() {
        return Err(format!(
            "the merged result is invalid ({} violations), first: {}",
            bad.len(),
            bad[0]
        ));
    }
    Ok(merged)
}

/// Read a results file for `merge`, refusing one over [`results::MAX_FILE_BYTES`].
fn read_capped(path: &Path) -> Result<String, Failure> {
    let len = std::fs::metadata(path)
        .map_err(|e| Failure::Usage(format!("cannot read {}: {e}", path.display())))?
        .len();
    if len > results::MAX_FILE_BYTES {
        return Err(Failure::Usage(format!(
            "{} is {len} bytes, over the {} byte cap",
            path.display(),
            results::MAX_FILE_BYTES
        )));
    }
    read(path)
}

/// Write `text` to a temporary file next to `path`, then rename it over `path`.
fn write_atomic(path: &str, text: &str) -> Result<(), Failure> {
    let target = Path::new(path);
    let tmp = target.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, text)
        .and_then(|_| std::fs::rename(&tmp, target))
        .map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            Failure::Usage(format!("cannot write {path}: {e}"))
        })
}

/// `merge`: refresh a subset of the committed results through the tool, never by hand.
///
/// ```text
/// merge --base results.json --fresh fresh.json --experiment E1 [--where fixture=NAME]
///       [--ignore-outside] --out out.json
/// ```
pub fn cmd_merge(args: &[String], out: &mut dyn Write) -> Result<(), Failure> {
    let need = |n: &str| flag(args, n).ok_or_else(|| Failure::Usage(format!("merge needs {n}")));
    let base =
        Results::from_text(&read_capped(Path::new(&need("--base")?))?).map_err(Failure::Usage)?;
    let fresh =
        Results::from_text(&read_capped(Path::new(&need("--fresh")?))?).map_err(Failure::Usage)?;
    let experiment = need("--experiment")?;
    if !EXPERIMENTS.contains(&experiment.as_str()) {
        return Err(Failure::Usage(format!("unknown experiment {experiment}")));
    }
    let wh = flag(args, "--where");
    let sel = match &wh {
        Some(w) => Some(
            w.split_once('=')
                .ok_or_else(|| Failure::Usage("--where wants field=value".into()))?,
        ),
        None => None,
    };
    let merged = merge_results(
        &base,
        &fresh,
        &experiment,
        sel,
        has(args, "--ignore-outside"),
        &meta::utc_now(),
        &meta::is_ancestor_of_head,
    )
    .map_err(Failure::Usage)?;
    let path = need("--out")?;
    write_atomic(&path, &merged.to_text())?;
    let n = merged.meta.merges.last().map_or(0, |g| g.rows.len());
    writeln!(out, "merged {n} rows of {experiment} into {path}")
        .map_err(|e| Failure::Usage(e.to_string()))
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
