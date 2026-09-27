use intel8086_backend::{compile, BackendError, Intel8086Backend};
use intel8086_simulator::Intel8086Simulator;
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

fn run_binary(op: &str, ty: &str, lhs: i64, rhs: i64) -> Intel8086Simulator {
    run_binary_sources(op, ty, lhs, rhs, "lhs", "rhs")
}

fn run_binary_sources(
    op: &str,
    ty: &str,
    lhs: i64,
    rhs: i64,
    first: &str,
    second: &str,
) -> Intel8086Simulator {
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
    let mut sim = Intel8086Simulator::new(1 << 20);
    let cs = 0x0010;
    sim.cs = cs;
    let origin = intel8086_simulator::simulator::phys_addr(cs, 0);
    sim.load_program_at(&bytes, origin);
    let result = sim.run_loaded_with_limit(100);
    assert!(result.halted);
    sim
}

fn run_not(ty: &str, value: i64) -> Intel8086Simulator {
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
    let mut sim = Intel8086Simulator::new(1 << 20);
    let cs = 0x0010;
    sim.cs = cs;
    let origin = intel8086_simulator::simulator::phys_addr(cs, 0);
    sim.load_program_at(&bytes, origin);
    let result = sim.run_loaded_with_limit(100);
    assert!(result.halted);
    sim
}

#[test]
fn empty_cir_emits_hlt() {
    let bytes = compile(&ctx("empty", &[], "void"), &[]).expect("lowering");
    assert_eq!(bytes, vec![0xF4]);
}

#[test]
fn backend_name_is_intel8086() {
    assert_eq!(Intel8086Backend.name(), "intel8086");
}

#[test]
#[should_panic(expected = "intel8086 backend is emit-only")]
fn backend_run_panics_per_spec() {
    Intel8086Backend.run(&[], &[]);
}

/// `const_i64 v=42; ret_i64 v` -> `MOV AX,42; HLT` = `[0xB8, 0x2A, 0x00, 0xF4]`.
///
/// This is the EXACT byte sequence `lang-aot --emit=intel8086` produces
/// for the trivial IIR program `const 42; ret`.
#[test]
fn canonical_const_42_then_ret() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xB8, 0x2A, 0x00, 0xF4]);
}

/// Byte-for-byte parity is necessary but not sufficient -- genuinely
/// execute the emitted bytes in the new simulator (accounting for
/// segmented CS:IP addressing) and check AX, not just assert a
/// hand-derived byte array.
#[test]
fn canonical_const_42_then_ret_actually_executes_to_ax_equals_42() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("lowering");

    // Load and run at a non-zero CS to prove this genuinely goes through
    // segmented CS:IP physical addressing, not a flat-memory shortcut --
    // CS=0x0010 -> physical base 0x00100 (see
    // intel8086_simulator::simulator::phys_addr).
    let mut sim = Intel8086Simulator::new(1 << 20);
    let cs = 0x0010u16;
    sim.cs = cs;
    let origin = intel8086_simulator::simulator::phys_addr(cs, 0);
    sim.load_program_at(&bytes, origin);

    // Two instructions: MOV AX,42 (materialises 42 into AX) then HLT,
    // which intel8086-simulator's execute() intercepts to set
    // halted = true and stop the fetch-decode-execute loop.
    let result = sim.run_loaded_with_limit(10);
    assert_eq!(result.steps, 2);
    assert_eq!(sim.ax, 42);
    assert!(result.halted);
    assert!(sim.halted);
}

#[test]
fn const_zero_then_ret_emits_four_bytes() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(0)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("zero", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xB8, 0x00, 0x00, 0xF4], "MOV AX,0; HLT");
}

#[test]
fn const_max_16bit_immediate_works() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(65535)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("max", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes[0..3], [0xB8, 0xFF, 0xFF], "MOV AX,0xFFFF");
}

#[test]
fn const_negative_out_of_range_errors() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(-1)], "i64"),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("neg", &[], "void"), &cir)
        .expect_err("-1 is below the unsigned 16-bit MOV immediate range");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(-1)));
}

#[test]
fn const_over_65535_out_of_range_errors() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(65536)], "i64"),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("big", &[], "void"), &cir)
        .expect_err("65536 overflows the unsigned 16-bit MOV immediate");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(65536)));
}

