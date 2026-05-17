//! # twigc — Twig compiler CLI (TW05-R / LANG73)
//!
//! ```text
//! USAGE:
//!   twigc [OPTIONS] <file.tw>
//!
//! OPTIONS:
//!   --check              Type-check only.  Exit 0 on success, 1 on type errors.
//!   --emit=iir           Compile to IIR and print a human-readable summary.
//!   --search-path=<DIR>  Add DIR to the module search path (repeatable).
//!   -h, --help           Print this help.
//!   -V, --version        Print version.
//!
//! DEFAULT (no flags):
//!   Compile and run via twig-vm; print the integer return value to stdout.
//! ```
//!
//! ## Exit codes
//!
//! | Code | Meaning |
//! |------|---------|
//! | 0    | Success (check passed / IIR printed / run returned) |
//! | 1    | Type error in a `(typed strict)` module |
//! | 2    | Any other compilation error (parse error, missing import, …) |
//! | 3    | Runtime trap from twig-vm |
//! | 4    | Usage error (bad flags, missing file argument) |
//!
//! ## Examples
//!
//! ```text
//! # Type-check only:
//! twigc --check src/main.tw
//!
//! # Dump IIR:
//! twigc --emit=iir src/main.tw
//!
//! # Compile and run:
//! twigc src/main.tw
//!
//! # Multi-module with explicit search path:
//! twigc --search-path=stdlib src/main.tw
//! ```

use std::path::PathBuf;
use std::process;

use twigc::{twigc_check, twigc_emit_iir, twigc_run, TwigcError};
use twig_module_driver::ModuleDriverError;

// ── Version constant ──────────────────────────────────────────────────────────

const VERSION: &str = env!("CARGO_PKG_VERSION");

// ── Usage string ──────────────────────────────────────────────────────────────

const USAGE: &str = "\
USAGE:
  twigc [OPTIONS] <file.tw>

OPTIONS:
  --check              Type-check only.  Exit 0 on success, 1 on type errors.
  --emit=iir           Compile to IIR and print a human-readable listing.
  --search-path=<DIR>  Add DIR to the module search path (repeatable).
  -h, --help           Print this help.
  -V, --version        Print version.

DEFAULT (no flags):
  Compile and run via twig-vm; print the integer return value to stdout.

EXIT CODES:
  0  Success
  1  Type error in a (typed strict) module
  2  Compilation error (parse, import, …)
  3  Runtime trap
  4  Usage error
";

// ── CLI parsing ───────────────────────────────────────────────────────────────

/// The mode of operation selected by the user.
#[derive(Debug, PartialEq)]
enum Mode {
    /// `--check` — type-check only, no execution.
    Check,
    /// `--emit=iir` — dump IIR listing to stdout.
    EmitIir,
    /// Default — compile and run, print integer result.
    Run,
}

/// Parsed command-line arguments.
struct Args {
    mode: Mode,
    file: PathBuf,
    search_paths: Vec<PathBuf>,
}

/// Parse `std::env::args().collect()`.
///
/// Prints usage to stderr and calls `process::exit(4)` on any parse error.
fn parse_args(raw: Vec<String>) -> Args {
    let mut mode = Mode::Run;
    let mut file: Option<PathBuf> = None;
    let mut search_paths: Vec<PathBuf> = Vec::new();

    let mut i = 1usize; // skip argv[0]
    while i < raw.len() {
        let arg = &raw[i];
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                process::exit(0);
            }
            "-V" | "--version" => {
                println!("twigc {VERSION}");
                process::exit(0);
            }
            "--check" => {
                mode = Mode::Check;
            }
            "--emit=iir" => {
                mode = Mode::EmitIir;
            }
            s if s.starts_with("--search-path=") => {
                let dir = s.trim_start_matches("--search-path=");
                search_paths.push(PathBuf::from(dir));
            }
            s if s.starts_with('-') => {
                eprintln!("twigc: unknown option: {s}");
                eprintln!("{USAGE}");
                process::exit(4);
            }
            _ => {
                if file.is_some() {
                    eprintln!("twigc: unexpected positional argument: {arg}");
                    eprintln!("{USAGE}");
                    process::exit(4);
                }
                file = Some(PathBuf::from(arg));
            }
        }
        i += 1;
    }

    let file = match file {
        Some(f) => f,
        None => {
            eprintln!("twigc: missing required argument: <file.tw>");
            eprintln!("{USAGE}");
            process::exit(4);
        }
    };

    Args { mode, file, search_paths }
}

// ── Error reporting ───────────────────────────────────────────────────────────

/// Print a `TwigcError` to stderr and return the appropriate exit code.
fn handle_error(e: &TwigcError) -> i32 {
    match e {
        TwigcError::Driver(ModuleDriverError::TypeErrors { path, errors }) => {
            eprintln!("twigc: type error(s) in {}:", path.display());
            for err in errors.iter().take(5) {
                eprintln!("  {}:{}: {}", err.line, err.column, err.message);
            }
            if errors.len() > 5 {
                eprintln!("  … and {} more", errors.len() - 5);
            }
            1
        }
        TwigcError::Driver(e) => {
            eprintln!("twigc: compilation error: {e}");
            2
        }
        TwigcError::Vm { message } => {
            eprintln!("twigc: runtime error: {message}");
            3
        }
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let args = parse_args(std::env::args().collect());

    let exit_code = match args.mode {
        Mode::Check => match twigc_check(&args.file, &args.search_paths) {
            Ok(()) => {
                // Silent success — exit 0.  Mirrors how `rustc --check-cfg` behaves.
                0
            }
            Err(ref e) => handle_error(e),
        },

        Mode::EmitIir => match twigc_emit_iir(&args.file, &args.search_paths) {
            Ok(listing) => {
                print!("{listing}");
                0
            }
            Err(ref e) => handle_error(e),
        },

        Mode::Run => match twigc_run(&args.file, &args.search_paths) {
            Ok(value) => {
                println!("{value}");
                0
            }
            Err(ref e) => handle_error(e),
        },
    };

    process::exit(exit_code);
}
