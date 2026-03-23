//! # Lattice value types — the type system for compile-time evaluation.
//!
//! Lattice expressions are evaluated at compile time (there is no runtime —
//! the output is static CSS). The evaluator works with a small set of value
//! types that mirror CSS values:
//!
//! | Rust type                    | Example         | CSS concept        |
//! |------------------------------|-----------------|---------------------|
//! | `LatticeValue::Number`       | `42`, `3.14`    | CSS NUMBER token    |
//! | `LatticeValue::Dimension`    | `16px`, `2em`   | CSS DIMENSION token |
//! | `LatticeValue::Percentage`   | `50%`, `100%`   | CSS PERCENTAGE token|
//! | `LatticeValue::String`       | `"Helvetica"`   | CSS STRING token    |
//! | `LatticeValue::Ident`        | `red`, `bold`   | CSS IDENT token     |
//! | `LatticeValue::Color`        | `#4a90d9`       | CSS HASH token      |
//! | `LatticeValue::Bool`         | `true`, `false` | Lattice-only        |
//! | `LatticeValue::Null`         | `null`          | Lattice-only        |
//! | `LatticeValue::List`         | `red, blue`     | For @each           |
//!
//! # Truthiness
//!
//! For `@if` conditions, values are coerced to boolean:
//!
//! - `false` → falsy
//! - `null` → falsy
//! - `0` (Number with value 0) → falsy
//! - Everything else → truthy
//!
//! # Formatting
//!
//! Each value type knows how to emit itself as CSS text. The `to_css_string`
//! method produces the canonical CSS representation:
//! - `Number(42.0)` → `"42"` (integers without decimal point)
//! - `Dimension { value: 16.0, unit: "px" }` → `"16px"`
//! - `Bool(true)` → `"true"` (Lattice booleans stringify for debugging)
//! - `Null` → `""` (like Sass — null is invisible in output)

/// All possible value types in the Lattice expression evaluator.
///
/// These are compile-time values — every expression in Lattice is evaluated
/// at compile time and converted to a CSS text string before emitting.
#[derive(Debug, Clone, PartialEq)]
pub enum LatticeValue {
    /// A pure number without a CSS unit.
    ///
    /// Examples: `42`, `3.14`, `0`, `-1`
    ///
    /// Operations: can be added, subtracted, multiplied with other Numbers;
    /// can scale Dimension and Percentage by multiplication.
    Number(f64),

    /// A number with a CSS unit.
    ///
    /// Examples: `16px`, `2em`, `1.5rem`, `100vh`, `300ms`
    ///
    /// Operations: Dimension ± Dimension is valid only when units match.
    /// `10px + 5px` → `15px`. `10px + 5em` is a type error.
    Dimension {
        value: f64,
        unit: String,
    },

    /// A percentage value.
    ///
    /// Examples: `50%`, `100%`, `33.33%`
    ///
    /// Operations: Percentage ± Percentage → Percentage.
    Percentage(f64),

    /// A quoted string value.
    ///
    /// The quotes are NOT stored — they're added back in CSS output.
    /// Examples: `"hello"` is stored as `String("hello")`.
    String(String),

    /// An unquoted identifier.
    ///
    /// CSS color keywords (red, blue), property values (bold, italic),
    /// theme names (dark, light) — all are stored as `Ident`.
    ///
    /// Note: `true`, `false`, and `null` are NOT stored as Ident — they
    /// are parsed into `Bool` and `Null` variants.
    Ident(String),

    /// A hex color value.
    ///
    /// The `#` prefix IS stored. Examples: `#4a90d9`, `#fff`, `#00000080`.
    Color(String),

    /// A boolean value.
    ///
    /// `true` and `false` are Lattice-only keywords. They appear in `@if`
    /// conditions and as function return values.
    Bool(bool),

    /// The null value.
    ///
    /// `null` is falsy and stringifies to an empty string, like Sass's null.
    /// Used for optional parameters and missing values.
    Null,

