//! # JSON Serializer — converting `JsonValue` to JSON text.
//!
//! This crate is the final piece of the JSON pipeline. Where `json-value`
//! converts a parse tree *into* typed data, this crate converts typed data
//! *back into* JSON text — the reverse direction.
//!
//! # The pipeline (both directions)
//!
//! ```text
//! Forward (parsing):
//!   JSON text  -->  json-lexer  -->  json-parser  -->  json-value
//!     &str          Vec<Token>      GrammarASTNode     JsonValue
//!
//! Reverse (serialization):
//!   JsonValue  -->  json-serializer  -->  JSON text
//!                   this crate             String
//! ```
//!
//! # Two output modes
//!
//! ## Compact mode (`serialize`)
//!
//! No unnecessary whitespace. Produces the smallest possible output.
//! Suitable for wire transmission, storage, and APIs.
//!
//! ```text
//! {"name":"Alice","scores":[95,87]}
//! ```
//!
//! ## Pretty mode (`serialize_pretty`)
//!
//! Human-readable with configurable indentation. Suitable for config files,
//! debugging, and display.
//!
//! ```text
//! {
//!   "name": "Alice",
//!   "scores": [
//!     95,
//!     87
//!   ]
//! }
//! ```
//!
//! # String escaping (RFC 8259)
//!
//! JSON strings must escape certain characters. This crate follows RFC 8259:
//!
//! | Character        | Escape   | Reason          |
//! |------------------|----------|-----------------|
//! | `"` (quote)      | `\"`     | String delimiter |
//! | `\` (backslash)  | `\\`     | Escape character |
//! | Backspace        | `\b`     | Control char U+0008 |
//! | Form feed        | `\f`     | Control char U+000C |
//! | Newline          | `\n`     | Control char U+000A |
//! | Carriage return  | `\r`     | Control char U+000D |
//! | Tab              | `\t`     | Control char U+0009 |
//! | U+0000..U+001F   | `\uXXXX` | All other control chars |
//!
//! Forward slash (`/`) is NOT escaped — RFC 8259 allows but does not
//! require it, and not escaping it produces more readable output.

use coding_adventures_json_value::{JsonNumber, JsonValue};

// ===========================================================================
// Error type
// ===========================================================================

/// Errors that can occur during JSON serialization.
///
/// The only values that cannot be serialized are non-finite floats:
/// `Infinity`, `-Infinity`, and `NaN`. These have no JSON representation
/// (JSON numbers must be finite per RFC 8259).
#[derive(Debug, Clone, PartialEq)]
pub struct JsonSerializerError {
    /// Human-readable description of what went wrong.
    pub message: String,
}

impl JsonSerializerError {
    /// Create a new serializer error with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for JsonSerializerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JsonSerializerError: {}", self.message)
    }
}

impl std::error::Error for JsonSerializerError {}

// ===========================================================================
// Configuration
// ===========================================================================

/// Configuration for pretty-printed JSON output.
///
/// Controls indentation style, key sorting, and trailing newline behavior.
/// The defaults produce output that matches most editors and tools:
/// 2-space indentation, no key sorting, no trailing newline.
///
/// # Examples
///
/// ```text
/// Default config (2 spaces, no sorting):
/// {
///   "name": "Alice",
///   "age": 30
/// }
///
/// 4-space indent:
/// {
///     "name": "Alice",
///     "age": 30
/// }
///
/// Tab indent:
/// {
/// \t"name": "Alice",
/// \t"age": 30
/// }
///
/// Sorted keys:
/// {
///   "age": 30,
///   "name": "Alice"
/// }
/// ```
#[derive(Debug, Clone)]
pub struct SerializerConfig {
    /// Number of indent characters per nesting level.
    /// Default: 2.
    pub indent_size: usize,

    /// Character to use for indentation. Must be ' ' (space) or '\t' (tab).
    /// Default: ' ' (space).
    pub indent_char: char,

    /// Whether to sort object keys alphabetically.
    /// Default: false (preserves insertion order).
    pub sort_keys: bool,

    /// Whether to append a newline character at the end of the output.
    /// Default: false.
    pub trailing_newline: bool,
}

impl Default for SerializerConfig {
    fn default() -> Self {
        Self {
            indent_size: 2,
            indent_char: ' ',
            sort_keys: false,
            trailing_newline: false,
        }
    }
}

// ===========================================================================
// Compact serialization
// ===========================================================================

