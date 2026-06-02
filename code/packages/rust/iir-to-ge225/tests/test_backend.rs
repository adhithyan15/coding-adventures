// Tests for iir-to-ge225 v0.1.0 (A5 skeleton).
//
// Mirrors the iir-to-intel4004 v0.1.0 test set 1-for-1: every assertion
// pins a guarantee made in the spec (code/specs/iir-to-ge225.md).
//
// Why 1-for-1 mirroring across the architecture-backend crates?
// Consistency across iir-to-{riscv, intel8008, armv7, intel4004, ge225}
// makes it cheap for a reviewer to scan a new backend's tests and confirm
// "yes, this covers the same surface area as its siblings".  Divergence
// has to be deliberate.

use interpreter_ir::IIRModule;
use iir_to_ge225::{
    lower_iir_to_ge225, validate_for_ge225, IIRGe225Config, IIRGe225Error, HALT_WORD,
};

// Helper: build an empty IIRModule.  The v0.1.0 backend ignores the
// contents entirely, but the constructor surface is non-trivial enough
// that hand-writing the literal in every test would be noise.
fn empty_module() -> IIRModule {
    IIRModule {
        name: "test_module".into(),
        functions: vec![],
        entry_point: None,
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    }
}

#[test]
fn validate_returns_empty_for_empty_module() {
    // v0.1.0 stub: validator returns no errors for any module.
    let module = empty_module();
    assert!(validate_for_ge225(&module).is_empty());
}

#[test]
fn lower_emits_exactly_three_bytes() {
    // Shape check: one 20-bit word, packed as 3 bytes.
    let module = empty_module();
    let cfg = IIRGe225Config::default();
    let bytes = lower_iir_to_ge225(&module, &cfg).expect("lowering should succeed");
    assert_eq!(bytes.len(), 3, "v0.1.0 emits exactly one 3-byte word");
}

#[test]
fn lower_emits_the_canonical_halt_word() {
    // The acceptance criterion of A5 v0.1.0: the output is the all-zeros
    // 20-bit HLT word, packed as [0x00, 0x00, 0x00].
    let module = empty_module();
    let cfg = IIRGe225Config::default();
    let bytes = lower_iir_to_ge225(&module, &cfg).expect("lowering should succeed");
    assert_eq!(
        bytes,
        vec![0x00, 0x00, 0x00],
        "v0.1.0 emits the canonical HLT sentinel"
    );
}

#[test]
fn halt_word_constant_pinned_to_zeros() {
    // Guard the public constant against accidental edits.
    assert_eq!(HALT_WORD, [0x00, 0x00, 0x00]);
}

#[test]
fn default_config_has_nonempty_module_name() {
    // A default-constructed config should produce something non-empty so
    // downstream consumers can use it as a filename/symbol hint without
    // null checks.
    let cfg = IIRGe225Config::default();
    assert!(!cfg.module_name.is_empty());
}

#[test]
fn new_sets_module_name() {
    // Builder contract: IIRGe225Config::new(s) stores s verbatim.
    let cfg = IIRGe225Config::new("my_module");
    assert_eq!(cfg.module_name, "my_module");
}

#[test]
fn errors_display_without_panic() {
    // Smoke: every IIRGe225Error variant has a Display impl that produces
    // a non-empty string.  Catches missing match arms in Display.
    let errs = vec![
        IIRGe225Error::ValidationFailed(vec!["x".into(), "y".into()]),
        IIRGe225Error::UnsupportedOp {
            function: "f".into(),
            op: "weird_op".into(),
        },
        IIRGe225Error::UnsupportedType {
            function: "f".into(),
            type_hint: "Quaternion".into(),
        },
        IIRGe225Error::InvalidOperand {
            function: "f".into(),
            detail: "expected Int".into(),
        },
    ];
    for err in errs {
        let s = format!("{err}");
        assert!(!s.is_empty(), "Display produced empty string for {err:?}");
    }
}
