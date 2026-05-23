//! `closurec` — CLI driver for the Closure Compiler clone.
//!
//! Per [CLOC08](../../../../specs/CLOC08-closurec-cli.md). The
//! binary that ties the whole pipeline together: reads input
//! JavaScript, runs the configured optimization pass pipeline,
//! emits output JavaScript plus an optional source-map blob.
//!
//! # Stage 4 wrap-up: the role this binary plays
//!
//! ```text
//!   ┌──────────────────────────────────────────────────────────┐
//!   │ closurec                                                 │
//!   │                                                          │
//!   │  args ─► parse ──► [lex] ──► [parse] ──► [typecheck] ──► │
//!   │                                                          │
//!   │   ──► [pass pipeline] ──► [emit] ──► (out.js, out.js.map)│
//!   └──────────────────────────────────────────────────────────┘
//! ```
//!
//! Each `[bracketed]` stage is one of the crates we've scaffolded
//! in Stages 1–4. This binary is just the glue.
//!
//! # Scope (v1)
//!
//! `javascript-ast` ships only `Program` / `SourceType` today and
//! every pass + the emitter are identity. So v1 of `closurec`:
//!
//! - parses the CLI surface so users can already script against
//!   the binary,
//! - validates argument combinations and errors out clearly on
//!   bad input,
//! - prints `closurec v0.1.0 - identity pipeline` to stdout on
//!   the happy path,
//! - exits with status 0 on success, 2 on usage error
//!   (per POSIX's "command-line usage" convention).
//!
//! The actual lex/parse/typecheck/passes/emit wiring lands when
//! the AST grows nodes. Pinning the CLI now lets shell scripts,
//! build systems, and the test harness all link against a
//! stable surface today.
//!
//! # The CLI surface (frozen here)
//!
//! ```text
//! closurec [OPTIONS] --input PATH
//!
//! Required:
//!   --input PATH            Path to the input JavaScript file.
//!
//! Optional:
//!   --output PATH           Path to write compiled output.
//!                           Defaults to stdout when omitted.
//!   --source-map BOOL       Emit a companion source-map blob.
//!                           Default: true.
//!   --ascii-only BOOL       Escape non-ASCII characters in output.
//!                           Default: false (UTF-8 output).
//!   --pretty BOOL           Emit human-readable output with
//!                           whitespace. Default: false (minified).
//!   --disable NAME          Disable a pass by canonical name.
//!                           Repeatable.
//!   --help, -h              Print this help text and exit 0.
//!   --version, -V           Print "closurec 0.1.0" and exit 0.
//! ```
//!
//! BOOL values accept `true|false|1|0|yes|no` (case-insensitive).
//!
//! No third-party clap dependency in v1 — `std::env::args` plus
//! the small parser in [`parse_args`] is enough to cover this
//! surface and stays cheap on cold-start.

use std::process::ExitCode;

/// Canonical pass names for `--disable`. Mirrors the pass set
/// from CLOC06. We don't strictly validate the list (the user
/// can disable a pass that doesn't exist; that's a no-op rather
/// than an error) but the list is the documented input.
const KNOWN_PASSES: &[&str] = &[
    "constant-fold",
    "fold-control-flow",
    "dce",
    "inline",
    "rename",
    "treeshake",
    "collapse-properties",
    "remove-unused-vars",
];

/// Result of parsing `argv`. Either we have a coherent
/// invocation ([`Action`]) or the user did something wrong
/// ([`UsageError`] — exit code 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseResult {
    Run(Action),
    UsageError(String),
}

/// What `closurec` was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// `--help` / `-h`: print usage and exit 0.
    PrintHelp,
    /// `--version` / `-V`: print version and exit 0.
    PrintVersion,
    /// Compile a file. v1 still just prints the identity-pipeline
    /// banner; the fields here are what v2+ will consume.
    Compile(CompileArgs),
}

/// Fully-resolved compile invocation. Validation has already
/// happened (input path is set, BOOL flags parsed, etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileArgs {
    /// Path to the input file. Required.
    pub input: String,
    /// Path to the output file, or `None` for stdout.
    pub output: Option<String>,
    /// Emit a source-map blob alongside the output.
    pub source_map: bool,
    /// Escape non-ASCII characters in the emitted JavaScript.
    pub ascii_only: bool,
    /// Emit human-readable whitespace.
    pub pretty: bool,
    /// Passes the user explicitly disabled (subset of
    /// [`KNOWN_PASSES`], but we don't validate; unknown names
    /// no-op).
    pub disabled_passes: Vec<String>,
}

