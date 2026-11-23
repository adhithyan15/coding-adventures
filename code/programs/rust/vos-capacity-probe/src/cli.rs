//! # `cli`: the commands behind `main`
//!
//! Kept in the library so that every command is a function with explicit inputs and writers,
//! which the tests call directly.  `dispatch` returns [`Failure::Usage`] for a malformed command
//! line (exit status 2) and [`Failure::Violations`] when a check found something wrong (exit
//! status 1): the two are never confused, because a gate that exits 1 for a typo would pass a
//! broken build as "the tool is strict".

use crate::exp::{knobs, plan, run_row, Tier, EXPERIMENTS};
use crate::join;
use crate::meta;
use crate::results::{self, decide, Merge, Results, MIN_SEEDS, TIMEOUT_S};
use crate::row::Row;
use crate::runner;
use crate::spot;
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

/// `--jobs`, clamped to `1..=64` (VOS-D097 item 5); the default is the host's parallelism, at
/// most 4.
fn jobs(args: &[String]) -> usize {
    runner::clamp_jobs(
        flag(args, "--jobs")
            .and_then(|j| j.parse().ok())
            .unwrap_or_else(|| {
                std::thread::available_parallelism()
                    .map(|n| n.get().min(4))
                    .unwrap_or(2)
            }),
    )
}

/// The executable that runs a row as a child: `--exe`, else this one (a test passes `--exe`
/// because its own executable is the test harness).
fn exe_of(args: &[String]) -> Result<PathBuf, Failure> {
    match flag(args, "--exe") {
        Some(p) => Ok(PathBuf::from(p)),
        None => std::env::current_exe().map_err(|e| Failure::Usage(e.to_string())),
    }
}

/// Read a text file, refusing one over [`results::MAX_FILE_BYTES`] (finding L1: `check`, `table`
/// and `decide` read through this too, not only `merge`).  The size is checked on the open file
/// and the read itself is capped, so a file that grows after the check cannot defeat it.
fn read(path: &Path) -> Result<String, Failure> {
    let cannot = |e: std::io::Error| Failure::Usage(format!("cannot read {}: {e}", path.display()));
    let file = std::fs::File::open(path).map_err(cannot)?;
    let len = file.metadata().map_err(cannot)?.len();
    let over = || {
        Failure::Usage(format!(
            "{} is over the {} byte cap",
            path.display(),
            results::MAX_FILE_BYTES
        ))
    };
    if len > results::MAX_FILE_BYTES {
        return Err(over());
    }
    let mut text = String::new();
    std::io::Read::read_to_string(
        &mut std::io::Read::take(file, results::MAX_FILE_BYTES + 1),
        &mut text,
    )
    .map_err(cannot)?;
    if text.len() as u64 > results::MAX_FILE_BYTES {
        return Err(over());
    }
    Ok(text)
}

