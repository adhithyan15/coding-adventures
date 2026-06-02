// Tests for iir-to-ge225 v0.4.0 (A5+++ — accumulator arithmetic).
//
// Mirrors iir-to-intel4004 v0.4.0's test set in spirit, adapted for
// GE-225's 20-bit-word / 3-bytes-per-word packing.
//
// Coverage:
//   §1 — validator stub
//   §2 — opcode constant pinning (incl. new ADD/SUB)
//   §3 — Config defaults
//   §4 — Error Display (still 6 variants)
//   §5 — v0.2.0 / v0.3.0 regressions
//   §6 — v0.4.0 NEW: ADD / SUB lowering

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_ge225::{
    lower_iir_to_ge225, validate_for_ge225, IIRGe225Config, IIRGe225Error, ADD_OPCODE_NIBBLE,
    HALT_WORD, LDA_OPCODE_NIBBLE, LD_OPCODE_NIBBLE, STA_OPCODE_NIBBLE, SUB_OPCODE_NIBBLE,
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
// §2. Opcode constant pinning
// ===========================================================================

#[test]
fn halt_word_constant_pinned_to_zeros() {
    assert_eq!(HALT_WORD, [0x00, 0x00, 0x00]);
}

#[test]
fn lda_opcode_nibble_pinned_to_0x1() {
    assert_eq!(LDA_OPCODE_NIBBLE, 0x1);
}

#[test]
fn sta_opcode_nibble_pinned_to_0x2() {
    assert_eq!(STA_OPCODE_NIBBLE, 0x2);
}

#[test]
fn ld_opcode_nibble_pinned_to_0x3() {
    assert_eq!(LD_OPCODE_NIBBLE, 0x3);
}

#[test]
fn add_opcode_nibble_pinned_to_0x4() {
    assert_eq!(ADD_OPCODE_NIBBLE, 0x4);
}

#[test]
fn sub_opcode_nibble_pinned_to_0x5() {
    assert_eq!(SUB_OPCODE_NIBBLE, 0x5);
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
        IIRGe225Error::OutOfRegisters {
            function: "f".into(),
            name: "v17".into(),
        },
    ];
    for err in errs {
        assert!(!format!("{err}").is_empty());
    }
}

// ===========================================================================
// §5. Regressions from v0.2.0 / v0.3.0
// ===========================================================================

#[test]
fn empty_module_still_emits_the_canonical_halt_word() {
    let bytes = lower_iir_to_ge225(&empty_module(), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0x00, 0x00, 0x00]);
}

#[test]
fn trivial_rom_is_still_six_bytes() {
    for &n in &[0i64, 5, 42, -1, 32767, -32768] {
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
        assert_eq!(bytes.len(), 6, "trivial ROM for n={n} should be 6 bytes");
    }
}

#[test]
fn ret_void_only_still_emits_just_halt() {
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
fn const_out_of_range_still_errors() {
    let f = IIRFunction::new(
        "too_big",
        vec![],
        "i32",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(70_000)], "i32"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("70_000 overflows the 16-bit ceiling");
    assert!(matches!(err, IIRGe225Error::InvalidOperand { .. }));
}

#[test]
fn mov_still_works() {
    // const a=7; mov b, a; ret b — same byte sequence as v0.3.0.
    let f = IIRFunction::new(
        "mov_chain",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(7)], "i16"),
            IIRInstr::new("mov", Some("b".into()), vec![Operand::Var("a".into())], "i16"),
            IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i16"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x07, // LDA 7
            0x02, 0x00, 0x00, // STA r0 (evict a)
            0x03, 0x00, 0x00, // LD r0
            0x02, 0x00, 0x01, // STA r1 (b = ACC)
            0x03, 0x00, 0x01, // LD r1 (reload b for ret)
            0x00, 0x00, 0x00, // HLT
        ]
    );
}

// ===========================================================================
// §6. v0.4.0 NEW — accumulator arithmetic
// ===========================================================================

