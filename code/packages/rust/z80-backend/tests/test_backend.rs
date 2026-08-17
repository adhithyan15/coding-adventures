use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use z80_backend::{compile, BackendError, Z80Backend};
use z80_simulator::Z80Simulator;

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

fn const_42_ret_cir() -> Vec<CIRInstr> {
    vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ]
}

#[test]
fn empty_cir_emits_halt() {
    let bytes = compile(&ctx("empty", &[], "void"), &[]).expect("lowering");
    assert_eq!(bytes, vec![0x76]);
}

#[test]
fn backend_name_is_z80() {
    assert_eq!(Z80Backend.name(), "z80");
}

#[test]
#[should_panic(expected = "z80 backend is emit-only")]
fn backend_run_panics_per_spec() {
    Z80Backend.run(&[], &[]);
}

/// Twig `42` canonical: LD A, 42 ; HALT = [0x3E, 0x2A, 0x76]. This is the
/// EXACT byte sequence `lang-aot --emit=z80` produces for the trivial
/// IIR program `const 42; ret`.
#[test]
fn canonical_const_42_then_ret() {
    let cir = const_42_ret_cir();
    let bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x3E, 0x2A, 0x76]);
}

/// Byte-for-byte parity is necessary but not sufficient — genuinely
/// EXECUTE the emitted bytes in the new simulator and check the
/// accumulator, not just assert a hand-derived byte array.
#[test]
fn canonical_const_42_then_ret_actually_executes_to_a_equals_42() {
    let cir = const_42_ret_cir();
    let bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("lowering");

    let mut sim = Z80Simulator::new(65536);
    sim.load_program(&bytes);
    let result = sim.run_loaded_with_limit(10);
    assert!(result.halted);
    assert_eq!(result.steps, 2, "LD A,42 then HALT is exactly two steps");
    assert_eq!(sim.regs.a, 42);
}

/// Cross-architecture consistency check called out explicitly by the
/// migration plan: the Z80 is a source/binary-compatible superset of the
/// Intel 8080 for this minimal-viable subset (`LD A,n` / `HALT` reuse
/// the 8080's `MVI A,n` / `HLT` encodings verbatim), so `z80-backend`
/// must emit the SAME bytes `intel8080-backend` emits for the identical
/// trivial CIR program.
///
/// This worktree was branched fresh off `origin/main`, which at this
/// point in the 9-architecture expansion has only the ARM1 lane merged
/// — the Intel 8080 lane (third of the expansion) is still an unmerged
/// sibling PR, so `intel8080-backend`/`intel8080-simulator` are not
/// crates this workspace snapshot can depend on (see the NOTE in
/// `Cargo.toml`'s `[dev-dependencies]`). The expected byte sequence
/// below is therefore pinned as a literal constant rather than computed
/// via a live call into `intel8080_backend::compile` — but it is the
/// EXACT sequence that crate's own test suite
/// (`intel8080-backend/tests/test_backend.rs::canonical_const_42_then_ret`)
/// asserts, and `intel8080-encoder`'s `canonical_const_42_bytes` test
/// pins `encode_mvi_a(42) == [0x3E, 0x2A]` / `HLT == 0x76` the same way.
/// A follow-up, once that lane merges, can replace this literal with a
/// direct `intel8080_backend::compile(...)` call.
#[test]
fn z80_backend_matches_intel8080_backend_byte_for_byte() {
    const INTEL8080_BACKEND_CANONICAL_CONST_42_RET: [u8; 3] = [0x3E, 0x2A, 0x76];

    let cir = const_42_ret_cir();
    let z80_bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("z80 lowering");
    assert_eq!(
        z80_bytes,
        INTEL8080_BACKEND_CANONICAL_CONST_42_RET,
        "z80-backend must emit the same bytes intel8080-backend emits for \
         const 42; ret -- both chips share the same LD A,n / HALT (MVI A,n / HLT) \
         encoding for this minimal-viable subset"
    );
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
    let err = compile(&ctx("big", &[], "void"), &cir).expect_err("256 overflows 8-bit LD A,n imm");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(256)));
}

#[test]
fn ret_void_alone_emits_just_halt() {
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
    let cir = const_42_ret_cir();
    let via_trait = Z80Backend.compile(&cir).expect("trait compile");
    let via_free_fn = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("free-fn compile");
    assert_eq!(via_trait, via_free_fn);
}

/// A `const_*` immediate whose byte value happens to equal the Z80's
/// `HALT` opcode byte (`0x76` = 118) must NOT be misread as a halt by
/// any termination check — proving `z80-backend`'s "have I already
/// emitted the real halt" logic tracks a boolean, not a trailing-byte
/// comparison (the Intel 8051 lane's bug class this migration explicitly
/// guards against; see `code/specs/z80-backend.md`).
#[test]
fn const_value_equal_to_halt_opcode_byte_is_not_misread() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(0x76)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("halt_valued_const", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x3E, 0x76, 0x76], "LD A,0x76 then the real HALT");

    let mut sim = Z80Simulator::new(65536);
    sim.load_program(&bytes);
    let result = sim.run_loaded_with_limit(10);
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(sim.regs.a, 0x76);
}
