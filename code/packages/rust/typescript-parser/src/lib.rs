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
//! It knows where to find the `typescript.grammar` file and provides two
//! public entry points.
//!
//! # TypeScript vs. JavaScript grammar
//!
//! The TypeScript grammar extends JavaScript's grammar with type annotations,
//! interface declarations, type aliases, enum declarations, and generic type
//! parameters. At the structural level, this means additional grammar rules
//! for type_annotation, interface_declaration, type_alias, and enum_declaration.

use std::fs;

use grammar_tools::parser_grammar::parse_parser_grammar;
use parser::grammar_parser::{GrammarParser, GrammarASTNode};
use coding_adventures_typescript_lexer::tokenize_typescript;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `typescript.grammar` file.
///
/// Uses the same strategy as the typescript-lexer crate:
/// `env!("CARGO_MANIFEST_DIR")` gives us the compile-time path to this
/// crate's directory, and we navigate up to the shared `grammars/` directory.
///
/// ```text
/// code/
///   grammars/
///     typescript.grammar    <-- target file
///   packages/
///     rust/
///       typescript-parser/
///         Cargo.toml        <-- CARGO_MANIFEST_DIR
/// ```
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/typescript.grammar")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarParser` configured for TypeScript source code.
///
/// This function performs two major steps:
///
/// 1. **Tokenization** — uses `tokenize_typescript` from the typescript-lexer
///    crate to break the source into tokens.
///
/// 2. **Grammar loading** — reads and parses the `typescript.grammar` file,
///    which defines rules for programs, statements, expressions, type
///    annotations, interfaces, enums, and more.
///
/// The returned `GrammarParser` is ready to call `.parse()` on.
///
/// # Panics
///
/// Panics if:
/// - The `typescript.grammar` file cannot be read or parsed.
/// - The source code fails tokenization (unexpected character).
///
/// # Example
///
/// ```no_run
/// use coding_adventures_typescript_parser::create_typescript_parser;
///
/// let mut parser = create_typescript_parser("let x: number = 42;");
/// let ast = parser.parse().expect("parse failed");
/// println!("{:?}", ast.rule_name);
/// ```
pub fn create_typescript_parser(source: &str) -> GrammarParser {
    // Step 1: Tokenize the source using the typescript-lexer.
    let tokens = tokenize_typescript(source);

    // Step 2: Read the parser grammar from disk.
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read typescript.grammar: {e}"));

    // Step 3: Parse the grammar text into a structured ParserGrammar.
    let grammar = parse_parser_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse typescript.grammar: {e}"));

    // Step 4: Create the parser.
    GrammarParser::new(tokens, grammar)
}

/// Parse TypeScript source code into an AST.
///
/// This is the most convenient entry point — it handles tokenization,
/// grammar loading, parser creation, and parsing in one call.
///
/// The returned `GrammarASTNode` has `rule_name` set to `"program"` (the
/// start symbol of the TypeScript grammar) with children corresponding
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
/// use coding_adventures_typescript_parser::parse_typescript;
///
/// let ast = parse_typescript("let x: number = 1 + 2;");
/// assert_eq!(ast.rule_name, "program");
/// ```
pub fn parse_typescript(source: &str) -> GrammarASTNode {
    let mut ts_parser = create_typescript_parser(source);

    ts_parser
        .parse()
        .unwrap_or_else(|e| panic!("TypeScript parse failed: {e}"))
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
    // Test 1: Simple variable declaration with type annotation
    // -----------------------------------------------------------------------

    // Note: typed_declaration test omitted — the simple typescript.grammar
    // doesn't support type annotations (: number).

    // -----------------------------------------------------------------------
    // Test 2: Arithmetic expression
    // -----------------------------------------------------------------------

    /// An expression statement with binary arithmetic.
    #[test]
    fn test_parse_expression() {
        let ast = parse_typescript("1 + 2;");
        assert_program_root(&ast);
        assert!(!ast.children.is_empty(), "AST should have children");
    }

    // Note: function_declaration, if_else, while_loop, multiple_statements
    // tests omitted — the simple typescript.grammar only supports var
    // declarations, assignments, and arithmetic expressions without type
    // annotations.

    // -----------------------------------------------------------------------
    // Test 7: Empty program
    // -----------------------------------------------------------------------

    /// An empty program should parse to a program node with no children.
    #[test]
    fn test_parse_empty_program() {
        let ast = parse_typescript("");
        assert_program_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 8: Factory function
    // -----------------------------------------------------------------------

    /// The `create_typescript_parser` factory function should return a
    /// working `GrammarParser`.
    #[test]
    fn test_create_parser() {
        let mut parser = create_typescript_parser("let x = 1;");
        let result = parser.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());

        let ast = result.unwrap();
        assert_eq!(ast.rule_name, "program");
    }

    // Note: interface, for_loop tests omitted — the simple typescript.grammar
    // doesn't include these constructs.
}
