//! Integration tests for `iir-codegen-adapters`.
//!
//! These tests exercise the crate's public API:
//! - `compile_iir()` — dispatch by backend name
//! - `build_iir_codegen_registry()` — registry construction and downcast
//! - `list_iir_backends()` — enumeration
//! - `IIRBackendArtifact` — variant matching and accessors
//! - `IIRAdapterError` — all error variants
//!
//! Each test is self-contained: it builds its own `IIRModule` fixture rather
//! than relying on shared state.  This makes failures easy to isolate.

use codegen_core::codegen::CodeGenerator;
use iir_codegen_adapters::{
    build_iir_codegen_registry, compile_iir, list_iir_backends, IIRAdapterError,
};
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};

// ── Fixtures ──────────────────────────────────────────────────────────────────

/// One function, `main() -> void`, a single `ret_void`.
fn void_module() -> IIRModule {
    let fn_ = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    );
    IIRModule {
        name: "void_test".into(),
        functions: vec![fn_],
        entry_point: Some("main".into()),
        language: "test".into(),
    }
}

/// `add(a: i32, b: i32) -> i32`
fn add_i32_module() -> IIRModule {
    let fn_ = IIRFunction::new(
        "add",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new(
                "add",
                Some("v0".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
        ],
    );
    IIRModule {
        name: "add_test".into(),
        functions: vec![fn_],
        entry_point: Some("add".into()),
        language: "test".into(),
    }
}

/// `mul(a: i64, b: i64) -> i64`
fn mul_i64_module() -> IIRModule {
    let fn_ = IIRFunction::new(
        "mul",
        vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
        "i64",
        vec![
            IIRInstr::new(
                "mul",
                Some("v0".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i64"),
        ],
    );
    IIRModule {
        name: "mul_test".into(),
        functions: vec![fn_],
        entry_point: Some("mul".into()),
        language: "test".into(),
    }
}

/// Module with two functions: add and sub.
fn two_function_module() -> IIRModule {
    let add = IIRFunction::new(
        "add",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new(
                "add", Some("v0".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
        ],
    );
    let sub = IIRFunction::new(
        "sub",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new(
                "sub", Some("v0".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
        ],
    );
    IIRModule {
        name: "two_fn".into(),
        functions: vec![add, sub],
        entry_point: Some("add".into()),
        language: "test".into(),
    }
}

/// A completely empty module — should fail validation on every backend.
fn empty_module() -> IIRModule {
    IIRModule {
        name: "empty".into(),
        functions: vec![],
        entry_point: None,
        language: "test".into(),
    }
}

/// A module with an unsupported opcode (`call_builtin`).
fn unsupported_op_module() -> IIRModule {
    let fn_ = IIRFunction::new(
        "bad",
        vec![],
        "void",
        vec![IIRInstr::new("call_builtin", None, vec![], "void")],
    );
    IIRModule {
        name: "bad".into(),
        functions: vec![fn_],
        entry_point: Some("bad".into()),
        language: "test".into(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: list_iir_backends
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn list_has_exactly_four_backends() {
    assert_eq!(list_iir_backends().len(), 4);
}

#[test]
fn list_contains_beam() {
    assert!(list_iir_backends().contains(&"iir-beam"));
}

#[test]
fn list_contains_wasm() {
    assert!(list_iir_backends().contains(&"iir-wasm"));
}

#[test]
fn list_contains_jvm() {
    assert!(list_iir_backends().contains(&"iir-jvm"));
}

#[test]
fn list_contains_clr() {
    assert!(list_iir_backends().contains(&"iir-clr"));
}

#[test]
fn list_is_sorted_alphabetically() {
    let b = list_iir_backends();
    let mut s = b.clone();
    s.sort();
    assert_eq!(b, s);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: compile_iir — successful compilation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn compile_void_to_beam() {
    let art = compile_iir(&void_module(), "iir-beam").unwrap();
    assert!(art.as_beam().is_some());
    assert_eq!(art.backend_name(), "iir-beam");
}

#[test]
fn compile_void_to_wasm() {
    let art = compile_iir(&void_module(), "iir-wasm").unwrap();
    assert!(art.as_wasm().is_some());
    assert_eq!(art.backend_name(), "iir-wasm");
}

#[test]
fn compile_void_to_jvm() {
    let art = compile_iir(&void_module(), "iir-jvm").unwrap();
    assert!(art.as_jvm().is_some());
    assert_eq!(art.backend_name(), "iir-jvm");
}

#[test]
fn compile_void_to_clr() {
    let art = compile_iir(&void_module(), "iir-clr").unwrap();
    assert!(art.as_clr().is_some());
    assert_eq!(art.backend_name(), "iir-clr");
}

#[test]
fn compile_add_i32_to_all_backends() {
    let m = add_i32_module();
    for backend in list_iir_backends() {
        compile_iir(&m, backend).unwrap_or_else(|e| {
            panic!("add_i32_module failed on backend {:?}: {}", backend, e)
        });
    }
}

#[test]
fn compile_mul_i64_to_all_backends() {
    let m = mul_i64_module();
    for backend in list_iir_backends() {
        compile_iir(&m, backend).unwrap_or_else(|e| {
            panic!("mul_i64_module failed on backend {:?}: {}", backend, e)
        });
    }
}

#[test]
fn compile_two_functions_to_all_backends() {
    let m = two_function_module();
    for backend in list_iir_backends() {
        compile_iir(&m, backend).unwrap_or_else(|e| {
            panic!("two_function_module failed on backend {:?}: {}", backend, e)
        });
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: compile_iir — error cases
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn unknown_backend_returns_error() {
    let e = compile_iir(&void_module(), "x86").unwrap_err();
    assert!(matches!(e, IIRAdapterError::UnknownBackend { .. }));
}

#[test]
fn unknown_backend_error_lists_available() {
    let e = compile_iir(&void_module(), "llvm").unwrap_err();
    match e {
        IIRAdapterError::UnknownBackend { requested, available } => {
            assert_eq!(requested, "llvm");
            assert_eq!(available.len(), 4);
        }
        other => panic!("unexpected: {:?}", other),
    }
}

#[test]
fn empty_backend_string_returns_error() {
    let e = compile_iir(&void_module(), "").unwrap_err();
    assert!(matches!(e, IIRAdapterError::UnknownBackend { .. }));
}

#[test]
fn empty_module_fails_all_backends() {
    let m = empty_module();
    for backend in list_iir_backends() {
        let result = compile_iir(&m, backend);
        assert!(
            matches!(result, Err(IIRAdapterError::ValidationFailed { .. })),
            "backend {:?} should return ValidationFailed for empty module",
            backend
        );
    }
}

#[test]
fn validation_error_contains_backend_name() {
    let e = compile_iir(&empty_module(), "iir-wasm").unwrap_err();
    match e {
        IIRAdapterError::ValidationFailed { backend, errors } => {
            assert_eq!(backend, "iir-wasm");
            assert!(!errors.is_empty());
        }
        other => panic!("unexpected: {:?}", other),
    }
}

#[test]
fn unsupported_op_fails_validation() {
    let m = unsupported_op_module();
    for backend in list_iir_backends() {
        let result = compile_iir(&m, backend);
        assert!(
            matches!(result, Err(IIRAdapterError::ValidationFailed { .. })),
            "backend {:?} should reject call_builtin op",
            backend
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: IIRBackendArtifact — accessor cross-checks
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn beam_artifact_wrong_accessors_return_none() {
    let art = compile_iir(&void_module(), "iir-beam").unwrap();
    assert!(art.as_beam().is_some());
    assert!(art.as_wasm().is_none());
    assert!(art.as_jvm().is_none());
    assert!(art.as_clr().is_none());
}

#[test]
fn wasm_artifact_wrong_accessors_return_none() {
    let art = compile_iir(&void_module(), "iir-wasm").unwrap();
    assert!(art.as_wasm().is_some());
    assert!(art.as_beam().is_none());
    assert!(art.as_jvm().is_none());
    assert!(art.as_clr().is_none());
}

#[test]
fn jvm_artifact_wrong_accessors_return_none() {
    let art = compile_iir(&void_module(), "iir-jvm").unwrap();
    assert!(art.as_jvm().is_some());
    assert!(art.as_beam().is_none());
    assert!(art.as_wasm().is_none());
    assert!(art.as_clr().is_none());
}

#[test]
fn clr_artifact_wrong_accessors_return_none() {
    let art = compile_iir(&void_module(), "iir-clr").unwrap();
    assert!(art.as_clr().is_some());
    assert!(art.as_beam().is_none());
    assert!(art.as_wasm().is_none());
    assert!(art.as_jvm().is_none());
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: IIRBackendArtifact — Display
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn artifact_display_contains_backend_label() {
    for (backend, expected_prefix) in [
        ("iir-beam", "Beam"),
        ("iir-wasm", "Wasm"),
        ("iir-jvm",  "Jvm"),
        ("iir-clr",  "Clr"),
    ] {
        let art = compile_iir(&void_module(), backend).unwrap();
        let s = art.to_string();
        assert!(
            s.starts_with(expected_prefix),
            "backend {:?}: Display {:?} should start with {:?}",
            backend, s, expected_prefix
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: build_iir_codegen_registry
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn registry_has_four_backends() {
    let reg = build_iir_codegen_registry();
    assert_eq!(reg.len(), 4);
}

#[test]
fn registry_names_match_list() {
    let reg = build_iir_codegen_registry();
    let reg_names = reg.names();
    let list_names: Vec<String> = list_iir_backends().iter().map(|s| s.to_string()).collect();
    assert_eq!(reg_names, list_names);
}

#[test]
fn registry_get_returns_some_for_each_backend() {
    let reg = build_iir_codegen_registry();
    for name in list_iir_backends() {
        assert!(reg.get(name).is_some(), "registry should have backend {:?}", name);
    }
}

#[test]
fn registry_get_returns_none_for_unknown() {
    let reg = build_iir_codegen_registry();
    assert!(reg.get("llvm").is_none());
    assert!(reg.get("").is_none());
}

#[test]
fn registry_downcast_beam() {
    use iir_to_beam::IIRBeamCodeGenerator;
    let reg = build_iir_codegen_registry();
    let any = reg.get("iir-beam").unwrap();
    let gen = any.downcast_ref::<IIRBeamCodeGenerator>().unwrap();
    assert_eq!(gen.name(), "iir-beam");
}

#[test]
fn registry_downcast_wasm() {
    use iir_to_wasm::IIRWasmCodeGenerator;
    let reg = build_iir_codegen_registry();
    let any = reg.get("iir-wasm").unwrap();
    let gen = any.downcast_ref::<IIRWasmCodeGenerator>().unwrap();
    assert_eq!(gen.name(), "iir-wasm");
}

#[test]
fn registry_downcast_jvm() {
    use iir_to_jvm_class_file::IIRJvmCodeGenerator;
    let reg = build_iir_codegen_registry();
    let any = reg.get("iir-jvm").unwrap();
    let gen = any.downcast_ref::<IIRJvmCodeGenerator>().unwrap();
    assert_eq!(gen.name(), "iir-jvm");
}

#[test]
fn registry_downcast_clr() {
    use iir_to_cil_bytecode::IIRClrCodeGenerator;
    let reg = build_iir_codegen_registry();
    let any = reg.get("iir-clr").unwrap();
    let gen = any.downcast_ref::<IIRClrCodeGenerator>().unwrap();
    assert_eq!(gen.name(), "iir-clr");
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: IIRAdapterError — Display
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn adapter_error_unknown_backend_display() {
    let e = compile_iir(&void_module(), "missing").unwrap_err();
    let s = e.to_string();
    assert!(s.contains("missing"), "should mention the requested name");
    assert!(s.contains("iir-beam"), "should list available backends");
}

#[test]
fn adapter_error_validation_failed_display() {
    let e = compile_iir(&empty_module(), "iir-jvm").unwrap_err();
    let s = e.to_string();
    assert!(s.contains("iir-jvm"));
    assert!(s.contains("EmptyModule"));
}
