//! # Lattice Transpiler — end-to-end Lattice source to CSS text pipeline.
//!
//! This crate is intentionally thin. It wires together three packages in
//! a simple pipeline:
//!
//! ```text
//! Lattice Source Text
//!         │
//!         ▼
//! ┌─────────────────┐
//! │  Lattice Lexer  │  ← lattice.tokens grammar file
//! └────────┬────────┘
//!          │ Vec<Token>
//!          ▼
//! ┌─────────────────┐
//! │  Lattice Parser │  ← lattice.grammar file
//! └────────┬────────┘
//!          │ GrammarASTNode (mixed CSS + Lattice nodes)
//!          ▼
//! ┌─────────────────────────────────┐
//! │  LatticeTransformer (3 passes)  │
//! │  Pass 1: Symbol Collection      │
//! │  Pass 2: Expansion              │
//! │  Pass 3: Cleanup                │
//! └────────┬────────────────────────┘
//!          │ GrammarASTNode (pure CSS nodes)
//!          ▼
//! ┌─────────────────┐
//! │   CSS Emitter   │
//! └────────┬────────┘
//!          │
//!          ▼
//!     CSS Text
//! ```
//!
//! Each package in the pipeline has its own test suite. This crate provides
//! integration tests that verify the full pipeline from source to output.
//!
//! # Usage
//!
//! ```no_run
//! use coding_adventures_lattice_transpiler::{transpile_lattice, transpile_lattice_minified};
//!
//! // Pretty-printed CSS
//! let css = transpile_lattice("$color: red; h1 { color: $color; }")
//!     .expect("transpile failed");
//!
//! // Minified CSS
//! let mini = transpile_lattice_minified("h1 { color: red; }")
//!     .expect("transpile failed");
//! ```

use coding_adventures_lattice_ast_to_css::errors::LatticeError;
use coding_adventures_lattice_ast_to_css::{transform_lattice_with_options};

// ===========================================================================
// Public API
// ===========================================================================

/// Transpile Lattice source text to CSS with default formatting.
///
/// This is the main entry point for the Lattice transpiler. Pass in a string
/// of Lattice source, get back properly formatted CSS text.
///
/// The output uses 2-space indentation and blank lines between rules.
///
/// # Errors
///
/// Returns `Err(LatticeError)` if the source has Lattice semantic errors:
/// - `UndefinedVariable` — `$var` referenced but never declared
/// - `UndefinedMixin` — `@include` references unknown mixin
/// - `WrongArity` — wrong number of arguments to mixin or function
/// - `CircularReference` — mixin or function calls itself
/// - `TypeError` — arithmetic on incompatible types
/// - `MissingReturn` — `@function` has no `@return`
///
/// Note: syntax errors (from the parser/lexer) currently cause panics.
/// This is consistent with the rest of the codebase and will be improved
/// in a future version.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_lattice_transpiler::transpile_lattice;
///
/// let css = transpile_lattice(r#"
///     $primary: #4a90d9;
///     $spacing: 8px;
///
///     @mixin flex-center() {
///         display: flex;
///         align-items: center;
///         justify-content: center;
///     }
///
///     .card {
///         @include flex-center;
///         padding: $spacing;
///         color: $primary;
///     }
/// "#).expect("transpile failed");
///
/// println!("{css}");
/// ```
pub fn transpile_lattice(source: &str) -> Result<String, LatticeError> {
    transform_lattice_with_options(source, "  ", false)
}

/// Transpile Lattice source text to minified CSS.
///
/// Like [`transpile_lattice`], but produces compact CSS with no unnecessary
/// whitespace — suitable for production deployment where file size matters.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_lattice_transpiler::transpile_lattice_minified;
///
/// let css = transpile_lattice_minified("h1 { color: red; font-size: 16px; }").unwrap();
/// // → "h1{color:red;font-size:16px;}\n"
/// println!("{css}");
/// ```
pub fn transpile_lattice_minified(source: &str) -> Result<String, LatticeError> {
    transform_lattice_with_options(source, "  ", true)
}

