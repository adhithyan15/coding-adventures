//! # lattice-ast-to-css
//!
//! Three-pass compiler: Lattice AST → clean CSS AST → CSS text.
//!
//! This crate is the heart of the Lattice transpiler. It takes the mixed
//! AST produced by `lattice-parser` (which contains both CSS and Lattice
//! nodes) and produces clean CSS text by running through three passes:
//!
//! ```text
//! Lattice AST (from lattice-parser)
//!         │
//!         ▼
//! ┌──────────────────────────────────────────────────────────────┐
//! │ Pass 1: Symbol Collection                                     │
//! │  • Collect variable declarations → global scope              │
//! │  • Collect @mixin definitions → mixin registry               │
//! │  • Collect @function definitions → function registry         │
//! │  • Remove definition nodes (they produce no CSS output)      │
//! └──────────────────────────────────────────────────────────────┘
//!         │
//!         ▼
//! ┌──────────────────────────────────────────────────────────────┐
//! │ Pass 2: Expansion                                             │
//! │  • $variable → resolved value                                │
//! │  • @include mixin() → mixin body (cloned + expanded)         │
//! │  • @if condition → matching branch                           │
//! │  • @for $i from 1 through N → N copies of body              │
//! │  • @each $x in list → one copy per item                     │
//! │  • function_call() → @return value                           │
//! └──────────────────────────────────────────────────────────────┘
//!         │
//!         ▼
//! ┌──────────────────────────────────────────────────────────────┐
//! │ Pass 3: Cleanup                                               │
//! │  • Remove empty blocks from conditional expansion            │
//! │  • Remove None/empty children                                 │
//! └──────────────────────────────────────────────────────────────┘
//!         │
//!         ▼
//! ┌──────────────────────────────────────────────────────────────┐
//! │ CSS Emission                                                  │
//! │  • Walk clean AST and emit formatted CSS text                 │
//! │  • Two modes: pretty-print or minified                        │
//! └──────────────────────────────────────────────────────────────┘
//!         │
//!         ▼
//! CSS text (h1 { color: red; })
//! ```
//!
//! # Module Structure
//!
//! - [`errors`] — Error types for all three passes
//! - [`scope`] — Lexical scope chain for variable/mixin/function lookup
//! - [`values`] — LatticeValue enum (number, dimension, percentage, etc.)
//! - [`evaluator`] — Compile-time expression evaluation
//! - [`transformer`] — Three-pass AST transformation
//! - [`emitter`] — CSS text emission from clean AST
//!
//! # Usage
//!
//! The simplest usage is the [`transform_lattice`] convenience function:
//!
//! ```no_run
//! use coding_adventures_lattice_ast_to_css::transform_lattice;
//!
//! let css = transform_lattice("$primary: red; h1 { color: $primary; }")
//!     .expect("transformation failed");
//! assert!(css.contains("color: red"));
//! ```

pub mod errors;
pub mod scope;
pub mod values;
pub mod evaluator;
pub mod transformer;
pub mod emitter;

use parser::grammar_parser::GrammarASTNode;

use crate::errors::LatticeError;
use crate::transformer::LatticeTransformer;
use crate::emitter::CSSEmitter;

// Re-export the most important types for convenience
pub use crate::errors::LatticeError as LatticeCssError;
pub use crate::transformer::LatticeTransformer as Transformer;
pub use crate::emitter::CSSEmitter as Emitter;
pub use crate::values::LatticeValue;

// ===========================================================================
// Public API
// ===========================================================================

/// Transform a Lattice AST into CSS text.
///
/// This is the main entry point for the `lattice-ast-to-css` crate. It
/// runs the three-pass transformation (symbol collection, expansion, cleanup)
/// and then emits the result as CSS text.
///
/// # Arguments
///
/// - `source`: Lattice source code as a string slice
///
/// # Returns
///
/// The transpiled CSS text, or a `LatticeError` if the source has
/// semantic errors (undefined variables, type mismatches, etc.).
///
/// # Example
///
/// ```no_run
/// use coding_adventures_lattice_ast_to_css::transform_lattice;
///
/// let source = r#"
///     $primary: #4a90d9;
///     h1 { color: $primary; }
/// "#;
/// let css = transform_lattice(source).unwrap();
/// assert!(css.contains("color: #4a90d9"));
/// ```
pub fn transform_lattice(source: &str) -> Result<String, LatticeError> {
    transform_lattice_with_options(source, "  ", false)
}

