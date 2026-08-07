//! Starlark parser backed by compiled parser grammar.

use coding_adventures_starlark_lexer::tokenize_starlark;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Recursion-depth cap for the Starlark [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] for why this guard exists at all (deep
/// recursion through `parse_rule` can overflow the *native* thread stack —
/// an uncatchable process abort — before this crate's own callers get a
/// chance to report anything). Before this constant was applied,
/// `create_starlark_parser` never called `with_max_depth` at all, leaving
/// every caller exposed to a native-stack-overflow DoS from adversarial
/// deeply-nested input (e.g. `x = (((...1...)))`).
///
/// **Not the shared engine's bare default** (see `csharp-parser`'s own
/// identically-named constant for why a blind
/// [`DEFAULT_MAX_RULE_DEPTH`](parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH)
/// (128) is unsafe-for-usability on a rich general-purpose grammar).
/// Measured directly instead (binary search over candidate
/// `with_max_depth` values against a fixed 5000-level adversarial
/// `x = (((...1...)))` input — ordinary parenthesised grouping, via
/// `paren_expr = LPAREN [ paren_body ] RPAREN` — on a default-~2MiB-stack
/// worker thread in a debug build, no `RUST_MIN_STACK` override or
/// explicit `Builder::stack_size` present): safe at **272**, crashes at
/// **273**.
///
/// `MAX_RULE_DEPTH` is set to **190** — about 30% below that floor
/// (comparable margin to `python-parser`'s own ~30%, `sql-parser`'s ~30%,
/// `verilog-parser`'s/`vhdl-parser`'s ~31%). Measured real-input headroom
/// at `190`: plain parenthesised nesting parses cleanly to at least 9
/// levels, matching ordinary hand-written nesting depth.
///
/// Starlark's expression grammar has roughly a dozen distinct
/// self-referential recursion shapes beyond paren-grouping (chained
/// ternaries via `test`, nested `lambda_expr`, chained `not`/unary
/// operators, `power` exponent chains, nested `list_expr`/`dict_expr`
/// literals, nested call/subscript suffixes in `primary`, and nested
/// `if`/`for`/`def` block bodies via `suite`) — this pass measures only
/// **one** of them (paren-grouping), the way `ruby-parser`'s own
/// `MAX_RULE_DEPTH` documents a single-shape measurement across its much
/// larger shape inventory. A full multi-shape audit (the `css-parser`/
/// `toml-parser` treatment) is a tracked follow-up.
const MAX_RULE_DEPTH: usize = 190;

pub fn create_starlark_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_starlark(source);
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH)
}

