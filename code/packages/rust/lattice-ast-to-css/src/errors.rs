//! # Lattice error types — structured errors for the AST-to-CSS compiler.
//!
//! Every error in the Lattice compiler carries a human-readable message and
//! the source position (line and column) where the error occurred. Position
//! information comes directly from the tokens that triggered the error —
//! the Lattice lexer embeds line/column info in every token.
//!
//! # Error Hierarchy
//!
//! The error types mirror the compiler's three passes:
//!
//! ```text
//! Pass 1 — Symbol Collection:
//!   ReturnOutsideFunctionError  — @return outside a @function body
//!
//! Pass 3 — Expansion:
//!   UndefinedVariableError      — $var referenced but never declared
//!   UndefinedMixinError         — @include references unknown mixin
//!   UndefinedFunctionError      — function call references unknown function
//!   WrongArityError             — wrong number of arguments
//!   CircularReferenceError      — mixin or function calls itself
//!   TypeErrorInExpression       — incompatible types in arithmetic
//!   UnitMismatchError           — incompatible units in arithmetic
//!   MissingReturnError          — function has no @return
//!
//! Internal signals (not real errors):
//!   Return                      — signals @return value (unwound via Err)
//! ```
//!
//! All error variants are collected in the [`LatticeError`] enum. Callers can
//! match on specific variants or handle the whole family with a single arm.
//!
//! # Example
//!
//! ```no_run
//! use coding_adventures_lattice_ast_to_css::errors::LatticeError;
//! use coding_adventures_lattice_ast_to_css::transform_lattice;
//!
//! match transform_lattice("$x: red; h1 { color: $undefined; }") {
//!     Ok(css) => println!("{css}"),
//!     Err(LatticeError::UndefinedVariable { name, line, column }) => {
//!         eprintln!("Undefined variable '{name}' at {line}:{column}");
//!     }
//!     Err(e) => eprintln!("Error: {e}"),
//! }
//! ```

use std::fmt;

/// All errors that can occur during Lattice-to-CSS compilation.
///
/// Most variants carry position information (line/column) from the source token
/// that triggered the error, enabling human-friendly error messages.
///
/// The [`Return`] variant is an *internal signal* used to implement `@return`
/// statement semantics. It is not a real error — when a `@return` statement is
/// encountered inside a function body, we propagate it upward via `Err(Return)`
/// so that the function evaluator can catch it and unwrap the value. This is
/// the Rust idiom for what Python implements with `raise ReturnSignal(value)`.
#[derive(Debug, Clone, PartialEq)]
pub enum LatticeError {
    // ------------------------------------------------------------------
    // Internal signal: used for @return control flow
    // ------------------------------------------------------------------

    /// Internal signal: `@return` was encountered with this value.
    ///
    /// This is *not* a user-visible error. It is raised by the function body
    /// evaluator and caught by `evaluate_function_call`. Using `Err(Return)`
    /// for control flow is idiomatic Rust when you need non-local exit
    /// (similar to how Python raises an exception for @return).
    Return {
        /// The CSS text of the returned value (e.g., "16px", "#4a90d9").
        value: String,
    },

    // ------------------------------------------------------------------
    // Real user-visible errors
    // ------------------------------------------------------------------

    /// `@return` appeared outside a `@function` body.
    ///
    /// Example: `@return 42;` at the top level (not inside `@function`).
    ReturnOutsideFunction {
        line: usize,
        column: usize,
    },

    /// A `$variable` was referenced but never declared.
    ///
    /// Example: `color: $nonexistent;` when `$nonexistent` was never set.
    UndefinedVariable {
        name: String,
        line: usize,
        column: usize,
    },

    /// `@include` referenced a mixin that was never defined.
    ///
    /// Example: `@include nonexistent;` when no `@mixin nonexistent` exists.
    UndefinedMixin {
        name: String,
        suggestion: Option<String>,
        line: usize,
        column: usize,
    },

    /// A function call referenced a Lattice function that was never defined.
    ///
    /// Note: CSS built-in functions (rgb, calc, var, etc.) are NOT subject
    /// to this check — they are passed through unchanged.
    ///
    /// Example: `padding: spacing(2);` when `@function spacing` was never defined.
    UndefinedFunction {
        name: String,
        line: usize,
        column: usize,
    },