/// `const a=3; const b=4; add c, a, b; ret c` — the canonical
/// trivial-add ROM.
///
/// State after each instruction:
///   LDA 3                      ACC=a=3, owner=a       (3 bytes)
///   STA r0  (evict a → r0)     env[a]=r0, owner=None  (6 bytes)
///   LDA 4                      ACC=b=4, owner=b       (9 bytes)
///   add c, a, b:
///     - lhs=a in r0, skip eviction
///     - rhs=b in ACC → STA r1  env[b]=r1, owner=None  (12 bytes)
///     - final evict_acc no-op
///     LD r0                    ACC=a=3                (15 bytes)
///     ADD r1                   ACC=a+b=7              (18 bytes)
///   env[c]=ACC, owner=c
///   ret c: c is ACC owner, just HLT                  (21 bytes)
#[test]
fn trivial_add_byte_sequence() {
    let f = IIRFunction::new(
        "trivial_add",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(3)], "i16"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(4)], "i16"),
            IIRInstr::new(
                "add",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i16",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i16"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x03, // LDA 3   (a → ACC)
            0x02, 0x00, 0x00, // STA r0  (evict a → r0)
            0x01, 0x00, 0x04, // LDA 4   (b → ACC)
            0x02, 0x00, 0x01, // STA r1  (evict b → r1)
            0x03, 0x00, 0x00, // LD r0   (ACC ← a)
            0x04, 0x00, 0x01, // ADD r1  (ACC ← a + b)
            0x00, 0x00, 0x00, // HLT
        ],
        "expected 7-word add ROM; got: {bytes:02x?}"
    );
    assert_eq!(bytes.len(), 21);
}

/// `const a=10; const b=3; sub c, a, b; ret c` — same structure as
/// trivial_add but with the SUB opcode in the arith step.
#[test]
fn trivial_sub_byte_sequence() {
    let f = IIRFunction::new(
        "trivial_sub",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(10)], "i16"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(3)], "i16"),
            IIRInstr::new(
                "sub",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i16",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i16"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x0A, // LDA 10  (a → ACC)
            0x02, 0x00, 0x00, // STA r0  (evict a)
            0x01, 0x00, 0x03, // LDA 3   (b → ACC)
            0x02, 0x00, 0x01, // STA r1  (evict b)
            0x03, 0x00, 0x00, // LD r0   (ACC ← a)
            0x05, 0x00, 0x01, // SUB r1  (ACC ← a - b)
            0x00, 0x00, 0x00, // HLT
        ]
    );
}

/// `const a=2; add c, a, a; ret c` — self-add: lhs and rhs are the
/// same variable.  Lhs in ACC gets evicted, rhs is then in the same
/// register, and the LD/ADD pair references r0 twice.
#[test]
fn self_add_uses_same_register_twice() {
    let f = IIRFunction::new(
        "self_add",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(2)], "i16"),
            IIRInstr::new(
                "add",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("a".into())],
                "i16",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i16"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x02, // LDA 2   (a → ACC)
            0x02, 0x00, 0x00, // STA r0  (evict a)
            0x03, 0x00, 0x00, // LD r0   (ACC ← a)
            0x04, 0x00, 0x00, // ADD r0  (ACC ← a + a)
            0x00, 0x00, 0x00, // HLT
        ]
    );
    assert_eq!(bytes.len(), 15);
}