    /// A comma-separated list of values.
    ///
    /// Used in `@each $color in red, green, blue { ... }` and multi-value
    /// declarations. Each item is a nested `LatticeValue`.
    List(Vec<LatticeValue>),
}

impl LatticeValue {
    /// Convert this value to its CSS text representation.
    ///
    /// This is the canonical form used for substituting values into CSS output.
    /// The rules:
    /// - Numbers: integers emit without decimal point (`42` not `42.0`)
    /// - Dimensions: value + unit, no space (`16px`)
    /// - Percentages: value + `%`, no space (`50%`)
    /// - Strings: surrounded by double quotes (`"hello"`)
    /// - Idents: as-is (`red`, `bold`)
    /// - Colors: as-is including `#` (`#4a90d9`)
    /// - Booleans: `true` or `false` (string representation)
    /// - Null: empty string (invisible in CSS output, like Sass null)
    /// - Lists: comma+space separated (`red, green, blue`)
    pub fn to_css_string(&self) -> String {
        match self {
            LatticeValue::Number(n) => format_float(*n),
            LatticeValue::Dimension { value, unit } => {
                format!("{}{}", format_float(*value), unit)
            }
            LatticeValue::Percentage(p) => {
                format!("{}%", format_float(*p))
            }
            LatticeValue::String(s) => format!("\"{}\"", s),
            LatticeValue::Ident(s) => s.clone(),
            LatticeValue::Color(s) => s.clone(),
            LatticeValue::Bool(b) => if *b { "true" } else { "false" }.to_string(),
            LatticeValue::Null => String::new(),
            LatticeValue::List(items) => {
                items.iter()
                    .map(|v| v.to_css_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        }
    }

    /// Determine whether this value is truthy for `@if` conditions.
    ///
    /// Truthiness rules (matching Sass conventions):
    ///
    /// | Value              | Truthy? |
    /// |--------------------|---------|
    /// | `false`            | No      |
    /// | `null`             | No      |
    /// | `Number(0.0)`      | No      |
    /// | Everything else    | Yes     |
    ///
    /// Note: unlike JavaScript, empty strings and empty lists ARE truthy.
    pub fn is_truthy(&self) -> bool {
        match self {
            LatticeValue::Bool(false) => false,
            LatticeValue::Null => false,
            LatticeValue::Number(n) if *n == 0.0 => false,
            _ => true,
        }
    }

    /// Get the type name of this value as a string, for error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            LatticeValue::Number(_) => "number",
            LatticeValue::Dimension { .. } => "dimension",
            LatticeValue::Percentage(_) => "percentage",
            LatticeValue::String(_) => "string",
            LatticeValue::Ident(_) => "ident",
            LatticeValue::Color(_) => "color",
            LatticeValue::Bool(_) => "bool",
            LatticeValue::Null => "null",
            LatticeValue::List(_) => "list",
        }
    }
}

/// Format a float without unnecessary trailing zeros.
///
/// The rule: if the float is exactly representable as an integer (no fractional
/// part), emit it as an integer. Otherwise, emit the float as-is.
///
/// Examples:
/// - `42.0` → `"42"`
/// - `3.14` → `"3.14"`
/// - `0.5` → `"0.5"`
/// - `-1.0` → `"-1"`
fn format_float(f: f64) -> String {
    if f == f.trunc() && f.is_finite() {
        // Integer-valued float: emit without decimal point
        format!("{}", f as i64)
    } else {
        format!("{}", f)
    }
}

// ===========================================================================
// Token → LatticeValue conversion
// ===========================================================================

