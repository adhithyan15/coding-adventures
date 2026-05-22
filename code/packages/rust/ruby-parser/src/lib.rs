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

    // -----------------------------------------------------------------------
    // Phase 6c — `while` / `until` loops
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_while() {
        let ast = parse_ruby("while x\n  y = 1\nend");
        let node = find_statement_inner(&ast, "while_statement")
            .expect("expected while_statement");
        let body_count = node
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "statement"))
            .count();
        assert!(body_count >= 1);
    }

    #[test]
    fn test_parse_until() {
        let ast = parse_ruby("until x\n  y = 1\nend");
        assert!(find_statement_inner(&ast, "until_statement").is_some());
    }

    #[test]
    fn test_parse_while_empty_body() {
        // `while cond ; end` — zero-iteration body.  The grammar's
        // Repetition matches zero statements, then `end` closes.
        let ast = parse_ruby("while x\nend");
        assert!(find_statement_inner(&ast, "while_statement").is_some());
    }

    // -----------------------------------------------------------------------
    // Phase 6d — array and hash literals
    // -----------------------------------------------------------------------

    /// Walk the AST looking for the first node whose rule_name matches.
    fn find_descendant<'a>(
        node: &'a GrammarASTNode,
        rule: &str,
    ) -> Option<&'a GrammarASTNode> {
        for c in &node.children {
            if let ASTNodeOrToken::Node(n) = c {
                if n.rule_name == rule {
                    return Some(n);
                }
                if let Some(found) = find_descendant(n, rule) {
                    return Some(found);
                }
            }
        }
        None
    }

    #[test]
    fn test_parse_array_literal() {
        let ast = parse_ruby("x = [1, 2, 3]");
        let arr = find_descendant(&ast, "array_literal")
            .expect("expected array_literal");
        // Each element is an `expression` subnode.
        let elem_count = arr
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression"))
            .count();
        assert_eq!(elem_count, 3);
    }

    #[test]
    fn test_parse_empty_array_literal() {
        let ast = parse_ruby("x = []");
        assert!(find_descendant(&ast, "array_literal").is_some());
    }

    #[test]
    fn test_parse_hash_literal_shorthand() {
        let ast = parse_ruby("x = {a: 1, b: 2}");
        let h = find_descendant(&ast, "hash_literal").expect("expected hash_literal");
        let entry_count = h
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "hash_entry"))
            .count();
        assert_eq!(entry_count, 2);
    }

    #[test]
    fn test_parse_hash_literal_rocket() {
        let ast = parse_ruby("x = {a => 1}");
        let h = find_descendant(&ast, "hash_literal").expect("expected hash_literal");
        assert!(
            find_descendant(h, "hash_entry").is_some(),
            "expected hash_entry subnode"
        );
    }

    // -----------------------------------------------------------------------
    // Phase 6e — symbol literals `:foo`, `:"bar"`
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_symbol_name() {
        let ast = parse_ruby("x = :foo");
        let sym = find_descendant(&ast, "symbol_literal").expect("expected symbol_literal");
        // The grammar puts COLON then NAME under the rule.
        let name_tok = sym.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if matches!(t.type_, lexer::token::TokenType::Name) => {
                Some(t.value.as_str())
            }
            _ => None,
        });
        assert_eq!(name_tok, Some("foo"));
    }

    #[test]
    fn test_parse_symbol_keyword() {
        // `:def` — the symbol name happens to be a Ruby keyword.
        let ast = parse_ruby("x = :def");
        assert!(find_descendant(&ast, "symbol_literal").is_some());
    }

    #[test]
    fn test_parse_symbol_quoted() {
        let ast = parse_ruby(r#"x = :"hello world""#);
        let sym = find_descendant(&ast, "symbol_literal").expect("expected symbol_literal");
        let str_tok = sym.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t)
                if matches!(t.type_, lexer::token::TokenType::String) =>
            {
                Some(t.value.as_str())
            }
            _ => None,
        });
        assert_eq!(str_tok, Some("hello world"));
    }

    // -----------------------------------------------------------------------
    // Phase 6f — `class Foo … end` / `module Foo … end`
    // -----------------------------------------------------------------------
    //
    // Like `def_statement`, the body of a class/module is a repetition of
    // `statement` with a negative-lookahead `!"end"` so the closing keyword
    // doesn't get eaten as a bare `expression_stmt → factor → KEYWORD`.

    #[test]
    fn test_parse_empty_class() {
        let ast = parse_ruby("class Foo\nend");
        assert_program_root(&ast);
        let cls = find_statement_inner(&ast, "class_statement")
            .expect("expected class_statement");
        // Class name is the first Name token.
        let name_tok = cls.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if matches!(t.type_, lexer::token::TokenType::Name) => {
                Some(t.value.as_str())
            }
            _ => None,
        });
        assert_eq!(name_tok, Some("Foo"));
    }

    #[test]
    fn test_parse_class_with_method_body() {
        // A class with a nested `def` — the inner statement is itself a
        // `def_statement` under the class body.
        let ast = parse_ruby("class Foo\n  def bar\n  end\nend");
        let cls = find_statement_inner(&ast, "class_statement")
            .expect("expected class_statement");
        // The body has at least one `statement` child.
        let body_count = cls
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "statement"))
            .count();
        assert!(body_count >= 1, "expected at least one body statement");
    }

    #[test]
    fn test_parse_empty_module() {
        let ast = parse_ruby("module M\nend");
        assert!(find_statement_inner(&ast, "module_statement").is_some());
    }

    #[test]
    fn test_parse_module_with_assignment_body() {
        let ast = parse_ruby("module M\n  x = 1\nend");
        let m = find_statement_inner(&ast, "module_statement")
            .expect("expected module_statement");
        let body_count = m
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "statement"))
            .count();
        assert!(body_count >= 1);
    }
}

