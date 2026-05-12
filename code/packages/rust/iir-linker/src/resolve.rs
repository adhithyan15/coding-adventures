//! Import/export resolution — the first pass of the static linker.
//!
//! # What this module does
//!
//! Given a slice of `IIRModule`s, `build_export_map` constructs a lookup table
//! mapping `(module_name, public_name) → &IIRFunction`.  `resolve_imports` then
//! walks every import in every module and checks it against this table.
//!
//! # Export map key
//!
//! The key is `(module_name, public_function_name)`.  `module_name` is the
//! `IIRModule::name` field of the *exporting* module.  `public_function_name`
//! is `IIRExport::public_name()` — the alias if one was set, otherwise the
//! function name.
//!
//! # Type checking
//!
//! Type checking only fires when the import provides non-empty `param_types`
//! and/or a return type other than `"any"`.  The importer is checked against
//! the exporter's `IIRFunction::params` and `return_type`.
//!
//! This is intentionally lenient: a caller that doesn't know or doesn't care
//! about types (e.g., a dynamically typed language frontend) can still use the
//! linker.

use std::collections::HashMap;

use interpreter_ir::function::IIRFunction;
use interpreter_ir::module::IIRModule;
use interpreter_ir::module_exports::IIRImport;

use crate::error::LinkError;

/// A resolved export: which module owns it and a reference to the function.
pub struct ResolvedExport<'a> {
    pub exporting_module: &'a str,
    pub function: &'a IIRFunction,
}

/// Build an export map from a set of modules.
///
/// Returns `(map, errors)` where `errors` contains one `DuplicateExport` per
/// collision.
///
/// # Key
///
/// `(exporting_module_name, public_function_name)` — both strings as owned
/// `String`s to avoid lifetime headaches in the caller.
pub fn build_export_map<'a>(
    modules: &'a [IIRModule],
) -> (HashMap<(String, String), ResolvedExport<'a>>, Vec<LinkError>) {
    let mut map: HashMap<(String, String), ResolvedExport<'a>> = HashMap::new();
    let mut errors: Vec<LinkError> = Vec::new();

    for module in modules {
        // If the module has no explicit exports, it exports *nothing* —
        // functions are private by default (LANG33 design goal).
        // Backward-compat: an empty `exports` list means no exports at all.
        for export in &module.exports {
            let key = (module.name.clone(), export.public_name().to_string());
            if map.contains_key(&key) {
                errors.push(LinkError::DuplicateExport {
                    module_name: module.name.clone(),
                    function_name: export.public_name().to_string(),
                });
                // Keep the first definition in the map so we continue
                // resolving later imports and can report more errors.
                continue;
            }
            // The validator already checked that function_name exists, so
            // `unwrap_or_else` is a belt-and-suspenders fallback.
            if let Some(fn_) = module.get_function(&export.function_name) {
                map.insert(
                    key,
                    ResolvedExport {
                        exporting_module: &module.name,
                        function: fn_,
                    },
                );
            }
        }
    }

    (map, errors)
}

/// Resolve all imports in `modules` against the export map.
///
/// Returns a list of `LinkError`s (empty = all imports satisfied).
///
/// Each import is matched by `(import.module_name, import.function_name)`.
/// If found, and if the import carries type annotations, the types are checked
/// against the exported function's declared signature.
pub fn resolve_imports<'a>(
    modules: &'a [IIRModule],
    export_map: &HashMap<(String, String), ResolvedExport<'a>>,
) -> Vec<LinkError> {
    let mut errors = Vec::new();

    for module in modules {
        for import in &module.imports {
            check_import(module, import, export_map, &mut errors);
        }
    }

    errors
}