/// Convert a token's type name and value to a `LatticeValue`.
///
/// This is the bridge between the parser's token world and the evaluator's
/// value world. Each CSS/Lattice token type maps to a value variant:
///
/// | Token type   | LatticeValue variant                          |
/// |--------------|----------------------------------------------|
/// | NUMBER       | `Number(f64)`                                |
/// | DIMENSION    | `Dimension { value, unit }`                  |
/// | PERCENTAGE   | `Percentage(f64)` (strips trailing %)        |
/// | STRING       | `String(text)` (quotes already stripped)     |
/// | HASH         | `Color(text)` (with # prefix)                |
/// | IDENT        | `Ident` or `Bool` or `Null` for literals      |
///
/// # Dimension Parsing
///
/// DIMENSION tokens combine the number and unit, e.g. `"16px"`. We split
/// on the first alphabetic character to separate `"16"` from `"px"`.
/// Negative dimensions like `"-1rem"` are handled by checking for a leading
/// minus sign.
pub fn token_to_value(type_name: &str, value: &str) -> LatticeValue {
    match type_name {
        "Number" | "NUMBER" => {
            LatticeValue::Number(value.parse::<f64>().unwrap_or(0.0))
        }

        "DIMENSION" => {
            // Split "16px" → (16.0, "px"), "-1.5rem" → (-1.5, "rem")
            let (num_str, unit) = split_dimension(value);
            let num = num_str.parse::<f64>().unwrap_or(0.0);
            LatticeValue::Dimension {
                value: num,
                unit: unit.to_string(),
            }
        }

        "PERCENTAGE" => {
            // "50%" → 50.0 (strip the %)
            let num_str = value.trim_end_matches('%');
            LatticeValue::Percentage(num_str.parse::<f64>().unwrap_or(0.0))
        }

        "String" | "STRING" => {
            // Quotes are already stripped by the lexer
            LatticeValue::String(value.to_string())
        }

        "HASH" => {
            // Keep the # prefix: "#4a90d9"
            LatticeValue::Color(value.to_string())
        }

        "Ident" | "IDENT" => {
            // Special idents: true, false, null
            match value {
                "true" => LatticeValue::Bool(true),
                "false" => LatticeValue::Bool(false),
                "null" => LatticeValue::Null,
                _ => LatticeValue::Ident(value.to_string()),
            }
        }

        _ => {
            // Unknown token type — treat as ident
            LatticeValue::Ident(value.to_string())
        }
    }
}

