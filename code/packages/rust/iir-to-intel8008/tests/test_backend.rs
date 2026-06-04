//! Integration tests for `iir-to-intel8008` v0.1.0 (A2 skeleton).
//!
//! Note: this crate is deprecated as of v0.4.0 (Phase 6 of the
//! historical-arch backend migration).  Tests still exercise the
//! deprecated API as a regression invariant.
#![allow(deprecated)]
//!
//! Smoke-level — confirms the validator stub, the emitter shape, and
//! the exact encoded `HLT` byte (`0x76`).

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_intel8008::{
    lower_iir_to_intel8008, validate_for_intel8008,
    IIRIntel8008Config, IIRIntel8008Error,
    CAL, HLT, JFC, JFP, JFS, JFZ, JMP, JTC, JTP, JTS, JTZ, MVI_A, RET,
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
    // v0.3.4 — new branch-related variants
    let _ = format!("{}", IIRIntel8008Error::UndefinedLabel {
        function: "f".into(), label: "ghost".into(),
    });
    let _ = format!("{}", IIRIntel8008Error::AddressOutOfRange {
        function: "f".into(), address: 0x4000,
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

// ===========================================================================
// 10. A2++.5.5 third slice — labels + unconditional jump (`jmp` + backpatching)
// ===========================================================================
//
// The 8008's JMP is a 3-byte absolute-address instruction in family
// `01 ddd 100`:
//
//   JMP unconditional = 01 111 100 = 0x7C    ← what we emit
//   JFC (carry-clear) = 01 000 100 = 0x40    ← what `0x44` would imply
//                                              if we got it wrong; pin
//                                              `JMP` to `0x7C` so the
//                                              simulator round-trips
//                                              as unconditional.
//
// Two-pass per-function lowering: pass 1 emits each `jmp` as
// `0x7C 0x00 0x00` and records (slot, target_label).  Pass 2 looks up
// each pending jmp's target in the per-function `labels` table and
// backpatches the two address bytes (low then high; the 8008 is
// little-endian for jump targets and uses 14 bits total — top 2 bits
// of the high byte are zero/ignored).

#[test]
fn jmp_constant_pinned_to_0x7c() {
    // Smoke: the exported JMP constant is the unconditional opcode,
    // NOT the JFC (0x44) conditional one.  Regression for the easy
    // mistake of reading the bit pattern `01 000 100` and assuming
    // `ddd=000` means "unconditional" — it actually means JFC.
    assert_eq!(JMP, 0x7C,
        "JMP unconditional should be 0x7C (01 111 100), not 0x44");
}

#[test]
fn jmp_to_forward_label_backpatches_target_address() {
    // const v=42; jmp end; const w=99; label end; ret v
    //
    // Pass 1 emits:
    //   00: MVI A, 0x2A         (3E 2A)
    //   02: JMP <forward>       (7C ?? ??)
    //   05: MVI B, 0x63         (06 63)
    //   07: <label "end" here>
    //   07: HLT                  (ret of v, v is in A so MOV elided)
    //
    // Pass 2: label "end" was recorded at offset 7 (= 0x0007).
    // The JMP slot is at offset 3 (low) / 4 (high), so they become
    // 0x07 and 0x00.
    let f = IIRFunction::new("fwd", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0x2A)], "u8"),
        IIRInstr::new("jmp",   None,             vec![Operand::Var("end".into())], "void"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(0x63)], "u8"),
        IIRInstr::new("label", None,             vec![Operand::Var("end".into())], "void"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x2A,    // 00 01   MVI A, 0x2A
        JMP,   0x07, 0x00, // 02 03 04   JMP 0x0007
        0x06,  0x63,    // 05 06   MVI B, 0x63 (unreachable after the jmp)
        // <-- label "end" lands at offset 7 -->
        HLT,            // 07      ret v (v is already in A — no MOV)
    ], "forward jmp expected; got: {bytes:02x?}");
}

#[test]
fn jmp_to_backward_label_backpatches_target_address() {
    // label loop; const v=1; jmp loop; ret_void
    //
    // Pass 1:
    //   00: <label "loop" here>
    //   00: MVI A, 0x01         (3E 01)
    //   02: JMP <backward>      (7C ?? ??)
    //   05: HLT
    //
    // Pass 2: "loop" recorded at offset 0.  JMP target = 0x0000.
    let f = IIRFunction::new("backward", vec![], "void", vec![
        IIRInstr::new("label", None,             vec![Operand::Var("loop".into())], "void"),
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0x01)], "u8"),
        IIRInstr::new("jmp",   None,             vec![Operand::Var("loop".into())], "void"),
        IIRInstr::new("ret_void", None,          vec![], "void"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x01,    // 00 01   MVI A, 1
        JMP,   0x00, 0x00, // 02 03 04   JMP 0x0000 (back to top)
        HLT,            // 05      ret_void (unreachable, but emitted)
    ], "backward jmp expected; got: {bytes:02x?}");
}

