//! Integration tests for `iir-to-armv7` v0.1.0 (A3 skeleton).
//!
//! Smoke-level — confirms the validator stub, the emitter shape, and
//! the exact encoded `BKPT #0xFFFF` word (`0xE12FFF7F`).

use interpreter_ir::IIRModule;
use iir_to_armv7::{
    lower_iir_to_armv7, validate_for_armv7,
    IIRArmv7Config, IIRArmv7Error, BKPT,
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
    assert!(validate_for_armv7(&empty_module()).is_empty());
}

// ===========================================================================
// 2. Lowering shape and the exact `BKPT` encoding
// ===========================================================================

#[test]
fn lower_emits_exactly_one_word() {
    let words = lower_iir_to_armv7(&empty_module(), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words.len(), 1,
        "v0.1.0 must emit exactly one word; got: {words:08x?}");
}

/// The emitted word is `0xE12FFF7F` — the canonical ARMv7-A
/// `BKPT #0xFFFF` (breakpoint with the maximum 16-bit immediate).
///
/// Bit layout (cond=AL=0xE):
///
/// ```text
/// 31..28  cond    = 0xE = 1110            (always — unconditional)
/// 27..20          = 0001 0010 = 0x12      (BKPT opcode family)
/// 19.. 8  imm12   = 0xFFF                 (top 12 bits of imm16)
///  7.. 4          = 0111 = 0x7            (BKPT opcode family)
///  3.. 0  imm4    = 0xF                   (bottom 4 bits of imm16)
/// ```
///
/// Concatenated: `1110 0001 0010 1111_1111_1111 0111 1111` =
/// `0xE12FFF7F`.  Pinning the exact constant guards against any
/// future change in the simulator's opcode table that would break the
/// encoding.
#[test]
fn lower_emits_the_canonical_bkpt_word() {
    let words = lower_iir_to_armv7(&empty_module(), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words[0], 0xE12F_FF7F,
        "expected canonical BKPT encoding 0xE12FFF7F; got 0x{:08x}", words[0]);
    assert_eq!(words[0], BKPT,
        "the emitted word should equal the exported BKPT constant");
}

#[test]
fn bkpt_constant_pinned_to_e12fff7f() {
    // Sanity: the exported BKPT constant matches the canonical
    // ARMv7-A documented encoding.  Guards against the kind of "00
    // 12 7f e1" little-endian byte-order confusion that's easy to
    // make when staring at a hexdump.
    assert_eq!(BKPT, 0xE12F_FF7F,
        "BKPT #0xFFFF should be 0xE12FFF7F (cond=AL, imm16=0xFFFF)");
}

// ===========================================================================
// 3. Config defaults
// ===========================================================================

#[test]
fn default_config_has_nonempty_module_name() {
    let cfg = IIRArmv7Config::default();
    assert!(!cfg.module_name.is_empty());
}

#[test]
fn new_sets_module_name() {
    let cfg = IIRArmv7Config::new("custom");
    assert_eq!(cfg.module_name, "custom");
}

// ===========================================================================
// 4. Error display
// ===========================================================================

#[test]
fn errors_display_without_panic() {
    let _ = format!("{}", IIRArmv7Error::ValidationFailed(vec!["x".into()]));
    let _ = format!("{}", IIRArmv7Error::UnsupportedOp {
        function: "f".into(), op: "weird".into(),
    });
    let _ = format!("{}", IIRArmv7Error::UnsupportedType {
        function: "f".into(), type_hint: "weird".into(),
    });
    let _ = format!("{}", IIRArmv7Error::InvalidOperand {
        function: "f".into(), detail: "bad".into(),
    });
}