/// Chained arithmetic: `(a + b) + d`.  Demonstrates that the result
/// of the first add becomes a register-bound operand of the second.
///
/// const a=1; const b=2; add c, a, b; const d=4; add e, c, d; ret e
#[test]
fn chained_add_works() {
    let f = IIRFunction::new(
        "chained_add",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i16"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(2)], "i16"),
            IIRInstr::new(
                "add",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i16",
            ),
            IIRInstr::new("const", Some("d".into()), vec![Operand::Int(4)], "i16"),
            IIRInstr::new(
                "add",
                Some("e".into()),
                vec![Operand::Var("c".into()), Operand::Var("d".into())],
                "i16",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("e".into())], "i16"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    // Word count audit:
    //   LDA 1, STA r0       (a)
    //   LDA 2, STA r1       (b — eviction happens because a was already evicted; here it's b being evicted before LD)
    //   LD r0, ADD r1       (c = a + b)  c→ACC owner
    //   STA r2              (evict c when next const arrives)
    //   LDA 4               (d)
    //   STA r3              (evict d before LD r2 in next add)
    //   LD r2, ADD r3       (e = c + d)
    //   HLT
    // = 12 words = 36 bytes.
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x01, // LDA 1
            0x02, 0x00, 0x00, // STA r0
            0x01, 0x00, 0x02, // LDA 2
            0x02, 0x00, 0x01, // STA r1
            0x03, 0x00, 0x00, // LD r0
            0x04, 0x00, 0x01, // ADD r1
            0x02, 0x00, 0x02, // STA r2 (evict c for next const)
            0x01, 0x00, 0x04, // LDA 4
            0x02, 0x00, 0x03, // STA r3 (evict d for add)
            0x03, 0x00, 0x02, // LD r2
            0x04, 0x00, 0x03, // ADD r3
            0x00, 0x00, 0x00, // HLT
        ]
    );
    assert_eq!(bytes.len(), 36);
}

/// `add` of an undefined LHS errors with `UndefinedVariable`.
#[test]
fn add_undefined_lhs_errors() {
    let f = IIRFunction::new(
        "add_undef_lhs",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(1)], "i16"),
            IIRInstr::new(
                "add",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i16",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("add with undefined lhs must error");
    match err {
        IIRGe225Error::UndefinedVariable { name, .. } => assert_eq!(name, "a"),
        other => panic!("expected UndefinedVariable, got {other:?}"),
    }
}

/// `add` of an undefined RHS errors with `UndefinedVariable`.
#[test]
fn add_undefined_rhs_errors() {
    let f = IIRFunction::new(
        "add_undef_rhs",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i16"),
            IIRInstr::new(
                "add",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i16",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("add with undefined rhs must error");
    match err {
        IIRGe225Error::UndefinedVariable { name, .. } => assert_eq!(name, "b"),
        other => panic!("expected UndefinedVariable, got {other:?}"),
    }
}

/// `sub` of an undefined RHS errors with `UndefinedVariable`.
#[test]
fn sub_undefined_rhs_errors() {
    let f = IIRFunction::new(
        "sub_undef_rhs",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(5)], "i16"),
            IIRInstr::new(
                "sub",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("unbound".into())],
                "i16",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("sub with undefined rhs must error");
    assert!(matches!(err, IIRGe225Error::UndefinedVariable { .. }));
}

/// `add` with a non-Var operand (e.g., immediate) errors with
/// `InvalidOperand` — v0.4.0 requires both operands to be SSA vars.
#[test]
fn add_with_immediate_operand_errors() {
    let f = IIRFunction::new(
        "add_imm",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i16"),
            // rhs is Int(2), not Var — should error.
            IIRInstr::new(
                "add",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Int(2)],
                "i16",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("add with immediate rhs must error");
    match err {
        IIRGe225Error::InvalidOperand { detail, .. } => {
            assert!(detail.contains("Var"), "got detail: {detail}");
        }
        other => panic!("expected InvalidOperand, got {other:?}"),
    }
}

/// `mul` is not in v0.4.0's SUPPORTED_OPS — should error with
/// `UnsupportedOp`.
#[test]
fn mul_still_unsupported() {
    let f = IIRFunction::new(
        "with_mul",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(2)], "i16"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(3)], "i16"),
            IIRInstr::new(
                "mul",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i16",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("mul is not yet supported");
    match err {
        IIRGe225Error::UnsupportedOp { op, .. } => assert_eq!(op, "mul"),
        other => panic!("expected UnsupportedOp, got {other:?}"),
    }
}
