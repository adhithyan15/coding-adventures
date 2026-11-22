//! # The command line (`warrant-corpus-gen`)
//!
//! ```text
//!   warrant-corpus-gen --warrant <path> --profile <path> --out <dir> [--check] [--bindings <path>]
//!
//!     --warrant   the .warrant file; a repo-relative path (no `..`, no `\`), recorded
//!                 verbatim as `spec` so the corpus does not depend on the working directory
//!     --profile   corpus.profile (VER04r 8.4)
//!     --out       the fixture directory: corpus.json, cases.tfv, chunks/
//!     --check     regenerate and compare byte for byte with --out; write nothing
//!     --bindings  an evidence.bindings file; its `faults=` lists must equal the profile's
//!
//! ```
//!
//! ## Exit status
//!
//! The convention is the independent oracle's (`warrant-oracle`, its `cli.py`): **1 means the
//! two sides disagree, 2 means the tool refused or could not run**.  A caller (a BUILD line, a
//! pre-commit hook) can then tell "the committed fixture is stale: regenerate and review" from
//! "the generator would not run on this input" without reading stderr.
//!
//! ```text
//!   0  success          written, or --check found the directory byte-identical
//!   1  drift            --check ran to the end and the committed bytes differ from the
//!                       regenerated ones (the only thing that exits 1; oracle: "MISMATCH")
//!   2  refused / error  generation refused (any GenError: a level or feature beyond ir.core,
//!                       a profile or checker error, a rule or limit violation, bindings whose
//!                       faults= lists differ from the profile's), an input that cannot be
//!                       read, a path that is refused, an I/O error while writing the output,
//!                       or a usage error (oracle: OracleError and argparse both give 2)
//! ```
//!
//! The split is by **where** the error arose, not by the `GenError` code alone: the `drift`
//! code is also raised by the bindings cross-check, which is a refusal (nothing is compared,
//! nothing is written), so it exits 2.  Only [`crate::fsio::check_corpus`]'s own `drift`
//! finding, in `--check` mode, is a drift.  A file that is simply missing from `--out` is
//! listed among that finding's differences (the file set is part of what is compared), so it
//! is a drift too; the oracle's `dump-lowering --check` has a single committed file and treats
//! an unreadable one as 2, which has no counterpart for a directory.  A directory that cannot
//! be listed at all is an I/O error: 2.
//!
//! The binary passes `run`'s value through unchanged (see `src/bin`).
//!
//! [`run`] takes its arguments and writers so tests drive it without a process; the binary
//! in `src/bin` is a thin wrapper.  The generator is a *dev tool* with no
//! `test-hooks` feature linked (that feature is dev-only).

use std::io::Write;
use std::path::Path;

use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

use crate::error::{code, GenError};
use crate::fsio::{check_corpus, read_regular, write_corpus};
use crate::{generate, GenInput};

/// The most bytes read from any input file.
pub const MAX_INPUT_BYTES: u64 = 4 * 1024 * 1024;

const USAGE: &str = "usage: warrant-corpus-gen --warrant <path> --profile <path> --out <dir> [--check] [--bindings <path>]";

/// The `faults=` lists of an `evidence.bindings` file: `op <name> ... faults=a,b ...` lines.
pub fn parse_binding_faults(text: &str) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut toks = line.split_whitespace();
        if toks.next() != Some("op") {
            continue;
        }
        let Some(name) = toks.next() else { continue };
        let faults = toks
            .find_map(|t| t.strip_prefix("faults="))
            .map(|l| l.split(',').filter(|s| !s.is_empty()).map(str::to_string).collect())
            .unwrap_or_default();
        out.push((name.to_string(), faults));
    }
    out
}

fn read_capped(path: &str) -> Result<Vec<u8>, GenError> {
    let at = |m: &str| GenError::new(code::IO, path, m);
    // the caller's own input: a link is followed, but only a regular file is read, through a
    // handle, and never more than the cap
    let (bytes, len) = read_regular(Path::new(path), MAX_INPUT_BYTES, true).map_err(|e| {
        at(if e.message.contains("regular") {
            "is not a regular file"
        } else {
            "cannot be read"
        })
    })?;
    if len > MAX_INPUT_BYTES || bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err(at("is over the size cap"));
    }
    Ok(bytes)
}

fn spec_path_ok(p: &str) -> bool {
    !p.is_empty()
        && !p.starts_with('/')
        && !p.contains('\\')
        && !p.contains(':')
        && p.split('/').all(|c| !c.is_empty() && c != "." && c != "..")
}

