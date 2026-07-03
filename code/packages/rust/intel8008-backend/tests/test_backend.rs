use intel8008_backend::{compile, BackendError, Intel8008Backend};
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
fn empty_cir_emits_hlt() {
    let bytes = compile(&ctx("empty", &[], "void"), &[]).expect("lowering");
    assert_eq!(bytes, vec![0x76]);
}

#[test]
fn backend_name_is_intel8008() {
    assert_eq!(Intel8008Backend.name(), "intel8008");
}

#[test]
#[should_panic(expected = "intel8008 backend is emit-only")]
fn backend_run_panics_per_spec() {
    Intel8008Backend.run(&[], &[]);
}

/// Twig `42` canonical: MVI A, 42 ; HLT = [0x3E, 0x2A, 0x76].
/// This is the EXACT byte sequence the lang-aot Intel 8008 e2e
/// smoke test pins.  Byte-for-byte parity with iir-to-intel8008
/// v0.3.9.
#[test]
fn canonical_const_42_then_ret() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x3E, 0x2A, 0x76]);
}

#[test]
fn const_zero_then_ret() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(0)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("z", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x3E, 0x00, 0x76]);
}

#[test]
fn const_max_8bit() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(255)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("m", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x3E, 0xFF, 0x76]);
}

#[test]
fn const_out_of_range_errors() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(256)], "i64"),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("big", &[], "void"), &cir).expect_err("256 overflows 8-bit MVI imm");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(256)));
}

#[test]
fn ret_void_alone_emits_just_hlt() {
    let cir = vec![ci("ret_void", None, vec![], "void")];
    let bytes = compile(&ctx("noop", &[], "void"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x76]);
}

#[test]
fn const_bool_true() {
    let cir = vec![
        ci("const_bool", Some("b"), vec![CIROperand::Bool(true)], "bool"),
        ci("ret_bool", None, vec![CIROperand::Var("b".into())], "bool"),
    ];
    let bytes = compile(&ctx("btrue", &[], "bool"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x3E, 0x01, 0x76]);
}

#[test]
fn unsupported_op_returns_err() {
    let cir = vec![ci(
        "add_i64",
        Some("c"),
        vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
        "i64",
    )];
    let err = compile(&ctx("addtest", &[], "i64"), &cir).expect_err("add not yet supported");
    assert!(matches!(err, BackendError::UnsupportedOp(s) if s == "add_i64"));
}

#[test]
fn multi_const_ret_falls_through() {
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(1)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(2)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("a".into())], "i64"),
    ];
    let err = compile(&ctx("two_const_ret_first", &[], "i64"), &cir)
        .expect_err("multi-var ret should fall through");
    assert!(matches!(err, BackendError::UnsupportedOp(_)));
}
