//! Byte-pinning tests for `riscv-backend` v0.1.0.
//!
//! Every emitted byte sequence the lang-aot RV32I e2e smoke test
//! pins is asserted here as a unit-level regression invariant.

use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use riscv_backend::{compile, BackendError, Riscv32Backend};

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
fn empty_cir_emits_canonical_ret() {
    // Empty body falls through to a bare `jalr x0, x1, 0`.
    let bytes = compile(&ctx("empty", &[], "void"), &[]).expect("lowering");
    assert_eq!(bytes, vec![0x67, 0x80, 0x00, 0x00]);
}

#[test]
fn backend_name_is_riscv32() {
    assert_eq!(Riscv32Backend.name(), "riscv32");
}

#[test]
#[should_panic(expected = "riscv32 backend is emit-only")]
fn backend_run_panics_per_spec() {
    Riscv32Backend.run(&[], &[]);
}

/// Twig `42` canonical:
///   addi t0, x0, 42      ; 0x02A0_0293
///   addi a0, t0, 0       ; 0x0002_8513
///   jalr x0, x1, 0       ; 0x0000_8067
///
/// Stored little-endian on disk:
///   [0x93, 0x02, 0xA0, 0x02,
///    0x13, 0x85, 0x02, 0x00,
///    0x67, 0x80, 0x00, 0x00]
///
/// This is the EXACT byte sequence the lang-aot RV32I e2e smoke
/// test pins for the Twig `42` program.  Byte-for-byte parity
/// invariant against any future encoder drift.
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
            0x93, 0x02, 0xA0, 0x02, // addi t0, x0, 42
            0x13, 0x85, 0x02, 0x00, // addi a0, t0, 0  (mv a0, t0)
            0x67, 0x80, 0x00, 0x00, // jalr x0, x1, 0  (ret)
        ]
    );
}

#[test]
fn const_zero_then_ret_emits_addi_zero() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(0)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("zero", &[], "i64"), &cir).expect("lowering");
    // addi t0, x0, 0 = (0 << 20) | (0 << 15) | (0 << 12) | (5 << 7) | 0x13
    //                = 0x0000_0293
    // addi a0, t0, 0 = 0x0002_8513
    // jalr x0, x1, 0 = 0x0000_8067
    assert_eq!(
        bytes,
        vec![
            0x93, 0x02, 0x00, 0x00,
            0x13, 0x85, 0x02, 0x00,
            0x67, 0x80, 0x00, 0x00,
        ]
    );
}

#[test]
fn const_bool_true_acts_as_imm_one() {
    let cir = vec![
        ci("const_bool", Some("b"), vec![CIROperand::Bool(true)], "bool"),
        ci("ret_bool", None, vec![CIROperand::Var("b".into())], "bool"),
    ];
    let bytes = compile(&ctx("btrue", &[], "bool"), &cir).expect("lowering");
    // addi t0, x0, 1 = 0x0010_0293 → [0x93, 0x02, 0x10, 0x00]
    assert_eq!(
        bytes,
        vec![
            0x93, 0x02, 0x10, 0x00,
            0x13, 0x85, 0x02, 0x00,
            0x67, 0x80, 0x00, 0x00,
        ]
    );
}

#[test]
fn const_negative_imm_in_range() {
    // imm = -1 is within `[-2048, 2047]`.
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(-1)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("neg", &[], "i64"), &cir).expect("lowering");
    // addi t0, x0, -1:  imm[11:0] for -1 is 0xFFF, so word
    //   = (0xFFF << 20) | 0 | 0 | (5 << 7) | 0x13
    //   = 0xFFF0_0293
    // little-endian: [0x93, 0x02, 0xF0, 0xFF]
    assert_eq!(&bytes[0..4], &[0x93, 0x02, 0xF0, 0xFF]);
    // and ends with mv + ret
    assert_eq!(&bytes[4..], &[0x13, 0x85, 0x02, 0x00, 0x67, 0x80, 0x00, 0x00]);
}

#[test]
fn const_out_of_range_errors() {
    // imm=2048 is outside the 12-bit signed `addi` window.
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(2048)], "i64"),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("big", &[], "void"), &cir)
        .expect_err("2048 overflows 12-bit signed addi imm");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(2048)));
}

#[test]
fn ret_void_alone_is_just_ret() {
    let cir = vec![ci("ret_void", None, vec![], "void")];
    let bytes = compile(&ctx("noop", &[], "void"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x67, 0x80, 0x00, 0x00]);
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
fn multi_const_uses_distinct_temps() {
    // Two distinct vars get TEMP_REGISTERS[0] (t0=5) and
    // TEMP_REGISTERS[1] (t1=6).
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(1)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(2)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("b".into())], "i64"),
    ];
    let bytes = compile(&ctx("two", &[], "i64"), &cir).expect("lowering");
    // addi t0, x0, 1  = 0x0010_0293  → [0x93, 0x02, 0x10, 0x00]
    // addi t1, x0, 2  = 0x0020_0313  → [0x13, 0x03, 0x20, 0x00]
    // addi a0, t1, 0  = 0x0003_0513  → [0x13, 0x05, 0x03, 0x00]
    // jalr x0, x1, 0  = 0x0000_8067  → [0x67, 0x80, 0x00, 0x00]
    assert_eq!(
        bytes,
        vec![
            0x93, 0x02, 0x10, 0x00,
            0x13, 0x03, 0x20, 0x00,
            0x13, 0x05, 0x03, 0x00,
            0x67, 0x80, 0x00, 0x00,
        ]
    );
}

#[test]
fn out_of_registers_after_seven_consts() {
    // 8 distinct vars exceeds TEMP_REGISTERS.len() == 7.
    let cir: Vec<CIRInstr> = (0..8)
        .map(|i| {
            let name = format!("v{i}");
            ci(
                "const_i64",
                Some(&name),
                vec![CIROperand::Int(i as i64)],
                "i64",
            )
        })
        .collect();
    let err = compile(&ctx("toomany", &[], "void"), &cir)
        .expect_err("8 distinct vars exhausts 7-temp pool");
    assert!(matches!(err, BackendError::OutOfRegisters));
}

#[test]
fn ret_undefined_var_errors() {
    let cir = vec![ci(
        "ret_i64",
        None,
        vec![CIROperand::Var("nope".into())],
        "i64",
    )];
    let err = compile(&ctx("bad_ret", &[], "i64"), &cir).expect_err("undef var");
    assert!(matches!(err, BackendError::UndefinedVariable(s) if s == "nope"));
}
