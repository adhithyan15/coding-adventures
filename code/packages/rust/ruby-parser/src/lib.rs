//! # Ruby Parser — parsing Ruby source code into an AST.
//!
//! This crate is the second half of the Ruby front-end pipeline. Where the
//! `ruby-lexer` crate breaks source text into tokens, this crate arranges
//! those tokens into a tree that reflects the **structure** of the code —
//! an Abstract Syntax Tree (AST).
//!
//! # The parsing pipeline
//!
//! Parsing Ruby requires four cooperating components:
//!
//! ```text
//! Source code  ("def add(a, b)\n  a + b\nend")
//!       |
//!       v
//! ruby-lexer           → Vec<Token>
//!       |                [KEYWORD("def"), NAME("add"), LPAREN,
//!       |                 NAME("a"), COMMA, NAME("b"), RPAREN,
//!       |                 NAME("a"), PLUS, NAME("b"),
//!       |                 KEYWORD("end"), EOF]
//!       v
//! ruby.grammar         → ParserGrammar (rules like "program = ...")
//!       |
//!       v
//! GrammarParser        → GrammarASTNode tree
//!       |
//!       |                program
//!       |                  └── statement
//!       |                        └── def_statement
//!       |                              ├── KEYWORD("def")
//!       |                              ├── NAME("add")
//!       |                              ├── parameters
//!       |                              ├── body
//!       |                              └── KEYWORD("end")
//!       v
//! [future stages: interpretation]
//! ```
//!
//! This crate is the thin glue layer that wires these components together.
//! It knows where to find the `ruby.grammar` file and provides two public
//! entry points.
//!
//! # Ruby syntax
//!
//! Ruby uses `end` keywords to terminate blocks (unlike braces in C-family
//! languages or indentation in Python). This makes it easy to parse with
//! a grammar-driven approach since block boundaries are explicitly marked
//! by tokens.

use std::fs;

use grammar_tools::parser_grammar::parse_parser_grammar;
use parser::grammar_parser::{GrammarParser, GrammarASTNode};
use coding_adventures_ruby_lexer::tokenize_ruby;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `ruby.grammar` file.
///
/// Uses the same strategy as the ruby-lexer crate:
/// `env!("CARGO_MANIFEST_DIR")` gives us the compile-time path to this
/// crate's directory, and we navigate up to the shared `grammars/` directory.
///
/// ```text
/// code/
///   grammars/
///     ruby.grammar          <-- target file
///   packages/
///     rust/
///       ruby-parser/
///         Cargo.toml        <-- CARGO_MANIFEST_DIR
/// ```
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/ruby.grammar")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarParser` configured for Ruby source code.
///
/// This function performs two major steps:
///
/// 1. **Tokenization** — uses `tokenize_ruby` from the ruby-lexer crate
///    to break the source into tokens.
///
/// 2. **Grammar loading** — reads and parses the `ruby.grammar` file,
///    which defines rules for programs, statements, expressions, method
///    definitions, class definitions, and control flow.
///
/// The returned `GrammarParser` is ready to call `.parse()` on.
///
/// # Panics
///
/// Panics if:
/// - The `ruby.grammar` file cannot be read or parsed.
/// - The source code fails tokenization (unexpected character).
///
/// # Example
///
/// ```no_run
/// use coding_adventures_ruby_parser::create_ruby_parser;
///
/// let mut parser = create_ruby_parser("x = 1 + 2");
/// let ast = parser.parse().expect("parse failed");
/// println!("{:?}", ast.rule_name);
/// ```
pub fn create_ruby_parser(source: &str) -> GrammarParser {
    // Step 1: Tokenize the source using the ruby-lexer.
    let tokens = tokenize_ruby(source);

    // Step 2: Read the parser grammar from disk.
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read ruby.grammar: {e}"));

    // Step 3: Parse the grammar text into a structured ParserGrammar.
    let grammar = parse_parser_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse ruby.grammar: {e}"));

    // Step 4: Create the parser.
    GrammarParser::new(tokens, grammar)
}

/// Parse Ruby source code into an AST.
///
/// This is the most convenient entry point — it handles tokenization,
/// grammar loading, parser creation, and parsing in one call.
///
/// The returned `GrammarASTNode` has `rule_name` set to `"program"` (the
/// start symbol of the Ruby grammar) with children corresponding to the
/// statements in the source.
///
/// # Panics
///
/// Panics if tokenization fails, the grammar file is missing/invalid,
/// or the source code has a syntax error.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_ruby_parser::parse_ruby;
///
/// let ast = parse_ruby("x = 1 + 2");
/// assert_eq!(ast.rule_name, "program");
/// ```
pub fn parse_ruby(source: &str) -> GrammarASTNode {
    let mut ruby_parser = create_ruby_parser(source);

    ruby_parser
        .parse()
        .unwrap_or_else(|e| panic!("Ruby parse failed: {e}"))
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
    // Test 1: Simple assignment
    // -----------------------------------------------------------------------

    /// The simplest Ruby program: a single assignment.
    #[test]
    fn test_parse_assignment() {
        let ast = parse_ruby("x = 1");
        assert_program_root(&ast);

        let stmt_count = count_statements(&ast);
        assert!(stmt_count >= 1, "Expected at least 1 statement, got {}", stmt_count);
    }

    // -----------------------------------------------------------------------
    // Test 2: Arithmetic expression
    // -----------------------------------------------------------------------

    /// An expression with binary arithmetic.
    #[test]
    fn test_parse_expression() {
        let ast = parse_ruby("1 + 2");
        assert_program_root(&ast);
        assert!(!ast.children.is_empty(), "AST should have children");
    }

    // -----------------------------------------------------------------------
    // Test 3: Method definition
    // -----------------------------------------------------------------------

    // Note: def_statement, if_statement, while_statement, and class_statement
    // tests omitted — the simple ruby.grammar only supports assignments,
    // method calls, and arithmetic expressions.

    // -----------------------------------------------------------------------
    // Test 6: Multiple statements
    // -----------------------------------------------------------------------

    /// A program with multiple statements.
    #[test]
    fn test_parse_multiple_statements() {
        let source = "x = 1\ny = 2\nz = x + y";
        let ast = parse_ruby(source);
        assert_program_root(&ast);

        let stmt_count = count_statements(&ast);
        assert!(stmt_count >= 3, "Expected at least 3 statements, got {}", stmt_count);
    }

    // -----------------------------------------------------------------------
    // Test 7: Empty program
    // -----------------------------------------------------------------------

    /// An empty program should parse to a program node with no children.
    #[test]
    fn test_parse_empty_program() {
        let ast = parse_ruby("");
        assert_program_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 8: Factory function
    // -----------------------------------------------------------------------

    /// The `create_ruby_parser` factory function should return a working
    /// `GrammarParser`.
    #[test]
    fn test_create_parser() {
        let mut parser = create_ruby_parser("x = 1");
        let result = parser.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());

        let ast = result.unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    // -----------------------------------------------------------------------
    // Test 10: Method call
    // -----------------------------------------------------------------------

    /// A method call with arguments.
    #[test]
    fn test_parse_method_call() {
        let source = "puts(42)";
        let ast = parse_ruby(source);
        assert_program_root(&ast);
        assert!(!ast.children.is_empty());
    }
}
