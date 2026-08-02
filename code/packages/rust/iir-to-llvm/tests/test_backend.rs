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
    // A non-Lispy `ref<…>` has no LLVM value model and is still rejected.
    let f = IIRFunction::new(
        "f",
        vec![("p".into(), "ref<Foo>".into())],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    );
    let errors = validate_for_llvm(&module_with(f));
    assert!(errors.iter().any(|e| e.contains("UnsupportedType") && e.contains("ref<Foo>")),
        "expected UnsupportedType for param `ref<Foo>`; got: {errors:?}");
}

/// E4-dyn (E4d-2b): a `str` parameter / return type is now accepted — a string
/// is carried as an i64 handle across function boundaries, so an ALGOL
/// `string procedure` (which returns a runtime string) lowers cleanly.
#[test]
fn validate_accepts_str_param_and_return() {
    let f = IIRFunction::new(
        "f",
        vec![("p".into(), "str".into())],
        "str",
        vec![IIRInstr::new("ret", None, vec![Operand::Var("p".into())], "str")],
    );
    let errors = validate_for_llvm(&module_with(f));
    assert!(errors.is_empty(), "str param/return should be valid; got: {errors:?}");
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
    // A `u32` divide. Every IIR value flows through an i64 slot in this backend
    // (frontends widen params/returns to i64 and carry the narrow width only on
    // the operation's type_hint — exactly this shape), so the unsigned divide
    // computes at i64 (`udiv`, NOT `sdiv`) and then masks the result to 32 bits
    // (E2 register width & wrap). Operand width stays i64; only the value wraps.
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
        "i64",
        vec![
            IIRInstr::new("div", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "u32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i64"),
        ]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("udiv i64 %a, %b"),
        "expected unsigned udiv at i64 width; got:\n{ll}");
    assert!(ll.contains(", 4294967295"),
        "expected the u32 wrap mask (0xFFFFFFFF); got:\n{ll}");
}

// ── E2 — register width & wrap (narrow unsigned arithmetic) ──────────────────

/// Build `f(a: i64, b: i64) -> i64 { v = <op>(a, b) : <hint>; ret v }` — the
/// shape every frontend produces for a narrow-typed binary op (i64 slots, the
/// narrow width carried only on the op's type_hint).
fn narrow_binop_fn(op: &str, hint: &str) -> IIRFunction {
    IIRFunction::new(
        "f",
        vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
        "i64",
        vec![
            IIRInstr::new(op, Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], hint),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i64"),
        ],
    )
}

/// Bitwise NOT — LLVM has no `not` instruction, so it is `xor x, -1`. For a
/// narrow unsigned width the E2 mask brings it into range (`~0u8 = 255`).
/// Unlocks Nib N3-`~` / Oct O2-`~`. (Verified end-to-end on real `clang`:
/// `not 0 : u8` returns exit `255`.)
#[test]
fn not_u8_is_xor_minus1_then_masked() {
    let f = IIRFunction::new("f", vec![("a".into(), "i64".into())], "i64", vec![
        IIRInstr::new("not", Some("v".into()), vec![Operand::Var("a".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i64"),
    ]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("xor i64 %a, -1"), "u8 not is `xor i64 a, -1`; got:\n{ll}");
    assert!(ll.contains(", 255"), "u8 not masks with 0xFF (so ~0u8 = 255); got:\n{ll}");
}

#[test]
fn not_i64_is_plain_xor_no_mask() {
    let f = IIRFunction::new("f", vec![("a".into(), "i64".into())], "i64", vec![
        IIRInstr::new("not", Some("v".into()), vec![Operand::Var("a".into())], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i64"),
    ]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("xor i64 %a, -1"), "i64 not is `xor i64 a, -1`; got:\n{ll}");
    assert!(!ll.contains(", 255") && !ll.contains(", 4294967295"),
        "full-width i64 not gets no mask; got:\n{ll}");
}

#[test]
fn e2_u8_add_computes_at_i64_then_masks() {
    // `200u8 + 100u8 = 44`: add at i64 (operands are i64 slots), then `and` to
    // 8 bits. Typing the add `i8` over i64 SSA operands would be invalid IR —
    // this is why the wrap is a mask, not a narrow-typed op.
    let ll = lower(&module_with(narrow_binop_fn("add", "u8")));
    assert!(ll.contains("add i64 %a, %b"), "u8 add computes at i64; got:\n{ll}");
    assert!(ll.contains(", 255"), "u8 add masks with 0xFF; got:\n{ll}");
}

#[test]
fn e2_u16_and_u4_masks_match_width() {
    let ll16 = lower(&module_with(narrow_binop_fn("mul", "u16")));
    assert!(ll16.contains("mul i64 %a, %b") && ll16.contains(", 65535"),
        "u16 mul → mul i64 + mask 0xFFFF; got:\n{ll16}");
    let ll4 = lower(&module_with(narrow_binop_fn("sub", "u4")));
    assert!(ll4.contains("sub i64 %a, %b") && ll4.contains(", 15"),
        "u4 sub → sub i64 + mask 0xF; got:\n{ll4}");
}

#[test]
fn e2_bitwise_u8_xor_masks() {
    let ll = lower(&module_with(narrow_binop_fn("xor", "u8")));
    assert!(ll.contains("xor i64 %a, %b") && ll.contains(", 255"),
        "u8 xor → xor i64 + mask 0xFF; got:\n{ll}");
}

#[test]
fn e2_wide_widths_emit_no_mask() {
    // i64/u64 are full-word; signed narrow (i8/i16/i32) are out of E2 scope —
    // none of them gets a width mask, so the op is a plain single instruction.
    for hint in ["i64", "u64"] {
        let ll = lower(&module_with(narrow_binop_fn("add", hint)));
        assert!(ll.contains("add i64 %a, %b"), "{hint} add at i64; got:\n{ll}");
        assert!(!ll.contains(", 255") && !ll.contains(", 4294967295"),
            "{hint} add must NOT mask; got:\n{ll}");
    }
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
fn boolean_call_result_stays_i1_for_logical_ops_and_branches() {
    let id = IIRFunction::new(
        "id",
        vec![("p".into(), "bool".into())],
        "bool",
        vec![IIRInstr::new("ret", None, vec![Operand::Var("p".into())], "bool")],
    );
    let main = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("f".into()), vec![Operand::Bool(false)], "bool"),
            IIRInstr::new(
                "call",
                Some("called".into()),
                vec![Operand::Var("id".into()), Operand::Var("f".into())],
                "bool",
            ),
            IIRInstr::new("not", Some("inverted".into()), vec![Operand::Var("called".into())], "bool"),
            IIRInstr::new(
                "jmp_if_false",
                None,
                vec![Operand::Var("inverted".into()), Operand::Var("no".into())],
                "bool",
            ),
            IIRInstr::new("ret", None, vec![Operand::Int(42)], "i64"),
            IIRInstr::new("label", None, vec![Operand::Var("no".into())], "void"),
            IIRInstr::new("ret", None, vec![Operand::Int(0)], "i64"),
        ],
    );
    let module = IIRModule {
        name: "bool_call".into(),
        functions: vec![id, main],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };

    let ll = lower(&module);
    assert!(ll.contains("%called = call i1 @id(i1 0)"), "expected bool call; got:\n{ll}");
    assert!(ll.contains("%inverted = xor i1 %called, -1"), "expected i1 not; got:\n{ll}");
    assert!(ll.contains("br i1 %inverted"), "expected i1 branch; got:\n{ll}");
    assert!(!ll.contains("trunc i64 %called to i1"), "must not widen and truncate a bool call; got:\n{ll}");
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
// LANG-FULL E4 — string literal output foothold
// ===========================================================================

#[test]
fn e4_string_literal_print_emits_headered_constant_and_runtime_call() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![
            IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("HELLO".into())], "str"),
            IIRInstr::new("print_str", None, vec![Operand::Var("s".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let module = module_with(f);
    assert!(validate_for_llvm(&module).is_empty(), "literal string print should validate");
    let ll = lower(&module);

    assert!(
        ll.contains("declare void @__print_str(ptr, i64)"),
        "missing __print_str declaration:\n{ll}"
    );
    assert!(
        ll.contains("@__twig_str_0 = private unnamed_addr constant { i64, [5 x i8] } { i64 5, [5 x i8] c\"\\48\\45\\4C\\4C\\4F\" }, align 8"),
        "missing length-prefixed string literal:\n{ll}"
    );
    assert!(
        ll.contains("getelementptr inbounds i8, ptr @__twig_str_0, i64 8"),
        "print_str should pass the payload pointer, not the header:\n{ll}"
    );
    assert!(
        ll.contains("call void @__print_str(ptr %__str1, i64 5)"),
        "missing print_str runtime call:\n{ll}"
    );
}

#[test]
fn e4_string_literal_len_folds_to_integer_return() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("HELLO".into())], "str"),
            IIRInstr::new("str_len", Some("n".into()), vec![Operand::Var("s".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "i64"),
        ],
    );
    let module = module_with(f);
    assert!(validate_for_llvm(&module).is_empty(), "literal string len should validate");
    let ll = lower(&module);

    assert!(
        ll.contains("ret i64 5"),
        "str_len over a literal should materialise the byte count:\n{ll}"
    );
    assert!(
        !ll.contains("@__print_str"),
        "str_len alone should not pull in the string print runtime:\n{ll}"
    );
}

#[test]
fn e4_string_literal_eq_folds_to_integer_return() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("str_const", Some("a".into()), vec![Operand::Str("HELLO".into())], "str"),
            IIRInstr::new("str_const", Some("b".into()), vec![Operand::Str("HELLO".into())], "str"),
            IIRInstr::new("str_eq", Some("ok".into()), vec![
                Operand::Var("a".into()),
                Operand::Var("b".into()),
            ], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("ok".into())], "i64"),
        ],
    );
    let module = module_with(f);
    assert!(validate_for_llvm(&module).is_empty(), "literal string eq should validate");
    let ll = lower(&module);

    assert!(
        ll.contains("ret i64 1"),
        "str_eq over equal literals should materialise true as 1:\n{ll}"
    );
    assert!(
        !ll.contains("@__print_str"),
        "str_eq alone should not pull in the string print runtime:\n{ll}"
    );
}

