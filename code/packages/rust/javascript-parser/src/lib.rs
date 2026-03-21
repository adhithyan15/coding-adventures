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
//! It knows where to find the `javascript.grammar` file and provides two
//! public entry points.

use std::fs;

use grammar_tools::parser_grammar::parse_parser_grammar;
use parser::grammar_parser::{GrammarParser, GrammarASTNode};
use coding_adventures_javascript_lexer::tokenize_javascript;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `javascript.grammar` file.
///
/// Uses the same strategy as the javascript-lexer crate:
/// `env!("CARGO_MANIFEST_DIR")` gives us the compile-time path to this
/// crate's directory, and we navigate up to the shared `grammars/` directory.
///
/// ```text
/// code/
///   grammars/
///     javascript.grammar    <-- target file
///   packages/
///     rust/
///       javascript-parser/
///         Cargo.toml        <-- CARGO_MANIFEST_DIR
/// ```
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/javascript.grammar")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarParser` configured for JavaScript source code.
///
/// This function performs two major steps:
///
/// 1. **Tokenization** — uses `tokenize_javascript` from the javascript-lexer
///    crate to break the source into tokens.
///
/// 2. **Grammar loading** — reads and parses the `javascript.grammar` file,
///    which defines rules for programs, statements, expressions, and
///    function definitions.
///
/// The returned `GrammarParser` is ready to call `.parse()` on.
///
/// # Panics
///
/// Panics if:
/// - The `javascript.grammar` file cannot be read or parsed.
/// - The source code fails tokenization (unexpected character).
///
/// # Example
///
/// ```no_run
/// use coding_adventures_javascript_parser::create_javascript_parser;
///
/// let mut parser = create_javascript_parser("var x = 42;");
/// let ast = parser.parse().expect("parse failed");
/// println!("{:?}", ast.rule_name);
/// ```
pub fn create_javascript_parser(source: &str) -> GrammarParser {
    // Step 1: Tokenize the source using the javascript-lexer.
    let tokens = tokenize_javascript(source);

    // Step 2: Read the parser grammar from disk.
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read javascript.grammar: {e}"));

    // Step 3: Parse the grammar text into a structured ParserGrammar.
    let grammar = parse_parser_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse javascript.grammar: {e}"));

    // Step 4: Create the parser.
    GrammarParser::new(tokens, grammar)
}

/// Parse JavaScript source code into an AST.
///
/// This is the most convenient entry point — it handles tokenization,
/// grammar loading, parser creation, and parsing in one call.
///
/// The returned `GrammarASTNode` has `rule_name` set to `"program"` (the
/// start symbol of the JavaScript grammar) with children corresponding
/// to the statements in the source.
///
/// # Panics
///
/// Panics if tokenization fails, the grammar file is missing/invalid,
/// or the source code has a syntax error.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_javascript_parser::parse_javascript;
///
/// let ast = parse_javascript("var x = 1 + 2;");
/// assert_eq!(ast.rule_name, "program");
/// ```
pub fn parse_javascript(source: &str) -> GrammarASTNode {
    let mut js_parser = create_javascript_parser(source);

    js_parser
        .parse()
        .unwrap_or_else(|e| panic!("JavaScript parse failed: {e}"))
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
    // Test 1: Simple variable declaration
    // -----------------------------------------------------------------------

    /// The simplest JavaScript program: a variable declaration.
    #[test]
    fn test_parse_var_declaration() {
        let ast = parse_javascript("var x = 1;");
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
        let ast = parse_javascript("1 + 2;");
        assert_program_root(&ast);
        assert!(!ast.children.is_empty(), "AST should have children");
    }

    // -----------------------------------------------------------------------
    // Test 3: Function declaration
    // -----------------------------------------------------------------------

    /// A function declaration with parameters and a return statement.
    #[test]
    fn test_parse_function_declaration() {
        let source = "function add(a, b) { return a + b; }";
        let ast = parse_javascript(source);
        assert_program_root(&ast);

        let has_func = find_rule(&ast, "function_declaration");
        assert!(has_func, "Expected to find a function_declaration rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 4: If/else
    // -----------------------------------------------------------------------

    /// An if/else statement tests conditional branching.
    #[test]
    fn test_parse_if_else() {
        let source = "if (x) { y = 1; } else { y = 2; }";
        let ast = parse_javascript(source);
        assert_program_root(&ast);

        let has_if = find_rule(&ast, "if_statement");
        assert!(has_if, "Expected to find an if_statement rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 5: While loop
    // -----------------------------------------------------------------------

    /// A while loop tests iteration.
    #[test]
    fn test_parse_while_loop() {
        let source = "while (x) { x = x - 1; }";
        let ast = parse_javascript(source);
        assert_program_root(&ast);

        let has_while = find_rule(&ast, "while_statement");
        assert!(has_while, "Expected to find a while_statement rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 6: Multiple statements
    // -----------------------------------------------------------------------

    /// A program with multiple statements.
    #[test]
    fn test_parse_multiple_statements() {
        let source = "var x = 1; var y = 2; var z = x + y;";
        let ast = parse_javascript(source);
        assert_program_root(&ast);

        let stmt_count = count_statements(&ast);
        assert_eq!(stmt_count, 3, "Expected 3 statements, got {}", stmt_count);
    }

    // -----------------------------------------------------------------------
    // Test 7: Empty program
    // -----------------------------------------------------------------------

    /// An empty program should parse to a program node with no children.
    #[test]
    fn test_parse_empty_program() {
        let ast = parse_javascript("");
        assert_program_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 8: Factory function
    // -----------------------------------------------------------------------

    /// The `create_javascript_parser` factory function should return a
    /// working `GrammarParser`.
    #[test]
    fn test_create_parser() {
        let mut parser = create_javascript_parser("var x = 1;");
        let result = parser.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());

        let ast = result.unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    // -----------------------------------------------------------------------
    // Test 9: For loop
    // -----------------------------------------------------------------------

    /// A for loop with initialization, condition, and update.
    #[test]
    fn test_parse_for_loop() {
        let source = "for (var i = 0; i < 10; i = i + 1) { x = x + i; }";
        let ast = parse_javascript(source);
        assert_program_root(&ast);

        let has_for = find_rule(&ast, "for_statement");
        assert!(has_for, "Expected to find a for_statement rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 10: Function call
    // -----------------------------------------------------------------------

    /// A function call with arguments.
    #[test]
    fn test_parse_function_call() {
        let source = "console.log(42);";
        let ast = parse_javascript(source);
        assert_program_root(&ast);
        assert!(!ast.children.is_empty());
    }
}
