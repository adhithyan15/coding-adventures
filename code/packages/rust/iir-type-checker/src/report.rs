//! `TypeCheckReport` — the result of a type-checking pass over an `IIRModule`.
//!
//! A `TypeCheckReport` is the complete answer to the question:
//! "How typed is this module, and where are the problems?"
//!
//! It bundles:
//!
//! - The [`TypingTier`] classification (Untyped / Partial / FullyTyped).
//! - Any [`TypeCheckError`]s (fatal — block optimised code-gen for the instruction).
//! - Any [`TypeCheckWarning`]s (informational — code-gen proceeds).
//! - A `HashMap<String, String>` of **inferred type annotations**: variable
//!   names (destination registers) to the type the checker inferred for them.
//!   Populated by [`crate::infer::infer_types_mut`] before checking; empty when
//!   only [`crate::check::check_module`] is used.
//!
//! # Quick-start
//!
//! ```
//! use interpreter_ir::module::IIRModule;
//! use interpreter_ir::function::IIRFunction;
//! use interpreter_ir::instr::{IIRInstr, Operand};
//! use iir_type_checker::check::check_module;
//! use iir_type_checker::tier::TypingTier;
//!
//! let fn_ = IIRFunction::new(
//!     "main", vec![], "void",
//!     vec![IIRInstr::new("ret_void", None, vec![], "void")],
//! );
//! let mut module = IIRModule::new("test", "tetrad");
//! module.functions.push(fn_);
//! let report = check_module(&module);
//!
//! assert!(report.errors.is_empty());
//! assert_eq!(report.tier, TypingTier::Untyped); // ret_void has no data dest
//! ```

use std::collections::HashMap;

use crate::errors::{TypeCheckError, TypeCheckWarning};
use crate::tier::TypingTier;

/// The complete type-check result for an [`IIRModule`](interpreter_ir::module::IIRModule).
///
/// Returned by [`crate::check::check_module`] and [`crate::infer_and_check`].
#[derive(Debug, Clone)]
pub struct TypeCheckReport {
    /// The typing tier of the module: `Untyped`, `Partial(f)`, or `FullyTyped`.
    pub tier: TypingTier,

    /// The fraction of data-flow instructions (those with a `dest` register)
    /// that carry a **concrete** `type_hint`.  This is always in `[0.0, 1.0]`
    /// and agrees with `tier`:
    ///
    /// - `0.0` → `Untyped`
    /// - `1.0` → `FullyTyped`
    /// - `(0, 1)` → `Partial(typed_fraction)`
    pub typed_fraction: f32,

    /// Fatal errors that the type checker detected.
    ///
    /// Each error prevents optimised code generation for the affected
    /// instruction.  The instruction will be compiled as a generic call-runtime
    /// operation.  The compiler should not abort on non-empty errors — instead
    /// it should degrade gracefully.
    pub errors: Vec<TypeCheckError>,

    /// Non-fatal warnings detected during type checking.
    ///
    /// Informational only.  The compiler should log these when run in
    /// verbose / developer mode.
    pub warnings: Vec<TypeCheckWarning>,

    /// Types inferred by the inference pass for destination registers.
    ///
    /// Keys are destination variable names (e.g. `"v0"`, `"tmp_1"`);
    /// values are concrete type strings (e.g. `"i64"`, `"bool"`).
    ///
    /// This map is empty when the module was already fully annotated
    /// before checking, or when only [`crate::check::check_module`] was called
    /// (no inference pass ran).
    pub inferred_types: HashMap<String, String>,
}

impl TypeCheckReport {
    /// Create an empty report with a given typed fraction.
    pub(crate) fn new(typed_fraction: f32) -> Self {
        TypeCheckReport {
            tier: TypingTier::from_fraction(typed_fraction),
            typed_fraction,
            errors: Vec::new(),
            warnings: Vec::new(),
            inferred_types: HashMap::new(),
        }
    }

    /// Return `true` if there are no fatal errors.
    ///
    /// ```
    /// use interpreter_ir::module::IIRModule;
    /// use interpreter_ir::function::IIRFunction;
    /// use interpreter_ir::instr::{IIRInstr, Operand};
    /// use iir_type_checker::check::check_module;
    ///
    /// let fn_ = IIRFunction::new(
    ///     "main", vec![], "void",
    ///     vec![IIRInstr::new("ret_void", None, vec![], "void")],
    /// );
    /// let mut module = IIRModule::new("t", "tetrad");
    /// module.functions.push(fn_);
    /// assert!(check_module(&module).ok());
    /// ```
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// Return a short summary string suitable for log output.
    ///
    /// ```
    /// use interpreter_ir::module::IIRModule;
    /// use interpreter_ir::function::IIRFunction;
    /// use interpreter_ir::instr::{IIRInstr, Operand};
    /// use iir_type_checker::check::check_module;
    ///
    /// let fn_ = IIRFunction::new(
    ///     "main", vec![], "void",
    ///     vec![IIRInstr::new("ret_void", None, vec![], "void")],
    /// );
    /// let mut module = IIRModule::new("t", "tetrad");
    /// module.functions.push(fn_);
    /// let report = check_module(&module);
    /// let s = report.summary();
    /// assert!(s.contains("errors=0"));
    /// ```
    pub fn summary(&self) -> String {
        format!(
            "tier={} typed={:.0}% errors={} warnings={} inferred={}",
            self.tier,
            self.typed_fraction * 100.0,
            self.errors.len(),
            self.warnings.len(),
            self.inferred_types.len(),
        )
    }
}

impl std::fmt::Display for TypeCheckReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_empty_report() {
        let r = TypeCheckReport::new(0.0);
        assert!(r.ok());
        assert_eq!(r.tier, TypingTier::Untyped);
        assert!(r.errors.is_empty());
        assert!(r.warnings.is_empty());
        assert!(r.inferred_types.is_empty());
    }

    #[test]
    fn ok_is_false_when_errors_present() {
        let mut r = TypeCheckReport::new(1.0);
        r.errors.push(crate::errors::TypeCheckError {
            fn_name: "f".into(),
            instr_idx: 0,
            kind: crate::errors::ErrorKind::InvalidType,
            message: "bad".into(),
        });
        assert!(!r.ok());
    }

    #[test]
    fn summary_contains_key_fields() {
        let r = TypeCheckReport::new(0.5);
        let s = r.summary();
        assert!(s.contains("Partial"));
        assert!(s.contains("errors=0"));
        assert!(s.contains("warnings=0"));
    }
}
