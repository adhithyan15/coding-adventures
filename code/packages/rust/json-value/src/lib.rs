//! # JSON Value — typed representation of JSON data.
//!
//! This crate bridges the gap between the generic AST produced by `json-parser`
//! and meaningful, typed data that application code can work with. It answers
//! the question: "What does this JSON *mean*?"
//!
//! # The pipeline
//!
//! ```text
//! JSON text  -->  json-lexer  -->  json-parser  -->  json-value  -->  application
//!   &str          Vec<Token>      GrammarASTNode     JsonValue        your code
//! ```
//!
//! The `json-parser` crate produces a `GrammarASTNode` tree — a generic parse
//! tree where every node is either a grammar rule ("value", "object", "pair",
//! "array") or a raw token (STRING, NUMBER, TRUE, FALSE, NULL, LBRACE, etc.).
//! This tree faithfully represents the *syntax* of JSON but carries no type
//! information. You'd have to inspect rule names and token types everywhere.
//!
//! `json-value` converts that generic tree into a `JsonValue` enum — a proper
//! Rust type that you can pattern-match on:
//!
//! ```text
//! match value {
//!     JsonValue::Object(pairs) => { /* iterate key-value pairs */ }
//!     JsonValue::Array(elems)  => { /* iterate elements */ }
//!     JsonValue::String(s)     => { /* use the string */ }
//!     JsonValue::Number(n)     => { /* use the number */ }
//!     JsonValue::Bool(b)       => { /* use the boolean */ }
//!     JsonValue::Null          => { /* handle null */ }
//! }
//! ```
//!
//! # Why JsonValue instead of serde_json::Value?
//!
//! This monorepo has a zero-external-dependency policy. We already built the
//! lexer and parser from scratch — `JsonValue` completes the pipeline using
//! our own infrastructure. It also serves as a learning exercise: the gap
//! between "I can parse JSON" and "I can use parsed JSON in my program" is
//! where most engineers never look.
//!
//! # Number representation
//!
//! JSON does not distinguish integers from floats — `42` and `42.0` are both
//! valid JSON numbers. However, for practical use, we distinguish them:
//!
//! - `42` (no decimal point, no exponent) --> `JsonNumber::Integer(42)`
//! - `3.14` (has decimal point) --> `JsonNumber::Float(3.14)`
//! - `1e10` (has exponent) --> `JsonNumber::Float(10000000000.0)`
//!
//! This matches the behavior of Python's `json.loads`, Ruby's `JSON.parse`,
//! and Go's `json.Unmarshal` with `json.Number`.

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use lexer::token::TokenType;

// ===========================================================================
// Error type
// ===========================================================================

/// Errors that can occur during AST-to-JsonValue conversion or parsing.
///
/// These errors indicate structural problems with the AST or invalid JSON
/// text. They are distinct from parse errors (which come from the parser
/// crate) — a `JsonValueError` means the AST was syntactically valid but
/// contained something we couldn't convert to a meaningful value.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonValueError {
    /// Human-readable description of what went wrong.
    pub message: String,
}

impl JsonValueError {
    /// Create a new error with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for JsonValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JsonValueError: {}", self.message)
    }
}

impl std::error::Error for JsonValueError {}

// ===========================================================================
// JsonNumber — integer or floating-point
// ===========================================================================

/// A JSON number, which can be either an integer or a floating-point value.
///
/// JSON itself makes no distinction between integers and floats — both are
/// just "numbers." However, most programming languages do distinguish them,
/// and users expect `42` to be an integer and `3.14` to be a float.
///
/// # Decision rule
///
/// The original JSON text determines which variant we use:
///
/// | JSON text | Has `.` or `e`/`E`? | Variant |
/// |-----------|---------------------|---------|
/// | `42`      | No                  | `Integer(42)` |
/// | `-17`     | No                  | `Integer(-17)` |
/// | `3.14`    | Yes (`.`)           | `Float(3.14)` |
/// | `1e10`    | Yes (`e`)           | `Float(1e10)` |
/// | `2.5E-3`  | Yes (`.` and `E`)   | `Float(0.0025)` |
#[derive(Debug, Clone, PartialEq)]
pub enum JsonNumber {
    /// A whole number with no decimal point or exponent.
    /// Range: i64::MIN to i64::MAX.
    Integer(i64),

    /// A number with a decimal point and/or exponent.
    /// Stored as IEEE 754 double-precision.
    Float(f64),
}

// ===========================================================================
// JsonValue — the six JSON types
// ===========================================================================

