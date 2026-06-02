// Tests for iir-to-ge225 v0.2.0 (A5+ — first real lowering).
//
// Mirrors iir-to-intel4004 v0.2.0's test set 1-for-1 in spirit:
// every assertion pins a guarantee made in the spec
// (code/specs/iir-to-ge225.md).
//
// Coverage:
//   §1 — validator stub
//   §2 — HLT shape + constant pinning (regression from v0.1.0)
//   §3 — Config defaults
//   §4 — Error Display
//   §5 — A5+: const → LDA, ret → HLT (the new behaviour)

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_ge225::{
    lower_iir_to_ge225, validate_for_ge225, IIRGe225Config, IIRGe225Error, HALT_WORD,
    LDA_OPCODE_NIBBLE,
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

// ===========================================================================
// §1. Validator stub
// ===========================================================================

#[test]
fn validate_returns_empty_for_empty_module() {
    assert!(validate_for_ge225(&empty_module()).is_empty());
}

// ===========================================================================
// §2. v0.1.0 HLT contract (regression — empty module still halts)
// ===========================================================================

#[test]
fn empty_module_still_emits_the_canonical_halt_word() {
    let bytes = lower_iir_to_ge225(&empty_module(), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![0x00, 0x00, 0x00],
        "v0.2.0 preserves v0.1.0's empty-module contract"
    );
}

#[test]
fn halt_word_constant_pinned_to_zeros() {
    assert_eq!(HALT_WORD, [0x00, 0x00, 0x00]);
}

#[test]
fn lda_opcode_nibble_pinned_to_0x1() {
    // LDA opcode lives in the low 4 bits of byte 0 of the 3-byte
    // word packing; high 4 bits of byte 0 are always zero.
    assert_eq!(LDA_OPCODE_NIBBLE, 0x1);
}

// ===========================================================================
// §3. Config defaults
// ===========================================================================

#[test]
fn default_config_has_nonempty_module_name() {
    assert!(!IIRGe225Config::default().module_name.is_empty());
}

#[test]
fn new_sets_module_name() {
    assert_eq!(IIRGe225Config::new("custom").module_name, "custom");
}

// ===========================================================================
// §4. Error Display
// ===========================================================================

#[test]
fn errors_display_without_panic() {
    let errs = vec![
        IIRGe225Error::ValidationFailed(vec!["x".into()]),
        IIRGe225Error::UnsupportedOp {
            function: "f".into(),
            op: "weird".into(),
        },
        IIRGe225Error::UnsupportedType {
            function: "f".into(),
            type_hint: "Quaternion".into(),
        },
        IIRGe225Error::InvalidOperand {
            function: "f".into(),
            detail: "bad".into(),
        },
        IIRGe225Error::UndefinedVariable {
            function: "f".into(),
            name: "v".into(),
        },
    ];
    for err in errs {
        assert!(!format!("{err}").is_empty());
    }
}

// ===========================================================================
// §5. A5+ — `const` lowers to `LDA n`; `ret` → HLT
// ===========================================================================
//
// Every `const` loads its 16-bit immediate into ACC via a single
// 3-byte LDA word: [0x01, hi, lo].  `ret` requires its src to be
// the current ACC owner and emits HLT.  `ret_void` just emits HLT.
//
// The trivial-case ROM (`const v=N; ret v`) is always 6 bytes — 3
// for LDA, 3 for HLT — regardless of N.

#[test]
fn const_5_then_ret_lowers_to_lda_5_then_halt() {
    // const v=5; ret v
    //   LDA 5 = [0x01, 0x00, 0x05]
    //   HLT   = [0x00, 0x00, 0x00]
    let f = IIRFunction::new(
        "five",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(5)], "i16"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i16"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![0x01, 0x00, 0x05, 0x00, 0x00, 0x00],
        "expected LDA 5 + HLT; got: {bytes:02x?}"
    );
}

#[test]
fn const_0_then_ret_emits_lda_zero() {
    // const v=0; ret v → LDA 0 + HLT
    let f = IIRFunction::new(
        "zero",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0)], "i16"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i16"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![0x01, 0x00, 0x00, 0x00, 0x00, 0x00],
        "LDA 0 byte 0 still has the LDA opcode nibble visible"
    );
}

#[test]
fn const_max_positive_16bit_emits_correct_bytes() {
    // const v=32767 (max i16); ret_void
    //   LDA 0x7FFF = [0x01, 0x7F, 0xFF]
    let f = IIRFunction::new(
        "max",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(32767)], "i16"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0x01, 0x7F, 0xFF, 0x00, 0x00, 0x00]);
}

#[test]
fn const_min_negative_16bit_emits_correct_bytes() {
    // const v=-32768 (min i16); ret_void
    //   -32768 → 0x8000 via two's complement
    //   LDA 0x8000 = [0x01, 0x80, 0x00]
    let f = IIRFunction::new(
        "min",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(-32768)], "i16"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0x01, 0x80, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn const_negative_one_uses_twos_complement() {
    // const v=-1 → 0xFFFF; LDA = [0x01, 0xFF, 0xFF]
    let f = IIRFunction::new(
        "minus_one",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(-1)], "i16"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0x01, 0xFF, 0xFF, 0x00, 0x00, 0x00]);
}