#[test]
fn e4_string_literal_concat_len_folds_to_integer_return() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("str_const", Some("a".into()), vec![Operand::Str("AB".into())], "str"),
            IIRInstr::new("str_const", Some("b".into()), vec![Operand::Str("CDE".into())], "str"),
            IIRInstr::new("str_concat", Some("s".into()), vec![
                Operand::Var("a".into()),
                Operand::Var("b".into()),
            ], "str"),
            IIRInstr::new("str_len", Some("n".into()), vec![Operand::Var("s".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "i64"),
        ],
    );
    let module = module_with(f);
    assert!(validate_for_llvm(&module).is_empty(), "literal string concat len should validate");
    let ll = lower(&module);

    assert!(
        ll.contains("ret i64 5"),
        "str_len over a literal concat should materialise the byte count:\n{ll}"
    );
    assert!(
        !ll.contains("@__print_str"),
        "str_concat + str_len alone should not pull in the string print runtime:\n{ll}"
    );
}

#[test]
fn e4_string_literal_concat_print_emits_derived_constant_and_runtime_call() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![
            IIRInstr::new("str_const", Some("a".into()), vec![Operand::Str("O".into())], "str"),
            IIRInstr::new("str_const", Some("b".into()), vec![Operand::Str("K".into())], "str"),
            IIRInstr::new("str_concat", Some("s".into()), vec![
                Operand::Var("a".into()),
                Operand::Var("b".into()),
            ], "str"),
            IIRInstr::new("print_str", None, vec![Operand::Var("s".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let module = module_with(f);
    assert!(validate_for_llvm(&module).is_empty(), "literal string concat print should validate");
    let ll = lower(&module);

    assert!(
        ll.contains("@__twig_str_2 = private unnamed_addr constant { i64, [2 x i8] } { i64 2, [2 x i8] c\"\\4F\\4B\" }, align 8"),
        "concat should materialise a derived length-prefixed string constant:\n{ll}"
    );
    assert!(
        ll.contains("getelementptr inbounds i8, ptr @__twig_str_2, i64 8"),
        "print_str should use the derived concat storage:\n{ll}"
    );
    assert!(
        ll.contains("call void @__print_str(ptr %__str1, i64 2)"),
        "concat print should call the string runtime with the derived byte length:\n{ll}"
    );
}

#[test]
fn e4_string_literal_slice_index_folds_to_integer_return() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new(
                "str_const",
                Some("s".into()),
                vec![Operand::Str("ABCDE".into())],
                "str",
            ),
            IIRInstr::new("const", Some("start".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("const", Some("end".into()), vec![Operand::Int(4)], "i64"),
            IIRInstr::new(
                "str_slice",
                Some("sub".into()),
                vec![
                    Operand::Var("s".into()),
                    Operand::Var("start".into()),
                    Operand::Var("end".into()),
                ],
                "str",
            ),
            IIRInstr::new("const", Some("i".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new(
                "str_index",
                Some("b".into()),
                vec![Operand::Var("sub".into()), Operand::Var("i".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i64"),
        ],
    );
    let module = module_with(f);
    assert!(
        validate_for_llvm(&module).is_empty(),
        "literal string slice/index should validate"
    );
    let ll = lower(&module);

    assert!(
        ll.contains("@__twig_str_1 = private unnamed_addr constant { i64, [3 x i8] } { i64 3, [3 x i8] c\"\\42\\43\\44\" }, align 8"),
        "str_slice should materialise BCD metadata:\n{ll}"
    );
    assert!(
        ll.contains("ret i64 67"),
        "str_slice feeding str_index should materialise byte 67:\n{ll}"
    );
}

#[test]
fn e4_string_literal_index_folds_to_integer_return() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("ABC".into())], "str"),
            IIRInstr::new("const", Some("i".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("str_index", Some("b".into()), vec![
                Operand::Var("s".into()),
                Operand::Var("i".into()),
            ], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i64"),
        ],
    );
    let module = module_with(f);
    assert!(validate_for_llvm(&module).is_empty(), "literal string index should validate");
    let ll = lower(&module);

    assert!(
        ll.contains("ret i64 66"),
        "str_index over a literal should materialise byte 66:\n{ll}"
    );
    assert!(
        !ll.contains("@__print_str"),
        "str_index alone should not pull in the string print runtime:\n{ll}"
    );
}

#[test]
fn e4_string_literal_index_accepts_computed_len_index() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("ABCDE".into())], "str"),
            IIRInstr::new("str_len", Some("n".into()), vec![Operand::Var("s".into())], "i64"),
            IIRInstr::new("const", Some("one".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("sub", Some("i".into()), vec![
                Operand::Var("n".into()),
                Operand::Var("one".into()),
            ], "i64"),
            IIRInstr::new("str_index", Some("b".into()), vec![
                Operand::Var("s".into()),
                Operand::Var("i".into()),
            ], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i64"),
        ],
    );
    let module = module_with(f);
    assert!(validate_for_llvm(&module).is_empty(), "computed string index should validate");
    let ll = lower(&module);

    assert!(
        ll.contains("ret i64 69"),
        "str_len + typed sub should compute byte index 4 for E in ABCDE:\n{ll}"
    );
    assert!(
        !ll.contains("@__print_str"),
        "computed string index alone should not pull in the string print runtime:\n{ll}"
    );
}

#[test]
fn e4_string_literal_index_out_of_bounds_traps() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("ABC".into())], "str"),
            IIRInstr::new("const", Some("i".into()), vec![Operand::Int(3)], "i64"),
            IIRInstr::new("str_index", Some("b".into()), vec![
                Operand::Var("s".into()),
                Operand::Var("i".into()),
            ], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i64"),
        ],
    );
    let module = module_with(f);
    assert!(
        validate_for_llvm(&module).is_empty(),
        "literal OOB string index should validate and trap at runtime"
    );
    let ll = lower(&module);

    assert!(
        ll.contains("declare void @llvm.trap()"),
        "str_index OOB should declare llvm.trap:\n{ll}"
    );
    assert!(
        ll.contains("call void @llvm.trap()"),
        "str_index OOB should emit a runtime trap:\n{ll}"
    );
}

#[test]
fn e4_string_literal_cmp_folds_to_integer_return() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("str_const", Some("a".into()), vec![Operand::Str("ALPHA".into())], "str"),
            IIRInstr::new("str_const", Some("b".into()), vec![Operand::Str("BETA".into())], "str"),
            IIRInstr::new("str_cmp", Some("ord".into()), vec![
                Operand::Var("a".into()),
                Operand::Var("b".into()),
            ], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("ord".into())], "i64"),
        ],
    );
    let module = module_with(f);
    assert!(validate_for_llvm(&module).is_empty(), "literal string cmp should validate");
    let ll = lower(&module);

    assert!(
        ll.contains("ret i64 -1"),
        "str_cmp over ordered literals should materialise -1:\n{ll}"
    );
    assert!(
        !ll.contains("@__print_str"),
        "str_cmp alone should not pull in the string print runtime:\n{ll}"
    );
}

