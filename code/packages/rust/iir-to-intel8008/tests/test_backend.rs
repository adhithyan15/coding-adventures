//! Integration tests for `iir-to-intel8008` v0.1.0 (A2 skeleton).
//!
//! Smoke-level — confirms the validator stub, the emitter shape, and
//! the exact encoded `HLT` byte (`0x76`).

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_intel8008::{
    lower_iir_to_intel8008, validate_for_intel8008,
    IIRIntel8008Config, IIRIntel8008Error, HLT, MVI_A,
};

fn module_with(f: IIRFunction) -> IIRModule {
    let entry = f.name.clone();
    IIRModule {
        name: "test".into(),
        functions: vec![f],
        entry_point: Some(entry),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn empty_module() -> IIRModule {
    IIRModule {
        name: "demo".into(),
        functions: vec![],
        entry_point: None,
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    }
}

// ===========================================================================
// 1. Validator stub
// ===========================================================================

#[test]
fn validate_returns_empty_for_empty_module() {
    assert!(validate_for_intel8008(&empty_module()).is_empty());
}

// ===========================================================================
// 2. Lowering shape and the exact `HLT` encoding
// ===========================================================================

#[test]
fn lower_emits_exactly_one_byte() {
    let bytes = lower_iir_to_intel8008(&empty_module(), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes.len(), 1,
        "v0.1.0 must emit exactly one byte; got: {bytes:?}");
}

/// The emitted byte is `0x76` — the canonical Intel 8008 `HLT`
/// (`MOV M,M` semantics, halt as a side effect in silicon).
///
/// Bit layout: `01 110 110` = `0x76`.  Volume I of the Intel 8008 User's
/// Manual lists this in the MOV r1,r2 family but the simulator's
/// `halted()` accessor flips true when this byte executes.  Pinning the
/// exact constant guards against any future change in the simulator's
/// opcode table.
#[test]
fn lower_emits_the_canonical_hlt_byte() {
    let bytes = lower_iir_to_intel8008(&empty_module(), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes[0], 0x76,
        "expected canonical HLT encoding 0x76; got 0x{:02x}", bytes[0]);
    assert_eq!(bytes[0], HLT,
        "the emitted byte should equal the exported HLT constant");
}

// ===========================================================================
// 3. Config defaults
// ===========================================================================

#[test]
fn default_config_has_nonempty_module_name() {
    let cfg = IIRIntel8008Config::default();
    assert!(!cfg.module_name.is_empty());
}

#[test]
fn new_sets_module_name() {
    let cfg = IIRIntel8008Config::new("custom");
    assert_eq!(cfg.module_name, "custom");
}

// ===========================================================================
// 4. Error display
// ===========================================================================

#[test]
fn errors_display_without_panic() {
    let _ = format!("{}", IIRIntel8008Error::ValidationFailed(vec!["x".into()]));
    let _ = format!("{}", IIRIntel8008Error::UnsupportedOp {
        function: "f".into(), op: "weird".into(),
    });
    let _ = format!("{}", IIRIntel8008Error::UnsupportedType {
        function: "f".into(), type_hint: "weird".into(),
    });
    let _ = format!("{}", IIRIntel8008Error::InvalidOperand {
        function: "f".into(), detail: "bad".into(),
    });
}

// ===========================================================================
// 5. A2+ — `const` lowers to `MVI A, n` (0x3E + immediate byte)
// ===========================================================================
//
// The accumulator-only first slice: every `const` goes into A; multi-
// register allocation (B/C/D/E/H/L) lands in A2++.  `ret`/`ret_void`
// emits `HLT` until A2++ wires up the 8008's CALL/RET stack.

#[test]
fn const_42_then_ret_lowers_to_mvi_a_42_then_hlt() {
    let f = IIRFunction::new("answer", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![MVI_A, 42, HLT],
        "expected MVI A,42 (0x3E 0x2A) + HLT (0x76); got: {bytes:02x?}");
}

#[test]
fn mvi_a_constant_pinned_to_0x3e() {
    // Sanity: the MVI_A constant matches the canonical Intel 8008
    // documented encoding (0x3E).
    assert_eq!(MVI_A, 0x3E,
        "MVI A immediate-load opcode should be 0x3E (bit pattern 00 111 110)");
}

#[test]
fn const_negative_uses_twos_complement_byte() {
    // -1 → 0xFF via two's-complement reinterpretation.
    let f = IIRFunction::new("f", vec![], "i8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(-1)], "i8"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![MVI_A, 0xFF, HLT],
        "expected MVI A,0xFF + HLT for `const -1`; got: {bytes:02x?}");
}

#[test]
fn const_out_of_byte_range_is_rejected() {
    let f = IIRFunction::new("f", vec![], "i16", vec![
        // 1000 fits in i16 but not in a single 8008 immediate byte.
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(1000)], "i16"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let err = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect_err("1000 should overflow the 8-bit immediate");
    match err {
        IIRIntel8008Error::InvalidOperand { detail, .. } => {
            assert!(detail.contains("8-bit"),
                "expected message naming the 8-bit limit; got: {detail}");
        }
        other => panic!("expected InvalidOperand, got: {other:?}"),
    }
}

#[test]
fn ret_void_alone_emits_just_hlt() {
    let f = IIRFunction::new("main", vec![], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![HLT],
        "ret_void-only function should emit just HLT; got: {bytes:02x?}");
}

#[test]
fn unsupported_op_is_rejected_with_function_name() {
    // After A2++.5 added add/sub, `safepoint` is still outside the
    // whitelist — use it as the canonical never-supported probe.
    let f = IIRFunction::new("boom", vec![], "void", vec![
        IIRInstr::new("safepoint", None, vec![], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let err = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect_err("`safepoint` should be UnsupportedOp");
    match err {
        IIRIntel8008Error::UnsupportedOp { function, op } => {
            assert_eq!(function, "boom");
            assert_eq!(op, "safepoint");
        }
        other => panic!("expected UnsupportedOp, got: {other:?}"),
    }
}

// ===========================================================================
// 6. A2++ — multi-register allocator (A → B → C → D → E → H → L)
// ===========================================================================
//
// `const` rounds out a register from REGISTER_POOL.  A is handed out
// first so the trivial `const v; ret v` case keeps its 3-byte shape
// (no redundant MOV A, X round-trip).  Subsequent consts spill into
// B, C, D, E, H, L in order.

#[test]
fn two_consts_use_a_then_b_then_mov_a_b_before_hlt() {
    // const v=1; const w=2; ret w
    //   MVI A, 1   (0x3E 0x01)
    //   MVI B, 2   (0x06 0x02)
    //   MOV A, B   (01 111 000 = 0x78)  ← stage w into A
    //   HLT        (0x76)
    let f = IIRFunction::new("f", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(1)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(2)], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("w".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x01,    // const v → A
        0x06,  0x02,    // const w → B (MVI B = 0x06)
        0x78,           // MOV A, B (01 111 000 — stage w into A for ret)
        HLT,
    ], "expected MVI A,1 + MVI B,2 + MOV A,B + HLT; got: {bytes:02x?}");
}

#[test]
fn ret_of_first_const_omits_the_redundant_mov() {
    // Regression for the A2+ pinned 3-byte shape: when the value being
    // returned is already in A, no `MOV A, X` is emitted.
    let f = IIRFunction::new("f", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![MVI_A, 0x2A, HLT],
        "A-first allocator should keep the trivial case at 3 bytes; got: {bytes:02x?}");
}

#[test]
fn mov_lowers_to_canonical_mov_ddd_sss() {
    // const v=7; mov w=v; ret w
    // v is in A (allocated first). w is in B (next pool slot).
    //   MVI A, 7    0x3E 0x07
    //   MOV B, A    01 000 111 = 0x47
    //   MOV A, B    0x78  ← stage w back into A for ret
    //   HLT         0x76
    let f = IIRFunction::new("f", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "u8"),
        IIRInstr::new("mov",   Some("w".into()), vec![Operand::Var("v".into())], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("w".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x07,    // const v=7 → A
        0x47,           // MOV B, A (01 000 111)
        0x78,           // MOV A, B (01 111 000) — stage w into A for ret
        HLT,
    ], "expected MVI A,7 + MOV B,A + MOV A,B + HLT; got: {bytes:02x?}");
}

#[test]
fn allocator_exhaustion_yields_out_of_registers() {
    // 7 const + 1 const → exhausts the 7-slot pool [A,B,C,D,E,H,L].
    let mut body = Vec::new();
    for i in 0..8 {
        body.push(IIRInstr::new(
            "const",
            Some(format!("v{i}")),
            vec![Operand::Int(i as i64)],
            "u8",
        ));
    }
    body.push(IIRInstr::new("ret_void", None, vec![], "void"));
    let f = IIRFunction::new("greedy", vec![], "void", body);

    let err = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect_err("8 consts should exhaust the 7-register pool");
    match err {
        IIRIntel8008Error::OutOfRegisters { function, name } => {
            assert_eq!(function, "greedy");
            assert_eq!(name, "v7", "should fail on the 8th local");
        }
        other => panic!("expected OutOfRegisters, got: {other:?}"),
    }
}

#[test]
fn undefined_variable_in_mov_is_rejected() {
    let f = IIRFunction::new("f", vec![], "void", vec![
        IIRInstr::new("mov", Some("w".into()),
            vec![Operand::Var("ghost".into())], "u8"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let err = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect_err("mov from undefined var should fail");
    match err {
        IIRIntel8008Error::UndefinedVariable { name, .. } => {
            assert_eq!(name, "ghost");
        }
        other => panic!("expected UndefinedVariable, got: {other:?}"),
    }
}

// ===========================================================================
// 7. A2++.5 — accumulator-target ALU (ADD, SUB)
// ===========================================================================
//
// All 8008 ALU ops use A as both left source and destination.  The
// lowering shape is therefore:
//
//   if a not in A: MOV A, a_reg
//   ADD/SUB b_reg                    ; result lands in A
//   if dest_reg not A: MOV dest_reg, A
//
// The first const allocates to A so the leading mv is usually skipped.

#[test]
fn add_two_consts_returns_their_sum_via_accumulator() {
    // const v=3; const w=4; add r v w; ret r
    //
    // Allocator: v→A, w→B, r→C.
    //   MVI A,3   = 0x3E 0x03
    //   MVI B,4   = 0x06 0x04
    //   ADD B     = 0x80           (10 000 000 — sss=B=0)
    //   MOV C,A   = 01 001 111 = 0x4F
    //   MOV A,C   = 01 111 001 = 0x79  (stage r into A for ret)
    //   HLT       = 0x76
    let f = IIRFunction::new("add3plus4", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(3)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(4)], "u8"),
        IIRInstr::new("add", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x03,    // MVI A, 3
        0x06,  0x04,    // MVI B, 4
        0x80,           // ADD B
        0x4F,           // MOV C, A
        0x79,           // MOV A, C (stage r into A for ret)
        HLT,
    ], "add 3+4 expected; got: {bytes:02x?}");
}

#[test]
fn sub_two_consts_emits_sub_b_after_mov() {
    // const v=10; const w=4; sub r v w; ret r
    // Same shape but using 0x90 (SUB B, family 10 010 000) instead of 0x80.
    let f = IIRFunction::new("ten_minus_four", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(10)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(4)], "u8"),
        IIRInstr::new("sub", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x0A,    // MVI A, 10
        0x06,  0x04,    // MVI B, 4
        0x90,           // SUB B   (10 010 000)
        0x4F,           // MOV C, A
        0x79,           // MOV A, C
        HLT,
    ], "sub 10-4 expected; got: {bytes:02x?}");
}

#[test]
fn add_when_lhs_is_already_in_a_skips_the_staging_mov() {
    // const v=5; add r v v; ret r
    // v is in A.  add r v v means r = v + v.  Sequence:
    //   MVI A, 5
    //   ADD A   (10 000 111 = 0x87)  ← uses v (in A) as both src and right operand
    //   MOV B, A — bind r → B
    //   MOV A, B — stage r back into A for ret
    //   HLT
    let f = IIRFunction::new("double", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(5)], "u8"),
        IIRInstr::new("add", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("v".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    // Note: no leading "MOV A, A" since v is already in A.
    assert_eq!(bytes, vec![
        MVI_A, 0x05,    // MVI A, 5
        0x87,           // ADD A    (10 000 111 — sss=A=7)
        0x47,           // MOV B, A
        0x78,           // MOV A, B
        HLT,
    ], "double-of-A expected; got: {bytes:02x?}");
}

// ===========================================================================
// 8. A2++.5.5 — bitwise accumulator-target ALU (AND, OR, XOR)
// ===========================================================================
//
// Identical accumulator-anchored shape to add/sub.  Only the 3-bit `ooo`
// selector changes:
//
//   AND = 0b100 → ANA r   first byte = 0xA0 | sss
//   XOR = 0b101 → XRA r   first byte = 0xA8 | sss
//   OR  = 0b110 → ORA r   first byte = 0xB0 | sss
//
// For each op below, the IIR sequence is `const v; const w; OP r v w; ret r`,
// the allocator places v→A, w→B, r→C, and the emitted byte stream
// follows the canonical:
//
//   MVI A, v_imm
//   MVI B, w_imm
//   OP  B            ← the only byte that varies between the three tests
//   MOV C, A
//   MOV A, C         (stage r into A for ret)
//   HLT

#[test]
fn and_two_consts_emits_ana_b_after_mov() {
    // ANA B = 10 100 000 = 0xA0
    let f = IIRFunction::new("and_fn", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0x0F)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(0x33)], "u8"),
        IIRInstr::new("and", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x0F,    // MVI A, 0x0F
        0x06,  0x33,    // MVI B, 0x33
        0xA0,           // ANA B   (10 100 000)
        0x4F,           // MOV C, A
        0x79,           // MOV A, C
        HLT,
    ], "and 0x0F & 0x33 expected; got: {bytes:02x?}");
}

#[test]
fn or_two_consts_emits_ora_b_after_mov() {
    // ORA B = 10 110 000 = 0xB0
    let f = IIRFunction::new("or_fn", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0x0F)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(0xF0)], "u8"),
        IIRInstr::new("or", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x0F,    // MVI A, 0x0F
        0x06,  0xF0,    // MVI B, 0xF0
        0xB0,           // ORA B   (10 110 000)
        0x4F,           // MOV C, A
        0x79,           // MOV A, C
        HLT,
    ], "or 0x0F | 0xF0 expected; got: {bytes:02x?}");
}

