use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use sparc_v8_backend::{compile, BackendError, SparcV8Backend};
use sparc_v8_simulator::SparcV8Simulator;

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
fn empty_cir_emits_ta_zero() {
    let bytes = compile(&ctx("empty", &[], "void"), &[]).expect("lowering");
    // ta 0 = 0x91D0_2000, big-endian = 91 D0 20 00
    assert_eq!(bytes, vec![0x91, 0xD0, 0x20, 0x00]);
}

#[test]
fn backend_name_is_sparc_v8() {
    assert_eq!(SparcV8Backend.name(), "sparc-v8");
}

#[test]
#[should_panic(expected = "sparc-v8 backend is emit-only")]
fn backend_run_panics_per_spec() {
    SparcV8Backend.run(&[], &[]);
}

/// `const_i64 v=42; ret_i64 v` -> `ADD %g0, 42, %o0; ta 0` =
/// `0x9000_202A 0x91D0_2000`.  Big-endian: `90 00 20 2A 91 D0 20 00`.
///
/// This is the EXACT byte sequence `lang-aot --emit=sparc-v8` produces
/// for the trivial IIR program `const 42; ret`.
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
            0x90, 0x00, 0x20, 0x2A, // ADD %g0, 42, %o0  (BE)
            0x91, 0xD0, 0x20, 0x00, // ta 0              (BE)
        ]
    );
}

/// Byte-for-byte parity is necessary but not sufficient — genuinely
/// execute the emitted bytes in the new simulator and check the
/// register state, not just assert a hand-derived byte array.
#[test]
fn canonical_const_42_then_ret_actually_executes_to_o0_equals_42() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("lowering");

    let mut sim = SparcV8Simulator::new(65536);
    sim.load_program(&bytes);
    // Two instructions: ADD %g0, 42, %o0 (materialises 42 into %o0)
    // then `ta 0`, which sparc-v8-simulator's executor intercepts to
    // set halted() = true and stop the fetch-decode-execute loop.
    let result = sim.run_loaded_with_limit(2);
    assert_eq!(result.steps, 2);
    assert!(result.halted);
    assert_eq!(sim.regs.read(8 /* %o0 */), 42);
}

#[test]
fn const_zero_then_ret_emits_eight_bytes() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(0)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("zero", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes.len(), 8);
    assert_eq!(bytes[0..4], [0x90, 0x00, 0x20, 0x00], "ADD %g0, 0, %o0");
    assert_eq!(bytes[4..8], [0x91, 0xD0, 0x20, 0x00], "ta 0");
}

#[test]
fn const_max_13bit_signed_immediate_works() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(4095)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("max", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes[0..4], [0x90, 0x00, 0x2F, 0xFF], "ADD %g0, 4095, %o0");
}

#[test]
fn const_min_13bit_signed_immediate_works() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(-4096)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("min", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes[0..4], [0x90, 0x00, 0x30, 0x00], "ADD %g0, -4096, %o0");
}

#[test]
fn const_out_of_range_errors() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(4096)], "i64"),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("big", &[], "void"), &cir).expect_err("4096 overflows 13-bit signed ADD imm");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(4096)));
}

#[test]
fn ret_void_alone_emits_just_ta_zero() {
    let cir = vec![ci("ret_void", None, vec![], "void")];
    let bytes = compile(&ctx("noop", &[], "void"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x91, 0xD0, 0x20, 0x00]);
}

#[test]
fn const_bool_true() {
    let cir = vec![
        ci("const_bool", Some("b"), vec![CIROperand::Bool(true)], "bool"),
        ci("ret_bool", None, vec![CIROperand::Var("b".into())], "bool"),
    ];
    let bytes = compile(&ctx("btrue", &[], "bool"), &cir).expect("lowering");
    assert_eq!(bytes[0..4], [0x90, 0x00, 0x20, 0x01], "ADD %g0, 1, %o0");
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
    // since v0.1.0 only handles single-var-in-%o0 case.
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
    let via_trait = SparcV8Backend.compile(&cir).expect("trait compile");
    let via_free_fn = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("free-fn compile");
    assert_eq!(via_trait, via_free_fn);
}
