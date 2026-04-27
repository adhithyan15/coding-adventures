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
use grammar_tools::compiler::{compile_parser_grammar, compile_token_grammar};
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
}

impl RustGrammarKind {
    fn input_extension(self) -> &'static str {
        match self {
            Self::Tokens => "tokens",
            Self::Grammar => "grammar",
        }
    }

    fn package_suffix(self) -> &'static str {
        match self {
            Self::Tokens => "-lexer",
            Self::Grammar => "-parser",
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
    versions: &'static [&'static str],
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
        }),
        "fsharp" => Some(RustVersionedFamilySpec {
            family_name: "fsharp",
            filter_names: &["fsharp"],
            input_dir: "fsharp",
            input_prefix: "fsharp",
            generic_stem: None,
            versions: FSHARP_VERSIONS,
        }),
        "haskell" => Some(RustVersionedFamilySpec {
            family_name: "haskell",
            filter_names: &["haskell"],
            input_dir: "haskell",
            input_prefix: "haskell",
            generic_stem: None,
            versions: HASKELL_VERSIONS,
        }),
        "java" => Some(RustVersionedFamilySpec {
            family_name: "java",
            filter_names: &["java"],
            input_dir: "java",
            input_prefix: "java",
            generic_stem: None,
            versions: JAVA_VERSIONS,
        }),
        "javascript" => Some(RustVersionedFamilySpec {
            family_name: "ecmascript",
            filter_names: &["javascript", "ecmascript"],
            input_dir: "ecmascript",
            input_prefix: "",
            generic_stem: Some("javascript"),
            versions: ECMASCRIPT_VERSIONS,
        }),
        "python" => Some(RustVersionedFamilySpec {
            family_name: "python",
            filter_names: &["python"],
            input_dir: "python",
            input_prefix: "python",
            generic_stem: None,
            versions: PYTHON_VERSIONS,
        }),
        "typescript" => Some(RustVersionedFamilySpec {
            family_name: "typescript",
            filter_names: &["typescript"],
            input_dir: "typescript",
            input_prefix: "",
            generic_stem: Some("typescript"),
            versions: TYPESCRIPT_VERSIONS,
        }),
        "verilog" => Some(RustVersionedFamilySpec {
            family_name: "verilog",
            filter_names: &["verilog"],
            input_dir: "verilog",
            input_prefix: "verilog",
            generic_stem: None,
            versions: VERILOG_VERSIONS,
        }),
        "vhdl" => Some(RustVersionedFamilySpec {
            family_name: "vhdl",
            filter_names: &["vhdl"],
            input_dir: "vhdl",
            input_prefix: "vhdl",
            generic_stem: None,
            versions: VHDL_VERSIONS,
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
            let input_path = grammars_dir.join(format!("{}.{}", stem, kind.input_extension()));
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

    let mut versions = Vec::new();
    for version in spec.versions {
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
            version: (*version).to_string(),
            input_path,
            module_name: rust_module_name_for_version(version),
        });
    }

    Ok(RustVersionedFamilyTarget {
        family_name: spec.family_name.to_string(),
        generic_input,
        versions,
    })
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
                let input_path =
                    grammars_dir.join(format!("{}.{}", grammar_name, kind.input_extension()));
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
    };

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
        root.join("code")
            .join("grammars")
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
}
