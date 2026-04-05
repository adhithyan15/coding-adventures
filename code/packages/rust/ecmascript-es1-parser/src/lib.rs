//! # ECMAScript 1 (1997) Parser — parsing ES1 JavaScript into an AST.
//!
//! This crate is the second half of the ES1 front-end pipeline. Where the
//! `ecmascript-es1-lexer` crate breaks source text into tokens, this crate
//! arranges those tokens into a tree that reflects the **structure** of the
//! code — an Abstract Syntax Tree (AST).
//!
//! # The parsing pipeline
//!
//! ```text
//! Source code  ("var x = 1 + 2;")
//!       |
//!       v
//! ecmascript-es1-lexer  → Vec<Token>
//!       |
//!       v
//! es1.grammar           → ParserGrammar (rules like "program = ...")
//!       |
//!       v
//! GrammarParser          → GrammarASTNode tree
//! ```
//!
//! # What ES1 grammar supports
//!
//! - Variable declarations (`var`)
//! - Function declarations and expressions
//! - All 14 ES1 statement types (if, while, for, for-in, switch, etc.)
//! - Full expression precedence chain (comma through primary)
//! - Object and array literals
//!
//! # What ES1 grammar does NOT support
//!
//! - No `try`/`catch`/`finally`/`throw` (added in ES3)
//! - No `===`/`!==` in equality expressions (added in ES3)
//! - No `instanceof` in relational expressions (added in ES3)
//! - No `debugger` statement (added in ES5)
//! - No getter/setter properties (added in ES5)

use std::fs;

use grammar_tools::parser_grammar::parse_parser_grammar;
use parser::grammar_parser::{GrammarParser, GrammarASTNode};
use coding_adventures_ecmascript_es1_lexer::tokenize_es1;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `es1.grammar` file.
///
/// ```text
/// code/
///   grammars/
///     ecmascript/
///       es1.grammar           <-- target file
///   packages/
///     rust/
///       ecmascript-es1-parser/
///         Cargo.toml          <-- CARGO_MANIFEST_DIR
/// ```
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/ecmascript/es1.grammar")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarParser` configured for ECMAScript 1 source code.
///
/// This function performs two major steps:
///
/// 1. **Tokenization** — uses `tokenize_es1` from the ecmascript-es1-lexer
///    crate to break the source into tokens.
///
/// 2. **Grammar loading** — reads and parses the `es1.grammar` file.
///
/// The returned `GrammarParser` is ready to call `.parse()` on.
///
/// # Panics
///
/// Panics if the grammar file cannot be read/parsed, or if tokenization fails.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_ecmascript_es1_parser::create_es1_parser;
///
/// let mut parser = create_es1_parser("var x = 42;");
/// let ast = parser.parse().expect("parse failed");
/// ```
pub fn create_es1_parser(source: &str) -> GrammarParser {
    // Step 1: Tokenize the source using the ES1 lexer.
    let tokens = tokenize_es1(source);

    // Step 2: Read the parser grammar from disk.
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read es1.grammar: {e}"));

    // Step 3: Parse the grammar text into a structured ParserGrammar.
    let grammar = parse_parser_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse es1.grammar: {e}"));

    // Step 4: Create the parser.
    GrammarParser::new(tokens, grammar)
}