#[test]
fn e4_str_const_rejects_non_ascii_literal() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![
            IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("hé".into())], "str"),
            IIRInstr::new("print_str", None, vec![Operand::Var("s".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let errors = validate_for_llvm(&module_with(f));
    assert!(
        errors.iter().any(|e| e.contains("printable ASCII")),
        "non-ASCII literal should be rejected in this foothold; got {errors:?}"
    );
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

/// McCarthy W12b: a tagged-word lisp builtin (`call_builtin "dyn_cons"`) lowers
/// to a `call i64 @__dyn_cons(i64, i64)` and emits exactly one matching
/// `declare`. A lisp heap reference type (`ref<LispyPair>`) is accepted (carried
/// as a tagged `i64`); a non-lisp `ref<Foo>` is still rejected (see
/// `validate_rejects_unsupported_type`).
#[test]
fn dyn_cons_lowers_to_runtime_call() {
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
                    Operand::Var("dyn_cons".into()),
                    Operand::Var("a".into()),
                    Operand::Var("b".into()),
                ],
                "ref<LispyPair>",
            ),
            IIRInstr::new(
                "call_builtin",
                Some("h".into()),
                vec![Operand::Var("dyn_car".into()), Operand::Var("p".into())],
                "any",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("h".into())], "any"),
        ],
    );
    let ll = lower(&module_with(f));
    assert!(ll.contains("declare i64 @__dyn_cons(i64, i64)"), "cons declare; got:\n{ll}");
    assert!(ll.contains("declare i64 @__dyn_car(i64)"), "car declare; got:\n{ll}");
    assert!(ll.contains("= call i64 @__dyn_cons(i64 "), "cons call site; got:\n{ll}");
    assert!(ll.contains("= call i64 @__dyn_car(i64 "), "car call site; got:\n{ll}");
    // Exactly one declare per used builtin (no duplicates from two call sites).
    assert_eq!(ll.matches("declare i64 @__dyn_cons").count(), 1, "one cons declare");
}

/// An unknown `dyn_*`-shaped builtin that is NOT in `DYN_BUILTINS` is rejected.
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
                vec![Operand::Var("dyn_bogus".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i64"),
        ],
    );
    let err = lower_iir_to_llvm(&module_with(f), &IIRLlvmConfig::default())
        .expect_err("unknown builtin must be rejected");
    assert!(matches!(err, IIRLlvmError::UnsupportedOp { .. }), "got: {err:?}");
}

/// McCarthy W12b-3: a variable assigned in 2+ instructions (here `m`, written in
/// two blocks like a `COND` result) is promoted to a stack slot — an entry
/// `alloca`, a `store` per assignment, and a `load` per read. A clause block with
/// no emitted instructions still gets an explicit fallthrough `br` (no two labels
/// back-to-back), and `ret` reads the merged value through a `load`.
#[test]
fn multi_assigned_var_is_promoted_to_alloca() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            // if cond goto L_else
            IIRInstr::new("const", Some("c".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("jmp_if_false", None,
                vec![Operand::Var("c".into()), Operand::Var("L_else".into())], "void"),
            IIRInstr::new("const", Some("m".into()), vec![Operand::Int(11)], "i64"),
            IIRInstr::new("jmp", None, vec![Operand::Var("L_end".into())], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("L_else".into())], "void"),
            IIRInstr::new("const", Some("m".into()), vec![Operand::Int(22)], "i64"),
            IIRInstr::new("label", None, vec![Operand::Var("L_end".into())], "void"),
            IIRInstr::new("ret", None, vec![Operand::Var("m".into())], "i64"),
        ],
    );
    let ll = lower(&module_with(f));
    assert!(ll.contains("%m.slot = alloca i64"), "entry alloca for the merge var; got:\n{ll}");
    assert!(ll.contains("store i64 11, ptr %m.slot"), "store of the then-value; got:\n{ll}");
    assert!(ll.contains("store i64 22, ptr %m.slot"), "store of the else-value; got:\n{ll}");
    assert!(ll.contains("= load i64, ptr %m.slot"), "load before ret; got:\n{ll}");
    // The `c == 0` clause test (i64 truthy) compares against zero, not `trunc void`.
    assert!(ll.contains("icmp ne i64"), "void-typed jmp_if compares against zero; got:\n{ll}");
    assert!(!ll.contains("trunc void"), "must never emit `trunc void`; got:\n{ll}");
}

/// A purely straight-line function (no var assigned twice) takes the fast path:
/// no `alloca`/`store`/`load` is emitted — the `const`/`mov` side-map still wins.
#[test]
fn single_assignment_stays_on_the_side_map() {
    let f = IIRFunction::new(
        "answer",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i64"),
        ],
    );
    let ll = lower(&module_with(f));
    assert!(!ll.contains("alloca"), "no slot for a single-assignment var; got:\n{ll}");
    assert!(ll.contains("ret i64 42"), "const folds straight into ret; got:\n{ll}");
}

/// McCarthy W13 (F6): a `symbol`-typed value (an interned tagged immediate) maps
/// to an `i64` and validates — it is a tagged 64-bit word like `any`/`ref<Lispy…>`.
#[test]
fn symbol_typed_const_validates_and_lowers() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("s".into()), vec![Operand::Int(2)], "symbol"),
            IIRInstr::new("ret", None, vec![Operand::Var("s".into())], "i64"),
        ],
    );
    assert!(validate_for_llvm(&module_with(f.clone())).is_empty(), "symbol const must validate");
    let ll = lower(&module_with(f));
    assert!(ll.contains("ret i64 2"), "symbol immediate flows as i64; got:\n{ll}");
}

// ===========================================================================
// 8. LLVM05 — byte-tape memory + Brainfuck I/O (LANG-MATRIX LM-L Brainfuck)
// ===========================================================================

/// `alloc_bytes dest <- size` lowers to a zero-filling `@calloc` and declares it.
#[test]
fn alloc_bytes_emits_calloc_and_declare() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(30_000)], "i64"),
            IIRInstr::new("alloc_bytes", Some("t".into()), vec![Operand::Var("n".into())], "i64"),
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("z".into())], "i64"),
        ],
    );
    let ll = lower(&module_with(f));
    assert!(ll.contains("declare ptr @calloc(i64, i64)"), "calloc declared once; got:\n{ll}");
    assert!(
        ll.contains("%t = call ptr @calloc(i64 30000, i64 1)"),
        "alloc_bytes → zero-filled calloc; got:\n{ll}"
    );
}

/// `load_byte`/`store_byte` index the tape at byte width: zero-extend on load,
/// truncate on store — the "byte width only at the tape boundary" contract.
#[test]
fn load_and_store_byte_zext_and_trunc_at_boundary() {
    // A single-assignment `base` + `idx` keep the snippet slot-free so the
    // gep/zext/trunc shapes are easy to eyeball.
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(8)], "i64"),
            IIRInstr::new("alloc_bytes", Some("base".into()), vec![Operand::Var("n".into())], "i64"),
            IIRInstr::new("const", Some("idx".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("const", Some("val".into()), vec![Operand::Int(65)], "i64"),
            IIRInstr::new(
                "store_byte",
                None,
                vec![Operand::Var("base".into()), Operand::Var("idx".into()), Operand::Var("val".into())],
                "i64",
            ),
            IIRInstr::new(
                "load_byte",
                Some("got".into()),
                vec![Operand::Var("base".into()), Operand::Var("idx".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("got".into())], "i64"),
        ],
    );
    let ll = lower(&module_with(f));
    assert!(ll.contains("getelementptr i8, ptr %base, i64 0"), "byte-indexed gep; got:\n{ll}");
    assert!(ll.contains("trunc i64 65 to i8"), "store truncates to the cell; got:\n{ll}");
    assert!(ll.contains("store i8"), "store writes one byte; got:\n{ll}");
    assert!(ll.contains("load i8, ptr"), "load reads one byte; got:\n{ll}");
    assert!(ll.contains("zext i8"), "load zero-extends to i64; got:\n{ll}");
}

