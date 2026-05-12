//! `IIRLinker` — the public stateful linker facade.
//!
//! Most callers want the free functions [`link`] and [`link_strict`] rather
//! than building an `IIRLinker` directly.  The struct API is provided for
//! tooling that wants to accumulate errors incrementally or inspect the
//! intermediate export map.

use interpreter_ir::module::IIRModule;

use crate::error::LinkError;
use crate::merge::merge_modules;
use crate::resolve::{build_export_map, resolve_imports, verify_imports_against};

// ---------------------------------------------------------------------------
// Free-function API (the recommended entry points)
// ---------------------------------------------------------------------------

/// Static-link two or more `IIRModule`s into one merged module.
///
/// # Algorithm
///
/// 1. Build the export map: `(module_name, public_name) → &IIRFunction`.
/// 2. Resolve every import against the export map; type-check if annotations
///    are present.
/// 3. Merge all functions into a single `IIRModule`, renaming collisions with
///    `"<module>::"` prefix and rewriting `call` instructions accordingly.
/// 4. Return the merged module, or all errors if any were found.
///
/// # Errors
///
/// Returns `Err(errors)` if any of:
/// - An import has no matching export (`LinkError::Unresolved`)
/// - An import's type annotations don't match (`LinkError::TypeMismatch`)
/// - Two modules export the same `(module_name, function_name)` pair
///   (`LinkError::DuplicateExport`)
///
/// # Example
///
/// ```rust
/// use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
/// use interpreter_ir::module_exports::{IIRExport, IIRImport};
/// use iir_linker::link;
///
/// let mut math = IIRModule::new("math", "twig");
/// math.entry_point = None;
/// math.add_or_replace(IIRFunction::new(
///     "add",
///     vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
///     "i64",
///     vec![IIRInstr::new("ret_void", None, vec![], "void")],
/// ));
/// math.exports.push(IIRExport::new("add"));
///
/// let mut app = IIRModule::new("app", "twig");
/// app.add_or_replace(IIRFunction::new(
///     "main", vec![], "void",
///     vec![IIRInstr::new("ret_void", None, vec![], "void")],
/// ));
/// app.imports.push(IIRImport::new("math", "add", "any"));
///
/// let merged = link(&[math, app]).unwrap();
/// assert!(merged.get_function("add").is_some());
/// assert!(merged.get_function("main").is_some());
/// assert!(merged.imports.is_empty()); // Self-contained after linking.
/// ```
pub fn link(modules: &[IIRModule]) -> Result<IIRModule, Vec<LinkError>> {
    IIRLinker::new().link(modules)
}

/// Link-and-fail-fast variant — returns `Err` on the first error found.
///
/// Useful for build tooling that wants a clear first error rather than a
/// batch.  Internally collects all errors via `link` and returns the first.
pub fn link_strict(modules: &[IIRModule]) -> Result<IIRModule, LinkError> {
    link(modules).map_err(|mut errors| {
        errors.remove(0) // errors is never empty when Err is returned
    })
}

/// Verify that all imports in `module` are satisfied by `providers`.
///
/// Does **not** merge or rewrite — useful for pre-flight checking in the REPL
/// before committing to a full link.
///
/// Returns a (possibly empty) list of `LinkError`s.
pub fn verify_imports(module: &IIRModule, providers: &[&IIRModule]) -> Vec<LinkError> {
    verify_imports_against(module, providers)
}

// ---------------------------------------------------------------------------
// IIRLinker struct (stateful)
// ---------------------------------------------------------------------------

/// Stateful linker.  Holds no long-lived state between calls; the struct API
/// is provided for consistency with the LANG20 `CodeGenerator` pattern.
pub struct IIRLinker;

impl IIRLinker {
    pub fn new() -> Self {
        IIRLinker
    }

