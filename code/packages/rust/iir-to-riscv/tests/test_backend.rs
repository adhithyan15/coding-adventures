//! Integration tests for `iir-to-riscv` v0.1.0 (A1 skeleton).
//!
//! Smoke-level — confirms the validator stub, the emitter shape, and the
//! exact encoded word for `ret`.

use interpreter_ir::IIRModule;
use iir_to_riscv::{lower_iir_to_riscv, validate_for_riscv, IIRRiscvConfig, IIRRiscvError};

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
    assert!(validate_for_riscv(&empty_module()).is_empty());
}

// ===========================================================================
// 2. Lowering shape and the exact `ret` encoding
// ===========================================================================

/// v0.1.0 emits exactly one instruction.
#[test]
fn lower_emits_exactly_one_word() {
    let words = lower_iir_to_riscv(&empty_module(), &IIRRiscvConfig::default())
        .expect("lowering");
    assert_eq!(words.len(), 1, "v0.1.0 must emit exactly one word; got: {words:?}");
}

/// The emitted word is `0x0000_8067` — the canonical RV32I encoding of
/// `ret` (`jalr x0, x1, 0`).
///
/// Bit layout (RV32I I-type, opcode JALR = 0b1100111):
/// ```text
/// 31           20 19  15 14  12 11   7 6      0
///  imm[11:0]=0  | rs1=1 | f3=0 | rd=0 | 1100111
/// ```
///
/// = `0000_0000_0000_00001_000_00000_1100111`
/// = `0x0000_8067`.
#[test]
fn lower_emits_the_canonical_ret_word() {
    let words = lower_iir_to_riscv(&empty_module(), &IIRRiscvConfig::default())
        .expect("lowering");
    assert_eq!(
        words[0], 0x0000_8067,
        "expected canonical `ret` encoding 0x00008067; got 0x{:08x}",
        words[0]
    );
}

// ===========================================================================
// 3. Config defaults
// ===========================================================================

#[test]
fn default_config_has_nonempty_module_name() {
    let cfg = IIRRiscvConfig::default();
    assert!(!cfg.module_name.is_empty());
}

#[test]
fn new_sets_module_name() {
    let cfg = IIRRiscvConfig::new("custom");
    assert_eq!(cfg.module_name, "custom");
}

// ===========================================================================
// 4. Error display
// ===========================================================================

#[test]
fn errors_display_without_panic() {
    let _ = format!("{}", IIRRiscvError::ValidationFailed(vec!["x".into()]));
    let _ = format!("{}", IIRRiscvError::UnsupportedOp {
        function: "f".into(), op: "weird".into(),
    });
    let _ = format!("{}", IIRRiscvError::UnsupportedType {
        function: "f".into(), type_hint: "weird".into(),
    });
    let _ = format!("{}", IIRRiscvError::InvalidOperand {
        function: "f".into(), detail: "bad".into(),
    });
}