/// `store_byte` with a `dest` is rejected — it produces no value.
#[test]
fn store_byte_with_dest_is_rejected() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(8)], "i64"),
            IIRInstr::new("alloc_bytes", Some("base".into()), vec![Operand::Var("n".into())], "i64"),
            IIRInstr::new("const", Some("idx".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("const", Some("val".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new(
                "store_byte",
                Some("oops".into()),
                vec![Operand::Var("base".into()), Operand::Var("idx".into()), Operand::Var("val".into())],
                "i64",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_llvm(&module_with(f), &IIRLlvmConfig::default());
    assert!(err.is_err(), "store_byte must not carry a dest");
}

/// Brainfuck `.` → libc `putchar`: the i64 cell is truncated to the `int` arg.
#[test]
fn putchar_truncs_to_i32_and_declares_libc() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(65)], "i64"),
            IIRInstr::new(
                "call_builtin",
                None,
                vec![Operand::Var("putchar".into()), Operand::Var("v".into())],
                "void",
            ),
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("z".into())], "i64"),
        ],
    );
    let ll = lower(&module_with(f));
    assert!(ll.contains("declare i32 @putchar(i32)"), "putchar declared; got:\n{ll}");
    assert!(ll.contains("trunc i64 65 to i32"), "cell truncated to int; got:\n{ll}");
    assert!(ll.contains("call i32 @putchar(i32"), "calls libc putchar; got:\n{ll}");
}

/// Brainfuck `,` → libc `getchar`: the returned `int` is sign-extended to the
/// i64 cell register (EOF -1 → 0xFF after a later store_byte truncation).
#[test]
fn getchar_calls_libc_and_sexts_to_i64() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new(
                "call_builtin",
                Some("v".into()),
                vec![Operand::Var("getchar".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i64"),
        ],
    );
    let ll = lower(&module_with(f));
    assert!(ll.contains("declare i32 @getchar()"), "getchar declared; got:\n{ll}");
    assert!(ll.contains("call i32 @getchar()"), "calls libc getchar; got:\n{ll}");
    assert!(ll.contains("sext i32"), "result sign-extended to i64; got:\n{ll}");
}

/// Regression: a stack-slot variable written by a real op 2+ times must NOT
/// emit `%v = …` twice (LLVM rejects "multiple definition of local value").
/// The slot wrapper renames each assignment to a fresh SSA name and stores it
/// into the slot. Before LM-L-Brainfuck this round-tripped only for `const`/
/// `mov` slot-dests (which emit no `%v =` line); Brainfuck's `add`-into-slot
/// was the first real trigger.
#[test]
fn slot_var_assigned_twice_by_arith_has_unique_ssa_names() {
    // `v` is the dest of two `add`s → a 2-assignment slot var.
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("const", Some("k".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new(
                "add",
                Some("v".into()),
                vec![Operand::Var("v".into()), Operand::Var("k".into())],
                "i64",
            ),
            IIRInstr::new(
                "add",
                Some("v".into()),
                vec![Operand::Var("v".into()), Operand::Var("k".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i64"),
        ],
    );
    let ll = lower(&module_with(f));
    // `v` is promoted to a slot…
    assert!(ll.contains("%v.slot = alloca i64"), "v promoted to a slot; got:\n{ll}");
    // …and the bare `%v = ` SSA name is never (re)defined.
    assert!(
        !ll.contains("%v = "),
        "slot dest must use fresh SSA names, not a reused %v; got:\n{ll}"
    );
    // Each add result is stored back to the slot.
    assert!(ll.matches("store i64").count() >= 3, "each assignment stores to the slot; got:\n{ll}");
}

// ── Reassigned-parameter promotion (LANG-FULL — LLVM first-class) ──────────────
//
// A parameter reassigned in the body is the `dest` of only one instruction, but
// its incoming argument binding is an implicit first assignment. Without
// promoting it to a stack slot, a reassignment across a loop back-edge is
// silently dropped by the straight-line const/mov side-map. These tests pin the
// fix: reassigned params get an i64 slot initialised from the argument.

#[test]
fn reassigned_parameter_is_promoted_to_a_stack_slot() {
    let f = IIRFunction::new(
        "run",
        vec![("acc".into(), "i64".into())],
        "i64",
        vec![
            IIRInstr::new("const", Some("c6".into()), vec![Operand::Int(6)], "i64"),
            IIRInstr::new("add", Some("t".into()),
                vec![Operand::Var("acc".into()), Operand::Var("c6".into())], "i64"),
            // Reassign the parameter — this is the second effective assignment.
            IIRInstr::new("mov", Some("acc".into()), vec![Operand::Var("t".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("acc".into())], "i64"),
        ],
    );
    let ll = lower(&module_with(f));
    assert!(ll.contains("%acc.slot = alloca i64"),
        "reassigned param must get a stack slot; got:\n{ll}");
    assert!(ll.contains("store i64 %acc, ptr %acc.slot"),
        "param slot must be initialised from the incoming argument; got:\n{ll}");
    assert!(ll.contains("load i64, ptr %acc.slot"),
        "param reads must load from the slot, not the stale SSA arg; got:\n{ll}");
}

#[test]
fn narrow_reassigned_parameter_is_zero_extended_into_its_slot() {
    let f = IIRFunction::new(
        "f",
        vec![("x".into(), "u8".into())],
        "i64",
        vec![
            IIRInstr::new("const", Some("c1".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("add", Some("t".into()),
                vec![Operand::Var("x".into()), Operand::Var("c1".into())], "i64"),
            IIRInstr::new("mov", Some("x".into()), vec![Operand::Var("t".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i64"),
        ],
    );
    let ll = lower(&module_with(f));
    // The u8 param is `i8` in LLVM; the slot is i64, so the init must widen.
    assert!(ll.contains("zext i8 %x to i64"),
        "narrow param must widen to the i64 slot; got:\n{ll}");
    assert!(ll.contains("store i64 %x.init, ptr %x.slot"),
        "widened param must be stored into its slot; got:\n{ll}");
}

#[test]
fn non_reassigned_parameter_stays_pure_ssa() {
    let f = IIRFunction::new(
        "id",
        vec![("x".into(), "i64".into())],
        "i64",
        vec![IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i64")],
    );
    let ll = lower(&module_with(f));
    assert!(!ll.contains("%x.slot"),
        "a parameter that is never reassigned must NOT be slotted; got:\n{ll}");
}

// ===========================================================================
// f64 variable slots (LANG-FULL enabler E3)
// ===========================================================================

/// A `real` local — seeded with an `f64` const then reassigned the result of
/// an `f64` op — is promoted to a slot, which must be a **`double`** slot:
/// `alloca double` + `store double` + `load double`. The old uniform-`i64`
/// slot produced `store i64 <double>`, invalid IR that clang rejected.
#[test]
fn f64_local_gets_a_double_slot() {
    // r := 2.5; r := r * 2.0; ret r   (two writes to `r` → slotted)
    let f = IIRFunction::new(
        "main",
        vec![],
        "f64",
        vec![
            IIRInstr::new("const", Some("r".into()), vec![Operand::Float(2.5)], "f64"),
            IIRInstr::new("const", Some("two".into()), vec![Operand::Float(2.0)], "f64"),
            IIRInstr::new("mul", Some("r".into()),
                vec![Operand::Var("r".into()), Operand::Var("two".into())], "f64"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "f64"),
        ],
    );
    let ll = lower(&module_with(f));
    assert!(ll.contains("%r.slot = alloca double"),
        "f64 local must get a double slot; got:\n{ll}");
    assert!(ll.contains("store double"),
        "f64 slot stores must be `store double`; got:\n{ll}");
    assert!(ll.contains("load double, ptr %r.slot"),
        "f64 slot reads must be `load double`; got:\n{ll}");
    // The float slot must never be accessed as i64 (the old bug).
    assert!(!ll.contains("i64, ptr %r.slot"),
        "float slot must not be load/store'd as i64; got:\n{ll}");
}

/// An `f64` const literal is rendered as LLVM's exact hexadecimal double form
/// (`0x...`), never Rust's `2e0`/`0e0` scientific notation (which lacks a
/// decimal point and is rejected by LLVM's assembler).
#[test]
fn f64_constants_use_hex_double_form() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "f64",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Float(2.0)], "f64"),
            IIRInstr::new("ret", None, vec![Operand::Var("a".into())], "f64"),
        ],
    );
    let ll = lower(&module_with(f));
    // 2.0 → IEEE-754 bits 0x4000000000000000
    assert!(ll.contains("0x4000000000000000"),
        "f64 literal must use the exact hex double form; got:\n{ll}");
    assert!(!ll.contains("2e0"),
        "must not emit decimal-point-less scientific notation; got:\n{ll}");
}

/// A **float comparison** result is a boolean — it must `zext i1` to an integer
/// (`i64`), never to the float operand width (`zext i1 to double` is invalid IR).
#[test]
fn float_comparison_result_zexts_to_integer() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Float(5.0)], "f64"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Float(5.0)], "f64"),
            IIRInstr::new("cmp_eq", Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "f64"),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i64"),
        ],
    );
    let ll = lower(&module_with(f));
    assert!(ll.contains("fcmp oeq double"),
        "float compare must use fcmp; got:\n{ll}");
    assert!(ll.contains("zext i1") && ll.contains("to i64"),
        "float-compare bool result must zext to i64; got:\n{ll}");
    assert!(!ll.contains("zext i1 %c.i1 to double"),
        "must NOT zext a bool to double (invalid IR); got:\n{ll}");
}

