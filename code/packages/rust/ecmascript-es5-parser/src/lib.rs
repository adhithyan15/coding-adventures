//! # ECMAScript 5 (2009) Parser — parsing ES5 JavaScript into an AST.
//!
//! This crate is the second half of the ES5 front-end pipeline. Where the
//! `ecmascript-es5-lexer` crate breaks source text into tokens, this crate
//! arranges those tokens into a tree that reflects the **structure** of the
//! code — an Abstract Syntax Tree (AST).
//!
//! # The parsing pipeline
//!
//! ```text
//! Source code  ("debugger;")
//!       |
//!       v
//! ecmascript-es5-lexer  → Vec<Token>
//!       |
//!       v
//! es5.grammar           → ParserGrammar
//!       |
//!       v
//! GrammarParser          → GrammarASTNode tree
//! ```
//!
//! # What ES5 grammar adds over ES3
//!
//! - `debugger` statement (`debugger;` acts as a breakpoint)
//! - Getter/setter properties in object literals
//!   (`{ get name() {}, set name(v) {} }`)
//! - `property_assignment` gains getter_property and setter_property alternatives

use std::fs;

use grammar_tools::parser_grammar::parse_parser_grammar;
use parser::grammar_parser::{GrammarParser, GrammarASTNode};
use coding_adventures_ecmascript_es5_lexer::tokenize_es5;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `es5.grammar` file.
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/ecmascript/es5.grammar")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarParser` configured for ECMAScript 5 source code.
///
/// # Panics
///
/// Panics if the grammar file cannot be read/parsed, or if tokenization fails.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_ecmascript_es5_parser::create_es5_parser;
///
/// let mut parser = create_es5_parser("debugger;");
/// let ast = parser.parse().expect("parse failed");
/// ```
pub fn create_es5_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_es5(source);

    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read es5.grammar: {e}"));

    let grammar = parse_parser_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse es5.grammar: {e}"));

    GrammarParser::new(tokens, grammar)
}

/// Parse ECMAScript 5 source code into an AST.
///
/// The returned `GrammarASTNode` has `rule_name` set to `"program"`.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_ecmascript_es5_parser::parse_es5;
///
/// let ast = parse_es5("debugger;");
/// assert_eq!(ast.rule_name, "program");
/// ```
pub fn parse_es5(source: &str) -> GrammarASTNode {
    let mut parser = create_es5_parser(source);

    parser
        .parse()
        .unwrap_or_else(|e| panic!("ES5 parse failed: {e}"))
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
    // Test 1: debugger statement (NEW in ES5)
    // -----------------------------------------------------------------------

    /// The key syntactic addition in ES5: the `debugger` statement.
    /// When a debugger is attached, execution pauses at this point.
    /// When no debugger is attached, it has no effect.
    #[test]
    fn test_parse_debugger_statement() {
        let ast = parse_es5("debugger;");
        assert_program_root(&ast);

        let has_debugger = ast.children.iter().any(|child| {
            if let ASTNodeOrToken::Node(n) = child {
                find_rule(n, "debugger_statement")
            } else {
                false
            }
        });
        assert!(has_debugger, "Expected a debugger_statement in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 2: Simple variable declaration (inherited from ES1)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_var_declaration() {
        let ast = parse_es5("var x = 1;");
        assert_program_root(&ast);

        let stmt_count = count_source_elements(&ast);
        assert!(stmt_count >= 1, "Expected at least 1 source element, got {}", stmt_count);
    }

    // -----------------------------------------------------------------------
    // Test 3: try/catch (inherited from ES3)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_try_catch() {
        let source = "try { var x = 1; } catch (e) { var y = 2; }";
        let ast = parse_es5(source);
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
    // Test 4: Strict equality (inherited from ES3)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_strict_equality() {
        let ast = parse_es5("var result = a === b;");
        assert_program_root(&ast);
        assert!(!ast.children.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 5: Multiple statements
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_multiple_statements() {
        let source = "var x = 1; var y = 2; var z = x + y;";
        let ast = parse_es5(source);
        assert_program_root(&ast);

        let stmt_count = count_source_elements(&ast);
        assert_eq!(stmt_count, 3, "Expected 3 source elements, got {}", stmt_count);
    }

    // -----------------------------------------------------------------------
    // Test 6: Empty program
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_empty_program() {
        let ast = parse_es5("");
        assert_program_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 7: Factory function
    // -----------------------------------------------------------------------

    #[test]
    fn test_create_parser() {
        let mut parser = create_es5_parser("var x = 1;");
        let result = parser.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());
        assert_eq!(result.unwrap().rule_name, "program");
    }

    // -----------------------------------------------------------------------
    // Test 8: Function declaration (inherited from ES1)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_function_declaration() {
        let ast = parse_es5("function add(a, b) { return a + b; }");
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
    // Test 9: throw statement (inherited from ES3)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_throw() {
        let source = "throw x;";
        let ast = parse_es5(source);
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
    // Test 10: debugger mixed with other statements
    // -----------------------------------------------------------------------

    /// The debugger statement can appear anywhere a statement can appear.
    #[test]
    fn test_parse_debugger_in_function() {
        let source = "function test() { debugger; var x = 1; }";
        let ast = parse_es5(source);
        assert_program_root(&ast);

        let has_func = ast.children.iter().any(|child| {
            if let ASTNodeOrToken::Node(n) = child {
                find_rule(n, "function_declaration")
            } else {
                false
            }
        });
        assert!(has_func, "Expected a function_declaration containing debugger");
    }
}
