//! # grammar-tools CLI library
//!
//! This module contains all of the logic for the `grammar-tools` CLI program
//! so that it can be independently unit-tested without spawning a subprocess.
//!
//! The binary (`src/main.rs`) is a thin wrapper that calls [`run`].
//!
//! ## Architecture
//!
//! ```text
//!   main() ──► run(argv)
//!                 │
//!                 ├── load spec from  code/specs/grammar-tools.json
//!                 ├── parse argv via cli-builder
//!                 │       ├── HelpResult    → print help, exit 0
//!                 │       ├── VersionResult → print version, exit 0
//!                 │       └── ParseResult   → dispatch on "command"
//!                 │
//!                 └── dispatch(command, files)
//!                         ├── "validate"         → validate_command(t, g)
//!                         ├── "validate-tokens"  → validate_tokens_only(t)
//!                         └── "validate-grammar" → validate_grammar_only(g)
//! ```
//!
//! ## Exit codes
//!
//! | Code | Meaning                              |
//! |------|--------------------------------------|
//! |  0   | All checks passed                    |
//! |  1   | One or more validation errors        |
//! |  2   | Usage error (wrong args / bad spec)  |

use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use grammar_tools::compiler::{compile_combined_grammar, compile_parser_grammar, compile_token_grammar};
use grammar_tools::cross_validator::cross_validate;
use grammar_tools::parser_grammar::{parse_parser_grammar, validate_parser_grammar};
use grammar_tools::token_grammar::{parse_token_grammar, token_names, validate_token_grammar};

// ===========================================================================
// Repository root detection
// ===========================================================================

