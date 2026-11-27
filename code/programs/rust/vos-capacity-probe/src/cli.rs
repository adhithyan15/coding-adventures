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
use crate::sign;
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
        Some("sign") => cmd_sign(&args[1..], out),
        Some("decision-diff") => cmd_decision_diff(&args[1..], out, err),
        Some("table") => cmd_table(&args[1..], out, err),
        Some("decide") => cmd_decide(&args[1..], out),
        Some("merge") => cmd_merge(&args[1..], out),
        Some("row") => cmd_row(&args[1..], out),
        Some("structure") => cmd_structure(&args[1..], out),
        Some(other) => Err(Failure::Usage(format!(
            "unknown command {other}; try list, run, check, validate, join, sign, decision-diff, table, decide, merge"
        ))),
        None => Err(Failure::Usage(
            "no command; try list, run, check, validate, join, sign, decision-diff, table, decide, merge".into(),
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
            verifies_sha256: None,
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
    // A merge makes a new recording of the rows, so a banner (which names the digest of the file
    // as it was signed) no longer describes it: it is cleared and the file must be signed again.
    meta.regeneration = None;
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
    publish(path.as_ref(), text, false)
}

/// Like [`write_atomic`], but the target is NEVER replaced: it is published with a hard link
/// (`std::fs::hard_link`), which fails if the target exists on NTFS and on unix alike, where a
/// rename would silently replace it.  An early `exists()` check alone leaves a window (a file
/// created between the check and the publish would be overwritten: a time-of-check to
/// time-of-use race); this closes it, because the "does it exist" test and the publish are one
/// system call.  Used for the files that are evidence: `join --out` and `check --rerun-out`.
pub fn write_new(path: impl AsRef<Path>, text: &str) -> Result<(), Failure> {
    publish(path.as_ref(), text, true)
}

/// The one implementation behind [`write_atomic`] (`new_only == false`: rename over the target)
/// and [`write_new`] (`new_only == true`: hard-link, then drop the temporary name).
fn publish(target: &Path, text: &str, new_only: bool) -> Result<(), Failure> {
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
            if new_only {
                // Fails with `AlreadyExists` if `target` is there; the temporary name is
                // removed either way, so the data lives under exactly one name.
                let linked = std::fs::hard_link(&tmp, target);
                let _ = std::fs::remove_file(&tmp);
                linked
            } else {
                std::fs::rename(&tmp, target)
            }
        })();
        return match done {
            Err(e) if new_only && e.kind() == std::io::ErrorKind::AlreadyExists => {
                let _ = std::fs::remove_file(&tmp);
                Err(Failure::Usage(format!(
                    "refusing to write {}: it already exists (it is never overwritten)",
                    target.display()
                )))
            }
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

/// The value of `name` when present: it must have a value that is not another flag, and the
/// flag may be given at most once.  "First one wins" would let a stale `--expect-run-id 1`
/// hide behind a later `--expect-run-id 2` (or the reverse) without anyone noticing which gate
/// applied, so a repeat is a usage error (exit 2).  Every caller is `run` or `join`; no command
/// legitimately repeats one of these flags.
fn value_of(args: &[String], name: &str) -> Result<Option<String>, Failure> {
    if args.iter().filter(|a| *a == name).count() > 1 {
        return Err(Failure::Usage(format!("{name} given more than once")));
    }
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
    // Bound the work before reading anything: a plan has at most MAX_SHARDS shards, so more
    // input files than that cannot be a valid set and are not worth opening.
    if files.len() > crate::exp::MAX_SHARDS as usize {
        return Err(Failure::Usage(format!(
            "join refuses: {} input files, at most {} shards exist",
            files.len(),
            crate::exp::MAX_SHARDS
        )));
    }
    // `join` never overwrites: an existing `--out` is refused outright.  That also covers
    // `--out` naming one of the inputs (an input must exist to be read), but the explicit
    // same-file comparison gives that case its own, clearer message (canonicalised, so
    // `./a.json` and `a.json` are the same file).
    let out_path = Path::new(&target);
    if let Ok(canon_out) = out_path.canonicalize() {
        if files.iter().any(|f| {
            Path::new(f.as_str())
                .canonicalize()
                .is_ok_and(|c| c == canon_out)
        }) {
            return Err(Failure::Usage(format!(
                "join refuses: --out {target} is one of the input files"
            )));
        }
    }
    if out_path.exists() {
        return Err(Failure::Usage(format!(
            "join refuses: --out {target} already exists (join never overwrites)"
        )));
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
    write_new(&target, &text)?;
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

/// `sign FILE --reviewer NAME --host LABEL --run-id I --run-attempt A --head-sha S
/// --expect-sha256 H --verify-file J --expect-verify-sha256 V --out OUT` (VOS30 section 9.2,
/// VOS-D120; the verify flags VOS-028c3c-3b-1b, VOS-D141).
///
/// Writes a NEW file, never in place.  Every other in-place edit in this tool (`merge`) replaces
/// a working file whose history git keeps, but a signed file is evidence: it goes where a
/// reviewer can compare it with the unsigned one (`unsigned_sha256` names the latter), and
/// `write_new` publishes it with a hard link, so it can neither overwrite an existing file nor
/// leave a half-written one.  The reviewer renames `OUT` over `FILE` in the PR.  Every refusal is
/// `Failure::Usage` (exit 2) and writes nothing.
pub fn cmd_sign(args: &[String], out: &mut dyn Write) -> Result<(), Failure> {
    refuse_unknown_flags(
        "sign",
        args,
        &[
            "--reviewer",
            "--host",
            "--run-id",
            "--run-attempt",
            "--head-sha",
            "--expect-sha256",
            "--verify-file",
            "--expect-verify-sha256",
            "--out",
        ],
    )?;
    let need = |name: &str| -> Result<String, Failure> {
        value_of(args, name)?.ok_or_else(|| Failure::Usage(format!("sign needs {name}")))
    };
    let req = sign::SignRequest {
        reviewer: need("--reviewer")?,
        host: need("--host")?,
        run_id: need("--run-id")?,
        run_attempt: need("--run-attempt")?,
        head_sha: need("--head-sha")?,
        expect_sha256: Some(need("--expect-sha256")?),
        // Both are required, and a missing flag is a usage error before anything is read.
        verify_text: None,
        expect_verify_sha256: Some(need("--expect-verify-sha256")?),
    };
    let verify_path = need("--verify-file")?;
    let target = need("--out")?;
    let pos = positionals(args);
    let [file] = pos.as_slice() else {
        return Err(Failure::Usage("sign takes exactly one results file".into()));
    };
    // `--out` is never one of the inputs and never an existing file; refused before the (long)
    // validation, as `join` does.
    let out_path = Path::new(&target);
    if out_path.canonicalize().is_ok_and(|c| {
        Path::new(file.as_str())
            .canonicalize()
            .is_ok_and(|i| i == c)
    }) {
        return Err(Failure::Usage(format!(
            "sign refuses: --out {target} is the input file (sign never edits in place)"
        )));
    }
    if out_path.exists() {
        return Err(Failure::Usage(format!(
            "sign refuses: --out {target} already exists (sign never overwrites)"
        )));
    }
    let text = read(Path::new(file.as_str()))?;
    // The verify file is read exactly once; `sign` hashes and parses these very bytes, so a file
    // swapped after the read cannot pass the digest check and then be used (no TOCTOU).
    let req = sign::SignRequest {
        verify_text: Some(read(Path::new(&verify_path))?),
        ..req
    };
    let signed =
        sign::sign(&text, &req, &meta::working_trees(), &meta::trees_at).map_err(Failure::Usage)?;
    let signed_text = signed.to_text();
    // The file we are about to write must be one the tool would itself accept back, banner
    // included (`validate` recomputes the digest), and must differ from the input in the banner
    // alone.
    let back = Results::from_canonical_text(&signed_text).map_err(|e| {
        Failure::Usage(format!(
            "sign: the signed file is not canonical (a bug): {e}"
        ))
    })?;
    if let Some(m) = results::validate_classes(&back).structural.first() {
        return Err(Failure::Usage(format!(
            "sign: the signed file is structurally invalid (a bug): {m}"
        )));
    }
    write_new(&target, &signed_text)?;
    writeln!(
        out,
        "signed {} rows as {} on {} (unsigned sha256 {}) at {target}",
        signed.rows.len(),
        req.reviewer,
        req.host,
        signed
            .meta
            .regeneration
            .as_ref()
            .map_or("", |g| g.unsigned_sha256.as_str())
    )
    .map_err(|e| Failure::Usage(e.to_string()))
}

/// The positional arguments of a command: those that are neither a flag nor the value of a flag
/// that takes one.
fn positionals(args: &[String]) -> Vec<&String> {
    const WITH_VALUE: [&str; 22] = [
        "--reviewer",
        "--host",
        "--head-sha",
        "--expect-sha256",
        "--jobs",
        "--exe",
        "--spot-budget",
        "--struct-budget",
        "--out",
        "--shard",
        "--rerun-out",
        "--recorded",
        "--rerun",
        "--run-id",
        "--run-attempt",
        "--expect-binary-sha256",
        "--expect-run-id",
        "--expect-run-attempt",
        "--expect-shards",
        "--expect-jobs",
        "--verify-file",
        "--expect-verify-sha256",
    ];
    args.iter()
        .enumerate()
        .filter(|(i, a)| {
            !a.starts_with("--") && (*i == 0 || !WITH_VALUE.contains(&args[i - 1].as_str()))
        })
        .map(|(_, a)| a)
        .collect()
}

/// One finding of [`tree_findings`]: either the file cannot say which tree it describes (or the
/// checkout cannot be read or is edited), or one recorded input differs from the checkout's.
enum TreeFinding {
    /// Always fatal for `check --full`, with or without `--at-checkout`.
    Missing(String),
    /// A recorded input id that differs from the checkout's: the `stale:` line without its
    /// prefix.  Fatal for plain `check --full`; a `note:` under `--at-checkout` (VOS-D141).
    Stale(String),
}

/// Every tree-id finding about `res` against the working tree's ids, in the order `check --full`
/// has always reported them (the tool record, then each merge, the missing-ids message of a
/// record before its stale lines).  [`full_refusals`], [`missing_tree_refusals`] and
/// [`stale_refusals`] are views of this one list, so the three cannot disagree.
fn tree_findings(res: &Results, current: &[(String, String)]) -> Vec<TreeFinding> {
    let mut v = Vec::new();
    if res.meta.trees.is_empty() {
        v.push(TreeFinding::Missing(
            "check --full refuses: the file records no tree ids (it predates 0.2.0), so it cannot \
             say which tree it describes; regenerate the rows"
                .to_string(),
        ));
    }
    let mut records = vec![("tool".to_string(), &res.meta.trees)];
    for (i, g) in res.meta.merges.iter().enumerate() {
        records.push((format!("merges[{i}]"), &g.trees));
    }
    for (what, trees) in records {
        if what != "tool" && trees.is_empty() {
            v.push(TreeFinding::Missing(format!(
                "check --full refuses: {what} records no tree ids; regenerate its rows"
            )));
        }
        for line in meta::stale_lines(&what, trees, current) {
            let body = line.trim_start_matches("stale: ").to_string();
            // `stale_lines` answers three cases that are not "an input differs" with a single
            // line (nothing recorded cannot be here, but a legacy whole-crate record and an
            // unreadable checkout can).  A verify at the checkout cannot proceed on those.
            if meta::is_comparable(trees, current) {
                v.push(TreeFinding::Stale(body));
            } else {
                v.push(TreeFinding::Missing(format!(
                    "check --full refuses ({body}); regenerate the rows"
                )));
            }
        }
    }
    v
}

/// Why `check --full` refuses, given the file and the working tree's ids: a file that records no
/// tree ids cannot say which tree it describes, and a recorded id that differs says it no longer
/// describes this one.  Both must be fixed by regenerating the rows, never by editing the ids.
///
/// This is [`missing_tree_refusals`] and [`stale_refusals`] together, in the historical order, so
/// that `check --full` without `--at-checkout` and `sign` (which also calls it) behave exactly
/// as they did before the split.
pub fn full_refusals(res: &Results, current: &[(String, String)]) -> Vec<String> {
    tree_findings(res, current)
        .into_iter()
        .map(|f| match f {
            TreeFinding::Missing(m) => m,
            TreeFinding::Stale(body) => {
                format!("check --full refuses ({body}); regenerate the rows")
            }
        })
        .collect()
}

/// The refusals that hold for `check --full` whatever its flags, `--at-checkout` included
/// (VOS-D141): a file with no tree ids or only the legacy whole-crate names, a checkout whose ids
/// cannot be read, and a checkout with an uncommitted change in an input.  A verify at the
/// checkout verifies a COMMITTED tree, so that tree is one a reviewer can name and check out
/// again; an edited working tree is not one.  (Plan coverage is a separate refusal of `check`.)
pub fn missing_tree_refusals(res: &Results, current: &[(String, String)]) -> Vec<String> {
    let mut v: Vec<String> = tree_findings(res, current)
        .into_iter()
        .filter_map(|f| match f {
            TreeFinding::Missing(m) => Some(m),
            TreeFinding::Stale(_) => None,
        })
        .collect();
    let dirty = meta::uncommitted_inputs(current);
    if !dirty.is_empty() {
        v.push(format!(
            "check --full refuses: the checkout has uncommitted changes in {} input(s) ({}); a \
             verify runs at a committed tree",
            dirty.len(),
            dirty.join(", ")
        ));
    }
    v
}

/// The recorded input ids that differ from the checkout's, one line each (the `stale:` line
/// without its prefix).  Without `--at-checkout` each is a refusal of `check --full`; with it
/// each is a `note:` line and the rows are re-run at the checkout's tree (VOS-D141: a tree
/// difference is reported, not refused, because the verify's own comparison under the 9.12.1
/// classes is what decides whether the recorded rows still reproduce).
pub fn stale_refusals(res: &Results, current: &[(String, String)]) -> Vec<String> {
    tree_findings(res, current)
        .into_iter()
        .filter_map(|f| match f {
            TreeFinding::Stale(body) => Some(body),
            TreeFinding::Missing(_) => None,
        })
        .collect()
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

/// The flags of `check --full --shard K/N [--rerun-out F] [--run-id I --run-attempt A]`
/// (VOS30 section 9.2, VOS-028c3c-2b part ii), parsed and cross-checked BEFORE the file is read,
/// so a malformed command line costs nothing.
struct ShardFlags {
    /// `(K, N)` when `--shard` is given.
    shard: Option<(u32, u32)>,
    /// `--rerun-out`, the verify-mode file to write.
    rerun_out: Option<String>,
    run_id: Option<String>,
    run_attempt: Option<String>,
}

fn shard_flags(args: &[String], full: bool) -> Result<ShardFlags, Failure> {
    let shard = match value_of(args, "--shard")? {
        Some(s) => Some(crate::exp::parse_shard(&s).map_err(Failure::Usage)?),
        None => None,
    };
    let rerun_out = value_of(args, "--rerun-out")?;
    let (run_id, run_attempt) = run_numbers(args)?;
    if shard.is_some() && !full {
        return Err(Failure::Usage(
            "check --shard needs --full: only the full verification is sharded".into(),
        ));
    }
    if shard.is_none() && (rerun_out.is_some() || run_id.is_some()) {
        return Err(Failure::Usage(
            "--rerun-out, --run-id and --run-attempt describe a shard: they need --shard".into(),
        ));
    }
    if rerun_out.is_none() && run_id.is_some() {
        return Err(Failure::Usage(
            "--run-id and --run-attempt are written into the --rerun-out file: they need it".into(),
        ));
    }
    // A verify shard is never overwritten, for the reason `join --out` is not: the file is
    // evidence.  Refused here, before the hours of re-running, not after.
    if let Some(target) = &rerun_out {
        if Path::new(target).exists() {
            return Err(Failure::Usage(format!(
                "check refuses: --rerun-out {target} already exists (it is never overwritten)"
            )));
        }
    }
    // A slice with no row (K beyond the knob count) re-runs nothing and would write an empty
    // "verification": refused, before any file is read.
    if let Some((k, n)) = shard {
        if results::shard_identities(k, n).is_empty() {
            return Err(Failure::Usage(format!(
                "check refuses: shard {k}/{n} holds no row of the plan (nothing to re-run)"
            )));
        }
    }
    Ok(ShardFlags {
        shard,
        rerun_out,
        run_id,
        run_attempt,
    })
}

/// The file `--rerun-out` writes: the RE-RUN rows of one shard (in the recorded file's order,
/// each with `green` recomputed), under the VERIFYING binary's own `meta` and a `recording`
/// block of mode `verify`, so `join` can fan the shards in exactly as it does recordings.
/// `verifies_sha256` is the unsigned digest of the recorded file that was re-run
/// ([`Results::unsigned_sha256`]): it names, inside the evidence itself, which recording this is
/// a verification of.  Pure over its inputs.
#[allow(clippy::too_many_arguments)]
pub fn verify_results(
    mut meta: results::Meta,
    recorded: &[Row],
    rerun: &std::collections::BTreeMap<String, Row>,
    shard: (u32, u32),
    jobs: usize,
    run_id: Option<String>,
    run_attempt: Option<String>,
    verifies_sha256: String,
) -> Results {
    meta.merges = Vec::new();
    meta.regeneration = None;
    meta.recording = Some(results::Recording {
        mode: "verify".into(),
        run_id,
        run_attempt,
        shard: Some(shard),
        shards: None,
        jobs: u32::try_from(jobs).unwrap_or(u32::MAX),
        verifies_sha256: Some(verifies_sha256),
    });
    let rows = recorded
        .iter()
        .filter_map(|r| rerun.get(&results::row_text_id(r)).cloned())
        .collect();
    Results { meta, rows }
}

/// `check [--full [--at-checkout] [--shard K/N [--rerun-out F] [--run-id I --run-attempt A]]] FILE
/// [--jobs N] [--exe PATH] [--spot-budget SECS] [--struct-budget SECS]`.
///
/// The order matters (finding M2): everything that needs no instance is checked first, and a file
/// that fails any of it returns at once, before a single row is re-run, in process or in a child.
/// A hand-edited knob can therefore never reach an instance builder.  VOS-D134 puts three
/// structural guards first of all ([`results::check_guards`]: structural findings, a duplicate
/// row identity, too many quick specs); they exit 2, not 1, because a file with that kind of
/// finding is not a recording to judge but one to discard.
///
/// With `--at-checkout` (full only; VOS-D141) a recorded input id that differs from the checkout's
/// is a `note:` and not a refusal; see [`missing_tree_refusals`] for what is still refused.
///
/// With `--shard K/N` (full only) the WHOLE file is still validated, plan coverage and
/// `full_refusals` included, and then only the rows of shard `K` are re-run (quick tier in
/// process, full tier in children), with the same comparison classes as the unsharded run.
pub fn cmd_check(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> Result<(), Failure> {
    // A mistyped flag (`--rerun-ot`) would otherwise be ignored and the file it names never
    // written, so `check` accepts exactly the flags it knows.
    refuse_unknown_flags(
        "check",
        args,
        &[
            "--full",
            "--at-checkout",
            "--shard",
            "--rerun-out",
            "--run-id",
            "--run-attempt",
            "--exe",
            "--jobs",
            "--spot-budget",
            "--struct-budget",
        ],
    )?;
    let full = has(args, "--full");
    // `--at-checkout` changes what a stale input id means, and only the full verification has
    // such a thing: refused on its own, before the file is read.
    let at_checkout = has(args, "--at-checkout");
    if at_checkout && !full {
        return Err(Failure::Usage(
            "check --at-checkout needs --full: only the full verification runs at the checkout"
                .into(),
        ));
    }
    let sf = shard_flags(args, full)?;
    // The verify file's `meta` names the binary that ran the rows.  `--exe` only chooses the child
    // that runs full-tier rows, so a `--rerun-out` file with a different `--exe` would name a
    // binary that did not run the quick rows: refused, before any work.
    if sf.rerun_out.is_some() {
        if let Some(p) = flag(args, "--exe") {
            let me = std::env::current_exe().map_err(|e| Failure::Usage(e.to_string()))?;
            let same = match (Path::new(&p).canonicalize(), me.canonicalize()) {
                (Ok(a), Ok(b)) => a == b,
                _ => false,
            };
            if !same {
                return Err(Failure::Usage(format!(
                    "check refuses: --rerun-out records this binary's digest, so --exe {p} must \
                     be this executable"
                )));
            }
        }
    }
    let path = positionals(args)
        .first()
        .copied()
        .ok_or_else(|| Failure::Usage("check needs a results file".into()))?;
    let res = Results::from_canonical_text(&read(Path::new(path))?).map_err(Failure::Usage)?;
    let w = |e: std::io::Error| Failure::Usage(e.to_string());
    let guards = results::check_guards(&res);
    if !guards.is_empty() {
        for g in &guards {
            let _ = writeln!(err, "check: structural: {g}");
        }
        return Err(Failure::Usage(format!(
            "{} structural finding(s): nothing was re-run",
            guards.len()
        )));
    }
    // No structural finding is left (the guards returned), so this is the row findings alone,
    // in `validate`'s order.
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
    // The inputs that differ from the recording, for the `note:` lines and the ok tail of a
    // verify at the checkout.  Empty without `--at-checkout`.
    let mut differing: Vec<String> = Vec::new();
    if at_checkout {
        // VOS-D141.  A verify at the checkout re-runs the rows with a binary built from THIS tree,
        // whatever tree the file was recorded at, and judges them under the unchanged 9.12.1
        // classes.  So an input id that differs from the recording is information, a `note:`,
        // not a refusal: the comparison of the rows is what decides whether the recording still
        // holds.  The file's own `trees` are never touched (they record where it was recorded),
        // so there is nothing to forge by passing the flag.  What stays fatal is everything that
        // would make the verify mean nothing: no tree ids, the legacy whole-crate record, an
        // unreadable or edited checkout (all in `missing_tree_refusals`), and, above, a gap in
        // the plan.  The `stale:` lines are replaced by the notes, not printed twice.
        let mut records = vec![("tool".to_string(), &res.meta.trees)];
        for (i, g) in res.meta.merges.iter().enumerate() {
            records.push((format!("merges[{i}]"), &g.trees));
        }
        for (what, trees) in &records {
            for line in meta::stale_lines(what, trees, &current) {
                if meta::is_comparable(trees, &current) {
                    writeln!(out, "check: note: {}", line.trim_start_matches("stale: "))
                        .map_err(w)?;
                } else {
                    // A legacy record or an unreadable checkout is no note: the refusal below
                    // says why it is fatal, and the line keeps its `stale:` form.
                    writeln!(out, "{line}").map_err(w)?;
                }
            }
            for name in meta::differing_names(trees, &current) {
                if !differing.contains(&name) {
                    differing.push(name);
                }
            }
        }
        bad.extend(missing_tree_refusals(&res, &current));
    } else {
        for line in &stale {
            writeln!(out, "{line}").map_err(w)?;
        }
        if full {
            bad.extend(full_refusals(&res, &current));
        }
    }
    let tail = if !bad.is_empty() {
        // Nothing is re-run: the file is already refused.
        String::new()
    } else if full {
        let exe = exe_of(args)?;
        // The verifying binary's provenance is gathered BEFORE the rows run, as `run` does.
        let meta = sf.rerun_out.as_ref().map(|_| meta::gather(&exe));
        let only = sf.shard.map(|(k, n)| results::shard_identities(k, n));
        let (quick_bad, mut rerun_rows) =
            results::rerun_collect_in(&res, jobs(args), true, only.as_ref());
        bad.extend(quick_bad);
        let progress = |n: usize, total: usize| {
            if n.is_multiple_of(25) || n == total {
                eprintln!("[full {n}/{total}]");
            }
        };
        let (full_bad, full_rows) =
            spot::full_rerun_collect_in(&res, &exe, jobs(args), &progress, only.as_ref());
        bad.extend(full_bad);
        rerun_rows.extend(full_rows);
        // Post-condition: the rows re-run are exactly the shard's rows that the file holds.  A
        // slice row that silently was not re-run would be a hole in the verification.
        if let Some(ids) = &only {
            let present: std::collections::BTreeSet<_> =
                res.rows.iter().map(results::identity_of).collect();
            let want: std::collections::BTreeSet<_> = ids.intersection(&present).cloned().collect();
            let got: std::collections::BTreeSet<_> =
                rerun_rows.values().map(results::identity_of).collect();
            if got != want {
                bad.push(format!(
                    "the shard's re-run covered {} rows but its slice of the file holds {}",
                    got.len(),
                    want.len()
                ));
            }
        }
        // The shard's re-run rows are written BEFORE the verdict, even when rows differ: the
        // fan-in needs every shard's file, and `decision-diff` shows what changed.
        if let (Some(target), Some(meta), Some(shard)) = (&sf.rerun_out, meta, sf.shard) {
            let file = verify_results(
                meta,
                &res.rows,
                &rerun_rows,
                shard,
                jobs(args),
                sf.run_id.clone(),
                sf.run_attempt.clone(),
                res.unsigned_sha256(),
            );
            let text = file.to_text();
            Results::from_canonical_text(&text).map_err(|e| {
                Failure::Usage(format!(
                    "check: the verify file is not canonical (a bug): {e}"
                ))
            })?;
            write_new(target, &text)?;
        }
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
        let base = match sf.shard {
            Some((k, n)) => format!(
                "shard {k}/{n}: {} of {} rows re-run (quick in process, full in a child) and \
                 reproduced, timings within the band, and the decision equal on the re-run rows",
                rerun_rows.len(),
                res.rows.len()
            ),
            None => format!(
                "{} rows; every row re-run (quick in process, full in a child) and reproduced, \
                 timings within the band, and the decision equal on the re-run rows",
                res.rows.len()
            ),
        };
        if at_checkout {
            // Say which inputs the recording and this tree disagree on: the evidence that "green"
            // is about the checkout's tree and not the recorded one (VOS-D141).
            if differing.is_empty() {
                format!("{base}; at the checkout every input equals the recorded one")
            } else {
                format!(
                    "{base}; at the checkout {} input(s) differ from the recorded ones: {}",
                    differing.len(),
                    differing.join(", ")
                )
            }
        } else {
            base
        }
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

/// What the two files of a re-run comparison are called in a refusal.  `decision-diff` names them by
/// its flags (`--recorded`, `--rerun`); `sign` names the recording `FILE` and the re-run by its own
/// flag (`--verify-file`).  The words of every refusal are otherwise shared, so the two commands
/// cannot drift apart (VOS-028c3c-3b-1b).
#[derive(Clone, Copy, Debug)]
pub struct RerunLabels<'a> {
    /// What the recorded file is called.
    pub recorded: &'a str,
    /// What the re-run (the joined verify file) is called.
    pub rerun: &'a str,
}

/// The two kinds of difference a clean verification has none of, kept apart so a reviewer sees at
/// once whether single rows moved, the decision moved, or both (a decision can move with every row
/// inside its band, and every row can differ with the decision unchanged).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RerunComparison {
    /// [`results::compare_verify`] lines, in the recorded order, over every identity.
    pub row_diffs: Vec<String>,
    /// [`results::decision_differences`] lines: the decision, the re-plan decision and K.
    pub decision_diffs: Vec<String>,
}

impl RerunComparison {
    /// No row and no decision difference: the recorded rows reproduce.
    pub fn is_clean(&self) -> bool {
        self.row_diffs.is_empty() && self.decision_diffs.is_empty()
    }
}

/// The authentication and comparison core shared by `decision-diff` and `sign` (VOS-028c3c-3b-1b):
/// pure, so each refusal is a unit test and neither command can pass a file the other would refuse.
///
/// Returns `Err(message)` for a refusal (the caller adds its own prefix and exit status), or the
/// row and decision differences of a re-run that is an authentic verification of `recorded`.
///
/// A comparison of two files proves a VERIFICATION only if the second really is one (the security
/// review of VOS-028c3c-2b part ii), so a file that merely has the same rows (the recording itself,
/// a copy, a hand-built file) must not pass.  Each refusal:
///
/// ```text
///   rerun      meta.recording is mode "verify" and JOINED (`shards` set, no single `shard`)
///   recorded   is not mode "verify" (no recording block, or mode "record")
///   both       record non-empty tree ids (a file that cannot say which tree it describes)
///   rerun      names the file it verified: recording.verifies_sha256 equals the unsigned
///              SHA-256 of `recorded` (VOS-D141; this replaces "the same tree ids", because a
///              verify at the checkout legitimately runs at a different tree)
///   both       have no structural finding (`check_guards`, as `check` requires first)
///   rerun      was verified at a committed tree and records no merges
///   expect_shards, when given, equals the joined recording's `shards`
///   both       hold the same row identities, each once
/// ```
///
/// Then EVERY identity is judged by [`results::compare_verify`] (the rule `check --full` applied
/// while it re-ran, so a joined verify file that hides a differing row behind an exit-0 shard is
/// caught), in the recorded order so the lines are stable, and the decision, the re-plan decision
/// and the cut density of the re-run's rows are compared with the recorded ones
/// ([`results::decision_differences`]).  Nothing is solved or re-run.
pub fn compare_rerun(
    recorded: &Results,
    rerun: &Results,
    expect_shards: Option<u32>,
    l: &RerunLabels,
) -> Result<RerunComparison, String> {
    let (rc, rr) = (l.recorded, l.rerun);
    let refuse = |m: String| -> Result<RerunComparison, String> { Err(m) };
    match &rerun.meta.recording {
        Some(rec) if rec.mode == "verify" && rec.shard.is_none() && rec.shards.is_some() => {
            if let (Some(want), Some(got)) = (expect_shards, rec.shards) {
                if want != got {
                    return refuse(format!(
                        "{rr} joins {got} shards but --expect-shards is {want}"
                    ));
                }
            }
        }
        Some(rec) if rec.mode == "verify" => {
            return refuse(format!(
                "{rr} is one verify shard, not a joined file (run `join` over every shard first)"
            ));
        }
        _ => {
            return refuse(format!(
                "{rr} is not a verify file (it needs a recording block of mode verify, as \
                 `check --full --shard --rerun-out` writes and `join` fans in)"
            ));
        }
    }
    if recorded
        .meta
        .recording
        .as_ref()
        .is_some_and(|r| r.mode == "verify")
    {
        return refuse(format!(
            "{rc} is itself a verify file (it must be the recording being verified)"
        ));
    }
    if recorded.meta.trees.is_empty() || rerun.meta.trees.is_empty() {
        return refuse(
            "a file records no tree ids, so it cannot say which tree it describes".into(),
        );
    }
    for (which, res) in [(rc, recorded), (rr, rerun)] {
        let g = results::check_guards(res);
        if let Some(first) = g.first() {
            return refuse(format!(
                "{which} has {} structural finding(s); first: {first}",
                g.len()
            ));
        }
    }
    // The verifying side must have run at a committed, well-formed tree, as the producer
    // (`check --full --at-checkout`, `missing_tree_refusals`) insists: with the tree-id equality
    // gone nothing else would stop a hand-built file with edited, legacy or junk ids.  A verify
    // file never records merges (`verify_results` clears them), so merges are refused too.
    if let Some(first) = meta::committed_tree_refusals(&rerun.meta.trees).first() {
        return refuse(format!(
            "{rr} was not verified at a committed tree: {first}"
        ));
    }
    if !rerun.meta.merges.is_empty() {
        return refuse(format!("{rr} records merges (a verify file never does)"));
    }
    // Last, after the structural findings (as the tree-id comparison it replaces was):
    // VOS-D141: a verify now runs at the checkout's tree, which need not be the recorded one, so
    // the tree ids can no longer say "this is a verification of that recording".  The digest the
    // verify shards carry (`recording.verifies_sha256`, equal across the joined shards) does:
    // it is the SHA-256 of the recorded file, banner removed, so it is the same before and after
    // `sign`.  Absent (a file from before the field) is refused, not waved through: nothing
    // would then bind this re-run to this recording.
    match rerun
        .meta
        .recording
        .as_ref()
        .and_then(|r| r.verifies_sha256.as_deref())
    {
        None => {
            return refuse(format!(
                "{rr} names no recorded file (recording.verifies_sha256 is absent), so it cannot be shown to verify {rc}"
            ));
        }
        Some(got) => {
            let want = recorded.unsigned_sha256();
            if got != want {
                return refuse(format!(
                    "{rr} verified a different file: its verifies_sha256 is {got} but the unsigned SHA-256 of {rc} is {want}"
                ));
            }
        }
    }
    let ids = |r: &Results| -> Result<std::collections::BTreeSet<results::Identity>, String> {
        let set: std::collections::BTreeSet<_> = r.rows.iter().map(results::identity_of).collect();
        if set.len() != r.rows.len() {
            return Err("a file lists a row identity twice".to_string());
        }
        Ok(set)
    };
    let (a, b) = (ids(recorded)?, ids(rerun)?);
    if a != b {
        let gap = a.difference(&b).next();
        let stray = b.difference(&a).next();
        return Err(format!(
            "the files hold different rows ({} recorded, {} re-run; \
             first missing from the re-run: {gap:?}; first not recorded: {stray:?})",
            a.len(),
            b.len()
        ));
    }
    // Same rows, the recorded order: the decision must not depend on how the fan-in ordered them.
    let by_id: std::collections::BTreeMap<results::Identity, &Row> = rerun
        .rows
        .iter()
        .map(|r| (results::identity_of(r), r))
        .collect();
    let mut out = RerunComparison::default();
    let mut again: Vec<Row> = Vec::new();
    for rec in &recorded.rows {
        if let Some(now) = by_id.get(&results::identity_of(rec)) {
            out.row_diffs.extend(results::compare_verify(rec, now));
            again.push((*now).clone());
        }
    }
    out.decision_diffs = results::decision_differences(&recorded.rows, &again);
    Ok(out)
}

/// `decision-diff --recorded R --rerun J [--expect-shards N]` (VOS30 section 9.2; VOS-028c3c-2b
/// part ii): the last step of the verify fan-in.  `J` is the `join` of the verify shards, and is
/// authenticated and compared with `R` by [`compare_rerun`] (a joined verify-mode file whose
/// `verifies_sha256` is the unsigned digest of `R`, with the same row identities, else exit 2);
/// nothing is solved or re-run.
///
/// ```text
///   exit 0   no row difference and no decision difference
///   exit 1   at least one difference, in two labelled sections: "row differences" and
///            "decision differences"
///   exit 2   usage, an unreadable or non-canonical file, a re-run that is not an authentic
///            joined verify file of this recording, or differing identity sets
/// ```
pub fn cmd_decision_diff(
    args: &[String],
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), Failure> {
    refuse_unknown_flags(
        "decision-diff",
        args,
        &["--recorded", "--rerun", "--expect-shards"],
    )?;
    if !positionals(args).is_empty() {
        return Err(Failure::Usage(
            "decision-diff takes only --recorded R --rerun J [--expect-shards N]".into(),
        ));
    }
    let expect_shards: Option<u32> = value_of(args, "--expect-shards")?
        .map(|v| {
            (!v.is_empty() && v.len() <= 5 && v.bytes().all(|b| b.is_ascii_digit()))
                .then(|| v.parse::<u32>().ok())
                .flatten()
                .ok_or_else(|| {
                    Failure::Usage("--expect-shards must be a small decimal integer".into())
                })
        })
        .transpose()?;
    let load = |flag: &str| -> Result<Results, Failure> {
        let p = value_of(args, flag)?
            .ok_or_else(|| Failure::Usage(format!("decision-diff needs {flag} FILE")))?;
        Results::from_canonical_text(&read(Path::new(&p))?)
            .map_err(|e| Failure::Usage(format!("{p}: {e}")))
    };
    let (recorded, rerun) = (load("--recorded")?, load("--rerun")?);
    // VOS-D141: the verdict comes from DATA, not from the exit codes of the shards.
    let found = compare_rerun(
        &recorded,
        &rerun,
        expect_shards,
        &RerunLabels {
            recorded: "--recorded",
            rerun: "--rerun",
        },
    )
    .map_err(|m| Failure::Usage(format!("decision-diff refuses: {m}")))?;
    let (row_diffs, diffs) = (&found.row_diffs, &found.decision_diffs);
    // Two labelled sections, so a reviewer sees at once whether single rows moved, the decision
    // moved, or both.
    if !row_diffs.is_empty() {
        let _ = writeln!(err, "decision-diff: row differences ({}):", row_diffs.len());
        for d in row_diffs {
            let _ = writeln!(err, "decision-diff:   row: {d}");
        }
    }
    if !diffs.is_empty() {
        let _ = writeln!(
            err,
            "decision-diff: decision differences ({}):",
            diffs.len()
        );
        for d in diffs {
            let _ = writeln!(err, "decision-diff:   decision: {d}");
        }
    }
    if found.is_clean() {
        writeln!(
            out,
            "decision-diff: ok ({} rows: every row reproduces under the 9.12.1 classes; the \
             decision, the re-plan decision and K are equal)",
            recorded.rows.len()
        )
        .map_err(|e| Failure::Usage(e.to_string()))
    } else {
        Err(Failure::Violations(row_diffs.len() + diffs.len()))
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

/// `table` splices text from the results file (the banner's reviewer and host) into the spec's
/// Markdown, so it first demands that the file's structure is sound: a hand-edited banner with a
/// newline, a `|` or a `<` must not reach the spec.  Exit 2 (usage), nothing rendered or written.
fn refuse_unrenderable(res: &Results) -> Result<(), Failure> {
    let v = results::validate_classes(res);
    match v.structural.first() {
        None => Ok(()),
        Some(m) => Err(Failure::Usage(format!(
            "table refuses: the results file is structurally invalid ({} finding(s)); first: {m}",
            v.structural.len()
        ))),
    }
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
        if let Some(r) = &res {
            refuse_unrenderable(r)?;
        }
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
    refuse_unrenderable(&res)?;
    let block = table::render_block(&res);
    if has(args, "--write") {
        let mut new = table::splice(&spec, &block)
            .ok_or_else(|| Failure::Usage("the spec has no marker pair to write between".into()))?;
        // The re-plan block of 8.6 (VOS-D142) is written in the same pass when the spec has its
        // marker pair; a broken pair is a usage error, not a silent skip.
        if table::extract_replan_block(&new)
            .map_err(Failure::Usage)?
            .is_some()
        {
            new = table::splice_replan(&new, &table::render_replan_block(&res))
                .ok_or_else(|| Failure::Usage("the spec has no replan marker pair".into()))?;
        }
        write_atomic(&sp, &new)?;
    } else {
        write!(out, "{block}").map_err(|e| Failure::Usage(e.to_string()))?;
    }
    Ok(())
}

/// `decide`; `decide --replan` prints section 8.6's decision instead (VOS-D142): the row, K, the
/// memory verdict and the conditions, the lines `table --write` renders into the replan block.
pub fn cmd_decide(args: &[String], out: &mut dyn Write) -> Result<(), Failure> {
    let (rp, _) = paths(args);
    let res = Results::from_canonical_text(&read(&rp)?).map_err(Failure::Usage)?;
    let w = |e: std::io::Error| Failure::Usage(e.to_string());
    if has(args, "--replan") {
        for l in table::replan_lines(&results::decide_replan(&res.rows)) {
            writeln!(out, "{l}").map_err(w)?;
        }
        return Ok(());
    }
    let d = decide(&res.rows);
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