impl Default for CompileArgs {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: None,
            source_map: true,
            ascii_only: false,
            pretty: false,
            disabled_passes: Vec::new(),
        }
    }
}

/// Parse `argv[1..]` (i.e., the args *without* the program
/// name) into a [`ParseResult`].
///
/// Pure function: no I/O, no global state. Tests drive it
/// directly without touching `std::env::args`.
pub fn parse_args(args: &[String]) -> ParseResult {
    // Handle --help / --version up front. They short-circuit
    // even if there are other args, matching `gcc --help`,
    // `cargo --help`, etc.
    for a in args.iter() {
        if a == "--help" || a == "-h" {
            return ParseResult::Run(Action::PrintHelp);
        }
        if a == "--version" || a == "-V" {
            return ParseResult::Run(Action::PrintVersion);
        }
    }

    let mut compile = CompileArgs::default();
    let mut input_seen = false;

    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "--input" => {
                let value = match args.get(i + 1) {
                    Some(v) => v,
                    None => {
                        return ParseResult::UsageError(
                            "--input requires a path argument".to_string(),
                        );
                    }
                };
                compile.input = value.clone();
                input_seen = true;
                i += 2;
            }
            "--output" => {
                let value = match args.get(i + 1) {
                    Some(v) => v,
                    None => {
                        return ParseResult::UsageError(
                            "--output requires a path argument".to_string(),
                        );
                    }
                };
                compile.output = Some(value.clone());
                i += 2;
            }
            "--source-map" => {
                let value = match args.get(i + 1) {
                    Some(v) => v,
                    None => {
                        return ParseResult::UsageError(
                            "--source-map requires a bool argument".to_string(),
                        );
                    }
                };
                match parse_bool(value) {
                    Some(b) => compile.source_map = b,
                    None => {
                        return ParseResult::UsageError(format!(
                            "--source-map expects a bool (true|false|1|0|yes|no), got {:?}",
                            value
                        ));
                    }
                }
                i += 2;
            }
            "--ascii-only" => {
                let value = match args.get(i + 1) {
                    Some(v) => v,
                    None => {
                        return ParseResult::UsageError(
                            "--ascii-only requires a bool argument".to_string(),
                        );
                    }
                };
                match parse_bool(value) {
                    Some(b) => compile.ascii_only = b,
                    None => {
                        return ParseResult::UsageError(format!(
                            "--ascii-only expects a bool (true|false|1|0|yes|no), got {:?}",
                            value
                        ));
                    }
                }
                i += 2;
            }
            "--pretty" => {
                let value = match args.get(i + 1) {
                    Some(v) => v,
                    None => {
                        return ParseResult::UsageError(
                            "--pretty requires a bool argument".to_string(),
                        );
                    }
                };
                match parse_bool(value) {
                    Some(b) => compile.pretty = b,
                    None => {
                        return ParseResult::UsageError(format!(
                            "--pretty expects a bool (true|false|1|0|yes|no), got {:?}",
                            value
                        ));
                    }
                }
                i += 2;
            }
            "--disable" => {
                let value = match args.get(i + 1) {
                    Some(v) => v,
                    None => {
                        return ParseResult::UsageError(
                            "--disable requires a pass name".to_string(),
                        );
                    }
                };
                compile.disabled_passes.push(value.clone());
                i += 2;
            }
            other => {
                return ParseResult::UsageError(format!("unknown argument {:?}", other));
            }
        }
    }

    if !input_seen {
        return ParseResult::UsageError(
            "--input is required (use --help for usage)".to_string(),
        );
    }

    ParseResult::Run(Action::Compile(compile))
}

/// Parse a flag value into a `bool`. Accepts the common
/// shell-friendly synonyms. Returns `None` on anything else so
/// the caller can produce a sensible error.
fn parse_bool(s: &str) -> Option<bool> {
    match s.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

/// The help text printed for `--help`. Pulled out so tests can
/// assert on it and so the format string is in exactly one
/// place.
pub fn help_text() -> &'static str {
    "closurec — Closure Compiler clone (CLOC08)\n\
     \n\
     Usage:\n  closurec [OPTIONS] --input PATH\n\
     \n\
     Required:\n  --input PATH            Path to the input JavaScript file.\n\
     \n\
     Optional:\n  --output PATH           Path to write compiled output (default: stdout).\n  \
     --source-map BOOL       Emit a companion source-map blob (default: true).\n  \
     --ascii-only BOOL       Escape non-ASCII characters in output (default: false).\n  \
     --pretty BOOL           Emit human-readable output with whitespace (default: false).\n  \
     --disable NAME          Disable a pass by canonical name (repeatable).\n  \
     --help, -h              Print this help text and exit 0.\n  \
     --version, -V           Print version and exit 0.\n\
     \n\
     Known passes (per CLOC06 canonical order):\n  \
     constant-fold, fold-control-flow, dce, inline, rename, treeshake,\n  \
     collapse-properties, remove-unused-vars.\n\
     \n\
     v0.1.0 is scaffolding — the pipeline is identity until the AST grows nodes.\n"
}

