//! # JSON Parser — parsing JSON source text into an AST.
//!
//! This crate is the second half of the JSON front-end pipeline. Where the
//! `json-lexer` crate breaks source text into tokens, this crate arranges
//! those tokens into a tree that reflects the **structure** of the data —
//! an Abstract Syntax Tree (AST).
//!
//! # The parsing pipeline
//!
//! Parsing JSON requires four cooperating components:
//!
//! ```text
//! Source text  ("{\"name\": \"Alice\", \"age\": 30}")
//!       |
//!       v
//! json-lexer           → Vec<Token>
//!       |                [LBRACE, STRING("name"), COLON, STRING("Alice"),
//!       |                 COMMA, STRING("age"), COLON, NUMBER("30"), RBRACE, EOF]
//!       v
//! json.grammar         → ParserGrammar (rules: value, object, pair, array)
//!       |
//!       v
//! GrammarParser        → GrammarASTNode tree
//!       |
//!       |                value
//!       |                  └── object
//!       |                        ├── LBRACE
//!       |                        ├── pair
//!       |                        │     ├── STRING("name")
//!       |                        │     ├── COLON
//!       |                        │     └── value
//!       |                        │           └── STRING("Alice")
//!       |                        ├── COMMA
//!       |                        ├── pair
//!       |                        │     ├── STRING("age")
//!       |                        │     ├── COLON
//!       |                        │     └── value
//!       |                        │           └── NUMBER("30")
//!       |                        └── RBRACE
//!       v
//! [application logic consumes the AST]
//! ```
//!
//! This crate is the thin glue layer that wires these components together.
//! It knows where to find the `json.grammar` file and provides two public
//! entry points.
//!
//! # Grammar-driven parsing
//!
//! The `GrammarParser` is a **recursive descent parser with backtracking and
//! packrat memoization**. JSON's grammar is only four rules:
//!
//! - `value` — the start symbol: object | array | STRING | NUMBER | TRUE | FALSE | NULL
//! - `object` — `{ [ pair { , pair } ] }`
//! - `pair` — `STRING : value`
//! - `array` — `[ [ value { , value } ] ]`
//!
//! The mutual recursion between `value`, `object`, and `array` allows JSON
//! to represent arbitrarily deep nested structures.
//!
//! # Why JSON?
//!
//! JSON is an ideal first target for the parser infrastructure because:
//!
//! 1. **Universally known** — every developer has worked with JSON.
//! 2. **Minimal grammar** — only 4 rules (vs. Starlark's ~40).
//! 3. **No ambiguity** — every token unambiguously identifies the construct.
//! 4. **Recursive** — objects and arrays nest arbitrarily deep.
//! 5. **Real-world usage** — parsing JSON is genuinely useful.

use std::fs;

use grammar_tools::parser_grammar::parse_parser_grammar;
use parser::grammar_parser::{GrammarParser, GrammarASTNode};
use coding_adventures_json_lexer::tokenize_json;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `json.grammar` file.
///
/// Uses the same strategy as the json-lexer crate: `env!("CARGO_MANIFEST_DIR")`
/// gives us the compile-time path to this crate's directory, and we navigate
/// up to the shared `grammars/` directory.
///
/// ```text
/// code/
///   grammars/
///     json.grammar          <-- target file
///   packages/
///     rust/
///       json-parser/
///         Cargo.toml        <-- CARGO_MANIFEST_DIR
/// ```
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/json.grammar")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarParser` configured for JSON source text.
///
/// This function performs two major steps:
///
/// 1. **Tokenization** — uses `tokenize_json` from the json-lexer crate
///    to break the source into tokens (STRING, NUMBER, TRUE, FALSE, NULL,
///    and structural delimiters).
///
/// 2. **Grammar loading** — reads and parses the `json.grammar` file,
///    which defines 4 rules: value, object, pair, and array.
///
/// The returned `GrammarParser` is ready to call `.parse()` on.
///
/// # Panics
///
/// Panics if:
/// - The `json.grammar` file cannot be read or parsed.
/// - The source text fails tokenization (unexpected character).
///
/// # Example
///
/// ```no_run
/// use coding_adventures_json_parser::create_json_parser;
///
/// let mut parser = create_json_parser("{\"key\": 42}");
/// let ast = parser.parse().expect("parse failed");
/// println!("{:?}", ast.rule_name);
/// ```
pub fn create_json_parser(source: &str) -> GrammarParser {
    // Step 1: Tokenize the source using the json-lexer.
    //
    // This produces a Vec<Token> with JSON token types:
    // STRING, NUMBER, TRUE, FALSE, NULL, LBRACE, RBRACE,
    // LBRACKET, RBRACKET, COLON, COMMA, and EOF.
    let tokens = tokenize_json(source);

    // Step 2: Read the parser grammar from disk.
    //
    // The grammar file defines the syntactic structure of JSON in EBNF
    // notation. It has just four rules:
    //   value  = object | array | STRING | NUMBER | TRUE | FALSE | NULL ;
    //   object = LBRACE [ pair { COMMA pair } ] RBRACE ;
    //   pair   = STRING COLON value ;
    //   array  = LBRACKET [ value { COMMA value } ] RBRACKET ;
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read json.grammar: {e}"));

    // Step 3: Parse the grammar text into a structured ParserGrammar.
    //
    // The ParserGrammar contains a list of GrammarRule objects, each with
    // a name and a body (a tree of GrammarElement nodes representing the
    // EBNF structure).
    let grammar = parse_parser_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse json.grammar: {e}"));

    // Step 4: Create the parser.
    //
    // The GrammarParser takes ownership of both the tokens and the grammar.
    // It builds internal indexes (rule lookup, memo cache) for efficient
    // parsing.
    GrammarParser::new(tokens, grammar)
}

/// Parse JSON source text into an AST.
///
/// This is the most convenient entry point — it handles tokenization,
/// grammar loading, parser creation, and parsing in one call.
///
/// The returned `GrammarASTNode` has `rule_name` set to `"value"` (the
/// start symbol of the JSON grammar) with children corresponding to the
/// structure of the JSON document.
///
/// # Panics
///
/// Panics if tokenization fails, the grammar file is missing/invalid,
/// or the source text has a syntax error.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_json_parser::parse_json;
///
/// let ast = parse_json("{\"key\": 42}");
/// assert_eq!(ast.rule_name, "value");
/// ```
pub fn parse_json(source: &str) -> GrammarASTNode {
    // Create a parser wired to the JSON grammar and tokens.
    let mut json_parser = create_json_parser(source);

    // Parse and unwrap — any GrammarParseError becomes a panic.
    //
    // In a production tool, you would propagate the error via Result.
    // For this educational codebase, panicking with a descriptive message
    // is sufficient.
    json_parser
        .parse()
        .unwrap_or_else(|e| panic!("JSON parse failed: {e}"))
}

// ===========================================================================
// Tests
// ===========================================================================

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
