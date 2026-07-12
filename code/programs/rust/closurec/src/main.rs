//! `closurec` — CLI driver for the Closure Compiler clone.
//!
//! Per [CLOC08](../../../../specs/CLOC08-closurec-cli-surface.md).
//! The binary that ties the whole pipeline together: reads input
//! JavaScript, runs the configured optimization pass pipeline,
//! emits output JavaScript plus an optional source-map blob.
//!
//! # Drop-in compatibility with the Java Closure Compiler
//!
//! The CLI flag surface mirrors the upstream Java Closure Compiler
//! (`google/closure-compiler`, see `CommandLineRunner.java`) flag
//! for flag. The goal is **drop-in compatibility**: a script
//! written against `java -jar closure-compiler.jar --js foo.js
//! --js_output_file out.js --compilation_level ADVANCED` should
//! work unchanged when the `java -jar …` invocation is swapped for
//! `closurec`.
//!
//! The flag set is declared in [`cli.spec.json`](../cli.spec.json),
//! a [cli-builder](../../../packages/rust/cli-builder) JSON
//! spec. The spec is embedded into the binary via
//! `include_str!` at compile time — no runtime file lookup is
//! required for the parser to come up.
//!
//! # Scope (v1)
//!
//! `javascript-ast` ships only `Program` / `SourceType` today and
//! every pass + the emitter are identity. So v1 of `closurec`:
//!
//! - parses every Closure Compiler flag (validation, type
//!   checking, repeatable handling, enum values) so users can
//!   already script against the binary the same way they would
//!   the Java tool,
//! - returns clear errors on misuse (cli-builder collects every
//!   error in a single pass and offers fuzzy "did you mean?"
//!   suggestions on unknown flags),
//! - on a valid invocation, prints
//!   `closurec v0.1.0 - identity pipeline\n` and exits 0,
//! - exits with status 1 on parse error (cli-builder convention;
//!   matches what users expect from a CLI parser).
//!
//! The actual lex/parse/typecheck/passes/emit wiring lands when
//! the AST grows nodes. Pinning the CLI surface now lets shell
//! scripts and CI configs link against a stable, Closure-Compiler-
//! compatible surface today.
//!
//! # Architecture
//!
//! ```text
//!   args ─► cli_builder::Parser::parse ──► ParserOutput
//!                                            │
//!                                            ├─ Parse(r)   ──► run_pipeline(r)   (v1: banner)
//!                                            ├─ Help(h)    ──► h.text
//!                                            └─ Version(v) ──► v.version
//! ```
//!
//! `parse_and_run(args)` is a pure function returning
//! `(text, ExitCode)` so tests can exercise the whole pipeline
//! without spawning the binary. `main` is a thin wrapper.

// This crate is deliberately documentation-heavy (Knuth-style literate
// programming): the module and item docs use prose paragraphs that follow
// numbered/bulleted lists. clippy 1.97's doc-list-continuation lints flag those
// as mis-indented list items; the formatting is intentional, so allow them
// crate-wide rather than reflow authored prose.
#![allow(
    clippy::doc_lazy_continuation,
    clippy::doc_overindented_list_items
)]

use cli_builder::types::ParserOutput;
use cli_builder::{load_spec_from_str, Parser};
use std::process::ExitCode;

// CLOC11.01 added these three modules to give the previously-empty
// CLI binary an actual body. See CLOC11 §5 for the architecture.
// CLOC11.02 added `globs` for --js pattern expansion.
// CLOC11.06 added `whitespace_only` for --compilation_level WHITESPACE_ONLY.
// CLOC11.19 added `defines` for --define / -D value substitution.
// CLOC11.30 added `wrapper` for --output_wrapper / --output_wrapper_file.
pub mod charset;
pub mod config;
pub mod defines;
pub mod globs;
pub mod help_markdown;
pub mod print_tree;
pub mod run;
pub mod source_map;
pub mod whitespace_only;
pub mod wire;
pub mod wrapper;

/// The cli-builder JSON spec, embedded at compile time. This is
/// the single source of truth for closurec's flag surface; it
/// declares every flag from `CommandLineRunner.java` plus their
/// types, enum values, defaults, and conflict rules.
const CLI_SPEC_JSON: &str = include_str!("../cli.spec.json");

