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
    let errs = validate_iir_for_clr(&single_fn(vec![
        IIRInstr::new("alloc", Some("v".into()), vec![Operand::Int(4)], "ref<i32>"),
    ]));
    assert!(errs.iter().any(|e| e.contains("UnsupportedOp")));
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