/// Verify that all imports in `module` are satisfied by `providers`.
///
/// This is the `verify_imports` entry point — it does not merge anything.
/// Used by the REPL to pre-flight an import before full linking.
pub fn verify_imports_against(
    module: &IIRModule,
    providers: &[&IIRModule],
) -> Vec<LinkError> {
    // Build a lightweight export map from just the provider modules.
    // We clone the provider slice into an owned Vec so we can call
    // `build_export_map` which takes `&[IIRModule]`.
    let owned: Vec<IIRModule> = providers.iter().map(|m| (*m).clone()).collect();
    let (map, mut errors) = build_export_map(&owned);
    for import in &module.imports {
        check_import(module, import, &map, &mut errors);
    }
    errors
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn check_import<'a>(
    importing_module: &IIRModule,
    import: &IIRImport,
    export_map: &HashMap<(String, String), ResolvedExport<'a>>,
    errors: &mut Vec<LinkError>,
) {
    let key = (import.module_name.clone(), import.function_name.clone());
    match export_map.get(&key) {
        None => {
            errors.push(LinkError::Unresolved {
                importing_module: importing_module.name.clone(),
                import_module: import.module_name.clone(),
                import_function: import.function_name.clone(),
            });
        }
        Some(resolved) => {
            // Type-check only when the import carries annotations.
            check_types(importing_module, import, resolved, errors);
        }
    }
}