/// Run the CLI from a fresh `argv` (without the program name).
///
/// Returns the text that should go to stdout (or stderr in the
/// error case — caller decides) and the exit code. Pure function:
/// no I/O, no global state, no `std::env::args()` access. Tests
/// drive it directly.
///
/// Exit codes follow standard CLI convention:
/// - `0` for success (Parse, Help, Version).
/// - `1` for parse errors (unknown flag, invalid value, missing
///   required flag, conflicting flags, etc.).
///
/// We don't use the Closure Java tool's exit code 2 / 3 / 4
/// distinctions in v1 — `2` is reserved for usage error,
/// `3` for compilation error, `4` for IO error once those paths
/// actually exist. Stick with 0/1 for now.
pub fn parse_and_run(args: &[String]) -> (String, ExitCode) {
    let (stdout, stderr, code) = parse_and_run_with_streams(args);
    // Back-compat shim: combine stdout + stderr into a single
    // string for callers that don't care about routing. Tests
    // that need to assert on stderr separately call
    // `parse_and_run_with_streams` directly.
    let mut combined = stdout;
    combined.push_str(&stderr);
    (combined, code)
}

/// CLOC11.75 — version of `parse_and_run` that splits the
/// output into stdout-bound and stderr-bound text. `main`
/// uses this to route the CV summary line to stderr when
/// `--correlation_vector_summary_stderr` is set, without
/// corrupting an stdout-bound JS payload.
///
/// All call sites other than `main` and the CLOC11.75 tests
/// can keep using `parse_and_run` — its return value
/// (stdout + stderr concatenated, plus exit code) is the
/// same byte-for-byte as before for runs without
/// `--correlation_vector_summary_stderr`.
pub fn parse_and_run_with_streams(args: &[String]) -> (String, String, ExitCode) {
    // Step 1: load the spec. This is cheap (serde_json parses the
    // ~9 KB spec in microseconds) and runs once per invocation.
    // We could lazy_static it for repeated CLI loops, but the
    // binary only ever processes one argv per run.
    let spec = match load_spec_from_str(CLI_SPEC_JSON) {
        Ok(s) => s,
        Err(e) => {
            // A malformed cli.spec.json is a bug in *us*, not
            // user error. Still surface it so users have a hope
            // of reporting it.
            return (
                format!("internal error: cli.spec.json failed to load: {}\n", e),
                String::new(),
                ExitCode::from(70), // EX_SOFTWARE per sysexits.h
            );
        }
    };

    // Step 2: parse argv. cli-builder collects every error in a
    // single pass and produces fuzzy "did you mean?" suggestions
    // on unknown flags.
    //
    // cli-builder expects argv-style input (program name first),
    // so prepend it. The program name we pass in is the canonical
    // one from the spec; cli-builder uses it for help generation.
    let mut argv = Vec::with_capacity(args.len() + 1);
    argv.push("closurec".to_string());
    argv.extend(args.iter().cloned());

    // Keep a clone of the spec so the --help_markdown
    // short-circuit (CLOC11.54) can iterate the flag list after
    // Parser::new consumes the original. A CliSpec is ~10 KB of
    // owned strings, so the clone is cheap compared to the
    // user-visible work that follows.
    let spec_for_help_md = spec.clone();

    let parser = Parser::new(spec);
    match parser.parse(&argv) {
        Ok(ParserOutput::Parse(result)) => {
            // Step 3 (CLOC11.01): turn the parsed flags into a
            // typed CompilerConfig.
            let cfg = match wire::config_from_parsed(&result) {
                Ok(c) => c,
                Err(e) => return (format!("{e}\n"), String::new(), ExitCode::from(1)),
            };

            // Step 3.5 (CLOC11.54): --help_markdown short-circuit.
            //
            // Emit the markdown flag dump and exit successfully.
            // This sits *after* config-building so a user-error in
            // a sibling flag still surfaces (matches the behavior
            // users expect from --help: the flag is honored even
            // when other flags are wonky, but cli-builder catches
            // outright unknown flags before we get here at all).
            if cfg.special_modes.help_markdown {
                return (
                    help_markdown::format_help_markdown(&spec_for_help_md),
                    String::new(),
                    ExitCode::SUCCESS,
                );
            }

            // Step 4 (CLOC11.01): run the compiler. v1 is an
            // identity pipeline — read inputs, concatenate,
            // write to --js_output_file or stdout. See run.rs.
            //
            // When the AST grows full lexing/parsing (CLOC11.06+),
            // this branch stays the same; only `run_compiler`'s
            // body grows.
            match run::run_compiler(&cfg) {
                Ok(out) => {
                    // Closure exits 0 on success regardless of
                    // whether output went to stdout or to a file.
                    (out.stdout_text, out.stderr_text, ExitCode::SUCCESS)
                }
                Err(e) => (format!("{e}\n"), String::new(), ExitCode::from(2)),
            }
        }
        Ok(ParserOutput::Help(h)) => (h.text, String::new(), ExitCode::SUCCESS),
        Ok(ParserOutput::Version(v)) => {
            // CLOC11.55: emit a CC-compat banner.
            //
            // CC's `closure-compiler.jar --version` prints:
            //
            //   Closure Compiler (https://github.com/google/closure-compiler)
            //   Version: vYYYYMMDD
            //   Built on: 2024-MM-DD HH:MM
            //
            // Toolchains that grep stdout for `Closure Compiler` or
            // `Version:` (e.g. Bazel rules that pin a compiler
            // version) need to see those strings to recognise this
            // binary as a drop-in.
            //
            // We diverge from CC in three ways:
            //  - Project URL points at this clone, not upstream
            //    (transparency: the user knows what they're running).
            //  - Version is semver (we don't date-stamp releases).
            //  - No `Built on:` line — we don't currently embed
            //    build timestamps. The two `grep`-worthy lines are
            //    what tools actually depend on.
            (
                format!(
                    "Closure Compiler (closurec — drop-in replacement, https://github.com/adhithyan15/coding-adventures)\nVersion: {}\n",
                    v.version
                ),
                String::new(),
                ExitCode::SUCCESS,
            )
        }
        Err(e) => {
            // cli-builder's Display for CliBuilderError already
            // formats nicely (multi-line if there are multiple
            // errors, with "did you mean?" suggestions).
            (format!("{}\n", e), String::new(), ExitCode::from(1))
        }
    }
}