#[test]
fn jmp_to_undefined_label_is_rejected() {
    let f = IIRFunction::new("dangling", vec![], "void", vec![
        IIRInstr::new("jmp", None, vec![Operand::Var("nowhere".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let err = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect_err("jmp to nonexistent label should fail");
    match err {
        IIRIntel8008Error::UndefinedLabel { function, label } => {
            assert_eq!(function, "dangling");
            assert_eq!(label, "nowhere");
        }
        other => panic!("expected UndefinedLabel, got: {other:?}"),
    }
}

#[test]
fn label_emits_no_bytes() {
    // A bare label alone should not emit any bytes — verify by
    // comparing a const-only function with and without a leading
    // label: both must produce the same byte stream.
    let f_bare = IIRFunction::new("bare", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "u8"),
    ]);
    let f_labeled = IIRFunction::new("labeled", vec![], "u8", vec![
        IIRInstr::new("label", None,             vec![Operand::Var("entry".into())], "void"),
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "u8"),
    ]);
    let cfg = IIRIntel8008Config::default();
    let bare    = lower_iir_to_intel8008(&module_with(f_bare),    &cfg).expect("bare");
    let labeled = lower_iir_to_intel8008(&module_with(f_labeled), &cfg).expect("labeled");
    assert_eq!(bare, labeled,
        "a `label` op must not emit any bytes; bare={bare:02x?} labeled={labeled:02x?}");
}

// ===========================================================================
// 11. A2++.5.5 fourth slice — boolean conditional jumps (jmp_if_true/false)
// ===========================================================================
//
// The 8008 has no "branch on register" — every conditional jump
// reads ONE of the four CPU flags (carry/zero/sign/parity) from the
// last arithmetic/logical op.  To branch on a boolean register's
// value we provoke the zero flag via `ANA A` (the 8008's "TEST A"
// idiom), then JFZ ("jump if Z clear" → cond was non-zero / true) or
// JTZ ("jump if Z set" → cond was zero / false).
//
// Lowering shape:
//
//   [optional]  MOV A, cond_reg     ; skipped when cond_reg == A
//               ANA A    (0xA7)     ; sets Z from A's value
//               JFZ/JTZ target      ; 3-byte conditional jump
//
// Opcode constants pinned in their own tests below to guard against
// any future copy-paste regression in the family-01 jump table.

#[test]
fn jfz_constant_pinned_to_0x48() {
    // JFZ = 01 001 000 (ccc=001 zero, T=0 clear)
    assert_eq!(JFZ, 0x48,
        "JFZ (jump if zero clear) should be 0x48; got 0x{:02x}", JFZ);
}

#[test]
fn jtz_constant_pinned_to_0x4c() {
    // JTZ = 01 001 100 (ccc=001 zero, T=1 set)
    assert_eq!(JTZ, 0x4C,
        "JTZ (jump if zero set) should be 0x4C; got 0x{:02x}", JTZ);
}

#[test]
fn jmp_if_true_emits_ana_a_then_jfz_with_backpatched_target() {
    // const cond=1; jmp_if_true cond, end; const x=0; label end; ret_void
    //
    // cond → A (first const).  No staging MOV needed.
    //
    //   00,01: MVI A, 1       (3E 01)
    //   02:    ANA A          (A7) — set Z from A=1 → Z=0
    //   03:    JFZ <fwd>      (48 ?? ??)
    //   06,07: MVI B, 0       (06 00)  unreachable
    //   <-- label "end" at offset 8 -->
    //   08:    HLT
    let f = IIRFunction::new("if_true", vec![], "void", vec![
        IIRInstr::new("const", Some("cond".into()), vec![Operand::Int(1)], "bool"),
        IIRInstr::new("jmp_if_true", None,
            vec![Operand::Var("cond".into()), Operand::Var("end".into())], "void"),
        IIRInstr::new("const", Some("x".into()), vec![Operand::Int(0)], "u8"),
        IIRInstr::new("label", None, vec![Operand::Var("end".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x01,    // 00 01    cond=1 → A
        0xA7,           // 02       ANA A (TEST A) — sets Z=0
        JFZ,  0x08, 0x00, // 03 04 05  JFZ 0x0008
        0x06, 0x00,     // 06 07    MVI B, 0 (unreachable)
        // label "end" lands at offset 8
        HLT,            // 08
    ], "jmp_if_true expected; got: {bytes:02x?}");
}

#[test]
fn jmp_if_false_emits_ana_a_then_jtz_with_backpatched_target() {
    // const cond=0; jmp_if_false cond, end; const x=99; label end; ret_void
    let f = IIRFunction::new("if_false", vec![], "void", vec![
        IIRInstr::new("const", Some("cond".into()), vec![Operand::Int(0)], "bool"),
        IIRInstr::new("jmp_if_false", None,
            vec![Operand::Var("cond".into()), Operand::Var("end".into())], "void"),
        IIRInstr::new("const", Some("x".into()), vec![Operand::Int(99)], "u8"),
        IIRInstr::new("label", None, vec![Operand::Var("end".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x00,    // 00 01    cond=0 → A
        0xA7,           // 02       ANA A — Z=1
        JTZ,  0x08, 0x00, // 03 04 05  JTZ 0x0008
        0x06, 0x63,     // 06 07    MVI B, 99 (unreachable)
        HLT,            // 08
    ], "jmp_if_false expected; got: {bytes:02x?}");
}

#[test]
fn jmp_if_true_with_cond_not_in_a_emits_staging_mov() {
    // const v=1 → A; const cond=0 → B; jmp_if_true cond, end; ret_void
    //
    // cond is in B, not A → need MOV A, B before ANA A.
    //
    //   00,01: MVI A, 1   (v) → A
    //   02,03: MVI B, 0   (cond) → B
    //   04:    MOV A, B   (stage cond into A)        = 0x78
    //   05:    ANA A      = 0xA7
    //   06:    JFZ end    = 0x48 ?? ??
    //   <-- label end at offset 9 -->
    //   09:    HLT
    let f = IIRFunction::new("if_b", vec![], "void", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(1)], "u8"),
        IIRInstr::new("const", Some("cond".into()), vec![Operand::Int(0)], "bool"),
        IIRInstr::new("jmp_if_true", None,
            vec![Operand::Var("cond".into()), Operand::Var("end".into())], "void"),
        IIRInstr::new("label", None, vec![Operand::Var("end".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x01,    // 00 01    v=1 → A
        0x06,  0x00,    // 02 03    cond=0 → B
        0x78,           // 04       MOV A, B (stage cond)
        0xA7,           // 05       ANA A
        JFZ,  0x09, 0x00, // 06 07 08  JFZ 0x0009
        HLT,            // 09
    ], "jmp_if_true with staging MOV expected; got: {bytes:02x?}");
}

#[test]
fn jmp_if_true_to_undefined_label_is_rejected() {
    let f = IIRFunction::new("dangling_cond", vec![], "void", vec![
        IIRInstr::new("const", Some("cond".into()), vec![Operand::Int(1)], "bool"),
        IIRInstr::new("jmp_if_true", None,
            vec![Operand::Var("cond".into()), Operand::Var("nowhere".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let err = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect_err("jmp_if_true to nonexistent label should fail");
    match err {
        IIRIntel8008Error::UndefinedLabel { function, label } => {
            assert_eq!(function, "dangling_cond");
            assert_eq!(label, "nowhere");
        }
        other => panic!("expected UndefinedLabel, got: {other:?}"),
    }
}

// ===========================================================================
// 12. A2++.5.5 fifth slice — `cmp` (equality) with flag-to-bool capture
// ===========================================================================
//
// `cmp dest, a, b` in IIR produces a boolean (`dest = (a == b) ? 1 : 0`).
// The 8008's `CMP` instruction (family `10 111 sss`, first byte
// `0xB8 | sss`) computes `A - r`, sets the zero flag (`Z = 1 iff A == r`),
// and DISCARDS the difference.  So lowering needs a flag-to-register
// capture sequence:
//
//   [optional]  MOV A, a_reg
//               CMP b_reg                ; sets Z
//               MVI dest_reg, 0          ; default false
//               JFZ <fallthrough>        ; if Z=0 (a != b), skip
//               MVI dest_reg, 1          ; Z=1 (a == b) → set true
//               <fallthrough>
//
// The JFZ's target is the byte position immediately past
// `MVI dest_reg, 1` — a fixed +4-byte forward offset from the JFZ
// itself.  Computed inline (NOT via `pending_jmps`) so the capture
// stays self-contained and doesn't pollute the user-visible label
// namespace with synthetic names.

#[test]
fn cmp_equal_pins_full_capture_byte_stream() {
    // const v=5; const w=5; cmp r v w; ret r
    //
    // Allocator: v→A(7), w→B(0), r→C(1).
    //
    //   00 01  MVI A, 5
    //   02 03  MVI B, 5
    //   04     CMP B           (0xB8)
    //   05 06  MVI C, 0        (0x0E 0x00) — default false
    //   07     JFZ <0x000C>    (0x48 0x0C 0x00)
    //   08 09  ...address bytes...
    //   0A 0B  MVI C, 1        (0x0E 0x01) — Z=1 → set true
    //   0C     MOV A, C        (0x79) — ret r stages r into A
    //   0D     HLT             (0x76)
    let f = IIRFunction::new("eq", vec![], "bool", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(5)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(5)], "u8"),
        IIRInstr::new("cmp",   Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "bool"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("r".into())], "bool"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x05,    // 00 01   MVI A, 5
        0x06,  0x05,    // 02 03   MVI B, 5
        0xB8,           // 04      CMP B (10 111 000)
        0x0E,  0x00,    // 05 06   MVI C, 0 (default false)
        JFZ,           // 07      JFZ (0x48)
        0x0C,  0x00,    // 08 09   target = 0x000C (fallthrough byte)
        0x0E,  0x01,    // 0A 0B   MVI C, 1 (Z=1 path)
        0x79,           // 0C      MOV A, C (stage r for ret)
        HLT,            // 0D
    ], "cmp full capture expected; got: {bytes:02x?}");
}

#[test]
fn cmp_with_lhs_not_in_a_emits_staging_mov() {
    // const v=10 → A; const w=20 → B; const x=20 → C;
    // cmp r w x; ret_void
    //
    // a (w) is in B, not A → need MOV A, B (0x78) before CMP.
    // b is x in C.
    //
    //   00 01  MVI A, 10        (3E 0A)
    //   02 03  MVI B, 20        (06 14)
    //   04 05  MVI C, 20        (0E 14)
    //   06     MOV A, B         (78)         — stage w
    //   07     CMP C            (B9 = 10 111 001 — sss=C=1)
    //   08 09  MVI D, 0         (16 00)
    //   0A     JFZ <0x000F>     (48)
    //   0B 0C  ...              (0F 00)
    //   0D 0E  MVI D, 1         (16 01)
    //   0F     HLT
    let f = IIRFunction::new("cmp_b_c", vec![], "void", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(10)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(20)], "u8"),
        IIRInstr::new("const", Some("x".into()), vec![Operand::Int(20)], "u8"),
        IIRInstr::new("cmp",   Some("r".into()),
            vec![Operand::Var("w".into()), Operand::Var("x".into())], "bool"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x0A,    // 00 01   MVI A, 10
        0x06,  0x14,    // 02 03   MVI B, 20
        0x0E,  0x14,    // 04 05   MVI C, 20
        0x78,           // 06      MOV A, B  (stage w into A)
        0xB9,           // 07      CMP C     (10 111 001 — sss=C=1)
        0x16,  0x00,    // 08 09   MVI D, 0  (D = 2; (2<<3)|0x06 = 0x16)
        JFZ,           // 0A      JFZ
        0x0F,  0x00,    // 0B 0C   target = 0x000F (fallthrough)
        0x16,  0x01,    // 0D 0E   MVI D, 1
        HLT,            // 0F
    ], "cmp with staging MOV expected; got: {bytes:02x?}");
}

/// `cmp r v v` — comparing a value with itself.  Trivially Z=1, so
/// the runtime always takes the "set true" branch.  Lowering shape
/// is identical to the standard case — the optimisation of
/// constant-folding this to `MVI dest, 1` is upstream's job.
#[test]
fn cmp_with_same_register_emits_cmp_a_then_capture() {
    // const v=42; cmp r v v; ret_void
    //
    // Both operands are A.  No staging MOV.  CMP A = 0xBF.
    //
    //   00 01  MVI A, 42
    //   02     CMP A       (BF = 10 111 111)
    //   03 04  MVI B, 0
    //   05     JFZ <0x000A>
    //   06 07  ...
    //   08 09  MVI B, 1
    //   0A     HLT
    let f = IIRFunction::new("self_cmp", vec![], "void", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "u8"),
        IIRInstr::new("cmp",   Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("v".into())], "bool"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x2A,    // 00 01   MVI A, 42
        0xBF,           // 02      CMP A (10 111 111 — sss=A=7)
        0x06,  0x00,    // 03 04   MVI B, 0  (B = 0; (0<<3)|0x06 = 0x06)
        JFZ,           // 05      JFZ
        0x0A,  0x00,    // 06 07   target = 0x000A
        0x06,  0x01,    // 08 09   MVI B, 1
        HLT,            // 0A
    ], "self-cmp expected; got: {bytes:02x?}");
}

#[test]
fn cmp_followed_by_jmp_if_true_composes_correctly() {
    // const v=3; const w=3; cmp eq v w; jmp_if_true eq, end; ret_void
    //
    // This test verifies the v0.3.5 jmp_if_true sequence runs cleanly
    // after a cmp — the `cmp` capture leaves the Z flag in whatever
    // state the trailing `MVI dest, 1` left it (Z flag is affected by
    // most ops but not by `MVI`), so `jmp_if_true` correctly re-tests
    // via its own `ANA A`.  This is the "happy path" cross-slice
    // composition test.
    //
    // Allocator: v→A, w→B, eq→C.
    //
    //   00 01  MVI A, 3
    //   02 03  MVI B, 3
    //   04     CMP B    (0xB8)
    //   05 06  MVI C, 0
    //   07     JFZ <0x000C>
    //   08 09  ...
    //   0A 0B  MVI C, 1
    //   0C     MOV A, C       (eq is in C, jmp_if_true stages into A)  = 0x79
    //   0D     ANA A          (TEST)                                   = 0xA7
    //   0E     JFZ <end>      (0x48)
    //   0F 10  ...end addr...
    //   <-- label "end" at offset 0x11 -->
    //   11     HLT
    let f = IIRFunction::new("cmp_then_branch", vec![], "void", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(3)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(3)], "u8"),
        IIRInstr::new("cmp",   Some("eq".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "bool"),
        IIRInstr::new("jmp_if_true", None,
            vec![Operand::Var("eq".into()), Operand::Var("end".into())], "void"),
        IIRInstr::new("label", None, vec![Operand::Var("end".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x03,    // 00 01   MVI A, 3
        0x06,  0x03,    // 02 03   MVI B, 3
        0xB8,           // 04      CMP B
        0x0E,  0x00,    // 05 06   MVI C, 0
        JFZ,           // 07      JFZ
        0x0C,  0x00,    // 08 09   target 0x000C
        0x0E,  0x01,    // 0A 0B   MVI C, 1
        0x79,           // 0C      MOV A, C (stage eq into A for jmp_if_true)
        0xA7,           // 0D      ANA A (TEST A — sets Z from eq's value)
        JFZ,           // 0E      JFZ end
        0x11,  0x00,    // 0F 10   end at offset 0x11
        HLT,            // 11
    ], "cmp + jmp_if_true compose expected; got: {bytes:02x?}");
}

// ===========================================================================
// 13. A2++.5.5 sixth slice — cmp_ne / cmp_lt / cmp_gt
// ===========================================================================
//
// All three reuse v0.3.6's CMP + capture skeleton, differing only
// in (a) which conditional jump skips the "set-true" overwrite and
// (b) whether the operands are swapped before staging:
//
//   cmp     → skip=JFZ (skip if Z clear / a != b), no swap
//   cmp_ne  → skip=JTZ (skip if Z set   / a == b), no swap
//   cmp_lt  → skip=JFC (skip if C clear / a >= b), no swap
//   cmp_gt  → skip=JFC, OPERANDS SWAPPED so CMP becomes b - a
//             ⇒ carry-set iff b < a iff a > b.
//
// Byte streams below pin each variant against the canonical
// "v=10, w=20" template.  Allocator: v→A, w→B, r→C.
//
//   00 01  MVI A, 10                        (3E 0A)
//   02 03  MVI B, 20                        (06 14)
//   04     CMP B                            (B8)
//   05 06  MVI C, 0                         (0E 00)
//   07     <skip_op>                        (one of JFZ/JTZ/JFC)
//   08 09  target = 0x000C
//   0A 0B  MVI C, 1                         (0E 01)
//   0C     MOV A, C  (ret r into A)         (79)
//   0D     HLT                              (76)
//
// For cmp_gt the only byte that changes is the CMP source register
// (B becomes A because we swapped — a was in A, b was in B, after
// swap left=B's reg, right=A's reg; staging emits MOV A, B then
// CMP A) — so the byte stream differs by one MOV + one CMP byte.

#[test]
fn jfc_constant_pinned_to_0x40() {
    // JFC = 01 000 000 (ccc=000 carry, T=0 clear)
    assert_eq!(JFC, 0x40,
        "JFC (jump if carry clear) should be 0x40; got 0x{:02x}", JFC);
}

#[test]
fn cmp_ne_pins_full_capture_byte_stream() {
    // const v=10; const w=20; cmp_ne r v w; ret r
    let f = IIRFunction::new("ne", vec![], "bool", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(10)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(20)], "u8"),
        IIRInstr::new("cmp_ne", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "bool"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("r".into())], "bool"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x0A,    // 00 01  MVI A, 10
        0x06,  0x14,    // 02 03  MVI B, 20
        0xB8,           // 04     CMP B
        0x0E,  0x00,    // 05 06  MVI C, 0 (default false)
        JTZ,           // 07     JTZ (skip if Z set / a == b)
        0x0C,  0x00,    // 08 09  target = 0x000C
        0x0E,  0x01,    // 0A 0B  MVI C, 1
        0x79,           // 0C     MOV A, C (ret r)
        HLT,            // 0D
    ], "cmp_ne expected; got: {bytes:02x?}");
}

#[test]
fn cmp_lt_pins_full_capture_byte_stream() {
    // const v=10; const w=20; cmp_lt r v w; ret r
    // v < w → carry SET after CMP, JFC NOT taken, falls through → MVI C, 1 → dest=1 ✓
    let f = IIRFunction::new("lt", vec![], "bool", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(10)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(20)], "u8"),
        IIRInstr::new("cmp_lt", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "bool"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("r".into())], "bool"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x0A,    // 00 01  MVI A, 10
        0x06,  0x14,    // 02 03  MVI B, 20
        0xB8,           // 04     CMP B
        0x0E,  0x00,    // 05 06  MVI C, 0
        JFC,           // 07     JFC (skip if carry clear / a >= b)
        0x0C,  0x00,    // 08 09  target = 0x000C
        0x0E,  0x01,    // 0A 0B  MVI C, 1
        0x79,           // 0C     MOV A, C
        HLT,            // 0D
    ], "cmp_lt expected; got: {bytes:02x?}");
}

#[test]
fn cmp_gt_swaps_operands_then_uses_jfc() {
    // const v=10; const w=20; cmp_gt r v w; ret r
    //
    // cmp_gt a, b is implemented as cmp_lt b, a — swap operands then
    // emit identical skeleton.  After swap: left=w(B), right=v(A).
    //
    // Staging: B → A via MOV A, B (0x78), then CMP A (since right=v in A).
    // CMP A = 0xBF (10 111 111).
    //
    //   00 01  MVI A, 10        (v → A)
    //   02 03  MVI B, 20        (w → B)
    //   04     MOV A, B         (stage w into A — was in B)   = 0x78
    //   05     CMP A            (right=A which was the v reg) = 0xBF
    //
    // Wait — the swap means left=b_reg (=B=0), right=a_reg (=A=7).
    // Staging emits MOV A, B because left=0 != A.  Then CMP right=A
    // → CMP A.  So CMP A = 0xBF.
    //
    //   00 01  MVI A, 10
    //   02 03  MVI B, 20
    //   04     MOV A, B    (0x78)
    //   05     CMP A       (0xBF)
    //   06 07  MVI C, 0
    //   08     JFC
    //   09 0A  target = 0x0D
    //   0B 0C  MVI C, 1
    //   0D     MOV A, C
    //   0E     HLT
    let f = IIRFunction::new("gt", vec![], "bool", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(10)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(20)], "u8"),
        IIRInstr::new("cmp_gt", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "bool"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("r".into())], "bool"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x0A,    // 00 01   MVI A, 10
        0x06,  0x14,    // 02 03   MVI B, 20
        0x78,           // 04      MOV A, B (stage swapped left=B into A)
        0xBF,           // 05      CMP A    (right=A after swap)
        0x0E,  0x00,    // 06 07   MVI C, 0
        JFC,           // 08      JFC (skip if carry clear / b >= a / NOT a>b)
        0x0D,  0x00,    // 09 0A   target = 0x000D
        0x0E,  0x01,    // 0B 0C   MVI C, 1
        0x79,           // 0D      MOV A, C
        HLT,            // 0E
    ], "cmp_gt expected; got: {bytes:02x?}");
}