pub fn parse_starlark(source: &str) -> GrammarASTNode {
    let mut parser = create_starlark_parser(source);
    parser
        .parse()
        .unwrap_or_else(|e| panic!("Starlark parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    // -----------------------------------------------------------------------
    // Helper: check that the root node has the expected rule name.
    // -----------------------------------------------------------------------

    /// All Starlark programs parse to a root node with rule_name "file",
    /// since that is the start symbol of the grammar.
    fn assert_file_root(ast: &GrammarASTNode) {
        assert_eq!(
            ast.rule_name, "file",
            "Expected root rule 'file', got '{}'",
            ast.rule_name
        );
    }

    /// Count how many statement nodes are direct or indirect children of
    /// the given AST. This is a rough measure of how many top-level
    /// statements the parser found.
    fn count_statements(ast: &GrammarASTNode) -> usize {
        ast.children.iter().filter(|child| {
            matches!(child, ASTNodeOrToken::Node(n) if n.rule_name == "statement")
        }).count()
    }

    // -----------------------------------------------------------------------
    // Test 1: Simple assignment
    // -----------------------------------------------------------------------

    /// The simplest Starlark program: a single assignment statement.
    /// This exercises the full pipeline: lexer -> parser -> AST.
    #[test]
    fn test_parse_simple_assignment() {
        let ast = parse_starlark("x = 1\n");
        assert_file_root(&ast);

        // The file should contain at least one statement.
        let stmt_count = count_statements(&ast);
        assert!(stmt_count >= 1, "Expected at least 1 statement, got {}", stmt_count);
    }

    // -----------------------------------------------------------------------
    // Test 2: Arithmetic expression
    // -----------------------------------------------------------------------

    /// An expression statement with binary arithmetic. This tests that
    /// the parser correctly handles operator precedence through the
    /// expression -> or_expr -> ... -> arith -> term -> factor -> power
    /// chain of grammar rules.
    #[test]
    fn test_parse_expression() {
        let ast = parse_starlark("1 + 2\n");
        assert_file_root(&ast);

        // Should parse without error. The expression `1 + 2` becomes an
        // assign_stmt (the grammar's catch-all for expression statements)
        // inside a simple_stmt inside a statement.
        assert!(!ast.children.is_empty(), "AST should have children");
    }

    // -----------------------------------------------------------------------
    // Test 3: Function definition
    // -----------------------------------------------------------------------

    /// A function definition with parameters and a return statement.
    /// This exercises compound_stmt -> def_stmt, suite with INDENT/DEDENT,
    /// and the parameters rule.
    #[test]
    fn test_parse_function_def() {
        let source = "def add(x, y):\n    return x + y\n";
        let ast = parse_starlark(source);
        assert_file_root(&ast);

        // The file should contain exactly one statement (the def).
        let stmt_count = count_statements(&ast);
        assert_eq!(stmt_count, 1, "Expected 1 statement (def), got {}", stmt_count);

        // Verify that somewhere in the tree there is a "def_stmt" node.
        let has_def = find_rule(&ast, "def_stmt");
        assert!(has_def, "Expected to find a def_stmt rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 4: If/else
    // -----------------------------------------------------------------------

    /// An if/else statement with indented blocks. This tests:
    /// - compound_stmt -> if_stmt rule
    /// - The "else" clause
    /// - Multiple suite blocks with INDENT/DEDENT
    #[test]
    fn test_parse_if_else() {
        let source = "if x:\n    y = 1\nelse:\n    y = 2\n";
        let ast = parse_starlark(source);
        assert_file_root(&ast);

        // Should find an if_stmt node in the tree.
        let has_if = find_rule(&ast, "if_stmt");
        assert!(has_if, "Expected to find an if_stmt rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 5: For loop
    // -----------------------------------------------------------------------

    /// A for loop iterating over a collection. This tests:
    /// - compound_stmt -> for_stmt rule
    /// - loop_vars rule (the iteration variable)
    /// - The "in" keyword as part of the for statement
    #[test]
    fn test_parse_for_loop() {
        let source = "for x in items:\n    print(x)\n";
        let ast = parse_starlark(source);
        assert_file_root(&ast);

        let has_for = find_rule(&ast, "for_stmt");
        assert!(has_for, "Expected to find a for_stmt rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 6: BUILD file pattern (function call with keyword args)
    // -----------------------------------------------------------------------

    /// Starlark is primarily used in BUILD files, which consist of function
    /// calls with keyword arguments. This test verifies that the parser
    /// handles the most common BUILD file pattern.
    ///
    /// Note: brackets suppress INDENT/DEDENT, so the multi-line argument
    /// list is parsed as a flat sequence of tokens without indentation.
    #[test]
    fn test_parse_build_file() {
        let source = "cc_library(\n    name = \"foo\",\n)\n";
        let ast = parse_starlark(source);
        assert_file_root(&ast);

        // This is an expression statement (function call), so it should
        // be parsed as an assign_stmt containing a primary with a call suffix.
        assert!(!ast.children.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 7: Multiple statements
    // -----------------------------------------------------------------------

    /// A file with multiple top-level statements. The grammar's file rule
    /// is `file = { NEWLINE | statement } ;` so it should handle any number
    /// of statements separated by newlines.
    #[test]
    fn test_parse_multiple_statements() {
        let source = "x = 1\ny = 2\nz = x + y\n";
        let ast = parse_starlark(source);
        assert_file_root(&ast);

        let stmt_count = count_statements(&ast);
        assert_eq!(stmt_count, 3, "Expected 3 statements, got {}", stmt_count);
    }

    // -----------------------------------------------------------------------
    // Test 8: Factory function
    // -----------------------------------------------------------------------

    /// The `create_starlark_parser` factory function should return a
    /// working `GrammarParser` that can successfully parse source code.
    #[test]
    fn test_create_parser() {
        let mut parser = create_starlark_parser("x = 1\n");
        let result = parser.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());

        let ast = result.unwrap();
        assert_eq!(ast.rule_name, "file");
    }

    // -----------------------------------------------------------------------
    // Test 9: Lambda expression
    // -----------------------------------------------------------------------

    /// Lambda expressions are anonymous functions. This tests the
    /// lambda_expr grammar rule.
    #[test]
    fn test_parse_lambda() {
        let source = "f = lambda x: x + 1\n";
        let ast = parse_starlark(source);
        assert_file_root(&ast);

        let has_lambda = find_rule(&ast, "lambda_expr");
        assert!(has_lambda, "Expected to find a lambda_expr rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 10: List literal
    // -----------------------------------------------------------------------

    /// List literals are a common Starlark construct, especially in BUILD
    /// files (e.g., srcs = ["a.cc", "b.cc"]).
    #[test]
    fn test_parse_list_literal() {
        let source = "items = [1, 2, 3]\n";
        let ast = parse_starlark(source);
        assert_file_root(&ast);

        let has_list = find_rule(&ast, "list_expr");
        assert!(has_list, "Expected to find a list_expr rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening) -- see MAX_RULE_DEPTH's own
    // doc comment for the measurement.
    // -----------------------------------------------------------------------

    fn nested_paren_source(n: usize) -> String {
        format!("x = {}1{}\n", "(".repeat(n), ")".repeat(n))
    }

    /// Deeply-nested input must not overflow the native stack on a
    /// default-stack thread -- the whole point of the guard.
    #[test]
    fn test_deeply_nested_input_does_not_overflow_on_default_stack() {
        let src = nested_paren_source(5000);
        let handle = std::thread::spawn(move || {
            let mut parser = create_starlark_parser(&src);
            let _ = parser.parse();
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }

    /// Reasonable, hand-writable nesting stays well under the cap.
    #[test]
    fn test_reasonable_nesting_stays_under_the_cap() {
        let mut parser = create_starlark_parser(&nested_paren_source(9));
        assert!(parser.parse().is_ok());
    }

    // -----------------------------------------------------------------------
    // Helper: recursively search for a rule name in the AST
    // -----------------------------------------------------------------------

    /// Recursively search the AST for a node with the given rule name.
    /// Returns true if found anywhere in the tree.
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
}

