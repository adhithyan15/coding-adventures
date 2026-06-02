// Tests for iir-to-ge225 v0.5.0 (A5+++++ — branch family + labels).
//
// Mirrors iir-to-intel8008 v0.3.4 (jump+label slice) in spirit,
// adapted for GE-225's 20-bit-word / 3-bytes-per-word packing.
//
// Coverage:
//   §1 — validator stub
//   §2 — opcode constant pinning (incl. new BR/BNZ/BZ)
//   §3 — Config defaults
//   §4 — Error Display (8 variants now)
//   §5 — v0.2.0 / v0.3.0 / v0.4.0 regressions
//   §6 — v0.5.0 NEW: label / jmp / jmp_if_true / jmp_if_false +
//        per-function backpatching

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_ge225::{
    lower_iir_to_ge225, validate_for_ge225, IIRGe225Config, IIRGe225Error, ADD_OPCODE_NIBBLE,
    BNZ_OPCODE_NIBBLE, BR_OPCODE_NIBBLE, BZ_OPCODE_NIBBLE, HALT_WORD, LDA_OPCODE_NIBBLE,
    LD_OPCODE_NIBBLE, STA_OPCODE_NIBBLE, SUB_OPCODE_NIBBLE,
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
fn opcode_nibbles_pinned() {
    assert_eq!(LDA_OPCODE_NIBBLE, 0x1);
    assert_eq!(STA_OPCODE_NIBBLE, 0x2);
    assert_eq!(LD_OPCODE_NIBBLE, 0x3);
    assert_eq!(ADD_OPCODE_NIBBLE, 0x4);
    assert_eq!(SUB_OPCODE_NIBBLE, 0x5);
    assert_eq!(BR_OPCODE_NIBBLE, 0x6);
    assert_eq!(BNZ_OPCODE_NIBBLE, 0x7);
    assert_eq!(BZ_OPCODE_NIBBLE, 0x8);
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
        IIRGe225Error::UndefinedLabel {
            function: "f".into(),
            label: "skip".into(),
        },
        IIRGe225Error::BranchTargetOutOfRange {
            function: "f".into(),
            label: "way_off".into(),
            offset: 100_000,
        },
    ];
    for err in errs {
        assert!(!format!("{err}").is_empty());
    }
}

// ===========================================================================
// §5. Regressions from v0.2.0 / v0.3.0 / v0.4.0
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
fn trivial_add_still_works() {
    // const a=3; const b=4; add c, a, b; ret c — unchanged from v0.4.0.
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
    assert_eq!(bytes.len(), 21, "trivial-add ROM unchanged at 21 bytes");
}

// ===========================================================================
// §6. v0.5.0 NEW — label / jmp / jmp_if_true / jmp_if_false
// ===========================================================================