fn check_types<'a>(
    importing_module: &IIRModule,
    import: &IIRImport,
    resolved: &ResolvedExport<'a>,
    errors: &mut Vec<LinkError>,
) {
    // Skip type checking if the import says "don't check".
    if import.param_types.is_empty() && import.return_type == "any" {
        return;
    }

    let fn_ = resolved.function;

    // Parameter type check: only if the import specifies param types.
    if !import.param_types.is_empty() {
        let actual: Vec<String> = fn_.params.iter().map(|(_, t)| t.clone()).collect();
        if import.param_types != actual {
            errors.push(LinkError::TypeMismatch {
                importing_module: importing_module.name.clone(),
                exporting_module: resolved.exporting_module.to_string(),
                function: import.function_name.clone(),
                expected: import.param_types.clone(),
                actual,
            });
        }
    }

    // Return type check: only if the import specifies a concrete return type.
    if import.return_type != "any" && import.return_type != fn_.return_type {
        // If we already emitted a TypeMismatch for params, the return type
        // mismatch is part of the same semantic error — but we still report it
        // separately so the user sees the full picture.
        errors.push(LinkError::TypeMismatch {
            importing_module: importing_module.name.clone(),
            exporting_module: resolved.exporting_module.to_string(),
            function: import.function_name.clone(),
            expected: vec![import.return_type.clone()],
            actual: vec![fn_.return_type.clone()],
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};
    use interpreter_ir::module_exports::{IIRExport, IIRImport};

    fn make_fn(name: &str, params: Vec<(&str, &str)>, ret: &str) -> IIRFunction {
        let params: Vec<(String, String)> = params
            .into_iter()
            .map(|(n, t)| (n.to_string(), t.to_string()))
            .collect();
        IIRFunction::new(
            name,
            params,
            ret,
            vec![IIRInstr::new("ret_void", None, vec![], "void")],
        )
    }

    fn make_math_module() -> IIRModule {
        let mut m = IIRModule::new("math", "twig");
        m.entry_point = None;
        m.add_or_replace(make_fn("add", vec![("a", "i64"), ("b", "i64")], "i64"));
        m.exports.push(IIRExport::new("add"));
        m
    }

    #[test]
    fn export_map_built_correctly() {
        let math = make_math_module();
        let modules = vec![math];
        let (map, errs) = build_export_map(&modules);
        assert!(errs.is_empty());
        let key = ("math".to_string(), "add".to_string());
        assert!(map.contains_key(&key));
    }

    #[test]
    fn export_map_alias_used_as_key() {
        let mut m = IIRModule::new("math", "twig");
        m.entry_point = None;
        m.add_or_replace(make_fn("internal_sqrt", vec![("x", "f64")], "f64"));
        m.exports.push(IIRExport::new("internal_sqrt").with_alias("sqrt"));
        let modules = vec![m];
        let (map, errs) = build_export_map(&modules);
        assert!(errs.is_empty());
        // Key is the *alias* ("sqrt"), not the internal name.
        assert!(map.contains_key(&("math".to_string(), "sqrt".to_string())));
        assert!(!map.contains_key(&("math".to_string(), "internal_sqrt".to_string())));
    }

    #[test]
    fn duplicate_export_produces_error() {
        let mut m1 = IIRModule::new("math", "twig");
        m1.entry_point = None;
        m1.add_or_replace(make_fn("add", vec![], "i64"));
        m1.exports.push(IIRExport::new("add"));

        let mut m2 = IIRModule::new("math", "twig"); // same module name!
        m2.entry_point = None;
        m2.add_or_replace(make_fn("add", vec![], "i64"));
        m2.exports.push(IIRExport::new("add"));

        let modules = vec![m1, m2];
        let (_, errs) = build_export_map(&modules);
        assert_eq!(errs.len(), 1);
        assert!(matches!(&errs[0], LinkError::DuplicateExport { function_name, .. } if function_name == "add"));
    }

    #[test]
    fn resolve_import_satisfied() {
        let math = make_math_module();
        let mut main = IIRModule::new("main", "twig");
        main.entry_point = None;
        main.imports.push(IIRImport::new("math", "add", "any"));
        let modules = vec![math, main];
        let (map, _) = build_export_map(&modules);
        let errs = resolve_imports(&modules, &map);
        assert!(errs.is_empty());
    }

    #[test]
    fn resolve_import_unresolved() {
        let mut main = IIRModule::new("main", "twig");
        main.entry_point = None;
        main.imports.push(IIRImport::new("math", "missing_fn", "any"));
        let modules = vec![main];
        let (map, _) = build_export_map(&modules);
        let errs = resolve_imports(&modules, &map);
        assert_eq!(errs.len(), 1);
        assert!(matches!(&errs[0], LinkError::Unresolved { import_function, .. } if import_function == "missing_fn"));
    }

    #[test]
    fn type_check_passes_when_matching() {
        let math = make_math_module();
        let mut main = IIRModule::new("main", "twig");
        main.entry_point = None;
        main.imports.push(
            IIRImport::new("math", "add", "i64")
                .with_params(vec!["i64".into(), "i64".into()]),
        );
        let modules = vec![math, main];
        let (map, _) = build_export_map(&modules);
        let errs = resolve_imports(&modules, &map);
        assert!(errs.is_empty(), "unexpected errors: {errs:?}");
    }

    #[test]
    fn type_check_fails_on_param_mismatch() {
        let math = make_math_module(); // "add" takes (i64, i64) → i64
        let mut main = IIRModule::new("main", "twig");
        main.entry_point = None;
        main.imports.push(
            IIRImport::new("math", "add", "i64")
                .with_params(vec!["f64".into(), "f64".into()]), // wrong!
        );
        let modules = vec![math, main];
        let (map, _) = build_export_map(&modules);
        let errs = resolve_imports(&modules, &map);
        assert_eq!(errs.len(), 1);
        assert!(matches!(&errs[0], LinkError::TypeMismatch { function, .. } if function == "add"));
    }

    #[test]
    fn type_check_skipped_when_import_uses_any() {
        let math = make_math_module();
        let mut main = IIRModule::new("main", "twig");
        main.entry_point = None;
        // "any" return type + empty params = skip type checking.
        main.imports.push(IIRImport::new("math", "add", "any"));
        let modules = vec![math, main];
        let (map, _) = build_export_map(&modules);
        let errs = resolve_imports(&modules, &map);
        assert!(errs.is_empty());
    }

    #[test]
    fn verify_imports_against_works() {
        let math = make_math_module();
        let mut main = IIRModule::new("main", "twig");
        main.entry_point = None;
        main.imports.push(IIRImport::new("math", "add", "any"));
        let errs = verify_imports_against(&main, &[&math]);
        assert!(errs.is_empty());
    }

    #[test]
    fn verify_imports_against_returns_unresolved() {
        let math = make_math_module();
        let mut main = IIRModule::new("main", "twig");
        main.entry_point = None;
        main.imports.push(IIRImport::new("math", "nonexistent", "any"));
        let errs = verify_imports_against(&main, &[&math]);
        assert_eq!(errs.len(), 1);
        assert!(matches!(&errs[0], LinkError::Unresolved { import_function, .. } if import_function == "nonexistent"));
    }
}