    /// A mixin or function was called with the wrong number of arguments.
    ///
    /// The `expected` field counts required parameters only (parameters with
    /// defaults are not required). `got` is the actual count passed.
    ///
    /// Example: `@mixin button($bg, $fg)` called as `@include button(red, blue, green);`
    WrongArity {
        /// "Mixin" or "Function" — what kind of thing was called.
        kind: String,
        name: String,
        expected: usize,
        got: usize,
        line: usize,
        column: usize,
    },

    /// A mixin or function directly or indirectly calls itself.
    ///
    /// The `chain` shows the full call path, e.g. `["a", "b", "a"]`
    /// for `@mixin a { @include b; }` and `@mixin b { @include a; }`.
    CircularReference {
        /// "mixin" or "function"
        kind: String,
        /// The call chain from the first occurrence to the repeat.
        chain: Vec<String>,
        line: usize,
        column: usize,
    },

    /// Arithmetic was attempted on incompatible types.
    ///
    /// Example: `10px + red` — can't add a dimension and an identifier.
    TypeError {
        /// "add", "subtract", "multiply", "negate"
        op: String,
        left_type: String,
        right_type: String,
        line: usize,
        column: usize,
    },

    /// Arithmetic was attempted on dimensions with incompatible units.
    ///
    /// Example: `10px + 5em` (different CSS length units can't be added
    /// at compile time — use `calc(10px + 5em)` instead).
    UnitMismatch {
        left_unit: String,
        right_unit: String,
        line: usize,
        column: usize,
    },

    /// A `@function` body has no `@return` statement.
    ///
    /// Every function must return a value. A function body that contains
    /// variable declarations but no `@return` in any reachable branch is
    /// an error.
    ///
    /// Example: `@function noop($x) { $y: $x; }` — never returns.
    MissingReturn {
        name: String,
        line: usize,
        column: usize,
    },

    // ------------------------------------------------------------------
    // Lattice v2: New Error Types
    // ------------------------------------------------------------------

    /// A `@while` loop exceeded the maximum iteration count.
    ///
    /// The max-iteration guard prevents infinite loops at compile time.
    /// Default limit is 1000 iterations. If a `@while` loop's condition
    /// remains truthy after this many iterations, compilation halts.
    ///
    /// Example: `@while true { }` with no mutation to break the loop.
    MaxIteration {
        max_iterations: usize,
        line: usize,
        column: usize,
    },

    /// `@extend` references a selector not found in the stylesheet.
    ///
    /// `@extend` works by appending the current rule's selector to another
    /// rule's selector list. If the target does not exist, this is an error.
    ///
    /// Example: `@extend %nonexistent;` where `%nonexistent` is never defined.
    ExtendTargetNotFound {
        target: String,
        line: usize,
        column: usize,
    },

    /// A value is outside the valid range for an operation.
    ///
    /// Used by built-in functions with bounded inputs: `nth()`, `lighten()`,
    /// `mix()`, etc.
    ///
    /// Example: `nth((a, b, c), 5)` — index 5 out of bounds.
    Range {
        message: String,
        line: usize,
        column: usize,
    },

    /// Division by zero in `math.div()`.
    ///
    /// Unlike CSS `calc()` which defers to the browser, Lattice evaluates
    /// `math.div()` at compile time and must reject zero divisors.
    ZeroDivision {
        line: usize,
        column: usize,
    },
}

// ===========================================================================
// Display — human-readable error messages
// ===========================================================================

