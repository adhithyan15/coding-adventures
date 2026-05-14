//! Integration tests for the `iir-to-cil-bytecode` crate.
//!
//! These tests exercise the public API end-to-end, covering:
//!
//! - `validate_iir_for_clr` — every error case and several valid programs.
//! - `lower_iir_to_cil` — opcode coverage, branch synthesis, comparisons,
//!   calls, register allocation, and multi-function modules.
//! - `IIRClrCodeGenerator` — the `CodeGenerator` protocol adapter.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_cil_bytecode::{
    IIRClrConfig, IIRClrError,
    lower_iir_to_cil, validate_iir_for_clr, IIRClrCodeGenerator,
    CILProgramArtifact,
};
use codegen_core::codegen::CodeGenerator;

// ===========================================================================
// Builder helpers
// ===========================================================================

/// Build a module with a single parameterless `main` function.
fn single_fn(instrs: Vec<IIRInstr>) -> IIRModule {
    let fn_ = IIRFunction::new("main", vec![], "void", instrs);
    let mut module = IIRModule::new("test", "tetrad");
    module.entry_point = Some("main".into());
    module.add_or_replace(fn_);
    module
}

/// Build a module with a single function that takes `params` and returns the
/// given type.
fn fn_with_params(
    params: Vec<(&str, &str)>,
    return_type: &str,
    instrs: Vec<IIRInstr>,
) -> IIRModule {
    let params_owned: Vec<(String, String)> = params
        .into_iter()
        .map(|(n, t)| (n.into(), t.into()))
        .collect();
    let fn_ = IIRFunction::new("myfn", params_owned, return_type, instrs);
    let mut module = IIRModule::new("test", "tetrad");
    module.entry_point = Some("myfn".into());
    module.add_or_replace(fn_);
    module
}

fn default_cfg() -> IIRClrConfig {
    IIRClrConfig::default()
}

/// CIL opcode constants used in assertions.
const RET: u8 = 0x2A;
const NOP: u8 = 0x00;
const ADD: u8 = 0x58;
const SUB: u8 = 0x59;
const MUL: u8 = 0x5A;
const DIV: u8 = 0x5B;
const REM: u8 = 0x5D; // `rem` — not in CILOpcode enum
const AND: u8 = 0x5F;
const OR:  u8 = 0x60;
const XOR: u8 = 0x61;
const SHL: u8 = 0x62;
const SHR: u8 = 0x63;
const NEG: u8 = 0x65; // raw `neg`
const NOT: u8 = 0x66; // raw `not`
const PFX: u8 = 0xFE; // 0xFE prefix for ceq/cgt/clt

// ===========================================================================
// validate_iir_for_clr — error cases
// ===========================================================================

#[test]
fn validate_empty_module_is_rejected() {
    let module = IIRModule {
        name: "empty".into(),
        functions: vec![],
        entry_point: None,
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let errs = validate_iir_for_clr(&module);
    assert!(!errs.is_empty());
    assert!(errs[0].contains("EmptyModule"), "got: {:?}", errs);
}

#[test]
fn validate_empty_function_is_rejected() {
    let errs = validate_iir_for_clr(&single_fn(vec![]));
    assert!(!errs.is_empty());
    assert!(errs[0].contains("EmptyFunction"), "got: {:?}", errs);
}

#[test]
fn validate_any_type_hint_is_rejected() {
    let errs = validate_iir_for_clr(&single_fn(vec![
        IIRInstr::new("add", Some("v".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
    ]));
    assert!(errs.iter().any(|e| e.contains("UntypedInstruction")));
}

#[test]
fn validate_polymorphic_type_hint_is_rejected() {
    let errs = validate_iir_for_clr(&single_fn(vec![
        IIRInstr::new("ret_void", None, vec![], "polymorphic"),
    ]));
    assert!(errs.iter().any(|e| e.contains("UntypedInstruction")));
}

#[test]
fn validate_float_const_is_rejected() {
    let errs = validate_iir_for_clr(&single_fn(vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Float(3.14)], "f64"),
    ]));
    assert!(errs.iter().any(|e| e.contains("Float")));
}

#[test]
fn validate_str_type_is_rejected() {
    let errs = validate_iir_for_clr(&single_fn(vec![
        IIRInstr::new("ret_void", None, vec![], "str"),
    ]));
    assert!(errs.iter().any(|e| e.contains("UnsupportedType")));
}

#[test]
fn validate_ref_type_is_rejected() {
    let errs = validate_iir_for_clr(&single_fn(vec![
        IIRInstr::new("ret_void", None, vec![], "ref<u8>"),
    ]));
    assert!(errs.iter().any(|e| e.contains("UnsupportedType")));
}

#[test]
fn validate_io_in_op_is_rejected() {
    let errs = validate_iir_for_clr(&single_fn(vec![
        IIRInstr::new("io_in", Some("v".into()), vec![], "i32"),
    ]));
    assert!(errs.iter().any(|e| e.contains("UnsupportedOp")));
}

#[test]
fn validate_alloc_op_is_rejected() {
    // Phase 2: `alloc` is only accepted for `ref<LispyPair>`.
    // `ref<i32>` should be rejected as UnsupportedType (not UnsupportedOp,
    // since `alloc` itself is now promoted to the accepted ops list and the
    // type restriction is enforced separately).
    let errs = validate_iir_for_clr(&single_fn(vec![
        IIRInstr::new("alloc", Some("v".into()), vec![Operand::Int(4)], "ref<i32>"),
    ]));
    assert!(!errs.is_empty(), "alloc ref<i32> must be rejected");
    assert!(errs.iter().any(|e| e.contains("UnsupportedType") || e.contains("UnsupportedOp")),
        "error should be UnsupportedType or UnsupportedOp: {:?}", errs);
}

#[test]
fn validate_safepoint_is_rejected() {
    let errs = validate_iir_for_clr(&single_fn(vec![
        IIRInstr::new("safepoint", None, vec![], "void"),
    ]));
    assert!(errs.iter().any(|e| e.contains("UnsupportedOp")));
}

// ===========================================================================
// validate_iir_for_clr — valid programs
// ===========================================================================

#[test]
fn validate_ret_void_is_valid() {
    let errs = validate_iir_for_clr(&single_fn(vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]));
    assert!(errs.is_empty(), "{:?}", errs);
}

#[test]
fn validate_int_arithmetic_is_valid() {
    let errs = validate_iir_for_clr(&single_fn(vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i32"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(2)], "i32"),
        IIRInstr::new("add", Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i32"),
    ]));
    assert!(errs.is_empty(), "{:?}", errs);
}

