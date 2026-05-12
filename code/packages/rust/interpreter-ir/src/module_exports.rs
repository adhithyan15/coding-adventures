//! `IIRExport` and `IIRImport` — module system declarations (LANG33).
//!
//! # Design
//!
//! A module system at the IIR level gives every language frontend
//! cross-module function calls for free.  Rather than every language
//! implementing its own "extern" or "require" mechanism, the IIR module
//! carries:
//!
//! - [`IIRExport`] — a function this module makes visible to other modules.
//! - [`IIRImport`] — a function this module requires from another module.
//!
//! The static linker (`iir-linker`) resolves imports to exports across a
//! set of peer `IIRModule`s and merges them into one linked module.
//!
//! Backends that have native import/export sections (WASM, JVM, CLR) can
//! alternatively use these lists to emit the appropriate section entries
//! without pre-linking, supporting lazy / dynamic resolution at runtime.
//!
//! # Backward compatibility
//!
//! Both fields default to empty `Vec`s.  Existing `IIRModule`s built before
//! LANG33 export nothing and import nothing — identical to pre-LANG33
//! behaviour.
//!
//! # Example
//!
//! ```rust
//! use interpreter_ir::module_exports::{IIRExport, IIRImport};
//!
//! // Module "math" exports "add" under the public name "add".
//! let export = IIRExport::new("add");
//! assert_eq!(export.public_name(), "add");
//!
//! // Module "math" exports "internal_sqrt" as "sqrt".
//! let aliased = IIRExport::new("internal_sqrt").with_alias("sqrt");
//! assert_eq!(aliased.public_name(), "sqrt");
//!
//! // Module "main" imports "add" from "math".
//! let import = IIRImport::new("math", "add", "i64");
//! assert_eq!(import.local_name(), "add");
//! ```

// ---------------------------------------------------------------------------
// IIRExport
// ---------------------------------------------------------------------------

/// A function that an [`IIRModule`](crate::module::IIRModule) makes visible
/// to other modules.
///
/// The `function_name` must refer to a function defined in the same module.
/// The optional `alias` lets the module publish a different public name (e.g.
/// renaming an internal implementation name to a clean API name).
///
/// # Encoding in backends
///
/// | Backend | What happens |
/// |---------|-------------|
/// | BEAM    | Listed in the `ExpT` chunk as `{FunctionName, Arity}`. |
/// | WASM    | Becomes an `Export` entry (kind = Function). |
/// | JVM     | Function compiled as `public static`. |
/// | CLR     | Function compiled as `.method public static`. |
#[derive(Debug, Clone, PartialEq)]
pub struct IIRExport {
    /// The name of the function as it appears in
    /// [`IIRModule::functions`](crate::module::IIRModule::functions).
    pub function_name: String,

    /// Optional public-facing alias.
    ///
    /// `None` → exported under the same name as `function_name`.
    /// `Some("sqrt")` → the function is exported as `"sqrt"` while the
    /// internal name stays `function_name`.
    pub alias: Option<String>,
}

impl IIRExport {
    /// Create a new export with no alias.
    ///
    /// ```
    /// # use interpreter_ir::module_exports::IIRExport;
    /// let e = IIRExport::new("add");
    /// assert_eq!(e.public_name(), "add");
    /// ```
    pub fn new(function_name: impl Into<String>) -> Self {
        IIRExport { function_name: function_name.into(), alias: None }
    }

    /// Builder: publish the function under a different name.
    ///
    /// ```
    /// # use interpreter_ir::module_exports::IIRExport;
    /// let e = IIRExport::new("internal_sqrt").with_alias("sqrt");
    /// assert_eq!(e.function_name, "internal_sqrt");
    /// assert_eq!(e.public_name(), "sqrt");
    /// ```
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into());
        self
    }

    /// The name that external modules use to refer to this function.
    ///
    /// Returns the alias if set, otherwise `function_name`.
    pub fn public_name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.function_name)
    }
}

// ---------------------------------------------------------------------------
// IIRImport
// ---------------------------------------------------------------------------

/// A function that an [`IIRModule`](crate::module::IIRModule) requires from
/// another module.
///
/// During static linking (`iir-linker::link`), each import is resolved to the
/// corresponding `IIRExport` in a peer module.  Type signatures are checked
/// if provided; a mismatch produces a [`LinkError::TypeMismatch`].
///
/// # Encoding in backends
///
/// | Backend | What happens |
/// |---------|-------------|
/// | BEAM    | After static linking, the merged module contains the function directly.  Before linking, `call` instructions targeting the import become `call_ext` with an import-table entry. |
/// | WASM    | Becomes an `Import` entry (module = `module_name`, name = `function_name`).  Function index map starts at N (imported count). |
/// | JVM     | Becomes an `invokestatic` with a Methodref CP entry pointing to the exporting class. |
/// | CLR     | Becomes a `call` with a MemberRef token pointing to the exporting assembly. |
///
/// [`LinkError::TypeMismatch`]: https://docs.rs/iir-linker/latest/iir_linker/enum.LinkError.html
#[derive(Debug, Clone, PartialEq)]
pub struct IIRImport {
    /// The module that provides this function.
    ///
    /// For static linking this must match the `name` of a peer `IIRModule`
    /// passed to `iir_linker::link`.  For native import sections (WASM/JVM)
    /// this becomes the module or class name.
    pub module_name: String,

    /// The function name as published by the exporting module (i.e. the
    /// `public_name()` of the corresponding `IIRExport`).
    pub function_name: String,

