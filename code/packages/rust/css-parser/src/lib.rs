//! CSS parser backed by compiled parser grammar.

use coding_adventures_css_lexer::tokenize_css;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Recursion-depth cap for the CSS [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep recursion through `parse_rule` can overflow the
/// *native* thread stack — an uncatchable process abort — before this
/// crate's own callers ever get a chance to report anything). Before this
/// constant was added, `create_css_parser` never called `with_max_depth` at
/// all, leaving every caller (including this crate's own `parse_css`)
/// exposed to a native-stack-overflow DoS from adversarial input.
///
/// # Five recursion shapes, measured independently
///
/// `css.grammar` has five distinct self-referential productions (each one
/// a genuine cycle of rule calls, not an EBNF `{ x }` repetition — those
/// cost zero native stack regardless of width, confirmed in
/// `reduce-parser`'s own `MAX_RULE_DEPTH` doc comment via a throwaway
/// probe grammar). Each was measured with the same methodology every
/// sibling `*-parser` crate uses: binary search directly over candidate
/// `with_max_depth` values against a fixed 5000-level/link adversarial
/// input of that shape (deep enough that the *cap itself*, not the input's
/// finite length, is what triggers first) on a default-~2MiB-stack worker
/// thread in a debug build — this single measurement directly yields the
/// rule-frame floor `MAX_RULE_DEPTH` needs to respect, without a separate
/// nesting-count-to-frame-count conversion step.
///
/// 1. **Nested qualified-rule blocks** (CSS Nesting), `.a{.a{.a{...}}}` —
///    `block -> block_contents -> block_item -> declaration_or_nested ->
///    qualified_rule -> block -> …`. Safe at **289**, crashes at **290**.
/// 2. **Nested `@supports`/`@media` parenthesised conditions**,
///    `@supports (((...)))` — `at_prelude_token -> paren_block ->
///    at_prelude_tokens -> at_prelude_token -> …`. Safe at **289**, crashes
///    at **290**.
/// 3. **Nested `calc()`/function calls in a value**, `calc(calc(calc(...)))`
///    — `function_args -> function_arg -> FUNCTION function_args RPAREN ->
///    …`. Safe at **248**, crashes at **249**.
/// 4. **Nested `:not()` pseudo-class arguments**, `:not(:not(:not(...)))`
///    — `pseudo_class_args -> pseudo_class_arg -> FUNCTION
///    pseudo_class_args RPAREN -> …`. Safe at **248**, crashes at **249**.
/// 5. **Nested `@media` at-rule blocks**, `@media{@media{@media{...}}}` —
///    `at_rule -> block -> block_contents -> block_item -> at_rule -> …`
///    — the **binding** shape. Safe at **247**, crashes at **248**.
///
/// `MAX_RULE_DEPTH` is set to **170** — about 31% below the binding
/// nested-`@media` floor of 247 (comparable margin to `apl-parser`'s own
/// ~26.5%, `j-parser`'s ~30%, `reduce-parser`'s ~28.5%), and therefore
/// safely below all four other floors (248, 248, 289, 289) as well.
///
/// Measured real-input headroom at `170` (using the CAPPED parser, so no
/// crash risk at all — confirmed directly, not extrapolated from the
/// floors above, since different shapes cost different amounts of native
/// stack per rule-frame and headroom does not scale uniformly with the
/// floor ratios): nested qualified rules parse cleanly to 33 levels (34
/// trips); nested `@media` at-rules parse cleanly to 39 levels (40 trips);
/// `@supports` paren nesting parses cleanly to 55 levels (56 trips);
/// `calc()` nesting parses cleanly to 79 levels (80 trips); `:not()`
/// nesting parses cleanly to 81 levels (82 trips) — all comfortably
/// beyond anything a hand-written stylesheet needs, and all five
/// independently confirmed not to crash a default-stack thread even
/// thousands of levels/links past the cap (see this crate's tests).
const MAX_RULE_DEPTH: usize = 170;

pub fn create_css_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_css(source);
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH)
}

