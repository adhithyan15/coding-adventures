use intel8051_backend::{compile, BackendError, Intel8051Backend};
use intel8051_simulator::Intel8051Simulator;
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
fn empty_cir_emits_halt() {
    let bytes = compile(&ctx("empty", &[], "void"), &[]).expect("lowering");
    assert_eq!(bytes, vec![0xA5]);
}

#[test]
fn backend_name_is_intel8051() {
    assert_eq!(Intel8051Backend.name(), "intel8051");
}

#[test]
#[should_panic(expected = "intel8051 backend is emit-only")]
fn backend_run_panics_per_spec() {
    Intel8051Backend.run(&[], &[]);
}

/// `const_i64 v=42; ret_i64 v` -> `MOV A, #42; HALT` =
/// `[0x74, 0x2A, 0xA5]`.  This is the EXACT byte sequence
/// `lang-aot --emit=intel8051` produces for the trivial IIR program
/// `const 42; ret`.
#[test]
fn canonical_const_42_then_ret() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x74, 0x2A, 0xA5]);
}

/// Byte-for-byte parity is necessary but not sufficient -- genuinely
/// execute the emitted bytes in the Intel 8051 simulator and check
/// the accumulator + halted state, not just assert a hand-derived
/// byte array.
///
/// This also proves the HALT sentinel (`0xA5`) is recognised and
/// stops the fetch-decode-execute loop within a bounded step count --
/// `run_loaded_with_limit`'s `max_steps` guard is exercised, not
/// relied upon to catch a runaway (a 2-instruction program that
/// genuinely halts always finishes in exactly 2 steps).
#[test]
fn canonical_const_42_then_ret_actually_executes_to_acc_equals_42() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("lowering");

    let mut sim = Intel8051Simulator::new();
    sim.load_program(&bytes, 0);
    let result = sim.run_loaded_with_limit(100);
    assert!(result.halted, "HALT sentinel must stop the fetch-decode-execute loop");
    assert_eq!(result.steps, 2, "MOV A,#imm then HALT is exactly 2 instructions");
    assert_eq!(sim.acc(), 42);
}

/// A backend that emitted an infinite loop instead of a real HALT
/// would never set `halted`, and `run_loaded_with_limit` would burn
/// through every step of its budget.  Prove the *converse* holds for
/// our HALT-sentinel convention: a genuinely unterminated program
/// (raw `SJMP $`, bypassing the backend entirely) does NOT halt and
/// DOES exhaust the step budget -- so `canonical_const_42_then_ret`'s
/// `result.halted == true` is a meaningful assertion, not a vacuous
/// one.
#[test]
fn sjmp_self_loop_does_not_halt_within_step_budget() {
    let mut sim = Intel8051Simulator::new();
    sim.load_program(&[0x80, 0xFE], 0); // SJMP $ (rel = -2)
    let result = sim.run_loaded_with_limit(50);
    assert!(!result.halted);
    assert_eq!(result.steps, 50);
}

#[test]
fn const_zero_then_ret_emits_three_bytes() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(0)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("zero", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x74, 0x00, 0xA5]);
}

#[test]
fn const_max_8bit_immediate_works() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(255)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("max", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x74, 0xFF, 0xA5]);
}

#[test]
fn const_negative_out_of_range_errors() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(-1)], "i64"),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("neg", &[], "void"), &cir)
        .expect_err("-1 is below the unsigned 8-bit MOV A,#imm range");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(-1)));
}

#[test]
fn const_over_255_out_of_range_errors() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(256)], "i64"),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("big", &[], "void"), &cir)
        .expect_err("256 overflows the unsigned 8-bit MOV A,#imm range");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(256)));
}

#[test]
fn ret_void_alone_emits_just_halt() {
    let cir = vec![ci("ret_void", None, vec![], "void")];
    let bytes = compile(&ctx("noop", &[], "void"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xA5]);
}

#[test]
fn const_bool_true() {
    let cir = vec![
        ci("const_bool", Some("b"), vec![CIROperand::Bool(true)], "bool"),
        ci("ret_bool", None, vec![CIROperand::Var("b".into())], "bool"),
    ];
    let bytes = compile(&ctx("btrue", &[], "bool"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x74, 0x01, 0xA5]);
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
    // since v0.1.0 only handles the single-var-in-A case.
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(1)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(2)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("a".into())], "i64"),
    ];
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
    let via_trait = Intel8051Backend.compile(&cir).expect("trait compile");
    let via_free_fn = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("free-fn compile");
    assert_eq!(via_trait, via_free_fn);
}

/// `const_i64 v=165` with NO following `ret` -- 165 (0xA5) is the
/// HALT sentinel's own byte value, so `MOV A, #165` ends in a byte
/// that is numerically identical to `encode_halt()`. A trailing-byte
/// comparison (the original, buggy defensive-termination check) would
/// be fooled into believing a real HALT was already emitted and skip
/// appending one, leaving the compiled program unterminated. This
/// proves the fix: the real terminator is still appended, so the
/// output is `[0x74, 0xA5, 0xA5]` (`MOV A, #0xA5` + the actual HALT),
/// not just `[0x74, 0xA5]`.
#[test]
fn const_matching_halt_sentinel_byte_value_still_gets_a_real_terminator() {
    let cir = vec![ci(
        "const_i64",
        Some("v"),
        vec![CIROperand::Int(0xA5)],
        "i64",
    )];
    let bytes = compile(&ctx("halt_byte_collision", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x74, 0xA5, 0xA5]);

    // And prove it actually halts when run, rather than spinning
    // through zeroed-memory NOPs until the simulator's step budget
    // is exhausted.
    let mut sim = Intel8051Simulator::new();
    sim.load_program(&bytes, 0);
    let result = sim.run_loaded_with_limit(1000);
    assert!(result.halted, "program should halt, not run out the step budget");
    assert!(result.steps < 1000, "should halt well before the step limit");
}
