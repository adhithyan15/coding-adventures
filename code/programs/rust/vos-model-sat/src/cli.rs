//! # The command line (VOS31 section 9), as a library so it can be tested
//!
//! [`parse`] turns arguments into a [`Command`]; [`run`] performs it and returns the exit code
//! and the lines for standard output and standard error.  `main` only prints them.
//!
//! ```text
//!   gen   : load -> analyse -> (no finding) write warrant/lemmas.sat       exit 0 / 1 / 2
//!   check : load -> check   -> compare with warrant/lemmas.sat             exit 0 / 1 / 2
//!   cross-check : the three cross-checks of VOS31 section 6, alone         exit 0 / 1 / 2
//! ```
//!
//! For `gen` and `check`, every failure to read an input (including the loader's "no verdict") is a
//! finding (`MS01`, exit 1), never an abort; for `cross-check` an unreadable crate is exit 2. Exit 2 is also for bad
//! arguments and for an evaluator that could not run.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::lane::{self, Analysis, Finding, LaneConfig};
use crate::oracle::{load_detailed, LoadError};

/// Which subcommand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    /// `cross-check`
    CrossCheck,
    /// `gen`
    Gen,
    /// `check`
    Check,
}

/// A parsed command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    /// The subcommand.
    pub verb: Verb,
    /// The contract crate directory.
    pub dir: PathBuf,
    /// `--root`.
    pub root: Option<PathBuf>,
    /// `--emit-drat` (only `check`).
    pub emit_drat: Option<PathBuf>,
    /// `--allow-missing-sat-key` (gen and check): do not require `sat = "model"` in vos.toml.
    pub allow_missing_sat_key: bool,
}

/// Parse `args` (without the program name); `None` is a usage error.
pub fn parse(args: &[String]) -> Option<Command> {
    let mut it = args.iter();
    let verb = match it.next()?.as_str() {
        "cross-check" => Verb::CrossCheck,
        "gen" => Verb::Gen,
        "check" => Verb::Check,
        _ => return None,
    };
    let mut dir = None;
    let mut root = None;
    let mut emit_drat = None;
    let mut allow = false;
    while let Some(a) = it.next() {
        match a.as_str() {
            "--allow-missing-sat-key" if verb != Verb::CrossCheck => allow = true,
            "--root" => root = Some(PathBuf::from(it.next()?)),
            "--emit-drat" if verb == Verb::Check => emit_drat = Some(PathBuf::from(it.next()?)),
            _ if dir.is_none() && !a.starts_with("--") => dir = Some(PathBuf::from(a)),
            _ => return None,
        }
    }
    Some(Command {
        verb,
        dir: dir?,
        root,
        emit_drat,
        allow_missing_sat_key: allow,
    })
}

fn finding_lines(findings: &[Finding]) -> Vec<String> {
    findings
        .iter()
        .map(|f| format!("{}: {}", f.code, f.message))
        .collect()
}

/// The warning printed on standard error whenever the opt-in bridge is used.
pub const BRIDGE_WARNING: &str = "WARNING: MS01 opt-in check skipped (--allow-missing-sat-key bridge, VOS-040b; removed by VOS-040c)";

/// Create `dir` (and any missing ancestors) for `--emit-drat`, one component at a time.  The
/// deepest directory that already exists must be plain (no `..`, no symlink in its path: the
/// loader's own `absolute_plain`), and every directory created here is checked not to be a
/// symlink before anything is written through it.
fn prepare_emit_dir(dir: &Path) -> Result<PathBuf, String> {
    let mut missing: Vec<std::ffi::OsString> = Vec::new();
    let mut base = dir.to_path_buf();
    loop {
        if base.as_os_str().is_empty() {
            base = PathBuf::from(".");
            break;
        }
        match std::fs::symlink_metadata(&base) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(format!("{}: is a symlink", base.display()))
            }
            Ok(m) if !m.is_dir() => return Err(format!("{}: is not a directory", base.display())),
            Ok(_) => break,
            Err(_) => {
                let Some(name) = base.file_name() else {
                    return Err(format!("{}: cannot be created", dir.display()));
                };
                missing.push(name.to_os_string());
                base = base.parent().map(Path::to_path_buf).unwrap_or_default();
            }
        }
    }
    let mut p = vos_link::fsio::absolute_plain(&base).map_err(|e| e.to_string())?;
    for name in missing.iter().rev() {
        p.push(name);
        std::fs::create_dir(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        let md = std::fs::symlink_metadata(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        if md.file_type().is_symlink() || !md.is_dir() {
            return Err(format!("{}: is not a plain directory", p.display()));
        }
    }
    Ok(p)
}

/// Write the DIMACS and DRAT of every certified obligation into `dir` (VOS31 8.1: only on
/// request).  Each file is created with `create_new`: an existing file or symlink is refused,
/// never overwritten or written through.
fn emit(dir: &Path, proofs: &[(String, String, String)]) -> Result<(), String> {
    let dir = prepare_emit_dir(dir)?;
    for (stem, cnf, drat) in proofs {
        for (ext, text) in [("cnf", cnf), ("drat", drat)] {
            let p = dir.join(format!("{stem}.{ext}"));
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&p)
                .map_err(|e| format!("{}: {e}", p.display()))?;
            f.write_all(text.as_bytes())
                .map_err(|e| format!("{}: {e}", p.display()))?;
        }
    }
    Ok(())
}

