//! Integration tests for `iir-to-intel4004` v0.1.0 (A4 skeleton).
//!
//! Smoke-level — confirms the validator stub, the emitter shape,
//! and the exact encoded `JUN 0x000` byte pair (`0x40 0x00`).

use interpreter_ir::IIRModule;
use iir_to_intel4004::{
    lower_iir_to_intel4004, validate_for_intel4004,
    IIRIntel4004Config, IIRIntel4004Error, HALT_LOOP,
};

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