#[test]
fn validate_bool_const_is_valid() {
    let errs = validate_iir_for_clr(&single_fn(vec![
        IIRInstr::new("const", Some("f".into()), vec![Operand::Bool(false)], "bool"),
        IIRInstr::new("ret", None, vec![Operand::Var("f".into())], "bool"),
    ]));
    assert!(errs.is_empty(), "{:?}", errs);
}

// ===========================================================================
// lower_iir_to_cil — basic opcode coverage
// ===========================================================================

#[test]
fn lower_ret_void_body_contains_ret() {
    let module = single_fn(vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    assert!(artifact.methods[0].body.contains(&RET));
}

#[test]
fn lower_body_is_non_empty() {
    let module = single_fn(vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    assert!(!artifact.methods[0].body.is_empty());
}

#[test]
fn lower_method_name_matches_function_name() {
    let module = single_fn(vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    assert_eq!(artifact.methods[0].name, "main");
}

#[test]
fn lower_add_emits_add_opcode() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(3)], "i32"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(4)], "i32"),
        IIRInstr::new("add", Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i32"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    assert!(body.contains(&ADD), "expected add (0x58): {body:?}");
}

#[test]
fn lower_sub_emits_sub_opcode() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(10)], "i32"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(3)], "i32"),
        IIRInstr::new("sub", Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i32"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    assert!(body.contains(&SUB), "expected sub (0x59): {body:?}");
}

#[test]
fn lower_mul_emits_mul_opcode() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(6)], "i32"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(7)], "i32"),
        IIRInstr::new("mul", Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i32"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    assert!(body.contains(&MUL), "expected mul (0x5A): {body:?}");
}

#[test]
fn lower_div_emits_div_opcode() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(10)], "i32"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(2)], "i32"),
        IIRInstr::new("div", Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i32"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    assert!(body.contains(&DIV), "expected div (0x5B): {body:?}");
}

#[test]
fn lower_mod_emits_rem_opcode() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(10)], "i32"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(3)], "i32"),
        IIRInstr::new("mod", Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i32"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    assert!(body.contains(&REM), "expected rem (0x5D): {body:?}");
}

#[test]
fn lower_neg_emits_neg_opcode() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(5)], "i32"),
        IIRInstr::new("neg", Some("b".into()), vec![Operand::Var("a".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i32"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    assert!(body.contains(&NEG), "expected neg (0x65): {body:?}");
}

#[test]
fn lower_and_emits_and_opcode() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(0xFF)], "i32"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(0x0F)], "i32"),
        IIRInstr::new("and", Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i32"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    assert!(body.contains(&AND), "expected and (0x5F): {body:?}");
}

#[test]
fn lower_or_emits_or_opcode() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(0xF0)], "i32"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(0x0F)], "i32"),
        IIRInstr::new("or", Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i32"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    assert!(body.contains(&OR), "expected or (0x60): {body:?}");
}

#[test]
fn lower_xor_emits_xor_opcode() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(0b1010)], "i32"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(0b1100)], "i32"),
        IIRInstr::new("xor", Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i32"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    assert!(body.contains(&XOR), "expected xor (0x61): {body:?}");
}

#[test]
fn lower_not_emits_native_not_opcode() {
    // We use the native `not` opcode (0x66), not the XOR-with-minus-one synthesis.
    let module = single_fn(vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(0xFF)], "i32"),
        IIRInstr::new("not", Some("b".into()), vec![Operand::Var("a".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i32"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    assert!(body.contains(&NOT), "expected not (0x66): {body:?}");
}

#[test]
fn lower_shl_emits_shl_opcode() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i32"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(3)], "i32"),
        IIRInstr::new("shl", Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i32"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    assert!(body.contains(&SHL), "expected shl (0x62): {body:?}");
}

#[test]
fn lower_shr_emits_shr_opcode() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(16)], "i32"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(1)], "i32"),
        IIRInstr::new("shr", Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i32"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    assert!(body.contains(&SHR), "expected shr (0x63): {body:?}");
}

// ===========================================================================
// lower_iir_to_cil — comparison opcodes
// ===========================================================================

#[test]
fn lower_cmp_eq_emits_ceq() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i32"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(1)], "i32"),
        IIRInstr::new("cmp_eq", Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "bool"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "bool"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    // ceq = 0xFE 0x01
    assert!(body.windows(2).any(|w| w == [PFX, 0x01]),
        "expected ceq (0xFE 0x01): {body:?}");
}

#[test]
fn lower_cmp_lt_emits_clt() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i32"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(2)], "i32"),
        IIRInstr::new("cmp_lt", Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "bool"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "bool"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    // clt = 0xFE 0x04
    assert!(body.windows(2).any(|w| w == [PFX, 0x04]),
        "expected clt (0xFE 0x04): {body:?}");
}

#[test]
fn lower_cmp_gt_emits_cgt() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(5)], "i32"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(2)], "i32"),
        IIRInstr::new("cmp_gt", Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "bool"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "bool"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    // cgt = 0xFE 0x02
    assert!(body.windows(2).any(|w| w == [PFX, 0x02]),
        "expected cgt (0xFE 0x02): {body:?}");
}

#[test]
fn lower_cmp_ne_produces_two_ceq_sequences() {
    // cmp_ne synthesizes: ceq; ldc.i4.0; ceq
    let module = single_fn(vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i32"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(2)], "i32"),
        IIRInstr::new("cmp_ne", Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "bool"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "bool"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    let ceq_count = body.windows(2).filter(|w| *w == [PFX, 0x01]).count();
    assert_eq!(ceq_count, 2, "cmp_ne must produce two ceq sequences: {body:?}");
}

#[test]
fn lower_cmp_le_uses_cgt_then_not() {
    // cmp_le synthesizes: cgt; ldc.i4.0; ceq
    let module = single_fn(vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(3)], "i32"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(5)], "i32"),
        IIRInstr::new("cmp_le", Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "bool"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "bool"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    // cgt = 0xFE 0x02
    assert!(body.windows(2).any(|w| w == [PFX, 0x02]),
        "cmp_le must contain cgt (0xFE 0x02): {body:?}");
    // followed by ceq (0xFE 0x01)
    assert!(body.windows(2).any(|w| w == [PFX, 0x01]),
        "cmp_le must contain ceq (0xFE 0x01): {body:?}");
}

#[test]
fn lower_cmp_ge_uses_clt_then_not() {
    // cmp_ge synthesizes: clt; ldc.i4.0; ceq
    let module = single_fn(vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(5)], "i32"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(3)], "i32"),
        IIRInstr::new("cmp_ge", Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "bool"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "bool"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    // clt = 0xFE 0x04
    assert!(body.windows(2).any(|w| w == [PFX, 0x04]),
        "cmp_ge must contain clt (0xFE 0x04): {body:?}");
    assert!(body.windows(2).any(|w| w == [PFX, 0x01]),
        "cmp_ge must contain ceq (0xFE 0x01): {body:?}");
}