impl fmt::Display for LatticeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LatticeError::Return { value } => {
                // Internal signal — should never be displayed to users.
                write!(f, "@return {value}")
            }
            LatticeError::ReturnOutsideFunction { line, column } => {
                write!(f, "@return outside @function at line {line}, column {column}")
            }
            LatticeError::UndefinedVariable { name, line, column } => {
                write!(f, "Undefined variable '{name}' at line {line}, column {column}")
            }
            LatticeError::UndefinedMixin { name, suggestion, line, column } => {
                match suggestion {
                    Some(suggested_name) => write!(
                        f,
                        "Undefined mixin '{name}'. Did you mean '{suggested_name}'? at line {line}, column {column}"
                    ),
                    None => write!(f, "Undefined mixin '{name}' at line {line}, column {column}"),
                }
            }
            LatticeError::UndefinedFunction { name, line, column } => {
                write!(f, "Undefined function '{name}' at line {line}, column {column}")
            }
            LatticeError::WrongArity { kind, name, expected, got, line, column } => {
                write!(
                    f,
                    "{kind} '{name}' expects {expected} args, got {got} at line {line}, column {column}"
                )
            }
            LatticeError::CircularReference { kind, chain, line, column } => {
                let chain_str = chain.join(" → ");
                write!(f, "Circular {kind}: {chain_str} at line {line}, column {column}")
            }
            LatticeError::TypeError { op, left_type, right_type, line, column } => {
                write!(
                    f,
                    "Cannot {op} '{left_type}' and '{right_type}' at line {line}, column {column}"
                )
            }
            LatticeError::UnitMismatch { left_unit, right_unit, line, column } => {
                write!(
                    f,
                    "Cannot add '{left_unit}' and '{right_unit}' units at line {line}, column {column}"
                )
            }
            LatticeError::MissingReturn { name, line, column } => {
                write!(f, "Function '{name}' has no @return at line {line}, column {column}")
            }
            LatticeError::MaxIteration { max_iterations, line, column } => {
                write!(
                    f,
                    "@while loop exceeded maximum iteration count ({max_iterations}) at line {line}, column {column}"
                )
            }
            LatticeError::ExtendTargetNotFound { target, line, column } => {
                write!(
                    f,
                    "@extend target '{target}' was not found in the stylesheet at line {line}, column {column}"
                )
            }
            LatticeError::Range { message, line, column } => {
                write!(f, "{message} at line {line}, column {column}")
            }
            LatticeError::ZeroDivision { line, column } => {
                write!(f, "Division by zero at line {line}, column {column}")
            }
        }
    }
}

// Implement the standard Error trait so LatticeError can be used with
// the ? operator and standard error handling patterns.
impl std::error::Error for LatticeError {}

// ===========================================================================
// Constructor helpers
// ===========================================================================
//
// These associated functions provide a clean API for creating errors without
// having to specify all fields. They mirror the Python error constructors.

impl LatticeError {
    /// Create a `Return` signal with the given CSS text value.
    pub fn return_signal(value: impl Into<String>) -> Self {
        LatticeError::Return { value: value.into() }
    }

    /// Create an `UndefinedVariable` error.
    pub fn undefined_variable(name: impl Into<String>, line: usize, column: usize) -> Self {
        LatticeError::UndefinedVariable {
            name: name.into(),
            line,
            column,
        }
    }

    /// Create an `UndefinedMixin` error.
    pub fn undefined_mixin(
        name: impl Into<String>,
        line: usize,
        column: usize,
        suggestion: Option<String>,
    ) -> Self {
        LatticeError::UndefinedMixin {
            name: name.into(),
            suggestion,
            line,
            column,
        }
    }

    /// Create an `UndefinedFunction` error.
    pub fn undefined_function(name: impl Into<String>, line: usize, column: usize) -> Self {
        LatticeError::UndefinedFunction {
            name: name.into(),
            line,
            column,
        }
    }

    /// Create a `WrongArity` error.
    pub fn wrong_arity(
        kind: impl Into<String>,
        name: impl Into<String>,
        expected: usize,
        got: usize,
        line: usize,
        column: usize,
    ) -> Self {
        LatticeError::WrongArity {
            kind: kind.into(),
            name: name.into(),
            expected,
            got,
            line,
            column,
        }
    }

    /// Create a `CircularReference` error.
    pub fn circular_reference(
        kind: impl Into<String>,
        chain: Vec<String>,
        line: usize,
        column: usize,
    ) -> Self {
        LatticeError::CircularReference {
            kind: kind.into(),
            chain,
            line,
            column,
        }
    }

    /// Create a `TypeError` error.
    pub fn type_error(
        op: impl Into<String>,
        left_type: impl Into<String>,
        right_type: impl Into<String>,
        line: usize,
        column: usize,
    ) -> Self {
        LatticeError::TypeError {
            op: op.into(),
            left_type: left_type.into(),
            right_type: right_type.into(),
            line,
            column,
        }
    }