#[test]
fn ret_void_alone_emits_just_hlt() {
    let cir = vec![ci("ret_void", None, vec![], "void")];
    let bytes = compile(&ctx("noop", &[], "void"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xF4]);
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
    assert_eq!(bytes[0..3], [0xB8, 0x01, 0x00], "MOV AX,1");
}

#[test]
fn word_u16_result_executes_in_ax() {
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
    assert_eq!(bytes, vec![0xB8, 0x34, 0x12, 0xF4]);

    let mut sim = Intel8086Simulator::new(65536);
    sim.load_program(&bytes);
    let result = sim.run_loaded_with_limit(10);
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(sim.ax, 0x1234);
}

#[test]
fn byte_result_is_zero_extended_in_ax() {
    let cir = vec![
        ci("const_u8", Some("byte"), vec![CIROperand::Int(0xFF)], "u8"),
        ci("ret_u8", None, vec![CIROperand::Var("byte".into())], "u8"),
    ];
    let bytes = compile(&ctx("byte_result", &[], "u8"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xB8, 0xFF, 0x00, 0xF4]);

    let mut sim = Intel8086Simulator::new(65536);
    sim.load_program(&bytes);
    let result = sim.run_loaded_with_limit(10);
    assert!(result.halted);
    assert_eq!(sim.ax, 0x00FF, "AL=0xFF and AH=0");
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
    let mut sim = Intel8086Simulator::new(65536);
    sim.load_program(&bytes);
    sim.run_loaded_with_limit(10);
    assert_eq!(sim.ax, 1);

    let mut return_second = cir.clone();
    return_second[2] = ci("ret_i64", None, vec![CIROperand::Var("b".into())], "i64");
    let bytes = compile(&ctx("two_const_ret_second", &[], "i64"), &return_second)
        .expect("secondary result");
    let mut sim = Intel8086Simulator::new(65536);
    sim.load_program(&bytes);
    sim.run_loaded_with_limit(10);
    assert_eq!(sim.ax, 2);

    let err = compile(
        &ctx("three_live", &[], "u8"),
        &[
            ci("const_u8", Some("a"), vec![CIROperand::Int(1)], "u8"),
            ci("const_u8", Some("b"), vec![CIROperand::Int(2)], "u8"),
            ci("const_u8", Some("c"), vec![CIROperand::Int(3)], "u8"),
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
    assert_eq!(run_binary("add", "u8", 0xFF, 2).ax, 1);
    assert_eq!(run_binary("add", "u16", 0xFFFF, 2).ax, 1);
}

#[test]
fn word02_subtract_and_bitwise_proofs() {
    assert_eq!(run_binary("sub", "u8", 0, 1).ax, 0x00FF);
    assert_eq!(run_binary("sub", "u16", 0, 1).ax, 0xFFFF);
    assert_eq!(run_binary("and", "u8", 0xF0, 0x3C).ax, 0x0030);
    assert_eq!(run_binary("or", "u8", 0xF0, 0x0F).ax, 0x00FF);
    assert_eq!(run_binary("xor", "u8", 0xAA, 0xFF).ax, 0x0055);
    assert_eq!(run_not("u8", 0x0F).ax, 0x00F0);
    assert_eq!(run_binary("and", "u16", 0xF0F0, 0x0FF0).ax, 0x00F0);
    assert_eq!(run_binary("or", "u16", 0xF000, 0x0F0F).ax, 0xFF0F);
    assert_eq!(run_binary("xor", "u16", 0x1234, 0x00FF).ax, 0x12CB);
    assert_eq!(run_not("u16", 0x00FF).ax, 0xFF00);
    assert_eq!(run_binary_sources("sub", "u8", 3, 9, "rhs", "lhs").ax, 6);
    assert_eq!(
        run_binary_sources("sub", "u16", 3, 0x1234, "rhs", "lhs").ax,
        0x1231
    );
    assert_eq!(run_binary_sources("add", "u16", 7, 99, "lhs", "lhs").ax, 14);
}

#[test]
fn backend_trait_compile_matches_free_function() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let via_trait = Intel8086Backend.compile(&cir).expect("trait compile");
    let via_free_fn = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("free-fn compile");
    assert_eq!(via_trait, via_free_fn);
}

// ===========================================================================
// The terminated:bool regression test
// ===========================================================================
//
// A real bug class was fixed in four prior lanes of this campaign (Intel
// 8051, Intel 8080, MOS 6502, Zilog Z80): the defensive "is the program
// already terminated?" check was written as a trailing-byte-value
// comparison (or an `is_empty()` check), which a legitimate const_*
// immediate's encoded bytes can fool. This test proves
// intel8086-backend does NOT have that bug.

#[test]
fn const_whose_encoded_high_byte_collides_with_halt_opcode_still_gets_real_terminator() {
    // imm = 0xF400 -- `MOV AX,0xF400` encodes as [0xB8, 0x00, 0xF4]. The
    // immediate's high byte (0xF4) numerically collides with HLT's own
    // opcode byte (intel8086_encoder::HALT_BYTE == 0xF4). A naive "is
    // the program already terminated?" check that compares the trailing
    // byte to HALT_BYTE would wrongly conclude this program already
    // halted and skip appending a real HLT -- leaving a 3-byte program
    // with NO genuine halt instruction, even though this CIR has no
    // ret_*/ret_void at all. The correct fix (this backend's
    // `terminated: bool` local) is set true ONLY by a genuine
    // ret_*/ret_void arm, so a program with no ret must always get a
    // real HLT appended, regardless of what byte value precedes it.
    let cir = vec![ci(
        "const_i64",
        Some("v"),
        vec![CIROperand::Int(0xF400)],
        "i64",
    )];
    let bytes = compile(&ctx("collide", &[], "i64"), &cir).expect("lowering");
    assert_eq!(
        bytes,
        vec![0xB8, 0x00, 0xF4, 0xF4],
        "MOV AX,0xF400; HLT -- a real terminator must be appended despite the \
         byte collision between the immediate's high byte and HALT_BYTE"
    );

    // Execute it for good measure: AX must hold 0xF400 and the simulator
    // must have genuinely halted (not run off into garbage memory).
    let mut sim = Intel8086Simulator::new(65536);
    sim.load_program(&bytes);
    let result = sim.run_loaded_with_limit(10);
    assert!(result.halted, "the emitted program must contain a real HLT");
    assert_eq!(sim.ax, 0xF400);
    assert_eq!(result.steps, 2, "MOV AX,0xF400, then the appended HLT");
}

#[test]
fn const_whose_low_byte_collides_with_halt_opcode_is_unaffected() {
    // Sanity check the other half of the byte pair: imm = 0x00F4 encodes
    // as [0xB8, 0xF4, 0x00] -- the LOW byte collides with HALT_BYTE, but
    // the trailing byte is 0x00, so even a naive trailing-byte-only
    // check would (by luck) still append HLT here. Included so the test
    // suite documents both halves of the immediate independently.
    let cir = vec![ci(
        "const_i64",
        Some("v"),
        vec![CIROperand::Int(0x00F4)],
        "i64",
    )];
    let bytes = compile(&ctx("collide_lo", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xB8, 0xF4, 0x00, 0xF4]);

    let mut sim = Intel8086Simulator::new(65536);
    sim.load_program(&bytes);
    let result = sim.run_loaded_with_limit(10);
    assert!(result.halted);
    assert_eq!(sim.ax, 0x00F4);
}

#[test]
fn two_live_consts_with_no_ret_still_get_exactly_one_terminator() {
    // Proves the `terminated` flag correctly resets on each subsequent
    // const_* -- if it didn't reset, a stale `true` from some
    // intermediate state could suppress the final HLT.
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(1)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(2)], "i64"),
    ];
    let bytes = compile(&ctx("two_consts", &[], "i64"), &cir).expect("lowering");
    // MOV AX,#1 + MOV BX,#2 + one HLT = 7 bytes.
    assert_eq!(bytes.len(), 7);
    assert_eq!(bytes.last(), Some(&0xF4));
    // Confirm there is exactly one HLT byte at the very end preceded by
    // the last MOV's non-HLT-opcode bytes (0xB8 is the MOV opcode, not
    // 0xF4), i.e. this isn't an accidental double-halt.
    assert_eq!(&bytes[3..7], &[0xBB, 0x02, 0x00, 0xF4]);
}
