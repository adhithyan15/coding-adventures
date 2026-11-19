//! # `check-publish`: the VOS-D104 sweep (VOS-039b-2b, codes `PB01` and `PB02`)
//!
//! `check-component` judges the implementation and contract crates.  A check program, a probe, the
//! document tool itself and the shape checker are nothing to it, and `publish = false` (VOS-D082) used
//! to be applied by hand, so a new one that forgot it was caught by nobody.  This is the sweep.
//!
//! ```text
//!   code/packages/rust/vos-*   code/programs/rust/vos-*      DIRECT children only, each with a Cargo.toml
//!        │
//!        ├─ vos.toml kind production | reference | contract ─► exempt, with a note
//!        │      (whether a shipped crate is a crates.io package is undecided: the release gate)
//!        └─ anything else (kind any, no kind, no vos.toml)  ─► [package] publish = false, or PB01
//! ```
//!
//! * **Scope.**  A directory named `vos-*`, directly under the two roots, that holds a `Cargo.toml`.
//!   There is no recursion: a fixture tree under `tests/fixtures/` is not a child, and a directory
//!   without a `Cargo.toml` is not a crate.
//! * **`PB01`** the crate does not say `publish = false` (the literal, in `[package]`).
//! * **`PB02`** its `Cargo.toml` or `vos.toml` could not be read as TOML (the rule could not decide).
//! * **`PB03`** its `vos.toml` claims kind production, reference or contract but the crate is not
//!   registered in the build tool's `repositoryInputConsumers` by a `vosImplementationRow` or
//!   `vosContractRow` line (see "The kind is the author's word" below).
//! * **No verdict** (exit 2): a symlinked crate directory or `Cargo.toml`, an unreadable directory,
//!   a file over its cap, a missing or unreadable `repository_inputs.go` when a crate needs it.  A symlink could make the sweep judge a crate other than the one reviewed.
//!
//! ## The kind is the author's word, so the exemption needs a second key
//!
//! The kind is the author's own word in `vos.toml`.  A tooling crate that added a one-line `vos.toml`
//! saying `kind = "production"` would escape PB01 if the kind alone exempted it: nothing runs
//! `check-component` on a crate that nobody registered.  So a kind-exempt crate is exempt ONLY when
//! it is also registered in `code/programs/go/build-tool/repository_inputs.go` (a `vosImplementationRow`
//! or `vosContractRow` line inside `repositoryInputConsumers`; `check-component` then runs in its BUILD
//! and judges whether the claim is true).  The row is read by the SAME scanner as CC05
//! ([`row_in_consumers_with`]: comments and raw strings do not count).  A claim without a row is PB03.
//!
//! Every name printed in a message is escaped with [`esc`]: a directory is author-controlled, and one
//! named `vos-x<newline>result: PASS` must not forge a report line.
//!
//! `Cargo.toml` is read with the same reader as `check-component` CC06, so `publish = false` in a
//! table, an inline table or a dotted key is the same path.

use std::path::{Path, PathBuf};

use vos_shape_check::manifest::{parse_vos_toml, Kind, CARGO_TOML_MAX_BYTES, VOS_TOML_MAX_BYTES};
use vos_shape_check::toml::{self as toml_reader, Value};

use crate::component::{read_text, row_in_consumers_with, BUILD_TOOL_ROWS};
use crate::error::{Finding, NoVerdict};
use crate::fsio::{absolute_plain, ensure_plain_dirs, esc, q};
use crate::ops::Outcome;

/// The directories swept, relative to the repository root.
pub const SCOPES: [[&str; 3]; 2] = [["code", "packages", "rust"], ["code", "programs", "rust"]];

/// The prefix of a crate directory that is swept.
pub const PREFIX: &str = "vos-";

/// The repository root of the directory the tool runs in: the ancestor above
/// `code/(packages|programs)/rust/<crate>`.  `cargo run` keeps the working directory, and a BUILD
/// line runs in the crate's own directory.  No `..` is ever put in a path.
pub fn root_from_cwd() -> Result<PathBuf, NoVerdict> {
    let cwd = std::env::current_dir()
        .and_then(|c| c.canonicalize())
        .map_err(|e| NoVerdict::new(format!("cwd: {e}")))?;
    root_above(&cwd).ok_or_else(|| {
        NoVerdict::new(format!(
            "{}: not inside code/packages/rust or code/programs/rust; pass --root <repo-root>",
            q(&cwd)
        ))
    })
}