/// `label start; ret_void` — labels emit zero bytes; just HLT.
#[test]
fn label_only_emits_no_bytes() {
    let f = IIRFunction::new(
        "label_only",
        vec![],
        "void",
        vec![
            IIRInstr::new("label", None, vec![Operand::Var("start".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0x00, 0x00, 0x00]);
}

/// `jmp x; label x; ret_void` — the `BR` placeholder is backpatched
/// to address 3 (just past the BR itself).
///
/// Bytes:
///   0: 0x06 0x00 0x03    BR 3  (backpatched: target at byte 3)
///   3: 0x00 0x00 0x00    HLT (label x lands here, no bytes emitted)
#[test]
fn trivial_jmp_emits_br_with_backpatched_address() {
    let f = IIRFunction::new(
        "trivial_jmp",
        vec![],
        "void",
        vec![
            IIRInstr::new("jmp", None, vec![Operand::Var("x".into())], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("x".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x06, 0x00, 0x03, // BR 0x0003 (label x)
            0x00, 0x00, 0x00, // HLT
        ]
    );
}

/// `jmp undefined` errors with `UndefinedLabel`.
#[test]
fn jmp_to_undefined_label_errors() {
    let f = IIRFunction::new(
        "jmp_undef",
        vec![],
        "void",
        vec![
            IIRInstr::new("jmp", None, vec![Operand::Var("nowhere".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("jmp to undefined label must error");
    match err {
        IIRGe225Error::UndefinedLabel { label, .. } => assert_eq!(label, "nowhere"),
        other => panic!("expected UndefinedLabel, got {other:?}"),
    }
}

/// Backward jump (label before jmp) — labels[loop] = 0, jmp loop
/// at offset 0 emits BR 0x0000.  Infinite loop, but the byte
/// sequence is correct.
#[test]
fn backward_jmp_resolves_correctly() {
    let f = IIRFunction::new(
        "loop_forever",
        vec![],
        "void",
        vec![
            IIRInstr::new("label", None, vec![Operand::Var("top".into())], "void"),
            IIRInstr::new("jmp", None, vec![Operand::Var("top".into())], "void"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x06, 0x00, 0x00, // BR 0x0000 (backward jump to top of function)
        ]
    );
}

/// `const cond=1; jmp_if_true cond, skip; const x=99; label skip;
/// ret_void` — exercises BNZ with a forward-backpatched address.
///
/// Trace:
///   0: LDA 1                  cond → ACC, owner=cond
///   3: BNZ <skip>             (cond is ACC owner, no LD)
///   6: STA r0                 evict cond → r0 for next const
///   9: LDA 99                 99 → ACC, owner=x
///   12: HLT                   label skip lands here
///
/// Backpatching: skip = 12 → BNZ slot at bytes 4..6 gets `0x00 0x0C`.
#[test]
fn jmp_if_true_with_cond_in_acc_skips_ld() {
    let f = IIRFunction::new(
        "if_then",
        vec![],
        "void",
        vec![
            IIRInstr::new("const", Some("cond".into()), vec![Operand::Int(1)], "bool"),
            IIRInstr::new(
                "jmp_if_true",
                None,
                vec![Operand::Var("cond".into()), Operand::Var("skip".into())],
                "void",
            ),
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(99)], "i16"),
            IIRInstr::new("label", None, vec![Operand::Var("skip".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x01, // LDA 1   (cond=1 → ACC)
            0x07, 0x00, 0x0C, // BNZ 12  (skip)
            0x02, 0x00, 0x00, // STA r0  (evict cond before next LDA)
            0x01, 0x00, 0x63, // LDA 99  (x=99 → ACC)
            0x00, 0x00, 0x00, // HLT     (label skip)
        ]
    );
}

/// `jmp_if_false` mirror — uses BZ instead of BNZ.
#[test]
fn jmp_if_false_with_cond_in_acc_emits_bz() {
    let f = IIRFunction::new(
        "if_false",
        vec![],
        "void",
        vec![
            IIRInstr::new("const", Some("cond".into()), vec![Operand::Int(0)], "bool"),
            IIRInstr::new(
                "jmp_if_false",
                None,
                vec![Operand::Var("cond".into()), Operand::Var("skip".into())],
                "void",
            ),
            IIRInstr::new("label", None, vec![Operand::Var("skip".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x00, // LDA 0   (cond=0 → ACC)
            0x08, 0x00, 0x06, // BZ 6    (skip — label is 6 bytes in)
            0x00, 0x00, 0x00, // HLT
        ]
    );
}

/// `jmp_if_true` where cond was evicted to a register first — must
/// emit an LD before the BNZ.
///
///   const x=42;     (x → ACC)
///   const cond=1;   (evict x → r0, cond → ACC)
///   const y=7;      (evict cond → r1, y → ACC)
///   jmp_if_true cond, skip   -- cond is in r1, must LD r1
///   label skip
///   ret_void
#[test]
fn jmp_if_true_with_cond_in_register_emits_ld() {
    let f = IIRFunction::new(
        "evicted_cond",
        vec![],
        "void",
        vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(42)], "i16"),
            IIRInstr::new("const", Some("cond".into()), vec![Operand::Int(1)], "bool"),
            IIRInstr::new("const", Some("y".into()), vec![Operand::Int(7)], "i16"),
            IIRInstr::new(
                "jmp_if_true",
                None,
                vec![Operand::Var("cond".into()), Operand::Var("skip".into())],
                "void",
            ),
            IIRInstr::new("label", None, vec![Operand::Var("skip".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    // Trace:
    //   0: LDA 42       (x → ACC, owner=x)
    //   3: STA r0       (evict x → r0 for next const)
    //   6: LDA 1        (cond → ACC, owner=cond)
    //   9: STA r1       (evict cond → r1 for next const)
    //   12: LDA 7       (y → ACC, owner=y)
    //   15: LD r1       (load cond into ACC for BNZ)
    //   18: BNZ <skip>  (backpatched to 21)
    //   21: HLT         (label skip lands here)
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x2A, // LDA 42
            0x02, 0x00, 0x00, // STA r0 (evict x)
            0x01, 0x00, 0x01, // LDA 1
            0x02, 0x00, 0x01, // STA r1 (evict cond)
            0x01, 0x00, 0x07, // LDA 7
            0x03, 0x00, 0x01, // LD r1  (reload cond for BNZ)
            0x07, 0x00, 0x15, // BNZ 21 (skip)
            0x00, 0x00, 0x00, // HLT
        ]
    );
}

/// The canonical "if-then-else" lowering:
///   const c = 1
///   jmp_if_false c, else_label
///   const a = 10            (then branch)
///   jmp end_label
///   label else_label
///   const a = 20            (else branch)
///   label end_label
///   ret_void
#[test]
fn canonical_if_then_else_sequence() {
    let f = IIRFunction::new(
        "if_then_else",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("c".into()), vec![Operand::Int(1)], "bool"),
            IIRInstr::new(
                "jmp_if_false",
                None,
                vec![Operand::Var("c".into()), Operand::Var("else_label".into())],
                "void",
            ),
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(10)], "i16"),
            IIRInstr::new("jmp", None, vec![Operand::Var("end_label".into())], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("else_label".into())], "void"),
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(20)], "i16"),
            IIRInstr::new("label", None, vec![Operand::Var("end_label".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    // Trace:
    //   0:  LDA 1               (c → ACC, owner=c)
    //   3:  BZ <else_label>     (cond in ACC, no LD)
    //   6:  STA r0               (evict c)
    //   9:  LDA 10               (a=10 → ACC)
    //   12: BR <end_label>
    //   15: <else_label>: STA r1 (evict a)  -- wait, a was already
    //       evicted on the then-branch... but env still says a
    //       points to ACC since the second `const a` will re-overwrite.
    //
    // Actually after the then-branch:
    //   - a is ACC owner (env[a] = ACC_MARKER)
    //   - r0=c, r1 unused
    //   - At label else_label: next const a=20 needs to evict ACC owner
    //     (currently a). So STA r1, then LDA 20 (a → ACC).
    //
    //   15: STA r1               (evict a from then branch → r1)
    //   18: LDA 20               (a=20 → ACC)
    //   21: HLT                   (label end_label lands here, ret_void)
    //
    // Backpatched: else_label at byte 15, end_label at byte 21.
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x01, // LDA 1
            0x08, 0x00, 0x0F, // BZ 15 (else_label)
            0x02, 0x00, 0x00, // STA r0 (evict c)
            0x01, 0x00, 0x0A, // LDA 10 (a → ACC)
            0x06, 0x00, 0x15, // BR 21 (end_label)
            0x02, 0x00, 0x01, // STA r1 (evict a from then branch)
            0x01, 0x00, 0x14, // LDA 20 (a → ACC, else branch)
            0x00, 0x00, 0x00, // HLT (end_label)
        ]
    );
}

/// `jmp_if_true` with cond referencing an unbound var errors with
/// `UndefinedVariable`.
#[test]
fn jmp_if_true_with_unbound_cond_errors() {
    let f = IIRFunction::new(
        "unbound_cond",
        vec![],
        "void",
        vec![
            IIRInstr::new(
                "jmp_if_true",
                None,
                vec![Operand::Var("never_bound".into()), Operand::Var("skip".into())],
                "void",
            ),
            IIRInstr::new("label", None, vec![Operand::Var("skip".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("jmp_if_true with unbound cond must error");
    match err {
        IIRGe225Error::UndefinedVariable { name, .. } => assert_eq!(name, "never_bound"),
        other => panic!("expected UndefinedVariable, got {other:?}"),
    }
}

/// Labels are per-function — referencing a label defined in
/// another function errors with `UndefinedLabel`.
#[test]
fn cross_function_labels_dont_resolve() {
    let f1 = IIRFunction::new(
        "f1",
        vec![],
        "void",
        vec![
            IIRInstr::new("label", None, vec![Operand::Var("shared".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let f2 = IIRFunction::new(
        "f2",
        vec![],
        "void",
        vec![
            IIRInstr::new("jmp", None, vec![Operand::Var("shared".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let module = IIRModule {
        name: "two".into(),
        functions: vec![f1, f2],
        entry_point: Some("f1".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let err = lower_iir_to_ge225(&module, &IIRGe225Config::default())
        .expect_err("cross-function label reference must error");
    match err {
        IIRGe225Error::UndefinedLabel { function, label } => {
            assert_eq!(function, "f2");
            assert_eq!(label, "shared");
        }
        other => panic!("expected UndefinedLabel, got {other:?}"),
    }
}

/// Unsupported op (e.g., `mul`) still errors with `UnsupportedOp`.
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
