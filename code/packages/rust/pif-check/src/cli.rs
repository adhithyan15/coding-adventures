//! The `pif-check` command line, as a function so tests can call it in-process.
//!
//! ```text
//!   pif-check --store <dir> --chain <file> [--signoff <file>] [--allow <file>]
//! ```
//!
//! * exit 0: the whole chain was accepted; the report (JSON, one line) is on standard output;
//! * exit 1: the gate refused; a JSON line `{"ok":false,"error":<code>,"detail":...}` is on
//!   standard output, so a machine reads the same channel either way;
//! * exit 2: a usage error; a message is on standard error.
//!
//! Nothing is written anywhere and nothing is fetched from a network: the only inputs are the
//! files named on the command line and the store directory.

use crate::chain::{load_chain, LoadOptions};
use crate::control::{check_shape, effective_chain, parse_allow, parse_chain, parse_signoff};
use crate::digest::Digest;
use crate::error::CheckError;
use crate::report::{error_report, success_report};
use crate::resolver::{DirSource, Resolver};
use std::fs;
use std::io::Read;
use std::path::Path;

/// What running the tool produced.
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// The size cap on a control file (chain, sign-off, allow-list).
pub const MAX_CONTROL_BYTES: u64 = 16 << 20;

const USAGE: &str =
    "usage: pif-check --store <dir> --chain <file> [--signoff <file>] [--allow <file>]\n";

/// The parsed command line.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Args {
    pub store: String,
    pub chain: String,
    pub signoff: Option<String>,
    pub allow: Option<String>,
}

/// Parse `--flag value` pairs. Unknown flags, repeats and missing values are usage errors.
pub fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut a = Args::default();
    let (mut store, mut chain) = (None, None);
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        let v = it
            .next()
            .ok_or_else(|| format!("{flag} needs a value"))?
            .clone();
        let slot = match flag.as_str() {
            "--store" => &mut store,
            "--chain" => &mut chain,
            "--signoff" => &mut a.signoff,
            "--allow" => &mut a.allow,
            _ => return Err(format!("unknown argument {flag}")),
        };
        if slot.replace(v).is_some() {
            return Err(format!("{flag} given twice"));
        }
    }
    a.store = store.ok_or("--store is required")?;
    a.chain = chain.ok_or("--chain is required")?;
    Ok(a)
}

/// Read a control file as UTF-8 text, refusing anything over [`MAX_CONTROL_BYTES`].
pub fn read_text(what: &'static str, path: &str) -> Result<String, CheckError> {
    let bad = |why| CheckError::BadControlFile { what, line: 0, why };
    let f = fs::File::open(Path::new(path)).map_err(|_| bad("cannot be opened"))?;
    let mut bytes = Vec::new();
    f.take(MAX_CONTROL_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| bad("cannot be read"))?;
    if bytes.len() as u64 > MAX_CONTROL_BYTES {
        return Err(bad("too large"));
    }
    String::from_utf8(bytes).map_err(|_| bad("is not UTF-8"))
}

fn check(a: &Args) -> Result<String, CheckError> {
    let chain = parse_chain(&read_text("chain", &a.chain)?)?;
    let signoff = a
        .signoff
        .as_deref()
        .map(|p| read_text("sign-off", p).and_then(|t| parse_signoff(&t)))
        .transpose()?;
    let allow = a
        .allow
        .as_deref()
        .map(|p| read_text("allow-list", p).and_then(|t| parse_allow(&t)))
        .transpose()?;
    let chain = effective_chain(chain, signoff.as_ref())?;
    check_shape(&chain)?;
    let source = DirSource::new(&a.store);
    let resolver = Resolver::new(&source);
    let loaded = load_chain(
        &resolver,
        &chain,
        &LoadOptions {
            allow: allow.as_ref(),
        },
    )?;
    if let Some(expected) = signoff.as_ref().and_then(|s| s.base_environment) {
        let actual: Digest = loaded.base_environment;
        if expected != actual {
            return Err(CheckError::SignoffBaseEnvironmentMismatch {
                expected: expected.hex(),
                actual: actual.hex(),
            });
        }
    }
    Ok(success_report(&loaded, allow.is_some()).render())
}

/// Run the tool on `args` (without the program name).
pub fn run(args: &[String]) -> Outcome {
    let parsed = match parse_args(args) {
        Ok(a) => a,
        Err(e) => {
            return Outcome {
                code: 2,
                stdout: String::new(),
                stderr: format!("pif-check: {e}\n{USAGE}"),
            }
        }
    };
    match check(&parsed) {
        Ok(line) => Outcome {
            code: 0,
            stdout: format!("{line}\n"),
            stderr: String::new(),
        },
        Err(e) => Outcome {
            code: 1,
            stdout: format!("{}\n", error_report(&e).render()),
            stderr: String::new(),
        },
    }
}