/// Integer programs are completely unaffected — an i64 local still gets an
/// `i64` slot (no float typing leaks into the common path).
#[test]
fn integer_local_still_gets_i64_slot() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("add", Some("x".into()),
                vec![Operand::Var("x".into()), Operand::Var("x".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i64"),
        ],
    );
    let ll = lower(&module_with(f));
    assert!(ll.contains("%x.slot = alloca i64"),
        "i64 local must keep its i64 slot; got:\n{ll}");
    assert!(!ll.contains("alloca double"),
        "no double slot should appear for an integer program; got:\n{ll}");
}

// ===========================================================================
// LANG-FULL E5 — bounds-checked arrays (static length-prefixed model)
// ===========================================================================

/// `alloc_array`/`array_set`/`array_get`/`array_len` lower to a length-prefixed
/// `@calloc` block with an explicit `icmp uge`/`llvm.trap` bounds check and a
/// typed `getelementptr`+`load`/`store`.
#[test]
fn array_ops_emit_calloc_trap_and_gep() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(3)], "i64"),
            IIRInstr::new("alloc_array", Some("a".into()), vec![Operand::Var("n".into())], "array<i64>"),
            IIRInstr::new("const", Some("i0".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "i64"),
            IIRInstr::new("array_set", None,
                vec![Operand::Var("a".into()), Operand::Var("i0".into()), Operand::Var("v".into())], "i64"),
            IIRInstr::new("array_get", Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("i0".into())], "i64"),
            IIRInstr::new("array_len", Some("m".into()), vec![Operand::Var("a".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ],
    );
    let ll = lower(&module_with(f));
    // Allocation: length-prefixed calloc block.
    assert!(ll.contains("declare ptr @calloc(i64, i64)"), "calloc declared; got:\n{ll}");
    assert!(ll.contains("call ptr @calloc(i64"), "alloc_array uses calloc");
    // Trap infra + explicit unsigned bounds check (catches negative + >= len).
    assert!(ll.contains("declare void @llvm.trap()"), "llvm.trap declared");
    assert!(ll.contains("icmp uge i64"), "explicit bounds compare");
    assert!(ll.contains("call void @llvm.trap()") && ll.contains("unreachable"),
        "OOB branches to a trap block");
    // Typed element access + length header read.
    assert!(ll.contains("getelementptr i64, ptr"), "typed element GEP");
    assert!(ll.contains("store i64"), "array_set stores the element");
}

/// An `f64` array uses `double` element GEPs / loads / stores.
#[test]
fn f64_array_uses_double_element() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "f64",
        vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(2)], "i64"),
            IIRInstr::new("alloc_array", Some("a".into()), vec![Operand::Var("n".into())], "array<f64>"),
            IIRInstr::new("const", Some("i0".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("const", Some("v".into()), vec![Operand::Float(2.5)], "f64"),
            IIRInstr::new("array_set", None,
                vec![Operand::Var("a".into()), Operand::Var("i0".into()), Operand::Var("v".into())], "f64"),
            IIRInstr::new("array_get", Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("i0".into())], "f64"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "f64"),
        ],
    );
    let ll = lower(&module_with(f));
    assert!(ll.contains("getelementptr double, ptr"), "double element GEP; got:\n{ll}");
    assert!(ll.contains("store double"), "array_set stores a double");
    assert!(ll.contains("load double"), "array_get loads a double");
}

/// E4d-BA-arr: a folded `str` literal stored into an `array<str>` element must be
/// converted from its global-pointer form to an i64 handle with `ptrtoint` before
/// the `store i64` — otherwise the emitted IR is `store i64 @__twig_str_N` (a `ptr`
/// constant in an i64 slot, which clang rejects). The str element is an i64 handle.
#[test]
fn str_array_set_ptrtoints_the_literal_handle() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(2)], "i64"),
            IIRInstr::new("alloc_array", Some("a".into()), vec![Operand::Var("n".into())], "array<str>"),
            IIRInstr::new("const", Some("i0".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("HI".into())], "str"),
            IIRInstr::new("array_set", None,
                vec![Operand::Var("a".into()), Operand::Var("i0".into()), Operand::Var("s".into())], "str"),
            IIRInstr::new("array_get", Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("i0".into())], "str"),
            IIRInstr::new("array_len", Some("m".into()), vec![Operand::Var("a".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("m".into())], "i64"),
        ],
    );
    let ll = lower(&module_with(f));
    assert!(ll.contains("getelementptr i64, ptr"), "str element GEP is i64; got:\n{ll}");
    assert!(ll.contains("ptrtoint ptr @__twig_str"),
        "array_set must ptrtoint the str literal to an i64 handle; got:\n{ll}");
    assert!(!ll.contains("store i64 @__twig_str"),
        "must not store a ptr constant into an i64 slot; got:\n{ll}");
}

/// `array<T>` validates (its element type is checked, not the wrapper).
#[test]
fn array_type_hint_validates() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("alloc_array", Some("a".into()), vec![Operand::Var("n".into())], "array<i64>"),
            IIRInstr::new("array_len", Some("m".into()), vec![Operand::Var("a".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("m".into())], "i64"),
        ],
    );
    assert!(validate_for_llvm(&module_with(f)).is_empty(),
        "array<T> ops must validate clean");
}

// ===========================================================================
// LANG-FULL E6 (layer 1) — typed module globals
// ===========================================================================

/// `bump`: `g := g + 1; return g`. Reads + writes the global `g`.
fn e6_bump() -> IIRFunction {
    IIRFunction::new(
        "bump",
        vec![],
        "i64",
        vec![
            IIRInstr::new("global_load", Some("cur".into()), vec![Operand::Str("g".into())], "i64"),
            IIRInstr::new("const", Some("one".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("add", Some("nxt".into()), vec![Operand::Var("cur".into()), Operand::Var("one".into())], "i64"),
            IIRInstr::new("global_store", None, vec![Operand::Str("g".into()), Operand::Var("nxt".into())], "void"),
            IIRInstr::new("ret", None, vec![Operand::Var("nxt".into())], "i64"),
        ],
    )
}

/// `main`: `g := 41; return bump()` ⇒ 42.
fn e6_main() -> IIRFunction {
    IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("seed".into()), vec![Operand::Int(41)], "i64"),
            IIRInstr::new("global_store", None, vec![Operand::Str("g".into()), Operand::Var("seed".into())], "void"),
            IIRInstr::new("call", Some("res".into()), vec![Operand::Var("bump".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("res".into())], "i64"),
        ],
    )
}

fn e6_module() -> IIRModule {
    let mut m = IIRModule::new("e6", "e6");
    m.add_or_replace(e6_main());
    m.add_or_replace(e6_bump());
    m
}

#[test]
fn e6_global_emits_internal_global_and_load_store() {
    let ll = lower_iir_to_llvm(&e6_module(), &IIRLlvmConfig::default()).expect("lower");
    // One module-level zero-initialised global for `g`.
    assert!(ll.contains("@__twig_global_0 = internal global i64 0"), "missing global def:\n{ll}");
    // `bump` reads it and writes it back.
    assert!(ll.contains("load i64, ptr @__twig_global_0"), "missing load:\n{ll}");
    assert!(ll.contains("store i64"), "missing store:\n{ll}");
    assert!(ll.contains("ptr @__twig_global_0"), "store/load should target the global:\n{ll}");
    // And the op is now accepted by the validator (no rejection messages).
    assert!(validate_for_llvm(&e6_module()).is_empty(), "global ops should validate");
}