/// Walk up from the current working directory looking for the sentinel file
/// `code/specs/grammar-tools.json` that marks the monorepo root.
///
/// This is necessary because the program can be invoked from any directory.
/// We follow the same convention used by all other language implementations:
/// walk up at most 20 levels and return the first directory that contains the
/// sentinel.  If not found, fall back to the current directory so that callers
/// can report a sensible error rather than panicking.
pub fn find_root() -> PathBuf {
    let start = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut current = start.clone();
    for _ in 0..20 {
        if current
            .join("code")
            .join("specs")
            .join("grammar-tools.json")
            .exists()
        {
            return current;
        }
        match current.parent() {
            Some(p) => current = p.to_path_buf(),
            None => break,
        }
    }
    start
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Count how many issues are actual errors (not warnings).
///
/// Issues starting with `"Warning"` are informational. Everything else is a
/// real error.  This convention is shared across all language implementations.
fn count_errors(issues: &[String]) -> usize {
    issues.iter().filter(|i| !i.starts_with("Warning")).count()
}

/// Print each issue prefixed with two spaces.
fn print_issues(issues: &[String]) {
    for issue in issues {
        println!("  {}", issue);
    }
}

// ===========================================================================
// validate_command — validate a (.tokens, .grammar) pair
// ===========================================================================

/// Validate a `.tokens` file and a `.grammar` file together.
///
/// Steps:
/// 1. Parse + validate the `.tokens` file.
/// 2. Parse + validate the `.grammar` file (using token names from step 1).
/// 3. Cross-validate the pair.
///
/// Returns `0` on success, `1` on any error.
pub fn validate_command(tokens_path: &str, grammar_path: &str) -> i32 {
    let mut total_errors: usize = 0;

    // -----------------------------------------------------------------------
    // Step 1: tokens file
    // -----------------------------------------------------------------------
    let tokens_filename = Path::new(tokens_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(tokens_path);

    print!("Validating {} ... ", tokens_filename);

    let tokens_source = match std::fs::read_to_string(tokens_path) {
        Ok(s) => s,
        Err(e) => {
            println!("ERROR");
            println!("  {}", e);
            return 1;
        }
    };

    let token_grammar = match parse_token_grammar(&tokens_source) {
        Ok(g) => g,
        Err(e) => {
            println!("PARSE ERROR");
            println!("  {}", e);
            return 1;
        }
    };

    let token_issues = validate_token_grammar(&token_grammar);
    let token_errors = count_errors(&token_issues);
    let n_tokens = token_grammar.definitions.len();
    let n_skip = token_grammar.skip_definitions.len();
    let n_error_defs = token_grammar.error_definitions.len();

    if token_errors > 0 {
        println!("{} error(s)", token_errors);
        print_issues(&token_issues);
        total_errors += token_errors;
    } else {
        let mut parts = vec![format!("{} tokens", n_tokens)];
        if n_skip > 0 {
            parts.push(format!("{} skip", n_skip));
        }
        if n_error_defs > 0 {
            parts.push(format!("{} error", n_error_defs));
        }
        println!("OK ({})", parts.join(", "));
    }

    // -----------------------------------------------------------------------
    // Step 2: grammar file
    // -----------------------------------------------------------------------
    let grammar_filename = Path::new(grammar_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(grammar_path);

    print!("Validating {} ... ", grammar_filename);

    let grammar_source = match std::fs::read_to_string(grammar_path) {
        Ok(s) => s,
        Err(e) => {
            println!("ERROR");
            println!("  {}", e);
            return 1;
        }
    };

    let parser_grammar = match parse_parser_grammar(&grammar_source) {
        Ok(g) => g,
        Err(e) => {
            println!("PARSE ERROR");
            println!("  {}", e);
            return 1;
        }
    };

    let tnames = token_names(&token_grammar);
    let parser_issues = validate_parser_grammar(&parser_grammar, Some(&tnames));
    let parser_errors = count_errors(&parser_issues);
    let n_rules = parser_grammar.rules.len();

    if parser_errors > 0 {
        println!("{} error(s)", parser_errors);
        print_issues(&parser_issues);
        total_errors += parser_errors;
    } else {
        println!("OK ({} rules)", n_rules);
    }

    // -----------------------------------------------------------------------
    // Step 3: cross-validation
    // -----------------------------------------------------------------------
    print!("Cross-validating ... ");

    let cross_issues = cross_validate(&token_grammar, &parser_grammar);
    let cross_errors = count_errors(&cross_issues);
    let cross_warnings = cross_issues.len() - cross_errors;

    if cross_errors > 0 {
        println!("{} error(s)", cross_errors);
        print_issues(&cross_issues);
        total_errors += cross_errors;
    } else if cross_warnings > 0 {
        println!("OK ({} warning(s))", cross_warnings);
        print_issues(&cross_issues);
    } else {
        println!("OK");
    }

    if total_errors > 0 {
        println!("\nFound {} error(s). Fix them and try again.", total_errors);
        1
    } else {
        println!("\nAll checks passed.");
        0
    }
}

// ===========================================================================
// validate_tokens_only — validate just a .tokens file
// ===========================================================================

/// Validate only a `.tokens` file.
///
/// Returns `0` on success, `1` on any error.
pub fn validate_tokens_only(tokens_path: &str) -> i32 {
    let tokens_filename = Path::new(tokens_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(tokens_path);

    print!("Validating {} ... ", tokens_filename);

    let tokens_source = match std::fs::read_to_string(tokens_path) {
        Ok(s) => s,
        Err(e) => {
            println!("ERROR");
            println!("  {}", e);
            return 1;
        }
    };

    let token_grammar = match parse_token_grammar(&tokens_source) {
        Ok(g) => g,
        Err(e) => {
            println!("PARSE ERROR");
            println!("  {}", e);
            return 1;
        }
    };

    let issues = validate_token_grammar(&token_grammar);
    let errors = count_errors(&issues);
    let n_tokens = token_grammar.definitions.len();

    if errors > 0 {
        println!("{} error(s)", errors);
        print_issues(&issues);
        println!("\nFound {} error(s). Fix them and try again.", errors);
        1
    } else {
        println!("OK ({} tokens)", n_tokens);
        println!("\nAll checks passed.");
        0
    }
}

// ===========================================================================
// validate_grammar_only — validate just a .grammar file
// ===========================================================================

/// Validate only a `.grammar` file (no `.tokens` file needed).
///
/// Because we have no token file, undefined-token-reference checks are
/// skipped (we pass `None` to `validate_parser_grammar`).
///
/// Returns `0` on success, `1` on any error.
pub fn validate_grammar_only(grammar_path: &str) -> i32 {
    let grammar_filename = Path::new(grammar_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(grammar_path);

    print!("Validating {} ... ", grammar_filename);

    let grammar_source = match std::fs::read_to_string(grammar_path) {
        Ok(s) => s,
        Err(e) => {
            println!("ERROR");
            println!("  {}", e);
            return 1;
        }
    };

    let parser_grammar = match parse_parser_grammar(&grammar_source) {
        Ok(g) => g,
        Err(e) => {
            println!("PARSE ERROR");
            println!("  {}", e);
            return 1;
        }
    };

    let issues = validate_parser_grammar(&parser_grammar, None::<&HashSet<String>>);
    let errors = count_errors(&issues);
    let n_rules = parser_grammar.rules.len();

    if errors > 0 {
        println!("{} error(s)", errors);
        print_issues(&issues);
        println!("\nFound {} error(s). Fix them and try again.", errors);
        1
    } else {
        println!("OK ({} rules)", n_rules);
        println!("\nAll checks passed.");
        0
    }
}

// ===========================================================================
// compile_tokens_command — compile a .tokens file to Rust source
// ===========================================================================

fn format_issue_block(issues: &[String]) -> String {
    issues
        .iter()
        .map(|issue| format!("  {}", issue))
        .collect::<Vec<_>>()
        .join("\n")
}

fn compile_tokens_source(tokens_path: &str, force: bool) -> Result<String, String> {
    let tokens_source = std::fs::read_to_string(tokens_path)
        .map_err(|e| format!("Failed to read {}: {}", tokens_path, e))?;

    let token_grammar = parse_token_grammar(&tokens_source)
        .map_err(|e| format!("Failed to parse {}: {}", tokens_path, e))?;

    if !force {
        let issues = validate_token_grammar(&token_grammar);
        let errors = count_errors(&issues);
        if errors > 0 {
            return Err(format!(
                "Validation failed for {}:\n{}",
                tokens_path,
                format_issue_block(&issues)
            ));
        }
    }

    let tokens_filename = Path::new(tokens_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(tokens_path);

    Ok(compile_token_grammar(&token_grammar, tokens_filename))
}

fn compile_grammar_source(grammar_path: &str, force: bool) -> Result<String, String> {
    let grammar_source = std::fs::read_to_string(grammar_path)
        .map_err(|e| format!("Failed to read {}: {}", grammar_path, e))?;

    let parser_grammar = parse_parser_grammar(&grammar_source)
        .map_err(|e| format!("Failed to parse {}: {}", grammar_path, e))?;

    if !force {
        let issues = validate_parser_grammar(&parser_grammar, None::<&HashSet<String>>);
        let errors = count_errors(&issues);
        if errors > 0 {
            return Err(format!(
                "Validation failed for {}:\n{}",
                grammar_path,
                format_issue_block(&issues)
            ));
        }
    }

    let grammar_filename = Path::new(grammar_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(grammar_path);

    Ok(compile_parser_grammar(&parser_grammar, grammar_filename))
}

/// Compile a `.tokens` file to Rust source code.
///
/// Parses and validates the file, then calls `compile_token_grammar` and
/// either writes the result to `output_path` or prints it to stdout.
///
/// Returns `0` on success, `1` on error.
pub fn compile_tokens_command(tokens_path: &str, output_path: Option<&str>, force: bool) -> i32 {
    let tokens_filename = Path::new(tokens_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(tokens_path);

    eprint!("Compiling {} ... ", tokens_filename);
    let code = match compile_tokens_source(tokens_path, force) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("ERROR");
            eprintln!("{}", format_issue_block(&[e]));
            return 1;
        }
    };

    match output_path {
        Some(path) => {
            if let Err(e) = std::fs::write(path, &code) {
                eprintln!("ERROR writing {}: {}", path, e);
                return 1;
            }
            eprintln!("OK \u{2192} {}", path);
        }
        None => {
            eprintln!("OK");
            print!("{}", code);
        }
    }
    0
}

// ===========================================================================
// compile_grammar_command — compile a .grammar file to Rust source
// ===========================================================================

/// Compile a `.grammar` file to Rust source code.
///
/// Returns `0` on success, `1` on error.
/// Compile a `.tokens` + `.grammar` pair into ONE Rust module.
///
/// The three Mosaic language compilers keep both grammars in a single
/// `src/_grammar.rs`. They are `*-compiler` crates, so the `*-lexer` /
/// `*-parser` sweep never covered them and nothing could regenerate them —
/// even though their own generated headers named a regeneration command
/// (#14067).
///
/// Returns `0` on success, `1` on error.
/// Parse, validate, and render a `.tokens` + `.grammar` pair as one module.
///
/// Shared by the `compile-combined` command and by
/// `generate-rust-compiled-grammars`, so the sweep and the one-off command
/// cannot produce different bytes for the same inputs.
fn combined_source(tokens_path: &str, grammar_path: &str, force: bool) -> Result<String, String> {
    let tokens_source = std::fs::read_to_string(tokens_path)
        .map_err(|e| format!("Failed to read {}: {}", tokens_path, e))?;
    let grammar_source = std::fs::read_to_string(grammar_path)
        .map_err(|e| format!("Failed to read {}: {}", grammar_path, e))?;

    let tokens = parse_token_grammar(&tokens_source)
        .map_err(|e| format!("Failed to parse {}: {}", tokens_path, e))?;
    let parser = parse_parser_grammar(&grammar_source)
        .map_err(|e| format!("Failed to parse {}: {}", grammar_path, e))?;

    if !force {
        // Validate the parser grammar against the token grammar's real name
        // set rather than standalone, so a rule referencing an undefined token
        // is caught -- and so a legitimate KEYWORD reference is not, now that
        // `token_names` reports the token a `keywords:` block synthesizes.
        let names = token_names(&tokens);
        let mut issues = validate_token_grammar(&tokens);
        issues.extend(validate_parser_grammar(&parser, Some(&names)));
        issues.extend(cross_validate(&tokens, &parser));
        if count_errors(&issues) > 0 {
            return Err(format!(
                "Validation failed for {} + {}:\n{}",
                tokens_path,
                grammar_path,
                format_issue_block(&issues)
            ));
        }
    }

    let name_of = |p: &str| {
        Path::new(p)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(p)
            .to_string()
    };
    Ok(compile_combined_grammar(
        &tokens,
        &parser,
        &name_of(tokens_path),
        &name_of(grammar_path),
    ))
}

pub fn compile_combined_command(
    tokens_path: &str,
    grammar_path: &str,
    output_path: Option<&str>,
    force: bool,
) -> i32 {
    let tokens_source = match std::fs::read_to_string(tokens_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read {}: {}", tokens_path, e);
            return 1;
        }
    };
    let grammar_source = match std::fs::read_to_string(grammar_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read {}: {}", grammar_path, e);
            return 1;
        }
    };

    let tokens = match parse_token_grammar(&tokens_source) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to parse {}: {}", tokens_path, e);
            return 1;
        }
    };
    let parser = match parse_parser_grammar(&grammar_source) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to parse {}: {}", grammar_path, e);
            return 1;
        }
    };

    if !force {
        // Validate the parser grammar against the token grammar's real name
        // set, not standalone. That is what catches a rule referencing a token
        // the tokens file does not define — and, since `token_names` now
        // reports the KEYWORD a `keywords:` block synthesizes, what stops a
        // legitimate KEYWORD reference being reported as undefined.
        let names = token_names(&tokens);
        let mut issues = validate_token_grammar(&tokens);
        issues.extend(validate_parser_grammar(&parser, Some(&names)));
        issues.extend(cross_validate(&tokens, &parser));
        let errors = count_errors(&issues);
        if errors > 0 {
            eprintln!(
                "Validation failed for {} + {}:\n{}",
                tokens_path,
                grammar_path,
                format_issue_block(&issues)
            );
            return 1;
        }
    }

    let tokens_name = Path::new(tokens_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(tokens_path);
    let grammar_name = Path::new(grammar_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(grammar_path);

    let code = compile_combined_grammar(&tokens, &parser, tokens_name, grammar_name);

    #[allow(clippy::needless_return)]
    match output_path {
        Some(out) => match std::fs::write(out, &code) {
            Ok(()) => {
                println!("Compiling {} + {} ... OK \u{2192} {}", tokens_name, grammar_name, out);
                0
            }
            Err(e) => {
                eprintln!("Failed to write {}: {}", out, e);
                1
            }
        },
        None => {
            print!("{}", code);
            0
        }
    }
}

pub fn compile_grammar_command(grammar_path: &str, output_path: Option<&str>, force: bool) -> i32 {
    let grammar_filename = Path::new(grammar_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(grammar_path);

    eprint!("Compiling {} ... ", grammar_filename);
    let code = match compile_grammar_source(grammar_path, force) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("ERROR");
            eprintln!("{}", format_issue_block(&[e]));
            return 1;
        }
    };

    match output_path {
        Some(path) => {
            if let Err(e) = std::fs::write(path, &code) {
                eprintln!("ERROR writing {}: {}", path, e);
                return 1;
            }
            eprintln!("OK \u{2192} {}", path);
        }
        None => {
            eprintln!("OK");
            print!("{}", code);
        }
    }
    0
}

// ===========================================================================
// generate_rust_compiled_grammars_command
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RustGrammarKind {
    Tokens,
    Grammar,
    /// Both grammars in one `_grammar.rs`, as the three Mosaic language
    /// compilers ship them. See `compile_combined_grammar` (#14067).
    Combined,
}

impl RustGrammarKind {
    fn input_extension(self) -> &'static str {
        match self {
            Self::Tokens => "tokens",
            Self::Grammar => "grammar",
            // Discovery anchors on the .tokens file; the sibling .grammar is
            // derived from it, since both live in code/grammars/<stem>/ under
            // the same stem.
            Self::Combined => "tokens",
        }
    }

    fn package_suffix(self) -> &'static str {
        match self {
            Self::Tokens => "-lexer",
            Self::Grammar => "-parser",
            Self::Combined => "-compiler",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustVersionedSource {
    version: String,
    input_path: PathBuf,
    module_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustVersionedFamilyTarget {
    family_name: String,
    generic_input: Option<PathBuf>,
    versions: Vec<RustVersionedSource>,
    /// `Some` for an inventory-driven family (LANG84): every release ID in
    /// the family's `releases.csv`, pending rows included, in canonical
    /// inventory order. `versions` then holds only the non-pending rows, and
    /// the renderer deduplicates identical grammars instead of emitting one
    /// module per release.
    inventory: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RustCompileInputs {
    Flat { input_path: PathBuf },
    Versioned(RustVersionedFamilyTarget),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustCompileTarget {
    package_name: String,
    package_stem: String,
    grammar_name: String,
    filter_names: Vec<String>,
    kind: RustGrammarKind,
    inputs: RustCompileInputs,
    output_path: PathBuf,
    force: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RustVersionedFamilySpec {
    family_name: &'static str,
    filter_names: &'static [&'static str],
    input_dir: &'static str,
    input_prefix: &'static str,
    generic_stem: Option<&'static str>,
    /// The hard-coded release list. Empty for an inventory-driven family.
    versions: &'static [&'static str],
    /// LANG84: the inventory file, relative to `input_dir`, that supplies the
    /// release list instead of `versions`. Its non-pending rows are compiled;
    /// its pending rows are listed but have no grammar.
    inventory: Option<&'static str>,
}

const CSHARP_VERSIONS: &[&str] = &[
    "1.0", "2.0", "3.0", "4.0", "5.0", "6.0", "7.0", "8.0", "9.0", "10.0", "11.0", "12.0",
];
const ECMASCRIPT_VERSIONS: &[&str] = &[
    "es1", "es3", "es5", "es2015", "es2016", "es2017", "es2018", "es2019", "es2020", "es2021",
    "es2022", "es2023", "es2024", "es2025",
];
const FSHARP_VERSIONS: &[&str] = &[
    "1.0", "2.0", "3.0", "3.1", "4.0", "4.1", "4.5", "4.6", "4.7", "5", "6", "7", "8", "9",
    "10",
];
const HASKELL_VERSIONS: &[&str] = &["1.0", "1.1", "1.2", "1.3", "1.4", "98", "2010"];
const JAVA_VERSIONS: &[&str] = &["1.0", "1.1", "1.4", "5", "7", "8", "10", "14", "17", "21"];
const PYTHON_VERSIONS: &[&str] = &["2.7", "3.0", "3.6", "3.8", "3.10", "3.12"];
const TYPESCRIPT_VERSIONS: &[&str] = &["ts1.0", "ts2.0", "ts3.0", "ts4.0", "ts5.0", "ts5.8"];
const VERILOG_VERSIONS: &[&str] = &["1995", "2001", "2005"];
const VHDL_VERSIONS: &[&str] = &["1987", "1993", "2002", "2008", "2019"];

fn normalize_name(name: &str) -> String {
    name.to_ascii_lowercase().replace('-', "_")
}

fn rust_grammar_name_for_package(package_name: &str, kind: RustGrammarKind) -> String {
    match (package_name, kind) {
        // Rust's XML lexer uses a regex-engine-friendly token grammar.
        ("xml-lexer", RustGrammarKind::Tokens) => "xml_rust".to_string(),
        _ => package_name
            .trim_end_matches(kind.package_suffix())
            .replace('-', "_"),
    }
}

fn rust_compile_force_for_package(package_name: &str, kind: RustGrammarKind) -> bool {
    matches!(
        (package_name, kind),
        ("excel-lexer", RustGrammarKind::Tokens)
            | ("excel-parser", RustGrammarKind::Grammar)
            | ("csharp-parser", RustGrammarKind::Grammar)
            | ("haskell-parser", RustGrammarKind::Grammar)
            | ("java-parser", RustGrammarKind::Grammar)
            | ("javascript-parser", RustGrammarKind::Grammar)
            | ("mosaic-lexer", RustGrammarKind::Tokens)
            | ("python-parser", RustGrammarKind::Grammar)
            | ("typescript-parser", RustGrammarKind::Grammar)
    )
}

fn rust_versioned_family_for_package(package_stem: &str) -> Option<RustVersionedFamilySpec> {
    match package_stem {
        "csharp" => Some(RustVersionedFamilySpec {
            family_name: "csharp",
            filter_names: &["csharp"],
            input_dir: "csharp",
            input_prefix: "csharp",
            generic_stem: None,
            versions: CSHARP_VERSIONS,
            inventory: None,
        }),
        "fsharp" => Some(RustVersionedFamilySpec {
            family_name: "fsharp",
            filter_names: &["fsharp"],
            input_dir: "fsharp",
            input_prefix: "fsharp",
            generic_stem: None,
            versions: FSHARP_VERSIONS,
            inventory: None,
        }),
        "haskell" => Some(RustVersionedFamilySpec {
            family_name: "haskell",
            filter_names: &["haskell"],
            input_dir: "haskell",
            input_prefix: "haskell",
            generic_stem: None,
            versions: HASKELL_VERSIONS,
            inventory: None,
        }),
        "java" => Some(RustVersionedFamilySpec {
            family_name: "java",
            filter_names: &["java"],
            input_dir: "java",
            input_prefix: "java",
            generic_stem: None,
            versions: JAVA_VERSIONS,
            inventory: None,
        }),
        "javascript" => Some(RustVersionedFamilySpec {
            family_name: "ecmascript",
            filter_names: &["javascript", "ecmascript"],
            input_dir: "ecmascript",
            input_prefix: "",
            generic_stem: Some("javascript"),
            versions: ECMASCRIPT_VERSIONS,
            inventory: None,
        }),
        // LANG84 phase 4a: the first inventory-driven family. Perl has 776
        // inventory rows and 117 checked partial pairs, but only a handful of
        // distinct grammars, so the release list comes from `releases.csv`
        // and identical grammars share one compiled module. The unversioned
        // `perl.tokens`/`perl.grammar` pair (LANG81's execution grammar) is
        // kept as `unversioned_*()`, never as the `""` release.
        "perl" => Some(RustVersionedFamilySpec {
            family_name: "perl",
            filter_names: &["perl"],
            input_dir: "perl",
            input_prefix: "perl",
            generic_stem: Some("perl"),
            versions: &[],
            inventory: Some("releases.csv"),
        }),
        "python" => Some(RustVersionedFamilySpec {
            family_name: "python",
            filter_names: &["python"],
            input_dir: "python",
            input_prefix: "python",
            generic_stem: None,
            versions: PYTHON_VERSIONS,
            inventory: None,
        }),
        "typescript" => Some(RustVersionedFamilySpec {
            family_name: "typescript",
            filter_names: &["typescript"],
            input_dir: "typescript",
            input_prefix: "",
            generic_stem: Some("typescript"),
            versions: TYPESCRIPT_VERSIONS,
            inventory: None,
        }),
        "verilog" => Some(RustVersionedFamilySpec {
            family_name: "verilog",
            filter_names: &["verilog"],
            input_dir: "verilog",
            input_prefix: "verilog",
            generic_stem: None,
            versions: VERILOG_VERSIONS,
            inventory: None,
        }),
        "vhdl" => Some(RustVersionedFamilySpec {
            family_name: "vhdl",
            filter_names: &["vhdl"],
            input_dir: "vhdl",
            input_prefix: "vhdl",
            generic_stem: None,
            versions: VHDL_VERSIONS,
            inventory: None,
        }),
        _ => None,
    }
}

fn rust_module_name_for_version(version: &str) -> String {
    let mut module_name = String::from("v_");
    for ch in version.chars() {
        if ch.is_ascii_alphanumeric() {
            module_name.push(ch.to_ascii_lowercase());
        } else {
            module_name.push('_');
        }
    }
    while module_name.ends_with('_') {
        module_name.pop();
    }
    module_name
}

fn build_versioned_family_target(
    grammars_dir: &Path,
    spec: RustVersionedFamilySpec,
    kind: RustGrammarKind,
) -> Result<RustVersionedFamilyTarget, String> {
    let input_dir = grammars_dir.join(spec.input_dir);
    if !input_dir.exists() {
        return Err(format!(
            "Versioned grammar directory '{}' does not exist.",
            input_dir.display()
        ));
    }

    let generic_input = match spec.generic_stem {
        Some(stem) => {
            let input_path = grammars_dir
                .join(stem)
                .join(format!("{}.{}", stem, kind.input_extension()));
            if !input_path.exists() {
                return Err(format!(
                    "Expected generic grammar file '{}' for family '{}'.",
                    input_path.display(),
                    spec.family_name
                ));
            }
            Some(input_path)
        }
        None => None,
    };

    // An inventory-driven family takes its releases from the CSV; the others
    // keep their hard-coded lists. Either way each listed release must have
    // its own materialized file — no release is an alias of another's.
    let (release_ids, inventory): (Vec<String>, Option<Vec<String>>) = match spec.inventory {
        Some(csv_name) => {
            let rows = read_release_inventory(&input_dir.join(csv_name))?;
            let compiled = rows
                .iter()
                .filter(|row| row.grammar_status != "pending")
                .map(|row| row.release.clone())
                .collect();
            let all = rows.into_iter().map(|row| row.release).collect();
            (compiled, Some(all))
        }
        None => (spec.versions.iter().map(|v| (*v).to_string()).collect(), None),
    };

    let mut versions = Vec::new();
    for version in &release_ids {
        let filename = format!("{}{}.{}", spec.input_prefix, version, kind.input_extension());
        let input_path = input_dir.join(&filename);
        if !input_path.exists() {
            return Err(format!(
                "Expected versioned grammar file '{}' for family '{}'.",
                input_path.display(),
                spec.family_name
            ));
        }

        versions.push(RustVersionedSource {
            version: version.clone(),
            input_path,
            module_name: rust_module_name_for_version(version),
        });
    }

    Ok(RustVersionedFamilyTarget {
        family_name: spec.family_name.to_string(),
        generic_input,
        versions,
        inventory,
    })
}

// ---------------------------------------------------------------------------
// LANG84 release inventories
// ---------------------------------------------------------------------------
//
// A `releases.csv` looks like this (Perl's, abridged; the URL cells are
// shortened here):
//
// ```text
// # provenance comment lines start with '#'
// release,date,channel,source_ref,source_sha,predecessor,grammar_status,grammar_stage,grammar_subset,iir_status,backend_status,profile,perlhist_line,lineage
// 1.000,1987-12-18,final,https://…/tree/perl-1.0,8d063cd8…,,partial,P,pl-decimal-print,-,-,-,78,first
// 1.001,1988-01-30,patch,https://…/perlhist.pod#L80,d97e2dc0…,1.000,pending,none,-,-,-,-,80,line
// ```
//
// That is the common LANG84 header: twelve required columns, then the
// language's extras (Perl keeps LANG82's `perlhist_line`). Perl's file had
// the four-column LANG82 header `release,date,perlhist_line,grammar_status`
// until LANG84 phase 2b migrated it.
//
// The generator needs two columns: `release` (the ID, which becomes both a
// file name and a Rust string literal) and `grammar_status` (whether a pair
// exists to compile). Columns are found by header NAME, never by position:
// the 2b migration moved `grammar_status` from the fourth column to the
// seventh, and a positional reader would have read `channel` instead. The
// cells are split on bare commas (inventory cells never quote), so every row
// must have exactly as many fields as the header -- a row with a stray comma
// would otherwise shift every later column without any error.

/// One inventory row, reduced to the two columns the generator reads.
#[derive(Debug, Clone, PartialEq, Eq)]
struct InventoryRow {
    release: String,
    grammar_status: String,
}

/// Is `release` a safe inventory ID? LANG84 fixes the charset to
/// `[A-Za-z0-9._-]`. The ID is spliced into a file name
/// (`<prefix><release>.tokens`), so a `/` or a `..` component would let a
/// CSV row name a file outside the grammar directory; refuse both, and refuse
/// the empty ID.
fn is_safe_release_id(release: &str) -> bool {
    !release.is_empty()
        && !release.contains("..")
        && release
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Read a LANG84 `releases.csv`: skip leading `#` comment lines, locate the
/// `release` and `grammar_status` columns by header name, and return every row
/// in file (= canonical inventory) order. Fails on an absent column, a row
/// whose field count differs from the header's, an unsafe or duplicate ID, or
/// a status outside `pending`, `partial`, `complete`.
fn read_release_inventory(path: &Path) -> Result<Vec<InventoryRow>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read inventory '{}': {}", path.display(), e))?;
    parse_release_inventory(&text).map_err(|e| format!("{}: {}", path.display(), e))
}

fn parse_release_inventory(text: &str) -> Result<Vec<InventoryRow>, String> {
    let mut lines = text
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty());
    let header: Vec<&str> = lines
        .next()
        .ok_or("inventory has no header row")?
        .split(',')
        .collect();
    let column = |name: &str| {
        header
            .iter()
            .position(|h| h.trim() == name)
            .ok_or(format!("inventory header has no '{name}' column"))
    };
    let release_col = column("release")?;
    let status_col = column("grammar_status")?;

    let mut seen = HashSet::new();
    let mut rows = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != header.len() {
            return Err(format!(
                "inventory row has {} fields, the header has {}: {line:?}",
                fields.len(),
                header.len()
            ));
        }
        let (release, status) = (fields[release_col], fields[status_col]);
        let (release, status) = (release.trim(), status.trim());
        if !is_safe_release_id(release) {
            return Err(format!("unsafe release ID {release:?}"));
        }
        if !matches!(status, "pending" | "partial" | "complete") {
            return Err(format!("release {release}: unknown grammar_status {status:?}"));
        }
        if !seen.insert(release.to_string()) {
            return Err(format!("duplicate release ID {release:?}"));
        }
        rows.push(InventoryRow {
            release: release.to_string(),
            grammar_status: status.to_string(),
        });
    }
    if rows.is_empty() {
        return Err("inventory has no release rows".to_string());
    }
    Ok(rows)
}

/// The canonical form two compiled grammars are compared in (LANG84
/// "Compiled tables deduplicate identical grammars"): the generated Rust with
/// its `//` comment lines dropped and every `line_number: N` field zeroed.
///
/// Comments and line numbers are the only things that differ between two
/// releases whose grammars are otherwise identical — each pair carries its
/// own provenance header, which shifts every line number below it. Anything
/// else (a pattern, a token name, a rule body, a skip definition) survives
/// canonicalization and keeps the two in separate classes.
fn canonical_compiled_grammar(code: &str) -> String {
    let mut out = String::with_capacity(code.len());
    for line in code.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        match trimmed.strip_prefix("line_number:") {
            Some(_) => {
                let indent = &line[..line.len() - trimmed.len()];
                out.push_str(indent);
                out.push_str("line_number: 0,");
            }
            None => out.push_str(line),
        }
        out.push('\n');
    }
    out
}

/// `g_` plus the first 12 hex digits of the SHA-256 of the canonical form.
/// Naming a class by its content rather than by its first release keeps the
/// module name stable when an earlier release is backfilled into the class.
fn dedup_module_name(canonical: &str) -> String {
    let hex = coding_adventures_sha256::sha256_hex(canonical.as_bytes());
    format!("g_{}", &hex[..12])
}

fn rust_target_matches_filters(target: &RustCompileTarget, filters: &HashSet<String>) -> bool {
    if filters.is_empty() {
        return true;
    }

    let mut candidates = vec![
        normalize_name(&target.package_name),
        normalize_name(&target.package_stem),
        normalize_name(&target.grammar_name),
    ];
    candidates.extend(target.filter_names.iter().map(|name| normalize_name(name)));

    candidates
        .iter()
        .any(|candidate| filters.contains(candidate))
}

fn find_rust_compile_targets(
    root: &Path,
    filters: &[String],
) -> Result<Vec<RustCompileTarget>, String> {
    let rust_packages_dir = root.join("code").join("packages").join("rust");
    let grammars_dir = root.join("code").join("grammars");

    let normalized_filters: HashSet<String> = filters.iter().map(|f| normalize_name(f)).collect();
    let mut targets = Vec::new();

    let entries = std::fs::read_dir(&rust_packages_dir).map_err(|e| {
        format!(
            "Failed to read Rust packages directory '{}': {}",
            rust_packages_dir.display(),
            e
        )
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            format!(
                "Failed to read an entry under '{}': {}",
                rust_packages_dir.display(),
                e
            )
        })?;

        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to inspect '{}': {}", entry.path().display(), e))?;
        if !file_type.is_dir() {
            continue;
        }

        let package_name = entry.file_name().to_string_lossy().to_string();
        let kind = if package_name.ends_with("-lexer") {
            RustGrammarKind::Tokens
        } else if package_name.ends_with("-parser") {
            RustGrammarKind::Grammar
        } else if package_name.ends_with("-compiler") {
            // The three Mosaic language compilers keep both grammars in one
            // _grammar.rs. They were the only 3 of 88 _grammar.rs files that
            // no sweep covered, so nothing regenerated them and nothing
            // noticed when their sources drifted (#14067). A `-compiler`
            // crate with no matching code/grammars/<stem>/ entry is skipped
            // by the existence check below, so this does not claim every
            // compiler crate in the repository.
            RustGrammarKind::Combined
        } else {
            continue;
        };

        let package_stem = package_name
            .trim_end_matches(kind.package_suffix())
            .to_string();
        let output_path = entry.path().join("src").join("_grammar.rs");
        if !output_path.parent().is_some_and(|dir| dir.exists()) {
            continue;
        }

        // Combined targets keep an EXISTING _grammar.rs in sync; they never
        // invent one. Several `-compiler` crates have a matching grammar
        // directory but parse at runtime or delegate to a sibling
        // lexer/parser crate, and generating an unused module into them would
        // be noise at best. Restricting to crates that already ship the file
        // makes this exactly "the three that nothing could regenerate", and
        // lets a fourth opt in simply by committing one.
        if kind == RustGrammarKind::Combined && !output_path.exists() {
            continue;
        }

        let versioned_spec = rust_versioned_family_for_package(&package_stem);
        let (grammar_name, filter_names, inputs) = match versioned_spec {
            Some(spec) => {
                let mut filter_names = spec
                    .filter_names
                    .iter()
                    .map(|name| (*name).to_string())
                    .collect::<Vec<_>>();
                if let Some(generic_stem) = spec.generic_stem {
                    filter_names.push(generic_stem.to_string());
                }

                (
                    spec.family_name.to_string(),
                    filter_names,
                    RustCompileInputs::Versioned(build_versioned_family_target(
                        &grammars_dir,
                        spec,
                        kind,
                    )?),
                )
            }
            None => {
                let grammar_name = rust_grammar_name_for_package(&package_name, kind);
                let input_path = grammars_dir
                    .join(&grammar_name)
                    .join(format!("{}.{}", grammar_name, kind.input_extension()));
                if !input_path.exists() {
                    continue;
                }

                (
                    grammar_name,
                    Vec::new(),
                    RustCompileInputs::Flat { input_path },
                )
            }
        };

        let target = RustCompileTarget {
            package_name,
            package_stem,
            grammar_name,
            filter_names,
            kind,
            inputs,
            output_path,
            force: rust_compile_force_for_package(
                entry.file_name().to_string_lossy().as_ref(),
                kind,
            ),
        };

        if rust_target_matches_filters(&target, &normalized_filters) {
            targets.push(target);
        }
    }

    targets.sort_by(|a, b| {
        (a.grammar_name.as_str(), a.kind, a.package_name.as_str()).cmp(&(
            b.grammar_name.as_str(),
            b.kind,
            b.package_name.as_str(),
        ))
    });

    Ok(targets)
}