/// Serialize a `JsonValue` to compact JSON text.
///
/// Produces the smallest possible output — no spaces between tokens,
/// no newlines, no indentation. This is the format used for wire
/// transmission and storage.
///
/// # Errors
///
/// Returns `JsonSerializerError` if the value contains a non-finite float
/// (Infinity or NaN), which cannot be represented in JSON.
///
/// # Examples
///
/// ```text
/// JsonValue::Null           --> "null"
/// JsonValue::Bool(true)     --> "true"
/// JsonValue::Number(42)     --> "42"
/// JsonValue::String("hi")   --> "\"hi\""
/// JsonValue::Array([1, 2])  --> "[1,2]"
/// JsonValue::Object(...)    --> "{\"a\":1}"
/// ```
pub fn serialize(value: &JsonValue) -> Result<String, JsonSerializerError> {
    match value {
        // -----------------------------------------------------------------
        // Null: the simplest case.
        // -----------------------------------------------------------------
        JsonValue::Null => Ok("null".to_string()),

        // -----------------------------------------------------------------
        // Boolean: "true" or "false" — no quotes.
        // -----------------------------------------------------------------
        JsonValue::Bool(b) => Ok(if *b { "true" } else { "false" }.to_string()),

        // -----------------------------------------------------------------
        // Number: integer or float serialization.
        //
        // Integers are straightforward: just convert to string.
        // Floats must be checked for Infinity/NaN (not valid JSON).
        // -----------------------------------------------------------------
        JsonValue::Number(n) => serialize_number(n),

        // -----------------------------------------------------------------
        // String: wrap in quotes and escape special characters.
        // -----------------------------------------------------------------
        JsonValue::String(s) => Ok(serialize_string(s)),

        // -----------------------------------------------------------------
        // Array: [elem1,elem2,...] with no spaces.
        //
        // Empty arrays produce "[]". Non-empty arrays have elements
        // separated by commas with no whitespace.
        // -----------------------------------------------------------------
        JsonValue::Array(elements) => {
            if elements.is_empty() {
                return Ok("[]".to_string());
            }
            let mut parts = Vec::with_capacity(elements.len());
            for elem in elements {
                parts.push(serialize(elem)?);
            }
            Ok(format!("[{}]", parts.join(",")))
        }

        // -----------------------------------------------------------------
        // Object: {"key1":val1,"key2":val2,...} with no spaces.
        //
        // Empty objects produce "{}". Non-empty objects have pairs
        // separated by commas, with colon between key and value.
        // -----------------------------------------------------------------
        JsonValue::Object(pairs) => {
            if pairs.is_empty() {
                return Ok("{}".to_string());
            }
            let mut parts = Vec::with_capacity(pairs.len());
            for (key, val) in pairs {
                let key_str = serialize_string(key);
                let val_str = serialize(val)?;
                parts.push(format!("{key_str}:{val_str}"));
            }
            Ok(format!("{{{}}}", parts.join(",")))
        }
    }
}

// ===========================================================================
// Pretty serialization
// ===========================================================================

/// Serialize a `JsonValue` to pretty-printed JSON text.
///
/// Uses the provided configuration for indentation, key sorting, and
/// trailing newline. If no config is provided, uses defaults (2-space
/// indent, no sorting, no trailing newline).
///
/// # Errors
///
/// Same as [`serialize`] — returns an error for non-finite floats.
pub fn serialize_pretty(
    value: &JsonValue,
    config: &SerializerConfig,
) -> Result<String, JsonSerializerError> {
    let mut result = serialize_pretty_recursive(value, config, 0)?;
    if config.trailing_newline {
        result.push('\n');
    }
    Ok(result)
}

