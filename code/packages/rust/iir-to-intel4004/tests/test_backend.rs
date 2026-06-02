//! Integration tests for `iir-to-intel4004` v0.1.0 (A4 skeleton).
//!
//! Smoke-level — confirms the validator stub, the emitter shape,
//! and the exact encoded `JUN 0x000` byte pair (`0x40 0x00`).

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_intel4004::{
    lower_iir_to_intel4004, validate_for_intel4004,
    IIRIntel4004Config, IIRIntel4004Error, HALT_LOOP, LDM_OPCODE,
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
    assert!(validate_for_intel4004(&empty_module()).is_empty());
}

// ===========================================================================
// 2. Lowering shape and the exact `JUN 0x000` encoding
// ===========================================================================

#[test]
fn lower_emits_exactly_two_bytes() {
    let bytes = lower_iir_to_intel4004(&empty_module(), &IIRIntel4004Config::default())
        .expect("lowering");
    assert_eq!(bytes.len(), 2,
        "v0.1.0 must emit exactly two bytes (the JUN 0x000 sentinel); got: {bytes:02x?}");
}

/// The emitted bytes are `0x40 0x00` — the canonical 4004-ROM
/// `JUN 0x000` halt idiom.
///
/// Bit layout (JUN = `0100 aaaa aaaaaaaa`):
///
/// ```text
/// byte 1: 0100 0000 = 0x40   (JUN opcode + high nibble of 12-bit addr = 0)
/// byte 2: 0000 0000 = 0x00   (low byte of address = 0)
/// ```
///
/// Pinning the exact constant guards against any future change in
/// 4004 simulator decoders that would break the encoding.
#[test]
fn lower_emits_the_canonical_jun_self_bytes() {
    let bytes = lower_iir_to_intel4004(&empty_module(), &IIRIntel4004Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0x40, 0x00],
        "expected canonical JUN 0x000 encoding [0x40, 0x00]; got: {bytes:02x?}");
    assert_eq!(bytes.as_slice(), &HALT_LOOP,
        "the emitted bytes should equal the exported HALT_LOOP constant");
}

#[test]
fn halt_loop_constant_pinned_to_40_00() {
    // Sanity: the HALT_LOOP constant matches the canonical 4004
    // JUN-0x000 documented encoding.
    assert_eq!(HALT_LOOP, [0x40, 0x00],
        "JUN 0x000 should be [0x40, 0x00] (opcode 0100, addr 0x000)");
}

// ===========================================================================
// 3. Config defaults
// ===========================================================================

#[test]
fn default_config_has_nonempty_module_name() {
    let cfg = IIRIntel4004Config::default();
    assert!(!cfg.module_name.is_empty());
}

#[test]
fn new_sets_module_name() {
    let cfg = IIRIntel4004Config::new("custom");
    assert_eq!(cfg.module_name, "custom");
}

// ===========================================================================
// 4. Error display
// ===========================================================================

#[test]
fn errors_display_without_panic() {
    let _ = format!("{}", IIRIntel4004Error::ValidationFailed(vec!["x".into()]));
    let _ = format!("{}", IIRIntel4004Error::UnsupportedOp {
        function: "f".into(), op: "weird".into(),
    });
    let _ = format!("{}", IIRIntel4004Error::UnsupportedType {
        function: "f".into(), type_hint: "weird".into(),
    });
    let _ = format!("{}", IIRIntel4004Error::InvalidOperand {
        function: "f".into(), detail: "bad".into(),
    });
}

// ===========================================================================
// 5. A4+ (v0.2.0) — `const` lowers to `LDM n`; `ret` → JUN-self (halt)
// ===========================================================================
//
// The accumulator-only first slice: every `const` goes into the
// accumulator via `LDM`.  `ret`/`ret_void` both emit the 2-byte
// JUN 0x000 halt sentinel because real RET (via BBL + the 4004's
// 3-deep internal stack) needs A4++'s call/return discipline.

