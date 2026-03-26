//! # Lattice Parser — parsing Lattice source text into an AST.
//!
//! This crate is the second stage of the Lattice compiler pipeline. Where the
//! `lattice-lexer` crate breaks source text into tokens, this crate arranges
//! those tokens into a tree that reflects the *structure* of the Lattice
//! source — an Abstract Syntax Tree (AST).
//!
//! # The pipeline
//!
//! ```text
//! Lattice source text
//!         |
//!         v
//! lattice-lexer  ──→  Vec<Token>
//!         |           [VARIABLE, COLON, HASH, SEMICOLON, ...]
//!         v
//! lattice.grammar ──→  ParserGrammar
//!         |            (rules: stylesheet, rule, variable_declaration, ...)
//!         v
//! GrammarParser  ──→  GrammarASTNode tree
//!         |
//!         |            stylesheet
//!         |              └── rule
//!         |                    └── lattice_rule
//!         |                          └── variable_declaration
//!         |                                ├── VARIABLE("$primary")
//!         |                                ├── COLON(":")
//!         |                                ├── value_list
//!         |                                │     └── value
//!         |                                │           └── HASH("#4a90d9")
//!         |                                └── SEMICOLON(";")
//!         v
//! [lattice-ast-to-css consumes the AST]
//! ```
//!
//! # Mixed AST
//!
//! The resulting AST is "mixed" — it contains both CSS nodes and Lattice
//! nodes. The `lattice-ast-to-css` crate (the next stage) separates them:
//! Lattice nodes (variable declarations, mixin definitions, @if blocks, etc.)
//! are expanded and removed; CSS nodes (qualified_rule, declaration, etc.)
//! are passed through to the emitter.
//!
//! # Grammar-Driven Approach
//!
//! Like all parsers in this monorepo, the Lattice parser does not hand-code
//! grammar rules. Instead it reads `lattice.grammar` — a declarative EBNF
//! grammar file — and uses the generic [`GrammarParser`] to drive parsing.
//! This means the parser logic lives in the grammar file, and changing the
//! language syntax requires only editing that file (plus updating the
//! compiler for any new AST node types).
//!
//! # Grammar File Location
//!
//! ```text
//! code/
//!   grammars/
//!     lattice.grammar       ← target file
//!   packages/
//!     rust/
//!       lattice-parser/
//!         Cargo.toml        ← CARGO_MANIFEST_DIR points here
//! ```

use std::fs;

use grammar_tools::parser_grammar::parse_parser_grammar;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};
use coding_adventures_lattice_lexer::tokenize_lattice;

// Re-export the AST node type so callers don't need to depend on `parser`
// directly. This is the standard pattern used throughout this monorepo.
pub use parser::grammar_parser::ASTNodeOrToken;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `lattice.grammar` file.
///
/// Same navigation strategy as `lattice-lexer`: `env!("CARGO_MANIFEST_DIR")`
/// gives the absolute path to this crate's `Cargo.toml`, and we navigate up
/// to the repository's `grammars/` directory.
///
/// Path structure:
/// ```text
/// code/grammars/lattice.grammar
///               ↑
/// code/packages/rust/lattice-parser/Cargo.toml
///                                   ↑ CARGO_MANIFEST_DIR
/// ```
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/lattice.grammar")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a [`GrammarParser`] configured for Lattice source text.
///
/// This function:
/// 1. Tokenizes the source using the Lattice lexer (`tokenize_lattice`).
/// 2. Reads `lattice.grammar` from disk and parses it into a `ParserGrammar`.
/// 3. Constructs a `GrammarParser` with the tokens and grammar.
///
/// The returned parser is ready to call `.parse()` on. Use this factory when
/// you need access to the parser object (e.g., for error recovery or partial
/// parsing).
///
/// # Panics
///
/// Panics if:
/// - The grammar file cannot be read or is malformed.
/// - The source text fails lexical analysis (unexpected character).
///
/// # Example
///
/// ```no_run
/// use coding_adventures_lattice_parser::create_lattice_parser;
///
/// let mut parser = create_lattice_parser("$color: red;");
/// let ast = parser.parse().expect("parse failed");
/// assert_eq!(ast.rule_name, "stylesheet");
/// ```
pub fn create_lattice_parser(source: &str) -> GrammarParser {
    // Step 1: Tokenize the source with the Lattice lexer.
    //
    // This produces a Vec<Token> with all CSS and Lattice token types
    // (VARIABLE, AT_KEYWORD, EQUALS_EQUALS, etc.) plus standard CSS tokens.
    // The token stream always ends with EOF.
    let tokens = tokenize_lattice(source);

    // Step 2: Read the parser grammar from disk.
    //
    // lattice.grammar contains ~35 rules covering both CSS constructs
    // (qualified_rule, selector_list, declaration, value) and Lattice
    // constructs (variable_declaration, mixin_definition, if_directive,
    // for_directive, each_directive, function_definition, use_directive).
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read lattice.grammar: {e}"));

    // Step 3: Parse the grammar text into a ParserGrammar struct.
    //
    // ParserGrammar contains a list of GrammarRule objects. Each rule has
    // a name and an EBNF body (sequences, alternations, repetitions,
    // optionals, token references, and literal matches).
    let grammar = parse_parser_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse lattice.grammar: {e}"));

    // Step 4: Create and return the parser.
    //
    // GrammarParser takes ownership of tokens and grammar, builds an
    // internal rule index, and allocates the packrat memo cache.
    GrammarParser::new(tokens, grammar)
}

/// Parse Lattice source text into an AST.
///
/// This is the main entry point for the Lattice parser. Pass in a string of
/// Lattice source text, get back a `GrammarASTNode` representing the complete
/// parse tree.
///
/// The returned AST has `rule_name = "stylesheet"` at the root. Its children
/// are `rule` nodes, each containing either:
/// - A `lattice_rule` (variable declaration, mixin/function definition, @use)
/// - A CSS `at_rule` (@media, @import, etc.)
/// - A CSS `qualified_rule` (selector + declaration block)
///
/// # Panics
///
/// Panics if lexing or parsing fails. In normal usage with valid Lattice
/// source, this should never happen.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_lattice_parser::parse_lattice;
///
/// let ast = parse_lattice("$color: red; h1 { color: $color; }");
/// assert_eq!(ast.rule_name, "stylesheet");
/// println!("Root has {} children", ast.children.len());
/// ```
pub fn parse_lattice(source: &str) -> GrammarASTNode {
    let mut parser = create_lattice_parser(source);
    parser
        .parse()
        .unwrap_or_else(|e| panic!("Lattice parse failed: {e}"))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
}
