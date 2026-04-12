//! # Java Parser — parsing Java source code into an AST.
//!
//! This crate is the second half of the Java front-end pipeline. Where
//! the `java-lexer` crate breaks source text into tokens, this crate
//! arranges those tokens into a tree that reflects the **structure** of the
//! code — an Abstract Syntax Tree (AST).
//!
//! # The parsing pipeline
//!
//! Parsing Java requires four cooperating components:
//!
//! ```text
//! Source code  ("class Hello { }")
//!       |
//!       v
//! java-lexer           -> Vec<Token>
//!       |                [KEYWORD("class"), NAME("Hello"), LBRACE("{"),
//!       |                 RBRACE("}"), EOF]
//!       v
//! java{v}.grammar      -> ParserGrammar (rules like "program = ...")
//!       |
//!       v
//! GrammarParser        -> GrammarASTNode tree
//!       |
//!       |                program
//!       |                  └── statement
//!       |                        └── class_declaration
//!       |                              ├── KEYWORD("class")
//!       |                              ├── NAME("Hello")
//!       |                              ├── LBRACE("{")
//!       |                              └── RBRACE("}")
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
//! file to use. Java uses a numeric versioning scheme:
//!
//! | `version` | grammar file loaded |
//! |---|---|
//! | `"1.0"` | `grammars/java/java1.0.grammar` |
//! | `"1.1"` | `grammars/java/java1.1.grammar` |
//! | `"1.4"` | `grammars/java/java1.4.grammar` |
//! | `"5"` | `grammars/java/java5.grammar` |
//! | `"7"` | `grammars/java/java7.grammar` |
//! | `"8"` | `grammars/java/java8.grammar` |
//! | `"10"` | `grammars/java/java10.grammar` |
//! | `"14"` | `grammars/java/java14.grammar` |
//! | `"17"` | `grammars/java/java17.grammar` |
//! | `"21"` | `grammars/java/java21.grammar` |
//!
//! An unknown version string returns `Err(String)`.

use std::fs;
use std::path::PathBuf;

use coding_adventures_java_lexer::tokenize_java;
use grammar_tools::parser_grammar::parse_parser_grammar;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

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
///       java-parser/
///         Cargo.toml    <-- env!("CARGO_MANIFEST_DIR")
/// ```
fn grammar_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("grammars")
}