/// Binary entry point.
///
/// `std::env::args` includes `argv[0]` (the program name); we
/// strip it so [`parse_and_run`] receives only the user-supplied
/// args. cli-builder will re-add the canonical program name from
/// the spec.
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // CLOC11.75 — use the streaming variant so the CV summary
    // line can land on stderr when
    // `--correlation_vector_summary_stderr` is set; without
    // that flag stderr_text is always empty and main behaves
    // identically to pre-CLOC11.75.
    let (stdout, stderr, code) = parse_and_run_with_streams(&args);
    print!("{}", stdout);
    if !stderr.is_empty() {
        eprint!("{}", stderr);
    }
    code
}

#[cfg(test)]
mod tests {
    //! Tests drive `parse_and_run` directly — no spawning the
    //! binary, no temp files. Each test asserts on the `(text,
    //! exit_code)` pair.
    //!
    //! The bar these tests are setting is *not* "every flag has a
    //! dedicated test" (cli-builder itself has thousands of
    //! tests on its own behavior). It's:
    //!   - the spec loads at all,
    //!   - --help and --version work,
    //!   - the canonical Closure Compiler invocation patterns
    //!     (--js / --js_output_file / --compilation_level /
    //!     --create_source_map) parse cleanly,
    //!   - unknown flags fail loudly with a useful suggestion,
    //!   - enums reject invalid values,
    //!   - repeatable flags accumulate.
    use super::*;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn spec_json_loads_cleanly() {
        // If the embedded spec is malformed, this fails — and we
        // know about it before users see it.
        let spec = load_spec_from_str(CLI_SPEC_JSON).expect("cli.spec.json must load");
        assert_eq!(spec.name, "closurec");
        assert_eq!(spec.cli_builder_spec_version, "1.0");
        // 100 user-visible flag entries (some are alias variants
        // of the same logical flag — see jscomp_dev_mode /
        // dev_mode, checks_only / checks_only_alias,
        // warnings_allowlist_file / warnings_whitelist_file_alias).
        assert!(
            spec.flags.len() >= 90,
            "expected ~100 Closure-Compiler flags; got {}",
            spec.flags.len()
        );
    }

    #[test]
    fn help_long_flag_returns_help_text() {
        let (text, _code) = parse_and_run(&args(&["--help"]));
        // cli-builder generates help that mentions the program
        // name and at least some of the flags. Don't pin exact
        // wording (it might evolve); pin structure.
        assert!(text.contains("closurec"));
        assert!(text.contains("--js"));
        assert!(text.contains("--js_output_file"));
    }

    #[test]
    fn help_short_flag_returns_help_text() {
        let (text, _code) = parse_and_run(&args(&["-h"]));
        assert!(text.contains("closurec"));
    }

