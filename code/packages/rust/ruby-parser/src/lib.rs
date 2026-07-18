//! Ruby parser backed by compiled parser grammar.

use coding_adventures_ruby_lexer::tokenize_ruby_for_version;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Recursion-depth cap for the Ruby [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] for why this guard exists at all (deep
/// recursion through `parse_rule` can overflow the *native* thread stack —
/// an uncatchable process abort — before this crate's own callers get a
/// chance to report anything). Before this constant was applied,
/// `create_ruby_parser` never called `with_max_depth` at all, leaving
/// every caller exposed to a native-stack-overflow DoS from adversarial
/// deeply-nested input (e.g. `x = (((...1...)))`).
///
/// `ruby.grammar` is the richest grammar audited in this pass — 18
/// distinct self-referential recursion shapes across three separate
/// mutually-recursive families: statement/block nesting (`if`/`case`/
/// `begin`/`def`/`class`/`module`/blocks/lambdas), expression/factor
/// nesting (parens/calls/array & hash literals/unary chains/ternaries),
/// and `case`/`in` structural pattern-matching nesting (array/hash/class
/// patterns). **Not the shared engine's bare default** (see
/// `csharp-parser`'s own identically-named constant for why a blind
/// `DEFAULT_MAX_RULE_DEPTH` (128) is unsafe-for-usability on a rich
/// general-purpose-language grammar). Measured directly instead (binary
/// search over candidate `with_max_depth` values against a fixed
/// 5000-level adversarial `x = (((...1...)))` input — ordinary
/// parenthesised grouping, one representative expression/factor shape —
/// on a default-~2MiB-stack worker thread in a debug build, no
/// `RUST_MIN_STACK` override or explicit `Builder::stack_size` present):
/// safe at **263**, crashes at **264**.
///
/// `MAX_RULE_DEPTH` is set to **180** — about 32% below that floor
/// (comparable margin to `apl-parser`'s own ~26.5%, `j-parser`'s ~30%,
/// `reduce-parser`'s ~28.5%). Measured real-input headroom at `180`: plain
/// parenthesised nesting parses cleanly to at least 10 levels — comfortably
/// beyond ordinary hand-written nesting depth.
///
/// This is measured against only **one** of Ruby's 18 recursion shapes
/// (ordinary paren grouping) — the other 17 (nested blocks, lambdas,
/// exception handling, structural pattern matching, etc.) are an
/// explicitly tracked follow-up, the way `css-parser`/`toml-parser`
/// measured *every* shape in their own (much smaller) grammars. This pass
/// at minimum replaces an unmeasured, silently-broken default with a
/// properly-measured floor for one representative shape.
const MAX_RULE_DEPTH: usize = 180;

/// Default Ruby era for the parser.  Phase 6w bumped this from "1.8"
/// (the lexer's default) to "3.0" so that era-gated lexer fusions
/// — most importantly `->` (Op("->") fused via `fuse_lambda_arrow`,
/// 1.9.1+) and the 2.x-era literal/operator fusions — are visible
/// to the parser by default.  Without this bump, `->(x) { x }` would
/// lex as `-`, `>`, ... and never match the `lambda_literal` rule.
///
/// "3.0" is chosen as the floor for "modern Ruby" — every 2.x and
/// 3.0 lexer fusion is on; later 3.x eras add nothing the parser
/// currently keys off.  Era-specific behaviour tests (e.g. lexer
/// crate tests asserting 1.8 emits `-`+`>` separately) can still use
/// the lower-level `tokenize_ruby_for_version` directly.
pub const DEFAULT_RUBY_ERA: &str = "3.0";

pub fn create_ruby_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_ruby_for_version(source, DEFAULT_RUBY_ERA)
        .expect("ruby lexer: DEFAULT_RUBY_ERA is a recognised era");
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH)
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
        // Phase 6s: each parameter is now wrapped in a `param` subnode
        // (to admit the optional `*`/`**` splat prefix), so we walk
        // the param children to extract Name tokens.
        let names: Vec<String> = params_node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "param" => {
                    n.children.iter().find_map(|cc| match cc {
                        ASTNodeOrToken::Token(t)
                            if matches!(t.type_, lexer::token::TokenType::Name) =>
                        {
                            Some(t.value.clone())
                        }
                        _ => None,
                    })
                }
                _ => None,
            })
            .collect();
        assert_eq!(names, vec!["x".to_string(), "y".to_string()]);
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

    #[test]
    fn test_parse_def_with_method_level_rescue() {
        // Phase 16e — a `def` body may carry a trailing `rescue` clause
        // without an explicit `begin`.
        let ast = parse_ruby("def f\n  x = 1\nrescue\n  y = 2\nend");
        let def = find_def_statement(&ast).expect("expected def_statement");
        assert!(
            find_descendant(def, "rescue_clause").is_some(),
            "expected a rescue_clause under the def body"
        );
    }

    #[test]
    fn test_parse_def_with_method_level_ensure() {
        // Phase 16e — a `def` body may carry a trailing `ensure` clause.
        let ast = parse_ruby("def f\n  x = 1\nensure\n  y = 2\nend");
        let def = find_def_statement(&ast).expect("expected def_statement");
        assert!(
            find_descendant(def, "ensure_clause").is_some(),
            "expected an ensure_clause under the def body"
        );
    }

    #[test]
    fn test_parse_def_with_typed_rescue_and_ensure() {
        // Phase 16e — full form: typed rescue + ensure on a method body.
        let ast = parse_ruby(
            "def f\n  x = 1\nrescue IOError => e\n  log = e\nensure\n  done = 1\nend",
        );
        let def = find_def_statement(&ast).expect("expected def_statement");
        assert!(find_descendant(def, "rescue_clause").is_some(), "expected rescue_clause");
        assert!(find_descendant(def, "ensure_clause").is_some(), "expected ensure_clause");
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

    // -----------------------------------------------------------------------
    // Regression — bare-identifier block bodies must not swallow their `end`.
    //
    // Before the `factor` guard, a block whose final statement is a bare
    // identifier mis-parsed: the identifier became a `method_call_no_paren`
    // callee and the block's terminating `end` (a KEYWORD token) was consumed
    // as that call's argument, so the enclosing `def`/`while`/`class` never
    // closed and its node vanished entirely.  These pin the repair: the
    // structural node is present, and value-keyword expressions still parse.
    // -----------------------------------------------------------------------

    #[test]
    fn bare_identifier_is_valid_def_body() {
        // `def f(a)\n a\nend` — the lone `a` used to eat the `end`.
        let ast = parse_ruby("def f(a)\n  a\nend");
        assert!(
            find_statement_inner(&ast, "def_statement").is_some(),
            "bare-identifier method body must keep the def intact"
        );
    }

    #[test]
    fn bare_identifier_is_valid_while_body() {
        let ast = parse_ruby("while c\n  a\nend");
        assert!(
            find_statement_inner(&ast, "while_statement").is_some(),
            "bare-identifier while body must keep the while intact"
        );
    }

    #[test]
    fn bare_identifier_is_valid_class_body() {
        let ast = parse_ruby("class Foo\n  a\nend");
        assert!(
            find_statement_inner(&ast, "class_statement").is_some(),
            "bare-identifier class body must keep the class intact"
        );
    }

    #[test]
    fn argless_return_does_not_eat_end() {
        // `return`'s optional expression must not consume the closing `end`.
        let ast = parse_ruby("def f\n  return\nend");
        assert!(
            find_statement_inner(&ast, "def_statement").is_some(),
            "argless return must not swallow the method's end"
        );
    }

    #[test]
    fn value_keywords_still_parse_as_expressions() {
        // The guard excludes only *structural* keywords; the value keywords
        // `nil`/`true`/`false`/`self` must still stand alone as expressions.
        for src in ["x = nil", "y = true", "z = false", "w = self"] {
            let ast = parse_ruby(src);
            assert!(
                count_statements(&ast) >= 1,
                "value-keyword expression `{src}` should still parse"
            );
        }
    }

    #[test]
    fn value_keyword_as_noparen_argument_still_parses() {
        // `puts nil` — a value keyword is a legitimate no-paren argument and
        // must survive the guard (only terminators are excluded).
        let ast = parse_ruby("puts nil");
        assert!(count_statements(&ast) >= 1, "`puts nil` should parse");
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
    // Phase 14d (FC) — `module M … end` → `Stmt::ModuleDef`.  Grammar is
    // unchanged (`module_statement = "module" NAME { !"end" statement }
    // "end"`); these tests pin the parse properties the 14d lowerer
    // relies on (name extraction, def/non-def body children).
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_module_name_is_first_name_token() {
        // `module Config; end` — the module name is the first Name
        // token (the `module` keyword is a Keyword-type token).
        let ast = parse_ruby("module Config\nend");
        let m = find_statement_inner(&ast, "module_statement")
            .expect("expected module_statement");
        let name_tok = m.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if matches!(t.type_, lexer::token::TokenType::Name) => {
                Some(t.value.as_str())
            }
            _ => None,
        });
        assert_eq!(name_tok, Some("Config"));
    }

    #[test]
    fn test_parse_module_body_with_def() {
        // A method def inside a module parses as a `def_statement` body
        // child (the lowerer hoists it to a top-level Function).
        let ast = parse_ruby("module M\n  def helper\n  end\nend");
        let m = find_statement_inner(&ast, "module_statement")
            .expect("expected module_statement");
        let names = body_inner_rule_names(m);
        assert!(
            names.iter().any(|r| r == "def_statement"),
            "expected a def_statement in the module body; got {:?}",
            names
        );
    }

    #[test]
    fn test_parse_module_body_mixes_def_and_assignment() {
        // `module M; VERSION = 3; def helper; end; end` — the body
        // holds both an `assignment` (stays in ModuleDef.body) and a
        // `def_statement` (hoisted).
        let ast = parse_ruby("module M\n  VERSION = 3\n  def helper\n  end\nend");
        let m = find_statement_inner(&ast, "module_statement")
            .expect("expected module_statement");
        let names = body_inner_rule_names(m);
        assert!(
            names.iter().any(|r| r == "assignment"),
            "expected an assignment in the module body; got {:?}",
            names
        );
        assert!(
            names.iter().any(|r| r == "def_statement"),
            "expected a def_statement in the module body; got {:?}",
            names
        );
    }

    // -----------------------------------------------------------------------
    // Phase 14a (FC) — empty `class Foo; end` first-class lowering.
    //
    // The grammar is unchanged (the `class_statement` rule already
    // accepts an empty body via `{ !"end" statement }` matching zero
    // times).  These tests pin the exact parse properties the Phase
    // 14a lowerer relies on: a single class_statement node, the class
    // name extractable as the first Name token, and zero body
    // statements for the empty form.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_empty_class_followed_by_top_level_stmt() {
        // An empty class must not swallow a following top-level
        // statement: `class Foo\nend\nx = 1` parses to a
        // class_statement *and* leaves `x = 1` as a sibling
        // statement at the program root.  This pins the negative
        // lookahead `!"end"` boundary the lowerer relies on when it
        // emits exactly one ClassDef stmt per class.
        let ast = parse_ruby("class Foo\nend\nx = 1");
        let cls = find_statement_inner(&ast, "class_statement")
            .expect("expected class_statement");
        // The empty class itself has no body statements.
        let body_count = cls
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "statement"))
            .count();
        assert_eq!(body_count, 0);
        // The trailing assignment survived as its own statement.
        assert!(
            find_statement_inner(&ast, "assignment").is_some(),
            "trailing `x = 1` should parse as a sibling assignment"
        );
    }

    #[test]
    fn test_parse_empty_class_camelcase_name() {
        // Multi-character CamelCase class name — the first Name token
        // is the whole identifier, not a truncation, and the `class`
        // keyword (a Keyword-type token) is not mistaken for it.
        let ast = parse_ruby("class WidgetFactory\nend");
        let cls = find_statement_inner(&ast, "class_statement")
            .expect("expected class_statement");
        let name_tok = cls.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if matches!(t.type_, lexer::token::TokenType::Name) => {
                Some(t.value.as_str())
            }
            _ => None,
        });
        assert_eq!(name_tok, Some("WidgetFactory"));
    }

    #[test]
    fn test_parse_empty_class_has_zero_body_statements() {
        // The empty-body invariant the lowerer depends on: an empty
        // class has no `statement` children in its body.
        let ast = parse_ruby("class Foo\nend");
        let cls = find_statement_inner(&ast, "class_statement")
            .expect("expected class_statement");
        let body_count = cls
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "statement"))
            .count();
        assert_eq!(body_count, 0, "empty class should have no body statements");
    }

    // -----------------------------------------------------------------------
    // Phase 14b (FC) — class body mixing method defs and executable
    // statements.  The grammar is still unchanged (`class_statement`'s
    // `{ !"end" statement }` body already accepts any statement); these
    // tests pin the parse shape the 14b lowerer walks: the body holds
    // one `statement` child per source line, each wrapping its own
    // inner rule (`def_statement`, `assignment`, nested
    // `class_statement`, …).
    // -----------------------------------------------------------------------

    /// Collect the inner-rule name of every `statement` child directly
    /// under `node`'s body (one level deep — does not recurse).
    fn body_inner_rule_names(node: &GrammarASTNode) -> Vec<String> {
        node.children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => {
                    n.children.iter().find_map(|inner| match inner {
                        ASTNodeOrToken::Node(d) => Some(d.rule_name.clone()),
                        _ => None,
                    })
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn test_parse_class_body_mixes_def_and_assignment() {
        // `class Foo\n  MAX = 10\n  def bar\n  end\nend` — the body has
        // two statements: an `assignment` (MAX = 10) and a
        // `def_statement` (bar).  The 14b lowerer hoists the def and
        // keeps the assignment in ClassDef.body.
        let ast = parse_ruby("class Foo\n  MAX = 10\n  def bar\n  end\nend");
        let cls = find_statement_inner(&ast, "class_statement")
            .expect("expected class_statement");
        let names = body_inner_rule_names(cls);
        assert!(
            names.iter().any(|r| r == "assignment"),
            "expected an assignment in the class body; got {:?}",
            names
        );
        assert!(
            names.iter().any(|r| r == "def_statement"),
            "expected a def_statement in the class body; got {:?}",
            names
        );
    }

    #[test]
    fn test_parse_class_body_multiple_assignments_preserved() {
        // Two executable statements parse as two distinct body
        // `statement` children, in source order.
        let ast = parse_ruby("class Cfg\n  A = 1\n  B = 2\nend");
        let cls = find_statement_inner(&ast, "class_statement")
            .expect("expected class_statement");
        let names = body_inner_rule_names(cls);
        assert_eq!(
            names,
            vec!["assignment".to_string(), "assignment".to_string()],
            "expected two assignment statements in order; got {:?}",
            names
        );
    }

    #[test]
    fn test_parse_nested_class_inside_class_body() {
        // A class declared inside another class parses as a nested
        // `class_statement` body child — the shape the 14b lowerer
        // recurses through (hoisting the inner class's defs exactly
        // once).
        let ast = parse_ruby("class Outer\n  class Inner\n  end\nend");
        let outer = find_statement_inner(&ast, "class_statement")
            .expect("expected outer class_statement");
        let names = body_inner_rule_names(outer);
        assert!(
            names.iter().any(|r| r == "class_statement"),
            "expected a nested class_statement in Outer's body; got {:?}",
            names
        );
    }

    // -----------------------------------------------------------------------
    // Phase 14c (FC) — inheritance `class Foo < Bar`.  The grammar's
    // `class_statement` gains an optional `[ "<" NAME ]` superclass
    // clause; `<` lexes as a Name-type token whose value is "<".
    // -----------------------------------------------------------------------

    /// Direct child tokens of `node` whose value equals `value`.
    fn body_has_token_value(node: &GrammarASTNode, value: &str) -> bool {
        node.children.iter().any(|c| matches!(
            c,
            ASTNodeOrToken::Token(t) if t.value == value
        ))
    }

    #[test]
    fn test_parse_class_with_superclass() {
        // `class Dog < Animal\nend` parses to a class_statement whose
        // direct children include the `<` separator token and the
        // superclass Name token `Animal`.
        let ast = parse_ruby("class Dog < Animal\nend");
        let cls = find_statement_inner(&ast, "class_statement")
            .expect("expected class_statement");
        assert!(
            body_has_token_value(cls, "<"),
            "expected a `<` superclass separator token in the class header"
        );
        assert!(
            body_has_token_value(cls, "Animal"),
            "expected the superclass name `Animal` token in the class header"
        );
        // The empty subclass has no body statements.
        let body_count = cls
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "statement"))
            .count();
        assert_eq!(body_count, 0, "empty subclass should have no body statements");
    }

    #[test]
    fn test_parse_base_class_has_no_superclass_separator() {
        // A base class `class Widget\nend` has no `<` token — the
        // optional superclass clause matched zero times.
        let ast = parse_ruby("class Widget\nend");
        let cls = find_statement_inner(&ast, "class_statement")
            .expect("expected class_statement");
        assert!(
            !body_has_token_value(cls, "<"),
            "base class must not carry a `<` superclass separator"
        );
    }

    #[test]
    fn test_parse_subclass_with_method_body() {
        // Inheritance composes with a non-empty body: `class Cat <
        // Animal; def meow; end; end` parses with the `<` separator AND
        // a def_statement body child.
        let ast = parse_ruby("class Cat < Animal\n  def meow\n  end\nend");
        let cls = find_statement_inner(&ast, "class_statement")
            .expect("expected class_statement");
        assert!(body_has_token_value(cls, "<"), "expected `<` separator");
        assert!(body_has_token_value(cls, "Animal"), "expected superclass `Animal`");
        let names = body_inner_rule_names(cls);
        assert!(
            names.iter().any(|r| r == "def_statement"),
            "expected a def_statement in the subclass body; got {:?}",
            names
        );
    }

    // -----------------------------------------------------------------------
    // Phase 14e (FC) — singleton class `class << receiver … end`.  The
    // `class_statement` rule gains a singleton alternative
    // (`"class" "<<" singleton_receiver …`); `<<` lexes as an Op token
    // (value "<<"), `self` as a Keyword.  The parse carries a
    // `singleton_receiver` child node the lowerer dispatches on.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_singleton_class_of_self() {
        // `class << self; end` parses to a class_statement carrying a
        // `singleton_receiver` child (whose token is `self`), plus the
        // `<<` separator token.
        let ast = parse_ruby("class << self\nend");
        let cls = find_statement_inner(&ast, "class_statement")
            .expect("expected class_statement");
        assert!(body_has_token_value(cls, "<<"), "expected `<<` separator token");
        let recv = cls.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "singleton_receiver" => Some(n),
            _ => None,
        });
        let recv = recv.expect("expected a singleton_receiver child node");
        let recv_tok = recv.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) => Some(t.value.as_str()),
            _ => None,
        });
        assert_eq!(recv_tok, Some("self"));
    }

    #[test]
    fn test_parse_singleton_class_body_with_def() {
        // A def inside a singleton class parses as a def_statement body
        // child (hoisted by the lowerer).
        let ast = parse_ruby("class << self\n  def foo\n  end\nend");
        let cls = find_statement_inner(&ast, "class_statement")
            .expect("expected class_statement");
        let names = body_inner_rule_names(cls);
        assert!(
            names.iter().any(|r| r == "def_statement"),
            "expected a def_statement in the singleton body; got {:?}",
            names
        );
    }

    #[test]
    fn test_parse_ordinary_class_has_no_singleton_receiver() {
        // Regression guard: `class Foo` must NOT carry a
        // singleton_receiver child (the singleton alternative must not
        // shadow the ordinary form).
        let ast = parse_ruby("class Foo\nend");
        let cls = find_statement_inner(&ast, "class_statement")
            .expect("expected class_statement");
        let has_recv = cls.children.iter().any(|c| matches!(
            c,
            ASTNodeOrToken::Node(n) if n.rule_name == "singleton_receiver"
        ));
        assert!(!has_recv, "ordinary class must have no singleton_receiver child");
    }

    // -----------------------------------------------------------------------
    // Phase 15a (FC) — instance variables `@x`.  No grammar change: the
    // lexer already emits `@x` as a `Name` token (sigil included), and
    // ivar assignment / expression parsing is covered by the Phase 6x
    // tests (`test_parse_instance_var_assignment`,
    // `test_parse_instance_var_in_expression`).  Phase 15a adds only the
    // compound-assign parse pin the new lowering path exercises.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_instance_var_compound_assignment() {
        // `@n += 1` parses as an assignment carrying the `@n` Name token
        // and the fused `+=` operator token (the shape the 15a lowerer's
        // compound-ivar path dispatches on).
        let ast = parse_ruby("@n += 1");
        let asn = find_statement_inner(&ast, "assignment")
            .expect("expected assignment");
        assert!(
            body_has_token_value(asn, "@n"),
            "expected the `@n` ivar Name token in the assignment header"
        );
        assert!(
            body_has_token_value(asn, "+="),
            "expected the fused `+=` compound-assign operator token"
        );
    }

    #[test]
    fn test_parse_class_var_compound_assignment() {
        // `@@n += 1` parses as an assignment carrying the `@@n` Name token
        // and the fused `+=` operator token (the shape the 15b lowerer's
        // compound-cvar path dispatches on).
        let ast = parse_ruby("@@n += 1");
        let asn = find_statement_inner(&ast, "assignment")
            .expect("expected assignment");
        assert!(
            body_has_token_value(asn, "@@n"),
            "expected the `@@n` cvar Name token in the assignment header"
        );
        assert!(
            body_has_token_value(asn, "+="),
            "expected the fused `+=` compound-assign operator token"
        );
    }

    #[test]
    fn test_parse_constant_assignment() {
        // `MAX = 10` parses as an assignment carrying the uppercase-initial
        // `MAX` Name token (the shape the 15c lowerer routes to
        // `Scope::Const`).
        let ast = parse_ruby("MAX = 10");
        let asn = find_statement_inner(&ast, "assignment")
            .expect("expected assignment");
        assert!(
            body_has_token_value(asn, "MAX"),
            "expected the `MAX` constant Name token in the assignment header"
        );
    }

    #[test]
    fn test_parse_scope_resolution_foo_bar() {
        // `Foo::Bar` parses with a `scope_resolution` postfix node
        // carrying the `::` operator token and the `Bar` Name.
        let ast = parse_ruby("Foo::Bar");
        let sr = find_descendant(&ast, "scope_resolution")
            .expect("expected scope_resolution node");
        assert!(
            body_has_token_value(sr, "::"),
            "scope_resolution should carry the `::` operator token"
        );
        assert!(
            body_has_token_value(sr, "Bar"),
            "scope_resolution should carry the `Bar` Name token"
        );
    }

    #[test]
    fn test_parse_scope_resolution_chain() {
        // `A::B::C` parses as a chain of TWO `scope_resolution` steps.
        let ast = parse_ruby("A::B::C");
        let factor = find_descendant(&ast, "factor")
            .expect("expected factor node");
        let steps = factor
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "scope_resolution"))
            .count();
        assert_eq!(steps, 2, "A::B::C has two `::` steps; got {}", steps);
    }

    #[test]
    fn test_parse_scope_resolution_then_dot_call() {
        // `Foo::Bar.baz` mixes a `::` step and a `.` step under the same
        // factor postfix — both node kinds coexist.
        let ast = parse_ruby("Foo::Bar.baz");
        let factor = find_descendant(&ast, "factor")
            .expect("expected factor node");
        assert!(
            factor.children.iter().any(|c|
                matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "scope_resolution")),
            "expected a scope_resolution step"
        );
        assert!(
            factor.children.iter().any(|c|
                matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "dot_call")),
            "expected a dot_call step"
        );
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

    // -----------------------------------------------------------------------
    // Phase 21a (FC) — block-local variables `{ |x; y| … }`
    //
    // After the regular block parameters, an optional `;` introduces a list
    // of block-local variables.  The pipe contents become
    // `NAME { COMMA NAME } [ ";" NAME { COMMA NAME } ]`.  These pins confirm
    // the `;` Semicolon token and the block-local names parse inside the
    // `block_params` node.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_brace_block_with_one_block_local() {
        let ast = parse_ruby("each { |x; y| x }");
        let mwb = find_statement_inner(&ast, "method_with_block")
            .expect("expected method_with_block");
        let bp = find_descendant(mwb, "block_params").expect("expected block_params");
        // The `;` Semicolon token survives inside block_params.
        assert!(bp.children.iter().any(|c| matches!(
            c,
            ASTNodeOrToken::Token(t) if matches!(t.type_, lexer::token::TokenType::Semicolon)
        )));
        // Both `x` (param) and `y` (block-local) appear as Name tokens.
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
    fn test_parse_do_block_with_two_block_locals() {
        let ast = parse_ruby("each do |a; b, c|\n  puts a\nend");
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
        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_block_without_locals_has_no_semicolon() {
        // Regression: a plain `|x, y|` block must NOT contain a Semicolon
        // token in its block_params (the `;` clause is optional).
        let ast = parse_ruby("each { |x, y| x }");
        let mwb = find_statement_inner(&ast, "method_with_block")
            .expect("expected method_with_block");
        let bp = find_descendant(mwb, "block_params").expect("expected block_params");
        assert!(!bp.children.iter().any(|c| matches!(
            c,
            ASTNodeOrToken::Token(t) if matches!(t.type_, lexer::token::TokenType::Semicolon)
        )));
    }

    // -----------------------------------------------------------------------
    // Phase 21b (FC) — implicit numbered block parameters `_1`..`_9`.
    //
    // A block with NO explicit `|...|` header may use `_1`..`_9` in its
    // body as positional parameters.  Parser-side, `_1` lexes as a plain
    // Name token (the lexer flags it with NUMBERED_BLOCK_PARAM_FLAG but
    // keeps the type); these pins confirm such blocks parse and the
    // `_N` Name token reaches the brace_block / do_block body with no
    // block_params header.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_block_with_numbered_param_parses() {
        let ast = parse_ruby("each { puts(_1) }");
        let mwb = find_statement_inner(&ast, "method_with_block")
            .expect("expected method_with_block");
        // No explicit pipe header.
        assert!(find_descendant(mwb, "block_params").is_none());
        // The `_1` Name token reaches the block body.
        assert!(tree_has_token_value(mwb, "_1"));
    }

    #[test]
    fn test_parse_block_with_two_numbered_params_parses() {
        let ast = parse_ruby("each { puts(_1 + _2) }");
        let mwb = find_statement_inner(&ast, "method_with_block")
            .expect("expected method_with_block");
        assert!(find_descendant(mwb, "block_params").is_none());
        assert!(tree_has_token_value(mwb, "_1"));
        assert!(tree_has_token_value(mwb, "_2"));
    }

    #[test]
    fn test_parse_do_block_with_numbered_param_parses() {
        let ast = parse_ruby("each do\n  puts(_1)\nend");
        let mwb = find_statement_inner(&ast, "method_with_block")
            .expect("expected method_with_block");
        assert!(find_descendant(mwb, "do_block").is_some());
        assert!(tree_has_token_value(mwb, "_1"));
    }

    // -----------------------------------------------------------------------
    // Phase 21c (FC) — implicit `it` block parameter (Ruby 3.4).
    //
    // A header-less block may use a bare `it` in its body as the first
    // block argument.  Parser-side `it` lexes as a plain Name token;
    // these pins confirm such blocks parse with no block_params header
    // and the `it` Name token reaches the body.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_block_with_implicit_it_parses() {
        let ast = parse_ruby("each { puts(it) }");
        let mwb = find_statement_inner(&ast, "method_with_block")
            .expect("expected method_with_block");
        assert!(find_descendant(mwb, "block_params").is_none());
        assert!(tree_has_token_value(mwb, "it"));
    }

    #[test]
    fn test_parse_do_block_with_implicit_it_parses() {
        let ast = parse_ruby("each do\n  puts(it)\nend");
        let mwb = find_statement_inner(&ast, "method_with_block")
            .expect("expected method_with_block");
        assert!(find_descendant(mwb, "do_block").is_some());
        assert!(tree_has_token_value(mwb, "it"));
    }

    #[test]
    fn test_parse_block_with_it_dot_method_parses() {
        // `it.foo` — `it` as a receiver still parses; the `it` token is
        // present in the block body.
        let ast = parse_ruby("each { puts(it.foo) }");
        let mwb = find_statement_inner(&ast, "method_with_block")
            .expect("expected method_with_block");
        assert!(tree_has_token_value(mwb, "it"));
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
    // Phase 11a — `break`/`next` WITH VALUES (coverage-confirmation)
    //
    // The Phase 6j grammar rules already accept an optional trailing
    // expression after `break`/`next` (`break_statement = "break" [ expression ]`).
    // These pins lock in additional surface forms from new angles —
    // a bare integer payload, `next` carrying a value, and a binary
    // expression payload — so a future grammar edit cannot silently
    // drop value-carrying loop control without tripping a test.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_break_with_int_value() {
        // `break 5` — simple integer payload (distinct from the
        // existing `break 1 + 2` pin, which exercises a binary expr).
        let ast = parse_ruby("break 5");
        let b = find_statement_inner(&ast, "break_statement")
            .expect("expected break_statement");
        assert!(b
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression")));
    }

    #[test]
    fn test_parse_next_with_value() {
        // `next 7` — `next` carrying an integer payload (the existing
        // `next` pin is bare; this confirms the optional-expr arm fires
        // for `next` as well as `break`).
        let ast = parse_ruby("next 7");
        let n = find_statement_inner(&ast, "next_statement")
            .expect("expected next_statement");
        assert!(n
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Node(nn) if nn.rule_name == "expression")));
    }

    #[test]
    fn test_parse_break_with_binary_name_value() {
        // `break x + 1` — a name-plus-literal binary expression payload;
        // the `+` token must survive inside the break_statement subtree.
        let ast = parse_ruby("break x + 1");
        let b = find_statement_inner(&ast, "break_statement")
            .expect("expected break_statement");
        assert!(b
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression")));
        assert!(tree_has_token_value(b, "+"));
    }

    // -----------------------------------------------------------------------
    // Phase 11b — `redo` keyword (restart current loop iteration)
    //
    // `redo` is a bare control-flow keyword (lexer-tagged KEYWORD) that
    // never carries a value.  The grammar rule `redo_statement = "redo"`
    // sits in the `statement` alternation right after `next_statement`.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_redo_bare() {
        let ast = parse_ruby("redo");
        assert!(find_statement_inner(&ast, "redo_statement").is_some());
    }

    #[test]
    fn test_parse_redo_has_no_expression_child() {
        // `redo` carries no operand — unlike `break`/`next`, its subtree
        // must contain NO `expression` node.
        let ast = parse_ruby("redo");
        let r = find_statement_inner(&ast, "redo_statement")
            .expect("expected redo_statement");
        assert!(!r
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression")));
    }

    #[test]
    fn test_parse_redo_inside_while_body() {
        // `redo` is most idiomatic inside a loop body; confirm it parses
        // as a statement within a `while … end` block.  Use the recursive
        // descendant search since the keyword nests below the top level.
        let ast = parse_ruby("while x\n  redo\nend");
        assert!(find_descendant(&ast, "redo_statement").is_some());
    }

    // -----------------------------------------------------------------------
    // Phase 11c — `retry` keyword (re-execute enclosing begin block)
    //
    // `retry` is a bare control-flow keyword (lexer-tagged KEYWORD) that
    // never carries a value.  The grammar rule `retry_statement = "retry"`
    // sits in the `statement` alternation right after `redo_statement`.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_retry_bare() {
        let ast = parse_ruby("retry");
        assert!(find_statement_inner(&ast, "retry_statement").is_some());
    }

    #[test]
    fn test_parse_retry_has_no_expression_child() {
        // `retry` carries no operand — its subtree must contain NO
        // `expression` node (like `redo`, unlike `break`/`next`).
        let ast = parse_ruby("retry");
        let r = find_statement_inner(&ast, "retry_statement")
            .expect("expected retry_statement");
        assert!(!r
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression")));
    }

    #[test]
    fn test_parse_retry_inside_begin_rescue_body() {
        // `retry` is idiomatic inside a `rescue` clause; confirm it parses
        // as a statement within a `begin … rescue … end` block.  Use the
        // recursive descendant search since the keyword nests deep.
        let ast = parse_ruby("begin\n  x = 1\nrescue\n  retry\nend");
        assert!(find_descendant(&ast, "retry_statement").is_some());
    }

    // -----------------------------------------------------------------------
    // Phase 11d — `return` WITH VALUE (coverage-confirmation)
    //
    // The Phase 6j rule `return_statement = "return" [ expression ]` already
    // accepts a trailing expression.  Existing pins cover `return 42`, bare
    // `return`, and `return x + 1` inside a def.  These pins lock in new
    // payload shapes — an array literal, a string literal, and a
    // parenthesized binary expression — so a future grammar edit cannot
    // silently drop value-carrying returns.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_return_with_array_value() {
        let ast = parse_ruby("return [1, 2]");
        let r = find_statement_inner(&ast, "return_statement")
            .expect("expected return_statement");
        assert!(r
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression")));
    }

    #[test]
    fn test_parse_return_with_string_value() {
        let ast = parse_ruby("return \"ok\"");
        let r = find_statement_inner(&ast, "return_statement")
            .expect("expected return_statement");
        assert!(r
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression")));
    }

    #[test]
    fn test_parse_return_with_paren_value() {
        // `return (1 + 2)` — parenthesized payload; the `+` token must
        // survive somewhere inside the return_statement subtree.
        let ast = parse_ruby("return (1 + 2)");
        let r = find_statement_inner(&ast, "return_statement")
            .expect("expected return_statement");
        assert!(r
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression")));
        assert!(tree_has_token_value(r, "+"));
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
        // Phase 6s: each argument is now wrapped in a `call_arg` rule
        // (to admit optional `*`/`**` splat prefixes).
        let arg_count = dot
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "call_arg"))
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
                ASTNodeOrToken::Node(sub)
                    if tree_has_token_value(sub, value) => {
                        return true;
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
    // Phase 10a (FC) — inclusive range `1..5` coverage confirmation.
    //
    // Inclusive ranges were first implemented in Phase 6n (the `range`
    // rule + `lower_range`).  Phase 10a is a coverage-confirmation phase
    // (cf. Phases 16b/16c): it adds explicit parser pins for inclusive
    // ranges in syntactic positions the original 6n tests did not cover —
    // string endpoints, a call argument, and a parenthesized range — so a
    // regression in any of those positions is caught by name.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_inclusive_range_string_endpoints() {
        // `"a".."z"` — string literal endpoints.  The lexer emits
        // String Dot Dot String and `fuse_range_ops` folds the two Dots
        // into a single `..`, so the range node is shape-identical to the
        // integer case (two `logical_or` operands flanking `..`).
        let ast = parse_ruby("\"a\"..\"z\"");
        let r = find_descendant(&ast, "range").expect("expected range node");
        assert!(
            tree_has_token_value(r, ".."),
            "expected `..` token inside the string-endpoint range"
        );
        let operand_count = r
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "logical_or"))
            .count();
        assert_eq!(operand_count, 2, "expected 2 logical_or operands");
    }

    #[test]
    fn test_parse_inclusive_range_as_call_argument() {
        // `foo(1..5)` — an inclusive range used as a method-call argument.
        // The range node must appear below the call's argument list.
        let ast = parse_ruby("foo(1..5)");
        let r = find_descendant(&ast, "range").expect("expected range node");
        assert!(
            tree_has_token_value(r, ".."),
            "expected `..` token in the call-argument range"
        );
    }

    #[test]
    fn test_parse_inclusive_range_parenthesized() {
        // `(1..5)` — a parenthesized inclusive range.  Parens are a
        // primary/atom wrapper; the range still parses to a `range` node
        // carrying the `..` token.
        let ast = parse_ruby("(1..5)");
        let r = find_descendant(&ast, "range").expect("expected range node");
        assert!(
            tree_has_token_value(r, ".."),
            "expected `..` token in the parenthesized range"
        );
    }

    // -----------------------------------------------------------------------
    // Phase 10c (FC) — endless range `1..` / `1...`.
    //
    // The `range` rule's trailing operand is now optional:
    //   range = logical_or [ ( "..." | ".." ) [ logical_or ] ]
    // so a range op with no following operand (the next token is a
    // closer that cannot begin a `logical_or`) yields a `range` node
    // with exactly ONE `logical_or` operand plus the op token.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_endless_range_inclusive() {
        // `1..` — endless inclusive range: one operand, `..` token, no
        // trailing operand.
        let ast = parse_ruby("1..");
        let r = find_descendant(&ast, "range").expect("expected range node");
        assert!(
            tree_has_token_value(r, ".."),
            "expected `..` token in the endless range"
        );
        let operand_count = r
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "logical_or"))
            .count();
        assert_eq!(operand_count, 1, "endless range should carry exactly 1 operand");
    }

    #[test]
    fn test_parse_endless_range_exclusive() {
        // `1...` — endless exclusive range carries the `...` token.
        let ast = parse_ruby("1...");
        let r = find_descendant(&ast, "range").expect("expected range node");
        assert!(
            tree_has_token_value(r, "..."),
            "expected `...` token in the endless exclusive range"
        );
        let operand_count = r
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "logical_or"))
            .count();
        assert_eq!(operand_count, 1, "endless range should carry exactly 1 operand");
    }

    #[test]
    fn test_parse_endless_range_parenthesized() {
        // `(1..)` — the closer here is `)`; the optional operand matches
        // nothing and the range stays endless.
        let ast = parse_ruby("(1..)");
        let r = find_descendant(&ast, "range").expect("expected range node");
        assert!(
            tree_has_token_value(r, ".."),
            "expected `..` token in the parenthesized endless range"
        );
        let operand_count = r
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "logical_or"))
            .count();
        assert_eq!(operand_count, 1, "endless range should carry exactly 1 operand");
    }

    // -----------------------------------------------------------------------
    // Phase 10d (FC) — beginless range `..5` / `...5`.
    //
    // The `range` rule gained a FIRST alternative leading with the op:
    //   range = ( "..." | ".." ) logical_or | logical_or [ … ] ;
    // so a leading `..`/`...` followed by an operand parses to a `range`
    // node carrying the op token and exactly ONE `logical_or` operand —
    // same shape (count-wise) as an endless range, but the op token comes
    // BEFORE the operand instead of after.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_beginless_range_inclusive() {
        // `x = ..5` — beginless inclusive as an assignment RHS.  A bare
        // leading `..` at statement start is a separate dispatch quirk
        // (like the bare-NAME quirk); routing through an expression
        // position (assignment RHS, parens, array) parses cleanly.
        let ast = parse_ruby("x = ..5");
        let r = find_descendant(&ast, "range").expect("expected range node");
        assert!(
            tree_has_token_value(r, ".."),
            "expected `..` token in the beginless range"
        );
        let operand_count = r
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "logical_or"))
            .count();
        assert_eq!(operand_count, 1, "beginless range should carry exactly 1 operand");
    }

    #[test]
    fn test_parse_beginless_range_exclusive() {
        // `(...5)` — beginless exclusive carries the `...` token.
        let ast = parse_ruby("(...5)");
        let r = find_descendant(&ast, "range").expect("expected range node");
        assert!(
            tree_has_token_value(r, "..."),
            "expected `...` token in the beginless exclusive range"
        );
    }

    #[test]
    fn test_parse_beginless_range_parenthesized() {
        // `(..5)` — parenthesized beginless range.
        let ast = parse_ruby("(..5)");
        let r = find_descendant(&ast, "range").expect("expected range node");
        assert!(
            tree_has_token_value(r, ".."),
            "expected `..` token in the parenthesized beginless range"
        );
        let operand_count = r
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "logical_or"))
            .count();
        assert_eq!(operand_count, 1, "beginless range should carry exactly 1 operand");
    }

    // -----------------------------------------------------------------------
    // Phase 19a (FC) — regex literal `/pattern/flags`.
    //
    // The lexer resolves the `/`-vs-division ambiguity and emits a regex
    // as a `String` token carrying the verbatim `/p/flags` (slashes
    // included), so the parser routes it through the ordinary
    // string-literal slot — no grammar change.  These pins confirm the
    // verbatim lexeme survives into the parse tree.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_regex_literal() {
        // `x = /foo/` — assignment RHS is a regex; the `/foo/` lexeme
        // appears verbatim in the tree as a String token.
        let ast = parse_ruby("x = /foo/");
        assert!(
            tree_has_token_value(&ast, "/foo/"),
            "expected verbatim `/foo/` regex token in the parse tree"
        );
    }

    #[test]
    fn test_parse_regex_literal_with_flags() {
        // `x = /foo/i` — flags ride along in the same verbatim lexeme.
        let ast = parse_ruby("x = /foo/i");
        assert!(
            tree_has_token_value(&ast, "/foo/i"),
            "expected verbatim `/foo/i` regex token in the parse tree"
        );
    }

    #[test]
    fn test_parse_regex_literal_in_call_argument() {
        // `foo(/bar/)` — regex as a method-call argument.
        let ast = parse_ruby("foo(/bar/)");
        assert!(
            tree_has_token_value(&ast, "/bar/"),
            "expected verbatim `/bar/` regex token in the call argument"
        );
    }

    // -----------------------------------------------------------------------
    // Phase 19b (FC) — regex flags `/r/i` (coverage).  Flags ride along
    // in the verbatim `/p/flags` lexeme (no grammar change since 19a);
    // these pins confirm MULTI-flag lexemes survive into the parse tree.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_regex_literal_multi_flag() {
        // `x = /foo/im` — two trailing flag letters in the lexeme.
        let ast = parse_ruby("x = /foo/im");
        assert!(
            tree_has_token_value(&ast, "/foo/im"),
            "expected verbatim `/foo/im` multi-flag regex token"
        );
    }

    #[test]
    fn test_parse_regex_literal_three_flags() {
        // `x = /a/mix` — three flag letters.
        let ast = parse_ruby("x = /a/mix");
        assert!(
            tree_has_token_value(&ast, "/a/mix"),
            "expected verbatim `/a/mix` three-flag regex token"
        );
    }

    #[test]
    fn test_parse_regex_literal_multi_flag_in_call_argument() {
        // `foo(/bar/im)` — multi-flag regex as a call argument.
        let ast = parse_ruby("foo(/bar/im)");
        assert!(
            tree_has_token_value(&ast, "/bar/im"),
            "expected verbatim `/bar/im` regex token in the call argument"
        );
    }

    // -----------------------------------------------------------------------
    // Phase 19c (FC) — regex interpolation `/a#{b}c/`.  The `regex_body`
    // lexer state captures `#{...}` markers verbatim into the body, so
    // the interpolated regex still arrives as ONE `String` token whose
    // value includes the markers — no grammar change.  These pins confirm
    // the verbatim interpolated lexeme survives into the parse tree.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_regex_literal_with_interpolation() {
        // `x = /a#{b}c/` — `#{b}` preserved verbatim in the regex token.
        let ast = parse_ruby("x = /a#{b}c/");
        assert!(
            tree_has_token_value(&ast, "/a#{b}c/"),
            "expected verbatim `/a#{{b}}c/` interpolated regex token"
        );
    }

    #[test]
    fn test_parse_regex_interpolation_single_marker() {
        // `x = /#{b}/` — a lone interpolation marker.
        let ast = parse_ruby("x = /#{b}/");
        assert!(
            tree_has_token_value(&ast, "/#{b}/"),
            "expected verbatim `/#{{b}}/` regex token"
        );
    }

    #[test]
    fn test_parse_regex_interpolation_with_flags() {
        // `x = /x#{b}/i` — interpolation plus a trailing flag.
        let ast = parse_ruby("x = /x#{b}/i");
        assert!(
            tree_has_token_value(&ast, "/x#{b}/i"),
            "expected verbatim `/x#{{b}}/i` regex token"
        );
    }

    // -----------------------------------------------------------------------
    // Phase 19d (FC) — `%r{...}` regex literal.  The lexer emits the whole
    // literal as one `String` token carrying the verbatim `%r{...}` source
    // (the `%r` + braces preserved) — the `%`-family sentinel-by-prefix
    // trick.  No grammar change.  These pins confirm the verbatim `%r{}`
    // lexeme survives into the parse tree.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_percent_r_regex_literal() {
        // `x = %r{hello}` — `%r{...}` regex as an assignment RHS.
        let ast = parse_ruby("x = %r{hello}");
        assert!(
            tree_has_token_value(&ast, "%r{hello}"),
            "expected verbatim `%r{{hello}}` regex token in the parse tree"
        );
    }

    #[test]
    fn test_parse_percent_r_regex_empty() {
        // `x = %r{}` — empty `%r{}` regex.
        let ast = parse_ruby("x = %r{}");
        assert!(
            tree_has_token_value(&ast, "%r{}"),
            "expected verbatim `%r{{}}` regex token"
        );
    }

    #[test]
    fn test_parse_percent_r_regex_in_call_argument() {
        // `foo(%r{bar})` — `%r{...}` regex as a method-call argument.
        let ast = parse_ruby("foo(%r{bar})");
        assert!(
            tree_has_token_value(&ast, "%r{bar}"),
            "expected verbatim `%r{{bar}}` regex token in the call argument"
        );
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

    // -----------------------------------------------------------------------
    // Phase 6r — multiple assignment `a, b = 1, 2`
    //
    // The grammar rule is:
    //   multi_assignment = NAME COMMA NAME { COMMA NAME }
    //                      EQUALS
    //                      expression { COMMA expression } ;
    //
    // Placed BEFORE `modifier_statement` and `assignment` in the statement
    // alternation so that `NAME COMMA NAME ... =` parses as a multi-assignment
    // and not as a `method_call_no_paren` (which couldn't match the second
    // comma anyway).  Single-LHS assignments (`a = 1`) still flow through
    // the existing `assignment` rule because `multi_assignment` requires
    // at least two LHS names and falls through cleanly when only one is
    // present.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_multi_assignment_two_names() {
        // Smallest form: `a, b = 1, 2`.
        let ast = parse_ruby("a, b = 1, 2");
        let m = find_descendant(&ast, "multi_assignment")
            .expect("expected multi_assignment node");
        // Two NAME tokens on the LHS, two NUMBER tokens (or NUMBER-bearing
        // expression nodes) on the RHS.
        fn count_token_type<F: Fn(&lexer::token::Token) -> bool>(
            node: &GrammarASTNode,
            pred: F,
        ) -> usize {
            fn walk<F: Fn(&lexer::token::Token) -> bool>(
                node: &GrammarASTNode,
                pred: &F,
                acc: &mut usize,
            ) {
                for c in &node.children {
                    match c {
                        ASTNodeOrToken::Token(t) if pred(t) => *acc += 1,
                        ASTNodeOrToken::Node(sub) => walk(sub, pred, acc),
                        _ => {}
                    }
                }
            }
            let mut n = 0;
            walk(node, &pred, &mut n);
            n
        }
        // The LHS NAME count should be exactly 2; the `EQUALS` token is
        // also part of the multi_assignment node.
        let name_count = count_token_type(m, |t| {
            t.type_ == lexer::token::TokenType::Name && (t.value == "a" || t.value == "b")
        });
        assert_eq!(name_count, 2, "expected 2 LHS NAME tokens for a, b");
        let eq_count = count_token_type(m, |t| {
            t.type_ == lexer::token::TokenType::Equals
        });
        assert_eq!(eq_count, 1, "expected exactly one `=` token");
    }

    #[test]
    fn test_parse_multi_assignment_three_names() {
        // Three-LHS form: `a, b, c = 1, 2, 3`.
        let ast = parse_ruby("a, b, c = 1, 2, 3");
        let m = find_descendant(&ast, "multi_assignment")
            .expect("expected multi_assignment node for 3-LHS");
        // Three expression subtrees on the RHS.
        let rhs_expr_count = m.children.iter().filter(|c| matches!(c,
            ASTNodeOrToken::Node(n) if n.rule_name == "expression"
        )).count();
        assert_eq!(rhs_expr_count, 3, "expected 3 RHS expression nodes");
    }

    #[test]
    fn test_parse_multi_assignment_with_complex_rhs() {
        // RHS may be any expression — arithmetic, names, etc.
        let ast = parse_ruby("a, b = x + 1, y * 2");
        let m = find_descendant(&ast, "multi_assignment")
            .expect("expected multi_assignment node");
        // Each RHS expression carries its own operator.
        assert!(tree_has_token_value(m, "+"), "expected `+` in first RHS");
        assert!(tree_has_token_value(m, "*"), "expected `*` in second RHS");
    }

    // -----------------------------------------------------------------------
    // Phase 9b (FC) — splat LHS in multi-assignment
    //
    // Grammar addition:
    //   multi_assignment = mlhs_target COMMA mlhs_target { COMMA mlhs_target }
    //                      EQUALS expression { COMMA expression } ;
    //   mlhs_target      = [ "*" ] NAME ;
    //
    // We assert: (1) `*` appears as a leading token in the right
    // `mlhs_target` slot, (2) the parser recognises the construct as
    // `multi_assignment`, and (3) the LHS target count comes out
    // right.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_multi_assignment_splat_at_end() {
        let ast = parse_ruby("a, *b = 1, 2, 3");
        let m = find_descendant(&ast, "multi_assignment")
            .expect("expected multi_assignment node");
        // Count mlhs_target children to confirm splat is its own slot.
        let lhs_target_count = m.children.iter().filter(|c| matches!(c,
            ASTNodeOrToken::Node(n) if n.rule_name == "mlhs_target"
        )).count();
        assert_eq!(lhs_target_count, 2, "expected 2 mlhs_target nodes");
        assert!(tree_has_token_value(m, "*"), "expected `*` in tree");
    }

    #[test]
    fn test_parse_multi_assignment_splat_at_start() {
        let ast = parse_ruby("*a, b = 1, 2, 3");
        let m = find_descendant(&ast, "multi_assignment")
            .expect("expected multi_assignment node");
        let lhs_target_count = m.children.iter().filter(|c| matches!(c,
            ASTNodeOrToken::Node(n) if n.rule_name == "mlhs_target"
        )).count();
        assert_eq!(lhs_target_count, 2, "expected 2 mlhs_target nodes");
        assert!(tree_has_token_value(m, "*"));
    }

    #[test]
    fn test_parse_multi_assignment_splat_in_middle() {
        let ast = parse_ruby("a, *b, c = 1, 2, 3, 4");
        let m = find_descendant(&ast, "multi_assignment")
            .expect("expected multi_assignment node");
        let lhs_target_count = m.children.iter().filter(|c| matches!(c,
            ASTNodeOrToken::Node(n) if n.rule_name == "mlhs_target"
        )).count();
        assert_eq!(lhs_target_count, 3, "expected 3 mlhs_target nodes");
        assert!(tree_has_token_value(m, "*"));
    }

    // -----------------------------------------------------------------------
    // Phase 9c (FC) — single-RHS tuple destructure (`a, b = arr`).
    //
    // The grammar already accepts these shapes: `multi_assignment`
    // requires ≥2 LHS targets and ≥1 RHS expression.  Phase 9c just
    // turns on the lowering for the 1-RHS case.  These tests assert
    // the grammar still recognises the construct correctly so the SIR
    // lowerer's single-RHS dispatch has well-formed input.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_multi_assignment_single_rhs_two_lhs() {
        // `a, b = arr` — 2 LHS, 1 RHS, no splat.
        let ast = parse_ruby("a, b = arr");
        let m = find_descendant(&ast, "multi_assignment")
            .expect("expected multi_assignment node");
        let lhs_target_count = m
            .children
            .iter()
            .filter(|c| {
                matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "mlhs_target")
            })
            .count();
        assert_eq!(lhs_target_count, 2, "expected 2 mlhs_target nodes");
        // No `*` token anywhere — this is the no-splat shape.
        assert!(
            !tree_has_token_value(m, "*"),
            "single-RHS tuple destructure should have no splat"
        );
    }

    #[test]
    fn test_parse_multi_assignment_single_rhs_three_lhs() {
        // `a, b, c = arr` — 3 LHS, 1 RHS.
        let ast = parse_ruby("a, b, c = arr");
        let m = find_descendant(&ast, "multi_assignment")
            .expect("expected multi_assignment node");
        let lhs_target_count = m
            .children
            .iter()
            .filter(|c| {
                matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "mlhs_target")
            })
            .count();
        assert_eq!(lhs_target_count, 3, "expected 3 mlhs_target nodes");
        assert!(!tree_has_token_value(m, "*"));
    }

    #[test]
    fn test_parse_multi_assignment_single_rhs_keeps_one_rhs_expression() {
        // The grammar should put exactly ONE expression on the RHS for
        // the single-RHS case — `multi_assignment`'s RHS rule is
        // `expression { COMMA expression }`, so the trailing repetition
        // group must be empty here.
        let ast = parse_ruby("a, b = arr");
        let m = find_descendant(&ast, "multi_assignment")
            .expect("expected multi_assignment node");
        // Find the EQUALS token's index, then count expression nodes
        // appearing after it.
        let eq_idx = m
            .children
            .iter()
            .position(|c| {
                matches!(c, ASTNodeOrToken::Token(t) if t.value == "=")
            })
            .expect("expected EQUALS token in multi_assignment");
        let rhs_expr_count = m.children[eq_idx + 1..]
            .iter()
            .filter(|c| {
                matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression")
            })
            .count();
        assert_eq!(
            rhs_expr_count, 1,
            "expected exactly 1 RHS expression, got {}",
            rhs_expr_count
        );
    }

    #[test]
    fn test_parse_single_assignment_not_consumed_by_multi() {
        // Regression: `a = 1` (one LHS) must still parse as a plain
        // `assignment`, not as a malformed `multi_assignment`.  The
        // `multi_assignment` rule requires `NAME COMMA NAME` minimum,
        // so single-NAME inputs fall through cleanly.
        let ast = parse_ruby("a = 1");
        assert!(
            find_descendant(&ast, "assignment").is_some(),
            "expected `assignment` node for single-LHS form"
        );
        assert!(
            find_descendant(&ast, "multi_assignment").is_none(),
            "single-LHS must NOT parse as multi_assignment"
        );
    }

    // -----------------------------------------------------------------------
    // Phase 6s — splat / double-splat in params and call args
    //
    // Grammar additions:
    //   params   = param { COMMA param } ;
    //   param    = [ "*" | "**" ] NAME ;
    //   call_arg = [ "*" | "**" ] expression ;
    //   method_call's argument slot now uses call_arg instead of bare expression.
    //
    // method_call_no_paren intentionally still uses bare `expression` to
    // avoid ambiguity with binary `*` at expression-start position
    // (`a * b` would otherwise parse as `a(splat b)`).  Paren-less splat
    // is a v0 deferred limitation; users can always fall back to `f(*arr)`.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_splat_param() {
        // `def f(*args) end` — single splat param.
        let ast = parse_ruby("def f(*args)\nend");
        // The `param` subnode is present and carries the `*` token plus the name.
        let param = find_descendant(&ast, "param").expect("expected param subnode");
        assert!(tree_has_token_value(param, "*"), "expected `*` splat prefix");
        assert!(tree_has_token_value(param, "args"), "expected `args` name");
        // Regression: NO `**` prefix.
        assert!(!tree_has_token_value(param, "**"),
            "single splat must not produce a `**` token");
    }

    #[test]
    fn test_parse_double_splat_param() {
        // `def f(**kwargs) end` — single double-splat param.
        let ast = parse_ruby("def f(**kwargs)\nend");
        let param = find_descendant(&ast, "param").expect("expected param subnode");
        assert!(tree_has_token_value(param, "**"), "expected `**` double-splat prefix");
        assert!(tree_has_token_value(param, "kwargs"), "expected `kwargs` name");
    }

    #[test]
    fn test_parse_mixed_params_with_splats() {
        // `def f(a, *rest, **opts) end` — three params: positional, splat, double-splat.
        let ast = parse_ruby("def f(a, *rest, **opts)\nend");
        let params_node = find_descendant(&ast, "params").expect("expected params");
        let param_count = params_node
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "param"))
            .count();
        assert_eq!(param_count, 3, "expected 3 param subnodes, got {param_count}");
        // The whole tree should contain `*`, `**`, and the bare NAMEs.
        assert!(tree_has_token_value(params_node, "*"));
        assert!(tree_has_token_value(params_node, "**"));
        assert!(tree_has_token_value(params_node, "rest"));
        assert!(tree_has_token_value(params_node, "opts"));
    }

    #[test]
    fn test_parse_splat_call_arg() {
        // `f(*arr)` — splat call argument.
        let ast = parse_ruby("f(*arr)");
        let call_arg = find_descendant(&ast, "call_arg")
            .expect("expected call_arg subnode");
        assert!(tree_has_token_value(call_arg, "*"), "expected `*` in call_arg");
        assert!(tree_has_token_value(call_arg, "arr"), "expected `arr` in call_arg");
    }

    #[test]
    fn test_parse_mixed_call_args_with_splats() {
        // `f(1, *arr, **hsh)` — three call_args: positional, splat, double-splat.
        let ast = parse_ruby("f(1, *arr, **hsh)");
        let call = find_descendant(&ast, "method_call").expect("expected method_call");
        let arg_count = call
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "call_arg"))
            .count();
        assert_eq!(arg_count, 3, "expected 3 call_args, got {arg_count}");
        assert!(tree_has_token_value(call, "*"));
        assert!(tree_has_token_value(call, "**"));
    }

    #[test]
    fn test_parse_double_splat_only_call_arg() {
        // Phase 22a (coverage) — `f(**opts)` with the double-splat as the
        // SOLE argument.  Earlier pins only exercised `**` alongside other
        // args (`f(1, *arr, **hsh)`); this confirms a lone `**` arg still
        // produces exactly one `call_arg` carrying the `**` prefix.
        let ast = parse_ruby("f(**opts)");
        let call = find_descendant(&ast, "method_call").expect("expected method_call");
        let arg_count = call
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "call_arg"))
            .count();
        assert_eq!(arg_count, 1, "expected exactly 1 call_arg, got {arg_count}");
        let call_arg = find_descendant(&ast, "call_arg").expect("expected call_arg");
        assert!(tree_has_token_value(call_arg, "**"), "expected `**` prefix");
        assert!(tree_has_token_value(call_arg, "opts"), "expected `opts` name");
    }

    #[test]
    fn test_parse_double_splat_hash_literal_inner() {
        // Phase 22a (coverage) — `f(**{a: 1})`: the double-splat operand is
        // itself a hash literal, not a bare name.  Confirms the `call_arg`
        // expression slot accepts a `hash_literal` after the `**` prefix.
        let ast = parse_ruby("f(**{a: 1})");
        let call_arg = find_descendant(&ast, "call_arg").expect("expected call_arg");
        assert!(tree_has_token_value(call_arg, "**"), "expected `**` prefix");
        // The inner hash literal must appear beneath the same call_arg.
        let hash = find_descendant(call_arg, "hash_literal")
            .expect("expected hash_literal inside the double-splat call_arg");
        assert!(tree_has_token_value(hash, "a"), "expected key `a` in inner hash");
    }

    #[test]
    fn test_parse_double_splat_in_dot_call() {
        // Phase 22a (coverage) — `obj.merge(**opts)`: the double-splat rides
        // through a `dot_call` argument list (a distinct grammar path from
        // the head `method_call` args).  Both reuse `call_arg`, so `**`
        // must surface there too.
        let ast = parse_ruby("obj.merge(**opts)");
        let dot = find_descendant(&ast, "dot_call").expect("expected dot_call");
        let call_arg = find_descendant(dot, "call_arg")
            .expect("expected call_arg inside dot_call");
        assert!(tree_has_token_value(call_arg, "**"), "expected `**` in dot_call arg");
        assert!(tree_has_token_value(call_arg, "opts"), "expected `opts` name");
    }

    #[test]
    fn test_parse_block_pass_call_arg() {
        // Phase 22b — `f(&blk)`: a block-pass argument.  The `&` prefix
        // joins `*`/`**` in the `call_arg` rule.  Confirms exactly one
        // `call_arg` carrying the `&` prefix and the operand name.
        let ast = parse_ruby("f(&blk)");
        let call = find_descendant(&ast, "method_call").expect("expected method_call");
        let arg_count = call
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "call_arg"))
            .count();
        assert_eq!(arg_count, 1, "expected exactly 1 call_arg, got {arg_count}");
        let call_arg = find_descendant(&ast, "call_arg").expect("expected call_arg");
        assert!(tree_has_token_value(call_arg, "&"), "expected `&` prefix");
        assert!(tree_has_token_value(call_arg, "blk"), "expected `blk` name");
    }

    #[test]
    fn test_parse_block_pass_after_positional() {
        // Phase 22b — `f(1, &blk)`: a positional arg followed by a
        // block-pass.  Confirms `&` interleaves with ordinary args on
        // the COMMA separator (two call_args total).
        let ast = parse_ruby("f(1, &blk)");
        let call = find_descendant(&ast, "method_call").expect("expected method_call");
        let arg_count = call
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "call_arg"))
            .count();
        assert_eq!(arg_count, 2, "expected 2 call_args, got {arg_count}");
        assert!(tree_has_token_value(call, "&"), "expected `&` somewhere in the call");
        assert!(tree_has_token_value(call, "blk"), "expected `blk` name");
    }

    #[test]
    fn test_parse_block_pass_in_dot_call() {
        // Phase 22b — `arr.each(&blk)`: the block-pass rides through a
        // `dot_call` argument list (the idiomatic `&:sym` / `&proc`
        // higher-order-call form).  Both head and dot calls reuse
        // `call_arg`, so `&` must surface in the dot-call path too.
        let ast = parse_ruby("arr.each(&blk)");
        let dot = find_descendant(&ast, "dot_call").expect("expected dot_call");
        let call_arg = find_descendant(dot, "call_arg")
            .expect("expected call_arg inside dot_call");
        assert!(tree_has_token_value(call_arg, "&"), "expected `&` in dot_call arg");
        assert!(tree_has_token_value(call_arg, "blk"), "expected `blk` name");
    }

    #[test]
    fn test_parse_forward_all_call_arg() {
        // Phase 22c — `n(...)`: forward-all argument.  The bare `...`
        // matches the new `"..."` alternative of `call_arg` (the
        // expression branch fails because `...` cannot complete a
        // beginless range with no operand before `)`).  Confirms one
        // call_arg carrying the `...` token.
        let ast = parse_ruby("n(...)");
        let call = find_descendant(&ast, "method_call").expect("expected method_call");
        let arg_count = call
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "call_arg"))
            .count();
        assert_eq!(arg_count, 1, "expected exactly 1 call_arg, got {arg_count}");
        let call_arg = find_descendant(&ast, "call_arg").expect("expected call_arg");
        assert!(tree_has_token_value(call_arg, "..."), "expected `...` forward token");
    }

    #[test]
    fn test_parse_forward_all_param() {
        // Phase 22c — `def m(...)`: forward-all parameter declaration.
        // The bare `...` matches the new whole-`params` alternative.
        let ast = parse_ruby("def m(...)\n  puts(1)\nend");
        let params = find_descendant(&ast, "params").expect("expected params");
        assert!(tree_has_token_value(params, "..."), "expected `...` in params");
        // No `param` subnode is produced for the bare forward form.
        let param_count = params
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "param"))
            .count();
        assert_eq!(param_count, 0, "bare `...` params must have 0 `param` nodes");
    }

    #[test]
    fn test_parse_forward_all_roundtrip() {
        // Phase 22c — the canonical forwarding shape:
        //   def m(...)
        //     n(...)
        //   end
        // Both the param decl and the inner forwarding call must parse.
        let ast = parse_ruby("def m(...)\n  n(...)\nend");
        let params = find_descendant(&ast, "params").expect("expected params");
        assert!(tree_has_token_value(params, "..."), "expected `...` in params");
        let call = find_descendant(&ast, "method_call").expect("expected inner method_call");
        let call_arg = find_descendant(call, "call_arg").expect("expected forwarding call_arg");
        assert!(tree_has_token_value(call_arg, "..."), "expected `...` in inner call");
    }

    #[test]
    fn test_parse_beginless_range_arg_still_parses() {
        // Phase 22c regression — `m(...5)`: a beginless EXCLUSIVE-range
        // argument must STILL parse as a range (not as forward-all),
        // because the prefixed-expression branch is tried first and the
        // `...5` completes a `range`.  The `...` token lives nested in
        // the call_arg's expression subtree, and a `range` node exists.
        let ast = parse_ruby("m(...5)");
        let call_arg = find_descendant(&ast, "call_arg").expect("expected call_arg");
        assert!(
            find_descendant(call_arg, "range").is_some(),
            "expected a range node for the beginless-range argument"
        );
        assert!(tree_has_token_value(call_arg, "5"), "expected range endpoint `5`");
    }

    #[test]
    fn test_parse_super_bare() {
        // Phase 22d / Issue #59 — bare `super` (zsuper): no argument list.
        // Now parses as a `super_expr` (the statement-only `super_statement`
        // was folded into `factor` so `super` works in expression position)
        // with NO `super_args` child (the absence marks it as implicit-forward
        // zsuper).
        let ast = parse_ruby("super");
        let sup = find_descendant(&ast, "super_expr").expect("expected super_expr");
        assert!(tree_has_token_value(sup, "super"), "expected `super` keyword");
        assert!(
            find_descendant(sup, "super_args").is_none(),
            "bare super must have NO super_args node"
        );
    }

    #[test]
    fn test_parse_super_empty_parens() {
        // Phase 22d / Issue #59 — `super()`: explicit empty arg list.  A
        // `super_args` node IS present (with zero `call_arg` children),
        // distinguishing it from bare zsuper.
        let ast = parse_ruby("super()");
        let sup = find_descendant(&ast, "super_expr").expect("expected super_expr");
        let args = find_descendant(sup, "super_args").expect("expected super_args node");
        let arg_count = args
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "call_arg"))
            .count();
        assert_eq!(arg_count, 0, "super() must have 0 call_args");
    }

    #[test]
    fn test_parse_super_with_args() {
        // Phase 22d / Issue #59 — `super(x, y)`: explicit args.  `super_args`
        // holds two `call_arg` children.
        let ast = parse_ruby("super(x, y)");
        let sup = find_descendant(&ast, "super_expr").expect("expected super_expr");
        let args = find_descendant(sup, "super_args").expect("expected super_args node");
        let arg_count = args
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "call_arg"))
            .count();
        assert_eq!(arg_count, 2, "super(x, y) must have 2 call_args, got {arg_count}");
        assert!(tree_has_token_value(sup, "x"));
        assert!(tree_has_token_value(sup, "y"));
    }

    #[test]
    fn test_parse_binary_star_still_parses_as_expression() {
        // Regression: `a * b` as a statement must still parse as a
        // bare expression-stmt with binary `*`, NOT as `a(splat b)`.
        // method_call_no_paren intentionally keeps bare `expression`
        // args to preserve this behaviour.
        let ast = parse_ruby("a * b");
        // No call_arg node should appear in the AST.
        assert!(
            find_descendant(&ast, "call_arg").is_none(),
            "binary `a * b` must NOT produce a call_arg"
        );
        // A term node containing the `*` should exist.
        let term = find_descendant(&ast, "term").expect("expected term node");
        assert!(tree_has_token_value(term, "*"), "expected binary `*` in term");
    }

    // -----------------------------------------------------------------------
    // Phase 6t — `yield` keyword with optional args
    //
    // Grammar:
    //   yield_statement = "yield" [ yield_args ] ;
    //   yield_args      = LPAREN [ call_arg { COMMA call_arg } ] RPAREN
    //                   | call_arg { COMMA call_arg } ;
    //
    // Placed before the generic method_call_no_paren so `yield x`
    // doesn't fall through to keyword-led no-paren call lowering.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_bare_yield() {
        // `yield` alone — no args.
        let ast = parse_ruby("yield");
        let y = find_descendant(&ast, "yield_statement")
            .expect("expected yield_statement node");
        // Token "yield" present, no yield_args wrapper.
        assert!(tree_has_token_value(y, "yield"), "expected `yield` keyword");
        assert!(
            find_descendant(y, "yield_args").is_none(),
            "bare `yield` should not produce a yield_args wrapper"
        );
        assert!(
            find_descendant(y, "call_arg").is_none(),
            "bare `yield` should not produce a call_arg"
        );
    }

    #[test]
    fn test_parse_yield_with_paren_args() {
        // `yield(x, y)` — parens-form with two args.
        let ast = parse_ruby("yield(x, y)");
        let y = find_descendant(&ast, "yield_statement")
            .expect("expected yield_statement node");
        let ya = find_descendant(y, "yield_args").expect("expected yield_args");
        let arg_count = ya
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "call_arg"))
            .count();
        assert_eq!(arg_count, 2, "expected 2 call_args, got {arg_count}");
    }

    #[test]
    fn test_parse_yield_with_parenless_args() {
        // `yield x, y` — parenless form.
        let ast = parse_ruby("yield x, y");
        let y = find_descendant(&ast, "yield_statement")
            .expect("expected yield_statement node for parenless yield");
        let ya = find_descendant(y, "yield_args").expect("expected yield_args");
        let arg_count = ya
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "call_arg"))
            .count();
        assert_eq!(arg_count, 2, "expected 2 call_args (parenless), got {arg_count}");
        // Regression: NO LPAREN/RPAREN tokens in the args.
        assert!(!tree_has_token_value(ya, "("),
            "parenless form must not produce an LPAREN");
    }

    // -----------------------------------------------------------------------
    // Phase 6u — `case … when … else … end`
    //
    // Grammar:
    //   case_statement = "case" expression { when_clause } [ else_clause ] "end" ;
    //   when_clause    = "when" expression { COMMA expression }
    //                          { !"when" !"else" !"end" statement } ;
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Phase 6v — `begin … rescue … ensure … end`
    //
    // Grammar:
    //   begin_statement = "begin"
    //                     { !"rescue" !"ensure" !"end" statement }
    //                     { rescue_clause }
    //                     [ ensure_clause ]
    //                     "end" ;
    //   rescue_clause   = "rescue" [ exception_list ] [ "=>" NAME ]
    //                          { !"rescue" !"ensure" !"end" statement } ;
    //   exception_list  = NAME { COMMA NAME } ;
    //   ensure_clause   = "ensure" { !"end" statement } ;
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Phase 6w — arrow-lambda literal `->(params){body}`
    //
    // Grammar:
    //   lambda_literal = "->" [ LPAREN [ params ] RPAREN ] block ;
    //
    // Placed inside `factor` so lambdas are valid in any expression
    // position.  `lambda { … }` / `proc { … }` still flow through
    // `method_with_block` (no separate grammar).
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Phase 6x — instance var `@x`, class var `@@x`, global var `$x` refs
    //
    // The lexer (Phase 4i/4j) emits these as a SINGLE Name-typed token whose
    // value carries the leading sigil (`@a`, `@@all`, `$c`).  This means the
    // parser sees them as bare NAME tokens at the factor / assignment LHS
    // level — no new grammar rules are required.
    //
    // The SIR lowerer routes `$x` to `Scope::Global` and preserves the bare
    // value for `@x` / `@@x` (no native ivar/cvar scope in v0).
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_instance_var_assignment() {
        // `@a = 1` — ivar appears as a single NAME token at the assignment LHS.
        let ast = parse_ruby("@a = 1");
        let assn = find_descendant(&ast, "assignment")
            .expect("expected assignment node for `@a = 1`");
        // The LHS NAME value should include the `@` sigil.
        assert!(tree_has_token_value(assn, "@a"), "expected `@a` token in assignment");
    }

    #[test]
    fn test_parse_class_var_assignment() {
        // `@@all = 0` — cvar via lexer's `@@`-prefixed Name token.
        let ast = parse_ruby("@@all = 0");
        let assn = find_descendant(&ast, "assignment")
            .expect("expected assignment node for `@@all = 0`");
        assert!(
            tree_has_token_value(assn, "@@all"),
            "expected `@@all` token in assignment"
        );
    }

    #[test]
    fn test_parse_global_var_assignment() {
        // `$config = 1` — gvar via lexer's `$`-prefixed Name token.
        let ast = parse_ruby("$config = 1");
        let assn = find_descendant(&ast, "assignment")
            .expect("expected assignment node for `$config = 1`");
        assert!(
            tree_has_token_value(assn, "$config"),
            "expected `$config` token in assignment"
        );
    }

    #[test]
    fn test_parse_instance_var_in_expression() {
        // `puts(@a)` — ivar appears as a call_arg expression.
        let ast = parse_ruby("puts(@a)");
        assert!(
            tree_has_token_value(&ast, "@a"),
            "expected `@a` token in argument position"
        );
        assert!(
            find_descendant(&ast, "method_call").is_some(),
            "expected method_call wrapping the puts(@a) invocation"
        );
    }

    #[test]
    fn test_parse_arrow_lambda_no_params() {
        // `-> { 1 }` — no params, brace block.  We wrap in an
        // assignment to dodge the bare-NAME-led statement ambiguity.
        let ast = parse_ruby("f = -> { 1 }");
        let ll = find_descendant(&ast, "lambda_literal")
            .expect("expected lambda_literal node");
        assert!(tree_has_token_value(ll, "->"), "expected `->` token");
        // No params subnode.
        assert!(
            find_descendant(ll, "params").is_none(),
            "expected no params subnode for bare arrow"
        );
        assert!(find_descendant(ll, "block").is_some());
    }

    #[test]
    fn test_parse_arrow_lambda_with_params() {
        // `->(x, y) { x + y }` — two parens-params, brace block.
        let ast = parse_ruby("f = ->(x, y) { x + y }");
        let ll = find_descendant(&ast, "lambda_literal")
            .expect("expected lambda_literal node");
        let p = find_descendant(ll, "params").expect("expected params subnode");
        let param_count = p
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "param"))
            .count();
        assert_eq!(param_count, 2, "expected 2 params, got {param_count}");
    }

    #[test]
    fn test_parse_arrow_lambda_inside_call() {
        // `each(->(x) { x })` — arrow lambda as a call arg.
        let ast = parse_ruby("each(->(x) { x })");
        assert!(
            find_descendant(&ast, "lambda_literal").is_some(),
            "expected lambda_literal inside call args"
        );
    }

    #[test]
    fn test_parse_lambda_keyword_with_brace_block() {
        // `lambda { |x| x + 1 }` — keyword form, uses method_with_block.
        let ast = parse_ruby("lambda { |x| x + 1 }");
        // Should NOT be a lambda_literal (that's the arrow form);
        // it's a method_with_block instead.
        assert!(
            find_descendant(&ast, "lambda_literal").is_none(),
            "lambda keyword form should NOT parse as lambda_literal"
        );
        assert!(
            find_descendant(&ast, "method_with_block").is_some(),
            "expected method_with_block for lambda keyword form"
        );
    }

    #[test]
    fn test_parse_begin_with_rescue() {
        let ast = parse_ruby("begin\n  x = 1\nrescue\n  x = 2\nend");
        let b = find_descendant(&ast, "begin_statement")
            .expect("expected begin_statement node");
        assert!(
            find_descendant(b, "rescue_clause").is_some(),
            "expected rescue_clause"
        );
        assert!(
            find_descendant(b, "ensure_clause").is_none(),
            "expected no ensure_clause"
        );
    }

    #[test]
    fn test_parse_begin_with_rescue_typed_and_var() {
        // `rescue StandardError => e` — exception type and binding variable.
        let ast = parse_ruby(
            "begin\n  x = 1\nrescue StandardError => e\n  x = 2\nend",
        );
        let rc = find_descendant(&ast, "rescue_clause")
            .expect("expected rescue_clause");
        // Exception list present.
        assert!(
            find_descendant(rc, "exception_list").is_some(),
            "expected exception_list under rescue_clause"
        );
        // Arrow token present.
        assert!(tree_has_token_value(rc, "=>"), "expected `=>` token");
        // The variable `e` is somewhere in the rescue subtree.
        assert!(tree_has_token_value(rc, "e"), "expected `e` token");
    }

    #[test]
    fn test_parse_begin_multi_type_rescue() {
        // Phase 16b — `rescue Foo, Bar => e` parses with an
        // `exception_list` carrying BOTH class Name tokens.
        let ast = parse_ruby(
            "begin\n  x = 1\nrescue Foo, Bar => e\n  y = 2\nend",
        );
        let rc = find_descendant(&ast, "rescue_clause")
            .expect("expected rescue_clause");
        let el = find_descendant(rc, "exception_list")
            .expect("expected exception_list");
        assert!(tree_has_token_value(el, "Foo"), "expected `Foo` exception type");
        assert!(tree_has_token_value(el, "Bar"), "expected `Bar` exception type");
        assert!(tree_has_token_value(rc, "=>"), "expected `=>` token");
        assert!(tree_has_token_value(rc, "e"), "expected binding `e`");
    }

    #[test]
    fn test_parse_begin_multiple_rescue_clauses() {
        // Phase 16b — two `rescue` clauses parse as two distinct
        // `rescue_clause` nodes under the begin_statement.
        let ast = parse_ruby(concat!(
            "begin\n",
            "  x = 1\n",
            "rescue TypeError => e\n",
            "  y = 2\n",
            "rescue NameError => f\n",
            "  z = 3\n",
            "end",
        ));
        let b = find_descendant(&ast, "begin_statement")
            .expect("expected begin_statement node");
        let clause_count = b
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "rescue_clause"))
            .count();
        assert_eq!(clause_count, 2, "expected two rescue_clause nodes; got {}", clause_count);
    }

    #[test]
    fn test_parse_begin_with_ensure() {
        let ast = parse_ruby("begin\n  x = 1\nensure\n  cleanup = 1\nend");
        let b = find_descendant(&ast, "begin_statement")
            .expect("expected begin_statement node");
        assert!(
            find_descendant(b, "ensure_clause").is_some(),
            "expected ensure_clause"
        );
    }

    #[test]
    fn test_parse_begin_with_rescue_and_ensure() {
        // Full form: body + rescue + ensure.
        let ast = parse_ruby(
            "begin\n  x = 1\nrescue\n  x = 2\nensure\n  x = 3\nend",
        );
        let b = find_descendant(&ast, "begin_statement")
            .expect("expected begin_statement node");
        let rc_count = b
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "rescue_clause"))
            .count();
        assert_eq!(rc_count, 1, "expected 1 rescue_clause, got {rc_count}");
        assert!(
            find_descendant(b, "ensure_clause").is_some(),
            "expected ensure_clause"
        );
    }

    #[test]
    fn test_parse_begin_ensure_multiple_statements() {
        // Phase 16c — an ensure clause with several statements collects
        // them all under the `ensure_clause` node (negative-lookahead
        // repetition stops at `end`).
        let ast = parse_ruby(
            "begin\n  x = 1\nensure\n  a = 1\n  b = 2\nend",
        );
        let ec = find_descendant(&ast, "ensure_clause")
            .expect("expected ensure_clause");
        let stmt_count = ec
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "statement"))
            .count();
        assert_eq!(stmt_count, 2, "expected 2 ensure-body statements; got {stmt_count}");
    }

    #[test]
    fn test_parse_raise_with_class_and_message() {
        // Phase 16d — `raise Foo, "boom"` parses as a paren-less method
        // call carrying the `raise` head Name and both arguments.
        let ast = parse_ruby("raise Foo, \"boom\"");
        let mc = find_descendant(&ast, "method_call_no_paren")
            .expect("expected method_call_no_paren");
        assert!(body_has_token_value(mc, "raise"), "expected `raise` head token");
        assert!(tree_has_token_value(mc, "Foo"), "expected exception class `Foo`");
    }

    #[test]
    fn test_parse_case_single_when() {
        // Smallest form: one when clause, no else.
        let ast = parse_ruby("case x\nwhen 1\n  y = 1\nend");
        let cs = find_descendant(&ast, "case_statement")
            .expect("expected case_statement node");
        let when_count = cs
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "when_clause"))
            .count();
        assert_eq!(when_count, 1, "expected 1 when_clause, got {when_count}");
        // No else_clause.
        assert!(
            find_descendant(cs, "else_clause").is_none(),
            "expected no else_clause"
        );
    }

    #[test]
    fn test_parse_case_multiple_whens_and_else() {
        let ast = parse_ruby(
            "case x\nwhen 1\n  a = 1\nwhen 2\n  a = 2\nelse\n  a = 3\nend",
        );
        let cs = find_descendant(&ast, "case_statement")
            .expect("expected case_statement node");
        let when_count = cs
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "when_clause"))
            .count();
        assert_eq!(when_count, 2, "expected 2 when_clauses, got {when_count}");
        assert!(
            find_descendant(cs, "else_clause").is_some(),
            "expected else_clause"
        );
    }

    #[test]
    fn test_parse_when_with_multiple_values() {
        // `when 1, 2, 3` — comma-separated value list inside one clause.
        let ast = parse_ruby("case x\nwhen 1, 2, 3\n  a = 1\nend");
        let wc = find_descendant(&ast, "when_clause")
            .expect("expected when_clause node");
        let value_count = wc
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression"))
            .count();
        assert_eq!(value_count, 3, "expected 3 when values, got {value_count}");
    }

    #[test]
    fn test_parse_yield_with_splat_arg() {
        // `yield *arr` — splat arg reuses Phase 6s's call_arg shape.
        let ast = parse_ruby("yield(*arr)");
        let y = find_descendant(&ast, "yield_statement")
            .expect("expected yield_statement node");
        let ca = find_descendant(y, "call_arg").expect("expected call_arg");
        assert!(tree_has_token_value(ca, "*"), "expected `*` splat prefix in yield arg");
        assert!(tree_has_token_value(ca, "arr"));
    }

    // -----------------------------------------------------------------------
    // Phase 6y — string interpolation expression parsing
    //
    // The lexer's Phase-3b state machine emits `"hello #{name}"` as a single
    // `TokenType::String` token whose value carries the `#{...}` markers
    // verbatim.  These tests merely confirm that the existing grammar
    // (STRING token at factor position) accepts interpolated forms in every
    // statement / expression context the lowerer will touch: assignment
    // RHS, argument position, and bare expression.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_interpolated_string_assignment() {
        // `x = "hello #{name}"` — interpolation on the RHS of an
        // assignment.  The whole literal is one STRING token, so the
        // grammar treats it identically to a plain string.
        let ast = parse_ruby(r##"x = "hello #{name}""##);
        assert!(
            find_descendant(&ast, "assignment").is_some(),
            "expected an assignment node for `x = \"...#{{name}}\"`"
        );
        // The raw token value (with `#{...}` preserved) must be present
        // in the tree.
        assert!(
            tree_has_token_value(&ast, "hello #{name}"),
            "expected the verbatim interpolated string token in the parse tree"
        );
    }

    #[test]
    fn test_parse_interpolated_string_in_call_arg() {
        // `puts("sum=#{1+2}")` — interpolation body is an arbitrary
        // expression; lexer brace-tracking keeps the `{...}` balanced.
        let ast = parse_ruby(r##"puts("sum=#{1+2}")"##);
        assert!(
            find_descendant(&ast, "method_call").is_some(),
            "expected a method_call node wrapping puts(...)"
        );
        assert!(
            tree_has_token_value(&ast, "sum=#{1+2}"),
            "expected the verbatim interpolation-bearing string token"
        );
    }

    #[test]
    fn test_parse_interpolated_string_only_interp() {
        // `x = "#{name}"` — the entire string is a single `#{...}`.
        // Lowering should emit only the interp expression with no
        // surrounding literal text; here we only verify the grammar
        // accepts it.
        let ast = parse_ruby(r##"x = "#{name}""##);
        assert!(
            find_descendant(&ast, "assignment").is_some(),
            "expected an assignment node"
        );
        assert!(
            tree_has_token_value(&ast, "#{name}"),
            "expected `#{{name}}` token value preserved verbatim"
        );
    }

    #[test]
    fn test_parse_interpolated_string_multiple_segments() {
        // `x = "a=#{a}, b=#{b}"` — multiple `#{...}` interp markers
        // with literal text bridging them.  Verifies the lexer's
        // brace tracking handles back-to-back interps and the grammar
        // accepts the resulting single token.
        let ast = parse_ruby(r##"x = "a=#{a}, b=#{b}""##);
        assert!(find_descendant(&ast, "assignment").is_some());
        assert!(tree_has_token_value(&ast, "a=#{a}, b=#{b}"));
    }

    // -----------------------------------------------------------------------
    // Phase 6z — float / hex / bin / oct integer literal parsing
    //
    // The lexer's Phase-4k float fusion produces a single `Number` token for
    // `1.5`, `1e10`, `1.5e-3`, etc., and Phase-4l's radix fusion produces a
    // single `Number` token for `0x1F`, `0b1010`, `0o17`, `0d42`.  The
    // parser sees them all uniformly at the factor position, so this phase
    // is grammar-zero: these tests just confirm the existing NUMBER-at-
    // factor handling accepts every shape and the SIR lowerer routes them.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_float_literal_assignment() {
        // `x = 1.5` — float on RHS.  Lexer's float fusion collapses
        // `1`/`.`/`5` into a single `Number("1.5")` token.
        let ast = parse_ruby("x = 1.5");
        assert!(
            find_descendant(&ast, "assignment").is_some(),
            "expected assignment node for float RHS"
        );
        assert!(
            tree_has_token_value(&ast, "1.5"),
            "expected fused `1.5` NUMBER token in the tree"
        );
    }

    #[test]
    fn test_parse_float_literal_with_exponent() {
        // `x = 1.5e-3` — fractional + signed exponent.  Tests the
        // full lexer fusion path through `Number`+`Name("e")`+`Op("-")`+`Int`.
        let ast = parse_ruby("x = 1.5e-3");
        assert!(find_descendant(&ast, "assignment").is_some());
        assert!(
            tree_has_token_value(&ast, "1.5e-3"),
            "expected fused `1.5e-3` NUMBER token"
        );
    }

    #[test]
    fn test_parse_hex_integer_literal() {
        // `x = 0xDEAD_BEEF` — hex literal with underscore separator.
        // Lexer Phase 4l fuses `Int("0")`+`Name("xDEAD_BEEF")` into a
        // single `Number("0xDEAD_BEEF")` token.
        let ast = parse_ruby("x = 0xDEAD_BEEF");
        assert!(find_descendant(&ast, "assignment").is_some());
        assert!(
            tree_has_token_value(&ast, "0xDEAD_BEEF"),
            "expected fused hex literal in tree"
        );
    }

    #[test]
    fn test_parse_binary_integer_literal() {
        // `x = 0b1010` — binary literal.  Same fusion mechanism as hex.
        let ast = parse_ruby("x = 0b1010");
        assert!(find_descendant(&ast, "assignment").is_some());
        assert!(tree_has_token_value(&ast, "0b1010"));
    }

    #[test]
    fn test_parse_octal_integer_literal() {
        // `x = 0o17` — octal literal.  Decimal value 15.
        let ast = parse_ruby("x = 0o17");
        assert!(find_descendant(&ast, "assignment").is_some());
        assert!(tree_has_token_value(&ast, "0o17"));
    }

    // -----------------------------------------------------------------------
    // Phase 7a — backtick command literals in parser
    //
    // The lexer's Phase-4m backtick_body state emits the whole `` `cmd` ``
    // literal as a single `TokenType::String` token whose value is the
    // body wrapped back in backticks (`` `body` ``).  The parser sees
    // this as a regular STRING token at the factor position — no grammar
    // changes needed.  These tests confirm the grammar accepts every
    // statement context (assignment RHS, call arg, bare expression).
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_backtick_command_literal_assignment() {
        // `x = `ls -la`` — backtick on RHS of an assignment.
        let ast = parse_ruby("x = `ls -la`");
        assert!(
            find_descendant(&ast, "assignment").is_some(),
            "expected assignment node for backtick RHS"
        );
        assert!(
            tree_has_token_value(&ast, "`ls -la`"),
            "expected backtick-wrapped lexeme `\\`ls -la\\`` in tree"
        );
    }

    #[test]
    fn test_parse_backtick_command_literal_in_call_arg() {
        // `puts(`pwd`)` — backtick as a method-call argument.
        let ast = parse_ruby("puts(`pwd`)");
        assert!(
            find_descendant(&ast, "method_call").is_some(),
            "expected method_call node"
        );
        assert!(tree_has_token_value(&ast, "`pwd`"));
    }

    #[test]
    fn test_parse_empty_backtick_command_literal() {
        // `x = ``` — empty body.  Lexer's backtick_body fuses both
        // backticks into a single Token whose value is "``".
        let ast = parse_ruby("x = ``");
        assert!(find_descendant(&ast, "assignment").is_some());
        assert!(
            tree_has_token_value(&ast, "``"),
            "expected empty-body backtick lexeme"
        );
    }

    // -----------------------------------------------------------------------
    // Phase 7b — heredocs in parser
    //
    // The lexer's Phase-3c heredoc body capture + Phase-4o opener-variant
    // handling finalise every heredoc (`<<EOF`, `<<-EOF`, `<<~EOF`) into
    // a single `TokenType::String` token whose value is the verbatim
    // canonical form (`<<TAG\n<body>TAG`).  The parser sees the heredoc
    // as a regular STRING token at the factor position — no grammar
    // changes needed.  These tests confirm the grammar accepts every
    // variant in assignment-RHS context.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_plain_heredoc_assignment() {
        // `x = <<EOF\nhello\nEOF\n` — plain heredoc on RHS.
        let ast = parse_ruby("x = <<EOF\nhello\nEOF\n");
        assert!(
            find_descendant(&ast, "assignment").is_some(),
            "expected assignment node"
        );
        assert!(
            tree_has_token_value(&ast, "<<EOF\nhello\nEOF"),
            "expected canonical heredoc lexeme in tree"
        );
    }

    #[test]
    fn test_parse_dash_indent_heredoc_assignment() {
        // `x = <<-EOF\nhello\n  EOF` — `<<-` indent-tolerant heredoc:
        // closing tag may be indented (lexer Phase 4o).
        let ast = parse_ruby("x = <<-EOF\nhello\n  EOF\n");
        assert!(find_descendant(&ast, "assignment").is_some());
        // The lexer's canonicalisation strips the indentation from the
        // closing tag in the *value*, so the in-tree lexeme is the
        // canonical form.
        assert!(
            tree_has_token_value(&ast, "<<-EOF\nhello\nEOF"),
            "expected canonical <<- heredoc lexeme in tree"
        );
    }

    #[test]
    fn test_parse_tilde_indent_heredoc_assignment() {
        // `x = <<~EOF\n  hello\n  EOF` — `<<~` indent-stripping heredoc.
        // The lexer strips common leading indent from each body line
        // before re-wrapping into the canonical token form.
        let ast = parse_ruby("x = <<~EOF\n  hello\n  EOF\n");
        assert!(find_descendant(&ast, "assignment").is_some());
        // Lexer stripped the common 2-space indent from `  hello`.
        assert!(
            tree_has_token_value(&ast, "<<~EOF\nhello\nEOF"),
            "expected canonical <<~ heredoc with indent stripped"
        );
    }

    // -----------------------------------------------------------------------
    // Phase 7c — Ruby 3.0 endless method definitions `def foo = expr`
    //
    // Grammar adds:
    //   endless_def_statement = "def" NAME [ LPAREN [ params ] RPAREN ] EQUALS expression ;
    //
    // Placed BEFORE def_statement in the statement alternation so PEG tries
    // the endless form first; if `=` isn't present the parser falls through
    // to the block-bodied def.
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Phase 23b — `defined?` operator
    //
    // The lexer emits `defined?` as a single KEYWORD token (trailing `?`
    // included).  Grammar: `defined_expression = "defined?" factor`,
    // placed first in the `factor` alternation so the keyword wins over
    // the bare-KEYWORD alternative.  Covers both the parenthesised form
    // `defined?(x)` and the bare tight form `defined? x`.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_defined_with_parens() {
        // `x = defined?(y)` — parenthesised operand on an assignment RHS.
        let ast = parse_ruby("x = defined?(y)\n");
        assert!(
            find_descendant(&ast, "assignment").is_some(),
            "expected assignment node"
        );
        assert!(
            find_descendant(&ast, "defined_expression").is_some(),
            "expected defined_expression node"
        );
        assert!(
            tree_has_token_value(&ast, "defined?"),
            "expected the `defined?` keyword token in the tree"
        );
    }

    #[test]
    fn test_parse_defined_without_parens() {
        // `x = defined? y` — bare tight form (operand is a NAME factor).
        let ast = parse_ruby("x = defined? y\n");
        let d = find_descendant(&ast, "defined_expression")
            .expect("expected defined_expression node");
        // The operand `y` is present as a Name token under the node.
        assert!(
            tree_has_token_value(d, "y"),
            "expected operand `y` under defined_expression"
        );
    }

    #[test]
    fn test_parse_defined_statement_position() {
        // `defined?(x)` as a bare expression statement (no assignment).
        let ast = parse_ruby("defined?(x)\n");
        assert!(
            find_descendant(&ast, "defined_expression").is_some(),
            "expected defined_expression node in statement position"
        );
    }

    // -----------------------------------------------------------------------
    // Phase 24a (FC) — `alias new old` method aliasing.  The `alias`
    // keyword leads a statement with two bare method-name (NAME) operands.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_alias_basic() {
        // `alias foo bar` — the canonical two-name form.
        let ast = parse_ruby("alias foo bar\n");
        assert!(
            find_descendant(&ast, "alias_statement").is_some(),
            "expected alias_statement node"
        );
        assert!(
            tree_has_token_value(&ast, "alias"),
            "expected the `alias` keyword token in the tree"
        );
    }

    #[test]
    fn test_parse_alias_carries_both_names() {
        // Both operands must appear under the alias_statement node.
        let ast = parse_ruby("alias size length\n");
        let a = find_descendant(&ast, "alias_statement")
            .expect("expected alias_statement node");
        assert!(
            tree_has_token_value(a, "size"),
            "expected new-name operand `size` under alias_statement"
        );
        assert!(
            tree_has_token_value(a, "length"),
            "expected old-name operand `length` under alias_statement"
        );
    }

    #[test]
    fn test_parse_alias_not_shadowed_by_method_call() {
        // The leading `alias` keyword must win over the bare-KEYWORD
        // `factor` alternative — i.e. `alias` is parsed as an
        // alias_statement (consuming BOTH operands), not as a method_call
        // / expression_stmt that swallows only `alias` and leaves the
        // names dangling.  We assert the alias_statement node is present
        // and that no `method_call` node captured the `alias` keyword.
        let ast = parse_ruby("alias quux corge\n");
        assert!(
            find_descendant(&ast, "alias_statement").is_some(),
            "expected alias_statement node (alias must not be shadowed)"
        );
        assert!(
            tree_has_token_value(&ast, "corge"),
            "expected old-name operand `corge` to be consumed by alias_statement"
        );
    }

    #[test]
    fn test_parse_undef_basic() {
        // `undef foo` — the canonical single-name form.
        let ast = parse_ruby("undef foo\n");
        assert!(
            find_descendant(&ast, "undef_statement").is_some(),
            "expected undef_statement node"
        );
        assert!(
            tree_has_token_value(&ast, "undef"),
            "expected the `undef` keyword token in the tree"
        );
    }

    #[test]
    fn test_parse_undef_carries_name() {
        // The single operand must appear under the undef_statement node.
        let ast = parse_ruby("undef obsolete\n");
        let u = find_descendant(&ast, "undef_statement")
            .expect("expected undef_statement node");
        assert!(
            tree_has_token_value(u, "obsolete"),
            "expected name operand `obsolete` under undef_statement"
        );
    }

    #[test]
    fn test_parse_undef_not_shadowed_by_method_call() {
        // The leading `undef` keyword must win over the bare-KEYWORD
        // `factor` alternative — i.e. `undef` is parsed as an
        // undef_statement (consuming the name operand), not as a
        // method_call / expression_stmt that swallows only `undef` and
        // leaves the name dangling.
        let ast = parse_ruby("undef quux\n");
        assert!(
            find_descendant(&ast, "undef_statement").is_some(),
            "expected undef_statement node (undef must not be shadowed)"
        );
        assert!(
            tree_has_token_value(&ast, "quux"),
            "expected name operand `quux` to be consumed by undef_statement"
        );
    }

    #[test]
    fn test_parse_file_keyword_as_factor() {
        // Phase 23a (FC) — `__FILE__` is NOT a lexer keyword (it starts
        // with `_`, so it lexes as an ordinary NAME) and needs no grammar
        // rule: it parses through `factor`'s bare-NAME alternative.  This
        // pin confirms the `__FILE__` token survives into the tree when it
        // appears as a standalone expression statement.
        let ast = parse_ruby("__FILE__\n");
        assert!(
            tree_has_token_value(&ast, "__FILE__"),
            "expected `__FILE__` token to be present in the parse tree"
        );
    }

    #[test]
    fn test_parse_file_keyword_in_call_arg() {
        // `puts(__FILE__)` — `__FILE__` parses as a call argument
        // expression (factor → NAME), confirming it composes in argument
        // position exactly like any other bare name.
        let ast = parse_ruby("puts(__FILE__)\n");
        assert!(
            tree_has_token_value(&ast, "__FILE__"),
            "expected `__FILE__` token in the `puts(__FILE__)` call args"
        );
    }

    #[test]
    fn test_parse_file_keyword_in_assignment_rhs() {
        // `path = __FILE__` — `__FILE__` parses as the assignment RHS
        // expression, the same NAME-factor path a normal variable read
        // would take.  We assert both the LHS name and the `__FILE__`
        // token are present.
        let ast = parse_ruby("path = __FILE__\n");
        assert!(
            tree_has_token_value(&ast, "path"),
            "expected LHS name `path` in the assignment"
        );
        assert!(
            tree_has_token_value(&ast, "__FILE__"),
            "expected `__FILE__` token as the assignment RHS"
        );
    }

    #[test]
    fn test_parse_line_keyword_as_factor() {
        // Phase 23c (FC) — `__LINE__`, like `__FILE__`, is NOT a lexer
        // keyword (it starts with `_`, so it lexes as an ordinary NAME) and
        // needs no grammar rule: it parses through `factor`'s bare-NAME
        // alternative.  This pin confirms the token survives into the tree.
        let ast = parse_ruby("__LINE__\n");
        assert!(
            tree_has_token_value(&ast, "__LINE__"),
            "expected `__LINE__` token to be present in the parse tree"
        );
    }

    #[test]
    fn test_parse_line_keyword_in_call_arg() {
        // `puts(__LINE__)` — `__LINE__` parses as a call argument
        // expression (factor → NAME).
        let ast = parse_ruby("puts(__LINE__)\n");
        assert!(
            tree_has_token_value(&ast, "__LINE__"),
            "expected `__LINE__` token in the `puts(__LINE__)` call args"
        );
    }

    #[test]
    fn test_parse_line_keyword_in_assignment_rhs() {
        // `n = __LINE__` — `__LINE__` parses as the assignment RHS
        // expression, the same NAME-factor path a normal variable read
        // would take.
        let ast = parse_ruby("n = __LINE__\n");
        assert!(
            tree_has_token_value(&ast, "n"),
            "expected LHS name `n` in the assignment"
        );
        assert!(
            tree_has_token_value(&ast, "__LINE__"),
            "expected `__LINE__` token as the assignment RHS"
        );
    }

    #[test]
    fn test_parse_dir_keyword_as_factor() {
        // Phase 23d (FC) — `__dir__`, like `__FILE__` / `__LINE__`, is NOT
        // a lexer keyword (it lexes as an ordinary NAME) and needs no
        // grammar rule: it parses through `factor`'s bare-NAME alternative.
        let ast = parse_ruby("__dir__\n");
        assert!(
            tree_has_token_value(&ast, "__dir__"),
            "expected `__dir__` token to be present in the parse tree"
        );
    }

    #[test]
    fn test_parse_dir_keyword_in_call_arg() {
        // `puts(__dir__)` — `__dir__` parses as a call argument expression
        // (factor → NAME).
        let ast = parse_ruby("puts(__dir__)\n");
        assert!(
            tree_has_token_value(&ast, "__dir__"),
            "expected `__dir__` token in the `puts(__dir__)` call args"
        );
    }

    #[test]
    fn test_parse_dir_keyword_in_assignment_rhs() {
        // `d = __dir__` — `__dir__` parses as the assignment RHS
        // expression, the same NAME-factor path a normal variable read
        // would take.
        let ast = parse_ruby("d = __dir__\n");
        assert!(
            tree_has_token_value(&ast, "d"),
            "expected LHS name `d` in the assignment"
        );
        assert!(
            tree_has_token_value(&ast, "__dir__"),
            "expected `__dir__` token as the assignment RHS"
        );
    }

    #[test]
    fn test_parse_using_is_method_call_no_paren() {
        // Phase 26a (FC) — `using Mod` is an ordinary paren-less method
        // call (`using` is a method, not a keyword): it parses as a
        // `method_call_no_paren` with the module as its sole argument.
        let ast = parse_ruby("using Foo\n");
        assert!(
            find_descendant(&ast, "method_call_no_paren").is_some(),
            "expected `using Foo` to parse as method_call_no_paren"
        );
    }

    #[test]
    fn test_parse_using_carries_callee_and_module() {
        // Both the `using` callee and the `Foo` module operand must
        // appear in the parse tree.
        let ast = parse_ruby("using Foo\n");
        assert!(
            tree_has_token_value(&ast, "using"),
            "expected the `using` callee token in the tree"
        );
        assert!(
            tree_has_token_value(&ast, "Foo"),
            "expected the `Foo` module operand token in the tree"
        );
    }

    #[test]
    fn test_parse_using_scoped_module() {
        // `using Foo::Bar` — the operand may be a scoped constant; the
        // scope-resolution tokens survive into the tree.
        let ast = parse_ruby("using Foo::Bar\n");
        assert!(
            find_descendant(&ast, "method_call_no_paren").is_some(),
            "expected `using Foo::Bar` to parse as method_call_no_paren"
        );
        assert!(
            tree_has_token_value(&ast, "Bar"),
            "expected the scoped `Bar` operand token in the tree"
        );
    }

    #[test]
    fn test_parse_refine_is_method_with_block() {
        // Phase 26b (FC) — `refine(Class) do ... end` is a block-taking
        // method call: it parses as a `method_with_block` with the target
        // class as a parenned argument and the refinement body as a block.
        let ast = parse_ruby("refine(String) do\n  1\nend\n");
        assert!(
            find_descendant(&ast, "method_with_block").is_some(),
            "expected `refine(String) do…end` to parse as method_with_block"
        );
    }

    #[test]
    fn test_parse_refine_carries_callee_and_class() {
        // Both the `refine` callee and the `String` target-class operand
        // must appear in the parse tree.
        let ast = parse_ruby("refine(String) do\n  1\nend\n");
        assert!(
            tree_has_token_value(&ast, "refine"),
            "expected the `refine` callee token in the tree"
        );
        assert!(
            tree_has_token_value(&ast, "String"),
            "expected the `String` target-class token in the tree"
        );
    }

    #[test]
    fn test_parse_refine_has_block_subnode() {
        // The trailing `do … end` must parse into a `block` subnode under
        // the method_with_block (where the refinement body lives).
        let ast = parse_ruby("refine(String) do\n  1\nend\n");
        let mwb = find_descendant(&ast, "method_with_block")
            .expect("expected method_with_block node");
        assert!(
            find_descendant(mwb, "block").is_some(),
            "expected a `block` subnode under the refine method_with_block"
        );
    }

    #[test]
    fn test_parse_endless_def_no_params() {
        // `def hello = 1` — endless method with no parameters.
        let ast = parse_ruby("def hello = 1");
        let d = find_descendant(&ast, "endless_def_statement")
            .expect("expected endless_def_statement node");
        // The method-name Name token should be present.
        assert!(tree_has_token_value(d, "hello"));
        // No params subnode for the bare form.
        assert!(
            find_descendant(d, "params").is_none(),
            "expected no params subnode for `def hello = 1`"
        );
    }

    #[test]
    fn test_parse_endless_def_with_params() {
        // `def add(x, y) = x + y` — endless method with two parameters.
        let ast = parse_ruby("def add(x, y) = x + y");
        let d = find_descendant(&ast, "endless_def_statement")
            .expect("expected endless_def_statement node");
        let p = find_descendant(d, "params").expect("expected params subnode");
        let param_count = p
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "param"))
            .count();
        assert_eq!(param_count, 2, "expected 2 params, got {param_count}");
        // The body expression should be present.
        assert!(find_descendant(d, "expression").is_some());
    }

    #[test]
    fn test_parse_endless_def_does_not_break_block_def() {
        // Regression: putting `endless_def_statement` first in the
        // alternation must not break the existing block-bodied def
        // form when there's no `=` after the signature.
        let ast = parse_ruby("def greet\n  puts(1)\nend");
        assert!(
            find_descendant(&ast, "def_statement").is_some(),
            "expected def_statement (block-bodied) — endless variant must fall through"
        );
        // The endless form should NOT have matched.
        assert!(
            find_descendant(&ast, "endless_def_statement").is_none(),
            "block-bodied def must not match endless_def_statement"
        );
    }

    // -----------------------------------------------------------------------
    // Phase 7d — Ruby 3.0 `case/in` pattern matching
    //
    // Grammar extends case_statement to accept either `when_clause` or
    // `in_clause` repetitions in any source order:
    //   case_statement = "case" expression { when_clause | in_clause }
    //                    [ else_clause ] "end" ;
    //   in_clause      = "in" pattern { … statement } ;
    //   pattern        = array_pattern | hash_pattern
    //                  | literal_pattern | binding_pattern ;
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_pin_pattern() {
        // Phase FC — `in ^x` parses as a `pin_pattern` carrying the
        // pinned name `x`.
        let ast = parse_ruby("case y\nin ^x\n  puts(1)\nend");
        let cs = find_descendant(&ast, "case_statement").expect("case_statement");
        let inc = find_descendant(cs, "in_clause").expect("in_clause");
        let pat = find_descendant(inc, "pattern").expect("pattern");
        assert!(
            find_descendant(pat, "pin_pattern").is_some(),
            "expected pin_pattern for `in ^x`"
        );
        assert!(tree_has_token_value(pat, "x"), "expected pinned name `x`");
    }

    #[test]
    fn test_parse_class_pattern() {
        // Phase FC — `in Foo(a)` parses as a `class_pattern` with the
        // class name and an inner positional pattern.
        let ast = parse_ruby("case y\nin Foo(a)\n  puts(1)\nend");
        let cs = find_descendant(&ast, "case_statement").expect("case_statement");
        let inc = find_descendant(cs, "in_clause").expect("in_clause");
        let pat = find_descendant(inc, "pattern").expect("pattern");
        let cp = find_descendant(pat, "class_pattern").expect("expected class_pattern");
        assert!(tree_has_token_value(cp, "Foo"), "expected class name `Foo`");
        assert!(tree_has_token_value(cp, "a"), "expected inner operand `a`");
    }

    #[test]
    fn test_parse_bare_constant_is_binding_not_class_pattern() {
        // A bare constant `Foo` (no parens) must still parse as a
        // `binding_pattern`, not a `class_pattern` — confirming the
        // `NAME LPAREN` requirement and that class_pattern doesn't
        // shadow the bare-NAME form.
        let ast = parse_ruby("case y\nin Foo\n  puts(1)\nend");
        let cs = find_descendant(&ast, "case_statement").expect("case_statement");
        let inc = find_descendant(cs, "in_clause").expect("in_clause");
        let pat = find_descendant(inc, "pattern").expect("pattern");
        assert!(
            find_descendant(pat, "class_pattern").is_none(),
            "bare `Foo` must not parse as class_pattern"
        );
        assert!(
            find_descendant(pat, "binding_pattern").is_some(),
            "bare `Foo` should parse as binding_pattern"
        );
    }

    #[test]
    fn test_parse_case_in_with_literal_pattern() {
        // `case x; in 1; puts("one"); end` — literal-pattern clause.
        let ast = parse_ruby("case x\nin 1\n  puts(\"one\")\nend");
        let cs = find_descendant(&ast, "case_statement")
            .expect("expected case_statement node");
        let inc = find_descendant(cs, "in_clause")
            .expect("expected in_clause for `in 1`");
        let pat = find_descendant(inc, "pattern").expect("expected pattern node");
        assert!(
            find_descendant(pat, "literal_pattern").is_some(),
            "expected literal_pattern for `in 1`"
        );
    }

    #[test]
    fn test_parse_case_in_with_binding_pattern() {
        // `case x; in y; puts(y); end` — bare-name binding pattern.
        let ast = parse_ruby("case x\nin y\n  puts(y)\nend");
        let cs = find_descendant(&ast, "case_statement").expect("case_statement");
        let inc = find_descendant(cs, "in_clause").expect("in_clause");
        let pat = find_descendant(inc, "pattern").expect("pattern");
        assert!(
            find_descendant(pat, "binding_pattern").is_some(),
            "expected binding_pattern for `in y`"
        );
    }

    #[test]
    fn test_parse_case_in_with_array_pattern() {
        // `case x; in [1, 2]; puts("pair"); end` — array pattern.
        let ast = parse_ruby("case x\nin [1, 2]\n  puts(\"pair\")\nend");
        let cs = find_descendant(&ast, "case_statement").expect("case_statement");
        let inc = find_descendant(cs, "in_clause").expect("in_clause");
        let pat = find_descendant(inc, "pattern").expect("pattern");
        assert!(
            find_descendant(pat, "array_pattern").is_some(),
            "expected array_pattern for `in [1, 2]`"
        );
    }

    #[test]
    fn test_parse_array_pattern_with_named_splat() {
        // Phase FC — `in [a, *rest, b]` parses with a `splat_pattern`
        // carrying the rest name, plus the fixed elements.
        let ast = parse_ruby("case y\nin [a, *rest, b]\n  puts(1)\nend");
        let pat = find_descendant(&ast, "array_pattern").expect("array_pattern");
        assert!(
            find_descendant(pat, "splat_pattern").is_some(),
            "expected splat_pattern in `[a, *rest, b]`"
        );
        for tok in ["a", "rest", "b"] {
            assert!(tree_has_token_value(pat, tok), "expected `{tok}` in pattern");
        }
    }

    #[test]
    fn test_parse_array_find_pattern_two_splats() {
        // `in [*, x, *]` (find pattern) parses with two splat_patterns.
        let ast = parse_ruby("case y\nin [*, x, *]\n  puts(1)\nend");
        let pat = find_descendant(&ast, "array_pattern").expect("array_pattern");
        let splats = count_descendants(pat, "splat_pattern");
        assert_eq!(splats, 2, "expected two splat_patterns in find pattern");
        assert!(tree_has_token_value(pat, "x"), "expected the `x` element");
    }

    #[test]
    fn test_parse_array_anonymous_splat() {
        // `in [10, *]` — a trailing anonymous splat parses.
        let ast = parse_ruby("case y\nin [10, *]\n  puts(1)\nend");
        let pat = find_descendant(&ast, "array_pattern").expect("array_pattern");
        assert!(
            find_descendant(pat, "splat_pattern").is_some(),
            "expected splat_pattern for trailing `*`"
        );
    }

    #[test]
    fn test_parse_case_in_with_hash_pattern() {
        // `case x; in {name: y}; puts(y); end` — hash pattern.
        let ast = parse_ruby("case x\nin {name: y}\n  puts(y)\nend");
        let cs = find_descendant(&ast, "case_statement").expect("case_statement");
        let inc = find_descendant(cs, "in_clause").expect("in_clause");
        let pat = find_descendant(inc, "pattern").expect("pattern");
        assert!(
            find_descendant(pat, "hash_pattern").is_some(),
            "expected hash_pattern for `in {{name: y}}`"
        );
    }

    // -----------------------------------------------------------------------
    // Phase 7e — Ruby 3.0 rightward assignment `expr => var`
    //
    // Grammar adds:
    //   rightward_assignment = expression "=>" NAME ;
    //
    // Placed AFTER modifier_statement and BEFORE assignment in the
    // statement alternation.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_rightward_assignment_with_literal() {
        // `1 => x` — value `1` is bound to `x`.
        let ast = parse_ruby("1 => x");
        let ra = find_descendant(&ast, "rightward_assignment")
            .expect("expected rightward_assignment node");
        assert!(
            tree_has_token_value(ra, "x"),
            "expected binding name `x` in rightward_assignment"
        );
        assert!(
            find_descendant(ra, "expression").is_some(),
            "expected expression node holding the value"
        );
    }

    #[test]
    fn test_parse_rightward_assignment_with_binary_expression() {
        // `1 + 2 => sum` — the LHS expression is a binary +.
        let ast = parse_ruby("1 + 2 => sum");
        let ra = find_descendant(&ast, "rightward_assignment")
            .expect("expected rightward_assignment node");
        assert!(tree_has_token_value(ra, "sum"));
        // The expression child must hold the binary form (sum node).
        let expr = find_descendant(ra, "expression").expect("expression");
        assert!(
            find_descendant(expr, "sum").is_some(),
            "expected sum node inside rightward_assignment expression"
        );
    }

    #[test]
    fn test_parse_rightward_assignment_with_call() {
        // `foo(1, 2) => result` — call as LHS value.
        let ast = parse_ruby("foo(1, 2) => result");
        let ra = find_descendant(&ast, "rightward_assignment")
            .expect("expected rightward_assignment node");
        assert!(tree_has_token_value(ra, "result"));
        let expr = find_descendant(ra, "expression").expect("expression");
        // The expression contains a method_call somewhere inside.
        assert!(
            find_descendant(expr, "method_call").is_some(),
            "expected method_call inside the LHS expression"
        );
    }

    #[test]
    fn test_parse_rightward_assignment_does_not_break_normal_assignment() {
        // Regression: `x = 1` must still match `assignment`, not
        // rightward_assignment (which requires `=>`).
        let ast = parse_ruby("x = 1");
        assert!(
            find_descendant(&ast, "assignment").is_some(),
            "expected normal assignment to still match"
        );
        assert!(
            find_descendant(&ast, "rightward_assignment").is_none(),
            "normal assignment must NOT match rightward_assignment"
        );
    }

    #[test]
    fn test_parse_case_when_still_works_after_in_clause_addition() {
        // Regression: extending the case_statement rule to accept `in_clause`
        // alongside `when_clause` must not break the original Phase-6u
        // `case … when … end` form.
        let ast = parse_ruby("case x\nwhen 1\n  puts(1)\nwhen 2\n  puts(2)\nend");
        let cs = find_descendant(&ast, "case_statement").expect("case_statement");
        // Two when_clauses, zero in_clauses.
        let when_count = cs
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "when_clause"))
            .count();
        let in_count = cs
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "in_clause"))
            .count();
        assert_eq!(when_count, 2, "expected 2 when_clauses");
        assert_eq!(in_count, 0, "expected 0 in_clauses");
    }

    // -----------------------------------------------------------------------
    // Phase 7f — Ruby 3.1 hash value-omitted shorthand `{x:, y:}`
    //
    // When a hash entry is written as `NAME COLON` (no value expression),
    // Ruby 3.1+ treats it as a punned shorthand for `NAME COLON NAME`,
    // i.e. the value is a local variable lookup with the same name as
    // the symbol key.  Grammar accepts this via a new alternation
    // `NAME COLON` placed AFTER `NAME COLON expression` so the parser
    // tries the longer form first (PEG ordered-choice semantics).
    //
    // The three tests below cover (1) pure shorthand `{x:, y:}`, (2)
    // mixed shorthand + explicit forms `{x:, y: 5}`, and (3) regression
    // that ordinary `{x: 1, y: 2}` still parses with two `expression`
    // children per entry (i.e. case (1) parses to ZERO `expression`
    // children — the value-omitted shape — not one).
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_hash_value_shorthand_pure() {
        // `{x:, y:}` — two value-omitted entries.  Each `hash_entry`
        // should have a Name token + COLON token + NO `expression`
        // subnode (the new value-omitted shape).
        let ast = parse_ruby("h = {x:, y:}");
        let h = find_descendant(&ast, "hash_literal").expect("expected hash_literal");
        let entries: Vec<&GrammarASTNode> = h
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "hash_entry" => Some(n),
                _ => None,
            })
            .collect();
        assert_eq!(entries.len(), 2, "expected 2 hash_entry subnodes");
        for ent in &entries {
            let expr_count = ent
                .children
                .iter()
                .filter(|c| {
                    matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression")
                })
                .count();
            assert_eq!(
                expr_count, 0,
                "value-omitted shorthand: expected 0 expression children, got {expr_count}"
            );
        }
    }

    #[test]
    fn test_parse_hash_value_shorthand_mixed() {
        // `{x:, y: 5}` — first entry is value-omitted, second is
        // explicit.  Test that the parser correctly distinguishes the
        // two shapes within a single hash literal.
        let ast = parse_ruby("h = {x:, y: 5}");
        let h = find_descendant(&ast, "hash_literal").expect("expected hash_literal");
        let entries: Vec<&GrammarASTNode> = h
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "hash_entry" => Some(n),
                _ => None,
            })
            .collect();
        assert_eq!(entries.len(), 2, "expected 2 hash_entry subnodes");
        let expr_counts: Vec<usize> = entries
            .iter()
            .map(|ent| {
                ent.children
                    .iter()
                    .filter(|c| {
                        matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression")
                    })
                    .count()
            })
            .collect();
        assert_eq!(
            expr_counts,
            vec![0, 1],
            "expected first entry value-omitted (0 expr), second explicit (1 expr)"
        );
    }

    #[test]
    fn test_parse_hash_value_shorthand_regression_existing_form() {
        // Regression: extending hash_entry with `NAME COLON` must NOT
        // break the existing `{x: 1, y: 2}` form.  Each entry must
        // still parse with exactly one `expression` child (the value).
        let ast = parse_ruby("h = {x: 1, y: 2}");
        let h = find_descendant(&ast, "hash_literal").expect("expected hash_literal");
        let entries: Vec<&GrammarASTNode> = h
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "hash_entry" => Some(n),
                _ => None,
            })
            .collect();
        assert_eq!(entries.len(), 2);
        for ent in &entries {
            let expr_count = ent
                .children
                .iter()
                .filter(|c| {
                    matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression")
                })
                .count();
            assert_eq!(
                expr_count, 1,
                "explicit shorthand: expected 1 expression child, got {expr_count}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Phase 8a (FC) — additional arithmetic / bitwise / shift op-assigns
    //
    // The `assignment` rule was extended to recognise `%=`, `**=`, `<<=`,
    // `&=`, `|=`, `^=` in addition to the pre-existing
    // `+= -= *= /= ||= &&=`.  These tests assert the grammar accepts the
    // new shapes and routes through the `assignment` rule.  Lowering
    // semantics are tested in the `ruby-to-semantic-ir` crate.
    //
    // `>>=` is NOT exercised here — the 1.8-era lexer state machine
    // emits `>>` as two `>` tokens, so the compound-fusion pass can't
    // pre-fuse `>>=` yet.  That's a tracked follow-up; see Phase 8a's
    // PR body for the deferred-limitation note.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_modulo_op_assign() {
        // `x %= 5` should parse as a single `assignment` node carrying
        // the fused `%=` operator token.
        let ast = parse_ruby("x %= 5");
        let a = find_descendant(&ast, "assignment").expect("expected assignment");
        let has_op = a.children.iter().any(|c| {
            matches!(c, ASTNodeOrToken::Token(t) if t.value == "%=")
        });
        assert!(has_op, "expected `%=` token under assignment");
    }

    #[test]
    fn test_parse_power_and_shift_op_assigns() {
        // Two assignments in one program — `x **= 2`, `x <<= 1` —
        // each parses as a separate `assignment` node, and each
        // carries its respective fused operator token.
        let ast = parse_ruby("x **= 2\nx <<= 1");
        let stmts: Vec<&GrammarASTNode> = ast
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => Some(n),
                _ => None,
            })
            .collect();
        assert_eq!(stmts.len(), 2, "expected 2 statements");
        let ops: Vec<String> = stmts
            .iter()
            .filter_map(|s| {
                let a = find_descendant(s, "assignment")?;
                a.children.iter().find_map(|c| match c {
                    ASTNodeOrToken::Token(t) if t.value == "**=" || t.value == "<<=" => {
                        Some(t.value.clone())
                    }
                    _ => None,
                })
            })
            .collect();
        assert_eq!(
            ops,
            vec!["**=".to_string(), "<<=".to_string()],
            "expected `**=` then `<<=`"
        );
    }

    #[test]
    fn test_parse_bitwise_op_assigns() {
        // All three bitwise compound forms parse cleanly.
        for src in &["x &= 7", "x |= 7", "x ^= 7"] {
            let ast = parse_ruby(src);
            let a = find_descendant(&ast, "assignment")
                .unwrap_or_else(|| panic!("expected assignment in {src:?}"));
            assert!(
                a.children.iter().any(|c| matches!(c, ASTNodeOrToken::Token(t) if {
                    let v = t.value.as_str();
                    v == "&=" || v == "|=" || v == "^="
                })),
                "expected bitwise op-assign token under assignment in {src:?}"
            );
        }
    }

    #[test]
    fn test_parse_plain_assignment_still_works_after_8a() {
        // Regression — extending the `assignment` operator alternation
        // must not break the original `x = 5` form.
        let ast = parse_ruby("x = 5");
        assert!(find_descendant(&ast, "assignment").is_some());
    }

    // -----------------------------------------------------------------------
    // Phase 8a-2 (FC) — right-shift compound assign `>>=`.
    //
    // The lexer now pre-fuses `>>` and `>>=` into single Name tokens,
    // so the parser's `assignment` rule accepts `>>=` the same way it
    // already accepts `<<=`.
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_right_shift_op_assign() {
        let ast = parse_ruby("x >>= 1");
        let a = find_descendant(&ast, "assignment").expect("expected assignment");
        let has_op = a.children.iter().any(|c| {
            matches!(c, ASTNodeOrToken::Token(t) if t.value == ">>=")
        });
        assert!(has_op, "expected `>>=` token under assignment");
    }

    #[test]
    fn test_parse_left_and_right_shift_op_assigns_round_trip() {
        // Two statements, `x <<= 1` and `x >>= 1`, each parses to a
        // separate assignment with its respective fused operator token.
        let ast = parse_ruby("x <<= 1\nx >>= 1");
        let stmts: Vec<&GrammarASTNode> = ast
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => Some(n),
                _ => None,
            })
            .collect();
        assert_eq!(stmts.len(), 2);
        let ops: Vec<String> = stmts
            .iter()
            .filter_map(|s| {
                let a = find_descendant(s, "assignment")?;
                a.children.iter().find_map(|c| match c {
                    ASTNodeOrToken::Token(t) if t.value == "<<=" || t.value == ">>=" => {
                        Some(t.value.clone())
                    }
                    _ => None,
                })
            })
            .collect();
        assert_eq!(ops, vec!["<<=".to_string(), ">>=".to_string()]);
    }

    // -----------------------------------------------------------------------
    // Phase KW7 (Ruby 1.0 unblock) — keyword parameters & arguments.
    //
    // Grammar additions:
    //   param    = [ "*" | "**" ] NAME [ COLON [ expression ] | EQUALS expression ] ;
    //   call_arg = NAME COLON expression | [ "*" | "**" | "&" ] expression ;
    //
    // These tests pin the PARSE-tree shape: a keyword param carries a COLON
    // token child (and, for the optional form, a trailing `expression`); a
    // keyword call arg is a `call_arg` node with a NAME token + COLON token +
    // `expression` child.
    // -----------------------------------------------------------------------

    /// Collect the `param` subnodes of the first `def_statement`'s `params`.
    fn def_param_nodes(ast: &GrammarASTNode) -> Vec<&GrammarASTNode> {
        let def = find_def_statement(ast).expect("expected def_statement");
        let params = def
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "params" => Some(n),
                _ => None,
            })
            .expect("expected params subnode");
        params
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "param" => Some(n),
                _ => None,
            })
            .collect()
    }

    /// Does this `param` node carry a single-colon token (⇒ keyword param)?
    fn param_has_colon(param: &GrammarASTNode) -> bool {
        param.children.iter().any(|c| matches!(
            c,
            ASTNodeOrToken::Token(t)
                if matches!(t.type_, lexer::token::TokenType::Colon) && t.value == ":"
        ))
    }

    /// Does this `param` node carry a trailing `expression` (⇒ has a default)?
    fn param_has_expression(param: &GrammarASTNode) -> bool {
        param
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression"))
    }

    #[test]
    fn test_parse_required_keyword_param() {
        // `def f(a:)` — a required keyword param: COLON present, no default.
        let ast = parse_ruby("def f(a:)\nend");
        let params = def_param_nodes(&ast);
        assert_eq!(params.len(), 1);
        assert!(param_has_colon(params[0]), "keyword param must carry a colon");
        assert!(
            !param_has_expression(params[0]),
            "required keyword param must have NO default expression"
        );
    }

    #[test]
    fn test_parse_optional_keyword_param() {
        // `def f(a: 1)` — an optional keyword param: COLON + default expr.
        let ast = parse_ruby("def f(a: 1)\nend");
        let params = def_param_nodes(&ast);
        assert_eq!(params.len(), 1);
        assert!(param_has_colon(params[0]), "keyword param must carry a colon");
        assert!(
            param_has_expression(params[0]),
            "optional keyword param must carry a default expression"
        );
    }

    #[test]
    fn test_parse_mixed_positional_and_keyword_params() {
        // `def f(a, b:, c: 2)` — one positional required, one required
        // keyword, one optional keyword.  Pins the per-param shape.
        let ast = parse_ruby("def f(a, b:, c: 2)\nend");
        let params = def_param_nodes(&ast);
        assert_eq!(params.len(), 3, "expected three params");
        // a — positional: no colon, no default.
        assert!(!param_has_colon(params[0]));
        assert!(!param_has_expression(params[0]));
        // b — required keyword: colon, no default.
        assert!(param_has_colon(params[1]));
        assert!(!param_has_expression(params[1]));
        // c — optional keyword: colon + default.
        assert!(param_has_colon(params[2]));
        assert!(param_has_expression(params[2]));
    }

    #[test]
    fn test_parse_keyword_call_arg() {
        // `f(x: 1)` — a keyword call arg.  The `call_arg` node must carry a
        // NAME token, a COLON token, and an `expression` child.
        let ast = parse_ruby("f(x: 1)");
        let call_arg = find_descendant(&ast, "call_arg").expect("expected call_arg");
        let name = call_arg.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t)
                if matches!(t.type_, lexer::token::TokenType::Name) =>
            {
                Some(t.value.clone())
            }
            _ => None,
        });
        assert_eq!(name.as_deref(), Some("x"), "keyword arg name must be `x`");
        assert!(
            call_arg.children.iter().any(|c| matches!(
                c,
                ASTNodeOrToken::Token(t)
                    if matches!(t.type_, lexer::token::TokenType::Colon) && t.value == ":"
            )),
            "keyword call arg must carry a colon token"
        );
        assert!(
            call_arg
                .children
                .iter()
                .any(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "expression")),
            "keyword call arg must carry a value expression"
        );
    }

    #[test]
    fn test_parse_positional_then_keyword_call_args() {
        // `f(1, y: 2)` — a positional arg followed by a keyword arg.  Two
        // `call_arg` nodes: the first has NO colon (positional), the second
        // has a colon (keyword).
        let ast = parse_ruby("f(1, y: 2)");
        let method_call =
            find_descendant(&ast, "method_call").expect("expected method_call");
        let call_args: Vec<&GrammarASTNode> = method_call
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "call_arg" => Some(n),
                _ => None,
            })
            .collect();
        assert_eq!(call_args.len(), 2, "expected two call_args");
        let colon = |ca: &GrammarASTNode| {
            ca.children.iter().any(|c| matches!(
                c,
                ASTNodeOrToken::Token(t)
                    if matches!(t.type_, lexer::token::TokenType::Colon) && t.value == ":"
            ))
        };
        assert!(!colon(call_args[0]), "first arg is positional (no colon)");
        assert!(colon(call_args[1]), "second arg is a keyword (colon)");
    }

    // -----------------------------------------------------------------------
    // Issue #59 — class-method definitions `def self.m` / `def Foo.m`
    // -----------------------------------------------------------------------

    /// `def self.zero; end` parses to a `def_statement` carrying a
    /// `def_receiver` whose singleton receiver is `self`.
    #[test]
    fn test_parse_def_self_method() {
        let ast = parse_ruby("def self.zero\nend");
        let def = find_descendant(&ast, "def_statement").expect("expected def_statement");
        let recv = find_descendant(def, "def_receiver").expect("expected def_receiver");
        // The receiver holds `self` (a KEYWORD token) via singleton_receiver.
        let sr = find_descendant(recv, "singleton_receiver").expect("singleton_receiver");
        let has_self = sr.children.iter().any(|c| matches!(
            c,
            ASTNodeOrToken::Token(t) if t.value == "self"
        ));
        assert!(has_self, "def self.m receiver must be `self`");
    }

    /// `def Foo.bar(x); end` parses to a `def_statement` with a `def_receiver`
    /// naming the constant `Foo` and a normal parameter list.
    #[test]
    fn test_parse_def_const_method_with_params() {
        let ast = parse_ruby("def Foo.bar(x)\nend");
        let def = find_descendant(&ast, "def_statement").expect("expected def_statement");
        let recv = find_descendant(def, "def_receiver").expect("expected def_receiver");
        let sr = find_descendant(recv, "singleton_receiver").expect("singleton_receiver");
        let has_foo = sr.children.iter().any(|c| matches!(
            c,
            ASTNodeOrToken::Token(t) if t.value == "Foo"
        ));
        assert!(has_foo, "def Foo.bar receiver must be `Foo`");
        // The parameter `x` is still parsed under a `params` node.
        assert!(find_descendant(def, "params").is_some(), "expected params");
    }

    /// A plain `def m` (no receiver) must NOT grow a `def_receiver` node —
    /// the optional prefix cleanly matches nothing (regression guard).
    #[test]
    fn test_parse_def_no_receiver_unchanged() {
        let ast = parse_ruby("def zero\n  0\nend");
        let def = find_descendant(&ast, "def_statement").expect("expected def_statement");
        assert!(
            find_descendant(def, "def_receiver").is_none(),
            "plain `def m` must have NO def_receiver node"
        );
    }

    /// Endless class-method form `def self.zero = 0`.
    #[test]
    fn test_parse_endless_def_self_method() {
        let ast = parse_ruby("def self.zero = 0");
        let def =
            find_descendant(&ast, "endless_def_statement").expect("expected endless_def_statement");
        assert!(
            find_descendant(def, "def_receiver").is_some(),
            "def self.zero = 0 must carry a def_receiver"
        );
    }

    // -----------------------------------------------------------------------
    // Issue #59 — `super` as an expression (`super_expr`)
    // -----------------------------------------------------------------------

    /// `x = super` parses `super` in expression position as a `super_expr`
    /// (the RHS of the assignment), not a statement-level `super_statement`.
    #[test]
    fn test_parse_super_as_assignment_rhs() {
        let ast = parse_ruby("x = super");
        let sup = find_descendant(&ast, "super_expr").expect("expected super_expr");
        assert!(
            find_descendant(sup, "super_args").is_none(),
            "bare super (expr) must have NO super_args node"
        );
    }

    /// `super + 1` parses as a `sum` whose left operand is a bare
    /// `super_expr` — the `+` cannot begin a `call_arg`, so `super` stays
    /// bare and the `+ 1` applies to its produced value.
    #[test]
    fn test_parse_super_plus_expr() {
        let ast = parse_ruby("super + 1");
        let sup = find_descendant(&ast, "super_expr").expect("expected super_expr");
        assert!(
            find_descendant(sup, "super_args").is_none(),
            "`super + 1` — super must be bare (no super_args)"
        );
        // The `+ 1` lives outside super_expr, under a `sum`.
        assert!(find_descendant(&ast, "sum").is_some(), "expected a sum node");
    }

    /// `puts(super)` — `super` used as a call argument (deep in expression
    /// position) parses as a `super_expr`.
    #[test]
    fn test_parse_super_as_call_arg() {
        let ast = parse_ruby("puts(super)");
        assert!(
            find_descendant(&ast, "super_expr").is_some(),
            "super inside puts(...) must parse as super_expr"
        );
    }

    /// `super + \" tail\"` (the P3 execution-proof shape) parses with a bare
    /// `super_expr` on the left of a `sum`.
    #[test]
    fn test_parse_super_string_concat() {
        let ast = parse_ruby("super + \" tail\"");
        assert!(
            find_descendant(&ast, "super_expr").is_some(),
            "expected super_expr"
        );
    }

    // -------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening) -- see MAX_RULE_DEPTH's own
    // doc comment for the measurement.
    // -------------------------------------------------------------------

    fn nested_paren_source(n: usize) -> String {
        format!("x = {}1{}", "(".repeat(n), ")".repeat(n))
    }

    fn try_parse(src: &str) -> Result<GrammarASTNode, String> {
        create_ruby_parser(src).parse().map_err(|e| e.to_string())
    }

    /// Deeply-nested input must not overflow the native stack on a
    /// default-stack thread -- the whole point of the guard.
    #[test]
    fn test_deeply_nested_input_does_not_overflow_on_default_stack() {
        let src = nested_paren_source(5000);
        let handle = std::thread::spawn(move || {
            let _ = try_parse(&src);
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }

    /// Reasonable, hand-writable nesting stays well under the cap.
    #[test]
    fn test_reasonable_nesting_stays_under_the_cap() {
        assert!(try_parse(&nested_paren_source(10)).is_ok());
    }
}