/// Run the command named by `args`.
pub fn dispatch(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> Result<(), Failure> {
    match args.first().map(String::as_str) {
        Some("list") => cmd_list(out),
        Some("run") => cmd_run(&args[1..], out, err),
        Some("check") => cmd_check(&args[1..], out, err),
        Some("validate") => cmd_validate(&args[1..], out, err),
        Some("join") => cmd_join(&args[1..], out, err),
        Some("table") => cmd_table(&args[1..], out, err),
        Some("decide") => cmd_decide(&args[1..], out),
        Some("merge") => cmd_merge(&args[1..], out),
        Some("row") => cmd_row(&args[1..], out),
        Some("structure") => cmd_structure(&args[1..], out),
        Some(other) => Err(Failure::Usage(format!(
            "unknown command {other}; try list, run, check, validate, join, table, decide, merge"
        ))),
        None => Err(Failure::Usage(
            "no command; try list, run, check, validate, join, table, decide, merge".into(),
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
    let (run_id, run_attempt) = run_numbers(args)?;
    // `--shard K/N` (VOS30 section 9.2): this process runs one slice of the full plan.  It needs
    // the full tier (a shard of the quick tier is not a thing the plan defines) and refuses
    // `--only` (the slice is of the whole plan, so a subset would silently be a different slice).
    let shard = match value_of(args, "--shard")? {
        Some(s) => Some(crate::exp::parse_shard(&s).map_err(Failure::Usage)?),
        None => None,
    };
    if shard.is_none() && (has(args, "--run-id") || has(args, "--run-attempt")) {
        return Err(Failure::Usage(
            "--run-id and --run-attempt describe a shard: they need --shard".into(),
        ));
    }
    let specs = match shard {
        Some((k, n)) => {
            if tier != Tier::Full {
                return Err(Failure::Usage("--shard needs --tier full".into()));
            }
            if !only.is_empty() {
                return Err(Failure::Usage(
                    "--shard and --only cannot be combined: a shard is a slice of the whole plan"
                        .into(),
                ));
            }
            crate::exp::plan_shard(seeds, k, n)
        }
        None => plan(tier, &only, seeds),
    };
    // Each row is a child process of this same executable.
    let exe = exe_of(args)?;
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
    let mut meta = meta::gather(&exe);
    let jobs = jobs(args);
    if let Some(shard) = shard {
        meta.recording = Some(results::Recording {
            mode: "record".into(),
            run_id,
            run_attempt,
            shard: Some(shard),
            shards: None,
            jobs: u32::try_from(jobs).unwrap_or(u32::MAX),
        });
    }
    let rows = runner::run_rows(&exe, &specs, jobs, &progress);
    let results = Results { meta, rows };
    if let Some(path) = flag(args, "--out") {
        write_atomic(&path, &results.to_text())?;
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
    merge_with(
        MergeKind::Refresh,
        base,
        fresh,
        experiment,
        sel,
        ignore_outside,
        at,
        is_ancestor,
    )
}

/// What a merge does with the fresh rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeKind {
    /// Replace the rows of the subset; a merge never adds, drops or renames a row.
    Refresh,
    /// `merge --add` (VOS30 section 9.2): append fresh rows whose identities are ALL absent from
    /// the base, for a plan change that brings new knobs.
    Add,
}

/// The pure part of `merge --add`: append, to `base`, the rows of `fresh` that belong to the
/// subset, all of whose `(experiment, knob, mode, seed)` identities must be absent from the base;
/// the `merges` entry records `"kind": "add"`.  Every other refusal of [`merge_results`] applies
/// unchanged except the two that only make sense for a refresh: that the subset's identities
/// equal the base's (the point of an add is that they do not), and that a `where` field exists in
/// a base row of the experiment (a new experiment has none).  An overlap with the base is refused.
pub fn add_results(
    base: &Results,
    fresh: &Results,
    experiment: &str,
    sel: Option<(&str, &str)>,
    ignore_outside: bool,
    at: &str,
    is_ancestor: &dyn Fn(&str) -> Result<bool, String>,
) -> Result<Results, String> {
    merge_with(
        MergeKind::Add,
        base,
        fresh,
        experiment,
        sel,
        ignore_outside,
        at,
        is_ancestor,
    )
}

#[allow(clippy::too_many_arguments)]
fn merge_with(
    kind: MergeKind,
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
    if let (MergeKind::Refresh, Some((f, _))) = (kind, sel) {
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
    if kind == MergeKind::Refresh {
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
    } else {
        if fresh_rows.is_empty() {
            return Err("the subset selects no row of the fresh run: nothing to add".into());
        }
        let have: std::collections::BTreeSet<_> = base.rows.iter().map(row_id).collect();
        if let Some(r) = fresh_rows.iter().find(|r| have.contains(&row_id(r))) {
            return Err(format!(
                "merge --add refuses an overlap with the base: {} is already there (a refresh is \
                 a merge without --add)",
                results::row_text_id(r)
            ));
        }
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
    let mut rows = Vec::with_capacity(base.rows.len() + fresh_rows.len());
    let mut replaced = Vec::new();
    if kind == MergeKind::Add {
        rows.extend(base.rows.iter().cloned());
        for f in &fresh_rows {
            replaced.push(results::row_text_id(f));
            rows.push((*f).clone());
        }
    } else {
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
        kind: (kind == MergeKind::Add).then(|| "add".to_string()),
        trees: fresh.meta.trees.clone(),
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

/// The most bytes of the target's file name kept in the temporary name (round 2, finding 8): the
/// temporary is `.STEM.PID.N.NANOS.tmp`, at most about 60 bytes besides the stem, and a file name
/// is limited to 255 bytes, so a long target name must not make the temporary name too long to
/// create.
pub const TMP_STEM_MAX: usize = 150;

/// The target's file name cut to at most [`TMP_STEM_MAX`] bytes on a character boundary.
pub fn tmp_name_stem(name: &str) -> String {
    let mut end = name.len().min(TMP_STEM_MAX);
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    name[..end].to_string()
}

/// Distinguishes the temporary files of one process.
static TMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Write `text` to `path` atomically (finding L2): a temporary file in the SAME directory,
/// opened `create_new` (it never follows a pre-planted file or link; mode 0600 on unix), named
/// with the process id, a counter and the clock so it cannot be guessed, written, `fsync`ed and
/// renamed over `path`, and the directory `fsync`ed.  A `path` that is a symlink is refused:
/// the rename would replace the link, not write through it, and an `--out` that is a link is
/// never what the caller meant.  The finished file keeps the permission bits (`mode & 0o777`: the
/// setuid, setgid and sticky bits are dropped, round 2 finding 8) of the file it replaces, or 0644.
pub fn write_atomic(path: impl AsRef<Path>, text: &str) -> Result<(), Failure> {
    let target = path.as_ref();
    let fail =
        |e: std::io::Error| Failure::Usage(format!("cannot write {}: {e}", target.display()));
    if std::fs::symlink_metadata(target).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(Failure::Usage(format!(
            "refusing to write {}: it is a symbolic link",
            target.display()
        )));
    }
    let dir = match target.parent() {
        Some(d) if !d.as_os_str().is_empty() => d.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let name = target
        .file_name()
        .ok_or_else(|| Failure::Usage(format!("{} has no file name", target.display())))?
        .to_string_lossy()
        .into_owned();
    let name = tmp_name_stem(&name);
    #[cfg(unix)]
    let mode = {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(target).map_or(0o644, |m| m.permissions().mode() & 0o777)
    };
    for _ in 0..32 {
        let n = TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos());
        let tmp = dir.join(format!(".{name}.{}.{n}.{nanos:x}.tmp", std::process::id()));
        let mut open = std::fs::OpenOptions::new();
        open.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut open, 0o600);
        let mut file = match open.open(&tmp) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(fail(e)),
        };
        let done = (|| {
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(std::fs::Permissions::from_mode(mode))?;
            }
            drop(file);
            std::fs::rename(&tmp, target)
        })();
        return match done {
            Ok(()) => {
                // Best effort: make the rename durable.
                if let Ok(d) = std::fs::File::open(&dir) {
                    let _ = d.sync_all();
                }
                Ok(())
            }
            Err(e) => {
                let _ = std::fs::remove_file(&tmp);
                Err(fail(e))
            }
        };
    }
    Err(Failure::Usage(format!(
        "cannot write {}: no unused temporary file name in {}",
        target.display(),
        dir.display()
    )))
}

/// `merge`: refresh a subset of the committed results through the tool, never by hand.
///
/// ```text
/// merge [--add] --base results.json --fresh fresh.json --experiment E1 [--where fixture=NAME]
///       [--ignore-outside] --out out.json
/// ```
pub fn cmd_merge(args: &[String], out: &mut dyn Write) -> Result<(), Failure> {
    let need = |n: &str| flag(args, n).ok_or_else(|| Failure::Usage(format!("merge needs {n}")));
    let base = Results::from_canonical_text(&read(Path::new(&need("--base")?))?)
        .map_err(Failure::Usage)?;
    let fresh = Results::from_canonical_text(&read(Path::new(&need("--fresh")?))?)
        .map_err(Failure::Usage)?;
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
    let merge = if has(args, "--add") {
        add_results
    } else {
        merge_results
    };
    let merged = merge(
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
    let verb = if has(args, "--add") {
        "added"
    } else {
        "merged"
    };
    writeln!(out, "{verb} {n} rows of {experiment} into {path}")
        .map_err(|e| Failure::Usage(e.to_string()))
}

/// Refuse any `--flag` of `args` that is not in `allowed`.  A mistyped `--expect-run-ids` would
/// otherwise be ignored and the gate it names silently skipped, so `join` (the integrity gate)
/// accepts exactly the flags it knows.
fn refuse_unknown_flags(cmd: &str, args: &[String], allowed: &[&str]) -> Result<(), Failure> {
    match args
        .iter()
        .find(|a| a.starts_with("--") && !allowed.contains(&a.as_str()))
    {
        Some(a) => Err(Failure::Usage(format!("{cmd}: unknown flag {a}"))),
        None => Ok(()),
    }
}

/// The value of `name` when present: it must have a value that is not another flag.
fn value_of(args: &[String], name: &str) -> Result<Option<String>, Failure> {
    match args.iter().position(|a| a == name) {
        None => Ok(None),
        Some(i) => match args.get(i + 1) {
            Some(v) if !v.starts_with("--") => Ok(Some(v.clone())),
            _ => Err(Failure::Usage(format!("{name} needs a value"))),
        },
    }
}

/// `--run-id` and `--run-attempt` (ASCII digits, 1 to 20; both or neither).  Explicit flags,
/// never the environment: a value that the process picked up on its own would be a value nobody
/// typed.
fn run_numbers(args: &[String]) -> Result<(Option<String>, Option<String>), Failure> {
    let id = value_of(args, "--run-id")?;
    let attempt = value_of(args, "--run-attempt")?;
    for (flag, v) in [("--run-id", &id), ("--run-attempt", &attempt)] {
        if v.as_deref().is_some_and(|s| !results::is_run_number(s)) {
            return Err(Failure::Usage(format!(
                "{flag} must be 1 to 20 ASCII digits"
            )));
        }
    }
    if id.is_some() != attempt.is_some() {
        return Err(Failure::Usage(
            "--run-id and --run-attempt go together (a local run gives neither)".into(),
        ));
    }
    Ok((id, attempt))
}

/// `join --out J [--expect-binary-sha256 H] [--expect-run-id I] [--expect-run-attempt A]
/// [--expect-shards N] [--expect-jobs J] SHARD.json...` (VOS30 section 9.2).
///
/// Every refusal is `Failure::Usage` (exit 2) and nothing is written: the joined text is built
/// and re-parsed in memory first, and only then written (atomically) to `--out`.
pub fn cmd_join(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> Result<(), Failure> {
    refuse_unknown_flags(
        "join",
        args,
        &[
            "--out",
            "--expect-binary-sha256",
            "--expect-run-id",
            "--expect-run-attempt",
            "--expect-shards",
            "--expect-jobs",
        ],
    )?;
    let number = |name: &str| -> Result<Option<u32>, Failure> {
        value_of(args, name)?
            .map(|v| {
                (!v.is_empty() && v.len() <= 5 && v.bytes().all(|b| b.is_ascii_digit()))
                    .then(|| v.parse::<u32>().ok())
                    .flatten()
                    .ok_or_else(|| {
                        Failure::Usage(format!("{name} must be a small decimal integer"))
                    })
            })
            .transpose()
    };
    let expect = join::Expect {
        binary_sha256: value_of(args, "--expect-binary-sha256")?,
        run_id: value_of(args, "--expect-run-id")?,
        run_attempt: value_of(args, "--expect-run-attempt")?,
        shards: number("--expect-shards")?,
        jobs: number("--expect-jobs")?,
    };
    let target =
        value_of(args, "--out")?.ok_or_else(|| Failure::Usage("join needs --out FILE".into()))?;
    let files = positionals(args);
    if files.is_empty() {
        return Err(Failure::Usage("join needs at least one shard file".into()));
    }
    let mut inputs = Vec::new();
    for f in files {
        let res = Results::from_canonical_text(&read(Path::new(f))?)
            .map_err(|e| Failure::Usage(format!("{f}: {e}")))?;
        inputs.push((f.clone(), res));
    }
    let joined = join::join(&inputs, &expect).map_err(Failure::Usage)?;
    let text = joined.results.to_text();
    // The file we are about to write must be one the tool would itself accept back.
    Results::from_canonical_text(&text).map_err(|e| {
        Failure::Usage(format!(
            "join: the joined file is not canonical (a bug): {e}"
        ))
    })?;
    write_atomic(&target, &text)?;
    for m in &joined.findings {
        let _ = writeln!(err, "join: finding: {m}");
    }
    writeln!(
        out,
        "joined {} shards into {} rows ({} row finding(s)) at {target}",
        inputs.len(),
        joined.results.rows.len(),
        joined.findings.len()
    )
    .map_err(|e| Failure::Usage(e.to_string()))
}

/// The positional arguments of a command: those that are neither a flag nor the value of a flag
/// that takes one.
fn positionals(args: &[String]) -> Vec<&String> {
    const WITH_VALUE: [&str; 13] = [
        "--jobs",
        "--exe",
        "--spot-budget",
        "--struct-budget",
        "--out",
        "--shard",
        "--run-id",
        "--run-attempt",
        "--expect-binary-sha256",
        "--expect-run-id",
        "--expect-run-attempt",
        "--expect-shards",
        "--expect-jobs",
    ];
    args.iter()
        .enumerate()
        .filter(|(i, a)| {
            !a.starts_with("--") && (*i == 0 || !WITH_VALUE.contains(&args[i - 1].as_str()))
        })
        .map(|(_, a)| a)
        .collect()
}

/// Why `check --full` refuses, given the file and the working tree's ids: a file that records no
/// tree ids cannot say which tree it describes, and a recorded id that differs says it no longer
/// describes this one.  Both must be fixed by regenerating the rows, never by editing the ids.
pub fn full_refusals(res: &Results, current: &[(String, String)]) -> Vec<String> {
    let mut v = Vec::new();
    if res.meta.trees.is_empty() {
        v.push(
            "check --full refuses: the file records no tree ids (it predates 0.2.0), so it cannot \
             say which tree it describes; regenerate the rows"
                .to_string(),
        );
    }
    let mut records = vec![("tool".to_string(), &res.meta.trees)];
    for (i, g) in res.meta.merges.iter().enumerate() {
        records.push((format!("merges[{i}]"), &g.trees));
    }
    for (what, trees) in records {
        if what != "tool" && trees.is_empty() {
            v.push(format!(
                "check --full refuses: {what} records no tree ids; regenerate its rows"
            ));
        }
        for line in meta::stale_lines(&what, trees, current) {
            v.push(format!(
                "check --full refuses ({}); regenerate the rows",
                line.trim_start_matches("stale: ")
            ));
        }
    }
    v
}

/// `--spot-budget SECS`, the spot check's total time (default [`spot::SPOT_BUDGET`], at most
/// 600 s).  A shorter one is for tests and local runs; the floor of verified rows still applies.
fn spot_budget(args: &[String]) -> Duration {
    flag(args, "--spot-budget")
        .and_then(|s| s.parse::<u64>().ok())
        .map_or(spot::SPOT_BUDGET, |n| Duration::from_secs(n.clamp(1, 600)))
}

/// `--struct-budget SECS`, the structural rebuild's total time (default [`spot::STRUCT_BUDGET`], at
/// most 600 s); a shorter one is for tests and local runs.
fn struct_budget(args: &[String]) -> Duration {
    flag(args, "--struct-budget")
        .and_then(|s| s.parse::<u64>().ok())
        .map_or(spot::STRUCT_BUDGET, |n| {
            Duration::from_secs(n.clamp(1, 600))
        })
}

/// `check [--full] FILE [--jobs N] [--exe PATH] [--spot-budget SECS] [--struct-budget SECS]`.
///
/// The order matters (finding M2): everything that needs no instance is checked first, and a file
/// that fails any of it returns at once, before a single row is re-run, in process or in a child.
/// A hand-edited knob can therefore never reach an instance builder.
pub fn cmd_check(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> Result<(), Failure> {
    let path = positionals(args)
        .first()
        .copied()
        .ok_or_else(|| Failure::Usage("check needs a results file".into()))?;
    let res = Results::from_canonical_text(&read(Path::new(path))?).map_err(Failure::Usage)?;
    let w = |e: std::io::Error| Failure::Usage(e.to_string());
    let mut bad = results::validate(&res);
    bad.extend(results::quick_plan_covered(&res));
    bad.extend(results::full_plan_covered(&res));
    let current = meta::working_trees();
    // Non-fatal: a fatal rule would couple every edit of a solver crate to a regeneration of the
    // results file (the VOS-D103 failure mode).  The teeth are the spot check and `--full`.
    let mut stale = meta::stale_lines("tool", &res.meta.trees, &current);
    for (i, g) in res.meta.merges.iter().enumerate() {
        stale.extend(meta::stale_lines(
            &format!("merges[{i}]"),
            &g.trees,
            &current,
        ));
    }
    for line in &stale {
        writeln!(out, "{line}").map_err(w)?;
    }
    let full = has(args, "--full");
    if full {
        bad.extend(full_refusals(&res, &current));
    }
    let tail = if !bad.is_empty() {
        // Nothing is re-run: the file is already refused.
        String::new()
    } else if full {
        let (quick_bad, mut rerun_rows) = results::rerun_collect(&res, jobs(args), true);
        bad.extend(quick_bad);
        let exe = exe_of(args)?;
        let progress = |n: usize, total: usize| {
            if n.is_multiple_of(25) || n == total {
                eprintln!("[full {n}/{total}]");
            }
        };
        let (full_bad, full_rows) = spot::full_rerun_collect(&res, &exe, jobs(args), &progress);
        bad.extend(full_bad);
        rerun_rows.extend(full_rows);
        // Round 2, finding 3: the decision the re-run rows give must be the recorded one.
        let again: Vec<Row> = res
            .rows
            .iter()
            .map(|r| {
                rerun_rows
                    .get(&results::row_text_id(r))
                    .cloned()
                    .unwrap_or_else(|| r.clone())
            })
            .collect();
        bad.extend(results::decision_differences(&res.rows, &again));
        format!(
            "{} rows; every row re-run (quick in process, full in a child) and reproduced, \
             timings within the band, and the decision equal on the re-run rows",
            res.rows.len()
        )
    } else {
        let quick = results::quick_specs(&res).len();
        bad.extend(results::rerun(&res, jobs(args)));
        // Only a file with something to sample needs an executable to sample with.
        let exe = exe_of(args)?;
        let report = if spot::population(&res).is_empty() {
            spot::SpotReport::default()
        } else {
            spot::spot_check(&res, &exe, jobs(args), spot_budget(args))
        };
        bad.extend(report.violations.iter().cloned());
        writeln!(out, "check: {}", report.line()).map_err(w)?;
        // Round 2, finding 1: every full-tier row has its instance rebuilt (not solved) and its
        // digest, size and entropy compared; what the plan classes too slow is counted.
        let structure = spot::structure_check(
            &res,
            &exe,
            jobs(args),
            struct_budget(args),
            spot::STRUCT_SLACK,
            &report.verified_rows,
        );
        bad.extend(structure.violations.iter().cloned());
        writeln!(out, "check: {}", structure.line()).map_err(w)?;
        let unsolved = results::decision_groups_not_resolved(&res.rows, &report.verified_rows);
        writeln!(
            out,
            "check: decision-critical groups not re-solved by this run: {} (their solver and \
             timing fields are verified only by `check --full`, the release gate for the \
             decision)",
            unsolved.len()
        )
        .map_err(w)?;
        format!(
            "{} rows, {quick} quick-tier rows re-run and reproduced",
            res.rows.len()
        )
    };
    for b in &bad {
        let _ = writeln!(err, "check: {b}");
    }
    if bad.is_empty() {
        writeln!(out, "check: ok ({tail})").map_err(w)?;
        Ok(())
    } else {
        Err(Failure::Violations(bad.len()))
    }
}

/// `validate FILE` (VOS-D130; VOS30 9.12.1): the schema and guard rules of `check`, and nothing
/// else, with the findings told apart by class.  It re-runs nothing and needs no executable,
/// so a recording's reviewer can classify it without trusting the machine that made it.
///
/// ```text
///   exit 2   structural finding, or the file cannot be read or is not a canonical results file
///            (the recording is discarded, VOS30 9.12.1 class 1)
///   exit 1   row findings only (the recording is kept and blocks commit B, class 2)
///   exit 0   clean (class 3)
/// ```
///
/// Exit 2 is `Failure::Usage` and exit 1 is `Failure::Violations`, as everywhere in this tool.
/// Plan coverage is NOT checked: a shard's file legitimately holds only its share of the plan.
pub fn cmd_validate(
    args: &[String],
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), Failure> {
    let pos = positionals(args);
    if pos.len() > 1 {
        return Err(Failure::Usage(
            "validate takes exactly one results file".into(),
        ));
    }
    let path = pos
        .first()
        .copied()
        .ok_or_else(|| Failure::Usage("validate needs a results file".into()))?;
    let res = Results::from_canonical_text(&read(Path::new(path))?).map_err(Failure::Usage)?;
    let w = |e: std::io::Error| Failure::Usage(e.to_string());
    let found = results::validate_classes(&res);
    for m in &found.structural {
        let _ = writeln!(err, "validate: structural: {m}");
    }
    for m in &found.rows {
        let _ = writeln!(err, "validate: row: {m}");
    }
    if !found.structural.is_empty() {
        return Err(Failure::Usage(format!(
            "{} structural finding(s) and {} row finding(s): the recording is structurally invalid",
            found.structural.len(),
            found.rows.len()
        )));
    }
    if !found.rows.is_empty() {
        return Err(Failure::Violations(found.rows.len()));
    }
    writeln!(out, "validate: ok ({} rows)", res.rows.len()).map_err(w)?;
    Ok(())
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
        Some(Results::from_canonical_text(&read(&rp)?).map_err(Failure::Usage)?)
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
        write_atomic(&sp, &new)?;
    } else {
        write!(out, "{block}").map_err(|e| Failure::Usage(e.to_string()))?;
    }
    Ok(())
}

/// `decide`.
pub fn cmd_decide(args: &[String], out: &mut dyn Write) -> Result<(), Failure> {
    let (rp, _) = paths(args);
    let res = Results::from_canonical_text(&read(&rp)?).map_err(Failure::Usage)?;
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
    // The child never gets more than the chain budget, whatever the command line says.
    let wall: u64 = flag(args, "--wall")
        .and_then(|s| s.parse().ok())
        .unwrap_or(TIMEOUT_S)
        .min(crate::exp::CHAIN_WALL_S);
    let mut progress = |r: &Row| {
        let _ = writeln!(out_sink(), "{}", runner::progress_line(r));
    };
    let row = run_row(&spec, Duration::from_secs(wall), &mut progress);
    writeln!(out, "{}", runner::row_line(&row)).map_err(|e| Failure::Usage(e.to_string()))?;
    out.flush().map_err(|e| Failure::Usage(e.to_string()))?;
    Ok(())
}

/// `structure`: the child process of `check`'s structural rebuild (VOS-028c2 round 2, finding
/// 1(b)).  It builds the row's instance and prints a row carrying only the deterministic
/// structural fields (`exp::structure_of`), never solving.  An enumeration row has no instance,
/// and it is cheap, so it is simply run (its deterministic metrics are the structure).
pub fn cmd_structure(args: &[String], out: &mut dyn Write) -> Result<(), Failure> {
    let spec = runner::spec_from_json(
        &flag(args, "--spec").ok_or_else(|| Failure::Usage("structure needs --spec".into()))?,
    )
    .map_err(Failure::Usage)?;
    let row = if spec.experiment.starts_with('E') {
        run_row(&spec, Duration::from_secs(TIMEOUT_S), &mut |_| {})
    } else {
        let s = crate::exp::structure_of(&spec).map_err(Failure::Usage)?;
        let mut r = Row::new(
            &spec.experiment,
            spec.knob.clone(),
            &spec.mode,
            spec.seed,
            spec.seeds,
        );
        // Not a verdict: a row that is not `crashed` (the blank row's word) means "built",
        // except that a rebuilt instance that collapses says so, with the reason or the note the
        // solving path would write (round 3, finding 1: a recorded `collapsed` or `unsat` row is
        // held to it).
        r.result = "unknown".into();
        if s.collapse_reason.is_some() || !s.collapse_note.is_empty() {
            r.result = "collapsed".into();
            r.collapsed_reason = s.collapse_reason;
            r.note = s.collapse_note;
        }
        r.instance_sha256 = s.instance_sha256;
        r.ands = s.ands;
        r.vars = s.vars;
        r.clauses = s.clauses;
        r.entropy_bits = s.entropy_bits;
        r.declared_bits = s.declared_bits;
        r
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    fn a(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn jobs_default_is_small_and_an_explicit_value_is_clamped() {
        assert_eq!(jobs(&a(&["--jobs", "0"])), 1);
        assert_eq!(jobs(&a(&["--jobs", "8"])), 8);
        assert_eq!(jobs(&a(&["--jobs", "100000"])), 64);
        assert!((1..=4).contains(&jobs(&a(&["--jobs", "junk"]))));
        assert!((1..=4).contains(&jobs(&a(&[]))));
    }

    #[test]
    fn positionals_skip_flags_and_the_values_of_flags_that_take_one() {
        let args = a(&["--jobs", "4", "f.json", "--full", "--exe", "/x/probe"]);
        assert_eq!(positionals(&args), vec!["f.json"]);
        assert_eq!(positionals(&a(&["f.json"])), vec!["f.json"]);
        assert!(positionals(&a(&["--full"])).is_empty());
    }

    #[test]
    fn exe_defaults_to_this_executable() {
        assert_eq!(exe_of(&a(&["--exe", "/p"])).unwrap(), PathBuf::from("/p"));
        assert!(exe_of(&a(&[])).is_ok());
    }
}