/// Parse ECMAScript 1 source code into an AST.
///
/// This is the most convenient entry point — it handles tokenization,
/// grammar loading, parser creation, and parsing in one call.
///
/// The returned `GrammarASTNode` has `rule_name` set to `"program"` (the
/// start symbol of the ES1 grammar).
///
/// # Panics
///
/// Panics if tokenization fails, the grammar file is missing/invalid,
/// or the source code has a syntax error.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_ecmascript_es1_parser::parse_es1;
///
/// let ast = parse_es1("var x = 1 + 2;");
/// assert_eq!(ast.rule_name, "program");
/// ```
pub fn parse_es1(source: &str) -> GrammarASTNode {
    let mut parser = create_es1_parser(source);

    parser
        .parse()
        .unwrap_or_else(|e| panic!("ES1 parse failed: {e}"))
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

    /// Count source_element children (each wraps a statement or function_declaration).
    /// The ES1 grammar uses: program = { source_element } ;
    fn count_source_elements(ast: &GrammarASTNode) -> usize {
        ast.children.iter().filter(|child| {
            matches!(child, ASTNodeOrToken::Node(n) if n.rule_name == "source_element")
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
    // Test 1: Simple variable declaration
    // -----------------------------------------------------------------------

    /// The simplest ES1 program: a variable declaration.
    #[test]
    fn test_parse_var_declaration() {
        let ast = parse_es1("var x = 1;");
        assert_program_root(&ast);

        let stmt_count = count_source_elements(&ast);
        assert!(stmt_count >= 1, "Expected at least 1 source element, got {}", stmt_count);
    }

    // -----------------------------------------------------------------------
    // Test 2: Arithmetic expression
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_expression() {
        let ast = parse_es1("1 + 2;");
        assert_program_root(&ast);
        assert!(!ast.children.is_empty(), "AST should have children");
    }

    // -----------------------------------------------------------------------
    // Test 3: Multiple statements
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_multiple_statements() {
        let source = "var x = 1; var y = 2; var z = x + y;";
        let ast = parse_es1(source);
        assert_program_root(&ast);

        let stmt_count = count_source_elements(&ast);
        assert_eq!(stmt_count, 3, "Expected 3 source elements, got {}", stmt_count);
    }

    // -----------------------------------------------------------------------
    // Test 4: Empty program
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_empty_program() {
        let ast = parse_es1("");
        assert_program_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 5: Factory function
    // -----------------------------------------------------------------------

    #[test]
    fn test_create_parser() {
        let mut parser = create_es1_parser("var x = 1;");
        let result = parser.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());

        let ast = result.unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    // -----------------------------------------------------------------------
    // Test 6: Function declaration
    // -----------------------------------------------------------------------

    /// ES1 supports function declarations with the `function` keyword.
    #[test]
    fn test_parse_function_declaration() {
        let ast = parse_es1("function add(a, b) { return a + b; }");
        assert_program_root(&ast);

        // Should find a function_declaration rule somewhere in the tree
        let has_func = ast.children.iter().any(|child| {
            if let ASTNodeOrToken::Node(n) = child {
                find_rule(n, "function_declaration")
            } else {
                false
            }
        });
        assert!(has_func, "Expected a function_declaration in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 7: If statement
    // -----------------------------------------------------------------------

    /// ES1 supports if/else statements.
    #[test]
    fn test_parse_if_statement() {
        let ast = parse_es1("if (x == 1) { var y = 2; }");
        assert_program_root(&ast);

        // The AST should contain an if_statement node
        let has_if = ast.children.iter().any(|child| {
            if let ASTNodeOrToken::Node(n) = child {
                find_rule(n, "if_statement")
            } else {
                false
            }
        });
        assert!(has_if, "Expected an if_statement in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 8: While loop
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_while_loop() {
        let ast = parse_es1("while (x != 0) { var y = 1; }");
        assert_program_root(&ast);

        let has_while = ast.children.iter().any(|child| {
            if let ASTNodeOrToken::Node(n) = child {
                find_rule(n, "while_statement")
            } else {
                false
            }
        });
        assert!(has_while, "Expected a while_statement in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 9: ES1 uses == not === in equality
    // -----------------------------------------------------------------------

    /// ES1 only has abstract equality (==, !=). A program using ==
    /// should parse correctly.
    #[test]
    fn test_parse_abstract_equality() {
        let ast = parse_es1("var result = a == b;");
        assert_program_root(&ast);
        assert!(!ast.children.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 10: Switch statement
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_switch() {
        let source = "switch (x) { case 1: var y = 1; break; default: var y = 0; }";
        let ast = parse_es1(source);
        assert_program_root(&ast);

        let has_switch = ast.children.iter().any(|child| {
            if let ASTNodeOrToken::Node(n) = child {
                find_rule(n, "switch_statement")
            } else {
                false
            }
        });
        assert!(has_switch, "Expected a switch_statement in the AST");
    }
}
