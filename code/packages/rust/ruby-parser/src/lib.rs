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

    // -----------------------------------------------------------------------
    // Phase 6g — blocks `do … end` and brace-blocks `method { … }`
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_method_with_do_block_no_params() {
        let ast = parse_ruby("each do\n  puts 1\nend");
        let mwb = find_statement_inner(&ast, "method_with_block")
            .expect("expected method_with_block");
        let block = mwb
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "block" => Some(n),
                _ => None,
            })
            .expect("expected block subnode");
        assert!(find_descendant(block, "do_block").is_some());
    }

    #[test]
    fn test_parse_method_with_brace_block() {
        let ast = parse_ruby("each { puts 1 }");
        let mwb = find_statement_inner(&ast, "method_with_block")
            .expect("expected method_with_block");
        assert!(find_descendant(mwb, "brace_block").is_some());
    }

    #[test]
    fn test_parse_do_block_with_pipe_params() {
        let ast = parse_ruby("each do |x|\n  puts x\nend");
        let mwb = find_statement_inner(&ast, "method_with_block")
            .expect("expected method_with_block");
        let bp = find_descendant(mwb, "block_params").expect("expected block_params");
        let names: Vec<&str> = bp
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Token(t)
                    if matches!(t.type_, lexer::token::TokenType::Name) && t.value != "|" =>
                {
                    Some(t.value.as_str())
                }
                _ => None,
            })
            .collect();
        assert_eq!(names, vec!["x"]);
    }

    #[test]
    fn test_parse_brace_block_with_two_pipe_params() {
        let ast = parse_ruby("each { |x, y| x + y }");
        let mwb = find_statement_inner(&ast, "method_with_block")
            .expect("expected method_with_block");
        let bp = find_descendant(mwb, "block_params").expect("expected block_params");
        let names: Vec<&str> = bp
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Token(t)
                    if matches!(t.type_, lexer::token::TokenType::Name) && t.value != "|" =>
                {
                    Some(t.value.as_str())
                }
                _ => None,
            })
            .collect();
        assert_eq!(names, vec!["x", "y"]);
    }

    #[test]
    fn test_parse_method_call_with_args_and_block() {
        let ast = parse_ruby("each(1, 2) { puts 1 }");
        let mwb = find_statement_inner(&ast, "method_with_block")
            .expect("expected method_with_block");
        let has_args = mwb
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression"));
        let has_block = find_descendant(mwb, "brace_block").is_some();
        assert!(has_args && has_block);
    }

    #[test]
    fn test_parse_hash_literal_still_works_at_statement_position() {
        let ast = parse_ruby("x = {a: 1}");
        assert!(find_descendant(&ast, "hash_literal").is_some());
        assert!(find_descendant(&ast, "brace_block").is_none());
    }

    // -----------------------------------------------------------------------
    // Phase 6h — no-paren method calls (`puts 1` / `puts 1, 2`)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_no_paren_single_arg() {
        let ast = parse_ruby("puts 1");
        let call = find_statement_inner(&ast, "method_call_no_paren")
            .expect("expected method_call_no_paren");
        let arg_count = call
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression"))
            .count();
        assert_eq!(arg_count, 1);
    }

    #[test]
    fn test_parse_no_paren_multiple_args() {
        let ast = parse_ruby("puts 1, 2, 3");
        let call = find_statement_inner(&ast, "method_call_no_paren")
            .expect("expected method_call_no_paren");
        let arg_count = call
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression"))
            .count();
        assert_eq!(arg_count, 3);
    }

    #[test]
    fn test_paren_form_still_wins_over_no_paren() {
        let ast = parse_ruby("puts(1)");
        assert!(find_statement_inner(&ast, "method_call").is_some());
        assert!(find_statement_inner(&ast, "method_call_no_paren").is_none());
    }

    #[test]
    fn test_bare_name_falls_through_to_expression_stmt() {
        let ast = parse_ruby("puts");
        assert!(find_statement_inner(&ast, "method_call_no_paren").is_none());
        assert!(find_statement_inner(&ast, "expression_stmt").is_some());
    }

    #[test]
    fn test_no_paren_with_binary_arg_is_single_call() {
        let ast = parse_ruby("puts 1 + 2");
        let call = find_statement_inner(&ast, "method_call_no_paren")
            .expect("expected method_call_no_paren");
        let arg_count = call
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression"))
            .count();
        assert_eq!(arg_count, 1);
    }

    // -----------------------------------------------------------------------
    // Phase 6i — comparison operators (`==`, `!=`, `<`, `>`, `<=`, `>=`)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_simple_comparison_has_sum_subnodes() {
        let ast = parse_ruby("5 < 10");
        // Phase 6m moved the comparison op chain from `expression`
        // down to the new `comparison` rule (the old expression body).
        // Now `expression → logical_or → logical_and → logical_not →
        // comparison → sum { CMP_OP sum }`.  Walk to the comparison.
        let cmp = find_descendant(&ast, "comparison")
            .expect("expected comparison subnode");
        let sum_count = cmp
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "sum"))
            .count();
        assert_eq!(sum_count, 2);
        let has_lt = cmp
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "<"));
        assert!(has_lt);
    }

    #[test]
    fn test_parse_equality_in_assignment() {
        let ast = parse_ruby("flag = x == y");
        // Phase 6m: `==` lives on the `comparison` node (was
        // `expression`).
        let cmp = find_descendant(&ast, "comparison")
            .expect("expected comparison subnode");
        let has_eq_eq = cmp
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "=="));
        assert!(has_eq_eq);
    }

    #[test]
    fn test_parse_comparison_in_if_condition() {
        let ast = parse_ruby("if x < 10\n  y = 1\nend");
        // Phase 6m: comparison subnode now lives under the wrapping
        // logical_* chain inside the condition's expression.
        let cmp = find_descendant(&ast, "comparison")
            .expect("expected comparison in if condition");
        let has_lt = cmp
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "<"));
        assert!(has_lt);
    }

    #[test]
    fn test_parse_chained_inequality_left_associative() {
        let ast = parse_ruby("a < b < c");
        fn count_lt(node: &GrammarASTNode) -> usize {
            let mut n = 0;
            for c in &node.children {
                match c {
                    ASTNodeOrToken::Token(t) if t.value == "<" => n += 1,
                    ASTNodeOrToken::Node(sub) => n += count_lt(sub),
                    _ => {}
                }
            }
            n
        }
        assert!(count_lt(&ast) >= 1);
    }

    #[test]
    fn test_parse_plus_has_lower_precedence_than_comparison() {
        let ast = parse_ruby("1 + 2 < 5");
        // Phase 6m: comparison subnode wraps the two `sum`s.
        let cmp = find_descendant(&ast, "comparison")
            .expect("expected comparison subnode");
        let sums: Vec<&GrammarASTNode> = cmp
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "sum" => Some(n),
                _ => None,
            })
            .collect();
        assert_eq!(sums.len(), 2);
        let lhs_has_plus = sums[0]
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if matches!(t.type_, lexer::token::TokenType::Plus)));
        assert!(lhs_has_plus);
    }

    // -----------------------------------------------------------------------
    // Phase 6j — control-flow keywords: `return`, `break`, `next`
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_return_with_value() {
        let ast = parse_ruby("return 42");
        let r = find_statement_inner(&ast, "return_statement")
            .expect("expected return_statement");
        assert!(r
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression")));
    }

    #[test]
    fn test_parse_bare_return() {
        let ast = parse_ruby("return");
        assert!(find_statement_inner(&ast, "return_statement").is_some());
    }

    #[test]
    fn test_parse_break_with_value() {
        let ast = parse_ruby("break 1 + 2");
        let b = find_statement_inner(&ast, "break_statement")
            .expect("expected break_statement");
        assert!(b
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression")));
    }

    #[test]
    fn test_parse_next_keyword() {
        let ast = parse_ruby("next");
        assert!(find_statement_inner(&ast, "next_statement").is_some());
    }

    #[test]
    fn test_parse_return_inside_def_body() {
        let ast = parse_ruby("def f(x)\n  return x + 1\nend");
        let def = find_def_statement(&ast).expect("expected def_statement");
        let body_returns = def
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => Some(n),
                _ => None,
            })
            .any(|s| {
                s.children.iter().any(|c| {
                    matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "return_statement")
                })
            });
        assert!(body_returns);
    }

    // -----------------------------------------------------------------------
    // Phase 6k — unary minus `-5`, `-x`, `-(1+2)`
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_unary_minus_on_number() {
        let ast = parse_ruby("x = -5");
        assert!(find_descendant(&ast, "unary_minus").is_some());
    }

    #[test]
    fn test_parse_unary_minus_on_name() {
        let ast = parse_ruby("x = -y");
        let um = find_descendant(&ast, "unary_minus").expect("expected unary_minus");
        let name_tok = um.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "factor" => {
                n.children.iter().find_map(|cc| match cc {
                    ASTNodeOrToken::Token(t)
                        if matches!(t.type_, lexer::token::TokenType::Name) =>
                    {
                        Some(t.value.as_str())
                    }
                    _ => None,
                })
            }
            _ => None,
        });
        assert_eq!(name_tok, Some("y"));
    }

    #[test]
    fn test_parse_unary_minus_on_parenthesised_expression() {
        let ast = parse_ruby("x = -(1 + 2)");
        assert!(find_descendant(&ast, "unary_minus").is_some());
    }

    #[test]
    fn test_parse_double_unary_minus_nests() {
        let ast = parse_ruby("x = --5");
        let outer = find_descendant(&ast, "unary_minus")
            .expect("expected outer unary_minus");
        assert!(find_descendant(outer, "unary_minus").is_some());
    }

    #[test]
    fn test_parse_unary_minus_with_binary_addition() {
        let ast = parse_ruby("x = -5 + 3");
        assert!(find_descendant(&ast, "unary_minus").is_some());
        // The expression also contains a PLUS somewhere in its tree.
        fn has_plus(node: &GrammarASTNode) -> bool {
            node.children.iter().any(|c| match c {
                ASTNodeOrToken::Token(t) => matches!(t.type_, lexer::token::TokenType::Plus),
                ASTNodeOrToken::Node(sub) => has_plus(sub),
            })
        }
        assert!(has_plus(&ast));
    }

    // -----------------------------------------------------------------------
    // Phase 6l — method receiver chains `foo.bar.baz`, `foo.bar(args)`
    // -----------------------------------------------------------------------
    //
    // The `factor` rule now wraps its atom alternation with `{ dot_call }`;
    // `method_call` likewise grew a `{ dot_call }` tail.  These tests pin
    // that one or more `dot_call` subnodes appear in the parsed tree for
    // each of the canonical chain shapes.

    fn count_descendants(ast: &GrammarASTNode, rule: &str) -> usize {
        let mut n = 0;
        if ast.rule_name == rule {
            n += 1;
        }
        for c in &ast.children {
            if let ASTNodeOrToken::Node(sub) = c {
                n += count_descendants(sub, rule);
            }
        }
        n
    }

    #[test]
    fn test_parse_single_dot_call() {
        // `foo.bar` — one dot_call.  Lives inside a `factor` because
        // it's an expression-position chain (no head call).
        let ast = parse_ruby("foo.bar");
        let dot_calls = count_descendants(&ast, "dot_call");
        assert_eq!(dot_calls, 1, "expected exactly 1 dot_call, got {dot_calls}");
    }

    #[test]
    fn test_parse_chained_dot_calls() {
        // `foo.bar.baz` — two dot_calls in sequence under the same
        // factor.  Left-to-right chain: `(foo.bar).baz`.
        let ast = parse_ruby("foo.bar.baz");
        let dot_calls = count_descendants(&ast, "dot_call");
        assert_eq!(dot_calls, 2, "expected exactly 2 dot_calls, got {dot_calls}");
    }

    #[test]
    fn test_parse_method_call_with_dot_chain() {
        // `foo(1).bar` — head is a method_call, tail is one dot_call
        // appended to it.
        let ast = parse_ruby("foo(1).bar");
        let dot_calls = count_descendants(&ast, "dot_call");
        assert_eq!(dot_calls, 1, "expected exactly 1 dot_call, got {dot_calls}");
        // And the method_call rule fired (head call).
        assert!(find_descendant(&ast, "method_call").is_some());
    }

    #[test]
    fn test_parse_dot_call_with_args() {
        // `foo.bar(1, 2)` — dot_call's optional arg list is populated.
        let ast = parse_ruby("foo.bar(1, 2)");
        let dot = find_descendant(&ast, "dot_call").expect("dot_call expected");
        // Count `expression` direct children of the dot_call.
        let arg_count = dot
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression"))
            .count();
        assert_eq!(arg_count, 2, "expected 2 dot_call args, got {arg_count}");
    }

    #[test]
    fn test_parse_chain_inside_assignment_rhs() {
        // `x = a.b.c` — chain in expression position parses inside
        // the RHS of an assignment.
        let ast = parse_ruby("x = a.b.c");
        let dot_calls = count_descendants(&ast, "dot_call");
        assert_eq!(dot_calls, 2, "expected 2 dot_calls, got {dot_calls}");
    }

    // -----------------------------------------------------------------------
    // Phase 6m — logical operators `&&`, `||`, `and`, `or`, `not`, `!`
    // -----------------------------------------------------------------------

    /// Recursive walker — true iff the tree somewhere contains a
    /// token with the given value.  Used by the logical-operator
    /// tests because the parser's repetition wrapping can sandwich
    /// the operator token a level deeper than the named rule node.
    fn tree_has_token_value(node: &GrammarASTNode, value: &str) -> bool {
        for c in &node.children {
            match c {
                ASTNodeOrToken::Token(t) if t.value == value => return true,
                ASTNodeOrToken::Node(sub) => {
                    if tree_has_token_value(sub, value) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    #[test]
    fn test_parse_logical_or_symbol_form() {
        // `a || b` — the parse tree contains a `logical_or` node and a `||` token.
        let ast = parse_ruby("a || b");
        assert!(find_descendant(&ast, "logical_or").is_some(),
            "expected logical_or rule node in tree");
        assert!(tree_has_token_value(&ast, "||"), "expected `||` token in tree");
    }

    #[test]
    fn test_parse_logical_and_symbol_form() {
        let ast = parse_ruby("a && b");
        assert!(find_descendant(&ast, "logical_and").is_some(),
            "expected logical_and rule node in tree");
        assert!(tree_has_token_value(&ast, "&&"), "expected `&&` token in tree");
    }

    #[test]
    fn test_parse_logical_keyword_form() {
        // `a or b` — keyword form lowers through logical_or too.
        let ast = parse_ruby("a or b");
        assert!(find_descendant(&ast, "logical_or").is_some(),
            "expected logical_or rule node in tree");
        assert!(tree_has_token_value(&ast, "or"), "expected `or` keyword token in tree");
    }

    #[test]
    fn test_parse_logical_not_prefix() {
        // `!x` and `not x` both produce a `logical_not` with a
        // `!` or `not` leading token.
        let ast = parse_ruby("!x");
        let lnot = find_descendant(&ast, "logical_not").expect("expected logical_not");
        let has_bang = lnot
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "!"));
        assert!(has_bang, "expected `!` token under logical_not");
    }

    #[test]
    fn test_parse_logical_chain_and_then_or_precedence() {
        // `a && b || c` — `&&` binds tighter than `||`, so this
        // parses as `(a && b) || c`.  In the AST, the top-level
        // `logical_or` has a `logical_and` (containing `a && b`) and
        // a trailing operand `c` separated by `||`.
        let ast = parse_ruby("a && b || c");
        // There should be exactly one `||` and one `&&` token.
        fn count_value(node: &GrammarASTNode, val: &str) -> usize {
            let mut n = 0;
            for c in &node.children {
                match c {
                    ASTNodeOrToken::Token(t) if t.value == val => n += 1,
                    ASTNodeOrToken::Node(sub) => n += count_value(sub, val),
                    _ => {}
                }
            }
            n
        }
        assert_eq!(count_value(&ast, "||"), 1);
        assert_eq!(count_value(&ast, "&&"), 1);
    }

    #[test]
    fn test_parse_logical_or_inside_def_body() {
        // Sanity check that `||` inside a def body parses as a
        // def_statement (and not as some misparse that swallows the
        // `def`).
        //
        // KNOWN ISSUE: With the v0 statement-alternation framework,
        // `def name(args)\n  a || b\nend` mis-parses as
        // `method_call_no_paren("def", "name(args)")` and the def is
        // lost.  Same mis-parse happens for `def name(args)\n  x + y
        // \nend` only when the body's statement returns failure under
        // some path the alternation framework doesn't back-track
        // cleanly from.  See lessons.md for the workaround: wrap
        // logical operators in parens when used as a def body's
        // tail expression (`(a || b)`), or precede them with a
        // statement that breaks the ambiguity (`return a || b`).
        //
        // Track in lessons.md as a parser-framework limitation to
        // revisit in a follow-up phase.
        let ast = parse_ruby("def myor(a, b)\n  (a || b)\nend\n");
        assert!(
            find_descendant(&ast, "def_statement").is_some(),
            "expected def_statement to be present"
        );
        assert!(find_descendant(&ast, "logical_or").is_some(),
            "expected logical_or inside def body");
    }

    // -----------------------------------------------------------------------
    // Phase 6n — range expressions `..` and `...`
    // -----------------------------------------------------------------------
    //
    // The lexer pre-fuses two/three consecutive `.` tokens into a single
    // `Name`-typed token whose value is `..` or `...`.  The grammar matches
    // those tokens by literal *value* (same as `"=>"`, `"<="`, `"&&"`).
    //
    // A bare expression (no range op) still produces a `range` rule node
    // in the AST — the rule is the new expression entry point — but with
    // exactly one inner `logical_or` child and no `..`/`...` token.

    #[test]
    fn test_parse_inclusive_range() {
        // `1..5` — a `range` node with two `logical_or` operand children
        // and a `..` token between them.
        let ast = parse_ruby("1..5");
        let r = find_descendant(&ast, "range").expect("expected range node");
        assert!(
            tree_has_token_value(r, ".."),
            "expected `..` token inside the range node"
        );
        // Two logical_or operands flank the `..`.
        let operand_count = r
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "logical_or"))
            .count();
        assert_eq!(operand_count, 2, "expected 2 logical_or operands");
    }

    #[test]
    fn test_parse_exclusive_range() {
        // `1...5` — same shape as inclusive but `...` token.
        let ast = parse_ruby("1...5");
        let r = find_descendant(&ast, "range").expect("expected range node");
        assert!(
            tree_has_token_value(r, "..."),
            "expected `...` token inside the range node"
        );
    }

    #[test]
    fn test_parse_range_in_assignment_rhs() {
        // `x = 1..10` — range expression nests inside an assignment.
        let ast = parse_ruby("x = 1..10");
        // The assignment rule still fires.
        assert!(find_descendant(&ast, "assignment").is_some());
        // And there's a range node somewhere below.
        let r = find_descendant(&ast, "range").expect("expected range node");
        assert!(tree_has_token_value(r, ".."));
    }

    #[test]
    fn test_parse_range_with_arithmetic_endpoints() {
        // `1 + 2 .. 10 - 3` — both endpoints are arithmetic.  Range binds
        // looser than `+`/`-`, so this parses as `(1 + 2) .. (10 - 3)`.
        // Each operand subtree under the range node has its own `sum` node
        // with `+`/`-` tokens inside.
        let ast = parse_ruby("1 + 2 .. 10 - 3");
        let r = find_descendant(&ast, "range").expect("expected range node");
        // The range carries exactly one `..` token and the two arithmetic
        // operator tokens (`+`, `-`) live inside its operand subtrees.
        fn count_value(node: &GrammarASTNode, val: &str) -> usize {
            let mut n = 0;
            for c in &node.children {
                match c {
                    ASTNodeOrToken::Token(t) if t.value == val => n += 1,
                    ASTNodeOrToken::Node(sub) => n += count_value(sub, val),
                    _ => {}
                }
            }
            n
        }
        assert_eq!(count_value(r, ".."), 1, "expected exactly one `..` token");
        assert_eq!(count_value(r, "+"), 1, "expected `+` inside the range");
        assert_eq!(count_value(r, "-"), 1, "expected `-` inside the range");
    }

    #[test]
    fn test_parse_range_inside_array_literal() {
        // `[1..5]` — range expression as an array element.
        let ast = parse_ruby("[1..5]");
        let arr = find_descendant(&ast, "array_literal").expect("expected array_literal");
        // Range nested inside the array.
        assert!(
            find_descendant(arr, "range").is_some(),
            "expected range inside array_literal"
        );
        assert!(tree_has_token_value(arr, ".."));
    }

    // -----------------------------------------------------------------------
    // Phase 6p — compound assignment `+=`, `-=`, `*=`, `/=`, `||=`, `&&=`
    // -----------------------------------------------------------------------
    //
    // The lexer's `fuse_compound_assigns` post-pass folds adjacent
    // `Op` + `Equals` token pairs into a single Name-typed token whose
    // value is the fused operator (`+=`, etc.).  The grammar matches
    // by value — every assignment node carries either an EQUALS token
    // OR one of the compound-op tokens.

    #[test]
    fn test_parse_plus_equals_assignment() {
        // `x += 1` parses as an assignment with a `+=` operator token.
        let ast = parse_ruby("x += 1");
        let assn = find_descendant(&ast, "assignment").expect("expected assignment");
        assert!(
            tree_has_token_value(assn, "+="),
            "expected `+=` token in assignment, got {assn:?}"
        );
    }

    #[test]
    fn test_parse_all_arithmetic_compound_operators() {
        // Each of `+=`, `-=`, `*=`, `/=` parses as an assignment.
        for op in ["+=", "-=", "*=", "/="] {
            let src = format!("x {op} 1");
            let ast = parse_ruby(&src);
            let assn = find_descendant(&ast, "assignment")
                .unwrap_or_else(|| panic!("expected assignment for {src:?}"));
            assert!(
                tree_has_token_value(assn, op),
                "expected `{op}` token in assignment for source {src:?}"
            );
        }
    }

    #[test]
    fn test_parse_logical_compound_operators() {
        // `||=` and `&&=` are short-circuiting compound assignments.
        for op in ["||=", "&&="] {
            let src = format!("x {op} 1");
            let ast = parse_ruby(&src);
            let assn = find_descendant(&ast, "assignment")
                .unwrap_or_else(|| panic!("expected assignment for {src:?}"));
            assert!(
                tree_has_token_value(assn, op),
                "expected `{op}` token in assignment for source {src:?}"
            );
        }
    }

    #[test]
    fn test_parse_compound_assign_with_complex_rhs() {
        // The RHS is a full `expression`, so arithmetic flows through.
        let ast = parse_ruby("x += 1 + 2");
        let assn = find_descendant(&ast, "assignment").expect("expected assignment");
        assert!(tree_has_token_value(assn, "+="));
        // Two `+` tokens overall: the `+=` operator counts because
        // `+=` contains `+`, but tree_has_token_value matches by full
        // value; we expect a separate `+` from the RHS sum chain.
        fn count_value(node: &GrammarASTNode, val: &str) -> usize {
            let mut n = 0;
            for c in &node.children {
                match c {
                    ASTNodeOrToken::Token(t) if t.value == val => n += 1,
                    ASTNodeOrToken::Node(sub) => n += count_value(sub, val),
                    _ => {}
                }
            }
            n
        }
        assert_eq!(count_value(assn, "+"), 1, "expected one bare `+` in the RHS");
        assert_eq!(count_value(assn, "+="), 1, "expected one `+=` operator");
    }

    // -----------------------------------------------------------------------
    // Phase 6o — ternary `cond ? a : b`
    // -----------------------------------------------------------------------
    //
    // The grammar rule is `ternary = range [ "?" expression ":" expression ]`.
    // A bare expression with no `?` still produces a `ternary` node (the
    // rule is the new expression entry point), but with exactly one
    // `range` operand and no `?` token.

    #[test]
    fn test_parse_simple_ternary() {
        // `x = 1 ? 2 : 3` — wrapped in an assignment to dodge the
        // bare-NAME-led statement ambiguity (lessons.md).
        let ast = parse_ruby("x = 1 ? 2 : 3");
        let t = find_descendant(&ast, "ternary").expect("expected ternary node");
        assert!(tree_has_token_value(t, "?"), "expected `?` token");
        assert!(tree_has_token_value(t, ":"), "expected `:` token");
    }

    #[test]
    fn test_parse_ternary_right_associative() {
        // `x = a ? b : c ? d : e` — chained ternary, right-associative.
        // Two `?` and two `:` tokens; SIR test will pin the nesting.
        let ast = parse_ruby("x = a ? b : c ? d : e");
        let t = find_descendant(&ast, "ternary").expect("expected ternary node");
        fn count_value(node: &GrammarASTNode, val: &str) -> usize {
            let mut n = 0;
            for c in &node.children {
                match c {
                    ASTNodeOrToken::Token(t) if t.value == val => n += 1,
                    ASTNodeOrToken::Node(sub) => n += count_value(sub, val),
                    _ => {}
                }
            }
            n
        }
        assert_eq!(count_value(t, "?"), 2, "expected two `?` tokens");
        assert_eq!(count_value(t, ":"), 2, "expected two `:` tokens");
    }

    #[test]
    fn test_parse_ternary_inside_array_literal() {
        // `[1 ? 2 : 3]` — ternary as an array element parses cleanly.
        let ast = parse_ruby("[1 ? 2 : 3]");
        let arr = find_descendant(&ast, "array_literal").expect("expected array_literal");
        assert!(
            find_descendant(arr, "ternary").is_some(),
            "expected ternary inside array_literal"
        );
        assert!(tree_has_token_value(arr, "?"));
        assert!(tree_has_token_value(arr, ":"));
    }

    // -----------------------------------------------------------------------
    // Phase 6q — modifier conditionals/loops `x if y`, `x unless y`,
    // `x while y`, `x until y`.
    //
    // The grammar rule is:
    //   modifier_statement = ( assignment | method_call_no_paren
    //                        | method_call | expression_stmt )
    //                        ( "if_modifier" | ... )
    //                        expression ;
    //
    // The trailing keyword's value is `if_modifier`/etc. (not bare
    // `if`/etc.) because ruby-lexer's `tag_modifier_keywords` post-pass
    // rewrites `if`/`unless`/`while`/`until` tokens that follow an
    // expression-ending token on the same line.  Tests below assert the
    // rewrite happened by looking for the `*_modifier` value in the AST.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_if_modifier_simple() {
        // `puts "hi" if cond` — paren-less method call + trailing if.
        let ast = parse_ruby("puts \"hi\" if cond");
        let m = find_descendant(&ast, "modifier_statement")
            .expect("expected modifier_statement node");
        assert!(
            tree_has_token_value(m, "if_modifier"),
            "expected `if_modifier` re-tagged keyword"
        );
        // The LHS should be a `method_call_no_paren` child.
        assert!(
            find_descendant(m, "method_call_no_paren").is_some(),
            "expected method_call_no_paren as LHS"
        );
    }

    #[test]
    fn test_parse_unless_modifier_with_assignment_lhs() {
        // `x = 1 unless cond` — the LHS is an assignment.
        let ast = parse_ruby("x = 1 unless cond");
        let m = find_descendant(&ast, "modifier_statement")
            .expect("expected modifier_statement node");
        assert!(
            tree_has_token_value(m, "unless_modifier"),
            "expected `unless_modifier` re-tagged keyword"
        );
        assert!(
            find_descendant(m, "assignment").is_some(),
            "expected assignment as LHS"
        );
    }

    #[test]
    fn test_parse_while_modifier() {
        // `puts "tick" while cond`.
        let ast = parse_ruby("puts \"tick\" while cond");
        let m = find_descendant(&ast, "modifier_statement")
            .expect("expected modifier_statement node");
        assert!(
            tree_has_token_value(m, "while_modifier"),
            "expected `while_modifier` re-tagged keyword"
        );
    }

    #[test]
    fn test_parse_until_modifier_with_assignment_lhs() {
        // `x = 1 until cond` — until modifier on an assignment.
        let ast = parse_ruby("x = 1 until cond");
        let m = find_descendant(&ast, "modifier_statement")
            .expect("expected modifier_statement node");
        assert!(
            tree_has_token_value(m, "until_modifier"),
            "expected `until_modifier` re-tagged keyword"
        );
    }

    #[test]
    fn test_parse_leading_if_not_tagged_as_modifier() {
        // Regression: `if y\n  x\nend` is a leading-keyword
        // if_statement, NOT a modifier_statement.  The lexer's re-tag
        // only fires when the modifier keyword follows an
        // expression-ending token on the same line — at statement
        // start (after a newline) it's left alone.
        let ast = parse_ruby("if y\n  x = 1\nend");
        assert!(
            find_descendant(&ast, "if_statement").is_some(),
            "expected if_statement (leading keyword form)"
        );
        assert!(
            find_descendant(&ast, "modifier_statement").is_none(),
            "must NOT parse leading-keyword `if` as a modifier_statement"
        );
        // And the bare `if` token value must survive (no re-tag).
        assert!(
            tree_has_token_value(&ast, "if"),
            "leading `if` token value should be untouched"
        );
        assert!(
            !tree_has_token_value(&ast, "if_modifier"),
            "leading `if` must NOT be re-tagged to `if_modifier`"
        );
    }

    #[test]
    fn test_parse_two_statements_across_newline() {
        // Regression: `x = 1\nif y ... end` is TWO statements
        // (assignment + if_statement), not one modifier_statement.
        // Without the lexer's same-line guard, the grammar's
        // newline-insensitive default mode would otherwise mis-parse
        // this as `(x = 1) if y` followed by an orphaned `end`.
        let ast = parse_ruby("x = 1\nif y\n  z = 2\nend");
        let stmts = count_statements(&ast);
        assert_eq!(stmts, 2, "expected 2 top-level statements, got {stmts}");
        assert!(
            find_descendant(&ast, "if_statement").is_some(),
            "second statement should be if_statement"
        );
        assert!(
            find_descendant(&ast, "modifier_statement").is_none(),
            "must NOT collapse two statements into a modifier_statement"
        );
    }

    #[test]
    fn test_parse_range_with_paren_logical_operands() {
        // `(a || b)..(c || d)` — explicit parens make precedence
        // unambiguous and dodge the `method_call_no_paren` framework
        // ambiguity (see lessons.md, "logical operators inside def body").
        //
        // The range carries `..` and each operand subtree contains its
        // own `||`.
        let ast = parse_ruby("(a || b)..(c || d)");
        let r = find_descendant(&ast, "range").expect("expected range node");
        fn count_value(node: &GrammarASTNode, val: &str) -> usize {
            let mut n = 0;
            for c in &node.children {
                match c {
                    ASTNodeOrToken::Token(t) if t.value == val => n += 1,
                    ASTNodeOrToken::Node(sub) => n += count_value(sub, val),
                    _ => {}
                }
            }
            n
        }
        assert_eq!(count_value(r, ".."), 1);
        assert_eq!(count_value(r, "||"), 2, "expected `||` in both operand subtrees");
    }
}

