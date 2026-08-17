use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use mos6502_backend::{compile, BackendError, Mos6502Backend};
use mos6502_simulator::Mos6502Simulator;

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
fn empty_cir_emits_brk() {
    let bytes = compile(&ctx("empty", &[], "void"), &[]).expect("lowering");
    assert_eq!(bytes, vec![0x00]);
}

#[test]
fn backend_name_is_mos6502() {
    assert_eq!(Mos6502Backend.name(), "mos6502");
}

#[test]
#[should_panic(expected = "mos6502 backend is emit-only")]
fn backend_run_panics_per_spec() {
    Mos6502Backend.run(&[], &[]);
}

/// `const_i64 v=42; ret_i64 v` -> `LDA #42; BRK` = `[0xA9, 0x2A, 0x00]`.
///
/// This is the EXACT byte sequence `lang-aot --emit=mos6502` produces for
/// the trivial IIR program `const 42; ret`.
#[test]
fn canonical_const_42_then_ret() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xA9, 0x2A, 0x00]);
}

/// Byte-for-byte parity is necessary but not sufficient -- genuinely
/// execute the emitted bytes in the new simulator and check the
/// accumulator, not just assert a hand-derived byte array.
#[test]
fn canonical_const_42_then_ret_actually_executes_to_a_equals_42() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("lowering");

    let mut sim = Mos6502Simulator::new(65536);
    sim.load_program(&bytes);
    // Two instructions: LDA #42 (materialises 42 into A) then BRK, which
    // mos6502-simulator's execute() intercepts to set halted = true and
    // stop the fetch-decode-execute loop.
    let result = sim.run_loaded_with_limit(10);
    assert_eq!(result.steps, 2);
    assert_eq!(sim.a, 42);
    assert!(result.halted);
    assert!(sim.halted);
}

#[test]
fn const_zero_then_ret_emits_three_bytes() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(0)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("zero", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xA9, 0x00, 0x00], "LDA #0; BRK");
}

#[test]
fn const_max_8bit_immediate_works() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(255)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("max", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes[0..2], [0xA9, 0xFF], "LDA #255");
}

#[test]
fn const_negative_out_of_range_errors() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(-1)], "i64"),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("neg", &[], "void"), &cir)
        .expect_err("-1 is below the unsigned 8-bit LDA immediate range");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(-1)));
}

#[test]
fn const_over_255_out_of_range_errors() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(256)], "i64"),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("big", &[], "void"), &cir)
        .expect_err("256 overflows the unsigned 8-bit LDA immediate");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(256)));
}

#[test]
fn ret_void_alone_emits_just_brk() {
    let cir = vec![ci("ret_void", None, vec![], "void")];
    let bytes = compile(&ctx("noop", &[], "void"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x00]);
}

#[test]
fn const_bool_true() {
    let cir = vec![
        ci("const_bool", Some("b"), vec![CIROperand::Bool(true)], "bool"),
        ci("ret_bool", None, vec![CIROperand::Var("b".into())], "bool"),
    ];
    let bytes = compile(&ctx("btrue", &[], "bool"), &cir).expect("lowering");
    assert_eq!(bytes[0..2], [0xA9, 0x01], "LDA #1");
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
    // Two consts where ret targets the first -- currently unsupported
    // since v0.1.0 only handles single-var-in-accumulator case.
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(1)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(2)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("a".into())], "i64"),
    ];
    // Should produce LDA #1 ; LDA #2 (b clobbers a) ; then ret tries to
    // return 'a' which isn't in the accumulator anymore -- error.
    let err = compile(&ctx("two_const_ret_first", &[], "i64"), &cir)
        .expect_err("multi-var ret should fall through");
    assert!(matches!(err, BackendError::UnsupportedOp(_)));
}

#[test]
fn backend_trait_compile_matches_free_function() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let via_trait = Mos6502Backend.compile(&cir).expect("trait compile");
    let via_free_fn = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("free-fn compile");
    assert_eq!(via_trait, via_free_fn);
}

/// CIR ending in `const_*` with NO following `ret_*` must still be
/// terminated by a REAL emitted `BRK`, not by coincidentally falling
/// into zero-filled memory that happens to decode as `BRK` too. This
/// specifically exercises `const 0`: `LDA #0` ends in a byte (0x00)
/// numerically identical to `BRK`'s own opcode, which a trailing-byte
/// comparison (the bug this check replaced, and the same class of bug
/// found and fixed in the Intel 8051 and Intel 8080 lanes of this
/// campaign) would misread as "already terminated" and skip appending
/// the real BRK.
#[test]
fn dangling_const_zero_with_no_ret_still_gets_a_real_terminator() {
    let cir = vec![ci("const_i64", Some("v"), vec![CIROperand::Int(0)], "i64")];
    let bytes = compile(&ctx("dangling_const_zero", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xA9, 0x00, 0x00], "LDA #0 then a real BRK");

    let mut sim = Mos6502Simulator::new(65536);
    sim.load_program(&bytes);
    let result = sim.run_loaded_with_limit(1000);
    assert!(result.halted, "program should halt, not run out the step budget");
    assert_eq!(result.steps, 2, "should halt after exactly LDA + the real BRK");
}
