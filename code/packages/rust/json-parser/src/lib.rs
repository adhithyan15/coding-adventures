//! JSON parser backed by compiled parser grammar.

use coding_adventures_json_lexer::tokenize_json;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

pub fn create_json_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_json(source);
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar)
}

pub fn parse_json(source: &str) -> GrammarASTNode {
    let mut parser = create_json_parser(source);
    parser
        .parse()
        .unwrap_or_else(|e| panic!("JSON parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    // -----------------------------------------------------------------------
    // Helper: check that the root node has the expected rule name.
    // -----------------------------------------------------------------------

    /// All JSON documents parse to a root node with rule_name "value",
    /// since that is the start symbol of the grammar.
    fn assert_value_root(ast: &GrammarASTNode) {
        assert_eq!(
            ast.rule_name, "value",
            "Expected root rule 'value', got '{}'",
            ast.rule_name
        );
    }

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

    // -----------------------------------------------------------------------
    // Test 1: Simple number
    // -----------------------------------------------------------------------

    /// The simplest JSON document: a single number. This exercises the full
    /// pipeline: lexer -> parser -> AST.
    #[test]
    fn test_parse_number() {
        let ast = parse_json("42");
        assert_value_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 2: Simple string
    // -----------------------------------------------------------------------

    /// A JSON document consisting of a single string value.
    #[test]
    fn test_parse_string() {
        let ast = parse_json("\"hello\"");
        assert_value_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 3: true, false, null
    // -----------------------------------------------------------------------

    /// The three JSON literal values should each parse as a value node.
    #[test]
    fn test_parse_literals() {
        let ast_true = parse_json("true");
        assert_value_root(&ast_true);

        let ast_false = parse_json("false");
        assert_value_root(&ast_false);

        let ast_null = parse_json("null");
        assert_value_root(&ast_null);
    }

    // -----------------------------------------------------------------------
    // Test 4: Empty object
    // -----------------------------------------------------------------------

    /// An empty object `{}` should parse to a value containing an object
    /// node with no pair children.
    #[test]
    fn test_parse_empty_object() {
        let ast = parse_json("{}");
        assert_value_root(&ast);

        let has_object = find_rule(&ast, "object");
        assert!(has_object, "Expected to find an 'object' rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 5: Empty array
    // -----------------------------------------------------------------------

    /// An empty array `[]` should parse to a value containing an array
    /// node with no value children.
    #[test]
    fn test_parse_empty_array() {
        let ast = parse_json("[]");
        assert_value_root(&ast);

        let has_array = find_rule(&ast, "array");
        assert!(has_array, "Expected to find an 'array' rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 6: Simple object with one pair
    // -----------------------------------------------------------------------

    /// A JSON object with one key-value pair. This tests the object and
    /// pair grammar rules.
    #[test]
    fn test_parse_simple_object() {
        let ast = parse_json("{\"key\": 42}");
        assert_value_root(&ast);

        let has_object = find_rule(&ast, "object");
        let has_pair = find_rule(&ast, "pair");
        assert!(has_object, "Expected 'object' rule in the AST");
        assert!(has_pair, "Expected 'pair' rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 7: Object with multiple pairs
    // -----------------------------------------------------------------------

    /// A JSON object with multiple key-value pairs tests the comma-separated
    /// repetition pattern in the grammar: `[ pair { COMMA pair } ]`.
    #[test]
    fn test_parse_multi_pair_object() {
        let ast = parse_json("{\"a\": 1, \"b\": 2, \"c\": 3}");
        assert_value_root(&ast);

        let has_object = find_rule(&ast, "object");
        assert!(has_object, "Expected 'object' rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 8: Simple array
    // -----------------------------------------------------------------------

    /// A JSON array with multiple values of the same type.
    #[test]
    fn test_parse_simple_array() {
        let ast = parse_json("[1, 2, 3]");
        assert_value_root(&ast);

        let has_array = find_rule(&ast, "array");
        assert!(has_array, "Expected 'array' rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 9: Mixed-type array
    // -----------------------------------------------------------------------

    /// A JSON array with values of different types: number, string, boolean,
    /// null. This tests that the `value` alternation works correctly inside
    /// array context.
    #[test]
    fn test_parse_mixed_array() {
        let ast = parse_json("[1, \"two\", true, null]");
        assert_value_root(&ast);

        let has_array = find_rule(&ast, "array");
        assert!(has_array, "Expected 'array' rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 10: Nested object
    // -----------------------------------------------------------------------

    /// A JSON object containing another object as a value. This exercises
    /// the mutual recursion between `value` and `object`.
    #[test]
    fn test_parse_nested_object() {
        let ast = parse_json("{\"outer\": {\"inner\": 42}}");
        assert_value_root(&ast);

        let has_object = find_rule(&ast, "object");
        assert!(has_object, "Expected 'object' rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 11: Nested array
    // -----------------------------------------------------------------------

    /// A JSON array containing another array. This exercises the mutual
    /// recursion between `value` and `array`.
    #[test]
    fn test_parse_nested_array() {
        let ast = parse_json("[[1, 2], [3, 4]]");
        assert_value_root(&ast);

        let has_array = find_rule(&ast, "array");
        assert!(has_array, "Expected 'array' rule in the AST");
    }

    // -----------------------------------------------------------------------
    // Test 12: Deeply nested structure
    // -----------------------------------------------------------------------

    /// A complex JSON document mixing objects and arrays at multiple levels.
    /// This is the ultimate test of the recursive grammar.
    #[test]
    fn test_parse_deeply_nested() {
        let source = r#"{"users": [{"name": "Alice", "scores": [95, 87]}, {"name": "Bob", "scores": [72]}]}"#;
        let ast = parse_json(source);
        assert_value_root(&ast);

        let has_object = find_rule(&ast, "object");
        let has_array = find_rule(&ast, "array");
        let has_pair = find_rule(&ast, "pair");
        assert!(has_object, "Expected 'object' rule");
        assert!(has_array, "Expected 'array' rule");
        assert!(has_pair, "Expected 'pair' rule");
    }

    // -----------------------------------------------------------------------
    // Test 13: Factory function
    // -----------------------------------------------------------------------

    /// The `create_json_parser` factory function should return a working
    /// `GrammarParser` that can successfully parse JSON.
    #[test]
    fn test_create_parser() {
        let mut parser = create_json_parser("42");
        let result = parser.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());

        let ast = result.unwrap();
        assert_eq!(ast.rule_name, "value");
    }

    // -----------------------------------------------------------------------
    // Test 14: Whitespace handling
    // -----------------------------------------------------------------------

    /// JSON allows arbitrary whitespace between tokens. The parser should
    /// handle prettified JSON the same as minified JSON.
    #[test]
    fn test_parse_with_whitespace() {
        let prettified = r#"{
  "name": "Alice",
  "age": 30
}"#;
        let ast = parse_json(prettified);
        assert_value_root(&ast);

        let has_object = find_rule(&ast, "object");
        assert!(has_object, "Expected 'object' rule in prettified JSON");
    }

    // -----------------------------------------------------------------------
    // Test 15: Negative and decimal numbers in context
    // -----------------------------------------------------------------------

    /// Numbers with all features (negative, decimal, exponent) should parse
    /// correctly when embedded in JSON structures.
    #[test]
    fn test_parse_complex_numbers() {
        let ast = parse_json("[-3.14, 0, 1e10, 2.5E-3]");
        assert_value_root(&ast);

        let has_array = find_rule(&ast, "array");
        assert!(has_array, "Expected 'array' rule");
    }

    // -----------------------------------------------------------------------
    // Test 16: String values with escapes in context
    // -----------------------------------------------------------------------

    /// Strings with escape sequences should parse correctly inside objects.
    #[test]
    fn test_parse_escaped_strings() {
        let ast = parse_json("{\"msg\": \"line1\\nline2\"}");
        assert_value_root(&ast);

        let has_pair = find_rule(&ast, "pair");
        assert!(has_pair, "Expected 'pair' rule");
    }
}

