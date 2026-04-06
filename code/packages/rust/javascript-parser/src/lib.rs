//! # JavaScript Parser — parsing JavaScript source code into an AST.
//!
//! This crate is the second half of the JavaScript front-end pipeline. Where
//! the `javascript-lexer` crate breaks source text into tokens, this crate
//! arranges those tokens into a tree that reflects the **structure** of the
//! code — an Abstract Syntax Tree (AST).
//!
//! # The parsing pipeline
//!
//! Parsing JavaScript requires four cooperating components:
//!
//! ```text
//! Source code  ("var x = 1 + 2;")
//!       |
//!       v
//! javascript-lexer     → Vec<Token>
//!       |                [KEYWORD("var"), NAME("x"), EQUALS("="),
//!       |                 NUMBER("1"), PLUS("+"), NUMBER("2"),
//!       |                 SEMICOLON(";"), EOF]
//!       v
//! javascript.grammar   → ParserGrammar (rules like "program = ...")
//!       |
//!       v
//! GrammarParser        → GrammarASTNode tree
//!       |
//!       |                program
//!       |                  └── statement
//!       |                        └── var_declaration
//!       |                              ├── KEYWORD("var")
//!       |                              ├── NAME("x")
//!       |                              ├── EQUALS("=")
//!       |                              └── expression
//!       v
//! [future stages: interpretation, compilation]
//! ```
//!
//! This crate is the thin glue layer that wires these components together.
//! It knows where to find the grammar files and provides two public entry
//! points.
//!
//! # Version-Aware API
//!
//! Both entry points accept a `version` parameter that selects which grammar
//! file to use. The JavaScript (ECMAScript) versioning scheme uses the
//! edition names defined by TC39:
//!
//! | `version` | grammar file loaded |
//! |---|---|
//! | `""` (empty) | `grammars/javascript.grammar` (generic) |
//! | `"es1"` | `grammars/ecmascript/es1.grammar` |
//! | `"es3"` | `grammars/ecmascript/es3.grammar` |
//! | `"es5"` | `grammars/ecmascript/es5.grammar` |
//! | `"es2015"` | `grammars/ecmascript/es2015.grammar` |
//! | `"es2016"` | `grammars/ecmascript/es2016.grammar` |
//! | … | … |
//! | `"es2025"` | `grammars/ecmascript/es2025.grammar` |
//!
//! An unknown version string returns `Err(String)`.

use std::fs;
use std::path::PathBuf;

use grammar_tools::parser_grammar::parse_parser_grammar;
use parser::grammar_parser::{GrammarParser, GrammarASTNode};
use coding_adventures_javascript_lexer::tokenize_javascript;

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
///       javascript-parser/
///         Cargo.toml    <-- env!("CARGO_MANIFEST_DIR")
/// ```
fn grammar_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("grammars")
}