/// A typed representation of a JSON value.
///
/// JSON has exactly six value types, and `JsonValue` mirrors them one-to-one:
///
/// | JSON type | Rust representation |
/// |-----------|---------------------|
/// | object    | `Vec<(String, JsonValue)>` — ordered key-value pairs |
/// | array     | `Vec<JsonValue>` — ordered sequence |
/// | string    | `String` |
/// | number    | `JsonNumber` (integer or float) |
/// | boolean   | `bool` |
/// | null      | unit (no data) |
///
/// # Why `Vec<(String, JsonValue)>` for objects?
///
/// RFC 8259 says JSON objects are "unordered collections of name/value pairs,"
/// but practically, insertion order matters for:
/// - Human readability (config files, API responses)
/// - Round-trip fidelity (parse then serialize should preserve order)
/// - Deterministic output (tests, diffs)
///
/// A `Vec` of pairs preserves insertion order. If you need key lookup, iterate
/// and find — JSON objects are typically small enough that linear search is fine.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    /// A JSON object: `{"key": value, ...}`.
    ///
    /// Stored as an ordered list of (key, value) pairs. Keys are strings,
    /// values are recursively `JsonValue`. Duplicate keys are preserved
    /// (though the spec discourages them).
    Object(Vec<(String, JsonValue)>),

    /// A JSON array: `[value, ...]`.
    ///
    /// Stored as an ordered list of values. Array elements can be any
    /// JSON type, including mixed types: `[1, "two", true, null]`.
    Array(Vec<JsonValue>),

    /// A JSON string: `"hello"`.
    ///
    /// The string value has already been unescaped by the lexer — escape
    /// sequences like `\n`, `\t`, `\"`, `\\`, and `\uXXXX` have been
    /// converted to their actual characters.
    String(String),

    /// A JSON number: `42` or `3.14`.
    ///
    /// See [`JsonNumber`] for the integer/float distinction.
    Number(JsonNumber),

    /// A JSON boolean: `true` or `false`.
    Bool(bool),

    /// A JSON null value.
    Null,
}

// ===========================================================================
// Core conversion: AST --> JsonValue
// ===========================================================================

/// Convert a json-parser AST node into a typed `JsonValue`.
///
/// This is the heart of the crate — a recursive tree walk that dispatches
/// on rule names and token types to build a `JsonValue` tree.
///
/// # Algorithm
///
/// The AST produced by `json-parser` has this structure:
///
/// ```text
/// value
///   └── object | array | STRING | NUMBER | TRUE | FALSE | NULL
///         │
///         ├── (if object) pair, pair, pair, ...
///         │     └── STRING, COLON, value
///         │
///         └── (if array) value, value, value, ...
/// ```
///
/// The tree walk dispatches on:
/// - **Token nodes** (leaves): STRING -> JsonString, NUMBER -> JsonNumber, etc.
/// - **Rule "value"**: unwrap to find the meaningful child (skip LBRACE, COMMA, etc.)
/// - **Rule "object"**: collect all "pair" children into key-value pairs
/// - **Rule "pair"**: extract the STRING key and recurse on the value child
/// - **Rule "array"**: collect all "value" children into a list
///
/// # Errors
///
/// Returns `JsonValueError` if:
/// - A "value" node has no meaningful child
/// - A "pair" node is missing its key or value
/// - A NUMBER token cannot be parsed as i64 or f64
/// - An unexpected rule name is encountered
pub fn from_ast(node: &GrammarASTNode) -> Result<JsonValue, JsonValueError> {
    match node.rule_name.as_str() {
        // -----------------------------------------------------------------
        // Rule: "value"
        // -----------------------------------------------------------------
        //
        // The "value" rule is the start symbol. It wraps exactly one
        // meaningful child — either an ASTNode (object or array) or a
        // Token (STRING, NUMBER, TRUE, FALSE, NULL).
        //
        // Structural tokens (LBRACE, RBRACE, etc.) may also appear as
        // children but are not meaningful — we skip them.
        "value" => {
            // Search children for the first meaningful one.
            for child in &node.children {
                match child {
                    // A child node means we have an object or array.
                    ASTNodeOrToken::Node(child_node) => {
                        return from_ast(child_node);
                    }
                    // A token child — check if it's a value token.
                    ASTNodeOrToken::Token(token) => {
                        if let Some(val) = token_to_json_value(token)? {
                            return Ok(val);
                        }
                        // Skip structural tokens (LBRACE, COMMA, etc.)
                    }
                }
            }
            Err(JsonValueError::new(
                "value node has no meaningful child",
            ))
        }

        // -----------------------------------------------------------------
        // Rule: "object"
        // -----------------------------------------------------------------
        //
        // An object node contains:
        //   LBRACE, [pair, COMMA, pair, COMMA, ...], RBRACE
        //
        // We iterate children, collecting only the "pair" sub-nodes.
        "object" => {
            let mut pairs = Vec::new();
            for child in &node.children {
                if let ASTNodeOrToken::Node(child_node) = child {
                    if child_node.rule_name == "pair" {
                        let (key, value) = extract_pair(child_node)?;
                        pairs.push((key, value));
                    }
                }
            }
            Ok(JsonValue::Object(pairs))
        }

        // -----------------------------------------------------------------
        // Rule: "array"
        // -----------------------------------------------------------------
        //
        // An array node contains:
        //   LBRACKET, [value, COMMA, value, COMMA, ...], RBRACKET
        //
        // We iterate children, collecting "value" sub-nodes. We also
        // handle the edge case where array elements might be direct
        // tokens rather than wrapped in a "value" node.
        "array" => {
            let mut elements = Vec::new();
            for child in &node.children {
                match child {
                    ASTNodeOrToken::Node(child_node) => {
                        if child_node.rule_name == "value" {
                            elements.push(from_ast(child_node)?);
                        }
                    }
                    // Handle direct token children (edge case).
                    ASTNodeOrToken::Token(token) => {
                        if let Some(val) = token_to_json_value(token)? {
                            elements.push(val);
                        }
                    }
                }
            }
            Ok(JsonValue::Array(elements))
        }

        // -----------------------------------------------------------------
        // Rule: "pair"
        // -----------------------------------------------------------------
        //
        // A pair is always: STRING COLON value
        // We extract the key and recurse on the value.
        "pair" => {
            let (key, value) = extract_pair(node)?;
            // Return as a single-pair object — though this case is unusual
            // since pairs are normally processed by the "object" handler.
            Ok(JsonValue::Object(vec![(key, value)]))
        }

        // -----------------------------------------------------------------
        // Unknown rule
        // -----------------------------------------------------------------
        other => Err(JsonValueError::new(format!(
            "unexpected rule name: {other}"
        ))),
    }
}

