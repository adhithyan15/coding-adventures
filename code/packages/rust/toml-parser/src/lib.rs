//! TOML parser backed by compiled parser grammar.

use coding_adventures_toml_lexer::tokenize_toml;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Recursion-depth cap for the TOML [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] and
/// [`parser::grammar_parser::DEFAULT_MAX_RULE_DEPTH`] for why the underlying
/// guard exists at all (deep recursion through `parse_rule` can overflow the
/// *native* thread stack — an uncatchable process abort — before this
/// crate's own callers ever get a chance to report anything). Before this
/// constant was added, `create_toml_parser` never called `with_max_depth`
/// at all, leaving every caller (including this crate's own `parse_toml`)
/// exposed to a native-stack-overflow DoS from adversarial input.
///
/// # Two recursion shapes, measured independently
///
/// `toml.grammar` has two distinct self-referential productions (each a
/// genuine cycle of rule calls through `value`, not an EBNF `{ x }`
/// repetition — those cost zero native stack regardless of width,
/// confirmed in `reduce-parser`'s own `MAX_RULE_DEPTH` doc comment via a
/// throwaway probe grammar). Each was measured directly: binary search
/// over candidate `with_max_depth` values against a fixed 5000-level
/// adversarial input of that shape (deep enough that the *cap itself*,
/// not the input's finite length, is what triggers first) on a
/// default-~2MiB-stack worker thread in a debug build.
///
/// 1. **Nested arrays**, `a = [[[[1]]]]` — `value -> array ->
///    array_values -> value -> …`. Safe at **236**, crashes at **237**.
/// 2. **Nested inline tables**, `a = { b = { c = { d = 1 } } }` —
///    `value -> inline_table -> keyval -> value -> …`. Safe at **236**,
///    crashes at **237**.
///
/// Both shapes land on the identical floor (236/237) — they cycle through
/// `value` with near-identical per-level rule-frame cost.
///
/// `MAX_RULE_DEPTH` is set to **165** — about 30% below the 236 floor
/// (comparable margin to `apl-parser`'s own ~26.5%, `j-parser`'s ~30%,
/// `reduce-parser`'s ~28.5%). Measured real-input headroom at `165` (using
/// the CAPPED parser, so no crash risk at all): both nested arrays and
/// nested inline tables parse cleanly to 53 levels (54 trips) — comfortably
/// beyond anything a hand-written TOML document needs, and both
/// independently confirmed not to crash a default-stack thread even
/// thousands of levels past the cap (see this crate's tests).
const MAX_RULE_DEPTH: usize = 165;

pub fn create_toml_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_toml(source);
    let grammar = _grammar::parser_grammar();
    GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH)
}

