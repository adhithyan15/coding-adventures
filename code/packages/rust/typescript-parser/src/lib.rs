//! # TypeScript Parser — parsing TypeScript source code into an AST.
//!
//! This crate is the second half of the TypeScript front-end pipeline. Where
//! the `typescript-lexer` crate breaks source text into tokens, this crate
//! arranges those tokens into a tree that reflects the **structure** of the
//! code — an Abstract Syntax Tree (AST).
//!
//! # The parsing pipeline
//!
//! Parsing TypeScript requires four cooperating components:
//!
//! ```text
//! Source code  ("let x: number = 1 + 2;")
//!       |
//!       v
//! typescript-lexer     → Vec<Token>
//!       |                [KEYWORD("let"), NAME("x"), COLON(":"),
//!       |                 KEYWORD("number"), EQUALS("="),
//!       |                 NUMBER("1"), PLUS("+"), NUMBER("2"),
//!       |                 SEMICOLON(";"), EOF]
//!       v
//! typescript.grammar   → ParserGrammar (rules like "program = ...")
//!       |
//!       v
//! GrammarParser        → GrammarASTNode tree
//!       |
//!       |                program
//!       |                  └── statement
//!       |                        └── var_declaration
//!       |                              ├── KEYWORD("let")
//!       |                              ├── NAME("x")
//!       |                              ├── type_annotation
//!       |                              ├── EQUALS("=")
//!       |                              └── expression
//!       v
//! [future stages: type checking, compilation]
//! ```
//!
//! This crate is the thin glue layer that wires these components together.
//! It knows where to find the grammar files and provides two public entry
//! points.
//!
//! # Version-Aware API
//!
//! Both entry points accept a `version` parameter that selects which grammar
//! file to use:
//!
//! | `version` | grammar file loaded |
//! |---|---|
//! | `""` (empty) | `grammars/typescript.grammar` (generic) |
//! | `"ts1.0"` | `grammars/typescript/ts1.0.grammar` |
//! | `"ts2.0"` | `grammars/typescript/ts2.0.grammar` |
//! | `"ts3.0"` | `grammars/typescript/ts3.0.grammar` |
//! | `"ts4.0"` | `grammars/typescript/ts4.0.grammar` |
//! | `"ts5.0"` | `grammars/typescript/ts5.0.grammar` |
//! | `"ts5.8"` | `grammars/typescript/ts5.8.grammar` |
//!
//! An unknown version string returns `Err(String)`.
//!
//! # TypeScript vs. JavaScript grammar
//!
//! The TypeScript grammar extends JavaScript's grammar with type annotations,
//! interface declarations, type aliases, enum declarations, and generic type
//! parameters. At the structural level, this means additional grammar rules
//! for type_annotation, interface_declaration, type_alias, and enum_declaration.

use std::fs;
use std::path::PathBuf;

use grammar_tools::parser_grammar::parse_parser_grammar;
use parser::grammar_parser::{GrammarParser, GrammarASTNode};
use coding_adventures_typescript_lexer::tokenize_typescript;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Returns the root `grammars/` directory by navigating up from this crate.
///
/// ```text
/// code/
///   grammars/           <-- returned by this function
///   packages/
///     rust/
///       typescript-parser/
///         Cargo.toml    <-- env!("CARGO_MANIFEST_DIR")
/// ```
fn grammar_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("grammars")
}