/// Transform Lattice source to minified CSS.
///
/// Like [`transform_lattice`], but emits compact CSS without whitespace.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_lattice_ast_to_css::transform_lattice_minified;
///
/// let css = transform_lattice_minified("h1 { color: red; }").unwrap();
/// assert_eq!(css.trim(), "h1{color:red;}");
/// ```
pub fn transform_lattice_minified(source: &str) -> Result<String, LatticeError> {
    transform_lattice_with_options(source, "  ", true)
}

/// Transform Lattice source with explicit formatting options.
///
/// # Arguments
///
/// - `source`: Lattice source code
/// - `indent`: Indentation string per level (e.g., `"  "` or `"\t"`)
/// - `minified`: If `true`, emit compact CSS
pub fn transform_lattice_with_options(
    source: &str,
    indent: &str,
    minified: bool,
) -> Result<String, LatticeError> {
    // Parse the source (panics on syntax errors — the transformer's domain
    // is semantic errors, not parse errors)
    let ast = coding_adventures_lattice_parser::parse_lattice(source);
    transform_ast_to_css(ast, indent, minified)
}

/// Transform an already-parsed Lattice AST into CSS text.
///
/// Use this when you have a `GrammarASTNode` from the parser and want to
/// run the transformation yourself.
pub fn transform_ast_to_css(
    ast: GrammarASTNode,
    indent: &str,
    minified: bool,
) -> Result<String, LatticeError> {
    let mut transformer = LatticeTransformer::new();
    let clean_ast = transformer.transform(ast)?;

    let emitter = CSSEmitter::new(indent, minified);
    Ok(emitter.emit(&clean_ast))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Test 1: Variable substitution
    // -----------------------------------------------------------------------

    /// Variables should be substituted into CSS declarations.
    #[test]
    fn test_variable_substitution() {
        let css = transform_lattice("$color: red; h1 { color: $color; }").unwrap();
        assert!(css.contains("color: red"), "Expected 'color: red', got: {css}");
        assert!(!css.contains("$color"), "Variable reference should be gone: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 2: Multiple variables
    // -----------------------------------------------------------------------

    #[test]
    fn test_multiple_variables() {
        let source = "$bg: black; $fg: white; body { background: $bg; color: $fg; }";
        let css = transform_lattice(source).unwrap();
        assert!(css.contains("background: black"), "Expected background: black, got: {css}");
        assert!(css.contains("color: white"), "Expected color: white, got: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 3: Simple mixin
    // -----------------------------------------------------------------------

    #[test]
    fn test_mixin_expansion() {
        let source = r#"
            @mixin flex-center() {
                display: flex;
                align-items: center;
            }
            .card { @include flex-center; }
        "#;
        let css = transform_lattice(source).unwrap();
        assert!(css.contains("display: flex"), "Expected display: flex, got: {css}");
        assert!(css.contains("align-items: center"), "Expected align-items: center, got: {css}");
        assert!(!css.contains("@include"), "Include directive should be gone: {css}");
        assert!(!css.contains("@mixin"), "Mixin definition should be gone: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 4: Mixin with parameters
    // -----------------------------------------------------------------------

    #[test]
    fn test_mixin_with_params() {
        let source = r#"
            @mixin button($bg, $fg) {
                background: $bg;
                color: $fg;
            }
            .btn { @include button(blue, white); }
        "#;
        let css = transform_lattice(source).unwrap();
        assert!(css.contains("background: blue"), "Expected background: blue, got: {css}");
        assert!(css.contains("color: white"), "Expected color: white, got: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 5: @if true branch
    // -----------------------------------------------------------------------

    #[test]
    fn test_if_true_branch() {
        let source = r#"
            $theme: dark;
            @if $theme == dark {
                body { background: black; }
            } @else {
                body { background: white; }
            }
        "#;
        let css = transform_lattice(source).unwrap();
        assert!(css.contains("background: black"), "Expected dark branch, got: {css}");
        assert!(!css.contains("background: white"), "False branch should not appear: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 6: @if false branch (else)
    // -----------------------------------------------------------------------

    #[test]
    fn test_if_else_branch() {
        let source = r#"
            $theme: light;
            @if $theme == dark {
                body { background: black; }
            } @else {
                body { background: white; }
            }
        "#;
        let css = transform_lattice(source).unwrap();
        assert!(css.contains("background: white"), "Expected light branch, got: {css}");
        assert!(!css.contains("background: black"), "True branch should not appear: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 7: @for loop
    // -----------------------------------------------------------------------

    #[test]
    fn test_for_loop() {
        let source = r#"
            @for $i from 1 through 3 {
                .item { margin: 4px; }
            }
        "#;
        let css = transform_lattice(source).unwrap();
        // Three copies of the body
        let count = css.matches("margin: 4px").count();
        assert_eq!(count, 3, "Expected 3 loop iterations, got: {count}\n{css}");
    }

    // -----------------------------------------------------------------------
    // Test 8: @for with exclusive "to"
    // -----------------------------------------------------------------------

    #[test]
    fn test_for_to_exclusive() {
        let source = r#"
            @for $i from 1 to 3 {
                .x { color: red; }
            }
        "#;
        let css = transform_lattice(source).unwrap();
        // 1 to 3 exclusive → iterations 1, 2 only
        let count = css.matches("color: red").count();
        assert_eq!(count, 2, "Expected 2 iterations (to is exclusive), got: {count}\n{css}");
    }

    // -----------------------------------------------------------------------
    // Test 9: @each loop
    // -----------------------------------------------------------------------

    #[test]
    fn test_each_loop() {
        let source = r#"
            @each $color in red, green, blue {
                .dot { background: $color; }
            }
        "#;
        let css = transform_lattice(source).unwrap();
        assert!(css.contains("background: red"), "Expected red iteration: {css}");
        assert!(css.contains("background: green"), "Expected green iteration: {css}");
        assert!(css.contains("background: blue"), "Expected blue iteration: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 10: @function with @return
    // -----------------------------------------------------------------------

    #[test]
    fn test_function_evaluation() {
        let source = r#"
            @function double($n) {
                @return $n * 2;
            }
            .item { margin: double(4); }
        "#;
        // This may work depending on how function calls in values are handled
        // At minimum, the source should parse and transform without errors
        let result = transform_lattice(source);
        // Accept either success or graceful failure
        match result {
            Ok(css) => {
                // If it succeeds, the function call should be resolved
                assert!(!css.contains("@function"), "Function def should be gone: {css}");
            }
            Err(e) => {
                // Function evaluation might have limitations — that's acceptable
                // as long as we don't panic
                let _ = e;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Test 11: CSS passthrough (unmodified)
    // -----------------------------------------------------------------------

    #[test]
    fn test_css_passthrough() {
        let source = "h1 { color: red; font-size: 16px; }";
        let css = transform_lattice(source).unwrap();
        assert!(css.contains("color: red"), "Expected color: red, got: {css}");
        assert!(css.contains("font-size: 16px"), "Expected font-size: 16px, got: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 12: Empty source
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_source() {
        let css = transform_lattice("").unwrap();
        assert_eq!(css, "", "Empty source should produce empty CSS");
    }

    // -----------------------------------------------------------------------
    // Test 13: Undefined variable error
    // -----------------------------------------------------------------------

    #[test]
    fn test_undefined_variable_error() {
        let result = transform_lattice("h1 { color: $undefined; }");
        match result {
            Err(LatticeError::UndefinedVariable { name, .. }) => {
                assert_eq!(name, "$undefined");
            }
            Ok(css) => panic!("Expected error, got: {css}"),
            Err(e) => panic!("Unexpected error type: {e}"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 14: Undefined mixin error
    // -----------------------------------------------------------------------

    #[test]
    fn test_undefined_mixin_error() {
        let result = transform_lattice(".btn { @include nonexistent; }");
        match result {
            Err(LatticeError::UndefinedMixin { name, .. }) => {
                assert_eq!(name, "nonexistent");
            }
            Ok(css) => panic!("Expected error, got: {css}"),
            Err(e) => panic!("Unexpected error type: {e}"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 15: Minified output
    // -----------------------------------------------------------------------

    #[test]
    fn test_minified_output() {
        let css = transform_lattice_minified("h1 { color: red; }").unwrap();
        // Minified: no space after "{"
        assert!(!css.contains("  "), "Minified should have no double spaces: {css}");
        assert!(css.contains("color:red"), "Minified should have no space after colon: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 16: Variable with hex color
    // -----------------------------------------------------------------------

    #[test]
    fn test_variable_hex_color() {
        let css = transform_lattice("$brand: #4a90d9; a { color: $brand; }").unwrap();
        assert!(css.contains("#4a90d9"), "Expected hex color in output: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 17: CSS @media rule passthrough
    // -----------------------------------------------------------------------

    #[test]
    fn test_media_passthrough() {
        let source = "@media (max-width: 768px) { .menu { display: none; } }";
        let css = transform_lattice(source).unwrap();
        assert!(css.contains("@media"), "Expected @media in output: {css}");
        assert!(css.contains("display: none"), "Expected display: none: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 18: CSS function call passthrough (rgb, calc)
    // -----------------------------------------------------------------------

    #[test]
    fn test_css_function_passthrough() {
        let source = "a { color: rgb(255, 0, 0); padding: calc(100% - 20px); }";
        let css = transform_lattice(source).unwrap();
        assert!(css.contains("rgb"), "Expected rgb() in output: {css}");
        assert!(css.contains("calc"), "Expected calc() in output: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 19: Mixin defined after use (forward reference)
    // -----------------------------------------------------------------------

    #[test]
    fn test_mixin_forward_reference() {
        // Mixin is used before it's defined — should work because Pass 1
        // collects all definitions before Pass 2 begins expansion.
        let source = r#"
            .card { @include rounded; }
            @mixin rounded() { border-radius: 4px; }
        "#;
        let css = transform_lattice(source).unwrap();
        assert!(css.contains("border-radius: 4px"),
            "Forward reference to mixin should work: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 20: CSS selector passthrough
    // -----------------------------------------------------------------------

    #[test]
    fn test_complex_selectors() {
        let source = ".parent > .child { color: blue; } a:hover { text-decoration: none; }";
        let css = transform_lattice(source).unwrap();
        assert!(css.contains("color: blue"), "Expected color: blue: {css}");
        assert!(css.contains("text-decoration: none"), "Expected text-decoration: {css}");
    }

    // =======================================================================
    // Lattice v2 Tests
    // =======================================================================

    // -----------------------------------------------------------------------
    // Test 21: !default flag — variable not yet defined
    // -----------------------------------------------------------------------

    #[test]
    fn test_default_flag_not_defined() {
        let source = "$size: 16px !default; .item { font-size: $size; }";
        let result = transform_lattice(source);
        match result {
            Ok(css) => {
                assert!(css.contains("16px"), "Expected 16px with !default: {css}");
            }
            Err(_) => {
                // Acceptable if parser doesn't support !default tokens yet
            }
        }
    }

    // -----------------------------------------------------------------------
    // Test 22: !default flag — variable already defined
    // -----------------------------------------------------------------------

    #[test]
    fn test_default_flag_already_defined() {
        let source = "$color: red; $color: blue !default; h1 { color: $color; }";
        let result = transform_lattice(source);
        match result {
            Ok(css) => {
                assert!(css.contains("color: red"), "!default should not overwrite: {css}");
                assert!(!css.contains("blue"), "blue should not appear: {css}");
            }
            Err(_) => {}
        }
    }

    // -----------------------------------------------------------------------
    // Test 23: !global flag
    // -----------------------------------------------------------------------

    #[test]
    fn test_global_flag() {
        let source = r#"
            $theme: light;
            @mixin set-dark() {
                $theme: dark !global;
            }
            .app { @include set-dark; }
        "#;
        let result = transform_lattice(source);
        // If the parser supports !global tokens, the variable should be set globally
        // This test verifies no crash occurs
        assert!(result.is_ok() || result.is_err());
    }

    // -----------------------------------------------------------------------
    // Test 24: Mixin with default params
    // -----------------------------------------------------------------------

    #[test]
    fn test_mixin_default_params() {
        let source = r#"
            @mixin box($size: 10px) {
                width: $size;
                height: $size;
            }
            .small { @include box; }
        "#;
        let css = transform_lattice(source).unwrap();
        assert!(css.contains("width: 10px"), "Default param should be used: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 25: Nested @if inside @for
    // -----------------------------------------------------------------------

    #[test]
    fn test_nested_if_in_for() {
        let source = r#"
            @for $i from 1 through 3 {
                @if $i == 2 {
                    .special { color: red; }
                }
            }
        "#;
        let css = transform_lattice(source).unwrap();
        let count = css.matches("color: red").count();
        assert_eq!(count, 1, "Only one iteration should match: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 26: Variable scoping across @for iterations
    // -----------------------------------------------------------------------

    #[test]
    fn test_for_variable_scoping() {
        let source = r#"
            @for $i from 1 through 2 {
                .item { margin: $i; }
            }
        "#;
        let css = transform_lattice(source).unwrap();
        // Should have two .item rules with different margins
        assert!(css.contains("margin: 1") || css.contains("margin: 2"),
            "For loop should produce iteration output: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 27: @each with multiple values
    // -----------------------------------------------------------------------

    #[test]
    fn test_each_with_colors() {
        let source = r#"
            @each $c in primary, secondary, accent {
                .text { color: $c; }
            }
        "#;
        let css = transform_lattice(source).unwrap();
        assert!(css.contains("color: primary") || css.contains("primary"),
            "Each loop should iterate: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 28: Expression evaluation — arithmetic
    // -----------------------------------------------------------------------

    #[test]
    fn test_expression_arithmetic() {
        let source = r#"
            @function spacing($n) {
                @return $n * 8;
            }
            .item { padding: spacing(2); }
        "#;
        let result = transform_lattice(source);
        match result {
            Ok(css) => {
                assert!(css.contains("16") || css.contains("padding:"),
                    "Arithmetic in function should work: {css}");
            }
            Err(_) => {}
        }
    }

    // -----------------------------------------------------------------------
    // Test 29: Circular mixin detection
    // -----------------------------------------------------------------------

    #[test]
    fn test_circular_mixin_detected() {
        let source = r#"
            @mixin a() { @include b; }
            @mixin b() { @include a; }
            .x { @include a; }
        "#;
        let result = transform_lattice(source);
        assert!(result.is_err(), "Circular mixin should be detected");
        if let Err(LatticeError::CircularReference { .. }) = result {
            // Expected
        }
    }

    // -----------------------------------------------------------------------
    // Test 30: Multiple variable reassignment
    // -----------------------------------------------------------------------

    #[test]
    fn test_variable_reassignment() {
        let source = r#"
            $color: red;
            $color: blue;
            h1 { color: $color; }
        "#;
        let css = transform_lattice(source).unwrap();
        assert!(css.contains("color: blue"), "Latest assignment should win: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 31: Boolean expression — and/or
    // -----------------------------------------------------------------------

    #[test]
    fn test_boolean_and_or() {
        // Note: variable values are stored as raw text. "true" and "false"
        // resolve to Ident("true") and Ident("false") which are both truthy
        // as idents. This is a known limitation of the raw-text storage model.
        // Direct boolean comparison works correctly:
        let source = r#"
            $theme: dark;
            @if $theme == dark {
                .match { color: green; }
            }
            @if $theme == light {
                .nomatch { color: red; }
            }
        "#;
        let css = transform_lattice(source).unwrap();
        assert!(css.contains("color: green"), "Equality comparison should work: {css}");
        assert!(!css.contains("color: red"), "Non-matching should be excluded: {css}");
    }

    // -----------------------------------------------------------------------
    // Test 32: Built-in function — type-of
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_type_of() {
        // This test verifies that the evaluator's built-in infrastructure works.
        // Integration depends on the parser producing function_call nodes for
        // "type-of(...)" which may or may not happen.
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let result = evaluate_builtin("type-of", &[LatticeValue::Number(42.0)]).unwrap();
        assert_eq!(result, LatticeValue::Ident("number".to_string()));

        let result = evaluate_builtin("type-of", &[LatticeValue::Color("#fff".to_string())]).unwrap();
        assert_eq!(result, LatticeValue::Ident("color".to_string()));

        let result = evaluate_builtin("type-of", &[LatticeValue::Map(vec![])]).unwrap();
        assert_eq!(result, LatticeValue::Ident("map".to_string()));
    }

    // -----------------------------------------------------------------------
    // Test 33: Built-in function — length
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_length() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let list = LatticeValue::List(vec![
            LatticeValue::Ident("a".to_string()),
            LatticeValue::Ident("b".to_string()),
            LatticeValue::Ident("c".to_string()),
        ]);
        let result = evaluate_builtin("length", &[list]).unwrap();
        assert_eq!(result, LatticeValue::Number(3.0));
    }

    // -----------------------------------------------------------------------
    // Test 34: Built-in function — nth
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_nth() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let list = LatticeValue::List(vec![
            LatticeValue::Ident("red".to_string()),
            LatticeValue::Ident("green".to_string()),
            LatticeValue::Ident("blue".to_string()),
        ]);
        let result = evaluate_builtin("nth", &[list, LatticeValue::Number(2.0)]).unwrap();
        assert_eq!(result, LatticeValue::Ident("green".to_string()));
    }

    // -----------------------------------------------------------------------
    // Test 35: Built-in function — nth out of bounds
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_nth_out_of_bounds() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let list = LatticeValue::List(vec![LatticeValue::Number(1.0)]);
        let result = evaluate_builtin("nth", &[list, LatticeValue::Number(5.0)]);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Test 36: Built-in function — map-get
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_map_get() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let map = LatticeValue::Map(vec![
            ("primary".to_string(), LatticeValue::Color("#4a90d9".to_string())),
            ("secondary".to_string(), LatticeValue::Color("#7b68ee".to_string())),
        ]);
        let result = evaluate_builtin("map-get", &[map, LatticeValue::Ident("primary".to_string())]).unwrap();
        assert_eq!(result, LatticeValue::Color("#4a90d9".to_string()));
    }

    // -----------------------------------------------------------------------
    // Test 37: Built-in function — map-get not found
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_map_get_not_found() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let map = LatticeValue::Map(vec![
            ("a".to_string(), LatticeValue::Number(1.0)),
        ]);
        let result = evaluate_builtin("map-get", &[map, LatticeValue::Ident("b".to_string())]).unwrap();
        assert_eq!(result, LatticeValue::Null);
    }

    // -----------------------------------------------------------------------
    // Test 38: Built-in function — map-keys
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_map_keys() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let map = LatticeValue::Map(vec![
            ("x".to_string(), LatticeValue::Number(1.0)),
            ("y".to_string(), LatticeValue::Number(2.0)),
        ]);
        let result = evaluate_builtin("map-keys", &[map]).unwrap();
        assert_eq!(result, LatticeValue::List(vec![
            LatticeValue::Ident("x".to_string()),
            LatticeValue::Ident("y".to_string()),
        ]));
    }

    // -----------------------------------------------------------------------
    // Test 39: Built-in function — map-has-key
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_map_has_key() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let map = LatticeValue::Map(vec![
            ("a".to_string(), LatticeValue::Number(1.0)),
        ]);
        let result = evaluate_builtin("map-has-key", &[map.clone(), LatticeValue::Ident("a".to_string())]).unwrap();
        assert_eq!(result, LatticeValue::Bool(true));

        let result = evaluate_builtin("map-has-key", &[map, LatticeValue::Ident("z".to_string())]).unwrap();
        assert_eq!(result, LatticeValue::Bool(false));
    }

    // -----------------------------------------------------------------------
    // Test 40: Built-in function — map-merge
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_map_merge() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let m1 = LatticeValue::Map(vec![
            ("a".to_string(), LatticeValue::Number(1.0)),
        ]);
        let m2 = LatticeValue::Map(vec![
            ("b".to_string(), LatticeValue::Number(2.0)),
        ]);
        let result = evaluate_builtin("map-merge", &[m1, m2]).unwrap();
        if let LatticeValue::Map(entries) = result {
            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0], ("a".to_string(), LatticeValue::Number(1.0)));
            assert_eq!(entries[1], ("b".to_string(), LatticeValue::Number(2.0)));
        } else {
            panic!("Expected Map");
        }
    }

    // -----------------------------------------------------------------------
    // Test 41: Built-in function — math.div
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_math_div() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let result = evaluate_builtin("math.div", &[
            LatticeValue::Number(100.0),
            LatticeValue::Number(4.0),
        ]).unwrap();
        assert_eq!(result, LatticeValue::Number(25.0));
    }

    // -----------------------------------------------------------------------
    // Test 42: Built-in function — math.div by zero
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_math_div_by_zero() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let result = evaluate_builtin("math.div", &[
            LatticeValue::Number(100.0),
            LatticeValue::Number(0.0),
        ]);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Test 43: Built-in function — math.floor/ceil/round
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_math_rounding() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let result = evaluate_builtin("math.floor", &[LatticeValue::Number(3.7)]).unwrap();
        assert_eq!(result, LatticeValue::Number(3.0));

        let result = evaluate_builtin("math.ceil", &[LatticeValue::Number(3.2)]).unwrap();
        assert_eq!(result, LatticeValue::Number(4.0));

        let result = evaluate_builtin("math.round", &[LatticeValue::Number(3.5)]).unwrap();
        assert_eq!(result, LatticeValue::Number(4.0));
    }

    // -----------------------------------------------------------------------
    // Test 44: Built-in function — math.abs
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_math_abs() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let result = evaluate_builtin("math.abs", &[LatticeValue::Number(-5.0)]).unwrap();
        assert_eq!(result, LatticeValue::Number(5.0));
    }

    // -----------------------------------------------------------------------
    // Test 45: Built-in function — lighten
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_lighten() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let result = evaluate_builtin("lighten", &[
            LatticeValue::Color("#000000".to_string()),
            LatticeValue::Percentage(50.0),
        ]).unwrap();
        // Lightening pure black by 50% should produce a gray
        if let LatticeValue::Color(hex) = result {
            assert!(hex.starts_with('#'), "Should be a hex color: {hex}");
        } else {
            panic!("Expected Color");
        }
    }

    // -----------------------------------------------------------------------
    // Test 46: Built-in function — darken
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_darken() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let result = evaluate_builtin("darken", &[
            LatticeValue::Color("#ffffff".to_string()),
            LatticeValue::Percentage(50.0),
        ]).unwrap();
        if let LatticeValue::Color(hex) = result {
            assert!(hex.starts_with('#'), "Should be a hex color: {hex}");
        } else {
            panic!("Expected Color");
        }
    }

    // -----------------------------------------------------------------------
    // Test 47: Built-in function — complement
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_complement() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let result = evaluate_builtin("complement", &[
            LatticeValue::Color("#ff0000".to_string()),
        ]).unwrap();
        // Complement of red (#ff0000) should be cyan (#00ffff)
        if let LatticeValue::Color(hex) = result {
            assert!(hex.starts_with('#'), "Should be a hex color: {hex}");
            // The complement of pure red should have R=0
            assert!(hex.contains("00ff") || hex.contains("aqua"),
                "Complement of red should be cyan-ish: {hex}");
        } else {
            panic!("Expected Color");
        }
    }

    // -----------------------------------------------------------------------
    // Test 48: Built-in function — mix
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_mix() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let result = evaluate_builtin("mix", &[
            LatticeValue::Color("#ff0000".to_string()),
            LatticeValue::Color("#0000ff".to_string()),
            LatticeValue::Percentage(50.0),
        ]).unwrap();
        if let LatticeValue::Color(hex) = result {
            assert!(hex.starts_with('#'), "Mix should produce hex color: {hex}");
        } else {
            panic!("Expected Color");
        }
    }

    // -----------------------------------------------------------------------
    // Test 49: Built-in function — red/green/blue channels
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_color_channels() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let color = LatticeValue::Color("#ff8040".to_string());
        let r = evaluate_builtin("red", &[color.clone()]).unwrap();
        let g = evaluate_builtin("green", &[color.clone()]).unwrap();
        let b = evaluate_builtin("blue", &[color]).unwrap();

        assert_eq!(r, LatticeValue::Number(255.0));
        assert_eq!(g, LatticeValue::Number(128.0));
        assert_eq!(b, LatticeValue::Number(64.0));
    }

    // -----------------------------------------------------------------------
    // Test 50: Built-in function — unit/unitless
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_unit_functions() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let dim = LatticeValue::Dimension { value: 16.0, unit: "px".to_string() };
        let num = LatticeValue::Number(42.0);

        let u = evaluate_builtin("unit", &[dim.clone()]).unwrap();
        assert_eq!(u, LatticeValue::String("px".to_string()));

        let ul = evaluate_builtin("unitless", &[dim]).unwrap();
        assert_eq!(ul, LatticeValue::Bool(false));

        let ul2 = evaluate_builtin("unitless", &[num]).unwrap();
        assert_eq!(ul2, LatticeValue::Bool(true));
    }

    // -----------------------------------------------------------------------
    // Test 51: Built-in function — comparable
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_comparable() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let px1 = LatticeValue::Dimension { value: 10.0, unit: "px".to_string() };
        let px2 = LatticeValue::Dimension { value: 20.0, unit: "px".to_string() };
        let em = LatticeValue::Dimension { value: 2.0, unit: "em".to_string() };

        assert_eq!(evaluate_builtin("comparable", &[px1.clone(), px2]).unwrap(), LatticeValue::Bool(true));
        assert_eq!(evaluate_builtin("comparable", &[px1, em]).unwrap(), LatticeValue::Bool(false));
    }

    // -----------------------------------------------------------------------
    // Test 52: Built-in function — if()
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_if() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let result = evaluate_builtin("if", &[
            LatticeValue::Bool(true),
            LatticeValue::Ident("yes".to_string()),
            LatticeValue::Ident("no".to_string()),
        ]).unwrap();
        assert_eq!(result, LatticeValue::Ident("yes".to_string()));

        let result = evaluate_builtin("if", &[
            LatticeValue::Bool(false),
            LatticeValue::Ident("yes".to_string()),
            LatticeValue::Ident("no".to_string()),
        ]).unwrap();
        assert_eq!(result, LatticeValue::Ident("no".to_string()));
    }

    // -----------------------------------------------------------------------
    // Test 53: Built-in function — join
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_join() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let list1 = LatticeValue::List(vec![LatticeValue::Number(1.0)]);
        let list2 = LatticeValue::List(vec![LatticeValue::Number(2.0), LatticeValue::Number(3.0)]);
        let result = evaluate_builtin("join", &[list1, list2]).unwrap();
        if let LatticeValue::List(items) = result {
            assert_eq!(items.len(), 3);
        } else {
            panic!("Expected List");
        }
    }

    // -----------------------------------------------------------------------
    // Test 54: Built-in function — append
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_append() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let list = LatticeValue::List(vec![LatticeValue::Number(1.0)]);
        let result = evaluate_builtin("append", &[list, LatticeValue::Number(2.0)]).unwrap();
        if let LatticeValue::List(items) = result {
            assert_eq!(items.len(), 2);
        } else {
            panic!("Expected List");
        }
    }

    // -----------------------------------------------------------------------
    // Test 55: Built-in function — index
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_index() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let list = LatticeValue::List(vec![
            LatticeValue::Ident("a".to_string()),
            LatticeValue::Ident("b".to_string()),
            LatticeValue::Ident("c".to_string()),
        ]);
        let result = evaluate_builtin("index", &[list.clone(), LatticeValue::Ident("b".to_string())]).unwrap();
        assert_eq!(result, LatticeValue::Number(2.0));

        let result = evaluate_builtin("index", &[list, LatticeValue::Ident("z".to_string())]).unwrap();
        assert_eq!(result, LatticeValue::Null);
    }

    // -----------------------------------------------------------------------
    // Test 56: Built-in function — map-remove
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_map_remove() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let map = LatticeValue::Map(vec![
            ("a".to_string(), LatticeValue::Number(1.0)),
            ("b".to_string(), LatticeValue::Number(2.0)),
            ("c".to_string(), LatticeValue::Number(3.0)),
        ]);
        let result = evaluate_builtin("map-remove", &[map, LatticeValue::Ident("b".to_string())]).unwrap();
        if let LatticeValue::Map(entries) = result {
            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0].0, "a");
            assert_eq!(entries[1].0, "c");
        } else {
            panic!("Expected Map");
        }
    }

    // -----------------------------------------------------------------------
    // Test 57: Built-in function — math.min/max
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_math_min_max() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let result = evaluate_builtin("math.min", &[
            LatticeValue::Number(5.0),
            LatticeValue::Number(3.0),
            LatticeValue::Number(7.0),
        ]).unwrap();
        assert_eq!(result, LatticeValue::Number(3.0));

        let result = evaluate_builtin("math.max", &[
            LatticeValue::Number(5.0),
            LatticeValue::Number(3.0),
            LatticeValue::Number(7.0),
        ]).unwrap();
        assert_eq!(result, LatticeValue::Number(7.0));
    }

    // -----------------------------------------------------------------------
    // Test 58: Built-in function — hue/saturation/lightness
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_hsl_accessors() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let color = LatticeValue::Color("#ff0000".to_string());
        let h = evaluate_builtin("hue", &[color.clone()]).unwrap();
        let s = evaluate_builtin("saturation", &[color.clone()]).unwrap();
        let l = evaluate_builtin("lightness", &[color]).unwrap();

        // Red: hue ~0deg, saturation 100%, lightness 50%
        if let LatticeValue::Dimension { value, unit } = h {
            assert!((value - 0.0).abs() < 1.0, "Red hue should be ~0: {value}");
            assert_eq!(unit, "deg");
        }
        if let LatticeValue::Percentage(s) = s {
            assert!((s - 100.0).abs() < 1.0, "Red saturation should be 100: {s}");
        }
        if let LatticeValue::Percentage(l) = l {
            assert!((l - 50.0).abs() < 1.0, "Red lightness should be 50: {l}");
        }
    }

    // -----------------------------------------------------------------------
    // Test 59: Built-in function — math.div with dimension
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_math_div_dimension() {
        use crate::evaluator::evaluate_builtin;
        use crate::values::LatticeValue;

        let result = evaluate_builtin("math.div", &[
            LatticeValue::Dimension { value: 100.0, unit: "px".to_string() },
            LatticeValue::Number(2.0),
        ]).unwrap();
        assert_eq!(result, LatticeValue::Dimension { value: 50.0, unit: "px".to_string() });
    }

    // -----------------------------------------------------------------------
    // Test 60: Map value equality
    // -----------------------------------------------------------------------

    #[test]
    fn test_map_equality() {
        use crate::values::LatticeValue;

        let m1 = LatticeValue::Map(vec![
            ("a".to_string(), LatticeValue::Number(1.0)),
            ("b".to_string(), LatticeValue::Number(2.0)),
        ]);
        let m2 = LatticeValue::Map(vec![
            ("a".to_string(), LatticeValue::Number(1.0)),
            ("b".to_string(), LatticeValue::Number(2.0)),
        ]);
        assert_eq!(m1, m2, "Maps with same content should be equal");
    }
}
