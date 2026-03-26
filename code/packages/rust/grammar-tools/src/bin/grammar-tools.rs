//! # grammar-tools CLI — validate `.tokens` and `.grammar` files from the command line.
//!
//! This binary exposes the validation functions from the `grammar_tools` library
//! as a command-line tool. It is the Rust counterpart of the Python
//! `python -m grammar_tools` CLI and produces identical output so that CI
//! scripts can use either implementation interchangeably.
//!
//! # Why a CLI binary?
//!
//! The library functions (`validate_token_grammar`, `validate_parser_grammar`,
//! `cross_validate`) exist as pure Rust functions, but grammar authors need a
//! quick way to check files from the terminal — without writing any Rust code.
//! Think of this like a compiler's `-fsyntax-only` flag: parse and validate,
//! report what is wrong, exit non-zero on failure.
//!
//! # Usage
//!
//! ```text
//! grammar-tools validate <file.tokens> <file.grammar>
//! grammar-tools validate-tokens <file.tokens>
//! grammar-tools validate-grammar <file.grammar>
//! grammar-tools --help
//! ```
//!
//! # Exit codes
//!
//! | Code | Meaning                                      |
//! |------|----------------------------------------------|
//! |  0   | All checks passed                            |
//! |  1   | One or more validation errors found          |
//! |  2   | Wrong number of arguments (usage error)      |
//!
//! # Output format (consistent with Python implementation)
//!
//! Success:
//! ```text
//! Validating lattice.tokens ... OK (N tokens, M skip, K error)
//! Validating lattice.grammar ... OK (P rules)
//! Cross-validating ... OK
//!
//! All checks passed.
//! ```
//!
//! Failure:
//! ```text
//! Validating broken.tokens ... 2 error(s)
//!   Line 5: Duplicate token name 'IDENT' ...
//! Found 4 error(s). Fix them and try again.
//! ```

use std::collections::HashSet;
use std::path::Path;
use std::process;

use grammar_tools::cross_validator::cross_validate;
use grammar_tools::parser_grammar::{parse_parser_grammar, validate_parser_grammar};
use grammar_tools::token_grammar::{parse_token_grammar, token_names, validate_token_grammar};
use grammar_tools::compiler::{compile_tokens_to_rust, compile_parser_to_rust};

// ===========================================================================
// Count errors (vs warnings)
// ===========================================================================

/// Count how many issues are actual errors (not warnings).
///
/// Issues starting with `"Warning"` are informational and do not cause the
/// tool to fail. Everything else counts as a real error.
///
/// This mirrors `_count_errors()` in the Python implementation.
fn count_errors(issues: &[String]) -> usize {
    issues.iter().filter(|i| !i.starts_with("Warning")).count()
}

/// Print a list of issues with two-space indentation.
fn print_issues(issues: &[String]) {
    for issue in issues {
        println!("  {}", issue);
    }
}

// ===========================================================================
// validate command — validate a (.tokens, .grammar) pair
// ===========================================================================

/// Validate a `.tokens` file and a `.grammar` file together.
///
/// This is the core of the `validate` subcommand:
/// 1. Parse the `.tokens` file and run `validate_token_grammar`.
/// 2. Parse the `.grammar` file and run `validate_parser_grammar`.
/// 3. Cross-validate the two with `cross_validate`.
///
/// Returns 0 if all checks pass, 1 if any errors are found.
fn validate_command(tokens_path: &str, grammar_path: &str) -> i32 {
    let mut total_errors: usize = 0;

    // -----------------------------------------------------------------------
    // Step 1: Parse and validate the .tokens file.
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
        // Build the "OK (N tokens, M skip, K error)" summary line.
        // Only include non-zero counts to avoid noise (mirrors Python).
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
    // Step 2: Parse and validate the .grammar file.
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

    // Pass the token names so undefined token references are caught.
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
    // Step 3: Cross-validate the two files together.
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

    // -----------------------------------------------------------------------
    // Summary
    // -----------------------------------------------------------------------
    if total_errors > 0 {
        println!("\nFound {} error(s). Fix them and try again.", total_errors);
        1
    } else {
        println!("\nAll checks passed.");
        0
    }
}

// ===========================================================================
// validate-tokens command — validate only a .tokens file
// ===========================================================================

