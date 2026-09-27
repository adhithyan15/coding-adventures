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

fn run_binary(op: &str, ty: &str, lhs: i64, rhs: i64) -> Z80Simulator {
    run_binary_sources(op, ty, lhs, rhs, "lhs", "rhs")
}

fn run_binary_sources(
    op: &str,
    ty: &str,
    lhs: i64,
    rhs: i64,
    first: &str,
    second: &str,
) -> Z80Simulator {
    let cir = vec![
        ci(
            &format!("const_{ty}"),
            Some("lhs"),
            vec![CIROperand::Int(lhs)],
            ty,
        ),
        ci(
            &format!("const_{ty}"),
            Some("rhs"),
            vec![CIROperand::Int(rhs)],
            ty,
        ),
        ci(
            &format!("{op}_{ty}"),
            Some("result"),
            vec![
                CIROperand::Var(first.into()),
                CIROperand::Var(second.into()),
            ],
            ty,
        ),
        ci(
            &format!("ret_{ty}"),
            None,
            vec![CIROperand::Var("result".into())],
            ty,
        ),
    ];
    let bytes = compile(&ctx("word02_binary", &[], ty), &cir).expect("WORD02 lowering");
    let mut sim = Z80Simulator::new(65536);
    sim.load_program(&bytes).unwrap();
    let result = sim.run_loaded_with_limit(100).unwrap();
    assert!(result.halted);
    sim
}

fn run_not(ty: &str, value: i64) -> Z80Simulator {
    let cir = vec![
        ci(
            &format!("const_{ty}"),
            Some("value"),
            vec![CIROperand::Int(value)],
            ty,
        ),
        ci(
            &format!("not_{ty}"),
            Some("result"),
            vec![CIROperand::Var("value".into())],
            ty,
        ),
        ci(
            &format!("ret_{ty}"),
            None,
            vec![CIROperand::Var("result".into())],
            ty,
        ),
    ];
    let bytes = compile(&ctx("word02_not", &[], ty), &cir).expect("WORD02 lowering");
    let mut sim = Z80Simulator::new(65536);
    sim.load_program(&bytes).unwrap();
    let result = sim.run_loaded_with_limit(100).unwrap();
    assert!(result.halted);
    sim
}

fn hl(sim: &Z80Simulator) -> u16 {
    u16::from_be_bytes([sim.regs.h, sim.regs.l])
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
    sim.load_program(&bytes).unwrap();
    let result = sim.run_loaded_with_limit(10).unwrap();
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
        z80_bytes, INTEL8080_BACKEND_CANONICAL_CONST_42_RET,
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
        ci(
            "const_bool",
            Some("b"),
            vec![CIROperand::Bool(true)],
            "bool",
        ),
        ci("ret_bool", None, vec![CIROperand::Var("b".into())], "bool"),
    ];
    let bytes = compile(&ctx("btrue", &[], "bool"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x3E, 0x01, 0x76]);
}

#[test]
fn word_u16_result_executes_in_hl() {
    let cir = vec![
        ci(
            "const_u16",
            Some("word"),
            vec![CIROperand::Int(0x1234)],
            "u16",
        ),
        ci("ret_u16", None, vec![CIROperand::Var("word".into())], "u16"),
    ];
    let bytes = compile(&ctx("word_result", &[], "u16"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x21, 0x34, 0x12, 0x76]);

    let mut sim = Z80Simulator::new(65536);
    sim.load_program(&bytes).unwrap();
    let result = sim.run_loaded_with_limit(10).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!((sim.regs.h, sim.regs.l), (0x12, 0x34));
}

#[test]
fn byte_result_executes_in_a_at_unsigned_boundary() {
    let cir = vec![
        ci("const_u8", Some("byte"), vec![CIROperand::Int(0xFF)], "u8"),
        ci("ret_u8", None, vec![CIROperand::Var("byte".into())], "u8"),
    ];
    let bytes = compile(&ctx("byte_result", &[], "u8"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x3E, 0xFF, 0x76]);

    let mut sim = Z80Simulator::new(65536);
    sim.load_program(&bytes).unwrap();
    let result = sim.run_loaded_with_limit(10).unwrap();
    assert!(result.halted);
    assert_eq!(sim.regs.a, 0xFF);
}

#[test]
fn const_u8_rejects_word_sized_literal() {
    let cir = vec![ci(
        "const_u8",
        Some("byte"),
        vec![CIROperand::Int(0x100)],
        "u8",
    )];
    let err = compile(&ctx("wide_byte", &[], "u8"), &cir).expect_err("u8 overflow");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(0x100)));
}

