//! Integration tests for `iir-to-llvm`.
//!
//! Test groups (grow with each release):
//!
//! 1. Validator behaviour (LLVM01 + LLVM02 rules)
//! 2. Header output shape (LLVM01)
//! 3. Function signatures (LLVM02)
//! 4. ret_void / ret lowering (LLVM02)
//! 5. const / mov lowering (LLVM02)
//! 6. Config defaults
//! 7. Error display

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_llvm::{lower_iir_to_llvm, validate_for_llvm, IIRLlvmConfig, IIRLlvmError};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn empty_module() -> IIRModule {
    IIRModule {
        name: "demo".into(),
        functions: vec![],
        entry_point: None,
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    }
}

fn module_with(func: IIRFunction) -> IIRModule {
    let entry = func.name.clone();
    IIRModule {
        name: "test".into(),
        functions: vec![func],
        entry_point: Some(entry),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    }
}

fn lower(module: &IIRModule) -> String {
    lower_iir_to_llvm(module, &IIRLlvmConfig::default()).expect("lowering should succeed")
}

// ===========================================================================
// 1. Validator behaviour
// ===========================================================================

#[test]
fn validate_returns_empty_for_empty_module() {
    assert!(validate_for_llvm(&empty_module()).is_empty());
}

/// A function with only supported ops/types validates clean.
#[test]
fn validate_accepts_supported_ret_void_function() {
    let f = IIRFunction::new("main", vec![], "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    assert!(validate_for_llvm(&module_with(f)).is_empty());
}

/// Unsupported op (e.g. `add`, not in v0.2.0 whitelist) is flagged.
#[test]
fn validate_rejects_unsupported_op() {
    let f = IIRFunction::new("main", vec![], "i32",
        vec![
            IIRInstr::new("add", Some("v".into()),
                vec![Operand::Int(1), Operand::Int(2)], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
        ]);
    let errors = validate_for_llvm(&module_with(f));
    assert!(errors.iter().any(|e| e.contains("UnsupportedOp")),
        "expected UnsupportedOp for `add`; got: {errors:?}");
}

/// Unsupported type (e.g. `ref<X>`) is flagged.
#[test]
fn validate_rejects_unsupported_type() {
    let f = IIRFunction::new("main", vec![], "ref<Foo>",
        vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    let errors = validate_for_llvm(&module_with(f));
    assert!(errors.iter().any(|e| e.contains("UnsupportedType")),
        "expected UnsupportedType for ret-type ref<Foo>; got: {errors:?}");
}

/// Unsupported param type is flagged.
#[test]
fn validate_rejects_unsupported_param_type() {
    let f = IIRFunction::new(
        "f",
        vec![("p".into(), "str".into())],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    );
    let errors = validate_for_llvm(&module_with(f));
    assert!(errors.iter().any(|e| e.contains("UnsupportedType") && e.contains("\"str\"")),
        "expected UnsupportedType for param `str`; got: {errors:?}");
}

// ===========================================================================
// 2. Header output shape (LLVM01 — kept green)
// ===========================================================================

#[test]
fn output_contains_module_id_comment() {
    let cfg = IIRLlvmConfig::new("hello_module");
    let ll = lower_iir_to_llvm(&empty_module(), &cfg).expect("lower");
    assert!(ll.contains("; ModuleID = 'hello_module'"));
}

#[test]
fn output_contains_target_triple() {
    let cfg = IIRLlvmConfig::default().with_target("riscv32-unknown-elf");
    let ll = lower_iir_to_llvm(&empty_module(), &cfg).expect("lower");
    assert!(ll.contains("target triple = \"riscv32-unknown-elf\""));
}

/// LLVM01 acceptance criterion: first non-blank line starts with `;` or `target`.
#[test]
fn output_starts_with_comment_or_target() {
    let ll = lower(&empty_module());
    let first = ll.lines().map(str::trim).find(|l| !l.is_empty()).unwrap();
    assert!(first.starts_with(';') || first.starts_with("target"),
        "first non-blank line: {first:?}");
}

// ===========================================================================
// 3. Function signatures (LLVM02)
// ===========================================================================

/// Void function with no params: `define void @main() {`.
#[test]
fn signature_void_no_params() {
    let f = IIRFunction::new("main", vec![], "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("define void @main()"),
        "expected `define void @main()` in:\n{ll}");
}

/// Signed-int return + two i32 params: param types use LLVM's signless `i32`.
#[test]
fn signature_i32_with_two_params() {
    let f = IIRFunction::new(
        "add",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![IIRInstr::new("ret", None, vec![Operand::Var("a".into())], "i32")],
    );
    let ll = lower(&module_with(f));
    assert!(ll.contains("define i32 @add(i32 %a, i32 %b)"),
        "expected `define i32 @add(i32 %a, i32 %b)` in:\n{ll}");
}

/// f64 maps to LLVM `double`, f32 to `float`.
#[test]
fn signature_float_types_map_correctly() {
    let f = IIRFunction::new(
        "f",
        vec![("x".into(), "f32".into()), ("y".into(), "f64".into())],
        "f64",
        vec![IIRInstr::new("ret", None, vec![Operand::Var("y".into())], "f64")],
    );
    let ll = lower(&module_with(f));
    assert!(ll.contains("define double @f(float %x, double %y)"),
        "expected float→`float`, f64→`double` in:\n{ll}");
}

/// u32 and i32 both map to LLVM `i32` (signless).
#[test]
fn signature_u32_and_i32_both_map_to_llvm_i32() {
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "u32".into()), ("b".into(), "i32".into())],
        "u32",
        vec![IIRInstr::new("ret", None, vec![Operand::Var("a".into())], "u32")],
    );
    let ll = lower(&module_with(f));
    assert!(ll.contains("define i32 @f(i32 %a, i32 %b)"),
        "expected both u32 and i32 → LLVM i32 in:\n{ll}");
}

