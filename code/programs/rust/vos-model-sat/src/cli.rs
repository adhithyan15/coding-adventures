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
//! Every failure to read an input is a finding (`MS01`), never an abort; exit 2 is for bad
//! arguments and for an evaluator that could not run.

use std::path::{Component, Path, PathBuf};

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

/// A path that is plain: no `..`, so `--emit-drat` cannot climb out of where it was pointed.
fn plain(p: &Path) -> bool {
    !p.components().any(|c| matches!(c, Component::ParentDir))
}

/// Write the DIMACS and DRAT of every certified obligation into `dir` (VOS31 8.1: only on
/// request).  Refuses a `..`, and refuses to overwrite a symlink.
fn emit(dir: &Path, proofs: &[(String, String, String)]) -> Result<(), String> {
    if !plain(dir) {
        return Err(format!("{}: `..` is not allowed", dir.display()));
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for (stem, cnf, drat) in proofs {
        for (ext, text) in [("cnf", cnf), ("drat", drat)] {
            let p = dir.join(format!("{stem}.{ext}"));
            if std::fs::symlink_metadata(&p).is_ok_and(|m| m.file_type().is_symlink()) {
                return Err(format!("{}: is a symlink", p.display()));
            }
            std::fs::write(&p, text).map_err(|e| format!("{}: {e}", p.display()))?;
        }
    }
    Ok(())
}

/// Perform `cmd`: `(exit code, stdout lines, stderr lines)`.
pub fn run(cmd: &Command) -> (u8, Vec<String>, Vec<String>) {
    run_with(cmd, None)
}

/// [`run`] on evidence the caller already gathered (tests), or `None` to gather it.
pub fn run_with(
    cmd: &Command,
    evidence: Option<&crate::crosscheck::Evidence>,
) -> (u8, Vec<String>, Vec<String>) {
    let files = match load_detailed(&cmd.dir, cmd.root.as_deref()) {
        Ok(f) => f,
        Err(LoadError::Inputs(m)) | Err(LoadError::NoVerdict(m))
            if cmd.verb != Verb::CrossCheck =>
        {
            return (1, vec![], vec![format!("MS01: {m}")]);
        }
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
    cfg.require_opt_in = !cmd.allow_missing_sat_key;
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
        let path = files.dir.join("warrant").join("lemmas.sat");
        if std::fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            return (
                1,
                out,
                vec![format!("MS01: {} is a symlink", path.display())],
            );
        }
        let text = a.table.unwrap_or_default();
        if let Err(e) = std::fs::write(&path, text) {
            return (2, out, vec![format!("no verdict: {}: {e}", path.display())]);
        }
        out.push(format!("wrote {}", path.display()));
    } else {
        out.push("warrant/lemmas.sat is current".to_string());
    }
    (0, out, vec![])
}
