//! The `pif-check` command line, as a function so tests can call it in-process.
//!
//! ```text
//!   pif-check --store <dir> --chain <file> (--allow <file> | --signoff <file> | --unsafe-no-allow)
//!             [--allow-no-claim] [--withdrawn-logic <n,n,...>]
//! ```
//!
//! * exit 0: the whole chain was accepted; the report (JSON, one line) is on standard output;
//! * exit 1: the gate refused; a JSON line `{"ok":false,"error":<code>,"detail":...}` is on
//!   standard output, so a machine reads the same channel either way;
//! * exit 2: a usage error; a message is on standard error;
//! * exit 3: the base chain is **unvetted**: neither an allow-list nor a sign-off was given and
//!   `--unsafe-no-allow` was not. The gate fails closed: a base chain that nobody vetted is
//!   not an environment to check a proof in.
//!
//! The report's top-level `"base_trust"` says how the base was vetted: `"signed"` (a sign-off
//! fixed it), `"allow-listed"` (every base file is on the allow-list) or `"unvetted"` (only with
//! `--unsafe-no-allow`).
//!
//! Nothing is written anywhere and nothing is fetched from a network: the only inputs are the
//! files named on the command line and the store directory.

use crate::chain::{load_chain, LoadOptions};
use crate::control::{check_shape, effective_chain, parse_allow, parse_chain, parse_signoff, Role};
use crate::digest::Digest;
use crate::error::CheckError;
use crate::report::{error_report, success_report, Provided};
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

const USAGE: &str = "usage: pif-check --store <dir> --chain <file> (--allow <file> | --signoff <file> | --unsafe-no-allow) [--allow-no-claim] [--withdrawn-logic <n,n,...>]\n";

/// The parsed command line.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Args {
    pub store: String,
    pub chain: String,
    pub signoff: Option<String>,
    pub allow: Option<String>,
    /// Check a base chain that nobody vetted (reported as `"base_trust":"unvetted"`).
    pub unsafe_no_allow: bool,
    /// Accept a chain with no claim file.
    pub allow_no_claim: bool,
    /// Gate configuration: `logic` versions the pin file has withdrawn.
    pub withdrawn_logic: Vec<u32>,
}

/// Parse the command line. Unknown flags, repeats and missing values are usage errors.
pub fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut a = Args::default();
    let (mut store, mut chain, mut withdrawn) = (None, None, None::<String>);
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--unsafe-no-allow" | "--allow-no-claim" => {
                let slot = if flag == "--unsafe-no-allow" {
                    &mut a.unsafe_no_allow
                } else {
                    &mut a.allow_no_claim
                };
                if std::mem::replace(slot, true) {
                    return Err(format!("{flag} given twice"));
                }
                continue;
            }
            _ => {}
        }
        let v = it
            .next()
            .ok_or_else(|| format!("{flag} needs a value"))?
            .clone();
        let slot = match flag.as_str() {
            "--store" => &mut store,
            "--chain" => &mut chain,
            "--signoff" => &mut a.signoff,
            "--allow" => &mut a.allow,
            "--withdrawn-logic" => &mut withdrawn,
            _ => return Err(format!("unknown argument {flag}")),
        };
        if slot.replace(v).is_some() {
            return Err(format!("{flag} given twice"));
        }
    }
    a.store = store.ok_or("--store is required")?;
    a.chain = chain.ok_or("--chain is required")?;
    if let Some(w) = withdrawn {
        a.withdrawn_logic = w
            .split(',')
            .map(|x| {
                x.parse::<u32>()
                    .map_err(|_| format!("bad --withdrawn-logic entry {x:?}"))
            })
            .collect::<Result<_, _>>()?;
    }
    Ok(a)
}

/// Read a control file as UTF-8 text. It must be a regular file (not a symbolic link, a FIFO or
/// a device: opening a FIFO would block forever) and at most [`MAX_CONTROL_BYTES`].
pub fn read_text(what: &'static str, path: &str) -> Result<String, CheckError> {
    let bad = |why| CheckError::BadControlFile { what, line: 0, why };
    // Look before opening: `symlink_metadata` does not follow a link and never blocks.
    let meta = fs::symlink_metadata(Path::new(path)).map_err(|_| bad("cannot be opened"))?;
    if !meta.is_file() {
        return Err(bad("is not a regular file"));
    }
    let f = fs::File::open(Path::new(path)).map_err(|_| bad("cannot be opened"))?;
    // And again on the open handle (fstat), so a swap between the two calls is caught.
    if !f.metadata().map_err(|_| bad("cannot be read"))?.is_file() {
        return Err(bad("is not a regular file"));
    }
    let mut bytes = Vec::new();
    f.take(MAX_CONTROL_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| bad("cannot be read"))?;
    if bytes.len() as u64 > MAX_CONTROL_BYTES {
        return Err(bad("too large"));
    }
    String::from_utf8(bytes).map_err(|_| bad("is not UTF-8"))
}

/// How the base chain was vetted.
fn base_trust(a: &Args) -> Result<&'static str, CheckError> {
    if a.signoff.is_some() {
        Ok("signed")
    } else if a.allow.is_some() {
        Ok("allow-listed")
    } else if a.unsafe_no_allow {
        Ok("unvetted")
    } else {
        Err(CheckError::Unvetted)
    }
}

fn check(a: &Args) -> Result<String, CheckError> {
    // Fail closed before touching any file: no vetting, no check.
    let trust = base_trust(a)?;
    let chain = parse_chain(&read_text("chain", &a.chain)?)?;
    let signoff = match a.signoff.as_deref() {
        Some(p) => Some(parse_signoff(&read_text("sign-off", p)?)?),
        None => None,
    };
    let allow = match a.allow.as_deref() {
        Some(p) => Some(parse_allow(&read_text("allow-list", p)?)?),
        None => None,
    };
    let chain = effective_chain(chain, signoff.as_ref())?;
    check_shape(&chain)?;
    if !a.allow_no_claim && !chain.iter().any(|e| e.role == Role::Claim) {
        return Err(CheckError::NoClaimFile);
    }
    let source = DirSource::new(&a.store);
    let resolver = Resolver::new(&source);
    let opts = LoadOptions {
        allow: allow.as_ref(),
        withdrawn_logic: a.withdrawn_logic.clone(),
    };
    let loaded = load_chain(&resolver, &chain, &opts)?;
    if let Some(expected) = signoff.as_ref().and_then(|s| s.base_environment) {
        let actual: Digest = loaded.base_environment;
        if expected != actual {
            return Err(CheckError::SignoffBaseEnvironmentMismatch {
                expected: expected.hex(),
                actual: actual.hex(),
            });
        }
    }
    let provided = Provided {
        allow_list: allow.is_some(),
        signoff: signoff.is_some(),
        base_trust: trust,
    };
    Ok(success_report(&loaded, &provided).render())
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
            code: if e == CheckError::Unvetted { 3 } else { 1 },
            stdout: format!("{}\n", error_report(&e).render()),
            stderr: String::new(),
        },
    }
}