/// Validate the JavaScript/ECMAScript version string and return the path to
/// the corresponding `.grammar` file.
///
/// Valid version strings are:
/// - `""` — selects the generic `javascript.grammar`
/// - `"es1"`, `"es3"`, `"es5"` — early ECMAScript editions
/// - `"es2015"` through `"es2025"` — annual ECMAScript releases
///
/// Returns `Err(String)` for any unrecognised version string.
fn grammar_path(version: &str) -> Result<PathBuf, String> {
    let root = grammar_root();

    match version {
        // Empty string → the generic, version-agnostic grammar.
        "" => Ok(root.join("javascript.grammar")),

        // Versioned ECMAScript grammars live in grammars/ecmascript/.
        "es1" | "es3" | "es5"
        | "es2015" | "es2016" | "es2017" | "es2018" | "es2019"
        | "es2020" | "es2021" | "es2022" | "es2023" | "es2024" | "es2025" => {
            Ok(root.join("ecmascript").join(format!("{version}.grammar")))
        }

        // Anything else is an error — we'd rather fail loudly than silently
        // fall back to the generic grammar and produce confusing results.
        other => Err(format!(
            "Unknown JavaScript/ECMAScript version '{other}'. \
             Valid values: \"\", \"es1\", \"es3\", \"es5\", \
             \"es2015\"–\"es2025\""
        )),
    }
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarParser` configured for JavaScript source code.
///
/// The `version` parameter selects which grammar file to load:
/// - `""` — uses the generic `javascript.grammar` (recommended for most
///   use cases where you don't need version-specific behaviour).
/// - `"es1"`, `"es3"`, `"es5"`, `"es2015"`–`"es2025"` — uses a
///   version-specific grammar for that ECMAScript edition.
///
/// This function performs two major steps:
///
/// 1. **Tokenization** — uses `tokenize_javascript` from the javascript-lexer
///    crate to break the source into tokens (also with the same `version`).
///
/// 2. **Grammar loading** — reads and parses the appropriate `.grammar` file,
///    which defines rules for programs, statements, expressions, and
///    function definitions.
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
/// use coding_adventures_javascript_parser::create_javascript_parser;
///
/// // Generic grammar:
/// let mut parser = create_javascript_parser("var x = 42;", "").unwrap();
/// let ast = parser.parse().expect("parse failed");
/// println!("{:?}", ast.rule_name);
///
/// // ES2015 grammar:
/// let mut parser_es6 = create_javascript_parser("let x = 42;", "es2015").unwrap();
/// ```
pub fn create_javascript_parser(source: &str, version: &str) -> Result<GrammarParser, String> {
    // Step 1: Tokenize the source using the javascript-lexer (same version).
    let tokens = tokenize_javascript(source, version)?;

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

/// Parse JavaScript source code into an AST.
///
/// This is the most convenient entry point — it handles tokenization,
/// grammar loading, parser creation, and parsing in one call.
///
/// The `version` parameter is the same as for [`create_javascript_parser`]:
/// pass `""` for the generic grammar or `"es2015"` etc. for a versioned one.
///
/// The returned `GrammarASTNode` has `rule_name` set to `"program"` (the
/// start symbol of the JavaScript grammar) with children corresponding
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
/// use coding_adventures_javascript_parser::parse_javascript;
///
/// // Generic grammar:
/// let ast = parse_javascript("var x = 1 + 2;", "").unwrap();
/// assert_eq!(ast.rule_name, "program");
///
/// // ES5 grammar:
/// let ast_es5 = parse_javascript("var x = 1;", "es5").unwrap();
/// ```
pub fn parse_javascript(source: &str, version: &str) -> Result<GrammarASTNode, String> {
    let mut js_parser = create_javascript_parser(source, version)?;

    js_parser
        .parse()
        .map_err(|e| format!("JavaScript parse failed: {e}"))
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
    // Test 1: Simple variable declaration (generic grammar)
    // -----------------------------------------------------------------------

    /// The simplest JavaScript program: a variable declaration.
    #[test]
    fn test_parse_var_declaration() {
        let ast = parse_javascript("var x = 1;", "").unwrap();
        assert_program_root(&ast);

        let stmt_count = count_statements(&ast);
        assert!(stmt_count >= 1, "Expected at least 1 statement, got {}", stmt_count);
    }

    // -----------------------------------------------------------------------
    // Test 2: Arithmetic expression
    // -----------------------------------------------------------------------

    /// An expression statement with binary arithmetic.
    #[test]
    fn test_parse_expression() {
        let ast = parse_javascript("1 + 2;", "").unwrap();
        assert_program_root(&ast);
        assert!(!ast.children.is_empty(), "AST should have children");
    }

    // -----------------------------------------------------------------------
    // Test 3: Multiple statements
    // -----------------------------------------------------------------------

    /// A program with multiple statements.
    #[test]
    fn test_parse_multiple_statements() {
        let source = "var x = 1; var y = 2; var z = x + y;";
        let ast = parse_javascript(source, "").unwrap();
        assert_program_root(&ast);

        let stmt_count = count_statements(&ast);
        assert_eq!(stmt_count, 3, "Expected 3 statements, got {}", stmt_count);
    }

    // -----------------------------------------------------------------------
    // Test 4: Empty program
    // -----------------------------------------------------------------------

    /// An empty program should parse to a program node with no children.
    #[test]
    fn test_parse_empty_program() {
        let ast = parse_javascript("", "").unwrap();
        assert_program_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 5: Factory function
    // -----------------------------------------------------------------------

    /// The `create_javascript_parser` factory function should return a
    /// working `GrammarParser`.
    #[test]
    fn test_create_parser() {
        let mut parser = create_javascript_parser("var x = 1;", "").unwrap();
        let result = parser.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());

        let ast = result.unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    // -----------------------------------------------------------------------
    // Test 6: Versioned grammar — es2015
    // -----------------------------------------------------------------------

    /// The es2015 versioned grammar should parse a basic var declaration.
    #[test]
    fn test_versioned_es2015() {
        let ast = parse_javascript("var x = 1;", "es2015").unwrap();
        assert_program_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 7: All versioned grammars parse an empty program
    // -----------------------------------------------------------------------

    /// Every versioned ECMAScript grammar should successfully parse an empty
    /// program (the simplest valid input).
    #[test]
    fn test_all_versioned_grammars() {
        let versions = [
            "es1", "es3", "es5",
            "es2015", "es2016", "es2017", "es2018", "es2019",
            "es2020", "es2021", "es2022", "es2023", "es2024", "es2025",
        ];
        for v in &versions {
            let result = parse_javascript("", v);
            assert!(result.is_ok(), "Version '{v}' should parse successfully: {:?}", result.err());
            assert_eq!(result.unwrap().rule_name, "program");
        }
    }

    // -----------------------------------------------------------------------
    // Test 8: Unknown version returns Err
    // -----------------------------------------------------------------------

    /// Passing an unrecognised version string should return Err, not panic.
    #[test]
    fn test_unknown_version_returns_err() {
        let result = parse_javascript("var x = 1;", "es99");
        assert!(result.is_err(), "Expected Err for unknown version 'es99'");

        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("es99"),
            "Error message should mention the bad version: {err_msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 9: create_javascript_parser with unknown version returns Err
    // -----------------------------------------------------------------------

    /// The factory function should also return Err for unknown versions.
    #[test]
    fn test_create_parser_unknown_version() {
        let result = create_javascript_parser("var x = 1;", "bad-version");
        assert!(result.is_err(), "Expected Err from create_javascript_parser with bad version");
    }
}
