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
//!   exit 0  success        exit 1  refused or drifted        exit 2  usage error
//! ```
//!
//! [`run`] takes its arguments and writers so tests drive it without a process; the binary
//! in `src/bin` is a four-line wrapper.  The generator is a *dev tool* with no
//! `test-hooks` feature linked (that feature is dev-only).

use std::io::Write;
use std::path::Path;

use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

use crate::error::{code, GenError};
use crate::fsio::{check_corpus, write_corpus};
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
    let meta = std::fs::metadata(path).map_err(|_| GenError::new(code::IO, path, "cannot be read"))?;
    if !meta.is_file() || meta.len() > MAX_INPUT_BYTES {
        return Err(GenError::new(code::IO, path, "is not a regular file within the size cap"));
    }
    std::fs::read(path).map_err(|_| GenError::new(code::IO, path, "cannot be read"))
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

fn go(a: &Args) -> Result<String, GenError> {
    if !spec_path_ok(&a.warrant) {
        return Err(GenError::new(
            code::IO,
            "--warrant",
            "must be a repo-relative path without `..`, `.`, `\\`, `:` or a leading `/` (it is recorded in the corpus)",
        ));
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
        check_corpus(out, &corpus)?;
        Ok(format!("ok: {} matches the regenerated corpus", a.out))
    } else {
        write_corpus(out, &corpus)?;
        Ok(format!("wrote {} files to {}", corpus.files().len(), a.out))
    }
}

/// Run the command line.  Returns the process exit code.
pub fn run(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let parsed = match parse_args(args) {
        Ok(p) => p,
        Err(m) => {
            let _ = writeln!(err, "warrant-corpus-gen: {m}\n{USAGE}");
            return 2;
        }
    };
    match go(&parsed) {
        Ok(msg) => {
            let _ = writeln!(out, "{msg}");
            0
        }
        Err(e) => {
            let _ = writeln!(err, "warrant-corpus-gen: {e}");
            1
        }
    }
}