fn indent_rust_code(code: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    let mut output = String::new();
    for line in code.lines() {
        if !line.is_empty() {
            output.push_str(&prefix);
        }
        output.push_str(line);
        output.push('\n');
    }
    output
}

fn compile_rust_input_source(
    kind: RustGrammarKind,
    input_path: &Path,
    force: bool,
) -> Result<String, String> {
    match kind {
        RustGrammarKind::Tokens => compile_tokens_source(input_path.to_string_lossy().as_ref(), force),
        RustGrammarKind::Grammar => {
            compile_grammar_source(input_path.to_string_lossy().as_ref(), force)
        }
        RustGrammarKind::Combined => {
            let grammar_path = input_path.with_extension("grammar");
            if !grammar_path.exists() {
                return Err(format!(
                    "Combined target needs a sibling grammar file, but {} does not exist",
                    grammar_path.display()
                ));
            }
            combined_source(
                input_path.to_string_lossy().as_ref(),
                grammar_path.to_string_lossy().as_ref(),
                force,
            )
        }
    }
}

fn render_versioned_rust_target(
    target: &RustCompileTarget,
    family: &RustVersionedFamilyTarget,
    force: bool,
) -> Result<String, String> {
    let (grammar_type, selector_name, import_path) = match target.kind {
        RustGrammarKind::Tokens => (
            "TokenGrammar",
            "token_grammar",
            "grammar_tools::token_grammar::TokenGrammar",
        ),
        RustGrammarKind::Grammar => (
            "ParserGrammar",
            "parser_grammar",
            "grammar_tools::parser_grammar::ParserGrammar",
        ),
        // Versioned families exist for languages with many editions (csharp1.0,
        // java8, ...). No combined target is versioned -- the three Mosaic
        // languages have a single edition each, and
        // `rust_versioned_family_for_package` returns None for them, so this
        // renderer is never reached with Combined. Erroring rather than
        // silently emitting a wrong module keeps that assumption checked
        // instead of assumed.
        RustGrammarKind::Combined => {
            return Err(format!(
                "Versioned families are not supported for combined targets ({})",
                target.package_name
            ))
        }
    };

    if let Some(inventory) = &family.inventory {
        return render_inventory_rust_target(
            target,
            family,
            inventory,
            (grammar_type, selector_name, import_path),
            force,
        );
    }

    let mut output = String::new();
    output.push_str("// AUTO-GENERATED FILE - DO NOT EDIT\n");
    output.push_str(&format!("// Source family: {}\n", family.family_name));
    output.push_str(&format!(
        "// Regenerate with: grammar-tools generate-rust-compiled-grammars {}\n",
        target.package_stem
    ));
    output.push_str("//\n");
    output.push_str(&format!(
        "// This file embeds versioned {} values as native Rust data structures.\n",
        grammar_type
    ));
    output.push_str(&format!(
        "// Call `{}` instead of reading and parsing grammar files at runtime.\n\n",
        selector_name
    ));
    output.push_str(&format!("use {};\n\n", import_path));

    output.push_str("pub const SUPPORTED_VERSIONS: &[&str] = &[\n");
    if family.generic_input.is_some() {
        output.push_str("    \"\",\n");
    }
    for source in &family.versions {
        output.push_str(&format!("    {:?},\n", source.version));
    }
    output.push_str("];\n\n");

    output.push_str(&format!(
        "pub fn {}(version: &str) -> Option<{}> {{\n",
        selector_name, grammar_type
    ));
    output.push_str("    match version {\n");
    if family.generic_input.is_some() {
        output.push_str(&format!("        \"\" => Some(generic::{}()),\n", selector_name));
    }
    for source in &family.versions {
        output.push_str(&format!(
            "        {:?} => Some({}::{}()),\n",
            source.version, source.module_name, selector_name
        ));
    }
    output.push_str("        _ => None,\n");
    output.push_str("    }\n");
    output.push_str("}\n\n");

    if let Some(generic_input) = &family.generic_input {
        let code = compile_rust_input_source(target.kind, generic_input, force)?;
        output.push_str("mod generic {\n");
        output.push_str(&indent_rust_code(&code, 4));
        output.push_str("}\n\n");
    }

    for source in &family.versions {
        let code = compile_rust_input_source(target.kind, &source.input_path, force)?;
        output.push_str(&format!("mod {} {{\n", source.module_name));
        output.push_str(&indent_rust_code(&code, 4));
        output.push_str("}\n\n");
    }

    Ok(output)
}