/// Transpile with explicit indentation control.
///
/// # Arguments
///
/// - `source`: Lattice source text
/// - `indent`: Indentation string per level (e.g., `"  "` or `"\t"`)
/// - `minified`: If `true`, emit minified CSS
pub fn transpile_lattice_with_indent(
    source: &str,
    indent: &str,
    minified: bool,
) -> Result<String, LatticeError> {
    transform_lattice_with_options(source, indent, minified)
}

// ===========================================================================
// Integration Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Test 1: Simple CSS passthrough
    // -----------------------------------------------------------------------

    /// Plain CSS with no Lattice extensions should be emitted unchanged
    /// (modulo formatting).
    #[test]
    fn test_plain_css_passthrough() {
        let css = transpile_lattice("h1 { color: red; }").unwrap();
        assert!(css.contains("color: red"), "Expected color: red, got: {css}");
        assert!(css.contains("h1"), "Expected h1 selector, got: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 2: Variable substitution end-to-end
    // -----------------------------------------------------------------------

    #[test]
    fn test_variable_substitution() {
        let css = transpile_lattice("$color: blue; p { color: $color; }").unwrap();
        assert!(css.contains("color: blue"), "Expected color: blue, got: {css}");
        assert!(!css.contains("$color"), "Variable ref should be gone: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 3: Mixin end-to-end
    // -----------------------------------------------------------------------

    #[test]
    fn test_mixin_end_to_end() {
        let source = r#"
            @mixin reset() {
                margin: 0;
                padding: 0;
            }
            body { @include reset; }
        "#;
        let css = transpile_lattice(source).unwrap();
        assert!(css.contains("margin: 0"), "Expected margin: 0, got: {css}");
        assert!(css.contains("padding: 0"), "Expected padding: 0, got: {css}");
        assert!(!css.contains("@mixin"), "Mixin def should be gone: {css}");
        assert!(!css.contains("@include"), "Include should be gone: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 4: @if branch selection
    // -----------------------------------------------------------------------

    #[test]
    fn test_conditional_compilation() {
        let source = r#"
            $debug: true;
            @if $debug == true {
                .debug { outline: 1px solid red; }
            }
        "#;
        let css = transpile_lattice(source).unwrap();
        // $debug == true is truthy, so the block should be included
        assert!(css.contains("outline"), "Expected debug styles, got: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 5: @for loop generates multiple rules
    // -----------------------------------------------------------------------

    #[test]
    fn test_for_loop_generates_repeated_output() {
        let source = r#"
            @for $i from 1 through 4 {
                .col { flex: 1; }
            }
        "#;
        let css = transpile_lattice(source).unwrap();
        let count = css.matches("flex: 1").count();
        assert_eq!(count, 4, "Expected 4 iterations, got {count}: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 6: @each loop expands list
    // -----------------------------------------------------------------------

    #[test]
    fn test_each_loop() {
        // Note: `.text-$size` selector interpolation requires a grammar
        // extension (VARIABLE inside selectors) that is not yet implemented.
        // This test uses a plain selector instead, verifying that @each
        // correctly expands a variable into CSS declaration values.
        let source = r#"
            @each $color in red, green, blue {
                .item { background: $color; }
            }
        "#;
        let css = transpile_lattice(source).unwrap();
        let count = css.matches("background:").count();
        assert_eq!(count, 3, "Expected 3 each iterations, got {count}: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 7: Minified output format
    // -----------------------------------------------------------------------

    #[test]
    fn test_minified_output() {
        let css = transpile_lattice_minified("h1 { color: red; font-weight: bold; }").unwrap();
        // Minified should not have "  " (double space) or newlines in rules
        assert!(!css.contains("  color"), "Minified should have no indent: {css}");
        // Should have the declarations
        assert!(css.contains("color:red"), "Expected minified color:red, got: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 8: Error on undefined variable
    // -----------------------------------------------------------------------

    #[test]
    fn test_error_undefined_variable() {
        let result = transpile_lattice("p { color: $missing; }");
        assert!(result.is_err(), "Expected error for undefined variable");
        match result {
            Err(LatticeError::UndefinedVariable { name, .. }) => {
                assert_eq!(name, "$missing");
            }
            other => panic!("Expected UndefinedVariable, got: {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Test 9: Error on undefined mixin
    // -----------------------------------------------------------------------

    #[test]
    fn test_error_undefined_mixin() {
        let result = transpile_lattice(".card { @include ghost; }");
        assert!(result.is_err(), "Expected error for undefined mixin");
        match result {
            Err(LatticeError::UndefinedMixin { name, .. }) => {
                assert_eq!(name, "ghost");
            }
            other => panic!("Expected UndefinedMixin, got: {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Test 10: CSS @media rule
    // -----------------------------------------------------------------------

    #[test]
    fn test_media_query_passthrough() {
        let source = "@media screen and (max-width: 768px) { .nav { display: none; } }";
        let css = transpile_lattice(source).unwrap();
        assert!(css.contains("@media"), "Expected @media, got: {css}");
        assert!(css.contains("display: none"), "Expected display: none, got: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 11: CSS @import
    // -----------------------------------------------------------------------

    #[test]
    fn test_import_passthrough() {
        let source = r#"@import url("reset.css");"#;
        let css = transpile_lattice(source).unwrap();
        assert!(css.contains("@import"), "Expected @import, got: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 12: Variable with color value
    // -----------------------------------------------------------------------

    #[test]
    fn test_variable_with_hex_color() {
        let css = transpile_lattice("$brand: #4a90d9; a { color: $brand; }").unwrap();
        assert!(css.contains("#4a90d9"), "Expected hex color, got: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 13: Complex selector
    // -----------------------------------------------------------------------

    #[test]
    fn test_complex_selector() {
        let css = transpile_lattice(".parent > .child { margin: 0; }").unwrap();
        assert!(css.contains("margin: 0"), "Expected margin: 0, got: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 14: Multiple rules in stylesheet
    // -----------------------------------------------------------------------

    #[test]
    fn test_multiple_rules() {
        let source = "h1 { color: red; } p { color: blue; }";
        let css = transpile_lattice(source).unwrap();
        assert!(css.contains("color: red"), "Expected h1 rule: {css}");
        assert!(css.contains("color: blue"), "Expected p rule: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 15: CSS function call passthrough
    // -----------------------------------------------------------------------

    #[test]
    fn test_css_function_passthrough() {
        let source = "div { background: linear-gradient(to right, red, blue); }";
        let css = transpile_lattice(source).unwrap();
        assert!(css.contains("linear-gradient"), "Expected gradient, got: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 16: Variable with dimension value
    // -----------------------------------------------------------------------

    #[test]
    fn test_variable_dimension() {
        let css = transpile_lattice("$size: 16px; p { font-size: $size; }").unwrap();
        assert!(css.contains("font-size: 16px"), "Expected font-size: 16px, got: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 17: Empty stylesheet
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_stylesheet() {
        let css = transpile_lattice("").unwrap();
        assert_eq!(css, "", "Empty source should produce empty CSS");
    }

    // -----------------------------------------------------------------------
    // Test 18: Variable-only stylesheet (no CSS output)
    // -----------------------------------------------------------------------

    #[test]
    fn test_variables_only_no_output() {
        // Variable declarations produce no CSS output
        let css = transpile_lattice("$a: red; $b: blue;").unwrap();
        assert_eq!(css, "", "Variables without rules should produce no CSS: '{css}'");
    }

    // -----------------------------------------------------------------------
    // Test 19: Mixin defined after use (forward reference)
    // -----------------------------------------------------------------------

    #[test]
    fn test_mixin_forward_reference() {
        let source = r#"
            .card { @include shadow; }
            @mixin shadow() { box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        "#;
        let css = transpile_lattice(source).unwrap();
        assert!(css.contains("box-shadow"), "Forward reference mixin should work: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 20: Custom indentation
    // -----------------------------------------------------------------------

    #[test]
    fn test_custom_indentation() {
        let css = transpile_lattice_with_indent("h1 { color: red; }", "\t", false).unwrap();
        assert!(css.contains("\tcolor"), "Expected tab-indented color, got: {css}");
    }
}