#[test]
fn xor_two_consts_emits_xra_b_after_mov() {
    // XRA B = 10 101 000 = 0xA8
    let f = IIRFunction::new("xor_fn", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0xFF)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(0x55)], "u8"),
        IIRInstr::new("xor", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0xFF,    // MVI A, 0xFF
        0x06,  0x55,    // MVI B, 0x55
        0xA8,           // XRA B   (10 101 000)
        0x4F,           // MOV C, A
        0x79,           // MOV A, C
        HLT,
    ], "xor 0xFF ^ 0x55 expected; got: {bytes:02x?}");
}

/// Sanity that the self-op skip-staging optimisation generalises to the
/// bitwise ops too: `and r v v` with v→A produces `ANA A` (0xA7), no
/// leading `MOV A, A`.
#[test]
fn and_when_lhs_is_already_in_a_skips_the_staging_mov() {
    // ANA A = 10 100 111 = 0xA7
    let f = IIRFunction::new("self_and", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0xAA)], "u8"),
        IIRInstr::new("and", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("v".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0xAA,    // MVI A, 0xAA
        0xA7,           // ANA A   (10 100 111 — sss=A=7)
        0x47,           // MOV B, A
        0x78,           // MOV A, B
        HLT,
    ], "self-AND expected; got: {bytes:02x?}");
}

