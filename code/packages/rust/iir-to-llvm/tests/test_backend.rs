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

/// Unsupported op (e.g. `safepoint`, still outside the LLVM03 whitelist) is flagged.
#[test]
fn validate_rejects_unsupported_op() {
    let f = IIRFunction::new("main", vec![], "void",
        vec![
            IIRInstr::new("safepoint", None, vec![], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
    let errors = validate_for_llvm(&module_with(f));
    assert!(errors.iter().any(|e| e.contains("UnsupportedOp")),
        "expected UnsupportedOp for `safepoint`; got: {errors:?}");
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

// ===========================================================================
// 8. LLVM03 — arithmetic
// ===========================================================================
//
// IIR's `add`/`sub`/`mul`/`div`/`rem` lower to LLVM `add`/`sub`/`mul` for
// signedness-agnostic ops, and to `sdiv`/`udiv`/`srem`/`urem` for the
// signedness-sensitive ones.  Floats use `fadd`/`fsub`/`fmul`/`fdiv`/`frem`.
//
// Signedness comes from the IIR type_hint prefix: `i*` → signed, `u*` →
// unsigned.

#[test]
fn arith_add_i32_emits_add() {
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("add", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
        ]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("%v = add i32 %a, %b"),
        "expected `%v = add i32 %a, %b` in:\n{ll}");
}

#[test]
fn arith_add_f64_emits_fadd_double() {
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "f64".into()), ("b".into(), "f64".into())],
        "f64",
        vec![
            IIRInstr::new("add", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "f64"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "f64"),
        ]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("%v = fadd double %a, %b"),
        "expected `%v = fadd double %a, %b` in:\n{ll}");
}

#[test]
fn arith_div_signed_emits_sdiv() {
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("div", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
        ]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("%v = sdiv i32 %a, %b"),
        "expected sdiv for i32; got:\n{ll}");
}

#[test]
fn arith_div_unsigned_emits_udiv() {
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "u32".into()), ("b".into(), "u32".into())],
        "u32",
        vec![
            IIRInstr::new("div", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "u32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "u32"),
        ]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("%v = udiv i32 %a, %b"),
        "expected udiv for u32; got:\n{ll}");
}

#[test]
fn arith_rem_signed_emits_srem_unsigned_urem() {
    // Both functions in one module to confirm sign-discrimination.
    let signed = IIRFunction::new(
        "s",
        vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
        "i64",
        vec![
            IIRInstr::new("rem", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i64"),
        ]);
    let unsigned = IIRFunction::new(
        "u",
        vec![("a".into(), "u64".into()), ("b".into(), "u64".into())],
        "u64",
        vec![
            IIRInstr::new("rem", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "u64"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "u64"),
        ]);
    let module = IIRModule {
        name: "two".into(),
        functions: vec![signed, unsigned],
        entry_point: None,
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let ll = lower(&module);
    assert!(ll.contains("srem i64"), "expected srem for i64; got:\n{ll}");
    assert!(ll.contains("urem i64"), "expected urem for u64; got:\n{ll}");
}

#[test]
fn arith_inlines_const_operand() {
    // const c = 5 ; add v, a, c ; ret v
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("const", Some("c".into()), vec![Operand::Int(5)], "i32"),
            IIRInstr::new("add",   Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("c".into())], "i32"),
            IIRInstr::new("ret",   None, vec![Operand::Var("v".into())], "i32"),
        ]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("%v = add i32 %a, 5"),
        "const operand should inline literally; got:\n{ll}");
}

#[test]
fn bitwise_bool_ops_lower_as_i1_logic() {
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "bool".into()), ("b".into(), "bool".into())],
        "bool",
        vec![
            IIRInstr::new("and", Some("both".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "bool"),
            IIRInstr::new("or", Some("either".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "bool"),
            IIRInstr::new("xor", Some("v".into()),
                vec![Operand::Var("both".into()), Operand::Var("either".into())], "bool"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "bool"),
        ]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("%both = and i1 %a, %b"),
        "expected i1 and; got:\n{ll}");
    assert!(ll.contains("%either = or i1 %a, %b"),
        "expected i1 or; got:\n{ll}");
    assert!(ll.contains("%v = xor i1 %both, %either"),
        "expected i1 xor; got:\n{ll}");
}

// ===========================================================================
// 9. LLVM03 — comparison
// ===========================================================================
//
// Cmps emit `icmp <pred>` (or `fcmp <pred>`) and produce an i1; if the IIR
// type_hint is wider than i1 we zext to that width.  Signedness predicates:
//
// | IIR op | i32 | u32 | f64 |
// |--------|-----|-----|-----|
// | eq     | eq  | eq  | oeq |
// | ne     | ne  | ne  | one |
// | lt     | slt | ult | olt |
// | gt     | sgt | ugt | ogt |