/// Validate just a `.tokens` file (no grammar file needed).
///
/// Returns 0 if all checks pass, 1 if any errors are found.
fn validate_tokens_only(tokens_path: &str) -> i32 {
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
// validate-grammar command — validate only a .grammar file
// ===========================================================================

/// Validate just a `.grammar` file (no tokens file needed).
///
/// Because we have no `.tokens` file, we can only check rule-level issues
/// (undefined rule references, duplicate rules, naming conventions) — not
/// undefined token references.
///
/// Returns 0 if all checks pass, 1 if any errors are found.
fn validate_grammar_only(grammar_path: &str) -> i32 {
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

    // Without a tokens file, pass None so token-reference checks are skipped.
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
// Help text
// ===========================================================================

/// Print usage information to stdout.
fn print_usage() {
    println!("Usage: grammar-tools <command> [args...]");
    println!();
    println!("Commands:");
    println!("  validate <file.tokens> <file.grammar>  Validate a token/grammar pair");
    println!("  validate-tokens <file.tokens>           Validate just a .tokens file");
    println!("  validate-grammar <file.grammar>         Validate just a .grammar file");
    println!("  compile-tokens <file.tokens> <export_name> Compile just a .tokens file to rust");
    println!("  compile-grammar <file.grammar> <export_name> Compile just a .grammar file to rust");
    println!();
    println!("Examples:");
    println!("  grammar-tools validate css.tokens css.grammar");
    println!("  grammar-tools validate-tokens css.tokens");
    println!("  grammar-tools validate-grammar css.grammar");
    println!("  grammar-tools compile-tokens json.tokens json_tokens");
    println!("  grammar-tools compile-grammar json.grammar json_grammar");
    println!();
    println!("Exit codes:");
    println!("  0   All checks passed");
    println!("  1   One or more validation errors found");
    println!("  2   Wrong number of arguments (usage error)");
}

// ===========================================================================
// main
// ===========================================================================

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // args[0] is the binary name; args[1..] are the user-supplied arguments.
    let user_args = &args[1..];

    let exit_code = if user_args.is_empty()
        || user_args[0] == "--help"
        || user_args[0] == "-h"
        || user_args[0] == "help"
    {
        print_usage();
        0
    } else {
        let command = &user_args[0];
        let rest = &user_args[1..];

        match command.as_str() {
            "validate" => {
                if rest.len() != 2 {
                    eprintln!(
                        "Error: 'validate' requires two arguments: <tokens> <grammar>"
                    );
                    eprintln!();
                    print_usage();
                    2
                } else {
                    validate_command(&rest[0], &rest[1])
                }
            }

            "validate-tokens" => {
                if rest.len() != 1 {
                    eprintln!(
                        "Error: 'validate-tokens' requires one argument: <tokens>"
                    );
                    eprintln!();
                    print_usage();
                    2
                } else {
                    validate_tokens_only(&rest[0])
                }
            }

            "validate-grammar" => {
                if rest.len() != 1 {
                    eprintln!(
                        "Error: 'validate-grammar' requires one argument: <grammar>"
                    );
                    eprintln!();
                    print_usage();
                    2
                } else {
                    validate_grammar_only(&rest[0])
                }
            }

            "compile-tokens" => {
                if rest.len() != 2 {
                    eprintln!(
                        "Error: 'compile-tokens' requires two arguments: <tokens> <export_name>"
                    );
                    eprintln!();
                    print_usage();
                    2
                } else {
                    match std::fs::read_to_string(&rest[0]) {
                        Ok(s) => match parse_token_grammar(&s) {
                            Ok(g) => {
                                let issues = validate_token_grammar(&g);
                                if count_errors(&issues) > 0 {
                                    eprintln!("Error: Cannot compile invalid grammar file.");
                                    print_issues(&issues);
                                    1
                                } else {
                                    print!("{}", compile_tokens_to_rust(&g, &rest[1]));
                                    0
                                }
                            }
                            Err(e) => {
                                eprintln!("PARSE ERROR\n  {}", e);
                                1
                            }
                        },
                        Err(e) => {
                            eprintln!("ERROR\n  {}", e);
                            1
                        }
                    }
                }
            }

            "compile-grammar" => {
                if rest.len() != 2 {
                    eprintln!(
                        "Error: 'compile-grammar' requires two arguments: <grammar> <export_name>"
                    );
                    eprintln!();
                    print_usage();
                    2
                } else {
                    match std::fs::read_to_string(&rest[0]) {
                        Ok(s) => match parse_parser_grammar(&s) {
                            Ok(g) => {
                                let issues = validate_parser_grammar(&g, None::<&HashSet<String>>);
                                if count_errors(&issues) > 0 {
                                    eprintln!("Error: Cannot compile invalid grammar file.");
                                    print_issues(&issues);
                                    1
                                } else {
                                    print!("{}", compile_parser_to_rust(&g, &rest[1]));
                                    0
                                }
                            }
                            Err(e) => {
                                eprintln!("PARSE ERROR\n  {}", e);
                                1
                            }
                        },
                        Err(e) => {
                            eprintln!("ERROR\n  {}", e);
                            1
                        }
                    }
                }
            }

            other => {
                eprintln!("Error: Unknown command '{}'", other);
                eprintln!();
                print_usage();
                2
            }
        }
    };

    process::exit(exit_code);
}
