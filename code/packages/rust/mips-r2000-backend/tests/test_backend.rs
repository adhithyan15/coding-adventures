use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use mips_r2000_backend::{compile, BackendError, MipsR2000Backend};
use mips_r2000_simulator::MipsR2000Simulator;

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
fn empty_cir_emits_jr_ra() {
    let bytes = compile(&ctx("empty", &[], "void"), &[]).expect("lowering");
    // JR $ra = 0x03E0_0008, big-endian = 03 E0 00 08
    assert_eq!(bytes, vec![0x03, 0xE0, 0x00, 0x08]);
}

#[test]
fn backend_name_is_mips_r2000() {
    assert_eq!(MipsR2000Backend.name(), "mips-r2000");
}

#[test]
#[should_panic(expected = "mips-r2000 backend is emit-only")]
fn backend_run_panics_per_spec() {
    MipsR2000Backend.run(&[], &[]);
}

/// `const_i64 v=42; ret_i64 v` -> `ADDIU $v0, $zero, 42; JR $ra` =
/// `0x2402_002A 0x03E0_0008`.  Big-endian: `24 02 00 2A 03 E0 00 08`.
///
/// This is the EXACT byte sequence `lang-aot --emit=mips-r2000` produces
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
            0x24, 0x02, 0x00, 0x2A, // ADDIU $v0, $zero, 42  (BE)
            0x03, 0xE0, 0x00, 0x08, // JR $ra                (BE)
        ]
    );
}

/// Byte-for-byte parity is necessary but not sufficient — genuinely
/// execute the emitted bytes in the new simulator and check the register
/// state, not just assert a hand-derived byte array.
#[test]
fn canonical_const_42_then_ret_actually_executes_to_v0_equals_42() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("lowering");

    let mut sim = MipsR2000Simulator::new(65536);
    sim.load_program(&bytes);
    // Two instructions: ADDIU (materialises 42 into $v0) then JR $ra
    // (jumps back to address 0 since $ra was never set by a caller —
    // JR is not a halt instruction).  Run exactly two steps and check
    // the register state directly, mirroring
    // `mips_r2000_simulator::simulator::tests::load_immediate_then_jump_register_return`.
    let result = sim.run_loaded_with_limit(2);
    assert_eq!(result.steps, 2);
    assert_eq!(sim.regs.read(2 /* $v0 */), 42);
}

#[test]
fn const_zero_then_ret_emits_eight_bytes() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(0)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("zero", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes.len(), 8);
    assert_eq!(bytes[0..4], [0x24, 0x02, 0x00, 0x00], "ADDIU $v0, $zero, 0");
    assert_eq!(bytes[4..8], [0x03, 0xE0, 0x00, 0x08], "JR $ra");
}

#[test]
fn const_max_16bit_signed_immediate_works() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(32767)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("max", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes[0..4], [0x24, 0x02, 0x7F, 0xFF], "ADDIU $v0, $zero, 32767");
}

#[test]
fn const_min_16bit_signed_immediate_works() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(-32768)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("min", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes[0..4], [0x24, 0x02, 0x80, 0x00], "ADDIU $v0, $zero, -32768");
}

#[test]
fn const_out_of_range_errors() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(32768)], "i64"),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("big", &[], "void"), &cir).expect_err("32768 overflows 16-bit signed ADDIU imm");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(32768)));
}

#[test]
fn ret_void_alone_emits_just_jr_ra() {
    let cir = vec![ci("ret_void", None, vec![], "void")];
    let bytes = compile(&ctx("noop", &[], "void"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x03, 0xE0, 0x00, 0x08]);
}

#[test]
fn const_bool_true() {
    let cir = vec![
        ci("const_bool", Some("b"), vec![CIROperand::Bool(true)], "bool"),
        ci("ret_bool", None, vec![CIROperand::Var("b".into())], "bool"),
    ];
    let bytes = compile(&ctx("btrue", &[], "bool"), &cir).expect("lowering");
    assert_eq!(bytes[0..4], [0x24, 0x02, 0x00, 0x01], "ADDIU $v0, $zero, 1");
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
    // since v0.1.0 only handles single-var-in-$v0 case.
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(1)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(2)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("a".into())], "i64"),
    ];
    // Should produce ADDIU $v0,$zero,1 ; ADDIU $v0,$zero,2 (b clobbers a) ;
    // then ret tries to return 'a' which isn't in $v0 — error.
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
    let via_trait = MipsR2000Backend.compile(&cir).expect("trait compile");
    let via_free_fn = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("free-fn compile");
    assert_eq!(via_trait, via_free_fn);
}