    #[test]
    fn version_long_flag_returns_version() {
        // The exact version is asserted by
        // `version_string_matches_crate_version` below using
        // env!("CARGO_PKG_VERSION"); here we just confirm a
        // semver-looking string is in the output.
        let (text, _code) = parse_and_run(&args(&["--version"]));
        assert!(
            text.contains('.'),
            "expected version output to contain a dotted version; got: {text}"
        );
    }

    // Note: the upstream Java Closure Compiler exposes only `--version`
    // (no `-V` short form), so there's no equivalent test here.
    // cli-builder's auto-injected builtin only adds `--version` long.

    /// After CLOC11.01 the binary actually tries to *read* the
    /// `--js` inputs, so passing a nonexistent path is no longer
    /// a successful run — it's an I/O error (exit 2). The point of
    /// these tests is to assert the *CLI surface* still parses
    /// cleanly (no exit-1 / unknown-flag / invalid-enum failures);
    /// the read failure is acceptable and expected.
    fn assert_parsed_cleanly(text: &str) {
        let lower = text.to_lowercase();
        assert!(
            !lower.contains("unknown") && !lower.contains("invalid"),
            "expected clean parse; got: {text}"
        );
    }

    #[test]
    fn canonical_closure_invocation_parses_cleanly() {
        // The single most common Closure Compiler CLI call —
        // mirror it.
        let (text, _code) = parse_and_run(&args(&[
            "--js",
            "src/in.js",
            "--js_output_file",
            "out/out.js",
            "--compilation_level",
            "ADVANCED",
            "--create_source_map",
            "out/out.js.map",
        ]));
        assert_parsed_cleanly(&text);
    }

    #[test]
    fn js_flag_is_repeatable() {
        // --js takes multiple values in real Closure invocations.
        let (text, _code) = parse_and_run(&args(&[
            "--js", "a.js", "--js", "b.js", "--js", "c.js",
        ]));
        assert_parsed_cleanly(&text);
    }

    #[test]
    fn unknown_flag_returns_error() {
        let (text, _code) =
            parse_and_run(&args(&["--js", "in.js", "--definitely_not_a_flag"]));
        // cli-builder produces a "Unknown flag" error and may
        // include "did you mean?" suggestion. Either way the
        // bad flag name should be mentioned.
        assert!(
            text.contains("definitely_not_a_flag")
                || text.to_lowercase().contains("unknown"),
            "expected unknown-flag error; got: {}",
            text
        );
    }

    #[test]
    fn invalid_enum_value_returns_error() {
        // ADVANCED, SIMPLE, WHITESPACE_ONLY, TRANSPILE_ONLY,
        // BUNDLE are valid; BOGUS is not.
        let (text, _code) = parse_and_run(&args(&[
            "--js", "in.js", "--compilation_level", "BOGUS",
        ]));
        assert!(
            text.to_lowercase().contains("bogus")
                || text.to_lowercase().contains("invalid"),
            "expected enum-value error; got: {}",
            text
        );
    }

    #[test]
    fn short_compilation_level_alias_works() {
        // -O is the Closure Compiler's short alias for
        // --compilation_level. Spec sets it up via `short: "O"`.
        let (text, _code) =
            parse_and_run(&args(&["--js", "in.js", "-O", "SIMPLE"]));
        assert_parsed_cleanly(&text);
    }

    #[test]
    fn short_warning_level_alias_works() {
        let (text, _code) =
            parse_and_run(&args(&["--js", "in.js", "-W", "VERBOSE"]));
        assert_parsed_cleanly(&text);
    }

    #[test]
    fn define_short_alias_works() {
        // -D NAME=value is Closure's short for --define NAME=value.
        let (text, _code) = parse_and_run(&args(&[
            "--js", "in.js", "-D", "FLAG_DEBUG=true", "-D", "VERSION=1",
        ]));
        assert_parsed_cleanly(&text);
    }

    #[test]
    fn formatting_flag_is_repeatable_with_enum_values() {
        // --formatting is a repeatable enum (PRETTY_PRINT,
        // PRINT_INPUT_DELIMITER, SINGLE_QUOTES).
        let (text, _code) = parse_and_run(&args(&[
            "--js",
            "in.js",
            "--formatting",
            "PRETTY_PRINT",
            "--formatting",
            "SINGLE_QUOTES",
        ]));
        assert_parsed_cleanly(&text);
    }