#[test]
fn cmp_eq_i32_emits_icmp_eq_and_zext_i32() {
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("eq", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
        ]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("%v.i1 = icmp eq i32 %a, %b"),
        "expected icmp eq i32; got:\n{ll}");
    assert!(ll.contains("%v = zext i1 %v.i1 to i32"),
        "expected zext to i32; got:\n{ll}");
}

#[test]
fn cmp_lt_unsigned_emits_ult() {
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "u32".into()), ("b".into(), "u32".into())],
        "i32",
        vec![
            IIRInstr::new("lt", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "u32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
        ]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("icmp ult i32"),
        "expected ult for u32 operand; got:\n{ll}");
}

#[test]
fn cmp_lt_float_emits_fcmp_olt() {
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "f64".into()), ("b".into(), "f64".into())],
        "i32",
        vec![
            IIRInstr::new("lt", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "f64"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
        ]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("fcmp olt double"),
        "expected fcmp olt double for f64; got:\n{ll}");
}

#[test]
fn cmp_prefixed_aliases_accepted() {
    // G1 compat: `cmp_eq` should lower the same as `eq`.
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("cmp_eq", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
        ]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("icmp eq i32 %a, %b"),
        "`cmp_eq` should lower like `eq`; got:\n{ll}");
}

#[test]
fn cmp_to_i1_avoids_zext() {
    // When the cmp's type_hint is "i1", we should NOT emit a zext.
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i1",
        vec![
            IIRInstr::new("eq", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i1"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i1"),
        ]);
    let ll = lower(&module_with(f));
    // Note: this emits `icmp eq i1 ...` because type_hint drives the
    // operand type as well; whether that matches IIR semantics is a higher-
    // level question.  What matters here: NO zext was emitted.
    assert!(!ll.contains("zext"),
        "type_hint=i1 should skip the zext step; got:\n{ll}");
}

// ===========================================================================
// 10. LLVM03 — control flow
// ===========================================================================