// ===========================================================================
// 14. A2++.5.5 seventh slice — cmp_gte / cmp_lte + remaining 5 cond-jump opcodes
// ===========================================================================
//
// `cmp_gte a, b` (a >= b) ⇔ NOT (a < b) ⇔ "carry clear after CMP b".
// In the shared `emit_cmp_capture` skeleton the "skip" jump fires when
// the boolean would be false — i.e. when carry is SET (a < b).  That's
// `JTC` (jump if flag-carry set, 0x44).
//
// `cmp_lte a, b` (a <= b) ⇔ (b >= a) — same skeleton as cmp_gte
// with operands swapped before staging.  Mirrors how v0.3.7 expressed
// `cmp_gt` as a swap of `cmp_lt`.
//
// Pinning every byte of the byte stream guards against silent skip-
// opcode drift between cmp_lt/cmp_gt/cmp_gte/cmp_lte.

#[test]
fn jtc_constant_pinned_to_0x44() {
    // JTC = 01 000 100 (ccc=000 carry, T=1 set)
    assert_eq!(JTC, 0x44,
        "JTC (jump if carry set) should be 0x44; got 0x{:02x}", JTC);
}

#[test]
fn jfs_constant_pinned_to_0x50() {
    // JFS = 01 010 000 (ccc=010 sign, T=0 clear)
    assert_eq!(JFS, 0x50);
}