/// Extract a key-value pair from a "pair" AST node.
///
/// A "pair" node always has three children:
/// 1. A STRING token (the key)
/// 2. A COLON token (the separator — ignored)
/// 3. A "value" node (the value — recursed into)
///
/// Returns `(key_string, json_value)` or an error.
fn extract_pair(
    pair_node: &GrammarASTNode,
) -> Result<(String, JsonValue), JsonValueError> {
    let mut key: Option<String> = None;
    let mut value: Option<JsonValue> = None;

    for child in &pair_node.children {
        match child {
            ASTNodeOrToken::Token(token) => {
                // The STRING token is the key. We identify it by its
                // TokenType — it will be TokenType::String.
                if token.type_ == TokenType::String && key.is_none() {
                    key = Some(decode_json_string(&token.value)?);
                }
                // Skip COLON and other structural tokens.
            }
            ASTNodeOrToken::Node(child_node) => {
                // The "value" child is the pair's value.
                if child_node.rule_name == "value" && value.is_none() {
                    value = Some(from_ast(child_node)?);
                }
            }
        }
    }

    match (key, value) {
        (Some(k), Some(v)) => Ok((k, v)),
        (None, _) => Err(JsonValueError::new("pair node missing STRING key")),
        (_, None) => Err(JsonValueError::new("pair node missing value")),
    }
}

/// Convert a single token to a `JsonValue`, if the token represents a value.
///
/// Returns:
/// - `Ok(Some(value))` for value tokens (STRING, NUMBER, TRUE, FALSE, NULL)
/// - `Ok(None)` for structural tokens (LBRACE, COMMA, etc.) — these are skipped
/// - `Err(...)` if a NUMBER token cannot be parsed
///
/// # Token type detection
///
/// The JSON lexer produces tokens with two type indicators:
/// - `type_: TokenType` — the built-in enum variant (String, Number, etc.)
/// - `type_name: Option<String>` — custom grammar name ("TRUE", "FALSE", "NULL")
///
/// For STRING and NUMBER, `type_` is `TokenType::String` / `TokenType::Number`.
/// For TRUE, FALSE, NULL, `type_` is `TokenType::Name` and `type_name` holds
/// the grammar name.
fn token_to_json_value(
    token: &lexer::token::Token,
) -> Result<Option<JsonValue>, JsonValueError> {
    // Check custom type names first (TRUE, FALSE, NULL).
    if let Some(ref type_name) = token.type_name {
        return match type_name.as_str() {
            "TRUE" => Ok(Some(JsonValue::Bool(true))),
            "FALSE" => Ok(Some(JsonValue::Bool(false))),
            "NULL" => Ok(Some(JsonValue::Null)),
            _ => Ok(None), // Unknown custom type — skip it.
        };
    }

    // Check built-in token types.
    match token.type_ {
        TokenType::String => Ok(Some(JsonValue::String(
            decode_json_string(&token.value)?,
        ))),

        TokenType::Number => {
            // Determine integer vs float by looking at the raw text.
            //
            // If the token value contains '.' or 'e' or 'E', it's a float.
            // Otherwise, it's an integer. This matches the spec's decision
            // rule and the behavior of standard JSON libraries.
            let text = &token.value;
            if text.contains('.') || text.contains('e') || text.contains('E') {
                let f: f64 = text.parse().map_err(|e| {
                    JsonValueError::new(format!(
                        "cannot parse number '{text}' as f64: {e}"
                    ))
                })?;
                Ok(Some(JsonValue::Number(JsonNumber::Float(f))))
            } else {
                let i: i64 = text.parse().map_err(|e| {
                    JsonValueError::new(format!(
                        "cannot parse number '{text}' as i64: {e}"
                    ))
                })?;
                Ok(Some(JsonValue::Number(JsonNumber::Integer(i))))
            }
        }

        // Structural tokens are not values — return None to signal "skip me."
        _ => Ok(None),
    }
}

