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

fn compile_tokens_only(tokens_path: &str, export_name: &str) -> i32 {
    match std::fs::read_to_string(tokens_path) {
        Ok(s) => match parse_token_grammar(&s) {
            Ok(g) => {
                let issues = validate_token_grammar(&g);
                if count_errors(&issues) > 0 {
                    eprintln!("Error: Cannot compile invalid grammar file.");
                    print_issues(&issues);
                    1
                } else {
                    print!("{}", compile_tokens_to_rust(&g, export_name));
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

fn compile_grammar_only(grammar_path: &str, export_name: &str) -> i32 {
    match std::fs::read_to_string(grammar_path) {
        Ok(s) => match parse_parser_grammar(&s) {
            Ok(g) => {
                let issues = validate_parser_grammar(&g, None::<&HashSet<String>>);
                if count_errors(&issues) > 0 {
                    eprintln!("Error: Cannot compile invalid grammar file.");
                    print_issues(&issues);
                    1
                } else {
                    print!("{}", compile_parser_to_rust(&g, export_name));
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

use std::path::PathBuf;

fn to_camel_case(snake_str: &str) -> String {
    let mut camel = String::new();
    let mut capitalize_next = true;
    for c in snake_str.chars() {
        if c == '_' || c == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            camel.extend(c.to_uppercase());
            capitalize_next = false;
        } else {
            camel.push(c);
        }
    }
    camel
}

fn find_monorepo_root() -> Option<PathBuf> {
    let mut current_dir = std::env::current_dir().ok()?;
    loop {
        let grammars = current_dir.join("code").join("grammars");
        if grammars.is_dir() {
            return Some(current_dir);
        }
        if !current_dir.pop() {
            break;
        }
    }
    None
}

fn generate_command() -> i32 {
    let mut has_errors = false;
    let root = match find_monorepo_root() {
        Some(p) => p,
        None => {
            eprintln!("Error: could not find monorepo root");
            return 1;
        }
    };

    let grammars_dir = root.join("code").join("grammars");
    let lang_dir = root.join("code").join("packages").join("rust");

    let entries = match std::fs::read_dir(&grammars_dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error reading grammars dir: {}", e);
            return 1;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "tokens" && ext != "grammar" {
            continue;
        }

        let is_tokens = ext == "tokens";
        let kind = if is_tokens { "lexer" } else { "parser" };
        let gn = path.file_stem().and_then(|n| n.to_str()).unwrap_or("");

        let possible_dirs = vec![
            lang_dir.join(format!("{}-{}", gn, kind)),
            lang_dir.join(format!("{}_{}", gn, kind)),
        ];

        let mut target_dir = None;
        for pd in possible_dirs {
            if pd.is_dir() {
                target_dir = Some(pd);
                break;
            }
        }

        let target_dir = match target_dir {
            Some(d) => d,
            None => continue,
        };

        println!("Generating for {} ...", path.file_name().unwrap().to_str().unwrap());

        let var_suffix = if is_tokens { "Tokens" } else { "Grammar" };
        let export_name = format!("{}{}", to_camel_case(&gn.replace('-', "_")), var_suffix);
        let fname_base = if is_tokens { format!("{}_tokens.rs", gn.replace('-', "_")) } else { format!("{}_grammar.rs", gn.replace('-', "_")) };
        let out_path = target_dir.join("src").join(&fname_base);

        let source = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => {
                has_errors = true;
                continue;
            }
        };

        let code = if is_tokens {
            match parse_token_grammar(&source) {
                Ok(tg) => {
                    let issues = validate_token_grammar(&tg);
                    if count_errors(&issues) > 0 {
                        eprintln!("Error: Cannot compile invalid grammar file {:?}", path);
                        print_issues(&issues);
                        has_errors = true;
                        continue;
                    }
                    compile_tokens_to_rust(&tg, &export_name)
                }
                Err(e) => {
                    eprintln!("Error: parse failed for {:?}: {}", path, e);
                    has_errors = true;
                    continue;
                }
            }
        } else {
            match parse_parser_grammar(&source) {
                Ok(pg) => {
                    let issues = validate_parser_grammar(&pg, None::<&HashSet<String>>);
                    if count_errors(&issues) > 0 {
                        eprintln!("Error: Cannot compile invalid grammar file {:?}", path);
                        print_issues(&issues);
                        has_errors = true;
                        continue;
                    }
                    compile_parser_to_rust(&pg, &export_name)
                }
                Err(e) => {
                    eprintln!("Error: parse failed for {:?}: {}", path, e);
                    has_errors = true;
                    continue;
                }
            }
        };

        if !target_dir.join("src").is_dir() {
           std::fs::create_dir_all(target_dir.join("src")).unwrap_or(());
        }

        match std::fs::write(&out_path, code) {
            Ok(_) => println!("  -> Saved {:?}", out_path),
            Err(e) => {
                eprintln!("Error writing {:?}: {}", out_path, e);
                has_errors = true;
            }
        }
    }

    if has_errors { 1 } else { 0 }
}

use cli_builder::spec_loader::load_spec_from_file;
use cli_builder::parser::Parser;
use cli_builder::types::ParserOutput;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let spec_path: PathBuf = [manifest_dir, "..", "..", "..", "specs", "grammar-tools.cli.json"].iter().collect();

    let spec = match load_spec_from_file(spec_path.to_str().unwrap()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Spec Error: {}", e);
            process::exit(1);
        }
    };

    let parser = Parser::new(spec);
    match parser.parse(&args) {
        Ok(ParserOutput::Parse(result)) => {
            let cmd = result.command_path.last().unwrap().as_str();
            let exit_code = match cmd {
                "validate" => {
                    let tokens = result.arguments["tokens_file"].as_str().unwrap();
                    let grammar = result.arguments["grammar_file"].as_str().unwrap();
                    validate_command(tokens, grammar)
                }
                "validate-tokens" => {
                    let tokens = result.arguments["tokens_file"].as_str().unwrap();
                    validate_tokens_only(tokens)
                }
                "validate-grammar" => {
                    let grammar = result.arguments["grammar_file"].as_str().unwrap();
                    validate_grammar_only(grammar)
                }
                "compile-tokens" => {
                    let tokens = result.arguments["tokens_file"].as_str().unwrap();
                    let export_name = result.arguments["export_name"].as_str().unwrap();
                    compile_tokens_only(tokens, export_name)
                }
                "compile-grammar" => {
                    let grammar = result.arguments["grammar_file"].as_str().unwrap();
                    let export_name = result.arguments["export_name"].as_str().unwrap();
                    compile_grammar_only(grammar, export_name)
                }
                "generate" => {
                    generate_command()
                }
                _ => {
                    eprintln!("Error: Unknown command '{}'", cmd);
                    2
                }
            };
            process::exit(exit_code);
        }
        Ok(ParserOutput::Help(h)) => {
            println!("{}", h.text);
            process::exit(0);
        }
        Ok(ParserOutput::Version(v)) => {
            println!("{}", v.version);
            process::exit(0);
        }
        Err(cli_builder::errors::CliBuilderError::ParseErrors(e)) => {
            for err in e.errors {
                eprintln!("Error: {}", err.message);
                if let Some(sug) = err.suggestion {
                    eprintln!("  {}", sug);
                }
            }
            process::exit(2);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}
