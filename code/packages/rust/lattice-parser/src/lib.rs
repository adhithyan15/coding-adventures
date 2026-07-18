//! Lattice parser backed by compiled parser grammar.

use coding_adventures_lattice_lexer::tokenize_lattice;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Recursion-depth cap for the Lattice [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] for why this guard exists at all (deep
/// recursion through `parse_rule` can overflow the *native* thread stack —
/// an uncatchable process abort — before this crate's own callers get a
/// chance to report anything). Before this constant was applied,
/// `create_lattice_parser` never called `with_max_depth` at all, leaving
/// every caller exposed to a native-stack-overflow DoS from adversarial
/// deeply-nested input (e.g. `@if (((...$x...))) { }`).
///
/// **Not the shared engine's bare default** (see `csharp-parser`'s own
/// identically-named constant for why a blind
/// [`DEFAULT_MAX_RULE_DEPTH`](parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH)
/// (128) is unsafe-for-usability on a rich general-purpose grammar).
/// Measured directly instead (binary search over candidate
/// `with_max_depth` values against a fixed 5000-level adversarial
/// `@if (((...$x...))) { }` input — ordinary parenthesised grouping, via
/// `lattice_primary = ... | LPAREN lattice_expression RPAREN` — on a
/// default-~2MiB-stack worker thread in a debug build): safe at **289**,
/// crashes at **290**.
///
/// `MAX_RULE_DEPTH` is set to **200** — about 31% below that floor
/// (comparable margin to `sql-parser`'s/`verilog-parser`'s/
/// `vhdl-parser`'s own ~30-31%). Measured real-input headroom at `200`:
/// plain parenthesised nesting parses cleanly to at least 15 levels —
/// comfortably beyond ordinary hand-written nesting depth.
///
/// Lattice's grammar has other self-referential recursion shapes beyond
/// paren-grouping (nested `qualified_rule`/`at_rule` blocks via
/// `block -> block_contents -> rule -> qualified_rule -> block`, i.e. CSS
/// nesting; nested `paren_block` at-rule preludes; nested `function_call`
/// arguments) — this pass measures only **one** of them (paren-grouping),
/// the way `ruby-parser`'s/`starlark-parser`'s own `MAX_RULE_DEPTH`
/// document a single-shape measurement across a larger shape inventory. A
/// full multi-shape audit (the `css-parser`/`toml-parser` treatment) is a
/// tracked follow-up.
const MAX_RULE_DEPTH: usize = 200;

pub fn create_lattice_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_lattice(source);
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH)
}

