//! `LinkError` — all failure modes the static linker can produce.
//!
//! # Design
//!
//! The linker operates on a *set* of `IIRModule`s and tries to produce one
//! merged module.  Failure modes fall into seven categories:
//!
//! 1. **Unresolved** — an import could not be matched to any export.
//! 2. **TypeMismatch** — an import was matched, but the caller and callee
//!    disagree about parameter or return types.
//! 3. **DuplicateExport** — one module exports the same public name twice,
//!    so the `(module_name, function_name)` key is ambiguous.
//! 4. **ConflictingExport** — two *different* modules export the same public
//!    name.  The merged module has a single flat namespace, so only one
//!    function can own a bare public name (issue #18429).
//! 5. **DuplicateModuleName** — two input modules carry the same `name`, so
//!    anything keyed by module name (exports, imports) would mix them up.
//! 6. **UnresolvedEntryPoint** — the entry module's `entry_point` names a
//!    function that module neither defines nor imports.
//! 7. **UndeclaredCall** — a function reference (`call` callee,
//!    `alloc_closure` body, spawn target) names something that is neither a
//!    function of its module nor a declared import — for example another
//!    module's *private* function (issue #18432), or a frontend that forgot
//!    an `IIRImport`.
//!
//! Every variant carries enough context to produce an actionable error message
//! without requiring access to the original `IIRModule`s.
//!
//! ## Why 4, 5 and 6 are errors rather than "pick one"
//!
//! All three describe a name that could bind to more than one function.  The
//! merge pass resolves a bare name to the *first* function that owns it, so
//! before #18429 each of them linked "successfully" and then quietly ran the
//! wrong code:
//!
//! ```text
//!   module left:  export foo      ─┐
//!   module right: export foo      ─┴─► merged: foo, foo   every (foo) runs left's
//!
//!   module a (name "util"): import x from lib  ─┐ imports keyed by "util" —
//!   module b (name "util"): private x          ─┘ b's own x is overridden
//!
//!   module app: entry_point = "start", no start ─► binds to some other
//!                                                  module's private start
//! ```
//!
//! **UndeclaredCall** (7) is the same failure for ordinary references: the
//! variant existed from the start but nothing raised it until #18432, so a
//! call to a name the module neither defined nor imported bound to another
//! module's private function of that name.
//!
//! A linker that guesses is worse than one that refuses: the error is cheap
//! to fix at build time and very expensive to find at run time.
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

    /// The same `(module_name, public_name)` key is exported twice.
    ///
    /// Import resolution is ambiguous when there are multiple candidates;
    /// the linker rejects this rather than picking arbitrarily.  This fires
    /// when one module lists the same public name twice, and also when two
    /// modules share a name (which additionally raises
    /// [`LinkError::DuplicateModuleName`]).
    DuplicateExport {
        /// The module name in question.
        module_name: String,
        /// The public function name in question.
        function_name: String,
    },

    /// Two *different* modules export the same public name.
    ///
    /// Exported functions keep their bare name in the merged module (that is
    /// how importers, and Twig's bare-name cross-module calls, find them).
    /// Two of them cannot both be called `foo`: the merged module would hold
    /// two functions named `foo` and every reference — including the second
    /// module's calls to its *own* `foo` — would run the first one.
    ///
    /// | `first_module` exports | `second_module` exports | Result |
    /// |------------------------|-------------------------|--------|
    /// | `foo`                  | `foo`                   | error  |
    /// | `foo_impl` as `foo`    | `foo`                   | error (public names clash) |
    /// | `foo`                  | private `foo`           | ok — the private one is renamed |
    ConflictingExport {
        /// The public name both modules claim.
        function_name: String,
        /// The module that claimed it first (input order).
        first_module: String,
        /// The module that tried to claim it again.
        second_module: String,
    },

    /// Two input modules have the same `IIRModule::name`.
    ///
    /// The linker keys exports and import rewrites by module name, so two
    /// modules called `util` would see each other's imports — an import in
    /// one could override a same-named private function in the other.
    DuplicateModuleName {
        /// The name that appears more than once.
        module_name: String,
    },

    /// The entry module's `entry_point` does not resolve inside that module.
    ///
    /// The merged entry point is the first module (in input order) whose
    /// `entry_point` is set.  That name must be a function the module
    /// defines, or the local name of one of its imports.  Otherwise it would
    /// fall through to whatever function happens to own the bare name in the
    /// merged module — possibly another module's *private* function.
    UnresolvedEntryPoint {
        /// The module whose entry point was used.
        module_name: String,
        /// The entry-point name that did not resolve.
        entry_point: String,
    },

    /// A function reference names something that is neither a function of
    /// its module nor the local name of one of its imports.
    ///
    /// Function references are the operands the merge pass renames: the
    /// `call` callee, the `alloc_closure` body name, and the `task_spawn` /
    /// `group_spawn` target.  Left alone, such a name would stay bare and
    /// bind to whichever function owns it after merging — possibly another
    /// module's *private* function (issue #18432):
    ///
    /// ```text
    ///   lib_a: private helper ─┐
    ///   lib_b: private helper ─┼─► merged `helper` is lib_a's; app's call
    ///   app:   call helper    ─┘   would run it without importing anything
    /// ```
    ///
    /// Either the module really meant another module's function (then it
    /// must import it, and that module must export it), or the frontend
    /// forgot to declare an `IIRImport`.  Each `(module, function, callee)`
    /// is reported once.
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
            LinkError::ConflictingExport {
                function_name,
                first_module,
                second_module,
            } => write!(
                f,
                "LinkError::ConflictingExport: modules {first_module:?} and \
                 {second_module:?} both export {function_name:?}"
            ),
            LinkError::DuplicateModuleName { module_name } => write!(
                f,
                "LinkError::DuplicateModuleName: more than one input module \
                 is named {module_name:?}"
            ),
            LinkError::UnresolvedEntryPoint {
                module_name,
                entry_point,
            } => write!(
                f,
                "LinkError::UnresolvedEntryPoint: module {module_name:?} names \
                 entry point {entry_point:?}, which it neither defines nor imports"
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
    fn conflicting_export_display_names_both_modules() {
        let e = LinkError::ConflictingExport {
            function_name: "foo".into(),
            first_module: "left".into(),
            second_module: "right".into(),
        };
        let s = format!("{e}");
        assert!(s.contains("ConflictingExport"));
        assert!(s.contains("foo"));
        assert!(s.contains("left"));
        assert!(s.contains("right"));
    }

    #[test]
    fn duplicate_module_name_display_contains_name() {
        let e = LinkError::DuplicateModuleName { module_name: "util".into() };
        let s = format!("{e}");
        assert!(s.contains("DuplicateModuleName"));
        assert!(s.contains("util"));
    }

    #[test]
    fn unresolved_entry_point_display_contains_module_and_name() {
        let e = LinkError::UnresolvedEntryPoint {
            module_name: "app".into(),
            entry_point: "start".into(),
        };
        let s = format!("{e}");
        assert!(s.contains("UnresolvedEntryPoint"));
        assert!(s.contains("app"));
        assert!(s.contains("start"));
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
