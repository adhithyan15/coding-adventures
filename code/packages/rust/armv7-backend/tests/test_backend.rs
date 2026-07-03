use armv7_backend::{compile, BackendError, Armv7Backend};
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
fn empty_cir_emits_bx_lr() {
    let bytes = compile(&ctx("empty", &[], "void"), &[]).expect("lowering");
    // BX LR = 0xE12F_FF1E, little-endian = 1E FF 2F E1
    assert_eq!(bytes, vec![0x1E, 0xFF, 0x2F, 0xE1]);
}

#[test]
fn backend_name_is_armv7() {
    assert_eq!(Armv7Backend.name(), "armv7");
}

#[test]
#[should_panic(expected = "armv7 backend is emit-only")]
fn backend_run_panics_per_spec() {
    Armv7Backend.run(&[], &[]);
}

/// `const_i64 v=42; ret_i64 v` → `MOV r0, #42; BX LR` =
/// `0xE3A0_002A 0xE12F_FF1E`.  Little-endian: `2A 00 A0 E3 1E FF 2F E1`.
///
/// This is the EXACT byte sequence the lang-aot ARMv7 e2e smoke
/// test pins.  Byte-for-byte parity with iir-to-armv7 v0.4.6.
#[test]
fn canonical_const_42_then_ret() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x2A, 0x00, 0xA0, 0xE3, // MOV r0, #42  (LE)
            0x1E, 0xFF, 0x2F, 0xE1, // BX LR        (LE)
        ]
    );
}

#[test]
fn const_zero_then_ret_emits_eight_bytes() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(0)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("zero", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes.len(), 8);
    assert_eq!(bytes[0..4], [0x00, 0x00, 0xA0, 0xE3], "MOV r0, #0");
    assert_eq!(bytes[4..8], [0x1E, 0xFF, 0x2F, 0xE1], "BX LR");
}

#[test]
fn const_max_8bit_immediate_works() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(255)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("max", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes[0..4], [0xFF, 0x00, 0xA0, 0xE3], "MOV r0, #255");
}

#[test]
fn const_out_of_range_errors() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(256)], "i64"),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("big", &[], "void"), &cir).expect_err("256 overflows 8-bit MOV imm");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(256)));
}

#[test]
fn ret_void_alone_emits_just_bx_lr() {
    let cir = vec![ci("ret_void", None, vec![], "void")];
    let bytes = compile(&ctx("noop", &[], "void"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x1E, 0xFF, 0x2F, 0xE1]);
}

#[test]
fn const_bool_true() {
    let cir = vec![
        ci("const_bool", Some("b"), vec![CIROperand::Bool(true)], "bool"),
        ci("ret_bool", None, vec![CIROperand::Var("b".into())], "bool"),
    ];
    let bytes = compile(&ctx("btrue", &[], "bool"), &cir).expect("lowering");
    assert_eq!(bytes[0..4], [0x01, 0x00, 0xA0, 0xE3], "MOV r0, #1");
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
fn multi_const_ret_falls_through_to_unsupported() {
    // Two consts where ret targets the first — currently unsupported
    // since v0.1.0 only handles single-var-in-r0 case.
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(1)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(2)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("a".into())], "i64"),
    ];
    // Should produce MOV r0, #1 ; MOV r0, #2 (b clobbers a) ;
    // then ret tries to return 'a' which isn't in r0 — error.
    let err = compile(&ctx("two_const_ret_first", &[], "i64"), &cir)
        .expect_err("multi-var ret should fall through");
    assert!(matches!(err, BackendError::UnsupportedOp(_)));
}