/// Render an inventory-driven family (LANG84): one module per DISTINCT
/// grammar, a `match` that maps every compiled release ID to its module, the
/// inventory and supported-release lists, and the unversioned pair under its
/// own function name.
///
/// ```text
///   releases.csv rows ──► non-pending IDs ──► compile each pair
///                                                │
///                       canonical form ◄─────────┘ (comments dropped,
///                            │                       line numbers zeroed)
///                            ▼
///                    SHA-256 → g_<12 hex>   (first release in a class
///                                            supplies the emitted code)
/// ```
///
/// The unversioned pair is deliberately NOT reachable as release `""`: LANG84
/// C4.4 makes an empty release string an error rather than a default, so it
/// gets `unversioned_<selector>()` instead.
fn render_inventory_rust_target(
    target: &RustCompileTarget,
    family: &RustVersionedFamilyTarget,
    inventory: &[String],
    (grammar_type, selector_name, import_path): (&str, &str, &str),
    force: bool,
) -> Result<String, String> {
    // Classes in first-seen (inventory) order, so the output is deterministic.
    struct Class {
        module_name: String,
        canonical: String,
        code: String,
        releases: Vec<String>,
    }
    let mut classes: Vec<Class> = Vec::new();
    for source in &family.versions {
        let code = compile_rust_input_source(target.kind, &source.input_path, force)?;
        let canonical = canonical_compiled_grammar(&code);
        match classes.iter_mut().find(|class| class.canonical == canonical) {
            Some(class) => class.releases.push(source.version.clone()),
            None => {
                let module_name = dedup_module_name(&canonical);
                // Two different grammars sharing 48 bits of SHA-256 is not
                // expected, but if it ever happens the two modules would
                // collide silently, so check rather than assume.
                if classes.iter().any(|class| class.module_name == module_name) {
                    return Err(format!(
                        "dedup module name {module_name} names two different grammars"
                    ));
                }
                classes.push(Class {
                    module_name,
                    canonical,
                    code,
                    releases: vec![source.version.clone()],
                });
            }
        }
    }

    let mut output = String::new();
    output.push_str("// AUTO-GENERATED FILE - DO NOT EDIT\n");
    output.push_str(&format!(
        "// Source family: {} (release list from code/grammars/{}/releases.csv)\n",
        family.family_name, family.family_name
    ));
    output.push_str(&format!(
        "// Regenerate with: grammar-tools generate-rust-compiled-grammars {}\n",
        target.package_stem
    ));
    output.push_str("//\n");
    output.push_str(&format!(
        "// {} compiled releases share {} distinct {} value(s) (LANG84 deduplication:\n",
        family.versions.len(),
        classes.len(),
        grammar_type
    ));
    output.push_str("// grammars equal apart from comments and line numbers share one module, whose\n");
    output.push_str("// line numbers come from the first release in the class).\n");
    output.push_str(&format!(
        "// Call `{}` instead of reading and parsing grammar files at runtime.\n\n",
        selector_name
    ));
    output.push_str(&format!("use {};\n\n", import_path));

    output.push_str("/// Every release in the inventory, pending rows included, in canonical\n");
    output.push_str("/// inventory order. A pending release is known but has no grammar.\n");
    output.push_str("pub const INVENTORY_RELEASES: &[&str] = &[\n");
    for release in inventory {
        output.push_str(&format!("    {:?},\n", release));
    }
    output.push_str("];\n\n");

    output.push_str("/// Every release with a compiled grammar (non-pending rows), in inventory order.\n");
    output.push_str("pub const SUPPORTED_RELEASES: &[&str] = &[\n");
    for source in &family.versions {
        output.push_str(&format!("    {:?},\n", source.version));
    }
    output.push_str("];\n\n");

    output.push_str(&format!(
        "/// The compiled {} of exactly one inventory release, or `None` when the\n",
        grammar_type
    ));
    output.push_str("/// release is pending or not in the inventory. There is no fallback.\n");
    output.push_str(&format!(
        "pub fn {}(release: &str) -> Option<{}> {{\n",
        selector_name, grammar_type
    ));
    output.push_str("    match release {\n");
    for class in &classes {
        for (index, release) in class.releases.iter().enumerate() {
            let lead = if index == 0 { "        " } else { "        | " };
            output.push_str(&format!("{lead}{release:?}\n"));
        }
        output.push_str(&format!(
            "            => Some({}::{}()),\n",
            class.module_name, selector_name
        ));
    }
    output.push_str("        _ => None,\n");
    output.push_str("    }\n");
    output.push_str("}\n\n");

    if let Some(generic_input) = &family.generic_input {
        let code = compile_rust_input_source(target.kind, generic_input, force)?;
        output.push_str(&format!(
            "/// The unversioned pair. It is not an inventory release and is never\n\
             /// returned for one; callers that use it must name it explicitly.\n\
             pub fn unversioned_{}() -> {} {{\n    generic::{}()\n}}\n\n",
            selector_name, grammar_type, selector_name
        ));
        output.push_str("mod generic {\n");
        output.push_str(&indent_rust_code(&code, 4));
        output.push_str("}\n\n");
    }

    for class in &classes {
        output.push_str(&format!(
            "// {} release(s), first {}\n",
            class.releases.len(),
            class.releases[0]
        ));
        output.push_str(&format!("mod {} {{\n", class.module_name));
        output.push_str(&indent_rust_code(&class.code, 4));
        output.push_str("}\n\n");
    }

    Ok(output)
}