#[test]
fn jts_constant_pinned_to_0x54() {
    // JTS = 01 010 100 (ccc=010 sign, T=1 set)
    assert_eq!(JTS, 0x54);
}

#[test]
fn jfp_constant_pinned_to_0x58() {
    // JFP = 01 011 000 (ccc=011 parity, T=0 clear)
    assert_eq!(JFP, 0x58);
}

#[test]
fn jtp_constant_pinned_to_0x5c() {
    // JTP = 01 011 100 (ccc=011 parity, T=1 set)
    assert_eq!(JTP, 0x5C);
}

#[test]
fn cmp_gte_pins_full_capture_byte_stream() {
    // const v=10; const w=20; cmp_gte r v w; ret r
    //
    // 10 >= 20 is false → CMP B sets carry (since A < B).  JTC fires,
    // skipping the "set true" path; dest stays 0.
    //
    //   00 01  MVI A, 10
    //   02 03  MVI B, 20
    //   04     CMP B
    //   05 06  MVI C, 0
    //   07     JTC          (skip if carry set / a < b)
    //   08 09  target 0x000C
    //   0A 0B  MVI C, 1
    //   0C     MOV A, C (ret r)
    //   0D     HLT
    let f = IIRFunction::new("gte", vec![], "bool", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(10)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(20)], "u8"),
        IIRInstr::new("cmp_gte", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "bool"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("r".into())], "bool"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x0A,    // 00 01  MVI A, 10
        0x06,  0x14,    // 02 03  MVI B, 20
        0xB8,           // 04     CMP B
        0x0E,  0x00,    // 05 06  MVI C, 0
        JTC,           // 07     JTC (skip if carry set)
        0x0C,  0x00,    // 08 09  target 0x000C
        0x0E,  0x01,    // 0A 0B  MVI C, 1
        0x79,           // 0C     MOV A, C
        HLT,            // 0D
    ], "cmp_gte expected; got: {bytes:02x?}");
}

