use intel8080_backend::{compile, BackendError, Intel8080Backend};
use intel8080_simulator::Intel8080Simulator;
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
fn backend_name_is_intel8080() {
    assert_eq!(Intel8080Backend.name(), "intel8080");
}

#[test]
#[should_panic(expected = "intel8080 backend is emit-only")]
fn backend_run_panics_per_spec() {
    Intel8080Backend.run(&[], &[]);
}

/// Twig `42` canonical: MVI A, 42 ; HLT = [0x3E, 0x2A, 0x76].
/// This is the EXACT byte sequence `lang-aot --emit=intel8080` produces
/// for the trivial IIR program `const 42; ret`.
#[test]
fn canonical_const_42_then_ret() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x3E, 0x2A, 0x76]);
}

/// Byte-for-byte parity is necessary but not sufficient — genuinely
/// execute the emitted bytes in the new simulator and check the
/// accumulator, not just assert a hand-derived byte array.
#[test]
fn canonical_const_42_then_ret_actually_executes_to_a_equals_42() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("lowering");

    let mut sim = Intel8080Simulator::new(65536);
    sim.load_program(&bytes);
    let result = sim.run_loaded_with_limit(10);
    assert!(result.halted);
    assert_eq!(result.steps, 2, "MVI A,42 then HLT is exactly two steps");
    assert_eq!(sim.regs.a, 42);
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

#[test]
fn backend_trait_compile_matches_free_function() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let via_trait = Intel8080Backend.compile(&cir).expect("trait compile");
    let via_free_fn = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("free-fn compile");
    assert_eq!(via_trait, via_free_fn);
}

/// CIR ending in `const_*` with NO following `ret_*` must still be
/// terminated: `bytes.is_empty()` is false after emitting `MVI A,n`
/// (2 bytes), so an `is_empty`-based check would wrongly conclude the
/// program is already terminated and skip appending `HLT`, leaving
/// the compiled program to fall into whatever follows in memory (0x00
/// decodes as NOP on the 8080) instead of halting.
#[test]
fn dangling_const_with_no_ret_still_gets_a_real_terminator() {
    let cir = vec![ci("const_i64", Some("v"), vec![CIROperand::Int(7)], "i64")];
    let bytes = compile(&ctx("dangling_const", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x3E, 0x07, 0x76]);

    let mut sim = Intel8080Simulator::new(64);
    sim.load_program(&bytes);
    let result = sim.run_loaded_with_limit(1000);
    assert!(result.halted, "program should halt, not run out the step budget");
    assert!(result.steps < 1000, "should halt well before the step limit");
}