fn compile_rust_target_to_string(target: &RustCompileTarget, force: bool) -> Result<String, String> {
    match &target.inputs {
        RustCompileInputs::Flat { input_path } => compile_rust_input_source(target.kind, input_path, force),
        RustCompileInputs::Versioned(family) => render_versioned_rust_target(target, family, force),
    }
}

/// Generate `_grammar.rs` files for Rust lexer/parser packages in the repo.
///
/// This command walks `code/packages/rust`, finds `*-lexer` and `*-parser`
/// crates with corresponding grammar files under `code/grammars`, and writes
/// `src/_grammar.rs` in each package. Optional filters may name either the
/// package (`sql-lexer`), the package stem (`sql`), or the grammar file stem
/// (`dartmouth_basic`).
pub fn generate_rust_compiled_grammars_command(filters: &[String], force: bool) -> i32 {
    let root = find_root();
    let targets = match find_rust_compile_targets(&root, filters) {
        Ok(targets) => targets,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };

    if targets.is_empty() {
        if filters.is_empty() {
            eprintln!("Error: no Rust grammar targets were found.");
        } else {
            eprintln!(
                "Error: no Rust grammar targets matched filters: {}",
                filters.join(", ")
            );
        }
        return 1;
    }

    println!(
        "Generating {} Rust compiled grammar file(s)...",
        targets.len()
    );

    let mut failures = 0usize;

    for target in &targets {
        let effective_force = force || target.force;
        match compile_rust_target_to_string(target, effective_force) {
            Ok(code) => {
                if let Err(e) = std::fs::write(&target.output_path, code) {
                    eprintln!(
                        "Failed to write '{}': {}",
                        target.output_path.display(),
                        e
                    );
                    failures += 1;
                } else {
                    println!("  OK -> {}", target.output_path.display());
                }
            }
            Err(e) => {
                eprintln!(
                    "Failed to generate '{}': {}",
                    target.output_path.display(),
                    e
                );
                failures += 1;
            }
        }
    }

    if failures > 0 {
        eprintln!(
            "\nFailed to generate {} of {} Rust compiled grammar file(s).",
            failures,
            targets.len()
        );
        1
    } else {
        println!(
            "\nGenerated {} Rust compiled grammar file(s) successfully.",
            targets.len()
        );
        0
    }
}