/// Validate the TypeScript version string and return the path to the
/// corresponding `.grammar` file.
///
/// Valid version strings are:
/// - `""` — selects the generic `typescript.grammar`
/// - `"ts1.0"`, `"ts2.0"`, `"ts3.0"`, `"ts4.0"`, `"ts5.0"`, `"ts5.8"`
///   — selects `typescript/<version>.grammar`
///
/// Returns `Err(String)` for any unrecognised version string.
fn grammar_path(version: &str) -> Result<PathBuf, String> {
    let root = grammar_root();

    match version {
        // Empty string → the generic, version-agnostic grammar.
        "" => Ok(root.join("typescript.grammar")),

        // Versioned TypeScript grammars live in grammars/typescript/.
        "ts1.0" | "ts2.0" | "ts3.0" | "ts4.0" | "ts5.0" | "ts5.8" => {
            Ok(root.join("typescript").join(format!("{version}.grammar")))
        }

        // Anything else is an error — we'd rather fail loudly than silently
        // fall back to the generic grammar and produce confusing results.
        other => Err(format!(
            "Unknown TypeScript version '{other}'. \
             Valid values: \"\", \"ts1.0\", \"ts2.0\", \"ts3.0\", \
             \"ts4.0\", \"ts5.0\", \"ts5.8\""
        )),
    }
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarParser` configured for TypeScript source code.
///
/// The `version` parameter selects which grammar file to load:
/// - `""` — uses the generic `typescript.grammar` (recommended for most use
///   cases where you don't need version-specific behaviour).
/// - `"ts1.0"` through `"ts5.8"` — uses a version-specific grammar that
///   matches the grammar rules of that TypeScript release.
///
/// This function performs two major steps:
///
/// 1. **Tokenization** — uses `tokenize_typescript` from the typescript-lexer
///    crate to break the source into tokens (also with the same `version`).
///
/// 2. **Grammar loading** — reads and parses the appropriate `.grammar` file,
///    which defines rules for programs, statements, expressions, type
///    annotations, interfaces, enums, and more.
///
/// The returned `GrammarParser` is ready to call `.parse()` on.
///
/// # Errors
///
/// Returns `Err(String)` if:
/// - The `version` string is not recognised.
/// - The grammar file cannot be read or parsed.
/// - The source code fails tokenization (unexpected character).
///
/// # Example
///
/// ```no_run
/// use coding_adventures_typescript_parser::create_typescript_parser;
///
/// // Generic grammar:
/// let mut parser = create_typescript_parser("let x: number = 42;", "").unwrap();
/// let ast = parser.parse().expect("parse failed");
/// println!("{:?}", ast.rule_name);
///
/// // TypeScript 5.8 grammar:
/// let mut parser58 = create_typescript_parser("let x = 1;", "ts5.8").unwrap();
/// ```
pub fn create_typescript_parser(source: &str, version: &str) -> Result<GrammarParser, String> {
    // Step 1: Tokenize the source using the typescript-lexer (same version).
    let tokens = tokenize_typescript(source, version)?;

    // Step 2: Resolve the parser grammar file path.
    let path = grammar_path(version)?;

    // Step 3: Read the parser grammar from disk.
    let grammar_text = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

    // Step 4: Parse the grammar text into a structured ParserGrammar.
    let grammar = parse_parser_grammar(&grammar_text)
        .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;

    // Step 5: Create the parser.
    Ok(GrammarParser::new(tokens, grammar))
}

/// Parse TypeScript source code into an AST.
///
/// This is the most convenient entry point — it handles tokenization,
/// grammar loading, parser creation, and parsing in one call.
///
/// The `version` parameter is the same as for [`create_typescript_parser`]:
/// pass `""` for the generic grammar or `"ts5.8"` etc. for a versioned one.
///
/// The returned `GrammarASTNode` has `rule_name` set to `"program"` (the
/// start symbol of the TypeScript grammar) with children corresponding
/// to the statements in the source.
///
/// # Errors
///
/// Returns `Err(String)` if the version is unknown, the grammar file is
/// missing or malformed, or the source has a syntax error.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_typescript_parser::parse_typescript;
///
/// // Generic grammar:
/// let ast = parse_typescript("let x: number = 1 + 2;", "").unwrap();
/// assert_eq!(ast.rule_name, "program");
///
/// // TypeScript 4.0 grammar:
/// let ast_v4 = parse_typescript("let x = 1;", "ts4.0").unwrap();
/// ```
pub fn parse_typescript(source: &str, version: &str) -> Result<GrammarASTNode, String> {
    let mut ts_parser = create_typescript_parser(source, version)?;

    ts_parser
        .parse()
        .map_err(|e| format!("TypeScript parse failed: {e}"))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn assert_program_root(ast: &GrammarASTNode) {
        assert_eq!(
            ast.rule_name, "program",
            "Expected root rule 'program', got '{}'",
            ast.rule_name
        );
    }

    fn count_statements(ast: &GrammarASTNode) -> usize {
        ast.children.iter().filter(|child| {
            matches!(child, ASTNodeOrToken::Node(n) if n.rule_name == "statement")
        }).count()
    }

    fn find_rule(node: &GrammarASTNode, target_rule: &str) -> bool {
        if node.rule_name == target_rule {
            return true;
        }
        for child in &node.children {
            if let ASTNodeOrToken::Node(child_node) = child {
                if find_rule(child_node, target_rule) {
                    return true;
                }
            }
        }
        false
    }

    // -----------------------------------------------------------------------
    // Test 1: Arithmetic expression (generic grammar)
    // -----------------------------------------------------------------------

    /// An expression statement with binary arithmetic.
    #[test]
    fn test_parse_expression() {
        let ast = parse_typescript("1 + 2;", "").unwrap();
        assert_program_root(&ast);
        assert!(!ast.children.is_empty(), "AST should have children");
    }

    // -----------------------------------------------------------------------
    // Test 2: Empty program (generic grammar)
    // -----------------------------------------------------------------------

    /// An empty program should parse to a program node with no children.
    #[test]
    fn test_parse_empty_program() {
        let ast = parse_typescript("", "").unwrap();
        assert_program_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 3: Factory function
    // -----------------------------------------------------------------------

    /// The `create_typescript_parser` factory function should return a
    /// working `GrammarParser`.
    #[test]
    fn test_create_parser() {
        let mut parser = create_typescript_parser("let x = 1;", "").unwrap();
        let result = parser.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());

        let ast = result.unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    // -----------------------------------------------------------------------
    // Test 4: Versioned grammar — ts5.8
    // -----------------------------------------------------------------------

    /// The ts5.8 versioned grammar should parse an arithmetic expression.
    #[test]
    fn test_versioned_ts58() {
        let ast = parse_typescript("1 + 2;", "ts5.8").unwrap();
        assert_program_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 5: All versioned grammars parse an empty program
    // -----------------------------------------------------------------------

    /// Every versioned TypeScript grammar should successfully parse an empty
    /// program (the simplest valid input).
    #[test]
    fn test_all_versioned_grammars() {
        let versions = ["ts1.0", "ts2.0", "ts3.0", "ts4.0", "ts5.0", "ts5.8"];
        for v in &versions {
            let result = parse_typescript("", v);
            assert!(result.is_ok(), "Version '{v}' should parse successfully: {:?}", result.err());
            assert_eq!(result.unwrap().rule_name, "program");
        }
    }

    // -----------------------------------------------------------------------
    // Test 6: Unknown version returns Err
    // -----------------------------------------------------------------------

    /// Passing an unrecognised version string should return Err, not panic.
    #[test]
    fn test_unknown_version_returns_err() {
        let result = parse_typescript("let x = 1;", "ts99.0");
        assert!(result.is_err(), "Expected Err for unknown version 'ts99.0'");

        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("ts99.0"),
            "Error message should mention the bad version: {err_msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 7: create_typescript_parser with unknown version returns Err
    // -----------------------------------------------------------------------

    /// The factory function should also return Err for unknown versions.
    #[test]
    fn test_create_parser_unknown_version() {
        let result = create_typescript_parser("let x = 1;", "bad-version");
        assert!(result.is_err(), "Expected Err from create_typescript_parser with bad version");
    }
}