/// Split a DIMENSION token value into its numeric part and unit.
///
/// DIMENSION tokens look like: `"16px"`, `"2em"`, `"-1.5rem"`, `"300ms"`.
/// We find the first alphabetic character and split there.
///
/// Returns `(numeric_str, unit_str)`.
fn split_dimension(s: &str) -> (&str, &str) {
    // Find where the numeric part ends and the unit begins.
    // The unit starts at the first alphabetic character.
    // Handle the optional leading minus sign.
    let start = if s.starts_with('-') { 1 } else { 0 };
    let split_pos = s[start..]
        .find(|c: char| c.is_alphabetic())
        .map(|i| i + start)
        .unwrap_or(s.len());

    (&s[..split_pos], &s[split_pos..])
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Format tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_number_formats() {
        assert_eq!(LatticeValue::Number(42.0).to_css_string(), "42");
        assert_eq!(LatticeValue::Number(3.14).to_css_string(), "3.14");
        assert_eq!(LatticeValue::Number(0.0).to_css_string(), "0");
        assert_eq!(LatticeValue::Number(-1.0).to_css_string(), "-1");
    }

    #[test]
    fn test_dimension_formats() {
        assert_eq!(
            LatticeValue::Dimension { value: 16.0, unit: "px".to_string() }.to_css_string(),
            "16px"
        );
        assert_eq!(
            LatticeValue::Dimension { value: 1.5, unit: "rem".to_string() }.to_css_string(),
            "1.5rem"
        );
        assert_eq!(
            LatticeValue::Dimension { value: -2.0, unit: "em".to_string() }.to_css_string(),
            "-2em"
        );
    }

    #[test]
    fn test_percentage_formats() {
        assert_eq!(LatticeValue::Percentage(50.0).to_css_string(), "50%");
        assert_eq!(LatticeValue::Percentage(33.33).to_css_string(), "33.33%");
    }

    #[test]
    fn test_string_formats() {
        assert_eq!(LatticeValue::String("hello".to_string()).to_css_string(), "\"hello\"");
        assert_eq!(LatticeValue::String("".to_string()).to_css_string(), "\"\"");
    }

    #[test]
    fn test_null_formats_empty() {
        assert_eq!(LatticeValue::Null.to_css_string(), "");
    }

    #[test]
    fn test_bool_formats() {
        assert_eq!(LatticeValue::Bool(true).to_css_string(), "true");
        assert_eq!(LatticeValue::Bool(false).to_css_string(), "false");
    }

    #[test]
    fn test_color_formats() {
        assert_eq!(LatticeValue::Color("#4a90d9".to_string()).to_css_string(), "#4a90d9");
        assert_eq!(LatticeValue::Color("#fff".to_string()).to_css_string(), "#fff");
    }

    #[test]
    fn test_list_formats() {
        let list = LatticeValue::List(vec![
            LatticeValue::Ident("red".to_string()),
            LatticeValue::Ident("green".to_string()),
            LatticeValue::Ident("blue".to_string()),
        ]);
        assert_eq!(list.to_css_string(), "red, green, blue");
    }

    // -----------------------------------------------------------------------
    // Truthiness tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_truthiness() {
        // Falsy values
        assert!(!LatticeValue::Bool(false).is_truthy());
        assert!(!LatticeValue::Null.is_truthy());
        assert!(!LatticeValue::Number(0.0).is_truthy());

        // Truthy values
        assert!(LatticeValue::Bool(true).is_truthy());
        assert!(LatticeValue::Number(1.0).is_truthy());
        assert!(LatticeValue::Ident("dark".to_string()).is_truthy());
        assert!(LatticeValue::Color("#fff".to_string()).is_truthy());
        // Empty string and empty list are truthy (unlike JavaScript)
        assert!(LatticeValue::String("".to_string()).is_truthy());
        assert!(LatticeValue::List(vec![]).is_truthy());
    }

    // -----------------------------------------------------------------------
    // token_to_value conversion tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_token_to_value_number() {
        let v = token_to_value("Number", "42");
        assert_eq!(v, LatticeValue::Number(42.0));

        let v2 = token_to_value("NUMBER", "3.14");
        assert_eq!(v2, LatticeValue::Number(3.14));
    }

    #[test]
    fn test_token_to_value_dimension() {
        let v = token_to_value("DIMENSION", "16px");
        assert_eq!(v, LatticeValue::Dimension { value: 16.0, unit: "px".to_string() });

        let v2 = token_to_value("DIMENSION", "-1.5rem");
        assert_eq!(v2, LatticeValue::Dimension { value: -1.5, unit: "rem".to_string() });
    }

    #[test]
    fn test_token_to_value_percentage() {
        let v = token_to_value("PERCENTAGE", "50%");
        assert_eq!(v, LatticeValue::Percentage(50.0));
    }

    #[test]
    fn test_token_to_value_ident_special() {
        let v_true = token_to_value("Ident", "true");
        assert_eq!(v_true, LatticeValue::Bool(true));

        let v_false = token_to_value("Ident", "false");
        assert_eq!(v_false, LatticeValue::Bool(false));

        let v_null = token_to_value("Ident", "null");
        assert_eq!(v_null, LatticeValue::Null);
    }

    #[test]
    fn test_token_to_value_hash() {
        let v = token_to_value("HASH", "#4a90d9");
        assert_eq!(v, LatticeValue::Color("#4a90d9".to_string()));
    }

    #[test]
    fn test_split_dimension() {
        assert_eq!(split_dimension("16px"), ("16", "px"));
        assert_eq!(split_dimension("-1rem"), ("-1", "rem"));
        assert_eq!(split_dimension("1.5em"), ("1.5", "em"));
        assert_eq!(split_dimension("100vh"), ("100", "vh"));
    }

    #[test]
    fn test_type_names() {
        assert_eq!(LatticeValue::Number(1.0).type_name(), "number");
        assert_eq!(LatticeValue::Dimension { value: 1.0, unit: "px".to_string() }.type_name(), "dimension");
        assert_eq!(LatticeValue::Null.type_name(), "null");
        assert_eq!(LatticeValue::Bool(true).type_name(), "bool");
    }
}
