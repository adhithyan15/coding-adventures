//! # Cross-language runtime errors.
//!
//! `RuntimeError` is the **shared** error type returned by every
//! `LangBinding` dispatch method (`apply_callable`, `send_message`,
//! `load_property`, `store_property`).  It is deliberately small and
//! unopinionated — each language layers its own exception model on
//! top:
//!
//! - **Lispy languages** map `RuntimeError` straight onto Scheme
//!   conditions or Common Lisp restarts.
//! - **Ruby / JavaScript / Python** allocate a language-specific
//!   exception object and raise it — the binding catches the
//!   `RuntimeError` at the dispatch boundary and re-raises in the
//!   language's exception machinery.
//! - **Smalltalk** triggers `doesNotUnderstand:` for `NoSuchMethod`
//!   and `MessageNotUnderstood` for general failures.
//! - **Tetrad / Perl** propagate via the language's own error
//!   conventions.
//!
//! The runtime never inspects the contents of a `RuntimeError`
//! beyond logging — it is a transport for "something went wrong;
//! the binding decides what to do".
//!
//! ## Why so few variants?
//!
//! Three principles:
//!
//! 1. **Mechanism, not policy** (LANG20 §"Architecture").  Each
//!    variant identifies the runtime *mechanism* that failed, not
//!    a language-level error class.  Languages translate as
//!    needed.
//! 2. **Stable ABI.**  `RuntimeError` crosses the C ABI boundary
//!    (LANG20 §"C ABI extensions") via discriminant + 64-bit
//!    payload.  Adding variants is backwards-compatible; reordering
//!    is not.
//! 3. **Defer real exception interop** to a future spec.  Cross-
//!    language exception unwinding (a Ruby exception caught in JS
//!    code, propagating through Twig) is a hard problem and is
//!    explicitly out of scope for LANG20.
//!
//! ## Variants
//!
//! | Variant | When | Typical caller response |
//! |---------|------|--------------------------|
//! | [`RuntimeError::NotCallable`] | `apply_callable` on a non-callable | language raises "TypeError: not a function" or similar |
//! | [`RuntimeError::NoSuchMethod`] | `send_message` selector not found on receiver | Ruby `NoMethodError`, Smalltalk `doesNotUnderstand:` |
//! | [`RuntimeError::NoSuchProperty`] | `load_property` / `store_property` on missing key (when language treats as error) | JS strict mode `ReferenceError`; Python `AttributeError` |
//! | [`RuntimeError::TypeError`] | builtin operand type mismatch | language-specific TypeError |
//! | [`RuntimeError::Custom`] | binding-defined message-based error | language passes through as-is |
//! | [`RuntimeError::Trap`] | a builtin hit a LANG83 trap (`Trap.Bounds`, `Trap.Overflow`, …) | the VM raises the same trap its own ops raise (twig-vm: `RunError::Trap`) |

use crate::value::SymbolId;

/// Errors returned by [`crate::LangBinding`] dispatch methods.
///
/// The runtime treats these as opaque transports — the binding is
/// responsible for translating them into the language's own
/// exception model at the dispatch boundary.  See module-level
/// docs for the mapping conventions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    /// `apply_callable` was invoked on a value that is not a
    /// callable in this language (a non-closure for Lispy, a
    /// non-function for JS, a non-Method for Smalltalk).
    NotCallable,

    /// `send_message`'s selector did not resolve on the receiver.
    /// Languages with method-missing (Ruby, Smalltalk) should
    /// catch this and dispatch to the override.
    NoSuchMethod {
        /// The selector that did not resolve.
        selector: SymbolId,
    },

    /// `load_property` or `store_property` on a key the receiver
    /// does not have.  Some languages treat this as a soft miss
    /// (return nil/undefined) and never raise; those languages
    /// should not return this variant.
    NoSuchProperty {
        /// The property key.
        key: SymbolId,
    },

    /// Type mismatch in a runtime operation — e.g. Lispy `(+ 1 'foo)`,
    /// JS `null.foo`, Ruby `1 + "x"`.
    ///
    /// The string is a binding-formatted human-readable message;
    /// the binding may parse it back if needed but typically just
    /// passes it to the language's TypeError constructor.
    TypeError(String),

    /// A binding-defined error.  The string is opaque to the
    /// runtime; the binding controls both the format and the
    /// downstream interpretation.
    Custom(String),

    /// A builtin hit one of the LANG83 standard traps — the closed set of
    /// kinds in `interpreter_ir::exception_kind` (`Trap.Bounds`,
    /// `Trap.Overflow`, …) — rather than a type mismatch.
    ///
    /// It is the shared-runtime twin of vm-core's `VMError::Trap` and
    /// twig-vm's `RunError::Trap`, and exists so that a builtin which does
    /// the same job as a LANG83 op fails the same way.  `dynval-runtime`'s
    /// `string-ref` / `substring` out of range are `Trap.Bounds`, like the
    /// `str_index` / `str_slice` ops, and `string-append` past the string
    /// ceiling is `Trap.Overflow`, like `str_concat` (#18800).  Before this
    /// variant the builtins could only say `TypeError`, so the same Twig
    /// expression failed with a different error depending on whether it
    /// compiled to the op or to the builtin.
    ///
    /// `kind` is the trap's registered name, e.g. `"Trap.Bounds"`.  This
    /// crate does not check it against the registry (it does not depend on
    /// `interpreter-ir`); the VM that receives it does.
    Trap {
        /// The registered trap kind, e.g. `"Trap.Bounds"`.
        kind: String,
    },
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::NotCallable => write!(f, "value is not callable"),
            RuntimeError::NoSuchMethod { selector } => {
                write!(f, "no such method: {selector}")
            }
            RuntimeError::NoSuchProperty { key } => {
                write!(f, "no such property: {key}")
            }
            RuntimeError::TypeError(msg) => write!(f, "type error: {msg}"),
            RuntimeError::Custom(msg) => write!(f, "{msg}"),
            // The same words as vm-core's and twig-vm's trap errors.
            RuntimeError::Trap { kind } => write!(f, "trap: {kind}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_callable_displays_cleanly() {
        let s = format!("{}", RuntimeError::NotCallable);
        assert_eq!(s, "value is not callable");
    }

    #[test]
    fn no_such_method_includes_selector() {
        let e = RuntimeError::NoSuchMethod { selector: SymbolId(7) };
        assert!(format!("{e}").contains("sym#7"));
    }

    #[test]
    fn no_such_property_includes_key() {
        let e = RuntimeError::NoSuchProperty { key: SymbolId(42) };
        assert!(format!("{e}").contains("sym#42"));
    }

    #[test]
    fn type_error_includes_message() {
        let e = RuntimeError::TypeError("expected number, got string".into());
        assert!(format!("{e}").contains("expected number"));
    }

    #[test]
    fn custom_passes_through_message() {
        let e = RuntimeError::Custom("anything".into());
        assert_eq!(format!("{e}"), "anything");
    }

    #[test]
    fn trap_displays_like_the_vms_traps() {
        let e = RuntimeError::Trap { kind: "Trap.Bounds".into() };
        assert_eq!(format!("{e}"), "trap: Trap.Bounds");
        assert_ne!(e, RuntimeError::Trap { kind: "Trap.Overflow".into() });
        assert_eq!(e.clone(), e);
    }

    #[test]
    fn implements_std_error() {
        // Compile-time check via trait object.
        let e: Box<dyn std::error::Error> = Box::new(RuntimeError::NotCallable);
        assert!(!format!("{e}").is_empty());
    }
}