    /// The name used inside *this* module's call instructions.
    ///
    /// `None` → call instructions use `function_name` directly.
    /// `Some("math_add")` → call instructions write `call("math_add", …)`
    /// while the resolved external name remains `function_name`.
    pub local_alias: Option<String>,

    /// Expected parameter types for type-checking during linking.
    ///
    /// Empty means "don't check" (trusted import, no type verification).
    /// When non-empty, each entry is an IIR type string (`"i64"`, `"bool"`, …).
    pub param_types: Vec<String>,

    /// Expected return type of the imported function.
    ///
    /// `"any"` means "don't check".
    pub return_type: String,
}

impl IIRImport {
    /// Create a new import with no type annotations.
    ///
    /// # Arguments
    ///
    /// * `module_name` — the exporting module's name.
    /// * `function_name` — the public function name in that module.
    /// * `return_type` — the return type (`"void"`, `"i64"`, `"any"`, …).
    ///
    /// ```
    /// # use interpreter_ir::module_exports::IIRImport;
    /// let imp = IIRImport::new("math", "sqrt", "f64");
    /// assert_eq!(imp.module_name, "math");
    /// assert_eq!(imp.function_name, "sqrt");
    /// assert_eq!(imp.local_name(), "sqrt");
    /// assert_eq!(imp.return_type, "f64");
    /// ```
    pub fn new(
        module_name:   impl Into<String>,
        function_name: impl Into<String>,
        return_type:   impl Into<String>,
    ) -> Self {
        IIRImport {
            module_name:   module_name.into(),
            function_name: function_name.into(),
            local_alias:   None,
            param_types:   Vec::new(),
            return_type:   return_type.into(),
        }
    }

    /// Builder: set the local alias used by call instructions inside this module.
    ///
    /// ```
    /// # use interpreter_ir::module_exports::IIRImport;
    /// let imp = IIRImport::new("math", "add", "i64").with_local_alias("math_add");
    /// assert_eq!(imp.local_name(), "math_add");
    /// assert_eq!(imp.function_name, "add");
    /// ```
    pub fn with_local_alias(mut self, alias: impl Into<String>) -> Self {
        self.local_alias = Some(alias.into());
        self
    }

    /// Builder: set the expected parameter types (for type-checking during linking).
    ///
    /// ```
    /// # use interpreter_ir::module_exports::IIRImport;
    /// let imp = IIRImport::new("math", "add", "i64")
    ///     .with_params(vec!["i64".into(), "i64".into()]);
    /// assert_eq!(imp.param_types.len(), 2);
    /// ```
    pub fn with_params(mut self, params: Vec<String>) -> Self {
        self.param_types = params;
        self
    }

    /// The name that call instructions inside this module use.
    ///
    /// Returns the local alias if set, otherwise `function_name`.
    pub fn local_name(&self) -> &str {
        self.local_alias.as_deref().unwrap_or(&self.function_name)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── IIRExport ──────────────────────────────────────────────────────────

    #[test]
    fn export_public_name_no_alias() {
        let e = IIRExport::new("add");
        assert_eq!(e.function_name, "add");
        assert_eq!(e.alias, None);
        assert_eq!(e.public_name(), "add");
    }

    #[test]
    fn export_public_name_with_alias() {
        let e = IIRExport::new("internal_sqrt").with_alias("sqrt");
        assert_eq!(e.function_name, "internal_sqrt");
        assert_eq!(e.alias, Some("sqrt".to_string()));
        assert_eq!(e.public_name(), "sqrt");
    }

    #[test]
    fn export_clone_is_equal() {
        let e = IIRExport::new("foo").with_alias("bar");
        assert_eq!(e.clone(), e);
    }

    #[test]
    fn export_debug_format_contains_function_name() {
        let e = IIRExport::new("my_fn");
        let s = format!("{:?}", e);
        assert!(s.contains("my_fn"));
    }

    // ── IIRImport ──────────────────────────────────────────────────────────

    #[test]
    fn import_local_name_no_alias() {
        let imp = IIRImport::new("math", "add", "i64");
        assert_eq!(imp.module_name, "math");
        assert_eq!(imp.function_name, "add");
        assert_eq!(imp.local_alias, None);
        assert_eq!(imp.local_name(), "add");
        assert_eq!(imp.return_type, "i64");
        assert!(imp.param_types.is_empty());
    }

    #[test]
    fn import_local_name_with_alias() {
        let imp = IIRImport::new("math", "add", "i64").with_local_alias("math_add");
        assert_eq!(imp.local_name(), "math_add");
        assert_eq!(imp.function_name, "add");
    }

    #[test]
    fn import_with_params() {
        let imp = IIRImport::new("math", "add", "i64")
            .with_params(vec!["i64".into(), "i64".into()]);
        assert_eq!(imp.param_types, vec!["i64", "i64"]);
    }

    #[test]
    fn import_clone_is_equal() {
        let imp = IIRImport::new("m", "f", "void").with_local_alias("fa");
        assert_eq!(imp.clone(), imp);
    }

    #[test]
    fn import_debug_format_contains_module_and_function() {
        let imp = IIRImport::new("mymod", "myfunc", "void");
        let s = format!("{:?}", imp);
        assert!(s.contains("mymod"));
        assert!(s.contains("myfunc"));
    }

    #[test]
    fn import_any_return_type_trusted() {
        let imp = IIRImport::new("lib", "compute", "any");
        assert_eq!(imp.return_type, "any");
        assert!(imp.param_types.is_empty());
    }

    #[test]
    fn import_void_return_type() {
        let imp = IIRImport::new("io", "print", "void");
        assert_eq!(imp.return_type, "void");
    }
}