// ===========================================================================
// lower_iir_to_cil — control flow
// ===========================================================================

#[test]
fn lower_label_and_jmp_emits_branch() {
    let module = single_fn(vec![
        IIRInstr::new("jmp", None, vec![Operand::Var("end".into())], "void"),
        IIRInstr::new("label", None, vec![Operand::Var("end".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    // br.s = 0x2B, br = 0x38
    assert!(
        body.contains(&0x2B) || body.contains(&0x38),
        "expected br/br.s in: {body:?}"
    );
}

#[test]
fn lower_jmp_if_true_emits_brtrue() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("cond".into()), vec![Operand::Bool(true)], "bool"),
        IIRInstr::new("label", None, vec![Operand::Var("loop".into())], "void"),
        IIRInstr::new("jmp_if_true", None,
            vec![Operand::Var("cond".into()), Operand::Var("loop".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    // brtrue.s = 0x2D, brtrue = 0x3A
    assert!(
        body.contains(&0x2D) || body.contains(&0x3A),
        "expected brtrue in: {body:?}"
    );
}

#[test]
fn lower_jmp_if_false_emits_brfalse() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("cond".into()), vec![Operand::Bool(false)], "bool"),
        IIRInstr::new("label", None, vec![Operand::Var("skip".into())], "void"),
        IIRInstr::new("jmp_if_false", None,
            vec![Operand::Var("cond".into()), Operand::Var("skip".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    // brfalse.s = 0x2C, brfalse = 0x39
    assert!(
        body.contains(&0x2C) || body.contains(&0x39),
        "expected brfalse in: {body:?}"
    );
}

// ===========================================================================
// lower_iir_to_cil — register/call operations
// ===========================================================================

#[test]
fn lower_load_reg_emits_copy() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("src".into()), vec![Operand::Int(99)], "i32"),
        IIRInstr::new("load_reg", Some("dst".into()),
            vec![Operand::Var("src".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("dst".into())], "i32"),
    ]);
    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    assert!(!artifact.methods[0].body.is_empty());
    assert!(artifact.methods[0].body.contains(&RET));
}

#[test]
fn lower_store_reg_is_copy() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("src".into()), vec![Operand::Int(7)], "i32"),
        IIRInstr::new("const", Some("dst".into()), vec![Operand::Int(0)], "i32"),
        IIRInstr::new("store_reg", None,
            vec![Operand::Var("dst".into()), Operand::Var("src".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("dst".into())], "i32"),
    ]);
    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    assert!(!artifact.methods[0].body.is_empty());
}

#[test]
fn lower_type_assert_emits_nop() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(1)], "i32"),
        IIRInstr::new("type_assert", None, vec![Operand::Var("v".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    assert!(body.contains(&NOP), "expected nop (0x00): {body:?}");
}

// ===========================================================================
// lower_iir_to_cil — parameter passing (ldarg/starg)
// ===========================================================================

#[test]
fn lower_param_function_uses_ldarg() {
    // add(a: i32, b: i32) -> i32 { ret a + b }
    // Parameters should use ldarg (0x02–0x05 or 0x0E).
    let module = fn_with_params(
        vec![("a", "i32"), ("b", "i32")],
        "i32",
        vec![
            IIRInstr::new("add", Some("v0".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
        ],
    );
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    // ldarg.0 = 0x02, ldarg.1 = 0x03
    assert!(body.contains(&0x02) || body.contains(&0x0E),
        "expected ldarg in: {body:?}");
    assert!(body.contains(&ADD), "expected add (0x58): {body:?}");
}

// ===========================================================================
// lower_iir_to_cil — multi-function modules
// ===========================================================================

#[test]
fn lower_two_function_module_produces_two_methods() {
    let fn1 = IIRFunction::new("square",
        vec![("x".into(), "i32".into())], "i32",
        vec![
            IIRInstr::new("mul", Some("v0".into()),
                vec![Operand::Var("x".into()), Operand::Var("x".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
        ]);
    let fn2 = IIRFunction::new("double",
        vec![("x".into(), "i32".into())], "i32",
        vec![
            IIRInstr::new("add", Some("v0".into()),
                vec![Operand::Var("x".into()), Operand::Var("x".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
        ]);
    let mut module = IIRModule::new("math", "tetrad");
    module.entry_point = Some("square".into());
    module.add_or_replace(fn1);
    module.add_or_replace(fn2);

    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    assert_eq!(artifact.methods.len(), 2);
    assert_eq!(artifact.methods[0].name, "square");
    assert_eq!(artifact.methods[1].name, "double");
    assert!(artifact.methods[0].body.contains(&MUL));
    assert!(artifact.methods[1].body.contains(&ADD));
}

#[test]
fn lower_call_emits_call_opcode() {
    // double(x) { ret x + x }
    // main() { v = call double 5; ret v }
    let fn_double = IIRFunction::new("double",
        vec![("x".into(), "i32".into())], "i32",
        vec![
            IIRInstr::new("add", Some("v".into()),
                vec![Operand::Var("x".into()), Operand::Var("x".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
        ]);
    let fn_main = IIRFunction::new("main",
        vec![], "i32",
        vec![
            IIRInstr::new("const", Some("five".into()), vec![Operand::Int(5)], "i32"),
            // call double with arg five
            IIRInstr::new("call", Some("result".into()),
                vec![Operand::Var("double".into()), Operand::Var("five".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("result".into())], "i32"),
        ]);
    let mut module = IIRModule::new("test", "tetrad");
    module.entry_point = Some("main".into());
    module.add_or_replace(fn_double);
    module.add_or_replace(fn_main);

    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    // find the main method (second in list since we added double first)
    let main_method = artifact.methods.iter().find(|m| m.name == "main").unwrap();
    // call opcode = 0x28
    assert!(main_method.body.contains(&0x28), "expected call (0x28): {:?}", main_method.body);
}

// ===========================================================================
// lower_iir_to_cil — artifact structure
// ===========================================================================

#[test]
fn lower_entry_label_matches_module_entry_point() {
    let module = single_fn(vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    assert_eq!(artifact.entry_label, "main");
}

#[test]
fn lower_data_offsets_is_empty() {
    let module = single_fn(vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    assert!(artifact.data_offsets.is_empty());
    assert_eq!(artifact.data_size, 0);
}

#[test]
fn lower_helper_specs_is_empty() {
    let module = single_fn(vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    // IIR backend does not inject runtime helpers.
    assert!(artifact.helper_specs.is_empty());
}

#[test]
fn lower_method_local_types_matches_local_count() {
    // const a; const b; add → 3 locals (a, b, c)
    let module = single_fn(vec![
        IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i32"),
        IIRInstr::new("const", Some("b".into()), vec![Operand::Int(2)], "i32"),
        IIRInstr::new("add", Some("c".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i32"),
    ]);
    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    // a, b, c → 3 local variables (no params)
    assert_eq!(artifact.methods[0].local_types.len(), 3);
    assert!(artifact.methods[0].local_types.iter().all(|t| t == "int32"));
}

#[test]
fn lower_method_parameter_types_matches_param_count() {
    // f(x: i32, y: i32) -> i32
    let module = fn_with_params(
        vec![("x", "i32"), ("y", "i32")],
        "i32",
        vec![
            IIRInstr::new("add", Some("v".into()),
                vec![Operand::Var("x".into()), Operand::Var("y".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
        ],
    );
    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    assert_eq!(artifact.methods[0].parameter_types.len(), 2);
}

#[test]
fn lower_max_stack_is_positive() {
    let module = single_fn(vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    assert!(artifact.methods[0].max_stack > 0);
}

// ===========================================================================
// lower_iir_to_cil — error cases
// ===========================================================================

#[test]
fn lower_validation_failure_returns_err() {
    let module = IIRModule {
        name: "empty".into(),
        functions: vec![],
        entry_point: None,
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let result = lower_iir_to_cil(&module, &default_cfg());
    assert!(matches!(result, Err(IIRClrError::ValidationFailed(_))));
}

#[test]
fn lower_unsupported_op_returns_err() {
    // Call lower_iir_to_cil with an unsupported op, bypassing validation.
    // We do this by using a module whose validator would reject it, but we
    // only check that the error type is right.  The validator is called
    // inside lower_iir_to_cil, so this will return ValidationFailed.
    let module = single_fn(vec![
        IIRInstr::new("io_in", Some("v".into()), vec![], "i32"),
    ]);
    let result = lower_iir_to_cil(&module, &default_cfg());
    assert!(result.is_err());
}

#[test]
fn lower_bool_const_true_is_valid() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("t".into()), vec![Operand::Bool(true)], "bool"),
        IIRInstr::new("ret", None, vec![Operand::Var("t".into())], "bool"),
    ]);
    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    // ldc.i4.1 = 0x17
    assert!(artifact.methods[0].body.contains(&0x17),
        "expected ldc.i4.1 (0x17): {:?}", artifact.methods[0].body);
}

#[test]
fn lower_bool_const_false_is_valid() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("f".into()), vec![Operand::Bool(false)], "bool"),
        IIRInstr::new("ret", None, vec![Operand::Var("f".into())], "bool"),
    ]);
    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    // ldc.i4.0 = 0x16
    assert!(artifact.methods[0].body.contains(&0x16),
        "expected ldc.i4.0 (0x16): {:?}", artifact.methods[0].body);
}

#[test]
fn lower_const_large_int_uses_full_form() {
    // 1000 doesn't fit in a byte: should use ldc.i4.s (0x1F) or ldc.i4 (0x20)
    let module = single_fn(vec![
        IIRInstr::new("const", Some("big".into()), vec![Operand::Int(1000)], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("big".into())], "i32"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    // ldc.i4 = 0x20 (full 4-byte form for values >127)
    assert!(body.contains(&0x20), "expected ldc.i4 (0x20): {body:?}");
}

// ===========================================================================
// Phase 2 — heap op lowering tests
// ===========================================================================
//
// These tests exercise the `object[]` cons-cell encoding for heap ops:
//
//   alloc ref<LispyPair>  →  ldc.i4.2; newarr TOKEN; stloc dest
//   field_load dest p 0   →  ldloc p; ldc.i4.0; ldelem.ref; stloc dest
//   field_load dest p 1   →  ldloc p; ldc.i4.1; ldelem.ref; stloc dest
//   field_store p 0 v     →  ldloc p; ldc.i4.0; ldloc v; stelem.ref
//   is_null dest x        →  ldloc x; ldnull; ceq; stloc dest
//   const nil ref<LP>     →  ldnull; stloc dest

/// Opcodes we look for in heap-op bytecode assertions.
const NEWARR:    u8 = 0x8D; // newarr
const LDNULL:    u8 = 0x14; // ldnull
const LDELEM_REF: u8 = 0xA2; // ldelem.ref
const STELEM_REF: u8 = 0xA4; // stelem.ref

// ---------------------------------------------------------------------------
// 1. alloc ref<LispyPair> is accepted and lowers to newarr
// ---------------------------------------------------------------------------

/// `alloc ref<LispyPair>` must produce a `newarr` (0x8D) instruction.
///
/// The CLR allocates a 2-element `System.Object[]` to represent the cons cell.
/// `newarr` takes a 4-byte type token, so the body contains [0x8D, b0, b1, b2, b3].
#[test]
fn heap_alloc_listy_pair_produces_newarr() {
    let module = single_fn(vec![
        IIRInstr::new("alloc", Some("p".into()), vec![], "ref<LispyPair>"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    assert!(body.contains(&NEWARR),
        "alloc ref<LispyPair> must emit newarr (0x8D): {body:?}");
}

// ---------------------------------------------------------------------------
// 2. alloc with unsupported type is rejected by the validator
// ---------------------------------------------------------------------------

/// Any `alloc` op with a type other than `ref<LispyPair>` must be rejected.
///
/// This ensures that we don't silently miscompile `alloc ref<Foo>` by treating
/// it as a LispyPair cons cell.
#[test]
fn heap_alloc_unsupported_type_rejected() {
    let errs = validate_iir_for_clr(&single_fn(vec![
        IIRInstr::new("alloc", Some("p".into()), vec![], "ref<FooBar>"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]));
    assert!(!errs.is_empty(), "alloc ref<FooBar> must be rejected");
    assert!(errs.iter().any(|e| e.contains("UnsupportedType") || e.contains("alloc")),
        "error must mention UnsupportedType or alloc: {:?}", errs);
}

// ---------------------------------------------------------------------------
// 3. field_load 0 (car) compiles and contains ldelem.ref
// ---------------------------------------------------------------------------

/// `field_load dest pair 0` implements `car`: loads the head of the list.
///
/// CIL: `ldloc pair; ldc.i4.0; ldelem.ref; stloc dest`
///
/// `ldelem.ref` (0xA2) pops the array reference and the index, pushes the
/// element.  For a cons cell pair[0] is the head value.
#[test]
fn heap_field_load_0_car_produces_ldelem_ref() {
    let module = single_fn(vec![
        IIRInstr::new("alloc", Some("p".into()), vec![], "ref<LispyPair>"),
        // field_load: dest="h", srcs=[Var("p"), Int(0)], type="ref<LispyPair>"
        IIRInstr::new("field_load", Some("h".into()),
            vec![Operand::Var("p".into()), Operand::Int(0)], "ref<LispyPair>"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    assert!(body.contains(&LDELEM_REF),
        "field_load 0 must emit ldelem.ref (0xA2): {body:?}");
}

// ---------------------------------------------------------------------------
// 4. field_load 1 (cdr) compiles and contains ldelem.ref
// ---------------------------------------------------------------------------

/// `field_load dest pair 1` implements `cdr`: loads the tail of the list.
///
/// Same structure as car, but with index 1 instead of 0.
#[test]
fn heap_field_load_1_cdr_produces_ldelem_ref() {
    let module = single_fn(vec![
        IIRInstr::new("alloc", Some("p".into()), vec![], "ref<LispyPair>"),
        IIRInstr::new("field_load", Some("t".into()),
            vec![Operand::Var("p".into()), Operand::Int(1)], "ref<LispyPair>"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    assert!(body.contains(&LDELEM_REF),
        "field_load 1 must emit ldelem.ref (0xA2): {body:?}");
}

// ---------------------------------------------------------------------------
// 5. field_store compiles and contains stelem.ref
// ---------------------------------------------------------------------------

/// `field_store pair idx value` stores a value into a cons cell field.
///
/// CIL: `ldloc pair; ldc.i4 idx; ldloc value; stelem.ref`
#[test]
fn heap_field_store_produces_stelem_ref() {
    let module = single_fn(vec![
        IIRInstr::new("alloc", Some("p".into()), vec![], "ref<LispyPair>"),
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "i32"),
        // store 42 into field 0 of p
        IIRInstr::new("field_store", None,
            vec![Operand::Var("p".into()), Operand::Int(0), Operand::Var("v".into())],
            "ref<LispyPair>"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    assert!(body.contains(&STELEM_REF),
        "field_store must emit stelem.ref (0xA4): {body:?}");
}

// ---------------------------------------------------------------------------
// 6. is_null compiles and contains ldnull + ceq
// ---------------------------------------------------------------------------

/// `is_null dest x` lowering produces `ldnull` followed by `ceq`.
///
/// The CLR has no single opcode for null checks.  The standard pattern is:
/// ```text
/// ldloc x
/// ldnull          ; push null reference
/// ceq             ; 1 if equal (both null), 0 otherwise
/// stloc dest
/// ```
#[test]
fn heap_is_null_produces_ldnull_and_ceq() {
    let module = single_fn(vec![
        IIRInstr::new("alloc", Some("p".into()), vec![], "ref<LispyPair>"),
        IIRInstr::new("is_null", Some("b".into()),
            vec![Operand::Var("p".into())], "ref<LispyPair>"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    assert!(body.contains(&LDNULL),
        "is_null must emit ldnull (0x14): {body:?}");
    // ceq = 0xFE 0x01
    assert!(body.windows(2).any(|w| w == [PFX, 0x01]),
        "is_null must emit ceq (0xFE 0x01): {body:?}");
}

// ---------------------------------------------------------------------------
// 7. const nil ref<LispyPair> produces ldnull
// ---------------------------------------------------------------------------

/// `const nil` with type `ref<LispyPair>` and no source operand encodes nil
/// as `ldnull`.
///
/// An empty list is a null `object[]` reference.
#[test]
fn heap_const_nil_listy_pair_produces_ldnull() {
    let module = single_fn(vec![
        IIRInstr::new("const", Some("nil".into()), vec![], "ref<LispyPair>"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    assert!(body.contains(&LDNULL),
        "const nil ref<LispyPair> must emit ldnull (0x14): {body:?}");
}

// ---------------------------------------------------------------------------
// 8. alloc produces newarr with the OBJECT_ARRAY_TYPE_TOKEN in LE bytes
// ---------------------------------------------------------------------------

/// Verify that the 4-byte token following `newarr` is `OBJECT_ARRAY_TYPE_TOKEN`
/// encoded in little-endian order.
///
/// CLR simulators must see the correct sentinel to know which type to allocate.
#[test]
fn heap_alloc_newarr_token_is_correct() {
    use ir_to_cil_bytecode::OBJECT_ARRAY_TYPE_TOKEN;

    let module = single_fn(vec![
        IIRInstr::new("alloc", Some("p".into()), vec![], "ref<LispyPair>"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    // Find the newarr opcode position.
    let newarr_pos = body.iter().position(|&b| b == NEWARR)
        .expect("newarr must appear in body");
    // Read the 4-byte token immediately after newarr.
    assert!(newarr_pos + 5 <= body.len(), "body must have 4 token bytes after newarr");
    let token = u32::from_le_bytes(body[newarr_pos + 1..newarr_pos + 5].try_into().unwrap());
    assert_eq!(token, OBJECT_ARRAY_TYPE_TOKEN,
        "newarr token must be OBJECT_ARRAY_TYPE_TOKEN (0x{:08X}): {body:?}",
        OBJECT_ARRAY_TYPE_TOKEN);
}

// ---------------------------------------------------------------------------
// 9. alloc produces ldc.i4.2 before newarr (array length = 2)
// ---------------------------------------------------------------------------

/// The cons cell is a 2-element array.  `ldc.i4.2` (0x18) must appear before
/// `newarr` to push the array length onto the CIL evaluation stack.
#[test]
fn heap_alloc_pushes_length_2_before_newarr() {
    let module = single_fn(vec![
        IIRInstr::new("alloc", Some("p".into()), vec![], "ref<LispyPair>"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    // ldc.i4.2 = 0x18
    let ldc2_pos = body.iter().position(|&b| b == 0x18)
        .expect("ldc.i4.2 (0x18) must appear for array length = 2");
    let newarr_pos = body.iter().position(|&b| b == NEWARR)
        .expect("newarr must appear after ldc.i4.2");
    assert!(ldc2_pos < newarr_pos,
        "ldc.i4.2 must precede newarr: {body:?}");
}

// ---------------------------------------------------------------------------
// 10. Hand-crafted pair construction + car + is_null in one function
// ---------------------------------------------------------------------------

/// Full cons cell round-trip:
///   1. Allocate a new pair.
///   2. Store an integer in field 0 (head / car).
///   3. Store null in field 1 (tail / cdr) — makes a one-element list.
///   4. Read back field 0 (car).
///   5. Test whether the tail (field 1) is null.
///
/// This verifies that all heap ops can appear together in the same function
/// body without interfering with register allocation or branch resolution.
#[test]
fn heap_full_cons_car_isnull_in_one_function() {
    let module = single_fn(vec![
        // p = alloc()
        IIRInstr::new("alloc", Some("p".into()), vec![], "ref<LispyPair>"),
        // v = 42
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "i32"),
        // nil = null
        IIRInstr::new("const", Some("nil".into()), vec![], "ref<LispyPair>"),
        // p[0] = v
        IIRInstr::new("field_store", None,
            vec![Operand::Var("p".into()), Operand::Int(0), Operand::Var("v".into())],
            "ref<LispyPair>"),
        // p[1] = nil
        IIRInstr::new("field_store", None,
            vec![Operand::Var("p".into()), Operand::Int(1), Operand::Var("nil".into())],
            "ref<LispyPair>"),
        // head = p[0]  (car)
        IIRInstr::new("field_load", Some("head".into()),
            vec![Operand::Var("p".into()), Operand::Int(0)], "ref<LispyPair>"),
        // tail = p[1]  (cdr)
        IIRInstr::new("field_load", Some("tail".into()),
            vec![Operand::Var("p".into()), Operand::Int(1)], "ref<LispyPair>"),
        // is_tail_null = is_null(tail)
        IIRInstr::new("is_null", Some("is_tail_null".into()),
            vec![Operand::Var("tail".into())], "ref<LispyPair>"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);

    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    let body = &artifact.methods[0].body;

    // All four heap opcodes must appear in the body.
    assert!(body.contains(&NEWARR),    "newarr (0x8D) must be present: {body:?}");
    assert!(body.contains(&LDNULL),    "ldnull (0x14) must be present: {body:?}");
    assert!(body.contains(&STELEM_REF), "stelem.ref (0xA4) must be present: {body:?}");
    assert!(body.contains(&LDELEM_REF), "ldelem.ref (0xA2) must be present: {body:?}");
    // ceq from is_null
    assert!(body.windows(2).any(|w| w == [PFX, 0x01]),
        "ceq (0xFE 0x01) from is_null must be present: {body:?}");
    // ret
    assert!(body.contains(&RET), "ret must be present: {body:?}");
}

// ---------------------------------------------------------------------------
// 11. Two field_stores produce two stelem.ref opcodes
// ---------------------------------------------------------------------------

/// Building a cons cell from scratch requires two `field_store` ops (head and
/// tail), which must produce exactly two `stelem.ref` (0xA4) instructions.
#[test]
fn heap_two_field_stores_produce_two_stelem_ref() {
    let module = single_fn(vec![
        IIRInstr::new("alloc", Some("p".into()), vec![], "ref<LispyPair>"),
        IIRInstr::new("const", Some("h".into()), vec![Operand::Int(1)], "i32"),
        IIRInstr::new("const", Some("t".into()), vec![Operand::Int(2)], "i32"),
        IIRInstr::new("field_store", None,
            vec![Operand::Var("p".into()), Operand::Int(0), Operand::Var("h".into())],
            "ref<LispyPair>"),
        IIRInstr::new("field_store", None,
            vec![Operand::Var("p".into()), Operand::Int(1), Operand::Var("t".into())],
            "ref<LispyPair>"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    let stelem_count = body.iter().filter(|&&b| b == STELEM_REF).count();
    assert_eq!(stelem_count, 2,
        "two field_stores must produce two stelem.ref (0xA4): {body:?}");
}

// ---------------------------------------------------------------------------
// 12. is_null on a freshly allocated pair compiles and contains ret
// ---------------------------------------------------------------------------

/// Trivial test: allocate a pair, test is_null, and return.  Verifies the
/// function body is well-formed (has a `ret`) even with heap ops.
#[test]
fn heap_alloc_is_null_function_produces_ret() {
    let module = single_fn(vec![
        IIRInstr::new("alloc", Some("p".into()), vec![], "ref<LispyPair>"),
        IIRInstr::new("is_null", Some("b".into()),
            vec![Operand::Var("p".into())], "ref<LispyPair>"),
        IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i32"),
    ]);
    let body = &lower_iir_to_cil(&module, &default_cfg()).unwrap().methods[0].body;
    assert!(body.contains(&RET),  "function must contain ret (0x2A): {body:?}");
    assert!(body.contains(&NEWARR), "function must contain newarr (0x8D): {body:?}");
    assert!(body.contains(&LDNULL), "function must contain ldnull (0x14): {body:?}");
}

// ---------------------------------------------------------------------------
// 13. const nil validation: ref<LispyPair> with no srcs is valid
// ---------------------------------------------------------------------------

/// The validator must accept `const` with `type_hint == "ref<LispyPair>"` and
/// no source operands (this encodes the nil literal).
#[test]
fn heap_validate_const_nil_ref_listy_pair_is_valid() {
    let errs = validate_iir_for_clr(&single_fn(vec![
        IIRInstr::new("const", Some("nil".into()), vec![], "ref<LispyPair>"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]));
    assert!(errs.is_empty(), "const nil ref<LispyPair> must pass validation: {:?}", errs);
}

// ---------------------------------------------------------------------------
// 14. field_load and field_store are accepted by the validator
// ---------------------------------------------------------------------------

/// `field_load` and `field_store` with `ref<LispyPair>` must pass validation.
/// This confirms the Phase 2 promotion is reflected in the validator.
#[test]
fn heap_validate_field_ops_are_valid() {
    let errs = validate_iir_for_clr(&single_fn(vec![
        IIRInstr::new("field_load", Some("x".into()),
            vec![Operand::Var("p".into()), Operand::Int(0)], "ref<LispyPair>"),
        IIRInstr::new("field_store", None,
            vec![Operand::Var("p".into()), Operand::Int(1), Operand::Var("x".into())],
            "ref<LispyPair>"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]));
    assert!(errs.is_empty(), "field_load / field_store should pass validation: {:?}", errs);
}

// ===========================================================================
// IIRClrCodeGenerator — CodeGenerator protocol
// ===========================================================================

#[test]
fn codegen_name_is_iir_clr() {
    assert_eq!(IIRClrCodeGenerator::default_name().name(), "iir-clr");
}

#[test]
fn codegen_validate_valid_is_empty() {
    let module = single_fn(vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    assert!(IIRClrCodeGenerator::default_name().validate(&module).is_empty());
}

#[test]
fn codegen_validate_bad_module_returns_errors() {
    let module = IIRModule {
        name: "bad".into(),
        functions: vec![],
        entry_point: None,
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let errors = IIRClrCodeGenerator::default_name().validate(&module);
    assert!(!errors.is_empty());
}

#[test]
fn codegen_generate_produces_non_empty_artifact() {
    let module = single_fn(vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    let artifact: CILProgramArtifact =
        IIRClrCodeGenerator::default_name().generate(&module);
    assert!(!artifact.methods.is_empty());
    assert!(!artifact.methods[0].body.is_empty());
}

#[test]
fn codegen_default_config() {
    // IIRClrCodeGenerator::default() should work the same as default_name()
    let gen = IIRClrCodeGenerator::default();
    assert_eq!(gen.name(), "iir-clr");
    let module = single_fn(vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    assert!(gen.validate(&module).is_empty());
}

#[test]
fn codegen_custom_assembly_name() {
    let gen = IIRClrCodeGenerator::new("CalcLib");
    assert_eq!(gen.name(), "iir-clr");
    // The assembly name is stored but not surfaced in the artifact in v1;
    // we just verify construction succeeds and generate works.
    let module = single_fn(vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    let artifact = gen.generate(&module);
    assert!(!artifact.methods[0].body.is_empty());
}

// ===========================================================================
// LANG37 — CLR closure lowering tests
// ===========================================================================
//
// LANG37 promotes the CLR backend from "reject alloc_closure / call_closure
// with ClosureOpcode" (LANG35) to a full `int32[]`-based dispatch-table
// implementation:
//
//   alloc_closure(Str("fn"), Var(cap0), …) : "closure"
//     → int32[] { fn_dispatch_idx, cap0_as_i32, … }
//
//   call_closure(Var(handle), Var(arg0), …) : "any"
//     → static __callClosure(int32[], int32[])
//
// Captures are limited to i32/bool in v1; i64/f32/f64 captures still produce
// a `ClosureOpcode` validation error.
//
// The CIL opcodes involved are:
//   newarr (0x8D)    — allocate int32[]
//   stelem.i4 (0x9E) — store int32 into int32[]
//   ldelem.i4 (0x94) — load int32 from int32[]
//   call (0x28)      — call __callClosure

/// Opcode constants for LANG37 closure assertions.
const STELEM_I4: u8 = 0x9E; // stelem.i4
const LDELEM_I4: u8 = 0x94; // ldelem.i4
const CALL:      u8 = 0x28; // call (also used in existing test above)
const DUP:       u8 = 0x25; // dup

// ---------------------------------------------------------------------------
// Shared helper: build a two-function module for closure tests
//
//   __lambda_0(x: i32) -> i32  { ret x }
//   main() -> i32              { cl = alloc_closure("__lambda_0")
//                                arg = const 42
//                                r   = call_closure(cl, arg)
//                                ret r }
//
// `__lambda_0` is added first so its CIL token (0x06000001) matches
// the dispatch index 0 assigned by `collect_closure_dispatch`.
// ---------------------------------------------------------------------------

fn closure_test_module() -> IIRModule {
    let lambda = IIRFunction::new(
        "__lambda_0",
        vec![("x".to_string(), "i32".to_string())],
        "i32",
        vec![
            IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i32"),
        ],
    );
    let main_fn = IIRFunction::new(
        "main",
        vec![],
        "i32",
        vec![
            // cl = alloc_closure("__lambda_0")  — 0 captures
            IIRInstr::new("alloc_closure", Some("cl".into()),
                vec![Operand::Str("__lambda_0".into())], "closure"),
            // arg = 42
            IIRInstr::new("const", Some("arg".into()), vec![Operand::Int(42)], "i32"),
            // r = call_closure(cl, arg)
            IIRInstr::new("call_closure", Some("r".into()),
                vec![Operand::Var("cl".into()), Operand::Var("arg".into())], "any"),
            // ret r
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let mut module = IIRModule::new("closure_test", "tetrad");
    module.entry_point = Some("main".into());
    module.add_or_replace(lambda);
    module.add_or_replace(main_fn);
    module
}

// ---------------------------------------------------------------------------
// 1. alloc_closure with i32 capture is accepted by the validator (no ClosureOpcode)
// ---------------------------------------------------------------------------

/// LANG37: `alloc_closure` with an `i32` capture must pass validation.
///
/// The LANG35 blanket rejection of all `alloc_closure` instructions has been
/// lifted.  The validator now only rejects `alloc_closure` whose captures have
/// i64/u64/f32/f64 types.
#[test]
fn lang37_alloc_closure_i32_cap_accepted_by_clr_validator() {
    // main(cap: i32) → the param type feeds into var_types lookup
    let mut module = IIRModule::new("test", "tetrad");
    let fn_ = IIRFunction::new(
        "main",
        vec![("cap".to_string(), "i32".to_string())],
        "closure",
        vec![
            IIRInstr::new("alloc_closure", Some("cl".into()),
                vec![
                    Operand::Str("__lambda_0".into()),
                    Operand::Var("cap".into()),  // i32 capture
                ],
                "closure"),
            IIRInstr::new("ret", None, vec![Operand::Var("cl".into())], "closure"),
        ],
    );
    module.add_or_replace(fn_);
    module.entry_point = Some("main".into());

    let errs = validate_iir_for_clr(&module);
    assert!(
        !errs.iter().any(|e| e.contains("ClosureOpcode")),
        "i32 capture must NOT produce ClosureOpcode error; got: {errs:?}"
    );
}

// ---------------------------------------------------------------------------
// 2. call_closure is accepted by the validator (no ClosureOpcode)
// ---------------------------------------------------------------------------

/// LANG37: `call_closure` must no longer produce a `ClosureOpcode` error.
///
/// `call_closure` always has type_hint `"any"` — the validator special-cases
/// it before the `UntypedInstruction` check so it never fires either error.
#[test]
fn lang37_call_closure_accepted_by_clr_validator() {
    let module = fn_with_params(
        vec![("h", "i32"), ("a", "i32")],
        "i32",
        vec![
            IIRInstr::new("call_closure", Some("r".into()),
                vec![Operand::Var("h".into()), Operand::Var("a".into())],
                "any"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let errs = validate_iir_for_clr(&module);
    assert!(
        !errs.iter().any(|e| e.contains("ClosureOpcode")),
        "call_closure must NOT produce ClosureOpcode error; got: {errs:?}"
    );
    assert!(
        !errs.iter().any(|e| e.contains("UntypedInstruction")),
        "call_closure must NOT produce UntypedInstruction error; got: {errs:?}"
    );
}

// ---------------------------------------------------------------------------
// 3. alloc_closure with i64 capture is still rejected (deferred to LANG38)
// ---------------------------------------------------------------------------

/// LANG37: `alloc_closure` with an `i64` capture must still produce a
/// `ClosureOpcode` error in the CLR backend.
///
/// The `int32[]` closure representation can only store 32-bit values.
/// Wider captures require boxing (LANG38).
#[test]
fn lang37_i64_capture_still_rejected() {
    let mut module = IIRModule::new("test", "tetrad");
    let fn_ = IIRFunction::new(
        "main",
        vec![("cap".to_string(), "i64".to_string())],  // i64 param → i64 capture
        "closure",
        vec![
            IIRInstr::new("alloc_closure", Some("cl".into()),
                vec![
                    Operand::Str("__lambda_0".into()),
                    Operand::Var("cap".into()),  // i64 capture — must be rejected
                ],
                "closure"),
            IIRInstr::new("ret", None, vec![Operand::Var("cl".into())], "closure"),
        ],
    );
    module.add_or_replace(fn_);
    module.entry_point = Some("main".into());

    let errs = validate_iir_for_clr(&module);
    assert!(
        errs.iter().any(|e| e.contains("ClosureOpcode")),
        "i64 capture must produce ClosureOpcode error; got: {errs:?}"
    );
}

// ---------------------------------------------------------------------------
// 4. alloc_closure with f32 capture is still rejected (deferred to LANG38)
// ---------------------------------------------------------------------------

/// LANG37: `alloc_closure` with an `f32` capture must produce a `ClosureOpcode`
/// error.  Float captures are deferred to LANG38.
#[test]
fn lang37_float_capture_still_rejected() {
    let mut module = IIRModule::new("test", "tetrad");
    let fn_ = IIRFunction::new(
        "main",
        vec![("cap".to_string(), "f32".to_string())],  // f32 param → f32 capture
        "closure",
        vec![
            IIRInstr::new("alloc_closure", Some("cl".into()),
                vec![
                    Operand::Str("__lambda_0".into()),
                    Operand::Var("cap".into()),  // f32 capture — must be rejected
                ],
                "closure"),
            IIRInstr::new("ret", None, vec![Operand::Var("cl".into())], "closure"),
        ],
    );
    module.add_or_replace(fn_);
    module.entry_point = Some("main".into());

    let errs = validate_iir_for_clr(&module);
    assert!(
        errs.iter().any(|e| e.contains("ClosureOpcode")),
        "f32 capture must produce ClosureOpcode error; got: {errs:?}"
    );
}

// ---------------------------------------------------------------------------
// 5. alloc_closure lowering emits newarr (0x8D)
// ---------------------------------------------------------------------------

/// `alloc_closure` lowers to `ldc.i4 {n+1}; newarr [System.Int32]; …`.
/// The `newarr` opcode (0x8D) must appear in the caller's method body.
#[test]
fn lang37_alloc_closure_emits_newarr() {
    let module = closure_test_module();
    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    let main_body = &artifact.methods.iter()
        .find(|m| m.name == "main")
        .expect("main method must exist")
        .body;
    assert!(main_body.contains(&NEWARR),
        "alloc_closure must emit newarr (0x8D): {main_body:?}");
}

// ---------------------------------------------------------------------------
// 6. alloc_closure lowering emits stelem.i4 (0x9E)
// ---------------------------------------------------------------------------

/// Each element stored into the closure array uses `stelem.i4` (0x9E):
/// - One for the dispatch index stored at `closure[0]`.
/// - One per captured variable.
#[test]
fn lang37_alloc_closure_emits_stelem_i4() {
    let module = closure_test_module();
    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    let main_body = &artifact.methods.iter()
        .find(|m| m.name == "main")
        .expect("main method must exist")
        .body;
    assert!(main_body.contains(&STELEM_I4),
        "alloc_closure must emit stelem.i4 (0x9E): {main_body:?}");
}

// ---------------------------------------------------------------------------
// 7. call_closure emits the call opcode (0x28) targeting __callClosure
// ---------------------------------------------------------------------------

/// `call_closure` emits `call int32 ClassName::__callClosure(int32[], int32[])`.
/// The `call` opcode is 0x28.
#[test]
fn lang37_call_closure_emits_call_dispatch() {
    let module = closure_test_module();
    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    let main_body = &artifact.methods.iter()
        .find(|m| m.name == "main")
        .expect("main method must exist")
        .body;
    assert!(main_body.contains(&CALL),
        "call_closure must emit call (0x28): {main_body:?}");
}

// ---------------------------------------------------------------------------
// 8. Dispatch method __callClosure is generated when alloc_closure is present
// ---------------------------------------------------------------------------

/// When the module contains `alloc_closure`, `lower_iir_to_cil` appends a
/// synthetic `__callClosure` method after all user functions.
#[test]
fn lang37_dispatch_method_generated() {
    let module = closure_test_module();
    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    assert!(
        artifact.methods.iter().any(|m| m.name == "__callClosure"),
        "artifact must contain __callClosure method; methods: {:?}",
        artifact.methods.iter().map(|m| &m.name).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// 9. __callClosure dispatch body contains ldelem.i4 (0x94)
// ---------------------------------------------------------------------------

/// The dispatch method reads `closure[0]` to get the function index and reads
/// `args[idx]` / `closure[idx]` for arguments and captures.  All these array
/// loads use `ldelem.i4` (0x94).
#[test]
fn lang37_dispatch_method_contains_ldelem_i4() {
    let module = closure_test_module();
    let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
    let dispatch_body = &artifact.methods.iter()
        .find(|m| m.name == "__callClosure")
        .expect("__callClosure must be generated")
        .body;
    assert!(dispatch_body.contains(&LDELEM_I4),
        "__callClosure body must contain ldelem.i4 (0x94): {dispatch_body:?}");
}

// Suppress "unused" warnings for the DUP constant (defined above for completeness
// but not yet needed in a direct assertion; it is verified indirectly via stelem.i4
// sequences that require dup to prime the array reference).
#[allow(dead_code)]
const _DUP_USED: u8 = DUP;