/// Perform `cmd`: `(exit code, stdout lines, stderr lines)`.  A run that used the
/// `--allow-missing-sat-key` bridge always starts its standard error with [`BRIDGE_WARNING`].
pub fn run(cmd: &Command) -> (u8, Vec<String>, Vec<String>) {
    run_with(cmd, None)
}

/// [`run`] on evidence the caller already gathered (tests), or `None` to gather it.
pub fn run_with(
    cmd: &Command,
    evidence: Option<&crate::crosscheck::Evidence>,
) -> (u8, Vec<String>, Vec<String>) {
    let (code, out, mut err) = run_inner(cmd, evidence);
    if cmd.allow_missing_sat_key {
        err.insert(0, BRIDGE_WARNING.to_string());
    }
    (code, out, err)
}

fn run_inner(
    cmd: &Command,
    evidence: Option<&crate::crosscheck::Evidence>,
) -> (u8, Vec<String>, Vec<String>) {
    let files = match load_detailed(&cmd.dir, cmd.root.as_deref()) {
        Err(LoadError::Inputs(m)) | Err(LoadError::NoVerdict(m))
            if cmd.verb != Verb::CrossCheck =>
        {
            return (1, vec![], vec![format!("MS01: {m}")]);
        }
        Ok(f) => f,
        Err(e) => return (2, vec![], vec![format!("no verdict: {e}")]),
    };
    if cmd.verb == Verb::CrossCheck {
        if let Some(ev) = evidence {
            let (lines, ok) = crate::run_cross_checks_with(ev);
            return (if ok { 0 } else { 1 }, lines, vec![]);
        }
        return match crate::run_cross_checks(&files) {
            Ok((lines, ok)) => (if ok { 0 } else { 1 }, lines, vec![]),
            Err(e) => (2, vec![], vec![format!("no verdict: {e}")]),
        };
    }
    let mut cfg = LaneConfig::real();
    if cmd.allow_missing_sat_key {
        cfg = cfg.bridge_missing_sat_key();
    }
    if cmd.emit_drat.is_some() {
        cfg = cfg.keep_proofs();
    }
    let result = if cmd.verb == Verb::Gen {
        lane::analyse_with(&files, &cfg, evidence)
    } else {
        lane::check_with(&files, &cfg, evidence)
    };
    let a: Analysis = match result {
        Ok(a) => a,
        Err(e) => return (2, vec![], vec![format!("no verdict: {e}")]),
    };
    let mut out = a.report.clone();
    if !a.findings.is_empty() {
        return (1, out, finding_lines(&a.findings));
    }
    if let Some(dir) = &cmd.emit_drat {
        if let Err(e) = emit(dir, &a.proofs) {
            return (2, out, vec![format!("no verdict: {e}")]);
        }
        out.push(format!("DRAT files written to {}", dir.display()));
    }
    if cmd.verb == Verb::Gen {
        let warrant = files.dir.join("warrant");
        if std::fs::symlink_metadata(&warrant).is_ok_and(|m| m.file_type().is_symlink()) {
            return (
                1,
                out,
                vec![format!("MS01: {} is a symlink", warrant.display())],
            );
        }
        let text = a.table.unwrap_or_default();
        // A temporary file in the same directory, then a rename over the target.
        if let Err(e) = vos_link::fsio::write_replacing(&warrant, "lemmas.sat", text.as_bytes()) {
            return (2, out, vec![format!("no verdict: {e}")]);
        }
        out.push(format!("wrote {}", warrant.join("lemmas.sat").display()));
    } else {
        out.push("warrant/lemmas.sat is current".to_string());
    }
    (0, out, vec![])
}
