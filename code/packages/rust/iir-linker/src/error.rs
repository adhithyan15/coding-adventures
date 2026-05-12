//! `LinkError` — all failure modes the static linker can produce.
//!
//! # Design
//!
//! The linker operates on a *set* of `IIRModule`s and tries to produce one
//! merged module.  Failure modes fall into four categories:
//!
//! 1. **Unresolved** — an import could not be matched to any export.
//! 2. **TypeMismatch** — an import was matched, but the caller and callee
//!    disagree about parameter or return types.
//! 3. **DuplicateExport** — two modules export the same `(module_name,
//!    function_name)` pair, making import resolution ambiguous.
//! 4. **UndeclaredCall** — a `call` instruction targets a name that is neither
//!    a local function nor a declared import.  This catches compiler bugs where
//!    the frontend forgot to add an `IIRImport` entry.
//!
//! All four variants carry enough context to produce a actionable error message
//! without requiring access to the original `IIRModule`s.
//!
//! # Example
//!
//! ```rust
//! use iir_linker::error::LinkError;
//!
//! let e = LinkError::Unresolved {
//!     importing_module: "main".into(),
//!     import_module:    "math".into(),
//!     import_function:  "sqrt".into(),
//! };
//! assert!(format!("{e}").contains("sqrt"));
//! ```

/// All failure modes that `iir_linker::link` can produce.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkError {
    /// An import could not be resolved to any exported function.
    ///
    /// Occurs when none of the peer `IIRModule`s export a function under
    /// the expected `(module_name, function_name)` key.
    Unresolved {
        /// The module that declared the unsatisfied import.
        importing_module: String,
        /// The module name the import expected to find the function in.
        import_module: String,
        /// The function name (public name) that was not found.
        import_function: String,
    },

    /// An import was resolved but the type signatures don't match.
    ///
    /// Only checked when the import provides non-empty `param_types` or a
    /// non-`"any"` `return_type`.
    TypeMismatch {
        /// The module that declared the import with mismatched types.
        importing_module: String,
        /// The module that exports the function.
        exporting_module: String,
        /// The function name (public name).
        function: String,
        /// What the importer expected (`import.param_types`).
        expected: Vec<String>,
        /// What the exporter actually has (`export_fn.params.*.type`).
        actual: Vec<String>,
    },

    /// Two modules both export under the same `(module_name, public_name)` key.
    ///
    /// Import resolution is ambiguous when there are multiple candidates;
    /// the linker rejects this rather than picking arbitrarily.
    DuplicateExport {
        /// The module name in question.
        module_name: String,
        /// The public function name in question.
        function_name: String,
    },

    /// A `call` instruction targets a name that is neither a local function
    /// nor a declared import.
    ///
    /// This is a compiler bug — the frontend generated a `call("foo", …)`
    /// without declaring `"foo"` in `IIRModule::imports`.
    UndeclaredCall {
        /// The module containing the bad call instruction.
        in_module: String,
        /// The function containing the bad call instruction.
        in_function: String,
        /// The callee name that was neither local nor imported.
        callee: String,
    },
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkError::Unresolved {
                importing_module,
                import_module,
                import_function,
            } => write!(
                f,
                "LinkError::Unresolved: module {importing_module:?} requires \
                 {import_module:?}::{import_function:?} but no matching export was found"
            ),
            LinkError::TypeMismatch {
                importing_module,
                exporting_module,
                function,
                expected,
                actual,
            } => write!(
                f,
                "LinkError::TypeMismatch: {importing_module:?} imports \
                 {exporting_module:?}::{function:?} with params {expected:?} \
                 but exporter has {actual:?}"
            ),
            LinkError::DuplicateExport {
                module_name,
                function_name,
            } => write!(
                f,
                "LinkError::DuplicateExport: multiple modules export \
                 {module_name:?}::{function_name:?}"
            ),
            LinkError::UndeclaredCall {
                in_module,
                in_function,
                callee,
            } => write!(
                f,
                "LinkError::UndeclaredCall: function {in_function:?} in module \
                 {in_module:?} calls {callee:?} which is not a local function \
                 or declared import"
            ),
        }
    }
}

impl std::error::Error for LinkError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unresolved_display_contains_function_name() {
        let e = LinkError::Unresolved {
            importing_module: "main".into(),
            import_module: "math".into(),
            import_function: "sqrt".into(),
        };
        let s = format!("{e}");
        assert!(s.contains("sqrt"));
        assert!(s.contains("math"));
        assert!(s.contains("main"));
    }

    #[test]
    fn type_mismatch_display_contains_expected_and_actual() {
        let e = LinkError::TypeMismatch {
            importing_module: "app".into(),
            exporting_module: "lib".into(),
            function: "add".into(),
            expected: vec!["i64".into(), "i64".into()],
            actual: vec!["i32".into(), "i32".into()],
        };
        let s = format!("{e}");
        assert!(s.contains("add"));
        assert!(s.contains("i64"));
        assert!(s.contains("i32"));
    }

    #[test]
    fn duplicate_export_display_contains_names() {
        let e = LinkError::DuplicateExport {
            module_name: "math".into(),
            function_name: "sqrt".into(),
        };
        let s = format!("{e}");
        assert!(s.contains("math"));
        assert!(s.contains("sqrt"));
    }

    #[test]
    fn undeclared_call_display_contains_callee() {
        let e = LinkError::UndeclaredCall {
            in_module: "app".into(),
            in_function: "main".into(),
            callee: "mystery_fn".into(),
        };
        let s = format!("{e}");
        assert!(s.contains("mystery_fn"));
        assert!(s.contains("main"));
    }

    #[test]
    fn link_error_implements_std_error() {
        // Verify the trait impl compiles and the source chain is empty.
        let e = LinkError::Unresolved {
            importing_module: "x".into(),
            import_module: "y".into(),
            import_function: "z".into(),
        };
        // std::error::Error requires Display + Debug, both derived/impl'd.
        let _: &dyn std::error::Error = &e;
    }

    #[test]
    fn link_error_clone_and_partial_eq() {
        let e = LinkError::Unresolved {
            importing_module: "a".into(),
            import_module: "b".into(),
            import_function: "c".into(),
        };
        assert_eq!(e.clone(), e);
    }
}