#[test]
fn cmp_lte_swaps_operands_then_uses_jtc() {
    // const v=10; const w=20; cmp_lte r v w; ret r
    //
    // cmp_lte a, b → cmp_gte b, a — swap operands then identical
    // skeleton.  After swap: left=w (in B), right=v (in A).
    //
    // Staging MOV A, B (0x78), CMP A (0xBF, since right=v=A).
    //
    //   00 01  MVI A, 10
    //   02 03  MVI B, 20
    //   04     MOV A, B    (stage w into A — swapped left)
    //   05     CMP A       (right=A after swap)
    //   06 07  MVI C, 0
    //   08     JTC
    //   09 0A  target 0x0D
    //   0B 0C  MVI C, 1
    //   0D     MOV A, C
    //   0E     HLT
    let f = IIRFunction::new("lte", vec![], "bool", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(10)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(20)], "u8"),
        IIRInstr::new("cmp_lte", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "bool"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("r".into())], "bool"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        MVI_A, 0x0A,    // 00 01   MVI A, 10
        0x06,  0x14,    // 02 03   MVI B, 20
        0x78,           // 04      MOV A, B (stage swapped left=B)
        0xBF,           // 05      CMP A    (right=A after swap)
        0x0E,  0x00,    // 06 07   MVI C, 0
        JTC,           // 08      JTC
        0x0D,  0x00,    // 09 0A   target 0x000D
        0x0E,  0x01,    // 0B 0C   MVI C, 1
        0x79,           // 0D      MOV A, C
        HLT,            // 0E
    ], "cmp_lte expected; got: {bytes:02x?}");
}