#[test]
fn typed_return_must_match_current_result_width() {
    let cir = vec![
        ci(
            "const_u16",
            Some("word"),
            vec![CIROperand::Int(0x1234)],
            "u16",
        ),
        ci("ret_u8", None, vec![CIROperand::Var("word".into())], "u8"),
    ];
    let err = compile(&ctx("wrong_width", &[], "u8"), &cir).expect_err("width mismatch");
    assert!(matches!(err, BackendError::UnsupportedOp(message) if message.contains("16-bit")));
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
fn word02_bounded_allocation() {
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(1)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(2)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("a".into())], "i64"),
    ];
    let bytes = compile(&ctx("two_const_ret_first", &[], "i64"), &cir).expect("two live values");
    let mut sim = Z80Simulator::new(65536);
    sim.load_program(&bytes).unwrap();
    sim.run_loaded_with_limit(10).unwrap();
    assert_eq!(sim.regs.a, 1);

    let mut return_second = cir.clone();
    return_second[2] = ci("ret_i64", None, vec![CIROperand::Var("b".into())], "i64");
    let bytes = compile(&ctx("two_const_ret_second", &[], "i64"), &return_second)
        .expect("secondary result");
    let mut sim = Z80Simulator::new(65536);
    sim.load_program(&bytes).unwrap();
    sim.run_loaded_with_limit(10).unwrap();
    assert_eq!(sim.regs.a, 2);

    let third = ci("const_u8", Some("c"), vec![CIROperand::Int(3)], "u8");
    let err = compile(
        &ctx("three_live", &[], "u8"),
        &[
            ci("const_u8", Some("a"), vec![CIROperand::Int(1)], "u8"),
            ci("const_u8", Some("b"), vec![CIROperand::Int(2)], "u8"),
            third,
        ],
    )
    .expect_err("third live value must be rejected");
    assert!(matches!(err, BackendError::UnsupportedOp(_)));

    let err = compile(
        &ctx("mixed_width", &[], "u16"),
        &[
            ci("const_u8", Some("a"), vec![CIROperand::Int(1)], "u8"),
            ci("const_u16", Some("b"), vec![CIROperand::Int(2)], "u16"),
        ],
    )
    .expect_err("mixed-width live values must be rejected");
    assert!(matches!(err, BackendError::UnsupportedOp(_)));
}

#[test]
fn word02_wrapping_add_proofs() {
    assert_eq!(run_binary("add", "u8", 0xFF, 2).regs.a, 1);
    assert_eq!(hl(&run_binary("add", "u16", 0xFFFF, 2)), 1);
}

#[test]
fn word02_subtract_and_bitwise_proofs() {
    assert_eq!(run_binary("sub", "u8", 0, 1).regs.a, 0xFF);
    assert_eq!(hl(&run_binary("sub", "u16", 0, 1)), 0xFFFF);
    assert_eq!(run_binary("and", "u8", 0xF0, 0x3C).regs.a, 0x30);
    assert_eq!(run_binary("or", "u8", 0xF0, 0x0F).regs.a, 0xFF);
    assert_eq!(run_binary("xor", "u8", 0xAA, 0xFF).regs.a, 0x55);
    assert_eq!(run_not("u8", 0x0F).regs.a, 0xF0);
    assert_eq!(hl(&run_binary("and", "u16", 0xF0F0, 0x0FF0)), 0x00F0);
    assert_eq!(hl(&run_binary("or", "u16", 0xF000, 0x0F0F)), 0xFF0F);
    assert_eq!(hl(&run_binary("xor", "u16", 0x1234, 0x00FF)), 0x12CB);
    assert_eq!(hl(&run_not("u16", 0x00FF)), 0xFF00);
    assert_eq!(
        run_binary_sources("sub", "u8", 3, 9, "rhs", "lhs").regs.a,
        6
    );
    assert_eq!(
        hl(&run_binary_sources("sub", "u16", 3, 0x1234, "rhs", "lhs")),
        0x1231
    );
    assert_eq!(
        hl(&run_binary_sources("add", "u16", 7, 99, "lhs", "lhs")),
        14
    );
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
    assert_eq!(
        bytes,
        vec![0x3E, 0x76, 0x76],
        "LD A,0x76 then the real HALT"
    );

    let mut sim = Z80Simulator::new(65536);
    sim.load_program(&bytes).unwrap();
    let result = sim.run_loaded_with_limit(10).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(sim.regs.a, 0x76);
}

/// CIR ending in `const_*` with NO following `ret_*` must still be
/// terminated. A `bytes.is_empty()`-based check (the bug found in an
/// earlier draft of this same lowering, copied from `intel8080-backend`
/// before its own fix) is fooled here: `bytes` is non-empty as soon as
/// `LD A,n` is emitted, so it would wrongly conclude the program is
/// already terminated and skip appending `HALT`, leaving the compiled
/// program to fall into whatever follows in memory instead of halting.
#[test]
fn dangling_const_with_no_ret_still_gets_a_real_terminator() {
    let cir = vec![ci("const_i64", Some("v"), vec![CIROperand::Int(7)], "i64")];
    let bytes = compile(&ctx("dangling_const", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x3E, 0x07, 0x76]);

    let mut sim = Z80Simulator::new(65536);
    sim.load_program(&bytes).unwrap();
    let result = sim.run_loaded_with_limit(1000).unwrap();
    assert!(
        result.halted,
        "program should halt, not run out the step budget"
    );
    assert!(
        result.steps < 1000,
        "should halt well before the step limit"
    );
}
