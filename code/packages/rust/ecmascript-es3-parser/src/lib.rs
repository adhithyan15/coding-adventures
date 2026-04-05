//! # ECMAScript 3 (1999) Parser — parsing ES3 JavaScript into an AST.
//!
//! This crate is the second half of the ES3 front-end pipeline. Where the
//! `ecmascript-es3-lexer` crate breaks source text into tokens, this crate
//! arranges those tokens into a tree that reflects the **structure** of the
//! code — an Abstract Syntax Tree (AST).
//!
//! # The parsing pipeline
//!
//! ```text
//! Source code  ("try { x(); } catch (e) { }")
//!       |
//!       v
//! ecmascript-es3-lexer  → Vec<Token>
//!       |
//!       v
//! es3.grammar           → ParserGrammar
//!       |
//!       v
//! GrammarParser          → GrammarASTNode tree
//! ```
//!
//! # What ES3 grammar adds over ES1
//!
//! - `try`/`catch`/`finally`/`throw` statements (structured error handling)
//! - `===` and `!==` in equality expressions (strict equality)
//! - `instanceof` in relational expressions
//! - `REGEX` as a primary expression

use std::fs;

use grammar_tools::parser_grammar::parse_parser_grammar;
use parser::grammar_parser::{GrammarParser, GrammarASTNode};
use coding_adventures_ecmascript_es3_lexer::tokenize_es3;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `es3.grammar` file.
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/ecmascript/es3.grammar")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarParser` configured for ECMAScript 3 source code.
///
/// # Panics
///
/// Panics if the grammar file cannot be read/parsed, or if tokenization fails.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_ecmascript_es3_parser::create_es3_parser;
///
/// let mut parser = create_es3_parser("try { x(); } catch (e) { }");
/// let ast = parser.parse().expect("parse failed");
/// ```
pub fn create_es3_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_es3(source);

    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read es3.grammar: {e}"));

    let grammar = parse_parser_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse es3.grammar: {e}"));

    GrammarParser::new(tokens, grammar)
}

/// Parse ECMAScript 3 source code into an AST.
///
/// The returned `GrammarASTNode` has `rule_name` set to `"program"`.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_ecmascript_es3_parser::parse_es3;
///
/// let ast = parse_es3("try { x(); } catch (e) { }");
/// assert_eq!(ast.rule_name, "program");
/// ```
pub fn parse_es3(source: &str) -> GrammarASTNode {
    let mut parser = create_es3_parser(source);

    parser
        .parse()
        .unwrap_or_else(|e| panic!("ES3 parse failed: {e}"))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    fn assert_program_root(ast: &GrammarASTNode) {
        assert_eq!(
            ast.rule_name, "program",
            "Expected root rule 'program', got '{}'",
            ast.rule_name
        );
    }

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
    // Test 1: Simple variable declaration (inherited from ES1)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_var_declaration() {
        let ast = parse_es3("var x = 1;");
        assert_program_root(&ast);

        let stmt_count = count_source_elements(&ast);
        assert!(stmt_count >= 1, "Expected at least 1 source element, got {}", stmt_count);
    }

    // -----------------------------------------------------------------------
    // Test 2: try/catch statement (NEW in ES3)
    // -----------------------------------------------------------------------

    /// The defining syntactic addition in ES3: structured error handling.
    /// Before ES3, the only way to handle errors was via the global `onerror`
    /// event handler.
    #[test]
    fn test_parse_try_catch() {
        let source = "try { var x = 1; } catch (e) { var y = 2; }";
        let ast = parse_es3(source);
        assert_program_root(&ast);

        let has_try = ast.children.iter().any(|child| {
            if let ASTNodeOrToken::Node(n) = child {
                find_rule(n, "try_statement")
            } else {
                false
            }
        });
        assert!(has_try, "Expected a try_statement in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 3: try/finally (without catch)
    // -----------------------------------------------------------------------

    /// ES3 allows try/finally without a catch clause.
    #[test]
    fn test_parse_try_finally() {
        let source = "try { var x = 1; } finally { var y = 2; }";
        let ast = parse_es3(source);
        assert_program_root(&ast);

        let has_try = ast.children.iter().any(|child| {
            if let ASTNodeOrToken::Node(n) = child {
                find_rule(n, "try_statement")
            } else {
                false
            }
        });
        assert!(has_try, "Expected a try_statement with finally in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 4: throw statement (NEW in ES3)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_throw() {
        let source = "throw x;";
        let ast = parse_es3(source);
        assert_program_root(&ast);

        let has_throw = ast.children.iter().any(|child| {
            if let ASTNodeOrToken::Node(n) = child {
                find_rule(n, "throw_statement")
            } else {
                false
            }
        });
        assert!(has_throw, "Expected a throw_statement in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 5: Strict equality in expressions (NEW in ES3)
    // -----------------------------------------------------------------------

    /// ES3 adds `===` and `!==` to equality expressions.
    #[test]
    fn test_parse_strict_equality() {
        let ast = parse_es3("var result = a === b;");
        assert_program_root(&ast);
        assert!(!ast.children.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 6: Multiple statements
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_multiple_statements() {
        let source = "var x = 1; var y = 2; var z = x + y;";
        let ast = parse_es3(source);
        assert_program_root(&ast);

        let stmt_count = count_source_elements(&ast);
        assert_eq!(stmt_count, 3, "Expected 3 source elements, got {}", stmt_count);
    }

    // -----------------------------------------------------------------------
    // Test 7: Empty program
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_empty_program() {
        let ast = parse_es3("");
        assert_program_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 8: Factory function
    // -----------------------------------------------------------------------

    #[test]
    fn test_create_parser() {
        let mut parser = create_es3_parser("var x = 1;");
        let result = parser.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());
        assert_eq!(result.unwrap().rule_name, "program");
    }

    // -----------------------------------------------------------------------
    // Test 9: Function declaration (inherited from ES1)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_function_declaration() {
        let ast = parse_es3("function add(a, b) { return a + b; }");
        assert_program_root(&ast);

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
    // Test 10: If statement (inherited from ES1)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_if_statement() {
        let ast = parse_es3("if (x === 1) { var y = 2; }");
        assert_program_root(&ast);

        let has_if = ast.children.iter().any(|child| {
            if let ASTNodeOrToken::Node(n) = child {
                find_rule(n, "if_statement")
            } else {
                false
            }
        });
        assert!(has_if, "Expected an if_statement in the AST");
    }
}