struct Args {
    warrant: String,
    profile: String,
    out: String,
    check: bool,
    bindings: Option<String>,
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let (mut warrant, mut profile, mut out, mut bindings) = (None, None, None, None);
    let mut check = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match a.as_str() {
            "--check" => check = true,
            "--warrant" => warrant = Some(value("--warrant")?),
            "--profile" => profile = Some(value("--profile")?),
            "--out" => out = Some(value("--out")?),
            "--bindings" => bindings = Some(value("--bindings")?),
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    Ok(Args {
        warrant: warrant.ok_or("--warrant is required")?,
        profile: profile.ok_or("--profile is required")?,
        out: out.ok_or("--out is required")?,
        check,
        bindings,
    })
}

/// Why [`go`] did not succeed, which decides the exit status (see the module documentation).
enum Failure {
    /// `--check` compared and the bytes differ: exit 1.
    Drift(GenError),
    /// Anything that stopped the tool before or instead of a comparison: exit 2.
    Refused(GenError),
}

/// Exit status of a `--check` drift.
pub const EXIT_DRIFT: i32 = 1;
/// Exit status of a refusal, an I/O error or a usage error.
pub const EXIT_REFUSED: i32 = 2;

impl From<GenError> for Failure {
    /// The `?` operator turns every ordinary error into a refusal; only the one comparison
    /// site builds [`Failure::Drift`] explicitly.
    fn from(e: GenError) -> Self {
        Failure::Refused(e)
    }
}

fn go(a: &Args, err: &mut dyn Write) -> Result<String, Failure> {
    if !spec_path_ok(&a.warrant) {
        return Err(GenError::new(
            code::IO,
            "--warrant",
            "must be a repo-relative path without `..`, `.`, `\\`, `:` or a leading `/` (it is recorded in the corpus)",
        )
        .into());
    }
    if a.bindings.is_none() {
        let _ = writeln!(err, "warrant-corpus-gen: note: --bindings not given, the profile's faults are not checked against an evidence.bindings");
    }
    let spec = read_capped(&a.warrant)?;
    let profile = read_capped(&a.profile)?;
    let bindings = match &a.bindings {
        Some(p) => Some(parse_binding_faults(&String::from_utf8_lossy(&read_capped(p)?))),
        None => None,
    };
    let doc = SourceDocument::admit(&spec, SourceLimits::default())
        .map_err(|e| GenError::new(code::CHECK, "--warrant", format!("{e:?}")))?;
    let layout = capture_layout(&doc, LayoutLimits::default())
        .map_err(|e| GenError::new(code::CHECK, "--warrant", format!("{e:?}")))?;
    let file = parse_document(&layout, &doc, ParserLimits::default())
        .map_err(|e| GenError::new(code::CHECK, "--warrant", format!("{e:?}")))?;
    let corpus = generate(&GenInput {
        file: &file,
        spec_path: &a.warrant,
        spec_bytes: &spec,
        profile_bytes: &profile,
        binding_faults: bindings.as_deref(),
    })?;
    let out = Path::new(&a.out);
    if a.check {
        // the one place a drift is told from a refusal: only the comparison's own `drift`
        // finding exits 1; an I/O error while listing or reading is a refusal
        check_corpus(out, &corpus).map_err(|e| {
            if e.code == code::DRIFT {
                Failure::Drift(e)
            } else {
                Failure::Refused(e)
            }
        })?;
        Ok(format!("ok: {} matches the regenerated corpus", a.out))
    } else {
        write_corpus(out, &corpus)?;
        Ok(format!("wrote {} files to {}", corpus.files().len(), a.out))
    }
}

/// Run the command line.  Returns the process exit code: 0, [`EXIT_DRIFT`] or [`EXIT_REFUSED`].
pub fn run(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let parsed = match parse_args(args) {
        Ok(p) => p,
        Err(m) => {
            let _ = writeln!(err, "warrant-corpus-gen: {m}\n{USAGE}");
            return EXIT_REFUSED;
        }
    };
    match go(&parsed, err) {
        Ok(msg) => {
            let _ = writeln!(out, "{msg}");
            0
        }
        Err(Failure::Drift(e)) => {
            let _ = writeln!(err, "warrant-corpus-gen: {e}");
            EXIT_DRIFT
        }
        Err(Failure::Refused(e)) => {
            let _ = writeln!(err, "warrant-corpus-gen: {e}");
            EXIT_REFUSED
        }
    }
}