fn decode_json_string(raw: &str) -> Result<String, JsonValueError> {
    let inner = if raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"') {
        &raw[1..raw.len() - 1]
    } else {
        raw
    };

    let chars: Vec<char> = inner.chars().collect();
    let mut result = String::new();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] != '\\' {
            result.push(chars[i]);
            i += 1;
            continue;
        }

        if i + 1 >= chars.len() {
            result.push('\\');
            break;
        }

        match chars[i + 1] {
            '"' => {
                result.push('"');
                i += 2;
            }
            '\\' => {
                result.push('\\');
                i += 2;
            }
            '/' => {
                result.push('/');
                i += 2;
            }
            'b' => {
                result.push('\u{0008}');
                i += 2;
            }
            'f' => {
                result.push('\u{000C}');
                i += 2;
            }
            'n' => {
                result.push('\n');
                i += 2;
            }
            'r' => {
                result.push('\r');
                i += 2;
            }
            't' => {
                result.push('\t');
                i += 2;
            }
            'u' => {
                if i + 5 >= chars.len() {
                    return Err(JsonValueError::new(
                        "incomplete unicode escape in JSON string",
                    ));
                }

                let hex: String = chars[i + 2..i + 6].iter().collect();
                let first_unit = u16::from_str_radix(&hex, 16).map_err(|_| {
                    JsonValueError::new(format!(
                        "invalid unicode escape in JSON string: \\u{}",
                        hex
                    ))
                })?;
                i += 6;

                if (0xD800..=0xDBFF).contains(&first_unit)
                    && i + 5 < chars.len()
                    && chars[i] == '\\'
                    && chars[i + 1] == 'u'
                {
                    let low_hex: String = chars[i + 2..i + 6].iter().collect();
                    let second_unit = u16::from_str_radix(&low_hex, 16).map_err(|_| {
                        JsonValueError::new(format!(
                            "invalid unicode escape in JSON string: \\u{}",
                            low_hex
                        ))
                    })?;

                    if (0xDC00..=0xDFFF).contains(&second_unit) {
                        result.push_str(&String::from_utf16_lossy(&[
                            first_unit,
                            second_unit,
                        ]));
                        i += 6;
                    } else {
                        result.push_str(&String::from_utf16_lossy(&[first_unit]));
                    }
                } else {
                    result.push_str(&String::from_utf16_lossy(&[first_unit]));
                }
            }
            other => {
                result.push(other);
                i += 2;
            }
        }
    }

    Ok(result)
}

// ===========================================================================
// Convenience: text --> JsonValue
// ===========================================================================

