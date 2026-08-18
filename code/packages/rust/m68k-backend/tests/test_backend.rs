use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use m68k_backend::{compile, BackendError, M68kBackend};
use m68k_simulator::M68kSimulator;

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
    assert_eq!(bytes, vec![0x4E, 0x4F]); // TRAP #15
}

#[test]
fn backend_name_is_m68k() {
    assert_eq!(M68kBackend.name(), "m68k");
}

#[test]
#[should_panic(expected = "m68k backend is emit-only")]
fn backend_run_panics_per_spec() {
    M68kBackend.run(&[], &[]);
}

/// `const_i64 v=42; ret_i64 v` -> `MOVE.L #42, D0; TRAP #15` =
/// `[0x20, 0x3C, 0x00, 0x00, 0x00, 0x2A, 0x4E, 0x4F]`.
///
/// This is the EXACT byte sequence `lang-aot --emit=m68k` produces for
/// the trivial IIR program `const 42; ret`.
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
            0x20, 0x3C, 0x00, 0x00, 0x00, 0x2A, // MOVE.L #42, D0
            0x4E, 0x4F, // TRAP #15
        ]
    );
}

/// Byte-for-byte parity is necessary but not sufficient -- genuinely
/// execute the emitted bytes in the M68K simulator and check the
/// register state, not just assert a hand-derived byte array.
#[test]
fn canonical_const_42_then_ret_actually_executes_to_d0_equals_42() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("lowering");

    let mut sim = M68kSimulator::new(65536);
    sim.run(&bytes);
    // Two instructions: MOVE.L #42, D0 (materialises 42 into D0) then
    // TRAP #15, which m68k-simulator's execute::exec_line4 intercepts
    // to set halted = true and stop the fetch-decode-execute loop.
    assert_eq!(sim.d[0], 42);
    assert!(sim.halted);
}

#[test]
fn const_zero_then_ret_emits_eight_bytes() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(0)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("zero", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes.len(), 8);
    assert_eq!(bytes[0..6], [0x20, 0x3C, 0x00, 0x00, 0x00, 0x00], "MOVE.L #0, D0");
    assert_eq!(bytes[6..8], [0x4E, 0x4F], "TRAP #15");
}

#[test]
fn const_max_u32_immediate_works() {
    let cir = vec![
        ci(
            "const_i64",
            Some("v"),
            vec![CIROperand::Int(i64::from(u32::MAX))],
            "i64",
        ),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("max", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes[0..6], [0x20, 0x3C, 0xFF, 0xFF, 0xFF, 0xFF]);
}

#[test]
fn const_negative_within_i32_range_works() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(-1)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("neg", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes[0..6], [0x20, 0x3C, 0xFF, 0xFF, 0xFF, 0xFF]);

    let mut sim = M68kSimulator::new(65536);
    sim.run(&bytes);
    assert_eq!(sim.d[0], 0xFFFF_FFFF);
    assert!(sim.halted);
}

#[test]
fn const_below_i32_min_out_of_range_errors() {
    let cir = vec![
        ci(
            "const_i64",
            Some("v"),
            vec![CIROperand::Int(i64::from(i32::MIN) - 1)],
            "i64",
        ),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("toolow", &[], "void"), &cir)
        .expect_err("below i32::MIN is out of MOVE.L-immediate range");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(n) if n == i64::from(i32::MIN) - 1));
}

#[test]
fn const_above_u32_max_out_of_range_errors() {
    let cir = vec![
        ci(
            "const_i64",
            Some("v"),
            vec![CIROperand::Int(i64::from(u32::MAX) + 1)],
            "i64",
        ),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("toobig", &[], "void"), &cir)
        .expect_err("above u32::MAX is out of MOVE.L-immediate range");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(n) if n == i64::from(u32::MAX) + 1));
}

#[test]
fn ret_void_alone_emits_just_halt() {
    let cir = vec![ci("ret_void", None, vec![], "void")];
    let bytes = compile(&ctx("noop", &[], "void"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x4E, 0x4F]);
}

#[test]
fn const_bool_true() {
    let cir = vec![
        ci("const_bool", Some("b"), vec![CIROperand::Bool(true)], "bool"),
        ci("ret_bool", None, vec![CIROperand::Var("b".into())], "bool"),
    ];
    let bytes = compile(&ctx("btrue", &[], "bool"), &cir).expect("lowering");
    assert_eq!(bytes[0..6], [0x20, 0x3C, 0x00, 0x00, 0x00, 0x01], "MOVE.L #1, D0");
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
    // since v0.1.0 only handles the single-var-in-D0 case.
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
    let via_trait = M68kBackend.compile(&cir).expect("trait compile");
    let via_free_fn = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("free-fn compile");
    assert_eq!(via_trait, via_free_fn);
}

/// Regression test for the termination-check security class described
/// in this crate's module doc: a `const_*` whose 32-bit immediate's low
/// byte happens to equal `TRAP #15`'s low byte (`0x4F`) must NOT fool
/// the defensive "already terminated?" check into skipping the real
/// halt.  `0x0000004F` = 79 decimal is exactly such a value: its
/// `MOVE.L #79, D0` encoding ends in the byte `0x4F`.
#[test]
fn const_ending_in_halt_low_byte_with_no_ret_still_appends_real_halt() {
    let cir = vec![ci("const_i64", Some("v"), vec![CIROperand::Int(79)], "i64")];
    let bytes = compile(&ctx("trap_lookalike", &[], "i64"), &cir).expect("lowering");

    // MOVE.L #79, D0 = [0x20, 0x3C, 0x00, 0x00, 0x00, 0x4F] -- note the
    // trailing 0x4F, numerically identical to TRAP #15's low byte.
    assert_eq!(bytes[0..6], [0x20, 0x3C, 0x00, 0x00, 0x00, 0x4F]);
    // The defensive terminator must still append a REAL TRAP #15 -- a
    // byte-value check would see the trailing 0x4F, wrongly assume a
    // halt is already there (it isn't -- 0x00,0x4F is not a valid
    // TRAP #15 opword at all, let alone one that was actually pushed
    // by a ret_* arm), and skip appending it.
    assert_eq!(bytes.len(), 8, "a real TRAP #15 must still be appended");
    assert_eq!(&bytes[6..8], &[0x4E, 0x4F], "the appended terminator is TRAP #15");

    // And it must actually halt when executed, not run off the end.
    let mut sim = M68kSimulator::new(65536);
    sim.run(&bytes);
    assert_eq!(sim.d[0], 79);
    assert!(sim.halted, "must halt, not run off the end into zeroed memory");
}