// ===========================================================================
// 4. ret_void / ret lowering
// ===========================================================================

#[test]
fn ret_void_emits_ret_void_line() {
    let f = IIRFunction::new("main", vec![], "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("  ret void"),
        "expected `  ret void` in:\n{ll}");
}

#[test]
fn ret_with_const_inlines_literal() {
    let f = IIRFunction::new("answer", vec![], "i64",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "i64"),
            IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "i64"),
        ]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("ret i64 42"),
        "expected `ret i64 42` (const inlined); got:\n{ll}");
    // And we must NOT have emitted a useless SSA assignment for the const.
    assert!(!ll.contains("= add i64 0, 42"),
        "const should not have produced an `add 0, x` no-op; got:\n{ll}");
}

#[test]
fn ret_with_param_uses_param_register() {
    let f = IIRFunction::new(
        "identity",
        vec![("x".into(), "i32".into())],
        "i32",
        vec![IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i32")],
    );
    let ll = lower(&module_with(f));
    assert!(ll.contains("ret i32 %x"),
        "expected `ret i32 %x`; got:\n{ll}");
}

#[test]
fn ret_with_undefined_var_is_error() {
    let f = IIRFunction::new("oops", vec![], "i32",
        vec![IIRInstr::new("ret", None,
            vec![Operand::Var("nonexistent".into())], "i32")]);
    let err = lower_iir_to_llvm(&module_with(f), &IIRLlvmConfig::default())
        .expect_err("undefined var should fail lowering");
    match err {
        IIRLlvmError::UndefinedVariable { name, .. } => assert_eq!(name, "nonexistent"),
        other => panic!("expected UndefinedVariable, got: {other:?}"),
    }
}

// ===========================================================================
// 5. const / mov lowering
// ===========================================================================

#[test]
fn const_does_not_emit_an_llvm_line() {
    let f = IIRFunction::new("only_const", vec![], "i64",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "i64"),
        ]);
    let ll = lower(&module_with(f));
    // The whole function body should be exactly one `ret` line.
    let body: Vec<&str> = ll.lines()
        .skip_while(|l| !l.starts_with("define"))
        .skip(1)  // skip the `define` line
        .take_while(|l| !l.starts_with('}'))
        .filter(|l| !l.trim().is_empty())
        .collect();
    assert_eq!(body.len(), 1,
        "expected exactly 1 body line (the ret); got {body:?} in:\n{ll}");
    assert!(body[0].trim().starts_with("ret "),
        "expected the single body line to be `ret …`; got {:?}", body[0]);
}

#[test]
fn mov_chains_through_constants() {
    // const v = 7 ; mov w v ; mov x w ; ret x
    let f = IIRFunction::new("chain", vec![], "i32",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "i32"),
            IIRInstr::new("mov",   Some("w".into()), vec![Operand::Var("v".into())], "i32"),
            IIRInstr::new("mov",   Some("x".into()), vec![Operand::Var("w".into())], "i32"),
            IIRInstr::new("ret",   None,             vec![Operand::Var("x".into())], "i32"),
        ]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("ret i32 7"),
        "mov chain should resolve `x` back to literal 7; got:\n{ll}");
}

#[test]
fn mov_aliases_a_param() {
    // fn f(p: i32) -> i32 { mov q p ; ret q }
    let f = IIRFunction::new(
        "f",
        vec![("p".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("mov", Some("q".into()), vec![Operand::Var("p".into())], "i32"),
            IIRInstr::new("ret", None,             vec![Operand::Var("q".into())], "i32"),
        ],
    );
    let ll = lower(&module_with(f));
    assert!(ll.contains("ret i32 %p"),
        "mov of a param should resolve to `%p`; got:\n{ll}");
}

// ===========================================================================
// 6. Config defaults
// ===========================================================================

#[test]
fn default_config_has_nonempty_triple() {
    let cfg = IIRLlvmConfig::default();
    assert!(!cfg.target_triple.is_empty());
    assert!(!cfg.module_name.is_empty());
}

#[test]
fn new_sets_module_name_keeps_default_triple() {
    let cfg = IIRLlvmConfig::new("custom");
    assert_eq!(cfg.module_name, "custom");
    assert_eq!(cfg.target_triple, IIRLlvmConfig::default().target_triple);
}

// ===========================================================================
// 7. Error display
// ===========================================================================

#[test]
fn errors_display_without_panic() {
    let _ = format!("{}", IIRLlvmError::ValidationFailed(vec!["x".into()]));
    let _ = format!("{}", IIRLlvmError::UnsupportedOp { function: "f".into(), op: "weird".into() });
    let _ = format!("{}", IIRLlvmError::UnsupportedType { function: "f".into(), type_hint: "weird".into() });
    let _ = format!("{}", IIRLlvmError::InvalidOperand { function: "f".into(), detail: "bad".into() });
    let _ = format!("{}", IIRLlvmError::UndefinedVariable { function: "f".into(), name: "nope".into() });
}
