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
    let f = IIRFunction::new("boom", vec![], "void", vec![
        IIRInstr::new("add", Some("v".into()),
            vec![Operand::Int(1), Operand::Int(2)], "u8"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let err = lower_iir_to_intel8008(&module_with(f), &IIRIntel8008Config::default())
        .expect_err("`add` should be UnsupportedOp in A2+");
    match err {
        IIRIntel8008Error::UnsupportedOp { function, op } => {
            assert_eq!(function, "boom");
            assert_eq!(op, "add");
        }
        other => panic!("expected UnsupportedOp, got: {other:?}"),
    }
}