/// Parse JSON text into a `JsonValue`.
///
/// This is the most convenient entry point — it handles the full pipeline:
///
/// ```text
/// JSON text  -->  json-lexer  -->  json-parser  -->  from_ast  -->  JsonValue
/// ```
///
/// # Errors
///
/// Returns `JsonValueError` if the text is not valid JSON or if the AST
/// cannot be converted to a `JsonValue`.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_json_value::parse;
///
/// let value = parse(r#"{"name": "Alice", "age": 30}"#).unwrap();
/// ```
pub fn parse(text: &str) -> Result<JsonValue, JsonValueError> {
    // Step 1: Parse text into an AST using the json-parser crate.
    //
    // parse_json() handles tokenization + grammar loading + parsing.
    // It panics on invalid JSON — we catch that by using std::panic::catch_unwind
    // and convert it to a JsonValueError for a nicer API.
    let ast = std::panic::catch_unwind(|| {
        coding_adventures_json_parser::parse_json(text)
    })
    .map_err(|_| {
        JsonValueError::new(format!("failed to parse JSON text"))
    })?;

    // Step 2: Convert the AST to a JsonValue.
    from_ast(&ast)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =====================================================================
    // Helper: parse JSON text and convert to JsonValue in one step.
    // =====================================================================

    /// Convenience function for tests — parse text and assert success.
    fn parse_ok(text: &str) -> JsonValue {
        parse(text).unwrap_or_else(|e| {
            panic!("Failed to parse JSON text '{text}': {e}")
        })
    }

    // =====================================================================
    // Tests: from_ast() via parse() — primitives
    // =====================================================================

    // -----------------------------------------------------------------
    // Test 1: String value
    // -----------------------------------------------------------------

    /// The simplest JSON string. The lexer already unescapes the quotes,
    /// so `"hello"` in JSON becomes `hello` in the token value.
    #[test]
    fn test_string_value() {
        let val = parse_ok(r#""hello""#);
        assert_eq!(val, JsonValue::String("hello".to_string()));
    }

    // -----------------------------------------------------------------
    // Test 2: Empty string
    // -----------------------------------------------------------------

    /// An empty JSON string `""` should produce an empty Rust String.
    #[test]
    fn test_empty_string() {
        let val = parse_ok(r#""""#);
        assert_eq!(val, JsonValue::String(String::new()));
    }

    // -----------------------------------------------------------------
    // Test 3: Integer number
    // -----------------------------------------------------------------

    /// A plain integer like `42` has no decimal point or exponent, so
    /// it should be stored as `JsonNumber::Integer(42)`.
    #[test]
    fn test_integer_number() {
        let val = parse_ok("42");
        assert_eq!(val, JsonValue::Number(JsonNumber::Integer(42)));
    }

    // -----------------------------------------------------------------
    // Test 4: Zero
    // -----------------------------------------------------------------

    /// Zero is a valid JSON number. It should parse as an integer.
    #[test]
    fn test_zero() {
        let val = parse_ok("0");
        assert_eq!(val, JsonValue::Number(JsonNumber::Integer(0)));
    }

    // -----------------------------------------------------------------
    // Test 5: Negative integer
    // -----------------------------------------------------------------

    /// Negative numbers are a single token in JSON — the minus sign is
    /// part of the NUMBER token, not a separate operator.
    #[test]
    fn test_negative_integer() {
        let val = parse_ok("-17");
        assert_eq!(val, JsonValue::Number(JsonNumber::Integer(-17)));
    }

    // -----------------------------------------------------------------
    // Test 6: Float number
    // -----------------------------------------------------------------

    /// A number with a decimal point should be stored as Float.
    #[test]
    fn test_float_number() {
        let val = parse_ok("3.14");
        assert_eq!(val, JsonValue::Number(JsonNumber::Float(3.14)));
    }

    // -----------------------------------------------------------------
    // Test 7: Exponent number
    // -----------------------------------------------------------------

    /// A number with an exponent (`e` or `E`) is always stored as Float,
    /// even if the result is a whole number (like 1e10 = 10000000000).
    #[test]
    fn test_exponent_number() {
        let val = parse_ok("1e10");
        assert_eq!(val, JsonValue::Number(JsonNumber::Float(1e10)));
    }

    // -----------------------------------------------------------------
    // Test 8: Negative exponent
    // -----------------------------------------------------------------

    /// Negative exponents produce small floats: `2.5E-3` = 0.0025.
    #[test]
    fn test_negative_exponent() {
        let val = parse_ok("2.5E-3");
        assert_eq!(val, JsonValue::Number(JsonNumber::Float(2.5e-3)));
    }

    // -----------------------------------------------------------------
    // Test 9: Boolean true
    // -----------------------------------------------------------------

    /// The JSON literal `true` maps to `JsonValue::Bool(true)`.
    #[test]
    fn test_bool_true() {
        let val = parse_ok("true");
        assert_eq!(val, JsonValue::Bool(true));
    }

    // -----------------------------------------------------------------
    // Test 10: Boolean false
    // -----------------------------------------------------------------

    /// The JSON literal `false` maps to `JsonValue::Bool(false)`.
    #[test]
    fn test_bool_false() {
        let val = parse_ok("false");
        assert_eq!(val, JsonValue::Bool(false));
    }

    // -----------------------------------------------------------------
    // Test 11: Null
    // -----------------------------------------------------------------

    /// The JSON literal `null` maps to `JsonValue::Null`.
    #[test]
    fn test_null() {
        let val = parse_ok("null");
        assert_eq!(val, JsonValue::Null);
    }

    // =====================================================================
    // Tests: from_ast() via parse() — objects
    // =====================================================================

    // -----------------------------------------------------------------
    // Test 12: Empty object
    // -----------------------------------------------------------------

    /// An empty object `{}` should produce `JsonValue::Object` with
    /// an empty pairs vector.
    #[test]
    fn test_empty_object() {
        let val = parse_ok("{}");
        assert_eq!(val, JsonValue::Object(vec![]));
    }

    // -----------------------------------------------------------------
    // Test 13: Simple object with one pair
    // -----------------------------------------------------------------

    /// A single key-value pair. The key is always a string, the value
    /// can be any JSON type.
    #[test]
    fn test_simple_object() {
        let val = parse_ok(r#"{"a": 1}"#);
        assert_eq!(
            val,
            JsonValue::Object(vec![(
                "a".to_string(),
                JsonValue::Number(JsonNumber::Integer(1)),
            )])
        );
    }

    // -----------------------------------------------------------------
    // Test 14: Object with multiple pairs
    // -----------------------------------------------------------------

    /// Multiple key-value pairs. Insertion order is preserved.
    #[test]
    fn test_multi_pair_object() {
        let val = parse_ok(r#"{"a": 1, "b": 2}"#);
        assert_eq!(
            val,
            JsonValue::Object(vec![
                ("a".to_string(), JsonValue::Number(JsonNumber::Integer(1))),
                ("b".to_string(), JsonValue::Number(JsonNumber::Integer(2))),
            ])
        );
    }

    // -----------------------------------------------------------------
    // Test 15: Object with mixed value types
    // -----------------------------------------------------------------

    /// An object whose values span all six JSON types.
    #[test]
    fn test_object_mixed_values() {
        let val = parse_ok(
            r#"{"s": "hello", "n": 42, "f": 3.14, "t": true, "fa": false, "nu": null}"#,
        );
        match val {
            JsonValue::Object(pairs) => {
                assert_eq!(pairs.len(), 6);
                assert_eq!(pairs[0].0, "s");
                assert_eq!(pairs[0].1, JsonValue::String("hello".to_string()));
                assert_eq!(pairs[1].0, "n");
                assert_eq!(pairs[1].1, JsonValue::Number(JsonNumber::Integer(42)));
                assert_eq!(pairs[2].0, "f");
                assert_eq!(pairs[2].1, JsonValue::Number(JsonNumber::Float(3.14)));
                assert_eq!(pairs[3].0, "t");
                assert_eq!(pairs[3].1, JsonValue::Bool(true));
                assert_eq!(pairs[4].0, "fa");
                assert_eq!(pairs[4].1, JsonValue::Bool(false));
                assert_eq!(pairs[5].0, "nu");
                assert_eq!(pairs[5].1, JsonValue::Null);
            }
            other => panic!("Expected Object, got {:?}", other),
        }
    }

    // =====================================================================
    // Tests: from_ast() via parse() — arrays
    // =====================================================================

    // -----------------------------------------------------------------
    // Test 16: Empty array
    // -----------------------------------------------------------------

    /// An empty array `[]` should produce `JsonValue::Array` with an
    /// empty elements vector.
    #[test]
    fn test_empty_array() {
        let val = parse_ok("[]");
        assert_eq!(val, JsonValue::Array(vec![]));
    }

    // -----------------------------------------------------------------
    // Test 17: Simple array
    // -----------------------------------------------------------------

    /// An array of integers: `[1, 2, 3]`.
    #[test]
    fn test_simple_array() {
        let val = parse_ok("[1, 2, 3]");
        assert_eq!(
            val,
            JsonValue::Array(vec![
                JsonValue::Number(JsonNumber::Integer(1)),
                JsonValue::Number(JsonNumber::Integer(2)),
                JsonValue::Number(JsonNumber::Integer(3)),
            ])
        );
    }

    // -----------------------------------------------------------------
    // Test 18: Mixed-type array
    // -----------------------------------------------------------------

    /// An array with mixed types: `[1, "two", true, null]`.
    #[test]
    fn test_mixed_array() {
        let val = parse_ok(r#"[1, "two", true, null]"#);
        assert_eq!(
            val,
            JsonValue::Array(vec![
                JsonValue::Number(JsonNumber::Integer(1)),
                JsonValue::String("two".to_string()),
                JsonValue::Bool(true),
                JsonValue::Null,
            ])
        );
    }

    // =====================================================================
    // Tests: from_ast() via parse() — nested structures
    // =====================================================================

    // -----------------------------------------------------------------
    // Test 19: Nested object
    // -----------------------------------------------------------------

    /// An object containing another object: `{"a": {"b": 1}}`.
    #[test]
    fn test_nested_object() {
        let val = parse_ok(r#"{"a": {"b": 1}}"#);
        assert_eq!(
            val,
            JsonValue::Object(vec![(
                "a".to_string(),
                JsonValue::Object(vec![(
                    "b".to_string(),
                    JsonValue::Number(JsonNumber::Integer(1)),
                )]),
            )])
        );
    }

    // -----------------------------------------------------------------
    // Test 20: Nested array
    // -----------------------------------------------------------------

    /// An array containing arrays: `[[1, 2], [3, 4]]`.
    #[test]
    fn test_nested_array() {
        let val = parse_ok("[[1, 2], [3, 4]]");
        assert_eq!(
            val,
            JsonValue::Array(vec![
                JsonValue::Array(vec![
                    JsonValue::Number(JsonNumber::Integer(1)),
                    JsonValue::Number(JsonNumber::Integer(2)),
                ]),
                JsonValue::Array(vec![
                    JsonValue::Number(JsonNumber::Integer(3)),
                    JsonValue::Number(JsonNumber::Integer(4)),
                ]),
            ])
        );
    }

    // -----------------------------------------------------------------
    // Test 21: Complex nested structure
    // -----------------------------------------------------------------

    /// A realistic JSON document with objects, arrays, and mixed types
    /// at multiple nesting levels.
    #[test]
    fn test_complex_nested() {
        let val = parse_ok(r#"{"users": [{"name": "Alice", "age": 30}]}"#);
        assert_eq!(
            val,
            JsonValue::Object(vec![(
                "users".to_string(),
                JsonValue::Array(vec![JsonValue::Object(vec![
                    ("name".to_string(), JsonValue::String("Alice".to_string())),
                    ("age".to_string(), JsonValue::Number(JsonNumber::Integer(30))),
                ])]),
            )])
        );
    }

    // -----------------------------------------------------------------
    // Test 22: Deeply nested structure
    // -----------------------------------------------------------------

    /// Three levels deep: object -> array -> object.
    #[test]
    fn test_deeply_nested() {
        let val = parse_ok(
            r#"{"users": [{"name": "Alice", "scores": [95, 87]}, {"name": "Bob", "scores": [72]}]}"#,
        );
        match &val {
            JsonValue::Object(pairs) => {
                assert_eq!(pairs.len(), 1);
                assert_eq!(pairs[0].0, "users");
                match &pairs[0].1 {
                    JsonValue::Array(users) => {
                        assert_eq!(users.len(), 2);
                        // First user
                        match &users[0] {
                            JsonValue::Object(user_pairs) => {
                                assert_eq!(user_pairs[0].0, "name");
                                assert_eq!(
                                    user_pairs[0].1,
                                    JsonValue::String("Alice".to_string())
                                );
                                assert_eq!(user_pairs[1].0, "scores");
                                assert_eq!(
                                    user_pairs[1].1,
                                    JsonValue::Array(vec![
                                        JsonValue::Number(JsonNumber::Integer(95)),
                                        JsonValue::Number(JsonNumber::Integer(87)),
                                    ])
                                );
                            }
                            other => panic!("Expected Object, got {:?}", other),
                        }
                    }
                    other => panic!("Expected Array, got {:?}", other),
                }
            }
            other => panic!("Expected Object, got {:?}", other),
        }
    }

    // =====================================================================
    // Tests: from_ast() via parse() — string escapes
    // =====================================================================

    // -----------------------------------------------------------------
    // Test 23: String with escape sequences
    // -----------------------------------------------------------------

    /// The JSON lexer resolves escape sequences, so `"hello\nworld"` in
    /// JSON becomes `hello\nworld` (with a real newline) in the token value.
    #[test]
    fn test_string_with_newline_escape() {
        let val = parse_ok(r#""hello\nworld""#);
        assert_eq!(
            val,
            JsonValue::String("hello\nworld".to_string())
        );
    }

    // -----------------------------------------------------------------
    // Test 24: String with tab escape
    // -----------------------------------------------------------------

    #[test]
    fn test_string_with_tab_escape() {
        let val = parse_ok(r#""hello\tworld""#);
        assert_eq!(
            val,
            JsonValue::String("hello\tworld".to_string())
        );
    }

    // -----------------------------------------------------------------
    // Test 25: String with backslash escape
    // -----------------------------------------------------------------

    #[test]
    fn test_string_with_backslash_escape() {
        let val = parse_ok(r#""path\\to\\file""#);
        assert_eq!(
            val,
            JsonValue::String("path\\to\\file".to_string())
        );
    }

    // -----------------------------------------------------------------
    // Test 26: String with quote escape
    // -----------------------------------------------------------------

    #[test]
    fn test_string_with_quote_escape() {
        let val = parse_ok(r#""say \"hi\"""#);
        assert_eq!(
            val,
            JsonValue::String("say \"hi\"".to_string())
        );
    }

    // =====================================================================
    // Tests: parse() error handling
    // =====================================================================

    // -----------------------------------------------------------------
    // Test 27: Invalid JSON text
    // -----------------------------------------------------------------

    /// Invalid JSON should return an error, not panic.
    #[test]
    fn test_parse_invalid_json() {
        let result = parse("not json");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------
    // Test 28: Incomplete JSON
    // -----------------------------------------------------------------

    /// A truncated JSON document should produce an error.
    #[test]
    fn test_parse_incomplete_json() {
        let result = parse("{");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------
    // Test 29: Empty input
    // -----------------------------------------------------------------

    /// An empty string is not valid JSON.
    #[test]
    fn test_parse_empty_input() {
        let result = parse("");
        assert!(result.is_err());
    }

    // =====================================================================
    // Tests: parse() convenience — whitespace tolerance
    // =====================================================================

    // -----------------------------------------------------------------
    // Test 30: Pretty-printed JSON
    // -----------------------------------------------------------------

    /// JSON with whitespace and newlines should parse identically to
    /// the compact version.
    #[test]
    fn test_parse_pretty_printed() {
        let pretty = r#"{
  "name": "Alice",
  "age": 30
}"#;
        let compact = r#"{"name":"Alice","age":30}"#;
        let val_pretty = parse_ok(pretty);
        let val_compact = parse_ok(compact);
        assert_eq!(val_pretty, val_compact);
    }

    // =====================================================================
    // Tests: JsonValueError display
    // =====================================================================

    // -----------------------------------------------------------------
    // Test 31: Error display format
    // -----------------------------------------------------------------

    #[test]
    fn test_error_display() {
        let err = JsonValueError::new("test error");
        assert_eq!(format!("{err}"), "JsonValueError: test error");
    }

    // =====================================================================
    // Tests: edge cases
    // =====================================================================

    // -----------------------------------------------------------------
    // Test 32: Single-element array
    // -----------------------------------------------------------------

    #[test]
    fn test_single_element_array() {
        let val = parse_ok("[42]");
        assert_eq!(
            val,
            JsonValue::Array(vec![JsonValue::Number(JsonNumber::Integer(42))])
        );
    }

    // -----------------------------------------------------------------
    // Test 33: Array of arrays
    // -----------------------------------------------------------------

    #[test]
    fn test_array_of_empty_arrays() {
        let val = parse_ok("[[], []]");
        assert_eq!(
            val,
            JsonValue::Array(vec![
                JsonValue::Array(vec![]),
                JsonValue::Array(vec![]),
            ])
        );
    }

    // -----------------------------------------------------------------
    // Test 34: Object with empty array value
    // -----------------------------------------------------------------

    #[test]
    fn test_object_with_empty_array() {
        let val = parse_ok(r#"{"items": []}"#);
        assert_eq!(
            val,
            JsonValue::Object(vec![(
                "items".to_string(),
                JsonValue::Array(vec![]),
            )])
        );
    }

    // -----------------------------------------------------------------
    // Test 35: Object with empty object value
    // -----------------------------------------------------------------

    #[test]
    fn test_object_with_empty_object() {
        let val = parse_ok(r#"{"nested": {}}"#);
        assert_eq!(
            val,
            JsonValue::Object(vec![(
                "nested".to_string(),
                JsonValue::Object(vec![]),
            )])
        );
    }

    // -----------------------------------------------------------------
    // Test 36: Large integer
    // -----------------------------------------------------------------

    #[test]
    fn test_large_integer() {
        let val = parse_ok("9007199254740992");
        assert_eq!(
            val,
            JsonValue::Number(JsonNumber::Integer(9007199254740992))
        );
    }

    // -----------------------------------------------------------------
    // Test 37: Negative float
    // -----------------------------------------------------------------

    #[test]
    fn test_negative_float() {
        let val = parse_ok("-3.14");
        assert_eq!(
            val,
            JsonValue::Number(JsonNumber::Float(-3.14))
        );
    }

    // -----------------------------------------------------------------
    // Test 38: Array of booleans and null
    // -----------------------------------------------------------------

    #[test]
    fn test_array_of_booleans_and_null() {
        let val = parse_ok("[true, false, null]");
        assert_eq!(
            val,
            JsonValue::Array(vec![
                JsonValue::Bool(true),
                JsonValue::Bool(false),
                JsonValue::Null,
            ])
        );
    }

    // -----------------------------------------------------------------
    // Test 39: Multiple objects in array
    // -----------------------------------------------------------------

    #[test]
    fn test_array_of_objects() {
        let val = parse_ok(r#"[{"a": 1}, {"b": 2}]"#);
        assert_eq!(
            val,
            JsonValue::Array(vec![
                JsonValue::Object(vec![(
                    "a".to_string(),
                    JsonValue::Number(JsonNumber::Integer(1)),
                )]),
                JsonValue::Object(vec![(
                    "b".to_string(),
                    JsonValue::Number(JsonNumber::Integer(2)),
                )]),
            ])
        );
    }

    // -----------------------------------------------------------------
    // Test 40: String with unicode escape
    // -----------------------------------------------------------------

    /// The JSON `\uXXXX` escape. The value layer should decode unicode
    /// escapes to the actual character.
    #[test]
    fn test_string_with_unicode_escape() {
        let val = parse_ok(r#""\u0041""#);
        assert_eq!(val, JsonValue::String("A".to_string()));
    }
}