pub fn parse_css(source: &str) -> GrammarASTNode {
    let mut parser = create_css_parser(source);
    parser
        .parse()
        .unwrap_or_else(|e| panic!("CSS parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn assert_stylesheet_root(ast: &GrammarASTNode) {
        assert_eq!(
            ast.rule_name, "stylesheet",
            "Expected root rule 'stylesheet', got '{}'",
            ast.rule_name
        );
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
    // Test 1: Simple rule
    // -----------------------------------------------------------------------

    /// A basic CSS rule with a type selector and one declaration.
    #[test]
    fn test_parse_simple_rule() {
        let ast = parse_css("body { color: red; }");
        assert_stylesheet_root(&ast);
        assert!(!ast.children.is_empty(), "AST should have children");
    }

    // -----------------------------------------------------------------------
    // Test 2: Multiple declarations
    // -----------------------------------------------------------------------

    /// A rule with multiple declarations separated by semicolons.
    #[test]
    fn test_parse_multiple_declarations() {
        let ast = parse_css("h1 { color: blue; font-size: 24px; }");
        assert_stylesheet_root(&ast);
        assert!(!ast.children.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 3: Multiple rules
    // -----------------------------------------------------------------------

    /// A stylesheet with multiple rules.
    #[test]
    fn test_parse_multiple_rules() {
        let source = "h1 { color: red; } p { margin: 0; }";
        let ast = parse_css(source);
        assert_stylesheet_root(&ast);

        let has_rule = find_rule(&ast, "rule");
        assert!(has_rule, "Expected 'rule' nodes in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 4: Class selector
    // -----------------------------------------------------------------------

    /// CSS class selectors begin with a dot (.) followed by an identifier.
    #[test]
    fn test_parse_class_selector() {
        let ast = parse_css(".highlight { background: yellow; }");
        assert_stylesheet_root(&ast);
        assert!(!ast.children.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 5: ID selector
    // -----------------------------------------------------------------------

    /// CSS ID selectors use a hash (#) followed by an identifier.
    #[test]
    fn test_parse_id_selector() {
        let ast = parse_css("#main { width: 960px; }");
        assert_stylesheet_root(&ast);
        assert!(!ast.children.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 6: At-rule
    // -----------------------------------------------------------------------

    /// At-rules like @media introduce conditional blocks.
    #[test]
    fn test_parse_at_rule() {
        let ast = parse_css("@media screen { body { color: black; } }");
        assert_stylesheet_root(&ast);

        let has_at_rule = find_rule(&ast, "at_rule");
        assert!(has_at_rule, "Expected 'at_rule' in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 7: Empty stylesheet
    // -----------------------------------------------------------------------

    /// An empty stylesheet should parse to a stylesheet node with no children.
    #[test]
    fn test_parse_empty_stylesheet() {
        let ast = parse_css("");
        assert_stylesheet_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 8: Factory function
    // -----------------------------------------------------------------------

    /// The `create_css_parser` factory function should return a working
    /// `GrammarParser` that can successfully parse CSS.
    #[test]
    fn test_create_parser() {
        let mut parser = create_css_parser("a { }");
        let result = parser.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());

        let ast = result.unwrap();
        assert_eq!(ast.rule_name, "stylesheet");
    }

    // -----------------------------------------------------------------------
    // Test 9: Selector with combinators
    // -----------------------------------------------------------------------

    /// Descendant combinator (space) between selectors.
    #[test]
    fn test_parse_descendant_selector() {
        let ast = parse_css("div p { color: green; }");
        assert_stylesheet_root(&ast);
        assert!(!ast.children.is_empty());
    }

    // -----------------------------------------------------------------------
    // Test 10: Whitespace handling
    // -----------------------------------------------------------------------

    /// CSS allows arbitrary whitespace between tokens. The parser should
    /// handle prettified and minified CSS identically.
    #[test]
    fn test_parse_with_whitespace() {
        let prettified = "body {\n  color: red;\n  margin: 0;\n}";
        let ast = parse_css(prettified);
        assert_stylesheet_root(&ast);
        assert!(!ast.children.is_empty());
    }

    // -----------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening) -- exercises all five
    // independently-measured shapes documented on `MAX_RULE_DEPTH`.
    // -----------------------------------------------------------------------

    fn nested_qualified_rule_source(n: usize) -> String {
        let mut s = String::new();
        for _ in 0..n {
            s.push_str(".a{");
        }
        s.push_str("color:red;");
        for _ in 0..n {
            s.push('}');
        }
        s
    }

    fn nested_at_rule_source(n: usize) -> String {
        let mut s = String::new();
        for _ in 0..n {
            s.push_str("@media{");
        }
        s.push_str("a{color:red;}");
        for _ in 0..n {
            s.push('}');
        }
        s
    }

    fn nested_calc_source(n: usize) -> String {
        let mut s = String::from("a{width:");
        for _ in 0..n {
            s.push_str("calc(");
        }
        s.push('1');
        for _ in 0..n {
            s.push(')');
        }
        s.push_str(";}");
        s
    }

    fn nested_not_source(n: usize) -> String {
        let mut s = String::new();
        for _ in 0..n {
            s.push_str(":not(");
        }
        s.push('a');
        for _ in 0..n {
            s.push(')');
        }
        s.push_str("{color:red;}");
        s
    }

    fn nested_supports_paren_source(n: usize) -> String {
        let mut s = String::from("@supports ");
        for _ in 0..n {
            s.push('(');
        }
        s.push_str("a:1");
        for _ in 0..n {
            s.push(')');
        }
        s.push_str("{a{color:red;}}");
        s
    }

    fn try_parse(src: &str) -> Result<GrammarASTNode, String> {
        create_css_parser(src).parse().map_err(|e| e.to_string())
    }

    /// Deeply-nested input, for every measured shape, must produce a
    /// recoverable error, not overflow the native stack. Parses 5000
    /// levels/links -- far past `MAX_RULE_DEPTH` -- on a worker thread
    /// with a generous 32 MiB stack, so the *guard* is what stops the
    /// recursion, not the stack running out.
    #[test]
    fn test_deeply_nested_input_returns_error_not_overflow_for_every_shape() {
        let sources = vec![
            nested_qualified_rule_source(5000),
            nested_at_rule_source(5000),
            nested_calc_source(5000),
            nested_not_source(5000),
            nested_supports_paren_source(5000),
        ];
        let handle = std::thread::Builder::new()
            .name("css-parser-depth-guard-regression".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(move || {
                for src in sources {
                    assert!(
                        try_parse(&src).is_err(),
                        "deeply-nested input must fail with an error, not parse or crash"
                    );
                }
            })
            .expect("failed to spawn worker thread");
        handle
            .join()
            .expect("depth guard must keep the worker thread from crashing");
    }

    /// A caller relying on `MAX_RULE_DEPTH` must have the guard trip
    /// *before* the native stack overflows on a default-stack thread --
    /// otherwise a production caller (or `cargo test`'s own per-test
    /// thread) would still crash. Parses far-too-deep input, for every
    /// shape, on a worker thread with **no** `stack_size` override (the
    /// same ~2 MiB a default thread gets).
    #[test]
    fn test_cap_trips_before_overflow_on_default_stack_for_every_shape() {
        let sources = vec![
            nested_qualified_rule_source(5000),
            nested_at_rule_source(5000),
            nested_calc_source(5000),
            nested_not_source(5000),
            nested_supports_paren_source(5000),
        ];
        let handle = std::thread::spawn(move || {
            for src in sources {
                assert!(try_parse(&src).is_err(), "deeply-nested input must error, not crash");
            }
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }

    /// Reasonable, hand-writable nesting for every shape stays well under
    /// the cap.
    #[test]
    fn test_reasonable_nesting_stays_under_the_cap_for_every_shape() {
        assert!(try_parse(&nested_qualified_rule_source(10)).is_ok());
        assert!(try_parse(&nested_at_rule_source(10)).is_ok());
        assert!(try_parse(&nested_calc_source(10)).is_ok());
        assert!(try_parse(&nested_not_source(10)).is_ok());
        assert!(try_parse(&nested_supports_paren_source(10)).is_ok());
    }
}

