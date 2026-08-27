//! Byte-pinning tests for `ibm704-backend` v0.2.0.
//!
//! Every emitted byte sequence the lang-aot IBM 704 e2e smoke test
//! pins is asserted here as a unit-level regression invariant.

use ibm704_backend::{compile, BackendError, Ibm704Backend};
use ibm704_encoder::{encode_cla, unpack_words};
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
fn empty_cir_emits_canonical_halt() {
    // Empty body falls through to a bare `HTR 0` halt sentinel.
    let bytes = compile(&ctx("empty", &[], "void"), &[]).expect("lowering");
    assert_eq!(bytes, vec![0; 5]);
}

#[test]
fn backend_name_is_ibm704() {
    assert_eq!(Ibm704Backend.name(), "ibm704");
}

#[test]
#[should_panic(expected = "ibm704 backend is emit-only")]
fn backend_run_panics_per_spec() {
    Ibm704Backend.run(&[], &[]);
}

/// Twig `42` canonical: `CLA 2; HTR 0; +42`. The last word is a literal-pool
/// entry because CLA's operand is an address, not an immediate.
#[test]
fn canonical_const_42_then_ret_twig_42() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x40, 0x00, 0x00, 0x02, // CLA 2
            0x00, 0x00, 0x00, 0x00, 0x00, // HTR 0
            0x00, 0x00, 0x00, 0x00, 0x2A, // +42 literal
        ]
    );
    assert_eq!(unpack_words(&bytes).unwrap(), vec![encode_cla(2), 0, 42]);
}

#[test]
fn const_zero_then_ret() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(0)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("zero", &[], "i64"), &cir).expect("lowering");
    assert_eq!(unpack_words(&bytes).unwrap(), vec![encode_cla(2), 0, 0]);
}

#[test]
fn const_bool_true_acts_as_imm_one() {
    let cir = vec![
        ci(
            "const_bool",
            Some("b"),
            vec![CIROperand::Bool(true)],
            "bool",
        ),
        ci("ret_bool", None, vec![CIROperand::Var("b".into())], "bool"),
    ];
    let bytes = compile(&ctx("btrue", &[], "bool"), &cir).expect("lowering");
    assert_eq!(unpack_words(&bytes).unwrap(), vec![encode_cla(2), 0, 1]);
}

#[test]
fn const_max_15bit_is_accepted() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(32767)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("max", &[], "i64"), &cir).expect("lowering");
    assert_eq!(unpack_words(&bytes).unwrap(), vec![encode_cla(2), 0, 32767]);
}

#[test]
fn const_out_of_range_errors() {
    // 32768 exceeds the 15-bit CLA immediate window.
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(32768)], "i64"),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("big", &[], "void"), &cir).expect_err("32768 overflows 15-bit CLA imm");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(32768)));
}

#[test]
fn negative_const_errors() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(-1)], "i64"),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("neg", &[], "void"), &cir).expect_err("negative");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(-1)));
}

#[test]
fn ret_void_alone_emits_just_halt() {
    let cir = vec![ci("ret_void", None, vec![], "void")];
    let bytes = compile(&ctx("noop", &[], "void"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0; 5]);
}

#[test]
fn unsupported_op_reports_unsupportedop() {
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
fn multi_const_ret_first_falls_through_to_unsupported() {
    // Two distinct const vars — the v0.1.0 backend only tracks
    // the last-loaded one, so `ret a` (when `b` is current) is
    // an UnsupportedOp.
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(1)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(2)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("a".into())], "i64"),
    ];
    let err =
        compile(&ctx("multi", &[], "i64"), &cir).expect_err("multi-var ret should fall through");
    assert!(matches!(err, BackendError::UnsupportedOp(_)));
}

#[test]
fn multiple_constants_receive_distinct_literal_pool_addresses() {
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(1)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(2)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("b".into())], "i64"),
    ];
    let bytes = compile(&ctx("multi", &[], "i64"), &cir).expect("lowering");

    assert_eq!(
        unpack_words(&bytes).unwrap(),
        vec![encode_cla(3), encode_cla(4), 0, 1, 2]
    );
}

#[test]
fn literal_pool_cannot_wrap_the_15_bit_address_space() {
    let mut cir = Vec::with_capacity(16_386);
    for _ in 0..16_385 {
        cir.push(ci("const_i64", Some("v"), vec![CIROperand::Int(1)], "i64"));
    }
    cir.push(ci(
        "ret_i64",
        None,
        vec![CIROperand::Var("v".into())],
        "i64",
    ));

    assert_eq!(
        compile(&ctx("too_large", &[], "i64"), &cir),
        Err(BackendError::ProgramTooLarge(32_771))
    );
}