pub fn parse_lattice(source: &str) -> GrammarASTNode {
    let mut parser = create_lattice_parser(source);
    parser
        .parse()
        .unwrap_or_else(|e| panic!("Lattice parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    // -----------------------------------------------------------------------
    // Helper functions
    // -----------------------------------------------------------------------

    /// Recursively search for a node with the given rule_name anywhere in
    /// the AST. Returns true if found.
    fn find_rule(node: &GrammarASTNode, target: &str) -> bool {
        if node.rule_name == target {
            return true;
        }
        for child in &node.children {
            if let ASTNodeOrToken::Node(child_node) = child {
                if find_rule(child_node, target) {
                    return true;
                }
            }
        }
        false
    }

    /// Collect all rule_names found anywhere in the AST tree.
    fn collect_rule_names(node: &GrammarASTNode) -> Vec<String> {
        let mut names = vec![node.rule_name.clone()];
        for child in &node.children {
            if let ASTNodeOrToken::Node(child_node) = child {
                names.extend(collect_rule_names(child_node));
            }
        }
        names
    }

    // -----------------------------------------------------------------------
    // Test 1: Root is stylesheet
    // -----------------------------------------------------------------------

    /// Every valid Lattice document parses to a root node with rule_name
    /// "stylesheet", since that is the start symbol of the grammar.
    #[test]
    fn test_root_is_stylesheet() {
        let ast = parse_lattice("");
        assert_eq!(ast.rule_name, "stylesheet",
            "Root should be 'stylesheet', got '{}'", ast.rule_name);
    }

    // -----------------------------------------------------------------------
    // Test 2: Variable declaration
    // -----------------------------------------------------------------------

    /// A variable declaration `$color: red;` should produce a
    /// `variable_declaration` node in the AST.
    #[test]
    fn test_variable_declaration() {
        let ast = parse_lattice("$color: red;");
        assert_eq!(ast.rule_name, "stylesheet");
        assert!(find_rule(&ast, "variable_declaration"),
            "Expected 'variable_declaration' rule in AST");
    }

    // -----------------------------------------------------------------------
    // Test 3: CSS qualified rule
    // -----------------------------------------------------------------------

    /// A plain CSS rule `h1 { color: red; }` should produce `qualified_rule`
    /// containing `selector_list`, `block`, and `declaration` nodes.
    #[test]
    fn test_qualified_rule() {
        let ast = parse_lattice("h1 { color: red; }");
        assert!(find_rule(&ast, "qualified_rule"),
            "Expected 'qualified_rule' in AST");
        assert!(find_rule(&ast, "declaration"),
            "Expected 'declaration' in AST");
    }

    // -----------------------------------------------------------------------
    // Test 4: Mixin definition
    // -----------------------------------------------------------------------

    /// `@mixin flex-center() { display: flex; }` should produce a
    /// `mixin_definition` node.
    #[test]
    fn test_mixin_definition() {
        let ast = parse_lattice("@mixin flex-center() { display: flex; }");
        assert!(find_rule(&ast, "mixin_definition"),
            "Expected 'mixin_definition' in AST");
    }

    #[test]
    fn test_mixin_definition_without_parens() {
        let ast = parse_lattice("@mixin flex-center { display: flex; }");
        assert!(find_rule(&ast, "mixin_definition"),
            "Expected 'mixin_definition' in AST");
    }

    // -----------------------------------------------------------------------
    // Test 5: @include directive
    // -----------------------------------------------------------------------

    /// `@include flex-center;` inside a rule should produce an
    /// `include_directive` node.
    #[test]
    fn test_include_directive() {
        let ast = parse_lattice(".card { @include flex-center; }");
        assert!(find_rule(&ast, "include_directive"),
            "Expected 'include_directive' in AST");
    }

    // -----------------------------------------------------------------------
    // Test 6: @if directive
    // -----------------------------------------------------------------------

    /// An `@if` block should produce an `if_directive` node with a
    /// `lattice_expression` and a `block`.
    #[test]
    fn test_if_directive() {
        let ast = parse_lattice("@if $theme == dark { body { background: black; } }");
        assert!(find_rule(&ast, "if_directive"),
            "Expected 'if_directive' in AST");
        assert!(find_rule(&ast, "lattice_expression"),
            "Expected 'lattice_expression' in AST");
    }

    // -----------------------------------------------------------------------
    // Test 7: @for loop
    // -----------------------------------------------------------------------

    /// An `@for` loop should produce a `for_directive` node.
    #[test]
    fn test_for_directive() {
        let ast = parse_lattice("@for $i from 1 through 3 { .item { margin: 4px; } }");
        assert!(find_rule(&ast, "for_directive"),
            "Expected 'for_directive' in AST");
    }

    // -----------------------------------------------------------------------
    // Test 8: @each loop
    // -----------------------------------------------------------------------

    /// An `@each` loop should produce an `each_directive` node.
    #[test]
    fn test_each_directive() {
        let ast = parse_lattice("@each $color in red, green, blue { .dot { background: $color; } }");
        assert!(find_rule(&ast, "each_directive"),
            "Expected 'each_directive' in AST");
    }

    // -----------------------------------------------------------------------
    // Test 9: @function definition
    // -----------------------------------------------------------------------

    /// A `@function` block should produce a `function_definition` node with
    /// a `function_body` containing `return_directive`.
    #[test]
    fn test_function_definition() {
        let ast = parse_lattice("@function spacing($n) { @return $n * 8px; }");
        assert!(find_rule(&ast, "function_definition"),
            "Expected 'function_definition' in AST");
        assert!(find_rule(&ast, "return_directive"),
            "Expected 'return_directive' in AST");
    }

    // -----------------------------------------------------------------------
    // Test 10: @use directive
    // -----------------------------------------------------------------------

    /// An `@use` directive should produce a `use_directive` node.
    #[test]
    fn test_use_directive() {
        let ast = parse_lattice("@use \"colors\";");
        assert!(find_rule(&ast, "use_directive"),
            "Expected 'use_directive' in AST");
    }

    // -----------------------------------------------------------------------
    // Test 11: Mixin with parameters
    // -----------------------------------------------------------------------

    /// A mixin with parameters should include `mixin_params` and `mixin_param`
    /// nodes in the AST.
    #[test]
    fn test_mixin_with_params() {
        let ast = parse_lattice("@mixin button($bg, $fg: white) { background: $bg; color: $fg; }");
        assert!(find_rule(&ast, "mixin_definition"),
            "Expected 'mixin_definition' in AST");
        assert!(find_rule(&ast, "mixin_params"),
            "Expected 'mixin_params' in AST");
        assert!(find_rule(&ast, "mixin_param"),
            "Expected 'mixin_param' in AST");
    }

    // -----------------------------------------------------------------------
    // Test 12: Nested qualified rules (selectors with blocks)
    // -----------------------------------------------------------------------

    /// A selector list with multiple selectors should produce `selector_list`
    /// and `complex_selector` nodes.
    #[test]
    fn test_selector_list() {
        let ast = parse_lattice("h1, h2, h3 { color: blue; }");
        assert!(find_rule(&ast, "selector_list"),
            "Expected 'selector_list' in AST");
    }

    // -----------------------------------------------------------------------
    // Test 13: Value list with function call
    // -----------------------------------------------------------------------

    /// CSS function calls in values like `rgb(255, 0, 0)` should be parsed
    /// as `function_call` nodes inside `value_list`.
    #[test]
    fn test_function_call_in_value() {
        let ast = parse_lattice("a { color: rgb(255, 0, 0); }");
        assert!(find_rule(&ast, "function_call"),
            "Expected 'function_call' in AST");
        assert!(find_rule(&ast, "value_list"),
            "Expected 'value_list' in AST");
    }

    // -----------------------------------------------------------------------
    // Test 14: Variable reference in value
    // -----------------------------------------------------------------------

    /// Variables referenced in values (`$color`) should appear in the AST.
    /// The `value` rule includes VARIABLE as a valid value token.
    #[test]
    fn test_variable_in_value() {
        let ast = parse_lattice("$primary: red; h1 { color: $primary; }");
        let names = collect_rule_names(&ast);
        assert!(names.contains(&"variable_declaration".to_string()),
            "Expected variable_declaration in AST");
        assert!(names.contains(&"declaration".to_string()),
            "Expected declaration in AST");
    }

    // -----------------------------------------------------------------------
    // Test 15: CSS @media at-rule
    // -----------------------------------------------------------------------

    /// A CSS `@media` rule should be parsed as `at_rule` (not a Lattice rule).
    #[test]
    fn test_media_at_rule() {
        let ast = parse_lattice("@media (max-width: 768px) { .menu { display: none; } }");
        assert!(find_rule(&ast, "at_rule"),
            "Expected 'at_rule' in AST for @media");
    }

    // -----------------------------------------------------------------------
    // Test 16: create_lattice_parser factory
    // -----------------------------------------------------------------------

    /// The factory function should return a working parser.
    #[test]
    fn test_create_lattice_parser() {
        let mut parser = create_lattice_parser("$x: 42px;");
        let result = parser.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());
        let ast = result.unwrap();
        assert_eq!(ast.rule_name, "stylesheet");
    }

    // -----------------------------------------------------------------------
    // Test 17: Complex Lattice stylesheet
    // -----------------------------------------------------------------------

    /// A stylesheet that exercises multiple Lattice features together.
    #[test]
    fn test_complex_lattice_stylesheet() {
        let source = r#"
            $primary: #4a90d9;
            $spacing: 8px;

            @mixin flex-row() {
                display: flex;
                flex-direction: row;
            }

            .container {
                @include flex-row;
                padding: $spacing;
                color: $primary;
            }
        "#;
        let ast = parse_lattice(source);
        assert_eq!(ast.rule_name, "stylesheet");
        assert!(find_rule(&ast, "variable_declaration"));
        assert!(find_rule(&ast, "mixin_definition"));
        assert!(find_rule(&ast, "include_directive"));
    }

    // -----------------------------------------------------------------------
    // Test 18: CSS class and ID selectors
    // -----------------------------------------------------------------------

    /// Class selectors (`.foo`) and ID selectors (`#bar`) should parse
    /// into `class_selector` and `id_selector` nodes.
    #[test]
    fn test_class_and_id_selectors() {
        let ast = parse_lattice(".foo { color: red; }");
        assert!(find_rule(&ast, "class_selector"),
            "Expected 'class_selector' in AST for .foo");
    }

    // -----------------------------------------------------------------------
    // Test 19: CSS pseudo-class
    // -----------------------------------------------------------------------

    /// CSS pseudo-classes like `:hover` should be parsed as `pseudo_class` nodes.
    #[test]
    fn test_pseudo_class() {
        let ast = parse_lattice("a:hover { color: blue; }");
        assert!(find_rule(&ast, "pseudo_class"),
            "Expected 'pseudo_class' in AST for :hover");
    }

    // -----------------------------------------------------------------------
    // Test 20: Multiple rules produce multiple children
    // -----------------------------------------------------------------------

    /// A stylesheet with multiple rules should have multiple `rule` children
    /// at the stylesheet root.
    #[test]
    fn test_multiple_rules() {
        let ast = parse_lattice("$a: 1; $b: 2; h1 { color: red; }");
        // stylesheet has multiple rule children
        assert!(ast.children.len() >= 3,
            "Expected at least 3 rule children, got {}", ast.children.len());
    }

    // -----------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening) -- see MAX_RULE_DEPTH's own
    // doc comment for the measurement.
    // -----------------------------------------------------------------------

    fn nested_paren_source(n: usize) -> String {
        format!("@if {}$x{} {{ }}\n", "(".repeat(n), ")".repeat(n))
    }

    /// Deeply-nested input must not overflow the native stack on a
    /// default-stack thread -- the whole point of the guard.
    #[test]
    fn test_deeply_nested_input_does_not_overflow_on_default_stack() {
        let src = nested_paren_source(5000);
        let handle = std::thread::spawn(move || {
            let mut parser = create_lattice_parser(&src);
            let _ = parser.parse();
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }

    /// Reasonable, hand-writable nesting stays well under the cap.
    #[test]
    fn test_reasonable_nesting_stays_under_the_cap() {
        let mut parser = create_lattice_parser(&nested_paren_source(10));
        assert!(parser.parse().is_ok());
    }
}