// ===========================================================================
// dispatch — route command name + file list to the right function
// ===========================================================================

/// Dispatch a parsed command name and file list to the appropriate function.
///
/// Returns an exit code (0, 1, or 2).
pub fn dispatch(command: &str, files: &[String], output_path: Option<&str>, force: bool) -> i32 {
    match command {
        "validate" => {
            if files.len() != 2 {
                eprintln!("Error: 'validate' requires exactly two files: <tokens> <grammar>");
                return 2;
            }
            validate_command(&files[0], &files[1])
        }
        "validate-tokens" => {
            if files.len() != 1 {
                eprintln!("Error: 'validate-tokens' requires exactly one file: <tokens>");
                return 2;
            }
            validate_tokens_only(&files[0])
        }
        "validate-grammar" => {
            if files.len() != 1 {
                eprintln!("Error: 'validate-grammar' requires exactly one file: <grammar>");
                return 2;
            }
            validate_grammar_only(&files[0])
        }
        "compile-tokens" => {
            if files.len() != 1 {
                eprintln!("Error: 'compile-tokens' requires exactly one file: <tokens>");
                return 2;
            }
            compile_tokens_command(&files[0], output_path, force)
        }
        "compile-grammar" => {
            if files.len() != 1 {
                eprintln!("Error: 'compile-grammar' requires exactly one file: <grammar>");
                return 2;
            }
            compile_grammar_command(&files[0], output_path, force)
        }
        "compile-combined" => {
            if files.len() != 2 {
                eprintln!(
                    "Error: 'compile-combined' requires exactly two files: <tokens> <grammar>"
                );
                return 2;
            }
            compile_combined_command(&files[0], &files[1], output_path, force)
        }
        "generate-rust-compiled-grammars" => generate_rust_compiled_grammars_command(files, force),
        other => {
            eprintln!("Error: unknown command '{}'", other);
            2
        }
    }
}

