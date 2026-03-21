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

    /// A typed variable declaration is the quintessential TypeScript pattern.
    #[test]
    fn test_parse_typed_declaration() {
        let ast = parse_typescript("let x: number = 42;");
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
        let ast = parse_typescript("1 + 2;");
        assert_program_root(&ast);
        assert!(!ast.children.is_empty(), "AST should have children");
    }

    // -----------------------------------------------------------------------
    // Test 3: Function declaration
    // -----------------------------------------------------------------------

    /// A function declaration with typed parameters and return type.
    #[test]
    fn test_parse_function_declaration() {
        let source = "function add(a: number, b: number): number { return a + b; }";
        let ast = parse_typescript(source);
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
        let ast = parse_typescript(source);
        assert_program_root(&ast);

        let has_if = find_rule(&ast, "if_statement");
        assert!(has_if, "Expected to find an if_statement rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 5: While loop
    // -----------------------------------------------------------------------

    /// A while loop.
    #[test]
    fn test_parse_while_loop() {
        let source = "while (x) { x = x - 1; }";
        let ast = parse_typescript(source);
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
        let source = "let x: number = 1; let y: number = 2; let z: number = x + y;";
        let ast = parse_typescript(source);
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

    // -----------------------------------------------------------------------
    // Test 9: Interface declaration
    // -----------------------------------------------------------------------

    /// An interface declaration is TypeScript-specific.
    #[test]
    fn test_parse_interface() {
        let source = "interface Point { x: number; y: number; }";
        let ast = parse_typescript(source);
        assert_program_root(&ast);

        let has_interface = find_rule(&ast, "interface_declaration");
        assert!(has_interface, "Expected to find an interface_declaration rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 10: For loop
    // -----------------------------------------------------------------------

    /// A for loop with initialization, condition, and update.
    #[test]
    fn test_parse_for_loop() {
        let source = "for (let i: number = 0; i < 10; i = i + 1) { x = x + i; }";
        let ast = parse_typescript(source);
        assert_program_root(&ast);

        let has_for = find_rule(&ast, "for_statement");
        assert!(has_for, "Expected to find a for_statement rule in the AST");
    }
}
