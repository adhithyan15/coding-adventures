//! Integration tests for `iir-linker`.
//!
//! These tests exercise the full `link` / `link_strict` / `verify_imports`
//! APIs from the outside, using `interpreter_ir` types exactly as a language
//! frontend or tool would.
//!
//! The tests are organised into sections:
//!
//! - Basic linking (happy path)
//! - Error cases (Unresolved, TypeMismatch, DuplicateExport)
//! - Name collision renaming
//! - Call instruction rewriting
//! - `link_strict` behaviour
//! - `verify_imports` pre-flight
//! - Edge cases (empty modules, no entry point, etc.)

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use interpreter_ir::module_exports::{IIRExport, IIRImport};
use iir_linker::{link, link_strict, verify_imports, LinkError};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_fn(name: &str, params: Vec<(&str, &str)>, ret: &str) -> IIRFunction {
    IIRFunction::new(
        name,
        params.into_iter().map(|(n, t)| (n.into(), t.into())).collect(),
        ret,
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    )
}

fn make_fn_calling(name: &str, callee: &str) -> IIRFunction {
    IIRFunction::new(
        name,
        vec![],
        "void",
        vec![
            IIRInstr::new(
                "call",
                Some("_r".into()),
                vec![Operand::Var(callee.into())],
                "void",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    )
}

/// Build a "math" module that exports `add(i64, i64) → i64`.
fn math_module() -> IIRModule {
    let mut m = IIRModule::new("math", "twig");
    m.entry_point = None;
    m.add_or_replace(make_fn("add", vec![("a", "i64"), ("b", "i64")], "i64"));
    m.exports.push(IIRExport::new("add"));
    m
}

/// Build an "app" module whose "main" calls imported "add" from "math".
fn app_module_importing_add() -> IIRModule {
    let mut m = IIRModule::new("app", "twig");
    m.add_or_replace(make_fn_calling("main", "add"));
    m.imports.push(IIRImport::new("math", "add", "any"));
    m
}

// ---------------------------------------------------------------------------
// Basic linking — happy path
// ---------------------------------------------------------------------------

#[test]
fn link_two_modules_succeeds() {
    let merged = link(&[math_module(), app_module_importing_add()]).unwrap();
    assert!(merged.get_function("add").is_some());
    assert!(merged.get_function("main").is_some());
}

#[test]
fn merged_module_has_no_imports() {
    let merged = link(&[math_module(), app_module_importing_add()]).unwrap();
    assert!(merged.imports.is_empty());
}

#[test]
fn merged_module_has_no_exports() {
    let merged = link(&[math_module(), app_module_importing_add()]).unwrap();
    assert!(merged.exports.is_empty());
}

#[test]
fn entry_point_preserved_from_first_module_with_one() {
    let math = math_module(); // entry_point = None
    let mut app = app_module_importing_add();
    app.entry_point = Some("main".into());
    let merged = link(&[math, app]).unwrap();
    assert_eq!(merged.entry_point, Some("main".into()));
}

#[test]
fn link_single_module_no_imports_succeeds() {
    let mut m = IIRModule::new("prog", "twig");
    m.add_or_replace(make_fn("main", vec![], "void"));
    let merged = link(&[m]).unwrap();
    assert_eq!(merged.functions.len(), 1);
}

#[test]
fn link_three_modules_all_merged() {
    let mut io = IIRModule::new("io", "twig");
    io.entry_point = None;
    io.add_or_replace(make_fn("print", vec![("s", "str")], "void"));
    io.exports.push(IIRExport::new("print"));

    let math = math_module();

    let mut app = IIRModule::new("app", "twig");
    app.add_or_replace(make_fn("main", vec![], "void"));
    app.imports.push(IIRImport::new("math", "add", "any"));
    app.imports.push(IIRImport::new("io", "print", "any"));

    let merged = link(&[io, math, app]).unwrap();
    assert!(merged.get_function("print").is_some());
    assert!(merged.get_function("add").is_some());
    assert!(merged.get_function("main").is_some());
}

#[test]
fn exported_alias_resolved_by_importer() {
    // Module "math" exports "internal_sqrt" under the alias "sqrt".
    let mut m = IIRModule::new("math", "twig");
    m.entry_point = None;
    m.add_or_replace(make_fn("internal_sqrt", vec![("x", "f64")], "f64"));
    m.exports.push(IIRExport::new("internal_sqrt").with_alias("sqrt"));

    // App imports "sqrt" (the alias, not "internal_sqrt").
    let mut app = IIRModule::new("app", "twig");
    app.add_or_replace(make_fn("main", vec![], "void"));
    app.imports.push(IIRImport::new("math", "sqrt", "any"));

    let merged = link(&[m, app]).unwrap();
    // The merged function keeps the exported alias name "sqrt".
    assert!(merged.get_function("sqrt").is_some());
}

#[test]
fn local_alias_used_in_call_instructions() {
    // App imports "add" from "math" under the local alias "math_add".
    let mut app = IIRModule::new("app", "twig");
    app.add_or_replace(make_fn_calling("main", "math_add")); // calls "math_add"
    app.imports.push(
        IIRImport::new("math", "add", "any").with_local_alias("math_add"),
    );

    let merged = link(&[math_module(), app]).unwrap();
    // After linking, "math_add" in the call instruction should be rewritten to "add".
    let main_fn = merged.get_function("main").unwrap();
    if let Some(call_instr) = main_fn.instructions.iter().find(|i| i.op == "call") {
        if let Some(Operand::Var(callee)) = call_instr.srcs.first() {
            assert_eq!(callee, "add", "call should target 'add' after rewrite");
        }
    }
}

// ---------------------------------------------------------------------------
// Error cases
// ---------------------------------------------------------------------------

#[test]
fn unresolved_import_returns_error() {
    let mut app = IIRModule::new("app", "twig");
    app.entry_point = None;
    app.imports.push(IIRImport::new("nonexistent_mod", "fn_a", "any"));
    let errs = link(&[app]).unwrap_err();
    assert!(errs.iter().any(|e| matches!(e,
        LinkError::Unresolved { import_module, import_function, .. }
        if import_module == "nonexistent_mod" && import_function == "fn_a"
    )));
}

#[test]
fn multiple_unresolved_imports_all_reported() {
    let mut app = IIRModule::new("app", "twig");
    app.entry_point = None;
    app.imports.push(IIRImport::new("x", "a", "any"));
    app.imports.push(IIRImport::new("x", "b", "any"));
    app.imports.push(IIRImport::new("y", "c", "any"));
    let errs = link(&[app]).unwrap_err();
    assert_eq!(errs.len(), 3);
    assert!(errs.iter().all(|e| matches!(e, LinkError::Unresolved { .. })));
}

#[test]
fn type_mismatch_on_param_types() {
    let math = math_module(); // add(i64, i64) → i64

    let mut app = IIRModule::new("app", "twig");
    app.entry_point = None;
    // Caller expects (f64, f64) but "add" takes (i64, i64).
    app.imports.push(
        IIRImport::new("math", "add", "i64")
            .with_params(vec!["f64".into(), "f64".into()]),
    );

    let errs = link(&[math, app]).unwrap_err();
    assert!(errs.iter().any(|e| matches!(e, LinkError::TypeMismatch { .. })));
}

#[test]
fn type_mismatch_on_return_type() {
    let math = math_module(); // add returns "i64"

    let mut app = IIRModule::new("app", "twig");
    app.entry_point = None;
    // Caller expects return type "f64", exporter returns "i64".
    app.imports.push(IIRImport::new("math", "add", "f64")); // wrong return type

    let errs = link(&[math, app]).unwrap_err();
    assert!(errs.iter().any(|e| matches!(e, LinkError::TypeMismatch { .. })));
}

#[test]
fn type_check_skipped_for_any_return_type() {
    let math = math_module(); // add returns "i64"

    let mut app = IIRModule::new("app", "twig");
    app.entry_point = None;
    // "any" return = don't check.
    app.imports.push(IIRImport::new("math", "add", "any"));

    let result = link(&[math, app]);
    assert!(result.is_ok(), "should not type-check when return type is 'any'");
}

#[test]
fn duplicate_export_detected() {
    let mut m1 = IIRModule::new("math", "twig");
    m1.entry_point = None;
    m1.add_or_replace(make_fn("sqrt", vec![("x", "f64")], "f64"));
    m1.exports.push(IIRExport::new("sqrt"));

    let mut m2 = IIRModule::new("math", "twig"); // Same module name!
    m2.entry_point = None;
    m2.add_or_replace(make_fn("sqrt", vec![("x", "f64")], "f64"));
    m2.exports.push(IIRExport::new("sqrt"));

    let errs = link(&[m1, m2]).unwrap_err();
    assert!(errs.iter().any(|e| matches!(e, LinkError::DuplicateExport { function_name, .. } if function_name == "sqrt")));
}

// ---------------------------------------------------------------------------
// Name collision renaming
// ---------------------------------------------------------------------------

#[test]
fn private_functions_with_same_name_get_renamed() {
    let mut m1 = IIRModule::new("a", "twig");
    m1.entry_point = None;
    m1.add_or_replace(make_fn("helper", vec![], "void"));

    let mut m2 = IIRModule::new("b", "twig");
    m2.entry_point = None;
    m2.add_or_replace(make_fn("helper", vec![], "void"));

    let merged = link(&[m1, m2]).unwrap();
    let names: Vec<&str> = merged.functions.iter().map(|f| f.name.as_str()).collect();
    // First module's "helper" keeps the name; second gets "b::helper".
    assert!(names.contains(&"helper"));
    assert!(names.contains(&"b::helper"));
}

#[test]
fn exported_function_name_takes_priority_in_collision() {
    // Module "lib" exports "format".
    // Module "app" also has a private "format" — it should be renamed.
    let mut lib = IIRModule::new("lib", "twig");
    lib.entry_point = None;
    lib.add_or_replace(make_fn("format", vec![], "str"));
    lib.exports.push(IIRExport::new("format"));

    let mut app = IIRModule::new("app", "twig");
    app.add_or_replace(make_fn("format", vec![], "str")); // private — should be renamed
    app.add_or_replace(make_fn("main", vec![], "void"));
    app.imports.push(IIRImport::new("lib", "format", "any"));

    let merged = link(&[lib, app]).unwrap();
    // "format" exists (from the exported function).
    assert!(merged.get_function("format").is_some());
    // The private "format" from "app" was renamed to "app::format".
    assert!(merged.get_function("app::format").is_some());
}

// ---------------------------------------------------------------------------
// link_strict
// ---------------------------------------------------------------------------

#[test]
fn link_strict_succeeds_on_valid_input() {
    let result = link_strict(&[math_module(), app_module_importing_add()]);
    assert!(result.is_ok());
}

#[test]
fn link_strict_returns_first_error_on_failure() {
    let mut app = IIRModule::new("app", "twig");
    app.entry_point = None;
    app.imports.push(IIRImport::new("x", "a", "any"));
    app.imports.push(IIRImport::new("x", "b", "any"));

    let err = link_strict(&[app]).unwrap_err();
    assert!(matches!(err, LinkError::Unresolved { .. }));
}

// ---------------------------------------------------------------------------
// verify_imports
// ---------------------------------------------------------------------------

#[test]
fn verify_imports_empty_when_all_satisfied() {
    let math = math_module();
    let mut app = IIRModule::new("app", "twig");
    app.entry_point = None;
    app.imports.push(IIRImport::new("math", "add", "any"));

    let errs = verify_imports(&app, &[&math]);
    assert!(errs.is_empty());
}

#[test]
fn verify_imports_reports_unresolved() {
    let mut app = IIRModule::new("app", "twig");
    app.entry_point = None;
    app.imports.push(IIRImport::new("math", "sqrt", "any"));

    let errs = verify_imports(&app, &[]);
    assert_eq!(errs.len(), 1);
    assert!(matches!(&errs[0], LinkError::Unresolved { import_function, .. } if import_function == "sqrt"));
}

#[test]
fn verify_imports_does_not_modify_modules() {
    let math = math_module();
    let mut app = IIRModule::new("app", "twig");
    app.entry_point = None;
    app.imports.push(IIRImport::new("math", "add", "any"));

    let original_import_count = app.imports.len();
    let _ = verify_imports(&app, &[&math]);
    // verify_imports is pure — it does not alter the module.
    assert_eq!(app.imports.len(), original_import_count);
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn link_empty_slice_returns_empty_module() {
    let merged = link(&[]).unwrap();
    assert!(merged.functions.is_empty());
    assert!(merged.exports.is_empty());
    assert!(merged.imports.is_empty());
}

#[test]
fn link_module_with_no_entry_point_succeeds() {
    let mut m = IIRModule::new("lib", "twig");
    m.entry_point = None;
    m.add_or_replace(make_fn("helper", vec![], "void"));
    m.exports.push(IIRExport::new("helper"));

    let merged = link(&[m]).unwrap();
    assert_eq!(merged.entry_point, None);
}

#[test]
fn link_multiple_modules_all_with_none_entry_point() {
    let mut m1 = IIRModule::new("a", "twig");
    m1.entry_point = None;
    m1.add_or_replace(make_fn("fa", vec![], "void"));

    let mut m2 = IIRModule::new("b", "twig");
    m2.entry_point = None;
    m2.add_or_replace(make_fn("fb", vec![], "void"));

    let merged = link(&[m1, m2]).unwrap();
    assert_eq!(merged.entry_point, None);
}

#[test]
fn imported_function_with_type_annotations_passes_when_matching() {
    let math = math_module(); // add(i64, i64) → i64

    let mut app = IIRModule::new("app", "twig");
    app.entry_point = None;
    app.imports.push(
        IIRImport::new("math", "add", "i64")
            .with_params(vec!["i64".into(), "i64".into()]),
    );

    let result = link(&[math, app]);
    assert!(result.is_ok(), "type-annotated import should succeed: {result:?}");
}

#[test]
fn empty_param_types_with_concrete_return_type_only_checks_return() {
    let math = math_module(); // add returns "i64"

    let mut app = IIRModule::new("app", "twig");
    app.entry_point = None;
    // Empty param_types = don't check params.  Return type "i64" matches.
    app.imports.push(IIRImport::new("math", "add", "i64"));

    let result = link(&[math, app]);
    assert!(result.is_ok(), "return type check should pass: {result:?}");
}

#[test]
fn link_error_display_is_non_empty() {
    let e = LinkError::Unresolved {
        importing_module: "main".into(),
        import_module: "math".into(),
        import_function: "log".into(),
    };
    let s = format!("{e}");
    assert!(!s.is_empty());
    assert!(s.contains("log"));
}

#[test]
fn link_result_module_name_from_first_input() {
    let math = math_module(); // name = "math"
    let app = app_module_importing_add(); // name = "app"
    let merged = link(&[math, app]).unwrap();
    // Name comes from the first module.
    assert_eq!(merged.name, "math");
}

#[test]
fn merged_module_validates_cleanly() {
    let merged = link(&[math_module(), app_module_importing_add()]).unwrap();
    let errors = merged.validate();
    assert!(errors.is_empty(), "merged module should validate cleanly: {errors:?}");
}

#[test]
fn link_iir_linker_struct_api_matches_free_fn() {
    // The IIRLinker struct API must produce the same result as the free function.
    use iir_linker::IIRLinker;
    let linker = IIRLinker::new();
    let via_struct = linker
        .link(&[math_module(), app_module_importing_add()])
        .unwrap();
    let via_free = link(&[math_module(), app_module_importing_add()]).unwrap();

    // Both should have the same function names.
    let names_struct: std::collections::BTreeSet<&str> =
        via_struct.functions.iter().map(|f| f.name.as_str()).collect();
    let names_free: std::collections::BTreeSet<&str> =
        via_free.functions.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names_struct, names_free);
}