/// The repository root above `dir` when `dir` is inside `<root>/code/<packages|programs>/rust`.
pub fn root_above(dir: &Path) -> Option<PathBuf> {
    for a in dir.ancestors() {
        let rust = a.file_name().is_some_and(|n| n == "rust");
        let kind = a
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|n| n == "packages" || n == "programs");
        let code = a
            .parent()
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .is_some_and(|n| n == "code");
        if rust && kind && code {
            return a.parent()?.parent()?.parent().map(Path::to_path_buf);
        }
    }
    None
}

/// Is `publish = false` written in `[package]` of this manifest text?  `Err` is a TOML failure.
pub fn says_publish_false(cargo_toml: &str) -> Result<bool, toml_reader::TomlError> {
    Ok(toml_reader::parse(cargo_toml)?.iter().any(|e| {
        e.path == ["package".to_string(), "publish".to_string()] && e.value == Value::Bool(false)
    }))
}

/// `vos-link check-publish [--root <repo-root>]`.
pub fn check_publish(root_arg: Option<&Path>) -> Result<Outcome, NoVerdict> {
    let root = match root_arg {
        Some(r) => absolute_plain(r)?,
        None => root_from_cwd()?,
    };
    // Both scan directories must exist and be plain: a root that has neither is not a repository.
    for scope in SCOPES {
        ensure_plain_dirs(&root, &scope)?;
    }
    let mut out = Outcome::default();
    let (mut swept, mut exempt) = (0usize, Vec::new());
    // The build tool's rows, read the first time a crate claims an exempt kind.
    let mut rows: Option<String> = None;
    for scope in SCOPES {
        let dir = scope.iter().fold(root.clone(), |p, s| p.join(s));
        for name in prefixed_children(&dir, MAX_ENTRIES)? {
            let crate_dir = dir.join(&name);
            let shown = format!("{}/{name}", scope.join("/"));
            // A symlink is looked at BEFORE anything is read through it.
            match std::fs::symlink_metadata(&crate_dir) {
                Ok(m) if m.file_type().is_symlink() => {
                    return Err(NoVerdict::new(format!(
                        "{}: a crate directory is a symlink; the sweep would judge a crate other than the one reviewed",
                        q(&crate_dir)
                    )))
                }
                Ok(m) if m.is_dir() => {}
                Ok(_) => continue, // a file named `vos-..` is not a crate
                Err(e) => return Err(NoVerdict::new(format!("{}: {e}", q(&crate_dir)))),
            }
            let Some(cargo) = read_text(&crate_dir.join("Cargo.toml"), CARGO_TOML_MAX_BYTES)?
            else {
                continue; // no Cargo.toml: not a crate (a fixture tree, a stray directory)
            };
            swept += 1;
            let ctx = Judged {
                root: &root,
                crate_dir: &crate_dir,
                name: &name,
                shown: &shown,
                cargo: &cargo,
            };
            judge(&mut out, &mut exempt, &mut rows, &ctx)?;
        }
    }
    out.header = format!("root {}, {swept} crates", q(&root));
    if !exempt.is_empty() {
        out.notes.push(format!(
            "publish policy undecided, release gate: {} are exempt (kind production, reference or contract); whether a shipped crate is a crates.io package is decided with the release gate (VOS-D104, the part that stays open)",
            exempt.join(", ")
        ));
    }
    out.notes.push("check-publish: every other `vos-*` crate directly under code/packages/rust and code/programs/rust that holds a Cargo.toml says `publish = false` in [package]. Not checked: crates not named `vos-*`, nested crates, a `[workspace.package]` default, and that a publish step is absent from CI.".to_string());
    Ok(out)
}

/// The most directory entries the sweep reads in one directory (the repository has a few hundred).
pub const MAX_ENTRIES: usize = 20_000;

/// Is the directory entry `raw` a name the sweep looks at?  `Ok(Some(name))` when it starts with
/// [`PREFIX`], `Ok(None)` when it does not, and `Err(())` when it starts with the prefix but is not
/// UTF-8 (no verdict, as everywhere in this tool: a name the report could not print faithfully).
#[allow(clippy::result_unit_err)]
pub fn swept_name(raw: &std::ffi::OsStr) -> Result<Option<&str>, ()> {
    if !raw.as_encoded_bytes().starts_with(PREFIX.as_bytes()) {
        return Ok(None);
    }
    raw.to_str().map(Some).ok_or(())
}