/// Validate the Java version string and return the path to the corresponding
/// `.grammar` file.
///
/// Valid version strings are:
/// - `"1.0"`, `"1.1"`, `"1.4"` — early JDK 1.x era
/// - `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"` — modern Java
///
/// Returns `Err(String)` for any unrecognised version string.
fn grammar_path(version: &str) -> Result<PathBuf, String> {
    let root = grammar_root();

    match version {
        // Early Java releases used the "1.x" naming convention.
        "1.0" | "1.1" | "1.4" => Ok(root.join("java").join(format!("java{version}.grammar"))),

        // Modern Java versions (post-J2SE renaming).
        "5" | "7" | "8" | "10" | "14" | "17" | "21" => {
            Ok(root.join("java").join(format!("java{version}.grammar")))
        }

        // Anything else is an error — we'd rather fail loudly than silently
        // fall back to a default grammar and produce confusing results.
        other => Err(format!(
            "Unknown Java version '{other}'. \
             Valid values: \"1.0\", \"1.1\", \"1.4\", \
             \"5\", \"7\", \"8\", \"10\", \"14\", \"17\", \"21\""
        )),
    }
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarParser` configured for Java source code.
///
/// The `version` parameter selects which grammar file to load:
/// - `"1.0"`, `"1.1"`, `"1.4"` — early JDK 1.x era grammars.
/// - `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"` — modern Java.
///
/// This function performs two major steps:
///
/// 1. **Tokenization** — uses `tokenize_java` from the java-lexer crate to
///    break the source into tokens (also with the same `version`).
///
/// 2. **Grammar loading** — reads and parses the appropriate `.grammar` file,
///    which defines rules for programs, statements, expressions, and class
///    definitions.
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
/// use coding_adventures_java_parser::create_java_parser;
///
/// // Java 21:
/// let mut parser = create_java_parser("class Hello { }", "21").unwrap();
/// let ast = parser.parse().expect("parse failed");
/// println!("{:?}", ast.rule_name);
///
/// // Java 8:
/// let mut parser_8 = create_java_parser("int x = 42;", "8").unwrap();
/// ```
pub fn create_java_parser(source: &str, version: &str) -> Result<GrammarParser, String> {
    // Step 1: Tokenize the source using the java-lexer (same version).
    let tokens = tokenize_java(source, version)?;

    // Step 2: Resolve the parser grammar file path.
    let path = grammar_path(version)?;

    // Step 3: Read the parser grammar from disk.
    let grammar_text =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

    // Step 4: Parse the grammar text into a structured ParserGrammar.
    let grammar = parse_parser_grammar(&grammar_text)
        .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;

    // Step 5: Create the parser.
    Ok(GrammarParser::new(tokens, grammar))
}

/// Parse Java source code into an AST.
///
/// This is the most convenient entry point — it handles tokenization,
/// grammar loading, parser creation, and parsing in one call.
///
/// The `version` parameter is the same as for [`create_java_parser`]:
/// pass a version like `"21"` for the latest LTS, or `"8"` for Java 8.
///
/// The returned `GrammarASTNode` has `rule_name` set to `"program"` (the
/// start symbol of the Java grammar) with children corresponding to the
/// top-level declarations and statements in the source.
///
/// # Errors
///
/// Returns `Err(String)` if the version is unknown, the grammar file is
/// missing or malformed, or the source has a syntax error.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_java_parser::parse_java;
///
/// // Java 21:
/// let ast = parse_java("class Hello { }", "21").unwrap();
/// assert_eq!(ast.rule_name, "program");
///
/// // Java 8:
/// let ast_8 = parse_java("int x = 1;", "8").unwrap();
/// ```
pub fn parse_java(source: &str, version: &str) -> Result<GrammarASTNode, String> {
    let mut java_parser = create_java_parser(source, version)?;

    java_parser
        .parse()
        .map_err(|e| format!("Java parse failed: {e}"))
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

    fn count_nodes(ast: &GrammarASTNode, target_rule: &str) -> usize {
        let mut count = usize::from(ast.rule_name == target_rule);
        for child in &ast.children {
            if let ASTNodeOrToken::Node(child_node) = child {
                count += count_nodes(child_node, target_rule);
            }
        }
        count
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
    // Test 1: Simple class declaration (Java 21 grammar)
    // -----------------------------------------------------------------------

    /// The simplest Java program: a class with an empty body.
    /// In Java, everything lives inside a class — even the simplest
    /// "Hello, World!" program needs a class wrapper.
    #[test]
    fn test_parse_class_declaration() {
        let ast = parse_java("class Hello { }", "21").unwrap();
        assert_program_root(&ast);

        assert!(
            find_rule(&ast, "class_declaration") || find_rule(&ast, "type_declaration"),
            "Expected a class/type declaration node"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: Arithmetic expression
    // -----------------------------------------------------------------------

    /// An expression statement with binary arithmetic.
    #[test]
    fn test_parse_expression() {
        let ast = parse_java("1 + 2;", "21").unwrap();
        assert_program_root(&ast);
        assert!(!ast.children.is_empty(), "AST should have children");
    }

    // -----------------------------------------------------------------------
    // Test 3: Multiple statements
    // -----------------------------------------------------------------------

    /// A program with multiple variable declarations.
    #[test]
    fn test_parse_multiple_statements() {
        let source = "int x = 1; int y = 2; int z = 3;";
        let ast = parse_java(source, "21").unwrap();
        assert_program_root(&ast);

        let decl_count = count_nodes(&ast, "var_declaration");
        assert_eq!(
            decl_count, 3,
            "Expected 3 variable declarations, got {}",
            decl_count
        );
    }

    // -----------------------------------------------------------------------
    // Test 4: Empty program
    // -----------------------------------------------------------------------

    /// An empty program should parse to a program node with no children.
    #[test]
    fn test_parse_empty_program() {
        let ast = parse_java("", "21").unwrap();
        assert_program_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 5: Factory function
    // -----------------------------------------------------------------------

    /// The `create_java_parser` factory function should return a working
    /// `GrammarParser`.
    #[test]
    fn test_create_parser() {
        let mut parser = create_java_parser("int x = 1;", "21").unwrap();
        let result = parser.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());

        let ast = result.unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    // -----------------------------------------------------------------------
    // Test 6: Versioned grammar — Java 8
    // -----------------------------------------------------------------------

    /// The Java 8 versioned grammar should parse a basic int declaration.
    #[test]
    fn test_versioned_java8() {
        let ast = parse_java("int x = 1;", "8").unwrap();
        assert_program_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 7: All versioned grammars parse an empty program
    // -----------------------------------------------------------------------

    /// Every versioned Java grammar should successfully parse an empty
    /// program (the simplest valid input).
    #[test]
    fn test_all_versioned_grammars() {
        let versions = ["1.0", "1.1", "1.4", "5", "7", "8", "10", "14", "17", "21"];
        for v in &versions {
            let result = parse_java("", v);
            assert!(
                result.is_ok(),
                "Version '{v}' should parse successfully: {:?}",
                result.err()
            );
            assert_eq!(result.unwrap().rule_name, "program");
        }
    }

    // -----------------------------------------------------------------------
    // Test 8: Unknown version returns Err
    // -----------------------------------------------------------------------

    /// Passing an unrecognised version string should return Err, not panic.
    #[test]
    fn test_unknown_version_returns_err() {
        let result = parse_java("int x = 1;", "99");
        assert!(result.is_err(), "Expected Err for unknown version '99'");

        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("99"),
            "Error message should mention the bad version: {err_msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 9: create_java_parser with unknown version returns Err
    // -----------------------------------------------------------------------

    /// The factory function should also return Err for unknown versions.
    #[test]
    fn test_create_parser_unknown_version() {
        let result = create_java_parser("int x = 1;", "bad-version");
        assert!(
            result.is_err(),
            "Expected Err from create_java_parser with bad version"
        );
    }
}