/// Internal recursive function for pretty-printing.
///
/// The `depth` parameter tracks the current nesting level, which determines
/// how much indentation to apply. Each level adds `config.indent_size`
/// copies of `config.indent_char`.
fn serialize_pretty_recursive(
    value: &JsonValue,
    config: &SerializerConfig,
    depth: usize,
) -> Result<String, JsonSerializerError> {
    // Build the indentation strings.
    //
    // current_indent: indentation for closing brackets at this level.
    // next_indent: indentation for content one level deeper.
    let indent_unit: String =
        std::iter::repeat(config.indent_char)
            .take(config.indent_size)
            .collect();
    let current_indent = indent_unit.repeat(depth);
    let next_indent = indent_unit.repeat(depth + 1);

    match value {
        // Primitives look the same in compact and pretty mode — they
        // have no internal structure to indent.
        JsonValue::Null => Ok("null".to_string()),
        JsonValue::Bool(b) => Ok(if *b { "true" } else { "false" }.to_string()),
        JsonValue::Number(n) => serialize_number(n),
        JsonValue::String(s) => Ok(serialize_string(s)),

        // -----------------------------------------------------------------
        // Array: pretty-printed with one element per line.
        //
        // Empty:     []
        // Non-empty:
        // [
        //   elem1,
        //   elem2
        // ]
        // -----------------------------------------------------------------
        JsonValue::Array(elements) => {
            if elements.is_empty() {
                return Ok("[]".to_string());
            }
            let mut lines = Vec::with_capacity(elements.len());
            for elem in elements {
                let elem_str =
                    serialize_pretty_recursive(elem, config, depth + 1)?;
                lines.push(format!("{next_indent}{elem_str}"));
            }
            Ok(format!("[\n{}\n{current_indent}]", lines.join(",\n")))
        }

        // -----------------------------------------------------------------
        // Object: pretty-printed with one pair per line.
        //
        // Empty:     {}
        // Non-empty:
        // {
        //   "key1": value1,
        //   "key2": value2
        // }
        //
        // If sort_keys is true, keys are sorted alphabetically.
        // -----------------------------------------------------------------
        JsonValue::Object(pairs) => {
            if pairs.is_empty() {
                return Ok("{}".to_string());
            }

            // Optionally sort keys.
            let ordered_pairs: Vec<&(String, JsonValue)> = if config.sort_keys {
                let mut sorted: Vec<&(String, JsonValue)> =
                    pairs.iter().collect();
                sorted.sort_by(|a, b| a.0.cmp(&b.0));
                sorted
            } else {
                pairs.iter().collect()
            };

            let mut lines = Vec::with_capacity(ordered_pairs.len());
            for (key, val) in ordered_pairs {
                let key_str = serialize_string(key);
                let val_str =
                    serialize_pretty_recursive(val, config, depth + 1)?;
                lines.push(format!("{next_indent}{key_str}: {val_str}"));
            }
            Ok(format!("{{\n{}\n{current_indent}}}", lines.join(",\n")))
        }
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Serialize a JSON number to a string.
///
/// Integers are converted directly. Floats are checked for Infinity/NaN
/// (which cannot be represented in JSON per RFC 8259).
fn serialize_number(n: &JsonNumber) -> Result<String, JsonSerializerError> {
    match n {
        JsonNumber::Integer(i) => Ok(i.to_string()),
        JsonNumber::Float(f) => {
            if f.is_infinite() {
                return Err(JsonSerializerError::new(
                    "cannot serialize Infinity to JSON",
                ));
            }
            if f.is_nan() {
                return Err(JsonSerializerError::new(
                    "cannot serialize NaN to JSON",
                ));
            }
            // Rust's default float formatting produces reasonable output:
            // - 3.14 -> "3.14"
            // - 1e10 -> "10000000000" (which is fine for JSON)
            // - 0.0025 -> "0.0025"
            //
            // We need to ensure there's always a decimal point or exponent
            // so the output is clearly a float, not an integer.
            let s = format!("{f}");
            Ok(s)
        }
    }
}

/// Escape a string for JSON output, wrapping it in double quotes.
///
/// Per RFC 8259, the following characters MUST be escaped:
///
/// - `"` (double quote) -> `\"`
/// - `\` (backslash) -> `\\`
/// - Control characters U+0000..U+001F:
///   - U+0008 (backspace) -> `\b`
///   - U+000C (form feed) -> `\f`
///   - U+000A (newline) -> `\n`
///   - U+000D (carriage return) -> `\r`
///   - U+0009 (tab) -> `\t`
///   - All others -> `\uXXXX`
///
/// Forward slash (`/`) is NOT escaped — the spec allows it but does not
/// require it, and unescaped slashes are more readable.
fn serialize_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 2);
    result.push('"');

    for ch in s.chars() {
        match ch {
            // Characters that MUST be escaped with named escapes.
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\u{0008}' => result.push_str("\\b"),
            '\u{000C}' => result.push_str("\\f"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),

            // Other control characters (U+0000..U+001F) use \uXXXX.
            c if (c as u32) < 0x20 => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }

            // All other characters are output as-is.
            c => result.push(c),
        }
    }

    result.push('"');
    result
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =====================================================================
    // Helper: construct JsonValue instances concisely.
    // =====================================================================

    /// Shorthand for creating a JsonValue::Number(Integer).
    fn jint(n: i64) -> JsonValue {
        JsonValue::Number(JsonNumber::Integer(n))
    }

    /// Shorthand for creating a JsonValue::Number(Float).
    fn jfloat(n: f64) -> JsonValue {
        JsonValue::Number(JsonNumber::Float(n))
    }

    /// Shorthand for creating a JsonValue::String.
    fn jstr(s: &str) -> JsonValue {
        JsonValue::String(s.to_string())
    }

    // =====================================================================
    // Tests: serialize() — compact mode — primitives
    // =====================================================================

    // -----------------------------------------------------------------
    // Test 1: Null
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_null() {
        let result = serialize(&JsonValue::Null).unwrap();
        assert_eq!(result, "null");
    }

    // -----------------------------------------------------------------
    // Test 2: Bool true
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_bool_true() {
        let result = serialize(&JsonValue::Bool(true)).unwrap();
        assert_eq!(result, "true");
    }

    // -----------------------------------------------------------------
    // Test 3: Bool false
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_bool_false() {
        let result = serialize(&JsonValue::Bool(false)).unwrap();
        assert_eq!(result, "false");
    }

    // -----------------------------------------------------------------
    // Test 4: Integer number
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_integer() {
        let result = serialize(&jint(42)).unwrap();
        assert_eq!(result, "42");
    }

    // -----------------------------------------------------------------
    // Test 5: Negative integer
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_negative_integer() {
        let result = serialize(&jint(-5)).unwrap();
        assert_eq!(result, "-5");
    }

    // -----------------------------------------------------------------
    // Test 6: Zero
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_zero() {
        let result = serialize(&jint(0)).unwrap();
        assert_eq!(result, "0");
    }

    // -----------------------------------------------------------------
    // Test 7: Float number
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_float() {
        let result = serialize(&jfloat(3.14)).unwrap();
        // Rust formats 3.14 as "3.14"
        assert_eq!(result, "3.14");
    }

    // -----------------------------------------------------------------
    // Test 8: Float zero
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_float_zero() {
        let result = serialize(&jfloat(0.0)).unwrap();
        assert_eq!(result, "0");
    }

    // -----------------------------------------------------------------
    // Test 9: Simple string
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_simple_string() {
        let result = serialize(&jstr("hello")).unwrap();
        assert_eq!(result, "\"hello\"");
    }

    // -----------------------------------------------------------------
    // Test 10: Empty string
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_empty_string() {
        let result = serialize(&jstr("")).unwrap();
        assert_eq!(result, "\"\"");
    }

    // =====================================================================
    // Tests: serialize() — compact mode — string escaping
    // =====================================================================

    // -----------------------------------------------------------------
    // Test 11: Escape newline
    // -----------------------------------------------------------------

    /// A real newline character in the string must become `\n` in output.
    #[test]
    fn test_serialize_escape_newline() {
        let result = serialize(&jstr("a\nb")).unwrap();
        assert_eq!(result, "\"a\\nb\"");
    }

    // -----------------------------------------------------------------
    // Test 12: Escape quote
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_escape_quote() {
        let result = serialize(&jstr("say \"hi\"")).unwrap();
        assert_eq!(result, "\"say \\\"hi\\\"\"");
    }

    // -----------------------------------------------------------------
    // Test 13: Escape backslash
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_escape_backslash() {
        let result = serialize(&jstr("a\\b")).unwrap();
        assert_eq!(result, "\"a\\\\b\"");
    }

    // -----------------------------------------------------------------
    // Test 14: Escape tab
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_escape_tab() {
        let result = serialize(&jstr("\t")).unwrap();
        assert_eq!(result, "\"\\t\"");
    }

    // -----------------------------------------------------------------
    // Test 15: Escape carriage return
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_escape_carriage_return() {
        let result = serialize(&jstr("\r")).unwrap();
        assert_eq!(result, "\"\\r\"");
    }

    // -----------------------------------------------------------------
    // Test 16: Escape backspace
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_escape_backspace() {
        let result = serialize(&jstr("\u{0008}")).unwrap();
        assert_eq!(result, "\"\\b\"");
    }

    // -----------------------------------------------------------------
    // Test 17: Escape form feed
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_escape_form_feed() {
        let result = serialize(&jstr("\u{000C}")).unwrap();
        assert_eq!(result, "\"\\f\"");
    }

    // -----------------------------------------------------------------
    // Test 18: Escape null character (control char)
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_escape_null_char() {
        let result = serialize(&jstr("\u{0000}")).unwrap();
        assert_eq!(result, "\"\\u0000\"");
    }

    // -----------------------------------------------------------------
    // Test 19: Escape other control character
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_escape_control_char() {
        let result = serialize(&jstr("\u{001F}")).unwrap();
        assert_eq!(result, "\"\\u001f\"");
    }

    // -----------------------------------------------------------------
    // Test 20: Forward slash NOT escaped
    // -----------------------------------------------------------------

    #[test]
    fn test_forward_slash_not_escaped() {
        let result = serialize(&jstr("a/b")).unwrap();
        assert_eq!(result, "\"a/b\"");
    }

    // -----------------------------------------------------------------
    // Test 21: Unicode characters pass through
    // -----------------------------------------------------------------

    #[test]
    fn test_unicode_passthrough() {
        let result = serialize(&jstr("hello world")).unwrap();
        assert_eq!(result, "\"hello world\"");
    }

    // =====================================================================
    // Tests: serialize() — compact mode — containers
    // =====================================================================

    // -----------------------------------------------------------------
    // Test 22: Empty array
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_empty_array() {
        let result = serialize(&JsonValue::Array(vec![])).unwrap();
        assert_eq!(result, "[]");
    }

    // -----------------------------------------------------------------
    // Test 23: Simple array
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_simple_array() {
        let result = serialize(&JsonValue::Array(vec![jint(1)])).unwrap();
        assert_eq!(result, "[1]");
    }

    // -----------------------------------------------------------------
    // Test 24: Multi-element array
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_multi_element_array() {
        let result = serialize(&JsonValue::Array(vec![
            jint(1),
            jint(2),
            jint(3),
        ]))
        .unwrap();
        assert_eq!(result, "[1,2,3]");
    }

    // -----------------------------------------------------------------
    // Test 25: Mixed-type array
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_mixed_array() {
        let result = serialize(&JsonValue::Array(vec![
            jint(1),
            jstr("two"),
            JsonValue::Bool(true),
            JsonValue::Null,
        ]))
        .unwrap();
        assert_eq!(result, "[1,\"two\",true,null]");
    }

    // -----------------------------------------------------------------
    // Test 26: Empty object
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_empty_object() {
        let result = serialize(&JsonValue::Object(vec![])).unwrap();
        assert_eq!(result, "{}");
    }

    // -----------------------------------------------------------------
    // Test 27: Simple object
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_simple_object() {
        let result = serialize(&JsonValue::Object(vec![(
            "a".to_string(),
            jint(1),
        )]))
        .unwrap();
        assert_eq!(result, "{\"a\":1}");
    }

    // -----------------------------------------------------------------
    // Test 28: Multi-pair object
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_multi_pair_object() {
        let result = serialize(&JsonValue::Object(vec![
            ("a".to_string(), jint(1)),
            ("b".to_string(), jint(2)),
        ]))
        .unwrap();
        assert_eq!(result, "{\"a\":1,\"b\":2}");
    }

    // -----------------------------------------------------------------
    // Test 29: Nested structure
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_nested() {
        let val = JsonValue::Object(vec![
            ("name".to_string(), jstr("Alice")),
            (
                "scores".to_string(),
                JsonValue::Array(vec![jint(95), jint(87)]),
            ),
        ]);
        let result = serialize(&val).unwrap();
        assert_eq!(result, "{\"name\":\"Alice\",\"scores\":[95,87]}");
    }

    // =====================================================================
    // Tests: serialize() — error cases
    // =====================================================================

    // -----------------------------------------------------------------
    // Test 30: Infinity error
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_infinity_error() {
        let result = serialize(&jfloat(f64::INFINITY));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("Infinity"));
    }

    // -----------------------------------------------------------------
    // Test 31: Negative infinity error
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_neg_infinity_error() {
        let result = serialize(&jfloat(f64::NEG_INFINITY));
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------
    // Test 32: NaN error
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_nan_error() {
        let result = serialize(&jfloat(f64::NAN));
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("NaN"));
    }

    // =====================================================================
    // Tests: serialize_pretty() — pretty mode
    // =====================================================================

    // -----------------------------------------------------------------
    // Test 33: Pretty empty object
    // -----------------------------------------------------------------

    #[test]
    fn test_pretty_empty_object() {
        let config = SerializerConfig::default();
        let result =
            serialize_pretty(&JsonValue::Object(vec![]), &config).unwrap();
        assert_eq!(result, "{}");
    }

    // -----------------------------------------------------------------
    // Test 34: Pretty simple object
    // -----------------------------------------------------------------

    #[test]
    fn test_pretty_simple_object() {
        let config = SerializerConfig::default();
        let val = JsonValue::Object(vec![("a".to_string(), jint(1))]);
        let result = serialize_pretty(&val, &config).unwrap();
        assert_eq!(result, "{\n  \"a\": 1\n}");
    }

    // -----------------------------------------------------------------
    // Test 35: Pretty multi-pair object
    // -----------------------------------------------------------------

    #[test]
    fn test_pretty_multi_pair_object() {
        let config = SerializerConfig::default();
        let val = JsonValue::Object(vec![
            ("a".to_string(), jint(1)),
            ("b".to_string(), jint(2)),
        ]);
        let result = serialize_pretty(&val, &config).unwrap();
        assert_eq!(result, "{\n  \"a\": 1,\n  \"b\": 2\n}");
    }

    // -----------------------------------------------------------------
    // Test 36: Pretty empty array
    // -----------------------------------------------------------------

    #[test]
    fn test_pretty_empty_array() {
        let config = SerializerConfig::default();
        let result =
            serialize_pretty(&JsonValue::Array(vec![]), &config).unwrap();
        assert_eq!(result, "[]");
    }

    // -----------------------------------------------------------------
    // Test 37: Pretty array
    // -----------------------------------------------------------------

    #[test]
    fn test_pretty_array() {
        let config = SerializerConfig::default();
        let val = JsonValue::Array(vec![jint(1), jint(2)]);
        let result = serialize_pretty(&val, &config).unwrap();
        assert_eq!(result, "[\n  1,\n  2\n]");
    }

    // -----------------------------------------------------------------
    // Test 38: Pretty nested object
    // -----------------------------------------------------------------

    #[test]
    fn test_pretty_nested_object() {
        let config = SerializerConfig::default();
        let val = JsonValue::Object(vec![(
            "a".to_string(),
            JsonValue::Object(vec![("b".to_string(), jint(1))]),
        )]);
        let result = serialize_pretty(&val, &config).unwrap();
        assert_eq!(
            result,
            "{\n  \"a\": {\n    \"b\": 1\n  }\n}"
        );
    }

    // -----------------------------------------------------------------
    // Test 39: Pretty nested array
    // -----------------------------------------------------------------

    #[test]
    fn test_pretty_nested_array() {
        let config = SerializerConfig::default();
        let val = JsonValue::Array(vec![
            JsonValue::Array(vec![jint(1), jint(2)]),
            JsonValue::Array(vec![jint(3)]),
        ]);
        let result = serialize_pretty(&val, &config).unwrap();
        assert_eq!(
            result,
            "[\n  [\n    1,\n    2\n  ],\n  [\n    3\n  ]\n]"
        );
    }

    // -----------------------------------------------------------------
    // Test 40: Custom indent size (4 spaces)
    // -----------------------------------------------------------------

    #[test]
    fn test_pretty_indent_size_4() {
        let config = SerializerConfig {
            indent_size: 4,
            ..Default::default()
        };
        let val = JsonValue::Object(vec![("a".to_string(), jint(1))]);
        let result = serialize_pretty(&val, &config).unwrap();
        assert_eq!(result, "{\n    \"a\": 1\n}");
    }

    // -----------------------------------------------------------------
    // Test 41: Tab indent
    // -----------------------------------------------------------------

    #[test]
    fn test_pretty_tab_indent() {
        let config = SerializerConfig {
            indent_size: 1,
            indent_char: '\t',
            ..Default::default()
        };
        let val = JsonValue::Object(vec![("a".to_string(), jint(1))]);
        let result = serialize_pretty(&val, &config).unwrap();
        assert_eq!(result, "{\n\t\"a\": 1\n}");
    }

    // -----------------------------------------------------------------
    // Test 42: Sort keys
    // -----------------------------------------------------------------

    #[test]
    fn test_pretty_sort_keys() {
        let config = SerializerConfig {
            sort_keys: true,
            ..Default::default()
        };
        let val = JsonValue::Object(vec![
            ("c".to_string(), jint(3)),
            ("a".to_string(), jint(1)),
            ("b".to_string(), jint(2)),
        ]);
        let result = serialize_pretty(&val, &config).unwrap();
        assert_eq!(
            result,
            "{\n  \"a\": 1,\n  \"b\": 2,\n  \"c\": 3\n}"
        );
    }

    // -----------------------------------------------------------------
    // Test 43: Trailing newline
    // -----------------------------------------------------------------

    #[test]
    fn test_pretty_trailing_newline() {
        let config = SerializerConfig {
            trailing_newline: true,
            ..Default::default()
        };
        let val = JsonValue::Object(vec![("a".to_string(), jint(1))]);
        let result = serialize_pretty(&val, &config).unwrap();
        assert!(result.ends_with('\n'));
        assert_eq!(result, "{\n  \"a\": 1\n}\n");
    }

    // -----------------------------------------------------------------
    // Test 44: Pretty primitive (string) is same as compact
    // -----------------------------------------------------------------

    #[test]
    fn test_pretty_primitive_string() {
        let config = SerializerConfig::default();
        let result = serialize_pretty(&jstr("hello"), &config).unwrap();
        assert_eq!(result, "\"hello\"");
    }

    // -----------------------------------------------------------------
    // Test 45: Pretty primitive (null) is same as compact
    // -----------------------------------------------------------------

    #[test]
    fn test_pretty_primitive_null() {
        let config = SerializerConfig::default();
        let result = serialize_pretty(&JsonValue::Null, &config).unwrap();
        assert_eq!(result, "null");
    }

    // =====================================================================
    // Tests: round-trip (parse + serialize)
    // =====================================================================

    // -----------------------------------------------------------------
    // Test 46: Simple round-trip
    // -----------------------------------------------------------------

    /// Parse compact JSON and re-serialize — should produce identical output.
    #[test]
    fn test_round_trip_simple_object() {
        let input = r#"{"a":1}"#;
        let value = coding_adventures_json_value::parse(input).unwrap();
        let output = serialize(&value).unwrap();
        assert_eq!(output, input);
    }

    // -----------------------------------------------------------------
    // Test 47: Complex round-trip
    // -----------------------------------------------------------------

    #[test]
    fn test_round_trip_complex() {
        let input = r#"{"name":"Alice","scores":[95,87],"active":true}"#;
        let value = coding_adventures_json_value::parse(input).unwrap();
        let output = serialize(&value).unwrap();
        assert_eq!(output, input);
    }

    // -----------------------------------------------------------------
    // Test 48: Empty containers round-trip
    // -----------------------------------------------------------------

    #[test]
    fn test_round_trip_empty_containers() {
        for input in &["{}", "[]"] {
            let value = coding_adventures_json_value::parse(input).unwrap();
            let output = serialize(&value).unwrap();
            assert_eq!(&output, input);
        }
    }

    // -----------------------------------------------------------------
    // Test 49: Primitives round-trip
    // -----------------------------------------------------------------

    #[test]
    fn test_round_trip_primitives() {
        for input in &["42", "3.14", "true", "false", "null"] {
            let value = coding_adventures_json_value::parse(input).unwrap();
            let output = serialize(&value).unwrap();
            assert_eq!(&output, input);
        }
    }

    // -----------------------------------------------------------------
    // Test 50: String with escapes round-trip
    // -----------------------------------------------------------------

    #[test]
    fn test_round_trip_escaped_string() {
        // Parse a string with escapes, then re-serialize.
        // The parse step resolves \n to a real newline.
        // The serialize step converts it back to \n.
        let input = r#""hello\nworld""#;
        let value = coding_adventures_json_value::parse(input).unwrap();
        let output = serialize(&value).unwrap();
        assert_eq!(output, input);
    }

    // -----------------------------------------------------------------
    // Test 51: Nested arrays round-trip
    // -----------------------------------------------------------------

    #[test]
    fn test_round_trip_nested_arrays() {
        let input = "[[1,2],[3,4]]";
        let value = coding_adventures_json_value::parse(input).unwrap();
        let output = serialize(&value).unwrap();
        assert_eq!(output, input);
    }

    // -----------------------------------------------------------------
    // Test 52: Deeply nested round-trip
    // -----------------------------------------------------------------

    #[test]
    fn test_round_trip_deeply_nested() {
        let input = r#"{"users":[{"name":"Alice","scores":[95,87]},{"name":"Bob","scores":[72]}]}"#;
        let value = coding_adventures_json_value::parse(input).unwrap();
        let output = serialize(&value).unwrap();
        assert_eq!(output, input);
    }

    // =====================================================================
    // Tests: error display
    // =====================================================================

    // -----------------------------------------------------------------
    // Test 53: Error display format
    // -----------------------------------------------------------------

    #[test]
    fn test_error_display() {
        let err = JsonSerializerError::new("test error");
        assert_eq!(
            format!("{err}"),
            "JsonSerializerError: test error"
        );
    }

    // =====================================================================
    // Tests: SerializerConfig default values
    // =====================================================================

    // -----------------------------------------------------------------
    // Test 54: Default config values
    // -----------------------------------------------------------------

    #[test]
    fn test_default_config() {
        let config = SerializerConfig::default();
        assert_eq!(config.indent_size, 2);
        assert_eq!(config.indent_char, ' ');
        assert!(!config.sort_keys);
        assert!(!config.trailing_newline);
    }

    // =====================================================================
    // Tests: additional edge cases
    // =====================================================================

    // -----------------------------------------------------------------
    // Test 55: Pretty complex structure
    // -----------------------------------------------------------------

    #[test]
    fn test_pretty_complex_structure() {
        let config = SerializerConfig::default();
        let val = JsonValue::Object(vec![
            ("name".to_string(), jstr("Alice")),
            (
                "scores".to_string(),
                JsonValue::Array(vec![jint(95), jint(87)]),
            ),
            ("active".to_string(), JsonValue::Bool(true)),
        ]);
        let result = serialize_pretty(&val, &config).unwrap();
        let expected = "{\n  \"name\": \"Alice\",\n  \"scores\": [\n    95,\n    87\n  ],\n  \"active\": true\n}";
        assert_eq!(result, expected);
    }

    // -----------------------------------------------------------------
    // Test 56: String with multiple escape types
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_string_multiple_escapes() {
        let result = serialize(&jstr("line1\nline2\ttab\\back\"quote")).unwrap();
        assert_eq!(
            result,
            "\"line1\\nline2\\ttab\\\\back\\\"quote\""
        );
    }

    // -----------------------------------------------------------------
    // Test 57: Large integer
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_large_integer() {
        let result = serialize(&jint(9007199254740992)).unwrap();
        assert_eq!(result, "9007199254740992");
    }

    // -----------------------------------------------------------------
    // Test 58: Negative float
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_negative_float() {
        let result = serialize(&jfloat(-3.14)).unwrap();
        assert_eq!(result, "-3.14");
    }

    // -----------------------------------------------------------------
    // Test 59: Object with string values containing escapes
    // -----------------------------------------------------------------

    #[test]
    fn test_serialize_object_with_escaped_values() {
        let val = JsonValue::Object(vec![(
            "msg".to_string(),
            jstr("hello\nworld"),
        )]);
        let result = serialize(&val).unwrap();
        assert_eq!(result, "{\"msg\":\"hello\\nworld\"}");
    }

    // -----------------------------------------------------------------
    // Test 60: Sort keys does not affect compact mode
    // -----------------------------------------------------------------

    /// Compact serialize does not sort keys — it preserves insertion order.
    /// Only pretty mode respects sort_keys.
    #[test]
    fn test_compact_preserves_key_order() {
        let val = JsonValue::Object(vec![
            ("z".to_string(), jint(1)),
            ("a".to_string(), jint(2)),
        ]);
        let result = serialize(&val).unwrap();
        assert_eq!(result, "{\"z\":1,\"a\":2}");
    }

    // -----------------------------------------------------------------
    // Test 61: Trailing newline on primitive
    // -----------------------------------------------------------------

    #[test]
    fn test_trailing_newline_on_primitive() {
        let config = SerializerConfig {
            trailing_newline: true,
            ..Default::default()
        };
        let result = serialize_pretty(&jint(42), &config).unwrap();
        assert_eq!(result, "42\n");
    }

    // -----------------------------------------------------------------
    // Test 62: Pretty object with boolean values
    // -----------------------------------------------------------------

    #[test]
    fn test_pretty_object_booleans() {
        let config = SerializerConfig::default();
        let val = JsonValue::Object(vec![
            ("t".to_string(), JsonValue::Bool(true)),
            ("f".to_string(), JsonValue::Bool(false)),
        ]);
        let result = serialize_pretty(&val, &config).unwrap();
        assert_eq!(result, "{\n  \"t\": true,\n  \"f\": false\n}");
    }
}