    #[test]
    fn deprecated_hyphenated_alias_is_rejected() {
        // The Java tool accepts `--checks-only` (hyphenated) as
        // an alias for `--checks_only` (underscored canonical).
        // cli-builder doesn't natively support multiple long-form
        // aliases per flag, so v0.1.0 implements only the
        // canonical underscored names. Passing the hyphenated
        // form should fail with a useful "unknown flag" error
        // pointing the user at the canonical name.
        //
        // This locks in the limitation as a *known* behavior
        // rather than an accident — future versions may add
        // hyphenated aliases via cli-builder enhancements
        // (tracked in CLOC08 known-gaps).
        let (text, _code) =
            parse_and_run(&args(&["--checks-only"]));
        assert!(
            text.to_lowercase().contains("unknown")
                || text.to_lowercase().contains("checks-only"),
            "expected unknown-flag error for hyphenated alias; got: {}",
            text
        );
    }

    #[test]
    fn empty_argv_runs_with_defaults() {
        // No flags = no required flags missing = identity banner.
        // (--js is not marked required in the spec; the Java tool
        // accepts an empty invocation too.)
        let (text, _code) = parse_and_run(&args(&[]));
        assert!(text.contains("identity pipeline"));
    }

    // ------------------------------------------------------------------
    // CLOC11.54 — --help_markdown behavior
    // ------------------------------------------------------------------

    #[test]
    fn help_markdown_flag_emits_markdown_and_exits_clean() {
        let (text, _code) = parse_and_run(&args(&["--help_markdown"]));
        // Title comes from spec.name.
        assert!(text.starts_with("# closurec\n"), "leading title: {text:?}");
        // The "Flags" section header is present.
        assert!(text.contains("## Flags\n"));
        // Specific known flags appear as section headings.
        assert!(text.contains("### `--js`"));
        assert!(text.contains("### `--checks_only`"));
        assert!(text.contains("### `--print_tree`"));
        // Version line wired up.
        assert!(text.contains("Version: "));
    }

    #[test]
    fn help_markdown_does_not_run_compiler_pipeline() {
        // Even with --js inputs and --js_output_file, the markdown
        // emission must short-circuit without trying to read any
        // files or write any output.
        let (text, _code) = parse_and_run(&args(&[
            "--help_markdown",
            "--js",
            "/tmp/does-not-exist-cloc11-54.js",
            "--js_output_file",
            "/tmp/nope.js",
        ]));
        assert!(text.starts_with("# closurec\n"));
        // No "failed to read input" or "GlobError" surfaced — we
        // never got to the pipeline.
        assert!(!text.contains("failed to read input"));
        assert!(!text.contains("GlobError"));
    }

    #[test]
    fn version_string_matches_crate_version() {
        // env!("CARGO_PKG_VERSION") matches Cargo.toml's
        // version field, which must in turn match the spec's
        // "version" field. If they drift, this catches it.
        let (text, _code) = parse_and_run(&args(&["--version"]));
        assert!(
            text.contains(env!("CARGO_PKG_VERSION")),
            "version output {:?} should contain crate version {:?}",
            text,
            env!("CARGO_PKG_VERSION")
        );
    }

    // ------------------------------------------------------------------
    // CLOC11.55 — --version Closure-Compiler-style banner
    // ------------------------------------------------------------------

    #[test]
    fn version_output_starts_with_closure_compiler_marker() {
        // Drop-in compat: tools that grep CC's stdout for
        // `Closure Compiler` must see the same string here.
        let (text, _code) = parse_and_run(&args(&["--version"]));
        assert!(
            text.starts_with("Closure Compiler "),
            "expected CC-marker prefix, got: {text:?}"
        );
    }

    #[test]
    fn version_output_contains_version_colon_line() {
        // The `Version: ` line is the second canonical hook for
        // version-extracting tools. Pin its presence so future
        // refactors of the banner format don't quietly drop it.
        let (text, _code) = parse_and_run(&args(&["--version"]));
        assert!(
            text.contains("\nVersion: "),
            "expected `Version: ` line, got: {text:?}"
        );
    }

    #[test]
    fn version_output_ends_with_single_newline() {
        // No trailing-blank-line drift: clean newline policy at
        // the end of the output, matches CC.
        let (text, _code) = parse_and_run(&args(&["--version"]));
        assert!(text.ends_with("\n"));
        assert!(
            !text.ends_with("\n\n"),
            "should not have a trailing blank line: {text:?}"
        );
    }
}