    /// Link `modules` into one merged module.  Same algorithm as the free
    /// function [`link`].
    pub fn link(&self, modules: &[IIRModule]) -> Result<IIRModule, Vec<LinkError>> {
        let mut all_errors: Vec<LinkError> = Vec::new();

        // Step 1: Build export map (may produce DuplicateExport errors).
        let (export_map, mut dup_errors) = build_export_map(modules);
        all_errors.append(&mut dup_errors);

        // Step 2: Resolve imports (may produce Unresolved + TypeMismatch errors).
        let mut import_errors = resolve_imports(modules, &export_map);
        all_errors.append(&mut import_errors);

        if !all_errors.is_empty() {
            return Err(all_errors);
        }

        // Step 3: Merge.
        Ok(merge_modules(modules, &export_map))
    }
}

impl Default for IIRLinker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};
    use interpreter_ir::module_exports::{IIRExport, IIRImport};

    fn make_ret_void_fn(name: &str, params: Vec<(&str, &str)>) -> IIRFunction {
        IIRFunction::new(
            name,
            params.into_iter().map(|(n, t)| (n.to_string(), t.to_string())).collect(),
            "void",
            vec![IIRInstr::new("ret_void", None, vec![], "void")],
        )
    }

    #[test]
    fn link_two_modules_produces_merged() {
        let mut math = IIRModule::new("math", "twig");
        math.entry_point = None;
        math.add_or_replace(make_ret_void_fn("add", vec![("a", "i64"), ("b", "i64")]));
        math.exports.push(IIRExport::new("add"));

        let mut app = IIRModule::new("app", "twig");
        app.add_or_replace(make_ret_void_fn("main", vec![]));
        app.imports.push(IIRImport::new("math", "add", "any"));

        let merged = link(&[math, app]).unwrap();
        assert!(merged.get_function("add").is_some());
        assert!(merged.get_function("main").is_some());
    }

    #[test]
    fn link_returns_unresolved_when_import_missing() {
        let mut app = IIRModule::new("app", "twig");
        app.entry_point = None;
        app.imports.push(IIRImport::new("nonexistent", "fn", "any"));

        let errs = link(&[app]).unwrap_err();
        assert!(errs.iter().any(|e| matches!(e, LinkError::Unresolved { .. })));
    }

    #[test]
    fn link_strict_returns_first_error() {
        let mut app = IIRModule::new("app", "twig");
        app.entry_point = None;
        app.imports.push(IIRImport::new("x", "a", "any"));
        app.imports.push(IIRImport::new("x", "b", "any"));

        let err = link_strict(&[app]).unwrap_err();
        assert!(matches!(err, LinkError::Unresolved { .. }));
    }

    #[test]
    fn link_single_module_no_imports() {
        let mut m = IIRModule::new("prog", "twig");
        m.add_or_replace(make_ret_void_fn("main", vec![]));
        let merged = link(&[m]).unwrap();
        assert_eq!(merged.functions.len(), 1);
        assert!(merged.get_function("main").is_some());
    }

    #[test]
    fn link_preserves_entry_point() {
        let mut m = IIRModule::new("app", "twig");
        m.add_or_replace(make_ret_void_fn("main", vec![]));
        m.entry_point = Some("main".into());
        let merged = link(&[m]).unwrap();
        assert_eq!(merged.entry_point, Some("main".into()));
    }

    #[test]
    fn verify_imports_returns_empty_when_satisfied() {
        let mut math = IIRModule::new("math", "twig");
        math.entry_point = None;
        math.add_or_replace(make_ret_void_fn("add", vec![]));
        math.exports.push(IIRExport::new("add"));

        let mut app = IIRModule::new("app", "twig");
        app.entry_point = None;
        app.imports.push(IIRImport::new("math", "add", "any"));

        let errs = verify_imports(&app, &[&math]);
        assert!(errs.is_empty());
    }

    #[test]
    fn verify_imports_returns_error_when_not_satisfied() {
        let mut app = IIRModule::new("app", "twig");
        app.entry_point = None;
        app.imports.push(IIRImport::new("math", "missing", "any"));

        let errs = verify_imports(&app, &[]);
        assert_eq!(errs.len(), 1);
    }

    #[test]
    fn linker_default_is_same_as_new() {
        // Ensure Default impl is available.
        let l = IIRLinker::default();
        let mut m = IIRModule::new("t", "x");
        m.entry_point = None;
        let merged = l.link(&[m]).unwrap();
        assert!(merged.functions.is_empty());
    }
}