#[test]
fn label_emits_basic_block_header() {
    let f = IIRFunction::new(
        "f",
        vec![],
        "void",
        vec![
            IIRInstr::new("jmp", None, vec![Operand::Var("L1".into())], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("L1".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("\nL1:\n"), "expected `L1:` block header; got:\n{ll}");
}

#[test]
fn jmp_emits_unconditional_br() {
    let f = IIRFunction::new(
        "f",
        vec![],
        "void",
        vec![
            IIRInstr::new("jmp", None, vec![Operand::Var("done".into())], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("done".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("br label %done"),
        "expected `br label %done`; got:\n{ll}");
}

#[test]
fn jmp_if_true_emits_br_with_fallthrough_block() {
    // Function: compare two i32s, branch to taken label if equal.
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "void",
        vec![
            IIRInstr::new("eq", Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
            IIRInstr::new("jmp_if_true", None,
                vec![Operand::Var("c".into()), Operand::Var("taken".into())], "i32"),
            IIRInstr::new("ret_void", None, vec![], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("taken".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
    let ll = lower(&module_with(f));
    // The conditional br uses the i1 form (no trunc round-trip), and the
    // false arm is a synthesized fallthrough block.
    assert!(ll.contains("br i1 %c.i1, label %taken, label %__fall"),
        "expected `br i1 %c.i1, label %taken, label %__fall…` (with i1 form, no trunc); got:\n{ll}");
    assert!(ll.contains("\n__fall"),
        "expected a synthesized `__fall…:` fallthrough block; got:\n{ll}");
}

#[test]
fn jmp_if_false_swaps_arms() {
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "void",
        vec![
            IIRInstr::new("eq", Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
            IIRInstr::new("jmp_if_false", None,
                vec![Operand::Var("c".into()), Operand::Var("not_taken".into())], "i32"),
            IIRInstr::new("ret_void", None, vec![], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("not_taken".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
    let ll = lower(&module_with(f));
    // jmp_if_false: TRUE arm is the synthesized fallthrough, FALSE arm is `not_taken`.
    let needle = "br i1 %c.i1, label %__fall";
    assert!(ll.contains(needle),
        "expected jmp_if_false to put fallthrough in the TRUE arm; got:\n{ll}");
    assert!(ll.contains(", label %not_taken"),
        "expected `not_taken` in the FALSE arm; got:\n{ll}");
}

// ===========================================================================
// 11. LLVM04 — call / call_builtin print_i64
// ===========================================================================
//
// `call` lowers user-defined function calls.  Per-arg LLVM types come from
// the callee's signature, which `lower_iir_to_llvm` pre-scans into a side
// map.  `call_builtin "print_i64"` lowers to `call void @__print_i64(i64 …)`
// + a module-top `declare void @__print_i64(i64)`.

#[test]
fn call_user_fn_non_void_emits_typed_call() {
    // square(x) { ret x*x }
    // f() -> i32 { v = const 5 : i32; r = call square(v) : i32; ret r }
    let square = IIRFunction::new(
        "square",
        vec![("x".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("mul", Some("r".into()),
                vec![Operand::Var("x".into()), Operand::Var("x".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ]);
    let f = IIRFunction::new(
        "f",
        vec![],
        "i32",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(5)], "i32"),
            IIRInstr::new("call",  Some("r".into()),
                vec![Operand::Var("square".into()), Operand::Var("v".into())], "i32"),
            IIRInstr::new("ret",   None, vec![Operand::Var("r".into())], "i32"),
        ]);
    let module = IIRModule {
        name: "test".into(),
        functions: vec![square, f],
        entry_point: Some("f".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let ll = lower(&module);
    assert!(ll.contains("%r = call i32 @square(i32 5)"),
        "expected `%r = call i32 @square(i32 5)`; got:\n{ll}");
}

#[test]
fn call_void_return_omits_lhs() {
    // sink(x) { ret_void }
    // f() { v = const 7; call sink(v); ret_void }
    let sink = IIRFunction::new(
        "sink",
        vec![("x".into(), "i32".into())],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    let f = IIRFunction::new(
        "f",
        vec![],
        "void",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "i32"),
            IIRInstr::new("call",  None,
                vec![Operand::Var("sink".into()), Operand::Var("v".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
    let module = IIRModule {
        name: "test".into(),
        functions: vec![sink, f],
        entry_point: Some("f".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let ll = lower(&module);
    assert!(ll.contains("  call void @sink(i32 7)"),
        "expected `  call void @sink(i32 7)`; got:\n{ll}");
    // Must NOT contain `%??? = call void` — LLVM rejects void on the LHS.
    assert!(!ll.contains("= call void"),
        "void call must not appear on LHS of `=`; got:\n{ll}");
}

#[test]
fn call_unknown_callee_is_error() {
    let f = IIRFunction::new(
        "f",
        vec![],
        "i32",
        vec![
            IIRInstr::new("call", Some("r".into()),
                vec![Operand::Var("ghost".into())], "i32"),
            IIRInstr::new("ret",  None, vec![Operand::Var("r".into())], "i32"),
        ]);
    let err = lower_iir_to_llvm(&module_with(f), &IIRLlvmConfig::default())
        .expect_err("unknown callee should fail lowering");
    match err {
        IIRLlvmError::UndefinedVariable { name, .. } => {
            assert!(name.contains("ghost"),
                "expected error mentioning \"ghost\"; got: {name}");
        }
        other => panic!("expected UndefinedVariable, got: {other:?}"),
    }
}

#[test]
fn call_arg_count_mismatch_is_error() {
    let callee = IIRFunction::new(
        "g",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![IIRInstr::new("ret", None, vec![Operand::Var("a".into())], "i32")]);
    let f = IIRFunction::new(
        "f",
        vec![],
        "i32",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(1)], "i32"),
            IIRInstr::new("call",  Some("r".into()),
                vec![Operand::Var("g".into()), Operand::Var("v".into())], "i32"), // only 1 arg, g wants 2
            IIRInstr::new("ret",   None, vec![Operand::Var("r".into())], "i32"),
        ]);
    let module = IIRModule {
        name: "test".into(),
        functions: vec![callee, f],
        entry_point: Some("f".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let err = lower_iir_to_llvm(&module, &IIRLlvmConfig::default())
        .expect_err("arg-count mismatch should fail");
    match err {
        IIRLlvmError::InvalidOperand { detail, .. } => {
            assert!(detail.contains("arg-count"), "expected arg-count error; got: {detail}");
        }
        other => panic!("expected InvalidOperand, got: {other:?}"),
    }
}

#[test]
fn call_builtin_print_i64_emits_extern_call_and_declare() {
    // f() { v = const 42 : i64; call_builtin print_i64(v); ret_void }
    let f = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "i64"),
            IIRInstr::new("call_builtin", None,
                vec![Operand::Var("print_i64".into()), Operand::Var("v".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("declare void @__print_i64(i64)"),
        "expected extern declare for @__print_i64; got:\n{ll}");
    assert!(ll.contains("call void @__print_i64(i64 42)"),
        "expected call site for @__print_i64; got:\n{ll}");
}

#[test]
fn declare_for_print_i64_is_emitted_exactly_once_per_module() {
    // Two functions both use print_i64 → only ONE declare line.
    let f1 = IIRFunction::new(
        "a",
        vec![],
        "void",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("call_builtin", None,
                vec![Operand::Var("print_i64".into()), Operand::Var("v".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
    let f2 = IIRFunction::new(
        "b",
        vec![],
        "void",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(2)], "i64"),
            IIRInstr::new("call_builtin", None,
                vec![Operand::Var("print_i64".into()), Operand::Var("v".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
    let module = IIRModule {
        name: "two".into(),
        functions: vec![f1, f2],
        entry_point: Some("a".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let ll = lower(&module);
    assert_eq!(
        ll.matches("declare void @__print_i64(i64)").count(),
        1,
        "expected exactly one `declare` for @__print_i64; got:\n{ll}"
    );
}

#[test]
fn declare_omitted_when_print_i64_unused() {
    let f = IIRFunction::new(
        "main", vec![], "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    let ll = lower(&module_with(f));
    assert!(!ll.contains("@__print_i64"),
        "no print_i64 use → no extern; got:\n{ll}");
}

#[test]
fn call_builtin_unknown_name_is_unsupported_op() {
    let f = IIRFunction::new(
        "main", vec![], "void",
        vec![
            IIRInstr::new("call_builtin", None,
                vec![Operand::Var("definitely_unknown".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
    let err = lower_iir_to_llvm(&module_with(f), &IIRLlvmConfig::default())
        .expect_err("unknown builtin should fail lowering");
    match err {
        IIRLlvmError::UnsupportedOp { op, .. } => {
            assert!(op.contains("definitely_unknown"),
                "expected error to mention the unknown builtin; got: {op}");
        }
        other => panic!("expected UnsupportedOp, got: {other:?}"),
    }
}

// ===========================================================================
// 12. Error display (kept green from LLVM02)
// ===========================================================================

#[test]
fn errors_display_without_panic() {
    let _ = format!("{}", IIRLlvmError::ValidationFailed(vec!["x".into()]));
    let _ = format!("{}", IIRLlvmError::UnsupportedOp { function: "f".into(), op: "weird".into() });
    let _ = format!("{}", IIRLlvmError::UnsupportedType { function: "f".into(), type_hint: "weird".into() });
    let _ = format!("{}", IIRLlvmError::InvalidOperand { function: "f".into(), detail: "bad".into() });
    let _ = format!("{}", IIRLlvmError::UndefinedVariable { function: "f".into(), name: "nope".into() });
}

/// McCarthy W12b: a tagged-word lisp builtin (`call_builtin "lispy_cons"`) lowers
/// to a `call i64 @__twig_lispy_cons(i64, i64)` and emits exactly one matching
/// `declare`. A lisp heap reference type (`ref<LispyPair>`) is accepted (carried
/// as a tagged `i64`); a non-lisp `ref<Foo>` is still rejected (see
/// `validate_rejects_unsupported_type`).
#[test]
fn lispy_cons_lowers_to_runtime_call() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(56)], "i64"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(72)], "i64"),
            IIRInstr::new(
                "call_builtin",
                Some("p".into()),
                vec![
                    Operand::Var("lispy_cons".into()),
                    Operand::Var("a".into()),
                    Operand::Var("b".into()),
                ],
                "ref<LispyPair>",
            ),
            IIRInstr::new(
                "call_builtin",
                Some("h".into()),
                vec![Operand::Var("lispy_car".into()), Operand::Var("p".into())],
                "any",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("h".into())], "any"),
        ],
    );
    let ll = lower(&module_with(f));
    assert!(ll.contains("declare i64 @__twig_lispy_cons(i64, i64)"), "cons declare; got:\n{ll}");
    assert!(ll.contains("declare i64 @__twig_lispy_car(i64)"), "car declare; got:\n{ll}");
    assert!(ll.contains("= call i64 @__twig_lispy_cons(i64 "), "cons call site; got:\n{ll}");
    assert!(ll.contains("= call i64 @__twig_lispy_car(i64 "), "car call site; got:\n{ll}");
    // Exactly one declare per used builtin (no duplicates from two call sites).
    assert_eq!(ll.matches("declare i64 @__twig_lispy_cons").count(), 1, "one cons declare");
}

/// An unknown `lispy_*`-shaped builtin that is NOT in `LISPY_BUILTINS` is rejected.
#[test]
fn unknown_builtin_still_rejected() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new(
                "call_builtin",
                Some("x".into()),
                vec![Operand::Var("lispy_bogus".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i64"),
        ],
    );
    let err = lower_iir_to_llvm(&module_with(f), &IIRLlvmConfig::default())
        .expect_err("unknown builtin must be rejected");
    assert!(matches!(err, IIRLlvmError::UnsupportedOp { .. }), "got: {err:?}");
}