pub fn parse_toml(source: &str) -> GrammarASTNode {
    let mut parser = create_toml_parser(source);
    parser
        .parse()
        .unwrap_or_else(|e| panic!("TOML parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::ASTNodeOrToken;

    // -----------------------------------------------------------------------
    // Helper: check that the root node has the expected rule name.
    // -----------------------------------------------------------------------

    /// All TOML documents parse to a root node with rule_name "document",
    /// since that is the start symbol of the grammar.
    fn assert_document_root(ast: &GrammarASTNode) {
        assert_eq!(
            ast.rule_name, "document",
            "Expected root rule 'document', got '{}'",
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

    /// Count how many times a rule appears in the AST (recursively).
    fn count_rule(node: &GrammarASTNode, target_rule: &str) -> usize {
        let mut count = if node.rule_name == target_rule { 1 } else { 0 };
        for child in &node.children {
            if let ASTNodeOrToken::Node(child_node) = child {
                count += count_rule(child_node, target_rule);
            }
        }
        count
    }

    // -----------------------------------------------------------------------
    // Test 1: Simple key-value pair
    // -----------------------------------------------------------------------

    /// The most basic TOML document: a single key-value pair.
    #[test]
    fn test_parse_simple_keyval() {
        let ast = parse_toml("title = \"TOML Example\"");
        assert_document_root(&ast);

        let has_keyval = find_rule(&ast, "keyval");
        let has_key = find_rule(&ast, "key");
        let has_value = find_rule(&ast, "value");
        assert!(has_keyval, "Expected 'keyval' rule in AST");
        assert!(has_key, "Expected 'key' rule in AST");
        assert!(has_value, "Expected 'value' rule in AST");
    }

    // -----------------------------------------------------------------------
    // Test 2: Integer value
    // -----------------------------------------------------------------------

    /// Key with an integer value.
    #[test]
    fn test_parse_integer_value() {
        let ast = parse_toml("port = 8080");
        assert_document_root(&ast);

        let has_keyval = find_rule(&ast, "keyval");
        assert!(has_keyval, "Expected 'keyval' rule in AST");
    }

    // -----------------------------------------------------------------------
    // Test 3: Boolean value
    // -----------------------------------------------------------------------

    /// Key with a boolean value.
    #[test]
    fn test_parse_boolean_value() {
        let ast = parse_toml("enabled = true");
        assert_document_root(&ast);

        let has_keyval = find_rule(&ast, "keyval");
        assert!(has_keyval, "Expected 'keyval' rule in AST");
    }

    // -----------------------------------------------------------------------
    // Test 4: Float value
    // -----------------------------------------------------------------------

    /// Key with a float value.
    #[test]
    fn test_parse_float_value() {
        let ast = parse_toml("pi = 3.14");
        assert_document_root(&ast);

        let has_keyval = find_rule(&ast, "keyval");
        assert!(has_keyval, "Expected 'keyval' rule in AST");
    }

    // -----------------------------------------------------------------------
    // Test 5: Table header
    // -----------------------------------------------------------------------

    /// A table header followed by key-value pairs.
    #[test]
    fn test_parse_table_header() {
        let source = "[server]\nhost = \"localhost\"";
        let ast = parse_toml(source);
        assert_document_root(&ast);

        let has_table_header = find_rule(&ast, "table_header");
        let has_keyval = find_rule(&ast, "keyval");
        assert!(has_table_header, "Expected 'table_header' rule in AST");
        assert!(has_keyval, "Expected 'keyval' rule in AST");
    }

    // -----------------------------------------------------------------------
    // Test 6: Array-of-tables header
    // -----------------------------------------------------------------------

    /// Array-of-tables headers create array elements.
    #[test]
    fn test_parse_array_table_header() {
        let source = "[[products]]\nname = \"Hammer\"";
        let ast = parse_toml(source);
        assert_document_root(&ast);

        let has_array_table = find_rule(&ast, "array_table_header");
        assert!(has_array_table, "Expected 'array_table_header' rule in AST");
    }

    // -----------------------------------------------------------------------
    // Test 7: Dotted key
    // -----------------------------------------------------------------------

    /// Dotted keys create intermediate tables.
    #[test]
    fn test_parse_dotted_key() {
        let ast = parse_toml("a.b.c = 1");
        assert_document_root(&ast);

        let has_key = find_rule(&ast, "key");
        let has_simple_key = find_rule(&ast, "simple_key");
        assert!(has_key, "Expected 'key' rule in AST");
        assert!(has_simple_key, "Expected 'simple_key' rule in AST");
    }

    // -----------------------------------------------------------------------
    // Test 8: Inline table
    // -----------------------------------------------------------------------

    /// Inline tables are compact single-line table definitions.
    #[test]
    fn test_parse_inline_table() {
        let ast = parse_toml("point = { x = 1, y = 2 }");
        assert_document_root(&ast);

        let has_inline_table = find_rule(&ast, "inline_table");
        assert!(has_inline_table, "Expected 'inline_table' rule in AST");
    }

    // -----------------------------------------------------------------------
    // Test 9: Array value
    // -----------------------------------------------------------------------

    /// Arrays are comma-separated values in brackets.
    #[test]
    fn test_parse_array_value() {
        let ast = parse_toml("colors = [\"red\", \"green\", \"blue\"]");
        assert_document_root(&ast);

        let has_array = find_rule(&ast, "array");
        assert!(has_array, "Expected 'array' rule in AST");
    }

    // -----------------------------------------------------------------------
    // Test 10: Multi-line array
    // -----------------------------------------------------------------------

    /// Arrays can span multiple lines with trailing commas.
    #[test]
    fn test_parse_multiline_array() {
        let source = "colors = [\n  \"red\",\n  \"green\",\n  \"blue\",\n]";
        let ast = parse_toml(source);
        assert_document_root(&ast);

        let has_array = find_rule(&ast, "array");
        assert!(has_array, "Expected 'array' rule in AST");
    }

    // -----------------------------------------------------------------------
    // Test 11: Multiple key-value pairs
    // -----------------------------------------------------------------------

    /// Multiple key-value pairs separated by newlines.
    #[test]
    fn test_parse_multiple_keyvals() {
        let source = "a = 1\nb = 2\nc = 3";
        let ast = parse_toml(source);
        assert_document_root(&ast);

        let keyval_count = count_rule(&ast, "keyval");
        assert_eq!(keyval_count, 3, "Expected 3 keyval rules, got {keyval_count}");
    }

    // -----------------------------------------------------------------------
    // Test 12: Datetime value
    // -----------------------------------------------------------------------

    /// Date/time values in key-value pairs.
    #[test]
    fn test_parse_datetime_value() {
        let ast = parse_toml("dob = 1979-05-27T07:32:00Z");
        assert_document_root(&ast);

        let has_value = find_rule(&ast, "value");
        assert!(has_value, "Expected 'value' rule in AST");
    }

    // -----------------------------------------------------------------------
    // Test 13: Empty document
    // -----------------------------------------------------------------------

    /// An empty document should parse to just a document node.
    #[test]
    fn test_parse_empty_document() {
        let ast = parse_toml("");
        assert_document_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 14: Comment-only document
    // -----------------------------------------------------------------------

    /// A document with only comments and blank lines.
    #[test]
    fn test_parse_comment_only() {
        let ast = parse_toml("# this is a comment\n# another comment");
        assert_document_root(&ast);
    }

    // -----------------------------------------------------------------------
    // Test 15: Nested table headers
    // -----------------------------------------------------------------------

    /// Dotted table headers create nested table structure.
    #[test]
    fn test_parse_nested_table_header() {
        let source = "[a.b.c]\nkey = \"value\"";
        let ast = parse_toml(source);
        assert_document_root(&ast);

        let has_table_header = find_rule(&ast, "table_header");
        assert!(has_table_header, "Expected 'table_header' rule in AST");
    }

    // -----------------------------------------------------------------------
    // Test 16: Literal string value
    // -----------------------------------------------------------------------

    /// Literal strings (single-quoted) as values.
    #[test]
    fn test_parse_literal_string_value() {
        let ast = parse_toml("path = 'C:\\Users\\Alice'");
        assert_document_root(&ast);

        let has_value = find_rule(&ast, "value");
        assert!(has_value, "Expected 'value' rule in AST");
    }

    // -----------------------------------------------------------------------
    // Test 17: Factory function
    // -----------------------------------------------------------------------

    /// The `create_toml_parser` factory function should return a working parser.
    #[test]
    fn test_create_parser() {
        let mut parser = create_toml_parser("key = 42");
        let result = parser.parse();
        assert!(result.is_ok(), "Parser should succeed: {:?}", result.err());

        let ast = result.unwrap();
        assert_eq!(ast.rule_name, "document");
    }

    // -----------------------------------------------------------------------
    // Test 18: Complete TOML document
    // -----------------------------------------------------------------------

    /// A realistic multi-section TOML document.
    #[test]
    fn test_parse_full_document() {
        let source = "# Configuration file
title = \"My App\"

[server]
host = \"localhost\"
port = 8080
enabled = true

[database]
connection = \"postgresql://localhost/mydb\"
pool_size = 5

[[users]]
name = \"Alice\"
role = \"admin\"

[[users]]
name = \"Bob\"
role = \"user\"
";
        let ast = parse_toml(source);
        assert_document_root(&ast);

        let has_table_header = find_rule(&ast, "table_header");
        let has_array_table = find_rule(&ast, "array_table_header");
        let has_keyval = find_rule(&ast, "keyval");
        assert!(has_table_header, "Expected table headers");
        assert!(has_array_table, "Expected array-of-tables headers");
        assert!(has_keyval, "Expected key-value pairs");

        // Should have 2 table headers ([server] and [database])
        let table_count = count_rule(&ast, "table_header");
        assert_eq!(table_count, 2, "Expected 2 table headers, got {table_count}");

        // Should have 2 array-of-tables headers ([[users]] x2)
        let array_table_count = count_rule(&ast, "array_table_header");
        assert_eq!(array_table_count, 2, "Expected 2 array table headers, got {array_table_count}");
    }

    // -----------------------------------------------------------------------
    // Test 19: Inline table with multiple pairs
    // -----------------------------------------------------------------------

    /// Inline tables can have multiple key-value pairs.
    #[test]
    fn test_parse_inline_table_multi() {
        let ast = parse_toml("person = { name = \"Alice\", age = 30, active = true }");
        assert_document_root(&ast);

        let has_inline_table = find_rule(&ast, "inline_table");
        assert!(has_inline_table, "Expected 'inline_table' rule in AST");
    }

    // -----------------------------------------------------------------------
    // Test 20: Array of integers
    // -----------------------------------------------------------------------

    /// Arrays of homogeneous integer values.
    #[test]
    fn test_parse_integer_array() {
        let ast = parse_toml("ports = [8001, 8001, 8002]");
        assert_document_root(&ast);

        let has_array = find_rule(&ast, "array");
        let has_array_values = find_rule(&ast, "array_values");
        assert!(has_array, "Expected 'array' rule in AST");
        assert!(has_array_values, "Expected 'array_values' rule in AST");
    }

    // -----------------------------------------------------------------------
    // Test 21: Quoted key in table header
    // -----------------------------------------------------------------------

    /// Table headers can use quoted keys.
    #[test]
    fn test_parse_quoted_table_header() {
        let source = "[\"quoted key\"]\nvalue = 1";
        let ast = parse_toml(source);
        assert_document_root(&ast);

        let has_table_header = find_rule(&ast, "table_header");
        assert!(has_table_header, "Expected 'table_header' rule in AST");
    }

    // -----------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening) -- exercises both
    // independently-measured shapes documented on `MAX_RULE_DEPTH`.
    // -----------------------------------------------------------------------

    fn nested_array_source(n: usize) -> String {
        let mut s = String::from("a = ");
        for _ in 0..n {
            s.push('[');
        }
        s.push('1');
        for _ in 0..n {
            s.push(']');
        }
        s
    }

    fn nested_inline_table_source(n: usize) -> String {
        let mut s = String::from("a = ");
        for i in 0..n {
            if i > 0 {
                s.push_str("b = ");
            }
            s.push_str("{ ");
        }
        s.push_str("c = 1");
        for _ in 0..n {
            s.push_str(" }");
        }
        s
    }

    fn try_parse(src: &str) -> Result<GrammarASTNode, String> {
        create_toml_parser(src).parse().map_err(|e| e.to_string())
    }

    /// Deeply-nested input, for both measured shapes, must produce a
    /// recoverable error, not overflow the native stack. Parses 5000
    /// levels -- far past `MAX_RULE_DEPTH` -- on a worker thread with a
    /// generous 32 MiB stack, so the *guard* is what stops the recursion,
    /// not the stack running out.
    #[test]
    fn test_deeply_nested_input_returns_error_not_overflow_for_every_shape() {
        let sources = vec![nested_array_source(5000), nested_inline_table_source(5000)];
        let handle = std::thread::Builder::new()
            .name("toml-parser-depth-guard-regression".to_string())
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
    /// thread) would still crash. Parses far-too-deep input, for both
    /// shapes, on a worker thread with **no** `stack_size` override (the
    /// same ~2 MiB a default thread gets).
    #[test]
    fn test_cap_trips_before_overflow_on_default_stack_for_every_shape() {
        let sources = vec![nested_array_source(5000), nested_inline_table_source(5000)];
        let handle = std::thread::spawn(move || {
            for src in sources {
                assert!(try_parse(&src).is_err(), "deeply-nested input must error, not crash");
            }
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }

    /// Reasonable, hand-writable nesting for both shapes stays well under
    /// the cap.
    #[test]
    fn test_reasonable_nesting_stays_under_the_cap_for_every_shape() {
        assert!(try_parse(&nested_array_source(10)).is_ok());
        assert!(try_parse(&nested_inline_table_source(10)).is_ok());
    }
}