/// End-to-end: compile the global program with real `clang` and run it — the
/// cross-function global must yield exit code 42. Skipped if clang is absent.
#[test]
fn e6_global_runs_on_real_clang() {
    use std::process::Command;
    if Command::new("clang").arg("--version").output().map(|o| !o.status.success()).unwrap_or(true) {
        eprintln!("clang not available — skipping e6_global_runs_on_real_clang");
        return;
    }
    let ll = lower_iir_to_llvm(&e6_module(), &IIRLlvmConfig::default()).expect("lower");
    let dir = std::env::temp_dir().join(format!("e6_llvm_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let ll_path = dir.join("prog.ll");
    let exe = dir.join("prog");
    std::fs::write(&ll_path, &ll).expect("write .ll");
    let built = Command::new("clang")
        .arg("-x").arg("ir").arg(&ll_path)
        .arg("-o").arg(&exe)
        .output().expect("run clang");
    assert!(built.status.success(),
        "clang failed:\n{}\n--- .ll ---\n{ll}", String::from_utf8_lossy(&built.stderr));
    let run = Command::new(&exe).status().expect("run exe");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(run.code(), Some(42), "global program should exit 42");
}

// ===========================================================================
// LANG-FULL E8 — numeric conversions (integer ↔ real)
// ===========================================================================

/// `int_to_real` lowers to `sitofp i64 … to double`.
#[test]
fn int_to_real_emits_sitofp() {
    let f = IIRFunction::new(
        "f",
        vec![("x".into(), "i64".into())],
        "f64",
        vec![
            IIRInstr::new("int_to_real", Some("r".into()), vec![Operand::Var("x".into())], "f64"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "f64"),
        ]);
    let ll = lower(&module_with(f));
    assert!(ll.contains("%r = sitofp i64 %x to double"),
        "expected sitofp; got:\n{ll}");
}

/// `real_to_int_floor` rounds with `@llvm.floor.f64`, range-checks (trap), then
/// `fptosi … to i64`. `real_to_int_trunc` uses `@llvm.trunc.f64` instead.
#[test]
fn real_to_int_emits_round_check_and_fptosi() {
    let floor_fn = IIRFunction::new(
        "f",
        vec![("x".into(), "f64".into())],
        "i64",
        vec![
            IIRInstr::new("real_to_int_floor", Some("r".into()), vec![Operand::Var("x".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ]);
    let ll = lower(&module_with(floor_fn));
    assert!(ll.contains("call double @llvm.floor.f64(double %x)"), "expected floor intrinsic:\n{ll}");
    assert!(ll.contains("fcmp oge double") && ll.contains("fcmp olt double"),
        "expected ordered range comparisons:\n{ll}");
    assert!(ll.contains("call void @llvm.trap()"), "expected trap on out-of-range:\n{ll}");
    assert!(ll.contains("fptosi double") && ll.contains("to i64"), "expected fptosi to i64:\n{ll}");
    assert!(ll.contains("declare double @llvm.floor.f64(double)"), "expected floor declare:\n{ll}");

    let trunc_fn = IIRFunction::new(
        "f",
        vec![("x".into(), "f64".into())],
        "i64",
        vec![
            IIRInstr::new("real_to_int_trunc", Some("r".into()), vec![Operand::Var("x".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ]);
    let llt = lower(&module_with(trunc_fn));
    assert!(llt.contains("call double @llvm.trunc.f64(double %x)"), "expected trunc intrinsic:\n{llt}");
}

/// End-to-end on real `clang`: an integer→real→integer round trip through both
/// conversion directions plus an f64 subtraction. `floor(45.0 − 2.7)` =
/// `floor(42.3)` = 42 ⇒ exit code 42. Skipped if clang is absent.
#[test]
fn conversions_round_trip_runs_on_real_clang() {
    use std::process::Command;
    if Command::new("clang").arg("--version").output().map(|o| !o.status.success()).unwrap_or(true) {
        eprintln!("clang not available — skipping conversions_round_trip_runs_on_real_clang");
        return;
    }
    // main() -> i64 { return floor(int_to_real(45) - 2.7); }  = floor(42.3) = 42
    let main = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("i".into()), vec![Operand::Int(45)], "i64"),
            IIRInstr::new("int_to_real", Some("fi".into()), vec![Operand::Var("i".into())], "f64"),
            IIRInstr::new("const", Some("d".into()), vec![Operand::Float(2.7)], "f64"),
            IIRInstr::new("sub", Some("diff".into()),
                vec![Operand::Var("fi".into()), Operand::Var("d".into())], "f64"),
            IIRInstr::new("real_to_int_floor", Some("r".into()), vec![Operand::Var("diff".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ]);
    let ll = lower(&module_with(main));
    let dir = std::env::temp_dir().join(format!("e8_llvm_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let ll_path = dir.join("prog.ll");
    let exe = dir.join("prog");
    std::fs::write(&ll_path, &ll).expect("write .ll");
    // `-lm`: at `-O0` (the default for `clang -x ir`) `@llvm.floor.f64` lowers to
    // a libm `floor` call, which on Linux must be linked explicitly with `-lm`
    // (on macOS libm lives in libSystem, so it is harmless there). A real
    // entier-using program links the same way.
    let built = Command::new("clang")
        .arg("-x").arg("ir").arg(&ll_path)
        .arg("-lm")
        .arg("-o").arg(&exe)
        .output().expect("run clang");
    assert!(built.status.success(),
        "clang failed:\n{}\n--- .ll ---\n{ll}", String::from_utf8_lossy(&built.stderr));
    let run = Command::new(&exe).status().expect("run exe");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(run.code(), Some(42), "floor(45.0 - 2.7) should exit 42");
}

// ===========================================================================
// LANG-FULL E4-dyn — runtime (branch-selected) string on LLVM
// ===========================================================================

/// A `str` variable assigned in **two basic blocks** (a value chosen by control
/// flow) is promoted to an `i64`-**handle** slot: each `str_const` stores the
/// literal global's *address* (`ptrtoint`), and `print_str` reads the length
/// from the block header at run time (`inttoptr` + `load i64`) rather than from a
/// compile-time constant. This is the E4-dyn runtime-string path (E4d-2); a
/// single-assignment string keeps the folded literal fast path (covered
/// elsewhere).
#[test]
fn e4dyn_branch_selected_string_uses_runtime_handle() {
    // A := "HI" (block 0) ; jmp L1 ; L1: A := "LO" (block 2) ; print A ; ret.
    // A is assigned in two blocks, so it becomes a runtime handle slot.
    let f = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![
            IIRInstr::new("str_const", Some("A".into()), vec![Operand::Str("HI".into())], "str"),
            IIRInstr::new("jmp", None, vec![Operand::Var("L1".into())], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("L1".into())], "void"),
            IIRInstr::new("str_const", Some("A".into()), vec![Operand::Str("LO".into())], "str"),
            IIRInstr::new("print_str", None, vec![Operand::Var("A".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let ll = lower(&module_with(f));

    // A is a slot (alloca) carrying an i64 handle.
    assert!(ll.contains("%A.slot = alloca i64"),
        "branch-assigned str `A` should be an i64 handle slot in:\n{ll}");
    // Each str_const stores the literal global's ADDRESS as the handle.
    assert!(ll.contains("ptrtoint ptr @") && ll.contains(" to i64"),
        "str_const into a slot should ptrtoint the literal global in:\n{ll}");
    // print_str recovers the pointer and loads the length from the header at run time.
    assert!(ll.contains("inttoptr i64"),
        "runtime print_str should inttoptr the handle in:\n{ll}");
    assert!(ll.contains("load i64, ptr %__strp"),
        "runtime print_str should load the length from the block header in:\n{ll}");
    assert!(ll.contains("getelementptr inbounds i8, ptr %__strp"),
        "runtime print_str should gep past the 8-byte header in:\n{ll}");
    assert!(ll.contains("call void @__print_str(ptr %__strb"),
        "runtime print_str should call @__print_str with the byte pointer in:\n{ll}");
}

/// A **single-assignment** string is unchanged: it keeps the folded literal fast
/// path (compile-time length, no `inttoptr`/runtime load).
#[test]
fn e4dyn_single_assignment_string_keeps_literal_fast_path() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![
            IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("HI".into())], "str"),
            IIRInstr::new("print_str", None, vec![Operand::Var("s".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let ll = lower(&module_with(f));
    assert!(!ll.contains("%s.slot"),
        "single-assignment str should NOT be a slot in:\n{ll}");
    assert!(!ll.contains("inttoptr"),
        "single-assignment str print should use the literal fast path (no inttoptr) in:\n{ll}");
    assert!(ll.contains("call void @__print_str"),
        "literal print_str still calls @__print_str in:\n{ll}");
}

// ===========================================================================
// E4-dyn (E4d-2b): a runtime string as a function RETURN VALUE / call result
// ===========================================================================

/// An ALGOL `string procedure` lowers to a function returning `str` — carried
/// as an i64 **handle** — that the caller prints. This exercises the E4d-2b
/// runtime path: `str` maps to `i64` at the function boundary, and `print_str`
/// of a *call result* (which has no compile-time length) reads the length from
/// the block header at run time.
#[test]
fn e4dyn_string_procedure_return_and_call_result_print() {
    // pick(n) -> str : if n > 0 then "HI" else "LO"  (branch-selected → slot)
    let pick = IIRFunction::new(
        "pick",
        vec![("n".into(), "i64".into())],
        "str",
        vec![
            IIRInstr::new("str_const", Some("pick".into()), vec![Operand::Str(String::new())], "str"),
            IIRInstr::new("const", Some("c0".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("cmp_gt", Some("t".into()),
                vec![Operand::Var("n".into()), Operand::Var("c0".into())], "i64"),
            IIRInstr::new("jmp_if_false", None,
                vec![Operand::Var("t".into()), Operand::Var("Lelse".into())], "void"),
            IIRInstr::new("str_const", Some("pick".into()), vec![Operand::Str("HI".into())], "str"),
            IIRInstr::new("jmp", None, vec![Operand::Var("Ldone".into())], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("Lelse".into())], "void"),
            IIRInstr::new("str_const", Some("pick".into()), vec![Operand::Str("LO".into())], "str"),
            IIRInstr::new("label", None, vec![Operand::Var("Ldone".into())], "void"),
            IIRInstr::new("ret", None, vec![Operand::Var("pick".into())], "str"),
        ],
    );
    // main() : print(pick(1))
    let main = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("one".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("call", Some("r".into()),
                vec![Operand::Var("pick".into()), Operand::Var("one".into())], "str"),
            IIRInstr::new("print_str", None, vec![Operand::Var("r".into())], "void"),
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("z".into())], "i64"),
        ],
    );
    let module = IIRModule {
        name: "strproc".into(),
        functions: vec![pick, main],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let ll = lower(&module);

    // `str` return type + call result map to i64 (the handle).
    assert!(ll.contains("define i64 @pick(i64 %n)"),
        "string procedure must lower to `define i64 @pick(i64 %n)`:\n{ll}");
    assert!(ll.contains("= call i64 @pick(i64"),
        "the call result must be typed i64 (a runtime handle):\n{ll}");
    assert!(ll.contains("ret i64 "),
        "pick must return an i64 handle:\n{ll}");
    // The call result is printed via the runtime path: recover the pointer,
    // read the length header, then call @__print_str.
    assert!(ll.contains("inttoptr i64"),
        "print of a call-result runtime string must inttoptr the handle:\n{ll}");
    assert!(ll.contains("load i64, ptr %__strp"),
        "print of a call-result runtime string must read the length header:\n{ll}");
    assert!(ll.contains("call void @__print_str(ptr %__strb"),
        "print of a call-result runtime string must call @__print_str with the bytes ptr:\n{ll}");
}

/// `str_len` of a runtime string (a call result / param) reads the length from
/// the block header at run time (`inttoptr` + `load i64`) instead of folding a
/// compile-time constant.
#[test]
fn e4dyn_str_len_of_runtime_string_reads_header() {
    // id(s) -> str : ret s     (s is a runtime str handle param)
    let id = IIRFunction::new(
        "id",
        vec![("s".into(), "str".into())],
        "str",
        vec![IIRInstr::new("ret", None, vec![Operand::Var("s".into())], "str")],
    );
    // main(p: str) : n = str_len(p); ret n   (p is a runtime handle param)
    let main = IIRFunction::new(
        "main",
        vec![("p".into(), "str".into())],
        "i64",
        vec![
            IIRInstr::new("str_len", Some("n".into()), vec![Operand::Var("p".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "i64"),
        ],
    );
    let module = IIRModule {
        name: "strlen_rt".into(),
        functions: vec![id, main],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let ll = lower(&module);
    assert!(ll.contains("inttoptr i64") && ll.contains("load i64, ptr %__slp"),
        "str_len of a runtime string must read the length header at run time:\n{ll}");
}

/// E4-dyn: BASIC string `INPUT A$` lowers `call_builtin "input_str"` to a call
/// to the AOT runtime helper `@__twig_input_str()`, which returns an i64 handle
/// to a `[i64 len][bytes]` heap block. The extern must be declared, and the
/// `str`-typed result must not be rejected by the validator.
#[test]
fn input_str_lowers_to_twig_input_str_call() {
    let f = IIRFunction::new("main", vec![], "i64", vec![
        IIRInstr::new("call_builtin", Some("t".into()),
            vec![Operand::Var("input_str".into())], "str"),
        IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("z".into())], "i64"),
    ]);
    assert!(validate_for_llvm(&module_with(f.clone())).is_empty(),
        "str-typed input_str call_builtin must validate");
    let ll = lower(&module_with(f));
    assert!(ll.contains("call i64 @__twig_input_str()"),
        "input_str must call the runtime helper; got:\n{ll}");
    assert!(ll.contains("declare i64 @__twig_input_str()"),
        "the @__twig_input_str extern must be declared; got:\n{ll}");
}

/// E4-dyn: `str_concat` over two RUNTIME string operands (here two `input_str`
/// handles — neither is a compile-time literal) lowers to a call to the AOT helper
/// `@__twig_str_concat(i64, i64)`, which reads both `[i64 len][bytes]` headers and
/// returns a handle to a fresh joined block. The extern must be declared, and the
/// runtime result must feed a runtime `str_len` (header read) without folding.
#[test]
fn runtime_str_concat_lowers_to_twig_str_concat_call() {
    let f = IIRFunction::new("main", vec![], "i64", vec![
        IIRInstr::new("call_builtin", Some("a".into()),
            vec![Operand::Var("input_str".into())], "str"),
        IIRInstr::new("call_builtin", Some("b".into()),
            vec![Operand::Var("input_str".into())], "str"),
        IIRInstr::new("str_concat", Some("s".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "str"),
        IIRInstr::new("str_len", Some("n".into()), vec![Operand::Var("s".into())], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "i64"),
    ]);
    assert!(validate_for_llvm(&module_with(f.clone())).is_empty(),
        "runtime str_concat must validate");
    let ll = lower(&module_with(f));
    assert!(ll.contains("call i64 @__twig_str_concat(i64 "),
        "runtime str_concat must call the runtime helper; got:\n{ll}");
    assert!(ll.contains("declare i64 @__twig_str_concat(i64, i64)"),
        "the @__twig_str_concat extern must be declared; got:\n{ll}");
}

/// A runtime string copy is represented as a concat with the empty literal suffix.
/// The literal global must be converted to an i64 handle before the helper call.
#[test]
fn runtime_str_concat_ptrtoints_a_mixed_literal_operand() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new(
                "call_builtin",
                Some("runtime".into()),
                vec![Operand::Var("input_str".into())],
                "str",
            ),
            IIRInstr::new(
                "str_const",
                Some("empty".into()),
                vec![Operand::Str(String::new())],
                "str",
            ),
            IIRInstr::new(
                "str_concat",
                Some("copy".into()),
                vec![Operand::Var("runtime".into()), Operand::Var("empty".into())],
                "str",
            ),
            IIRInstr::new("str_len", Some("n".into()), vec![Operand::Var("copy".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "i64"),
        ],
    );
    let ll = lower(&module_with(f));
    assert!(
        ll.contains("ptrtoint ptr @__twig_str_0 to i64"),
        "a mixed literal concat operand must become an i64 handle; got:\n{ll}"
    );
    assert!(
        ll.contains("call i64 @__twig_str_concat(i64 %runtime, i64 %__scch"),
        "the runtime concat must receive the converted literal handle; got:\n{ll}"
    );
}

/// LANG-FULL tail — a string LITERAL passed across a function boundary must be
/// converted from its global-pointer form to an i64 handle with `ptrtoint` before
/// the call. Otherwise `call i64 @strlen(i64 @__twig_str_0)` puts a `ptr` constant
/// in an `i64` argument slot — invalid IR clang rejects. Regression for the
/// Twig/lisp cell `(define (strlen (s : str)) (string-length s)) (strlen "HELLO")`.
#[test]
fn str_literal_call_arg_is_ptrtoint_to_i64() {
    // fn strlen(s: str) -> i64 { str_len s }
    let strlen = IIRFunction::new(
        "strlen",
        vec![("s".into(), "str".into())],
        "i64",
        vec![
            IIRInstr::new("str_len", Some("n".into()), vec![Operand::Var("s".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "i64"),
        ],
    );
    // fn main() -> i64 { let s1 = "HELLO"; strlen(s1) }
    let main = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("str_const", Some("s1".into()), vec![Operand::Str("HELLO".into())], "str"),
            IIRInstr::new("call", Some("r".into()),
                vec![Operand::Var("strlen".into()), Operand::Var("s1".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ],
    );
    let module = IIRModule {
        name: "t".into(),
        functions: vec![strlen, main],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let ll = lower(&module);
    assert!(ll.contains("ptrtoint ptr @__twig_str"),
        "a str literal passed as a call arg must be ptrtoint'd to i64; got:\n{ll}");
    assert!(!ll.contains("@strlen(i64 @__twig_str"),
        "the global pointer must NOT be passed directly in an i64 arg slot; got:\n{ll}");
}

/// LANG-FULL tail — `str_eq` over runtime string PARAMETERS lowers to a call to the
/// archive helper `@__twig_str_eq(i64, i64)` (not a compile-time fold). Regression for
/// the Twig cell `(define (same a b) (if (string=? a b) 42 0)) (same "OK" (...))`,
/// where `lower_str_eq` previously errored on the params `a`/`b` (not literals).
#[test]
fn str_eq_over_params_calls_twig_str_eq() {
    let same = IIRFunction::new(
        "same",
        vec![("a".into(), "str".into()), ("b".into(), "str".into())],
        "i64",
        vec![
            IIRInstr::new("str_eq", Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ],
    );
    let ll = lower(&module_with(same));
    assert!(ll.contains("call i64 @__twig_str_eq(i64 %a, i64 %b)"),
        "str_eq over params must call the runtime helper; got:\n{ll}");
    assert!(ll.contains("declare i64 @__twig_str_eq(i64, i64)"),
        "the @__twig_str_eq extern must be declared; got:\n{ll}");
}

/// LANG-FULL tail — runtime lexical string ordering has the same handle path as
/// equality, but preserves the shared -1/0/1 contract for a downstream numeric branch.
#[test]
fn str_cmp_over_params_calls_twig_str_cmp() {
    let compare = IIRFunction::new(
        "compare",
        vec![("a".into(), "str".into()), ("b".into(), "str".into())],
        "i64",
        vec![
            IIRInstr::new(
                "str_cmp",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ],
    );
    let ll = lower(&module_with(compare));
    assert!(
        ll.contains("call i64 @__twig_str_cmp(i64 %a, i64 %b)"),
        "str_cmp over params must call the runtime helper; got:\n{ll}"
    );
    assert!(
        ll.contains("declare i64 @__twig_str_cmp(i64, i64)"),
        "the @__twig_str_cmp extern must be declared; got:\n{ll}"
    );
}

// ===========================================================================
// 8. E6d-6-LLVM — structural heap ops (alloc / field_store / field_load /
//    is_null) + special-char function-name quoting.
//
//    These give the LLVM column the same word-granular heap model the native
//    backend uses, so Twig records (and, once the tagged-world int→any
//    coercion lands, unions) run on LLVM. A heap object is a `__twig_gc_alloc`'d
//    block; the handle + every field are raw 64-bit words, so a field lives at
//    byte offset `idx*8` — one `getelementptr i64, ptr, i64 <idx>`.
// ===========================================================================

/// A one-function module that exercises all four heap ops, threading a param
/// `v` into a freshly-allocated object and reading it back.
fn heap_ops_module() -> IIRModule {
    let f = IIRFunction::new(
        "main",
        vec![("v".into(), "i64".into())],
        "i64",
        vec![
            IIRInstr::new("alloc", Some("c".into()), vec![], "ref<LispyPair>"),
            IIRInstr::new(
                "field_store",
                None,
                vec![Operand::Var("c".into()), Operand::Int(0), Operand::Var("v".into())],
                "void",
            ),
            IIRInstr::new(
                "field_load",
                Some("r".into()),
                vec![Operand::Var("c".into()), Operand::Int(1)],
                "ref<any>",
            ),
            IIRInstr::new("is_null", Some("n".into()), vec![Operand::Var("c".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ],
    );
    module_with(f)
}

#[test]
fn alloc_calls_gc_alloc_with_default_size_and_declares_extern() {
    let ll = lower(&heap_ops_module());
    // Default payload is a 2-word LispyPair (16 bytes), matching the native backend.
    assert!(ll.contains("call i64 @__twig_gc_alloc(i64 16)"), "{ll}");
    // The extern must be declared exactly once when `alloc` is used.
    assert_eq!(ll.matches("declare i64 @__twig_gc_alloc(i64)").count(), 1, "{ll}");
}

#[test]
fn alloc_honours_explicit_payload_size() {
    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("alloc", Some("c".into()), vec![Operand::Int(24)], "ref<LispyPair>"),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i64"),
        ],
    );
    let ll = lower(&module_with(f));
    assert!(ll.contains("call i64 @__twig_gc_alloc(i64 24)"), "{ll}");
}

#[test]
fn field_store_writes_word_at_scaled_offset() {
    let ll = lower(&heap_ops_module());
    // inttoptr the i64 handle, GEP by the field index (i64-scaled → *8), store the word.
    assert!(ll.contains("inttoptr i64 %c to ptr"), "{ll}");
    assert!(ll.contains("getelementptr i64, ptr"), "{ll}");
    assert!(ll.contains("store i64 %v, ptr"), "{ll}");
}

#[test]
fn field_load_reads_word_at_scaled_offset() {
    let ll = lower(&heap_ops_module());
    // field_load[1] → GEP index 1 then load an i64 into the dest.
    assert!(ll.contains("getelementptr i64, ptr %flp") || ll.contains("getelementptr i64, ptr"), "{ll}");
    assert!(ll.contains("%r = load i64, ptr"), "{ll}");
}

#[test]
fn is_null_compares_handle_to_zero_and_zexts() {
    let ll = lower(&heap_ops_module());
    assert!(ll.contains("icmp eq i64 %c, 0"), "{ll}");
    assert!(ll.contains("zext i1 %n.i1 to i64"), "{ll}");
}

#[test]
fn field_store_must_not_have_dest() {
    let f = IIRFunction::new(
        "main",
        vec![("v".into(), "i64".into())],
        "void",
        vec![
            IIRInstr::new("alloc", Some("c".into()), vec![], "ref<LispyPair>"),
            IIRInstr::new(
                "field_store",
                Some("bad".into()),
                vec![Operand::Var("c".into()), Operand::Int(0), Operand::Var("v".into())],
                "void",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_llvm(&module_with(f), &IIRLlvmConfig::default()).unwrap_err();
    assert!(matches!(err, IIRLlvmError::InvalidOperand { .. }), "{err:?}");
}

#[test]
fn field_index_must_be_non_negative_int() {
    let f = IIRFunction::new(
        "main",
        vec![("v".into(), "i64".into())],
        "void",
        vec![
            IIRInstr::new("alloc", Some("c".into()), vec![], "ref<LispyPair>"),
            IIRInstr::new(
                "field_store",
                None,
                // srcs[1] is a Var, not an Int field index → rejected.
                vec![Operand::Var("c".into()), Operand::Var("v".into()), Operand::Var("v".into())],
                "void",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_llvm(&module_with(f), &IIRLlvmConfig::default()).unwrap_err();
    assert!(matches!(err, IIRLlvmError::InvalidOperand { .. }), "{err:?}");
}

/// A function whose name needs LLVM quoting (`?`), called from `main`. Both the
/// `define` and the `call` must use the SAME quoted spelling so the reference
/// resolves — an unquoted `@Some?` is a hard LLVM parse error.
#[test]
fn special_char_function_names_are_quoted_at_define_and_call() {
    let predicate = IIRFunction::new(
        "Some?",
        vec![("v".into(), "i64".into())],
        "i64",
        vec![IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i64")],
    );
    let main = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("k".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new(
                "call",
                Some("r".into()),
                vec![Operand::Var("Some?".into()), Operand::Var("k".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ],
    );
    let module = IIRModule {
        name: "t".into(),
        functions: vec![predicate, main],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let ll = lower(&module);
    assert!(ll.contains(r#"define i64 @"Some?"("#), "define not quoted:\n{ll}");
    assert!(ll.contains(r#"call i64 @"Some?"("#), "call not quoted:\n{ll}");
    // A hyphenated name (Twig record accessor `point-x`) also needs quoting; a
    // plain identifier must stay UNQUOTED (quoting is conservative but the common
    // case must not regress to noisy output).
    assert!(!ll.contains(r#"@"main""#), "plain name should stay unquoted:\n{ll}");
}