// ===========================================================================
// 15. A2++.5.5 eighth slice — real RET + CAL + module-level call backpatching
// ===========================================================================
//
// `RET` (0x07) — single-byte unconditional return; pops the 8008's
// internal 7-deep return-address stack and jumps there.
//
// `CAL` (0x7E) — 3-byte unconditional call; pushes the address of the
// next instruction onto the stack, then jumps to the target.  NOT
// `0x46` (which is CFZ, conditional call-if-zero-clear).
//
// Lowering changes:
//   - `ret <v>` / `ret_void` now emit `RET` (0x07) for non-entry-point
//     functions; `HLT` is still emitted for the module's entry-point
//     function (calling `RET` there would underflow the empty return
//     stack and pop a garbage address).
//   - New `call dest, fn_name` op emits `CAL + low + high` (3 bytes),
//     captures the return value from A into dest_reg.  Module-level
//     `pending_calls` resolves the 14-bit address after all functions
//     have been laid out.

fn multi_fn_module(entry: &str, functions: Vec<IIRFunction>) -> IIRModule {
    IIRModule {
        name: "test".into(),
        functions,
        entry_point: Some(entry.into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    }
}

#[test]
fn ret_constant_pinned_to_0x07() {
    assert_eq!(RET, 0x07,
        "RET (unconditional return) should be 0x07; got 0x{:02x}", RET);
}

#[test]
fn cal_constant_pinned_to_0x7e() {
    // CAL = 01 111 110 — NOT 0x46 (which is CFZ).
    assert_eq!(CAL, 0x7E,
        "CAL (unconditional call) should be 0x7E; got 0x{:02x}", CAL);
}

#[test]
fn non_entry_function_ret_emits_real_ret_not_hlt() {
    // Module with `main` as the entry point and `helper` as a callee.
    // Even though we don't call helper from main yet, its trailing
    // `ret <v>` should emit RET (0x07), not HLT.
    let main_fn = IIRFunction::new("main", vec![], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let helper = IIRFunction::new("helper", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel8008(
        &multi_fn_module("main", vec![main_fn, helper]),
        &IIRIntel8008Config::default(),
    ).expect("lowering");
    // Layout:
    //   00:  HLT          (main's ret_void — main IS entry, so HLT)
    //   01,02: MVI A, 42  (helper's const v)
    //   03:  RET          (helper's ret v — v already in A; helper is NOT entry)
    assert_eq!(bytes, vec![
        HLT,            // 00  main ret_void (entry → HLT)
        MVI_A, 0x2A,    // 01 02  helper: MVI A, 42
        RET,            // 03  helper ret v (non-entry → RET)
    ], "non-entry helper should emit RET; got: {bytes:02x?}");
}

#[test]
fn entry_function_ret_still_emits_hlt() {
    // Sanity regression: when a function IS the entry point, ret
    // continues to emit HLT (RET would underflow the empty stack).
    // This is the SAME shape as the v0.2.0 const_42 test — we're just
    // confirming v0.3.9's lowering didn't break it.
    let f = IIRFunction::new("answer", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![MVI_A, 0x2A, HLT],
        "entry function ret should still emit HLT; got: {bytes:02x?}");
}

#[test]
fn call_emits_cal_with_backpatched_target_address() {
    // Two functions: `main` (entry) calls `helper`.  `helper` lives at
    // a known offset within the module byte stream.
    //
    // main: call r, helper; ret_void
    //   00:  CAL <helper_addr>   (7E ?? ??)
    //   03:  HLT                  (ret_void in entry → HLT)
    //
    // helper: const v=7; ret v
    //   04, 05: MVI A, 7   (3E 07)
    //   06: RET             (07)
    //
    // After backpatching:
    //   bytes[1..3] = (0x04, 0x00)
    let main_fn = IIRFunction::new("main", vec![], "void", vec![
        IIRInstr::new("call", Some("r".into()),
            vec![Operand::Var("helper".into())], "u8"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let helper = IIRFunction::new("helper", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel8008(
        &multi_fn_module("main", vec![main_fn, helper]),
        &IIRIntel8008Config::default(),
    ).expect("lowering");
    assert_eq!(bytes, vec![
        CAL,   0x04, 0x00,   // 00 01 02  CAL helper (at 0x04)
        // r is allocated in main.  It's the first const-or-call, so it
        // goes into register A.  dest_reg == A so no capture MOV.
        HLT,                  // 03  main ret_void (entry → HLT)
        MVI_A, 0x07,          // 04 05  helper: MVI A, 7
        RET,                  // 06  helper: ret v (non-entry → RET)
    ], "call + RET expected; got: {bytes:02x?}");
}

#[test]
fn call_to_undefined_function_is_rejected() {
    let main_fn = IIRFunction::new("main", vec![], "void", vec![
        IIRInstr::new("call", None, vec![Operand::Var("ghost".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let err = lower_iir_to_intel8008(
        &multi_fn_module("main", vec![main_fn]),
        &IIRIntel8008Config::default(),
    ).expect_err("call to undefined function should fail");
    match err {
        IIRIntel8008Error::UndefinedFunction { caller, callee } => {
            assert_eq!(caller, "main");
            assert_eq!(callee, "ghost");
        }
        other => panic!("expected UndefinedFunction, got: {other:?}"),
    }
}

#[test]
fn call_with_no_dest_discards_return_value() {
    // Void-call shape: no register allocated for the return value.
    //   main: call helper; ret_void
    //   00:  CAL <helper_addr>
    //   03:  HLT
    //   04: helper body
    let main_fn = IIRFunction::new("main", vec![], "void", vec![
        IIRInstr::new("call", None, vec![Operand::Var("helper".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let helper = IIRFunction::new("helper", vec![], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let bytes = lower_iir_to_intel8008(
        &multi_fn_module("main", vec![main_fn, helper]),
        &IIRIntel8008Config::default(),
    ).expect("lowering");
    assert_eq!(bytes, vec![
        CAL,   0x04, 0x00,   // 00 01 02  CAL helper
        HLT,                  // 03  main ret_void
        RET,                  // 04  helper ret_void (non-entry → RET)
    ], "void call expected; got: {bytes:02x?}");
}

#[test]
fn errors_for_undefined_function_display_without_panic() {
    let _ = format!("{}", IIRIntel8008Error::UndefinedFunction {
        caller: "main".into(), callee: "ghost".into(),
    });
}

// Note on the high-byte split + AddressOutOfRange coverage gap:
//
// In v0.3.4 the allocator caps each function at ~25 emitted bytes
// before exhausting the 7-register pool (the test
// `allocator_exhaustion_yields_out_of_registers` pins this).  That's
// nowhere near the 256-byte boundary where the JMP target high byte
// would become nonzero, let alone the 16384-byte ceiling that would
// trigger `AddressOutOfRange`.
//
// The `AddressOutOfRange` error variant exists for forward
// compatibility with the stack-spilled future slices that can emit
// arbitrarily large functions.  The high-byte split itself is plain
// `(target >> 8) & 0x3F` arithmetic which the two range-tested cases
// above (`0x0007` and `0x0000`) exercise with their low-byte branches.
