//! Integration tests for `iir-to-intel8008` v0.1.0 (A2 skeleton).
//!
//! Smoke-level — confirms the validator stub, the emitter shape, and
//! the exact encoded `HLT` byte (`0x76`).

use interpreter_ir::IIRModule;
use iir_to_intel8008::{
    lower_iir_to_intel8008, validate_for_intel8008,
    IIRIntel8008Config, IIRIntel8008Error, HLT,
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