// ===========================================================================
// 9. A2++.5.5 second slice — carry/borrow chained ALU (ADC, SBB)
// ===========================================================================
//
// Same accumulator-anchored shape as add/sub.  Only the 3-bit `ooo`
// selector changes:
//
//   ADC = 0b001 → ACA r   first byte = 0x88 | sss
//   SBB = 0b011 → SCA r   first byte = 0x98 | sss
//
// These add or subtract the carry/borrow flag set by a PRIOR ALU op.
// The backend doesn't enforce flag-producer ordering — the front-end
// must arrange for the producer (an ADD that overflowed) to be the
// immediately-preceding flag-affecting instruction.
//
// In the canonical multi-byte addition idiom:
//
//   r_lo = lo_a + lo_b      ; ADD — sets carry if overflow
//   r_hi = hi_a +carry hi_b ; ADC — consumes carry
//
// the two-instruction sequence is what makes ADC useful.

#[test]
fn adc_two_consts_emits_aca_b_after_mov() {
    // ACA B = 10 001 000 = 0x88
    let f = IIRFunction::new("adc_fn", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0x10)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(0x20)], "u8"),
        IIRInstr::new("adc", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x10,    // MVI A, 0x10
        0x06,  0x20,    // MVI B, 0x20
        0x88,           // ACA B   (10 001 000)
        0x4F,           // MOV C, A
        0x79,           // MOV A, C
        HLT,
    ], "adc expected; got: {bytes:02x?}");
}

#[test]
fn sbb_two_consts_emits_sca_b_after_mov() {
    // SCA B = 10 011 000 = 0x98
    let f = IIRFunction::new("sbb_fn", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0x80)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(0x01)], "u8"),
        IIRInstr::new("sbb", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x80,    // MVI A, 0x80
        0x06,  0x01,    // MVI B, 0x01
        0x98,           // SCA B   (10 011 000)
        0x4F,           // MOV C, A
        0x79,           // MOV A, C
        HLT,
    ], "sbb expected; got: {bytes:02x?}");
}

/// Self-op variant of adc — `adc r v v` where v→A skips the staging
/// MOV, identical to the self-add pattern.  ACA A = 0x89 (10 001 111).
#[test]
fn adc_when_lhs_is_already_in_a_skips_the_staging_mov() {
    let f = IIRFunction::new("self_adc", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0x40)], "u8"),
        IIRInstr::new("adc", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("v".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x40,    // MVI A, 0x40
        0x8F,           // ACA A   (10 001 111 — sss=A=7)
        0x47,           // MOV B, A
        0x78,           // MOV A, B
        HLT,
    ], "self-ADC expected; got: {bytes:02x?}");
}
