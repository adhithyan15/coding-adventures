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

    let tokens_source = match std::fs::read_to_string(tokens_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ERROR");
            eprintln!("  {}", e);
            return 1;
        }
    };

    let token_grammar = match parse_token_grammar(&tokens_source) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("PARSE ERROR");
            eprintln!("  {}", e);
            return 1;
        }
    };

    if !force {
        let issues = validate_token_grammar(&token_grammar);
        let errors = count_errors(&issues);
        if errors > 0 {
            eprintln!("{} error(s)", errors);
            print_issues(&issues);
            return 1;
        }
    }

    let code = compile_token_grammar(&token_grammar, tokens_filename);

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

    let grammar_source = match std::fs::read_to_string(grammar_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ERROR");
            eprintln!("  {}", e);
            return 1;
        }
    };

    let parser_grammar = match parse_parser_grammar(&grammar_source) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("PARSE ERROR");
            eprintln!("  {}", e);
            return 1;
        }
    };

    if !force {
        let issues = validate_parser_grammar(&parser_grammar, None::<&HashSet<String>>);
        let errors = count_errors(&issues);
        if errors > 0 {
            eprintln!("{} error(s)", errors);
            print_issues(&issues);
            return 1;
        }
    }

    let code = compile_parser_grammar(&parser_grammar, grammar_filename);

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
    let spec_path = root
        .join("code")
        .join("specs")
        .join("grammar-tools.json");

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
        assert_eq!(dispatch("validate", &["one.tokens".to_string()], None, false), 2);
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
            dispatch("validate-tokens", &[grammar_path("json.tokens")], None, false),
            0
        );
    }

    #[test]
    fn dispatch_validate_grammar_correct() {
        if !exists("json.grammar") {
            return;
        }
        assert_eq!(
            dispatch("validate-grammar", &[grammar_path("json.grammar")], None, false),
            0
        );
    }

    // -----------------------------------------------------------------------
    // compile_tokens_command
    // -----------------------------------------------------------------------

    #[test]
    fn compile_tokens_command_missing_returns_1() {
        assert_eq!(compile_tokens_command("/nonexistent/x.tokens", None, false), 1);
    }

    #[test]
    fn compile_tokens_command_to_stdout_succeeds() {
        if !exists("json.tokens") {
            return;
        }
        assert_eq!(compile_tokens_command(&grammar_path("json.tokens"), None, false), 0);
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
            dispatch("compile-tokens", &[grammar_path("json.tokens")], None, false),
            0
        );
    }

    // -----------------------------------------------------------------------
    // compile_grammar_command
    // -----------------------------------------------------------------------

    #[test]
    fn compile_grammar_command_missing_returns_1() {
        assert_eq!(compile_grammar_command("/nonexistent/x.grammar", None, false), 1);
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
            dispatch("compile-grammar", &[grammar_path("json.grammar")], None, false),
            0
        );
    }
}
