// Tests for intel4004-backend.
//
// Pin the same byte sequences iir-to-intel4004 v0.3.0 emitted, but
// built from CIR (`const_i64`/`ret_i64`/etc.) instead of IIR.

use intel4004_backend::{compile, BackendError, Intel4004Backend};
use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};

fn ctx<'a>(name: &'a str, params: &'a [(String, String)], ret_ty: &'a str) -> FunctionContext<'a> {
    FunctionContext {
        name,
        params,
        return_type: ret_ty,
    }
}

fn ci(op: &str, dest: Option<&str>, srcs: Vec<CIROperand>, ty: &str) -> CIRInstr {
    CIRInstr::new(op, dest, srcs, ty)
}

#[test]
fn empty_cir_emits_halt_loop() {
    let bytes = compile(&ctx("empty", &[], "void"), &[]).expect("lowering");
    assert_eq!(bytes, vec![0x40, 0x00]);
}

#[test]
fn backend_name_is_intel4004() {
    assert_eq!(Intel4004Backend.name(), "intel4004");
}

#[test]
fn backend_compile_returns_some_on_valid_input() {
    let cir = vec![ci("ret_void", None, vec![], "void")];
    assert_eq!(
        Intel4004Backend.compile(&cir),
        Some(vec![0x40, 0x00])
    );
}

#[test]
fn backend_compile_returns_none_on_unsupported_op() {
    let cir = vec![ci("add_i64", Some("z"), vec![], "i64")];
    assert_eq!(Intel4004Backend.compile(&cir), None);
}

#[test]
#[should_panic(expected = "intel4004 backend is emit-only")]
fn backend_run_panics_per_spec() {
    Intel4004Backend.run(&[0x40, 0x00], &[]);
}

/// `const_i64 v=5; ret_i64 v` → `LDM 5; JUN 0x000` = `[0xD5, 0x40, 0x00]`.
///
/// This is the canonical 3-byte ROM the `iir-to-intel4004` smoke
/// test in lang-aot pins.
#[test]
fn trivial_const_5_then_ret_emits_three_bytes() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(5)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("five", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xD5, 0x40, 0x00]);
}

#[test]
fn trivial_const_zero_then_ret_emits_three_bytes() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(0)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("zero", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xD0, 0x40, 0x00]);
}

#[test]
fn trivial_const_max_4bit_then_ret_emits_three_bytes() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(15)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("max", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xDF, 0x40, 0x00]);
}

#[test]
fn trivial_const_negative_one_uses_twos_complement() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(-1)], "i64"),
        ci("ret_void", None, vec![], "void"),
    ];
    let bytes = compile(&ctx("neg", &[], "void"), &cir).expect("lowering");
    // -1 → 0xF nibble → LDM 0xF = 0xDF
    assert_eq!(bytes, vec![0xDF, 0x40, 0x00]);
}

#[test]
fn const_out_of_range_errors() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(16)], "i64"),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("big", &[], "void"), &cir)
        .expect_err("16 overflows the 4-bit nibble");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(16)));
}

#[test]
fn trivial_const_bool_true() {
    let cir = vec![
        ci("const_bool", Some("b"), vec![CIROperand::Bool(true)], "bool"),
        ci("ret_bool", None, vec![CIROperand::Var("b".into())], "bool"),
    ];
    let bytes = compile(&ctx("btrue", &[], "bool"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xD1, 0x40, 0x00]);
}

/// Multiple consts trigger the eviction pattern: first const lives
/// in ACC, second evicts the first to r0 via XCH r0, then LDM.
#[test]
fn two_consts_with_ret_of_second_evicts_first() {
    // const a=1; const b=2; ret_i64 b
    //   LDM 1     (a in ACC)
    //   XCH r0    (evict a → r0)
    //   LDM 2     (b in ACC)
    //   JUN 0x000 (ret b — b in ACC, no LD needed)
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(1)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(2)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("b".into())], "i64"),
    ];
    let bytes = compile(&ctx("two", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xD1, 0xB0, 0xD2, 0x40, 0x00]);
}

#[test]
fn ret_of_evicted_var_emits_ld_to_reload() {
    // const a=3; const b=4; ret_i64 a
    //   LDM 3, XCH r0, LDM 4, LD r0, JUN 0x000
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(3)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(4)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("a".into())], "i64"),
    ];
    let bytes = compile(&ctx("ret_a", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xD3, 0xB0, 0xD4, 0xA0, 0x40, 0x00]);
}

#[test]
fn ret_void_only_emits_just_halt_loop() {
    let cir = vec![ci("ret_void", None, vec![], "void")];
    let bytes = compile(&ctx("noop", &[], "void"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x40, 0x00]);
}

#[test]
fn ret_of_undefined_variable_errors() {
    let cir = vec![ci(
        "ret_i64",
        None,
        vec![CIROperand::Var("never_defined".into())],
        "i64",
    )];
    let err = compile(&ctx("bad", &[], "i64"), &cir)
        .expect_err("ret of undefined var must error");
    assert!(matches!(err, BackendError::UndefinedVariable(s) if s == "never_defined"));
}

#[test]
fn mov_emits_ld_plus_xch() {
    // const a=5; mov b, a; ret_i64 b
    //   LDM 5 (a in ACC)
    //   XCH r0 (evict a to r0 for mov source)
    //   LD r0 (ACC ← a)
    //   XCH r1 (b = ACC, ACC = old r1 garbage)
    //   LD r1 (ACC ← b for ret)
    //   JUN 0x000
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(5)], "i64"),
        ci("mov_i64", Some("b"), vec![CIROperand::Var("a".into())], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("b".into())], "i64"),
    ];
    let bytes = compile(&ctx("mov", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xD5, 0xB0, 0xA0, 0xB1, 0xA1, 0x40, 0x00]);
}

#[test]
fn unsupported_add_returns_err() {
    let cir = vec![ci(
        "add_i64",
        Some("c"),
        vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
        "i64",
    )];
    let err = compile(&ctx("addtest", &[], "i64"), &cir).expect_err("add not supported");
    assert!(matches!(err, BackendError::UnsupportedOp(s) if s == "add_i64"));
}
