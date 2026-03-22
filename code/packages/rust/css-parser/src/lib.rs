//! # CSS Parser — parsing CSS source code into an AST.
//!
//! This crate is the second half of the CSS front-end pipeline. Where the
//! `css-lexer` crate breaks source text into tokens, this crate arranges
//! those tokens into a tree that reflects the **structure** of the stylesheet
//! — an Abstract Syntax Tree (AST).
//!
//! # The parsing pipeline
//!
//! Parsing CSS requires four cooperating components:
//!
//! ```text
//! Source code  ("body { color: red; }")
//!       |
//!       v
//! css-lexer            → Vec<Token>
//!       |                [IDENT("body"), LBRACE, IDENT("color"), COLON,
//!       |                 IDENT("red"), SEMICOLON, RBRACE, EOF]
//!       v
//! css.grammar          → ParserGrammar (rules like "stylesheet = ...")
//!       |
//!       v
//! GrammarParser        → GrammarASTNode tree
//!       |
//!       |                stylesheet
//!       |                  └── rule
//!       |                        └── qualified_rule
//!       |                              ├── selector_list
//!       |                              │     └── IDENT("body")
//!       |                              └── block
//!       |                                    └── declaration
//!       |                                          ├── IDENT("color")
//!       |                                          ├── COLON
//!       |                                          └── IDENT("red")
//!       v
//! [future stages: rendering, layout]
//! ```
//!
//! This crate is the thin glue layer that wires these components together.
//! It knows where to find the `css.grammar` file and provides two public
//! entry points.
//!
//! # Grammar-driven parsing
//!
//! The CSS grammar is significantly larger than JSON's 4 rules, covering
//! selectors (type, class, ID, attribute, pseudo-class, pseudo-element,
//! combinators), at-rules (@media, @import, @keyframes), declarations
//! with values, and CSS nesting.

use std::fs;

use grammar_tools::parser_grammar::parse_parser_grammar;
use parser::grammar_parser::{GrammarParser, GrammarASTNode};
use coding_adventures_css_lexer::tokenize_css;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `css.grammar` file.
///
/// Uses the same strategy as the css-lexer crate: `env!("CARGO_MANIFEST_DIR")`
/// gives us the compile-time path to this crate's directory, and we navigate
/// up to the shared `grammars/` directory.
///
/// ```text
/// code/
///   grammars/
///     css.grammar           <-- target file
///   packages/
///     rust/
///       css-parser/
///         Cargo.toml        <-- CARGO_MANIFEST_DIR
/// ```
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/css.grammar")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarParser` configured for CSS source code.
///
/// This function performs two major steps:
///
/// 1. **Tokenization** — uses `tokenize_css` from the css-lexer crate
///    to break the source into tokens (IDENT, HASH, DIMENSION, STRING,
///    AT_KEYWORD, and structural delimiters).
///
/// 2. **Grammar loading** — reads and parses the `css.grammar` file,
///    which defines rules for stylesheets, selectors, declarations, and
///    at-rules.
///
/// The returned `GrammarParser` is ready to call `.parse()` on.
///
/// # Panics
///
/// Panics if:
/// - The `css.grammar` file cannot be read or parsed.
/// - The source code fails tokenization (unexpected character).
///
/// # Example
///
/// ```no_run
/// use coding_adventures_css_parser::create_css_parser;
///
/// let mut parser = create_css_parser("body { color: red; }");
/// let ast = parser.parse().expect("parse failed");
/// println!("{:?}", ast.rule_name);
/// ```
pub fn create_css_parser(source: &str) -> GrammarParser {
    // Step 1: Tokenize the source using the css-lexer.
    let tokens = tokenize_css(source);

    // Step 2: Read the parser grammar from disk.
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read css.grammar: {e}"));

    // Step 3: Parse the grammar text into a structured ParserGrammar.
    let grammar = parse_parser_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse css.grammar: {e}"));

    // Step 4: Create the parser.
    GrammarParser::new(tokens, grammar)
}

/// Parse CSS source code into an AST.
///
/// This is the most convenient entry point — it handles tokenization,
/// grammar loading, parser creation, and parsing in one call.
///
/// The returned `GrammarASTNode` has `rule_name` set to `"stylesheet"` (the
/// start symbol of the CSS grammar) with children corresponding to the
/// rules in the stylesheet.
///
/// # Panics
///
/// Panics if tokenization fails, the grammar file is missing/invalid,
/// or the source code has a syntax error.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_css_parser::parse_css;
///
/// let ast = parse_css("body { color: red; }");
/// assert_eq!(ast.rule_name, "stylesheet");
/// ```
pub fn parse_css(source: &str) -> GrammarASTNode {
    let mut css_parser = create_css_parser(source);

    css_parser
        .parse()
        .unwrap_or_else(|e| panic!("CSS parse failed: {e}"))
}

// ===========================================================================
// Tests
// ===========================================================================

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
}