/// Returns the bin's canonical version string. Single source of
/// truth so the test, the `--version` handler, and any future
/// telemetry agree.
pub fn version_string() -> &'static str {
    concat!("closurec ", env!("CARGO_PKG_VERSION"))
}

/// Render a [`ParseResult`] into stdout text + exit code.
///
/// Pure function: no real I/O. Tests can call it directly and
/// inspect both outputs. `main` is a thin wrapper that just
/// prints + exits.
pub fn run(result: ParseResult) -> (String, ExitCode) {
    match result {
        ParseResult::Run(Action::PrintHelp) => (help_text().to_string(), ExitCode::SUCCESS),
        ParseResult::Run(Action::PrintVersion) => {
            (format!("{}\n", version_string()), ExitCode::SUCCESS)
        }
        ParseResult::Run(Action::Compile(_args)) => {
            // v1: identity pipeline. Print the banner so users
            // know the binary loaded and parsed its args; real
            // compilation lands when the AST grows nodes.
            //
            // We deliberately don't read `_args.input` yet —
            // there'd be no point opening a file we can't
            // actually compile. Refusing now would also mean
            // every test had to set up a real file path.
            //
            // Once compilation lands, `_args.input` becomes the
            // first thing we touch and a missing-file error will
            // surface here.
            (
                "closurec v0.1.0 - identity pipeline\n".to_string(),
                ExitCode::SUCCESS,
            )
        }
        ParseResult::UsageError(msg) => {
            // Conventional POSIX exit for misuse is 2 (1 is
            // reserved for compilation failure, etc.).
            (
                format!("error: {}\nuse --help for usage\n", msg),
                ExitCode::from(2),
            )
        }
    }
}

/// Binary entry point.
fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let result = parse_args(&argv);
    let (text, code) = run(result);
    print!("{}", text);
    code
}

#[cfg(test)]
mod tests {
    //! These tests cover the parser and the renderer. The
    //! parser is a pure function over `&[String]`, so we can
    //! drive it directly without spawning the binary. That
    //! keeps tests fast and deterministic.
    use super::*;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn help_short_flag_returns_print_help() {
        assert_eq!(
            parse_args(&args(&["-h"])),
            ParseResult::Run(Action::PrintHelp)
        );
    }

    #[test]
    fn help_long_flag_returns_print_help() {
        assert_eq!(
            parse_args(&args(&["--help"])),
            ParseResult::Run(Action::PrintHelp)
        );
    }

    #[test]
    fn version_short_flag_returns_print_version() {
        assert_eq!(
            parse_args(&args(&["-V"])),
            ParseResult::Run(Action::PrintVersion)
        );
    }

    #[test]
    fn version_long_flag_returns_print_version() {
        assert_eq!(
            parse_args(&args(&["--version"])),
            ParseResult::Run(Action::PrintVersion)
        );
    }

    #[test]
    fn help_short_circuits_even_with_other_args() {
        // `gcc --help` etc. all short-circuit; matching that.
        assert_eq!(
            parse_args(&args(&["--input", "a.js", "--help"])),
            ParseResult::Run(Action::PrintHelp)
        );
    }

    #[test]
    fn missing_input_is_usage_error() {
        match parse_args(&args(&["--pretty", "true"])) {
            ParseResult::UsageError(msg) => {
                assert!(
                    msg.contains("--input"),
                    "expected --input mention in error; got {:?}",
                    msg
                );
            }
            other => panic!("expected UsageError; got {:?}", other),
        }
    }

    #[test]
    fn unknown_flag_is_usage_error() {
        match parse_args(&args(&["--input", "a.js", "--bogus"])) {
            ParseResult::UsageError(msg) => {
                assert!(
                    msg.contains("--bogus"),
                    "expected --bogus mention in error; got {:?}",
                    msg
                );
            }
            other => panic!("expected UsageError; got {:?}", other),
        }
    }

