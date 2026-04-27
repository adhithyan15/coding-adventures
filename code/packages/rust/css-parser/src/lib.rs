//! CSS parser backed by compiled parser grammar.

use coding_adventures_css_lexer::tokenize_css;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

pub fn create_css_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_css(source);
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar)
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
}