    /// Create a `UnitMismatch` error.
    pub fn unit_mismatch(
        left_unit: impl Into<String>,
        right_unit: impl Into<String>,
        line: usize,
        column: usize,
    ) -> Self {
        LatticeError::UnitMismatch {
            left_unit: left_unit.into(),
            right_unit: right_unit.into(),
            line,
            column,
        }
    }

    /// Create a `MissingReturn` error.
    pub fn missing_return(name: impl Into<String>, line: usize, column: usize) -> Self {
        LatticeError::MissingReturn {
            name: name.into(),
            line,
            column,
        }
    }

    /// Create a `MaxIteration` error.
    pub fn max_iteration(max_iterations: usize, line: usize, column: usize) -> Self {
        LatticeError::MaxIteration { max_iterations, line, column }
    }

    /// Create an `ExtendTargetNotFound` error.
    pub fn extend_target_not_found(target: impl Into<String>, line: usize, column: usize) -> Self {
        LatticeError::ExtendTargetNotFound { target: target.into(), line, column }
    }

    /// Create a `Range` error.
    pub fn range_error(message: impl Into<String>, line: usize, column: usize) -> Self {
        LatticeError::Range { message: message.into(), line, column }
    }

    /// Create a `ZeroDivision` error.
    pub fn zero_division(line: usize, column: usize) -> Self {
        LatticeError::ZeroDivision { line, column }
    }

    /// Check if this is a `Return` signal (not a real error).
    pub fn is_return(&self) -> bool {
        matches!(self, LatticeError::Return { .. })
    }

    /// Extract the return value from a `Return` signal. Returns `None` if
    /// this is not a `Return` variant.
    pub fn return_value(&self) -> Option<&str> {
        if let LatticeError::Return { value } = self {
            Some(value)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undefined_variable_display() {
        let e = LatticeError::undefined_variable("$color", 5, 12);
        let msg = e.to_string();
        assert!(msg.contains("$color"));
        assert!(msg.contains("5"));
        assert!(msg.contains("12"));
    }

    #[test]
    fn test_return_signal() {
        let e = LatticeError::return_signal("16px");
        assert!(e.is_return());
        assert_eq!(e.return_value(), Some("16px"));
    }

    #[test]
    fn test_wrong_arity_display() {
        let e = LatticeError::wrong_arity("Mixin", "button", 2, 3, 1, 1);
        let msg = e.to_string();
        assert!(msg.contains("button"));
        assert!(msg.contains("2"));
        assert!(msg.contains("3"));
    }

    #[test]
    fn test_circular_reference_display() {
        let e = LatticeError::circular_reference(
            "mixin",
            vec!["a".to_string(), "b".to_string(), "a".to_string()],
            1, 1,
        );
        let msg = e.to_string();
        assert!(msg.contains("a → b → a"));
    }

    #[test]
    fn test_type_error_display() {
        let e = LatticeError::type_error("add", "10px", "red", 3, 7);
        let msg = e.to_string();
        assert!(msg.contains("add"));
        assert!(msg.contains("10px"));
        assert!(msg.contains("red"));
    }

    #[test]
    fn test_error_is_std_error() {
        let e = LatticeError::undefined_variable("$x", 1, 1);
        // Verify it implements std::error::Error
        let _: &dyn std::error::Error = &e;
    }

    #[test]
    fn test_max_iteration_display() {
        let e = LatticeError::max_iteration(1000, 5, 3);
        let msg = e.to_string();
        assert!(msg.contains("1000"));
        assert!(msg.contains("@while"));
    }

    #[test]
    fn test_extend_target_not_found_display() {
        let e = LatticeError::extend_target_not_found("%message", 10, 5);
        let msg = e.to_string();
        assert!(msg.contains("%message"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn test_range_error_display() {
        let e = LatticeError::range_error("Index out of bounds", 1, 1);
        let msg = e.to_string();
        assert!(msg.contains("Index out of bounds"));
    }

    #[test]
    fn test_zero_division_display() {
        let e = LatticeError::zero_division(3, 7);
        let msg = e.to_string();
        assert!(msg.contains("Division by zero"));
    }
}