#[test]
fn const_bool_true_emits_lda_one() {
    let f = IIRFunction::new(
        "btrue",
        vec![],
        "bool",
        vec![
            IIRInstr::new("const", Some("b".into()), vec![Operand::Bool(true)], "bool"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0x01, 0x00, 0x01, 0x00, 0x00, 0x00]);
}

#[test]
fn const_bool_false_emits_lda_zero() {
    let f = IIRFunction::new(
        "bfalse",
        vec![],
        "bool",
        vec![
            IIRInstr::new("const", Some("b".into()), vec![Operand::Bool(false)], "bool"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0x01, 0x00, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn const_out_of_range_errors() {
    // 65536 doesn't fit in 16 bits — must error, not silently truncate.
    let f = IIRFunction::new(
        "too_big",
        vec![],
        "i32",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(65536)], "i32"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("65536 must overflow the 16-bit immediate");
    match err {
        IIRGe225Error::InvalidOperand { detail, .. } => {
            assert!(
                detail.contains("16-bit"),
                "error should mention the 16-bit ceiling: {detail}"
            );
        }
        other => panic!("expected InvalidOperand, got {other:?}"),
    }
}

#[test]
fn ret_void_only_emits_just_halt() {
    // No const at all — ret_void alone → 3 bytes HLT.
    let f = IIRFunction::new(
        "noop",
        vec![],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0x00, 0x00, 0x00]);
}

#[test]
fn trivial_rom_is_six_bytes() {
    // The canonical `const v=N; ret v` trivial-case ROM size: 3 bytes
    // LDA + 3 bytes HLT = 6 bytes total, regardless of N.  Pinning this
    // shape catches accidental regressions in the per-instruction byte
    // count.
    for &n in &[0i64, 1, 42, 255, 256, 32767, -1, -32768] {
        let f = IIRFunction::new(
            "trivial",
            vec![],
            "i16",
            vec![
                IIRInstr::new("const", Some("v".into()), vec![Operand::Int(n)], "i16"),
                IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i16"),
            ],
        );
        let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
            .expect("lowering");
        assert_eq!(
            bytes.len(),
            6,
            "trivial ROM for n={n} should be 6 bytes; got: {bytes:02x?}"
        );
        // First 3 bytes must be the LDA word; last 3 must be HLT.
        assert_eq!(bytes[0], 0x01, "byte 0 must be LDA opcode for n={n}");
        assert_eq!(&bytes[3..], &HALT_WORD, "tail must be HLT for n={n}");
    }
}

#[test]
fn multiple_consts_then_ret_of_current_acc_works() {
    // const a=1; const b=2; ret b
    //   LDA 1 + LDA 2 + HLT = 9 bytes
    // b is the current ACC owner; ret b is fine.
    let f = IIRFunction::new(
        "two_consts",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i16"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(2)], "i16"),
            IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i16"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x01, // LDA 1
            0x01, 0x00, 0x02, // LDA 2
            0x00, 0x00, 0x00, // HLT
        ]
    );
}

#[test]
fn ret_of_stale_acc_owner_errors_in_v0_2_0() {
    // const a=1; const b=2; ret a
    //   `a` was overwritten by `b`; v0.2.0 has no register file to
    //   recover the value, so this must error (rather than silently
    //   producing wrong code that returns 2 when the IR says return a).
    //   A5++ will lift this restriction by allocating real GP
    //   registers, mirroring the iir-to-intel4004 v0.3.0 transition.
    let f = IIRFunction::new(
        "stale",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i16"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(2)], "i16"),
            IIRInstr::new("ret", None, vec![Operand::Var("a".into())], "i16"),
        ],
    );
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("ret of stale ACC owner must error");
    match err {
        IIRGe225Error::UndefinedVariable { name, .. } => {
            assert_eq!(name, "a");
        }
        other => panic!("expected UndefinedVariable, got {other:?}"),
    }
}

#[test]
fn ret_of_completely_undefined_variable_errors() {
    // No `const` at all — `ret v` references an unbound var.
    let f = IIRFunction::new(
        "unbound",
        vec![],
        "i16",
        vec![IIRInstr::new(
            "ret",
            None,
            vec![Operand::Var("never_defined".into())],
            "i16",
        )],
    );
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("ret of unbound var must error");
    assert!(matches!(err, IIRGe225Error::UndefinedVariable { .. }));
}

#[test]
fn unsupported_op_errors_with_op_name() {
    // `mov` isn't in v0.2.0's SUPPORTED_OPS — must error explicitly.
    let f = IIRFunction::new(
        "with_mov",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i16"),
            IIRInstr::new("mov", Some("b".into()), vec![Operand::Var("a".into())], "i16"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("mov is not yet supported");
    match err {
        IIRGe225Error::UnsupportedOp { op, .. } => assert_eq!(op, "mov"),
        other => panic!("expected UnsupportedOp, got {other:?}"),
    }
}