#[test]
fn ldm_opcode_pinned_to_0xd0() {
    // Sanity: the LDM_OPCODE high nibble matches the canonical
    // 4004-documented encoding (1101_xxxx).
    assert_eq!(LDM_OPCODE, 0xD0,
        "LDM opcode high nibble should be 0xD0 (1101_0000)");
}

#[test]
fn const_5_then_ret_lowers_to_ldm_5_then_jun_self() {
    // const v=5; ret v
    //   LDM 5     = 0xD5
    //   JUN 0x000 = 0x40 0x00
    let f = IIRFunction::new("five", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(5)], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0xD5, 0x40, 0x00],
        "expected LDM 5 (0xD5) + JUN 0x000 (0x40 0x00); got: {bytes:02x?}");
}

#[test]
fn const_15_then_ret_lowers_to_ldm_15_then_jun_self() {
    // const v=15 (the max 4-bit value); ret v
    //   LDM 15    = 0xDF
    //   JUN 0x000 = 0x40 0x00
    let f = IIRFunction::new("max", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(15)], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0xDF, 0x40, 0x00],
        "expected LDM 15 (0xDF) + JUN 0x000; got: {bytes:02x?}");
}

#[test]
fn const_0_then_ret_emits_ldm_0() {
    // const v=0; ret v
    //   LDM 0 = 0xD0 — pinned because OR-ing with 0 should leave
    //   the LDM opcode high nibble visibly intact.
    let f = IIRFunction::new("zero", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0)], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0xD0, 0x40, 0x00],
        "expected LDM 0 (0xD0) + JUN 0x000; got: {bytes:02x?}");
}

#[test]
fn const_negative_uses_twos_complement_nibble() {
    // -1 → 0xF via 4-bit two's-complement reinterpretation.
    // LDM 0xF = 0xDF.
    let f = IIRFunction::new("minus_one", vec![], "i8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(-1)], "i8"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let bytes = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0xDF, 0x40, 0x00],
        "expected LDM 0xF + JUN 0x000 for `const -1`; got: {bytes:02x?}");
}

#[test]
fn const_negative_minus_eight_uses_8_nibble() {
    // -8 → 0x8 (the minimum signed 4-bit value).  LDM 0x8 = 0xD8.
    let f = IIRFunction::new("minus_eight", vec![], "i8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(-8)], "i8"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let bytes = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0xD8, 0x40, 0x00],
        "expected LDM 0x8 + JUN 0x000 for `const -8`; got: {bytes:02x?}");
}

#[test]
fn const_out_of_nibble_range_is_rejected() {
    let f = IIRFunction::new("oversized", vec![], "i16", vec![
        // 16 is just past the 4-bit range.
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(16)], "i16"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let err = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect_err("16 should overflow the 4-bit immediate");
    match err {
        IIRIntel4004Error::InvalidOperand { detail, .. } => {
            assert!(detail.contains("4-bit"),
                "expected message naming the 4-bit limit; got: {detail}");
        }
        other => panic!("expected InvalidOperand, got: {other:?}"),
    }
}

#[test]
fn ret_void_alone_emits_just_jun_self() {
    let f = IIRFunction::new("main", vec![], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let bytes = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0x40, 0x00],
        "ret_void-only function should emit just JUN 0x000; got: {bytes:02x?}");
}

#[test]
fn unsupported_op_is_rejected_with_function_name() {
    let f = IIRFunction::new("boom", vec![], "void", vec![
        IIRInstr::new("safepoint", None, vec![], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let err = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect_err("`safepoint` should be UnsupportedOp");
    match err {
        IIRIntel4004Error::UnsupportedOp { function, op } => {
            assert_eq!(function, "boom");
            assert_eq!(op, "safepoint");
        }
        other => panic!("expected UnsupportedOp, got: {other:?}"),
    }
}