    #[test]
    fn input_only_sets_defaults() {
        // Minimum legal invocation: just --input.
        let r = parse_args(&args(&["--input", "src.js"]));
        match r {
            ParseResult::Run(Action::Compile(a)) => {
                assert_eq!(a.input, "src.js");
                assert_eq!(a.output, None);
                assert!(a.source_map); // default true
                assert!(!a.ascii_only); // default false
                assert!(!a.pretty); // default false
                assert!(a.disabled_passes.is_empty());
            }
            other => panic!("expected Compile; got {:?}", other),
        }
    }

    #[test]
    fn all_options_round_trip() {
        let r = parse_args(&args(&[
            "--input",
            "in.js",
            "--output",
            "out.js",
            "--source-map",
            "false",
            "--ascii-only",
            "true",
            "--pretty",
            "yes",
            "--disable",
            "dce",
            "--disable",
            "rename",
        ]));
        match r {
            ParseResult::Run(Action::Compile(a)) => {
                assert_eq!(a.input, "in.js");
                assert_eq!(a.output.as_deref(), Some("out.js"));
                assert!(!a.source_map);
                assert!(a.ascii_only);
                assert!(a.pretty); // "yes" parses true
                assert_eq!(a.disabled_passes, vec!["dce", "rename"]);
            }
            other => panic!("expected Compile; got {:?}", other),
        }
    }

    #[test]
    fn bool_synonyms_all_parse() {
        // Lock the synonym set so a future refactor doesn't
        // accidentally drop one.
        for v in &["true", "True", "TRUE", "1", "yes", "on"] {
            assert_eq!(parse_bool(v), Some(true), "true synonym {:?}", v);
        }
        for v in &["false", "False", "FALSE", "0", "no", "off"] {
            assert_eq!(parse_bool(v), Some(false), "false synonym {:?}", v);
        }
        assert_eq!(parse_bool("nope"), None);
        assert_eq!(parse_bool(""), None);
    }

    #[test]
    fn invalid_bool_is_usage_error() {
        match parse_args(&args(&["--input", "a.js", "--pretty", "maybe"])) {
            ParseResult::UsageError(msg) => {
                assert!(msg.contains("--pretty"));
                assert!(msg.contains("\"maybe\""));
            }
            other => panic!("expected UsageError; got {:?}", other),
        }
    }

    #[test]
    fn flag_without_value_is_usage_error() {
        // --input at end of argv.
        match parse_args(&args(&["--input"])) {
            ParseResult::UsageError(msg) => {
                assert!(msg.contains("--input"));
            }
            other => panic!("expected UsageError; got {:?}", other),
        }
    }

    #[test]
    fn empty_argv_is_usage_error_missing_input() {
        match parse_args(&args(&[])) {
            ParseResult::UsageError(msg) => {
                assert!(msg.contains("--input"));
            }
            other => panic!("expected UsageError; got {:?}", other),
        }
    }

    #[test]
    fn help_text_mentions_all_known_passes() {
        // The known-pass list is documented; if a pass is added
        // to KNOWN_PASSES but missed in the help text, this
        // catches it.
        let h = help_text();
        for p in KNOWN_PASSES {
            assert!(
                h.contains(p),
                "help text should mention pass {:?}; got:\n{}",
                p,
                h
            );
        }
    }

    #[test]
    fn version_string_includes_crate_version() {
        // env!("CARGO_PKG_VERSION") matches Cargo.toml's
        // version = "0.1.0" today; if Cargo.toml moves to 0.2.0
        // this test re-evaluates against the new value.
        let v = version_string();
        assert!(v.starts_with("closurec "));
        assert!(v.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn run_print_help_returns_success_and_help() {
        let (text, _code) = run(ParseResult::Run(Action::PrintHelp));
        assert!(text.contains("Usage:"));
        // Can't easily assert on ExitCode equality (it doesn't
        // impl PartialEq), but ExitCode::SUCCESS is what we
        // return.
    }

    #[test]
    fn run_print_version_returns_success_and_version() {
        let (text, _code) = run(ParseResult::Run(Action::PrintVersion));
        assert!(text.contains("0.1.0"));
        assert!(text.contains("closurec"));
    }

    #[test]
    fn run_compile_v1_prints_identity_banner() {
        let (text, _code) = run(ParseResult::Run(Action::Compile(CompileArgs {
            input: "x.js".to_string(),
            ..Default::default()
        })));
        assert!(text.contains("identity pipeline"));
        assert!(text.contains("v0.1.0"));
    }

    #[test]
    fn run_usage_error_returns_nonzero_and_message() {
        let (text, _code) =
            run(ParseResult::UsageError("--input is required".to_string()));
        assert!(text.contains("--input"));
        assert!(text.contains("use --help"));
    }
}