/// The names of the entries of `dir` that start with [`PREFIX`], sorted, reading at most `max`
/// entries.  Unlike [`crate::fsio::list_names`] (which caps a crate's own directory at 64 entries)
/// this reads a directory of hundreds of packages, with its own cap.
pub fn prefixed_children(dir: &Path, max: usize) -> Result<Vec<String>, NoVerdict> {
    let io = |e: std::io::Error| NoVerdict::new(format!("{}: {e}", q(dir)));
    let mut names = Vec::new();
    let mut seen = 0usize;
    for entry in std::fs::read_dir(dir).map_err(io)? {
        let entry = entry.map_err(io)?;
        seen += 1;
        if seen > max {
            return Err(NoVerdict::new(format!(
                "{}: more than {max} entries",
                q(dir)
            )));
        }
        let raw = entry.file_name();
        match swept_name(&raw) {
            Ok(Some(name)) => names.push(name.to_string()),
            Ok(None) => {}
            Err(()) => {
                return Err(NoVerdict::new(format!(
                    "{}: a `vos-` name is not UTF-8",
                    q(dir)
                )))
            }
        }
    }
    names.sort();
    Ok(names)
}

/// Where one crate is, for [`judge`].
struct Judged<'a> {
    root: &'a Path,
    crate_dir: &'a Path,
    /// The directory name (also the crate name the build tool's rows use).
    name: &'a str,
    /// The path shown in a message, relative to the root.
    shown: &'a str,
    cargo: &'a str,
}

/// Is `name` registered in the build tool's consumer rows?  `rows` caches the file: it is read the
/// first time and never when no crate claims an exempt kind.  A missing or unreadable file is no
/// verdict (the rule could not decide), never a pass.
fn registered(root: &Path, rows: &mut Option<String>, name: &str) -> Result<bool, NoVerdict> {
    if rows.is_none() {
        let path = root.join(BUILD_TOOL_ROWS);
        let text = read_text(&path, ROWS_MAX_BYTES)?.ok_or_else(|| {
            NoVerdict::new(format!(
                "{}: not found (check-publish needs it to confirm a kind-exempt crate is registered)",
                q(&path)
            ))
        })?;
        *rows = Some(text);
    }
    let src = rows.as_deref().unwrap_or_default();
    Ok(row_in_consumers_with(src, "vosImplementationRow", name)
        || row_in_consumers_with(src, "vosContractRow", name))
}

/// The cap on the build tool's row file (the same as CC05's).
const ROWS_MAX_BYTES: usize = 4 * 1024 * 1024;

/// Judge one crate.
fn judge(
    out: &mut Outcome,
    exempt: &mut Vec<String>,
    rows: &mut Option<String>,
    c: &Judged<'_>,
) -> Result<(), NoVerdict> {
    let shown = c.shown;
    let kind = match read_text(&c.crate_dir.join("vos.toml"), VOS_TOML_MAX_BYTES)? {
        None => None,
        Some(text) => match parse_vos_toml(&text) {
            Ok(v) => v.kind,
            Err((line, why)) => {
                out.findings.push(Finding::new(
                    "PB02",
                    format!(
                        "{}:{line}: {}",
                        esc(&format!("{shown}/vos.toml")),
                        esc(&why)
                    ),
                ));
                return Ok(());
            }
        },
    };
    if let Some(k @ (Kind::Production | Kind::Reference | Kind::Contract)) = kind {
        if registered(c.root, rows, c.name)? {
            exempt.push(esc(c.name));
        } else {
            out.findings.push(Finding::new(
                "PB03",
                format!(
                    "{}: claims kind {} but is not a registered VOS component ({BUILD_TOOL_ROWS} has no `vosImplementationRow` or `vosContractRow` line for it); either register it (check-component runs in its BUILD) or set publish = false",
                    esc(shown),
                    k.name()
                ),
            ));
        }
        return Ok(());
    }
    match says_publish_false(c.cargo) {
        Ok(true) => {}
        Ok(false) => out.findings.push(Finding::new(
            "PB01",
            format!(
                "{}: `publish = false` is missing from [package]; a crate of kind {} is evidence or tooling and must not be publishable (VOS-D082, VOS-D104)",
                esc(&format!("{shown}/Cargo.toml")),
                kind.map_or("(none)", Kind::name)
            ),
        )),
        Err(e) => out.findings.push(Finding::new(
            "PB02",
            format!(
                "{}:{}: {}",
                esc(&format!("{shown}/Cargo.toml")),
                e.line,
                esc(&e.message)
            ),
        )),
    }
    Ok(())
}