// ===========================================================================
// run — top-level entry point called by main()
// ===========================================================================

/// Parse `argv` with cli-builder and dispatch to the right command.
///
/// `argv[0]` is the binary name (as per convention); `argv[1..]` are the
/// user-supplied arguments.
pub fn run(argv: Vec<String>) -> i32 {
    let root = find_root();
    let spec_path = root.join("code").join("specs").join("grammar-tools.json");

    let spec = match load_spec_from_file(spec_path.to_str().unwrap_or("")) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: could not load CLI spec: {}", e);
            return 2;
        }
    };

    let parser = Parser::new(spec);

    match parser.parse(&argv) {
        Ok(ParserOutput::Help(h)) => {
            println!("{}", h.text);
            0
        }
        Ok(ParserOutput::Version(v)) => {
            println!("{}", v.version);
            0
        }
        Ok(ParserOutput::Parse(result)) => {
            // Extract the required COMMAND positional argument.
            let command = match result.arguments.get("command") {
                Some(v) => match v.as_str() {
                    Some(s) => s.to_string(),
                    None => {
                        eprintln!("Error: COMMAND must be a string");
                        return 2;
                    }
                },
                None => {
                    eprintln!("Error: COMMAND argument is missing");
                    return 2;
                }
            };

            // Extract the variadic FILES positional argument (may be absent).
            let files: Vec<String> = match result.arguments.get("files") {
                Some(v) => {
                    if let Some(arr) = v.as_array() {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect()
                    } else if let Some(s) = v.as_str() {
                        vec![s.to_string()]
                    } else {
                        vec![]
                    }
                }
                None => vec![],
            };

            // Extract the optional --output flag.
            let output_path: Option<String> = result
                .flags
                .get("output")
                .and_then(|v| v.as_str().map(|s| s.to_string()));

            // Extract the optional --force flag.
            let force = result
                .flags
                .get("force")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            dispatch(&command, &files, output_path.as_deref(), force)
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            2
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn grammar_path(name: &str) -> String {
        let root = find_root();
        let stem = name.split('.').next().unwrap_or(name);
        root.join("code")
            .join("grammars")
            .join(stem)
            .join(name)
            .to_string_lossy()
            .to_string()
    }

    fn exists(name: &str) -> bool {
        std::path::Path::new(&grammar_path(name)).exists()
    }

    // -----------------------------------------------------------------------
    // validate_command
    // -----------------------------------------------------------------------

    #[test]
    fn validate_command_json_pair_succeeds() {
        if !exists("json.tokens") || !exists("json.grammar") {
            return;
        }
        assert_eq!(
            validate_command(&grammar_path("json.tokens"), &grammar_path("json.grammar")),
            0
        );
    }

    #[test]
    fn validate_command_lisp_pair_succeeds() {
        if !exists("lisp.tokens") || !exists("lisp.grammar") {
            return;
        }
        assert_eq!(
            validate_command(&grammar_path("lisp.tokens"), &grammar_path("lisp.grammar")),
            0
        );
    }

    #[test]
    fn validate_command_missing_tokens_returns_1() {
        assert_eq!(validate_command("/nonexistent/x.tokens", "any.grammar"), 1);
    }

    #[test]
    fn validate_command_missing_grammar_returns_1() {
        if !exists("json.tokens") {
            return;
        }
        assert_eq!(
            validate_command(&grammar_path("json.tokens"), "/nonexistent/x.grammar"),
            1
        );
    }

    // -----------------------------------------------------------------------
    // validate_tokens_only
    // -----------------------------------------------------------------------

    #[test]
    fn validate_tokens_only_json_succeeds() {
        if !exists("json.tokens") {
            return;
        }
        assert_eq!(validate_tokens_only(&grammar_path("json.tokens")), 0);
    }

    #[test]
    fn validate_tokens_only_missing_returns_1() {
        assert_eq!(validate_tokens_only("/nonexistent/x.tokens"), 1);
    }

    // -----------------------------------------------------------------------
    // validate_grammar_only
    // -----------------------------------------------------------------------

    #[test]
    fn validate_grammar_only_json_succeeds() {
        if !exists("json.grammar") {
            return;
        }
        assert_eq!(validate_grammar_only(&grammar_path("json.grammar")), 0);
    }

    #[test]
    fn validate_grammar_only_missing_returns_1() {
        assert_eq!(validate_grammar_only("/nonexistent/x.grammar"), 1);
    }

    // -----------------------------------------------------------------------
    // dispatch
    // -----------------------------------------------------------------------

    #[test]
    fn dispatch_unknown_command_returns_2() {
        assert_eq!(dispatch("unknown", &[], None, false), 2);
    }

    #[test]
    fn dispatch_validate_wrong_file_count_returns_2() {
        assert_eq!(
            dispatch("validate", &["one.tokens".to_string()], None, false),
            2
        );
    }

    #[test]
    fn dispatch_validate_tokens_no_files_returns_2() {
        assert_eq!(dispatch("validate-tokens", &[], None, false), 2);
    }

    #[test]
    fn dispatch_validate_grammar_no_files_returns_2() {
        assert_eq!(dispatch("validate-grammar", &[], None, false), 2);
    }

    #[test]
    fn dispatch_compile_tokens_no_files_returns_2() {
        assert_eq!(dispatch("compile-tokens", &[], None, false), 2);
    }

    #[test]
    fn dispatch_compile_grammar_no_files_returns_2() {
        assert_eq!(dispatch("compile-grammar", &[], None, false), 2);
    }

    #[test]
    fn rust_grammar_name_for_xml_lexer_uses_xml_rust() {
        assert_eq!(
            rust_grammar_name_for_package("xml-lexer", RustGrammarKind::Tokens),
            "xml_rust"
        );
    }

    #[test]
    fn rust_grammar_name_for_hyphenated_package_uses_underscores() {
        assert_eq!(
            rust_grammar_name_for_package("dartmouth-basic-parser", RustGrammarKind::Grammar),
            "dartmouth_basic"
        );
    }

    #[test]
    fn find_rust_compile_targets_includes_sql_pair() {
        let root = find_root();
        let targets = find_rust_compile_targets(&root, &["sql".to_string()]).unwrap();
        let package_names: Vec<String> = targets.iter().map(|t| t.package_name.clone()).collect();

        assert!(package_names.contains(&"sql-lexer".to_string()));
        assert!(package_names.contains(&"sql-parser".to_string()));
    }

    #[test]
    fn find_rust_compile_targets_filters_by_grammar_name() {
        let root = find_root();
        let targets = find_rust_compile_targets(&root, &["dartmouth_basic".to_string()]).unwrap();
        let package_names: Vec<String> = targets.iter().map(|t| t.package_name.clone()).collect();

        assert!(package_names.contains(&"dartmouth-basic-lexer".to_string()));
        assert!(package_names.contains(&"dartmouth-basic-parser".to_string()));
        assert!(!package_names.contains(&"sql-lexer".to_string()));
    }

    #[test]
    fn find_rust_compile_targets_treats_verilog_and_vhdl_as_versioned_families() {
        let root = find_root();

        for package_name in ["verilog-lexer", "vhdl-lexer"] {
            let targets = find_rust_compile_targets(&root, &[package_name.to_string()]).unwrap();
            let target = targets
                .iter()
                .find(|target| target.package_name == package_name)
                .expect("missing expected versioned lexer target");

            assert!(
                matches!(target.inputs, RustCompileInputs::Versioned(_)),
                "{package_name} should use versioned compiled grammars"
            );
        }
    }

    #[test]
    fn dispatch_validate_correct() {
        if !exists("json.tokens") || !exists("json.grammar") {
            return;
        }
        assert_eq!(
            dispatch(
                "validate",
                &[grammar_path("json.tokens"), grammar_path("json.grammar")],
                None,
                false,
            ),
            0
        );
    }

    #[test]
    fn dispatch_validate_tokens_correct() {
        if !exists("json.tokens") {
            return;
        }
        assert_eq!(
            dispatch(
                "validate-tokens",
                &[grammar_path("json.tokens")],
                None,
                false
            ),
            0
        );
    }

    #[test]
    fn dispatch_validate_grammar_correct() {
        if !exists("json.grammar") {
            return;
        }
        assert_eq!(
            dispatch(
                "validate-grammar",
                &[grammar_path("json.grammar")],
                None,
                false
            ),
            0
        );
    }

    // -----------------------------------------------------------------------
    // compile_tokens_command
    // -----------------------------------------------------------------------

    #[test]
    fn compile_tokens_command_missing_returns_1() {
        assert_eq!(
            compile_tokens_command("/nonexistent/x.tokens", None, false),
            1
        );
    }

    #[test]
    fn compile_tokens_command_to_stdout_succeeds() {
        if !exists("json.tokens") {
            return;
        }
        assert_eq!(
            compile_tokens_command(&grammar_path("json.tokens"), None, false),
            0
        );
    }

    #[test]
    fn compile_tokens_command_to_file_succeeds() {
        if !exists("json.tokens") {
            return;
        }
        let out = std::env::temp_dir().join("json_tokens_test.rs");
        let result = compile_tokens_command(
            &grammar_path("json.tokens"),
            Some(out.to_str().unwrap()),
            false,
        );
        assert_eq!(result, 0);
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(content.contains("TOKEN_GRAMMAR") || content.contains("token_grammar"));
        assert!(content.contains("DO NOT EDIT"));
    }

    #[test]
    fn dispatch_compile_tokens_correct() {
        if !exists("json.tokens") {
            return;
        }
        assert_eq!(
            dispatch(
                "compile-tokens",
                &[grammar_path("json.tokens")],
                None,
                false
            ),
            0
        );
    }

    // -----------------------------------------------------------------------
    // compile_grammar_command
    // -----------------------------------------------------------------------

    #[test]
    fn compile_grammar_command_missing_returns_1() {
        assert_eq!(
            compile_grammar_command("/nonexistent/x.grammar", None, false),
            1
        );
    }

    #[test]
    fn compile_grammar_command_to_stdout_succeeds() {
        if !exists("json.grammar") {
            return;
        }
        assert_eq!(
            compile_grammar_command(&grammar_path("json.grammar"), None, false),
            0
        );
    }

    #[test]
    fn compile_grammar_command_to_file_succeeds() {
        if !exists("json.grammar") {
            return;
        }
        let out = std::env::temp_dir().join("json_grammar_test.rs");
        let result = compile_grammar_command(
            &grammar_path("json.grammar"),
            Some(out.to_str().unwrap()),
            false,
        );
        assert_eq!(result, 0);
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(content.contains("PARSER_GRAMMAR") || content.contains("parser_grammar"));
        assert!(content.contains("DO NOT EDIT"));
    }

    #[test]
    fn dispatch_compile_grammar_correct() {
        if !exists("json.grammar") {
            return;
        }
        assert_eq!(
            dispatch(
                "compile-grammar",
                &[grammar_path("json.grammar")],
                None,
                false
            ),
            0
        );
    }

    // -- LANG84 inventory-driven families ----------------------------------

    #[test]
    fn release_inventory_reads_columns_by_header_name() {
        let csv = "# provenance\n\
                   grammar_status,date,release\n\
                   partial,1987,1.000\n\
                   pending,1988,1.001\r\n";
        let rows = parse_release_inventory(csv).unwrap();
        assert_eq!(
            rows,
            vec![
                InventoryRow { release: "1.000".into(), grammar_status: "partial".into() },
                InventoryRow { release: "1.001".into(), grammar_status: "pending".into() },
            ]
        );
    }

    /// LANG84 phase 2b: Perl's inventory now has the common header, with
    /// `grammar_status` seventh and LANG82's `perlhist_line` as an extra.
    #[test]
    fn release_inventory_reads_the_lang84_header() {
        let csv = "# LANG84 release inventory: Perl\n\
                   release,date,channel,source_ref,source_sha,predecessor,grammar_status,\
                   grammar_stage,grammar_subset,iir_status,backend_status,profile,\
                   perlhist_line,lineage\n\
                   1.000,1987-12-18,final,https://x/perl-1.0,8d063cd8450e59ea1c611a2f4f5a21059a2804f1,\
                   ,partial,P,pl-decimal-print,-,-,-,78,first\n\
                   1.001,1988-01-30,patch,https://x/perlhist.pod#L80,\
                   d97e2dc0ca5fcae83ac557cf21372672cd497510,1.000,pending,none,-,-,-,-,80,line\n";
        let rows = parse_release_inventory(csv).unwrap();
        assert_eq!(
            rows,
            vec![
                InventoryRow { release: "1.000".into(), grammar_status: "partial".into() },
                InventoryRow { release: "1.001".into(), grammar_status: "pending".into() },
            ]
        );
    }

    #[test]
    fn release_inventory_refuses_bad_rows() {
        for (csv, why) in [
            ("release,date\n1.0,1987\n", "grammar_status"),
            ("date,grammar_status\n1987,partial\n", "release"),
            ("release,grammar_status\n../x,partial\n", "unsafe"),
            ("release,grammar_status\na/b,partial\n", "unsafe"),
            ("release,grammar_status\n,partial\n", "unsafe"),
            ("release,grammar_status\n1.0,done\n", "grammar_status"),
            ("release,grammar_status\n1.0,partial\n1.0,pending\n", "duplicate"),
            ("release,grammar_status\n1.0\n", "1 fields, the header has 2"),
            ("release,grammar_status\n1.0,partial,x\n", "3 fields, the header has 2"),
            ("release,grammar_status\n", "no release rows"),
            ("# only a comment\n", "no header"),
        ] {
            let error = parse_release_inventory(csv).unwrap_err();
            assert!(error.contains(why), "{csv:?}: {error}");
        }
    }

    #[test]
    fn canonical_form_ignores_only_comments_and_line_numbers() {
        let a = "// Source: a.tokens\nX {\n    line_number: 5,\n    name: \"A\",\n}\n";
        let b = "// Source: b.tokens\nX {\n    line_number: 9,\n    name: \"A\",\n}\n";
        let c = "// Source: a.tokens\nX {\n    line_number: 5,\n    name: \"B\",\n}\n";
        assert_eq!(canonical_compiled_grammar(a), canonical_compiled_grammar(b));
        assert_ne!(canonical_compiled_grammar(a), canonical_compiled_grammar(c));
        assert_eq!(
            dedup_module_name(&canonical_compiled_grammar(a)),
            dedup_module_name(&canonical_compiled_grammar(b))
        );
        assert!(dedup_module_name("x").starts_with("g_"));
        assert_eq!(dedup_module_name("x").len(), 14);
    }

    #[test]
    fn perl_is_an_inventory_driven_family_with_deduplicated_modules() {
        let root = find_root();
        let targets = find_rust_compile_targets(&root, &["perl".to_string()]).unwrap();
        let names: Vec<_> = targets.iter().map(|t| t.package_name.as_str()).collect();
        assert_eq!(names, ["perl-lexer", "perl-parser"]);
        for target in &targets {
            let RustCompileInputs::Versioned(family) = &target.inputs else {
                panic!("{} must be versioned", target.package_name);
            };
            let inventory = family.inventory.as_ref().expect("CSV-driven");
            assert!(inventory.len() > family.versions.len());
            assert_eq!(inventory[0], "1.000");
            assert_eq!(family.versions[0].version, "1.000");
            let code = compile_rust_target_to_string(target, target.force).unwrap();
            let modules = code.lines().filter(|l| l.starts_with("mod g_")).count();
            assert!(modules >= 1 && modules < family.versions.len(), "{modules} modules");
            // Every compiled release appears exactly once as a match arm.
            for source in &family.versions {
                let arm = format!("{:?}\n", source.version);
                let arms = code
                    .lines()
                    .filter(|l| {
                        let l = l.trim_start().trim_start_matches("| ");
                        format!("{l}\n") == arm
                    })
                    .count();
                assert_eq!(arms, 1, "{}", source.version);
            }
            // The unversioned pair is a named function, never the "" release.
            assert!(code.contains("pub fn unversioned_"));
            assert!(!code.contains("\"\" =>"));
        }
    }
}
