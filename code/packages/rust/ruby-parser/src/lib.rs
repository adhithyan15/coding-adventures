//! Ruby parser backed by compiled parser grammar.

use coding_adventures_ruby_lexer::tokenize_ruby;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

pub fn create_ruby_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_ruby(source);
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar)
}

pub fn parse_ruby(source: &str) -> GrammarASTNode {
    let mut parser = create_ruby_parser(source);
    parser
        .parse()
        .unwrap_or_else(|e| panic!("Ruby parse failed: {e}"))
}

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

    // -----------------------------------------------------------------------
    // Phase 6a — method definitions
    // -----------------------------------------------------------------------

    /// Look up the first `def_statement` node in a parsed program.
    fn find_def_statement(ast: &GrammarASTNode) -> Option<&GrammarASTNode> {
        for child in &ast.children {
            if let ASTNodeOrToken::Node(n) = child {
                if n.rule_name == "statement" {
                    for inner in &n.children {
                        if let ASTNodeOrToken::Node(d) = inner {
                            if d.rule_name == "def_statement" {
                                return Some(d);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    #[test]
    fn test_parse_def_no_params_no_body() {
        let ast = parse_ruby("def foo()\nend");
        assert_program_root(&ast);
        let def = find_def_statement(&ast).expect("expected def_statement");
        let name_token = def.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if matches!(t.type_, lexer::token::TokenType::Name) => {
                Some(t.value.as_str())
            }
            _ => None,
        });
        assert_eq!(name_token, Some("foo"));
    }

    #[test]
    fn test_parse_def_with_params() {
        let ast = parse_ruby("def add(x, y)\nend");
        assert_program_root(&ast);
        let def = find_def_statement(&ast).expect("expected def_statement");
        let params_node = def
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "params" => Some(n),
                _ => None,
            })
            .expect("expected params subnode");
        let names: Vec<&str> = params_node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Token(t)
                    if matches!(t.type_, lexer::token::TokenType::Name) =>
                {
                    Some(t.value.as_str())
                }
                _ => None,
            })
            .collect();
        assert_eq!(names, vec!["x", "y"]);
    }

    #[test]
    fn test_parse_def_with_body() {
        let ast = parse_ruby("def add(x, y)\n  x + y\nend");
        assert_program_root(&ast);
        let def = find_def_statement(&ast).expect("expected def_statement");
        let body_stmts: usize = def
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "statement"))
            .count();
        assert!(body_stmts >= 1, "expected body statements, got {body_stmts}");
    }

    #[test]
    fn test_parse_def_without_parens() {
        // `def foo` without `()` is valid Ruby — the parens are
        // optional in the grammar.
        let ast = parse_ruby("def foo\nend");
        assert_program_root(&ast);
        assert!(find_def_statement(&ast).is_some());
    }

    // -----------------------------------------------------------------------
    // Phase 6b — `if … else … end` and `unless`
    // -----------------------------------------------------------------------

    fn find_statement_inner<'a>(
        ast: &'a GrammarASTNode,
        rule: &str,
    ) -> Option<&'a GrammarASTNode> {
        for child in &ast.children {
            if let ASTNodeOrToken::Node(n) = child {
                if n.rule_name == "statement" {
                    for inner in &n.children {
                        if let ASTNodeOrToken::Node(d) = inner {
                            if d.rule_name == rule {
                                return Some(d);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    #[test]
    fn test_parse_if_with_body() {
        let ast = parse_ruby("if x\n  y = 1\nend");
        assert_program_root(&ast);
        let node = find_statement_inner(&ast, "if_statement")
            .expect("expected if_statement");
        // The body has at least one `statement` subnode.
        let body_count = node
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "statement"))
            .count();
        assert!(body_count >= 1);
    }

    #[test]
    fn test_parse_if_else() {
        let ast = parse_ruby("if x\n  y = 1\nelse\n  y = 2\nend");
        let node = find_statement_inner(&ast, "if_statement")
            .expect("expected if_statement");
        let has_else = node
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "else_clause"));
        assert!(has_else, "expected else_clause subnode");
    }

    #[test]
    fn test_parse_if_elsif_else() {
        let ast = parse_ruby("if x\n  a = 1\nelsif y\n  a = 2\nelse\n  a = 3\nend");
        let node = find_statement_inner(&ast, "if_statement")
            .expect("expected if_statement");
        let elsif_count = node
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "elsif_clause"))
            .count();
        assert_eq!(elsif_count, 1);
        let has_else = node
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "else_clause"));
        assert!(has_else);
    }

    #[test]
    fn test_parse_unless() {
        let ast = parse_ruby("unless x\n  y = 1\nend");
        assert!(find_statement_inner(&ast, "unless_statement").is_some());
    }
}

