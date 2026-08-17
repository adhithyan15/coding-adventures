use arm1_backend::{compile, Arm1Backend, BackendError};
use arm1_simulator::ARM1;
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
    // SWI #0x123456 (AL) = 0xEF12_3456, little-endian = 56 34 12 EF
    assert_eq!(bytes, vec![0x56, 0x34, 0x12, 0xEF]);
}

#[test]
fn backend_name_is_arm1() {
    assert_eq!(Arm1Backend.name(), "arm1");
}

#[test]
#[should_panic(expected = "arm1 backend is emit-only")]
fn backend_run_panics_per_spec() {
    Arm1Backend.run(&[], &[]);
}

/// `const_i64 v=42; ret_i64 v` -> `MOV R0, #42; SWI #0x123456` =
/// `0xE3A0_002A 0xEF12_3456`.  Little-endian:
/// `2A 00 A0 E3 56 34 12 EF`.
///
/// This is the EXACT byte sequence `lang-aot --emit=arm1` produces
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
            0x2A, 0x00, 0xA0, 0xE3, // MOV R0, #42  (LE)
            0x56, 0x34, 0x12, 0xEF, // SWI #0x123456 (LE)
        ]
    );
}

/// Byte-for-byte parity is necessary but not sufficient — genuinely
/// execute the emitted bytes in the ARM1 simulator and check the
/// register state, not just assert a hand-derived byte array.
#[test]
fn canonical_const_42_then_ret_actually_executes_to_r0_equals_42() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("lowering");

    let mut cpu = ARM1::new(4096);
    cpu.load_program(&bytes, 0);
    // Two instructions: MOV R0, #42 (materialises 42 into R0) then
    // the pseudo-halt SWI #0x123456, which arm1-simulator's
    // execute_swi intercepts to set halted() = true and stop the
    // fetch-decode-execute loop.
    let traces = cpu.run(100);
    assert_eq!(traces.len(), 2);
    assert_eq!(cpu.read_register(0), 42);
    assert!(cpu.halted());
}

#[test]
fn const_zero_then_ret_emits_eight_bytes() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(0)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("zero", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes.len(), 8);
    assert_eq!(bytes[0..4], [0x00, 0x00, 0xA0, 0xE3], "MOV R0, #0");
    assert_eq!(bytes[4..8], [0x56, 0x34, 0x12, 0xEF], "SWI #0x123456");
}

#[test]
fn const_max_8bit_immediate_works() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(255)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("max", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes[0..4], [0xFF, 0x00, 0xA0, 0xE3], "MOV R0, #255");
}

#[test]
fn const_negative_out_of_range_errors() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(-1)], "i64"),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("neg", &[], "void"), &cir)
        .expect_err("-1 is below the unrotated 8-bit MOV immediate range");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(-1)));
}

#[test]
fn const_over_255_out_of_range_errors() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(256)], "i64"),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("big", &[], "void"), &cir)
        .expect_err("256 overflows the unrotated 8-bit MOV immediate");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(256)));
}

#[test]
fn ret_void_alone_emits_just_halt() {
    let cir = vec![ci("ret_void", None, vec![], "void")];
    let bytes = compile(&ctx("noop", &[], "void"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x56, 0x34, 0x12, 0xEF]);
}

#[test]
fn const_bool_true() {
    let cir = vec![
        ci("const_bool", Some("b"), vec![CIROperand::Bool(true)], "bool"),
        ci("ret_bool", None, vec![CIROperand::Var("b".into())], "bool"),
    ];
    let bytes = compile(&ctx("btrue", &[], "bool"), &cir).expect("lowering");
    assert_eq!(bytes[0..4], [0x01, 0x00, 0xA0, 0xE3], "MOV R0, #1");
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
    // since v0.1.0 only handles single-var-in-R0 case.
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(1)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(2)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("a".into())], "i64"),
    ];
    // Should produce MOV R0,#1 ; MOV R0,#2 (b clobbers a) ; then ret
    // tries to return 'a' which isn't in R0 — error.
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
    let via_trait = Arm1Backend.compile(&cir).expect("trait compile");
    let via_free_fn = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("free-fn compile");
    assert_eq!(via_trait, via_free_fn);
}
