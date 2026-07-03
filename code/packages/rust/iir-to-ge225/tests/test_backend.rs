// Tests for iir-to-ge225 v0.7.0 (A5+++++++ — comparison ops).
//
// Note: this crate is deprecated as of v0.10.0 (Phase 3 of the
// historical-arch backend migration).  Tests still exercise the
// deprecated `lower_iir_to_ge225` function to lock in the
// regression invariant — the `#![allow(deprecated)]` below
// suppresses the otherwise-noisy build warnings.
#![allow(deprecated)]

//
// Mirrors iir-to-intel4004 v0.5.0 / iir-to-armv7 v0.4.x cmp slice
// in spirit, adapted for GE-225's 20-bit-word / 3-bytes-per-word
// packing.
//
// Coverage:
//   §1 — validator stub
//   §2 — opcode constant pinning (BMI now ACTIVELY used)
//   §3 — Config defaults
//   §4 — Error Display
//   §5 — v0.2.0–v0.6.0 regressions
//   §6 — v0.7.0 NEW: cmp_lt / cmp_eq / cmp_ne / cmp_le / cmp_gt / cmp_ge

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_ge225::{
    lower_iir_to_ge225, validate_for_ge225, IIRGe225Config, IIRGe225Error, ADD_OPCODE_NIBBLE,
    BMI_OPCODE_NIBBLE, BNZ_OPCODE_NIBBLE, BR_OPCODE_NIBBLE, BZ_OPCODE_NIBBLE, HALT_WORD,
    JSR_OPCODE_NIBBLE, LDA_OPCODE_NIBBLE, LD_OPCODE_NIBBLE, RTS_OPCODE_NIBBLE, RTS_WORD,
    STA_OPCODE_NIBBLE, SUB_OPCODE_NIBBLE,
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
// §2. Opcode constant pinning (incl. BMI now active)
// ===========================================================================

#[test]
fn halt_word_constant_pinned_to_zeros() {
    assert_eq!(HALT_WORD, [0x00, 0x00, 0x00]);
}

#[test]
fn rts_word_constant_pinned() {
    assert_eq!(RTS_WORD, [0x0A, 0x00, 0x00]);
}

#[test]
fn opcode_nibbles_pinned_through_v0_7_0() {
    assert_eq!(LDA_OPCODE_NIBBLE, 0x1);
    assert_eq!(STA_OPCODE_NIBBLE, 0x2);
    assert_eq!(LD_OPCODE_NIBBLE, 0x3);
    assert_eq!(ADD_OPCODE_NIBBLE, 0x4);
    assert_eq!(SUB_OPCODE_NIBBLE, 0x5);
    assert_eq!(BR_OPCODE_NIBBLE, 0x6);
    assert_eq!(BNZ_OPCODE_NIBBLE, 0x7);
    assert_eq!(BZ_OPCODE_NIBBLE, 0x8);
    assert_eq!(JSR_OPCODE_NIBBLE, 0x9);
    assert_eq!(RTS_OPCODE_NIBBLE, 0xA);
    assert_eq!(BMI_OPCODE_NIBBLE, 0xB); // now actively used by cmp_lt/cmp_le
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
        IIRGe225Error::UndefinedVariable {
            function: "f".into(),
            name: "v".into(),
        },
        IIRGe225Error::BranchTargetOutOfRange {
            function: "f".into(),
            label: "way_off".into(),
            offset: 100_000,
        },
        IIRGe225Error::UndefinedFunction {
            caller: "main".into(),
            callee: "missing".into(),
        },
    ];
    for err in errs {
        assert!(!format!("{err}").is_empty());
    }
}

// ===========================================================================
// §5. Regressions from v0.2.0–v0.6.0
// ===========================================================================

#[test]
fn empty_module_still_emits_halt() {
    let bytes = lower_iir_to_ge225(&empty_module(), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0x00, 0x00, 0x00]);
}

#[test]
fn trivial_rom_still_six_bytes() {
    for &n in &[0i64, 5, -1, 32767] {
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
        assert_eq!(bytes.len(), 6, "trivial ROM stays 6 bytes for n={n}");
    }
}

#[test]
fn trivial_add_still_works() {
    let f = IIRFunction::new(
        "add_test",
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
    assert_eq!(bytes.len(), 21);
}

// ===========================================================================
// §6. v0.7.0 NEW — cmp ops
// ===========================================================================

/// Canonical `cmp_lt c, a, b; ret c` byte sequence.
///
/// Trace (with const a=2; const b=5):
///   0:  LDA 2      (a → ACC, owner=a)
///   3:  STA r0     (evict a → r0)
///   6:  LDA 5      (b → ACC, owner=b)
///   9:  STA r1     (evict b → r1)
///   12: LD r0      (ACC ← a)
///   15: SUB r1     (ACC ← a - b; if a<b, ACC negative)
///   18: BMI 27     (slot — true target = 27)
///   21: LDA 0      (false branch)
///   24: BR 30      (slot — end target = 30)
///   27: LDA 1      (true branch; LDA 1 lands here)
///   30: (end — c is ACC owner; ret c just emits HLT)
///   30: HLT
///
/// Total: 33 bytes (cmp_lt is 21 bytes of materialisation after
/// the const+const eviction prep).
#[test]
fn canonical_cmp_lt_byte_sequence() {
    let f = IIRFunction::new(
        "cmp_lt_test",
        vec![],
        "bool",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(2)], "i16"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(5)], "i16"),
            IIRInstr::new(
                "cmp_lt",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "bool",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "bool"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x02, // 0:  LDA 2
            0x02, 0x00, 0x00, // 3:  STA r0 (evict a)
            0x01, 0x00, 0x05, // 6:  LDA 5
            0x02, 0x00, 0x01, // 9:  STA r1 (evict b)
            0x03, 0x00, 0x00, // 12: LD r0 (ACC ← a)
            0x05, 0x00, 0x01, // 15: SUB r1 (ACC ← a - b)
            0x0B, 0x00, 0x1B, // 18: BMI 27 (true target)
            0x01, 0x00, 0x00, // 21: LDA 0 (false branch)
            0x06, 0x00, 0x1E, // 24: BR 30 (end target)
            0x01, 0x00, 0x01, // 27: LDA 1 (true branch)
            0x00, 0x00, 0x00, // 30: HLT (c in ACC)
        ]
    );
}

/// `cmp_eq` emits BZ instead of BMI.
#[test]
fn cmp_eq_emits_bz_pattern() {
    let f = IIRFunction::new(
        "cmp_eq_test",
        vec![],
        "bool",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(3)], "i16"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(3)], "i16"),
            IIRInstr::new(
                "cmp_eq",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "bool",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "bool"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    // Same layout as cmp_lt but with BZ (0x08) instead of BMI (0x0B).
    let test_byte = bytes[18];
    assert_eq!(test_byte, 0x08, "cmp_eq must use BZ (0x08) at offset 18");
}

/// `cmp_ne` emits BNZ.
#[test]
fn cmp_ne_emits_bnz_pattern() {
    let f = IIRFunction::new(
        "cmp_ne_test",
        vec![],
        "bool",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(3)], "i16"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(4)], "i16"),
            IIRInstr::new(
                "cmp_ne",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "bool",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "bool"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    let test_byte = bytes[18];
    assert_eq!(test_byte, 0x07, "cmp_ne must use BNZ (0x07) at offset 18");
}

/// `cmp_le` emits BOTH BMI and BZ tests pointing at the same true
/// target.  Total cmp materialisation grows from 18 to 21 bytes.
///
/// Trace after const a=2; const b=5:
///   12: LD r0
///   15: SUB r1
///   18: BMI 30   (slot 1, true target)
///   21: BZ 30    (slot 2, same true target)
///   24: LDA 0
///   27: BR 33    (end target)
///   30: LDA 1    (true)
///   33: HLT
///
/// Total bytes: 36.
#[test]
fn canonical_cmp_le_byte_sequence() {
    let f = IIRFunction::new(
        "cmp_le_test",
        vec![],
        "bool",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(2)], "i16"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(5)], "i16"),
            IIRInstr::new(
                "cmp_le",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "bool",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "bool"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x02, // 0: LDA 2
            0x02, 0x00, 0x00, // 3: STA r0
            0x01, 0x00, 0x05, // 6: LDA 5
            0x02, 0x00, 0x01, // 9: STA r1
            0x03, 0x00, 0x00, // 12: LD r0
            0x05, 0x00, 0x01, // 15: SUB r1
            0x0B, 0x00, 0x1E, // 18: BMI 30 (true target)
            0x08, 0x00, 0x1E, // 21: BZ 30 (same true target)
            0x01, 0x00, 0x00, // 24: LDA 0
            0x06, 0x00, 0x21, // 27: BR 33 (end target)
            0x01, 0x00, 0x01, // 30: LDA 1
            0x00, 0x00, 0x00, // 33: HLT
        ]
    );
}

/// `cmp_gt a, b` is `cmp_lt b, a` — operand swap, same BMI pattern.
///
/// Trace after const a=2; const b=5; cmp_gt c, a, b:
/// After swap: lhs=b, rhs=a.  LD r1 (b) then SUB r0 (a), then BMI.
#[test]
fn cmp_gt_uses_operand_swap() {
    let f = IIRFunction::new(
        "cmp_gt_test",
        vec![],
        "bool",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(2)], "i16"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(5)], "i16"),
            IIRInstr::new(
                "cmp_gt",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "bool",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "bool"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    // After eviction, env[a]=r0, env[b]=r1.  cmp_gt swaps to make
    // effective lhs=b (r1), rhs=a (r0).  So LD r1 then SUB r0.
    assert_eq!(&bytes[12..18], &[
        0x03, 0x00, 0x01, // LD r1 (b)
        0x05, 0x00, 0x00, // SUB r0 (a)
    ]);
    // And BMI test (cmp_gt is still single-test).
    assert_eq!(bytes[18], 0x0B, "cmp_gt uses BMI (0x0B) after swap");
}

/// `cmp_ge a, b` is `cmp_le b, a` — operand swap, double-test
/// (BMI + BZ) pattern.
#[test]
fn cmp_ge_uses_operand_swap_and_double_test() {
    let f = IIRFunction::new(
        "cmp_ge_test",
        vec![],
        "bool",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(2)], "i16"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(5)], "i16"),
            IIRInstr::new(
                "cmp_ge",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "bool",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    // After swap: LD r1 (b), SUB r0 (a), then BMI + BZ pattern.
    assert_eq!(&bytes[12..21], &[
        0x03, 0x00, 0x01, // LD r1 (b after swap)
        0x05, 0x00, 0x00, // SUB r0 (a after swap)
        0x0B, 0x00, 0x1E, // BMI 30 (true target)
    ]);
    assert_eq!(bytes[21], 0x08, "cmp_ge double-test must have BZ at offset 21");
}

/// `cmp_lt` with lhs already in ACC: the eviction prep pulls lhs
/// out of ACC into a register, then proceeds as normal.
#[test]
fn cmp_with_lhs_in_acc_evicts_then_runs_normally() {
    let f = IIRFunction::new(
        "cmp_acc",
        vec![],
        "bool",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(5)], "i16"),
            // After this, a is ACC owner.
            IIRInstr::new(
                "cmp_lt",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("a".into())],
                "bool",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    // LDA 5, then evict a → STA r0, then LD r0; SUB r0; BMI...
    assert_eq!(&bytes[0..6], &[
        0x01, 0x00, 0x05, // LDA 5
        0x02, 0x00, 0x00, // STA r0 (evict a)
    ]);
    assert_eq!(&bytes[6..12], &[
        0x03, 0x00, 0x00, // LD r0 (a in ACC)
        0x05, 0x00, 0x00, // SUB r0 (a - a = 0; not negative)
    ]);
    assert_eq!(bytes[12], 0x0B, "BMI at offset 12 after LD+SUB");
}

/// `cmp_lt` with undefined lhs errors crisply.
#[test]
fn cmp_undefined_lhs_errors() {
    let f = IIRFunction::new(
        "cmp_undef_lhs",
        vec![],
        "bool",
        vec![
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(1)], "i16"),
            IIRInstr::new(
                "cmp_lt",
                Some("c".into()),
                vec![Operand::Var("never".into()), Operand::Var("b".into())],
                "bool",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("cmp_lt with undefined lhs must error");
    match err {
        IIRGe225Error::UndefinedVariable { name, .. } => assert_eq!(name, "never"),
        other => panic!("expected UndefinedVariable, got {other:?}"),
    }
}

/// `cmp_eq` with undefined rhs errors.
#[test]
fn cmp_eq_undefined_rhs_errors() {
    let f = IIRFunction::new(
        "cmp_eq_undef_rhs",
        vec![],
        "bool",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i16"),
            IIRInstr::new(
                "cmp_eq",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("missing".into())],
                "bool",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("cmp_eq with undefined rhs must error");
    match err {
        IIRGe225Error::UndefinedVariable { name, .. } => assert_eq!(name, "missing"),
        other => panic!("expected UndefinedVariable, got {other:?}"),
    }
}

/// Comparison result is a real bool living in ACC — feeding it
/// into `jmp_if_true` should skip the redundant LD (because c is
/// the ACC owner after cmp_lt).
///
/// Trace: const a=2; const b=5; cmp_lt c, a, b; jmp_if_true c, skip;
/// label skip; ret_void.
#[test]
fn cmp_result_feeds_directly_into_jmp_if_true() {
    let f = IIRFunction::new(
        "cmp_into_jmp",
        vec![],
        "void",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(2)], "i16"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(5)], "i16"),
            IIRInstr::new(
                "cmp_lt",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "bool",
            ),
            IIRInstr::new(
                "jmp_if_true",
                None,
                vec![Operand::Var("c".into()), Operand::Var("skip".into())],
                "void",
            ),
            IIRInstr::new("label", None, vec![Operand::Var("skip".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    // The byte right after cmp_lt's LDA 1 (offset 30) should be the
    // BNZ for jmp_if_true — no LD prefix because c is current ACC
    // owner.  Layout:
    //   0..30: cmp_lt materialisation (same as canonical test)
    //   30: LDA 1 from cmp_lt true branch (bytes 27..30 are LDA 1)
    // Wait — re-trace: bytes 0..18 are 6 const-prep instructions;
    // bytes 18..30 are BMI/LDA0/BR/LDA1 (12 bytes).  bytes[30] is
    // the start of the next instruction after cmp_lt.
    //
    // Since c lives in ACC after cmp_lt, jmp_if_true skips LD and
    // emits just BNZ.
    assert_eq!(bytes[30], 0x07, "BNZ at offset 30 (no LD prefix because c is ACC owner)");
}

/// `cmp_lt` followed by chained arithmetic to verify ACC-tracking
/// state is correct after cmp: result is in ACC, owner is `c`.
///
/// Test: cmp_lt c, a, b; add d, c, c; ret d
/// After cmp_lt, c is ACC owner.  add d, c, c needs c in a register
/// (evict + LD pattern) — same as v0.4.0 self-add semantics.
#[test]
fn cmp_result_can_be_added() {
    let f = IIRFunction::new(
        "cmp_then_add",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i16"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(2)], "i16"),
            IIRInstr::new(
                "cmp_lt",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "bool",
            ),
            IIRInstr::new(
                "add",
                Some("d".into()),
                vec![Operand::Var("c".into()), Operand::Var("c".into())],
                "i16",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("d".into())], "i16"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    // The test just confirms lowering succeeds and produces a
    // word-aligned byte stream ending in HLT.
    assert_eq!(bytes.len() % 3, 0);
    assert_eq!(&bytes[bytes.len() - 3..], &HALT_WORD);
}

// ===========================================================================
// §8. v0.9.0 NEW — `neg dest, src` lowering (LDA 0 + SUB pattern)
// ===========================================================================

/// Canonical `const v=5; neg w, v; ret w` byte sequence.
///
/// Trace:
///   0:  LDA 5      (v → ACC, owner=v)
///   3:  STA r0     (evict v → r0 because LDA 0 below clobbers ACC)
///   6:  LDA 0      (ACC ← 0)
///   9:  SUB r0     (ACC ← 0 - v = -v; env[w] = ACC, owner=w)
///   12: HLT        (w in ACC for ret w)
/// Total: 15 bytes.
#[test]
fn canonical_neg_byte_sequence() {
    let f = IIRFunction::new(
        "neg_v",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(5)], "i16"),
            IIRInstr::new("neg", Some("w".into()), vec![Operand::Var("v".into())], "i16"),
            IIRInstr::new("ret", None, vec![Operand::Var("w".into())], "i16"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x05, // LDA 5
            0x02, 0x00, 0x00, // STA r0 (evict v)
            0x01, 0x00, 0x00, // LDA 0
            0x05, 0x00, 0x00, // SUB r0
            0x00, 0x00, 0x00, // HLT
        ]
    );
    assert_eq!(bytes.len(), 15);
}

/// `neg` of a register-resident value (after another const evicts
/// it) skips the redundant eviction step.
///
/// Trace: const v=7; const u=3; neg w, v; ret_void
///   0:  LDA 7      (v → ACC)
///   3:  STA r0     (evict v → r0 for next const)
///   6:  LDA 3      (u → ACC, owner=u)
///   9:  STA r1     (evict u → r1 because LDA 0 below clobbers ACC)
///   12: LDA 0      (ACC ← 0)
///   15: SUB r0     (ACC ← 0 - v)
///   18: HLT
#[test]
fn neg_when_src_in_register_skips_first_eviction() {
    let f = IIRFunction::new(
        "neg_reg",
        vec![],
        "void",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "i16"),
            IIRInstr::new("const", Some("u".into()), vec![Operand::Int(3)], "i16"),
            IIRInstr::new("neg", Some("w".into()), vec![Operand::Var("v".into())], "i16"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x07, // LDA 7
            0x02, 0x00, 0x00, // STA r0 (evict v)
            0x01, 0x00, 0x03, // LDA 3
            0x02, 0x00, 0x01, // STA r1 (evict u — done by neg's "evict remaining ACC owner")
            0x01, 0x00, 0x00, // LDA 0
            0x05, 0x00, 0x00, // SUB r0 (negate v)
            0x00, 0x00, 0x00, // HLT
        ]
    );
}

/// Double-neg: `neg w, v; neg x, w; ret x` should yield x = v
/// (the byte trace is what we pin, not the semantic).
#[test]
fn double_neg_works() {
    let f = IIRFunction::new(
        "double_neg",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "i16"),
            IIRInstr::new("neg", Some("w".into()), vec![Operand::Var("v".into())], "i16"),
            IIRInstr::new("neg", Some("x".into()), vec![Operand::Var("w".into())], "i16"),
            IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i16"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    // Just confirm lowering succeeds and ends with HLT.  Tracing
    // every byte for chained negs is over-pinning.
    assert_eq!(bytes.len() % 3, 0, "word-aligned");
    assert_eq!(&bytes[bytes.len() - 3..], &HALT_WORD);
    assert!(
        bytes.len() >= 15,
        "double neg should be at least the single-neg ROM size"
    );
}

/// `neg` of an undefined src errors crisply.
#[test]
fn neg_undefined_src_errors() {
    let f = IIRFunction::new(
        "neg_undef",
        vec![],
        "i16",
        vec![
            IIRInstr::new("neg", Some("w".into()), vec![Operand::Var("never".into())], "i16"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("neg of undefined src must error");
    match err {
        IIRGe225Error::UndefinedVariable { name, .. } => assert_eq!(name, "never"),
        other => panic!("expected UndefinedVariable, got {other:?}"),
    }
}

/// `neg` with no srcs errors with `InvalidOperand`.
#[test]
fn neg_no_srcs_errors() {
    let f = IIRFunction::new(
        "neg_empty",
        vec![],
        "i16",
        vec![
            IIRInstr::new("neg", Some("w".into()), vec![], "i16"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("neg without src must error");
    assert!(matches!(err, IIRGe225Error::InvalidOperand { .. }));
}

/// `neg` feeds into `ret` cleanly — the result is in ACC, no LD
/// prefix needed.
#[test]
fn neg_result_feeds_directly_into_ret() {
    let f = IIRFunction::new(
        "neg_then_ret",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "i16"),
            IIRInstr::new("neg", Some("w".into()), vec![Operand::Var("v".into())], "i16"),
            IIRInstr::new("ret", None, vec![Operand::Var("w".into())], "i16"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    // After SUB r0, w lives in ACC. ret w finds w == acc_owner and
    // emits just HLT — no LD prefix.
    assert_eq!(&bytes[bytes.len() - 3..], &HALT_WORD);
}

// ===========================================================================
// §7. v0.8.0 NEW — call_builtin no-op lowering
// ===========================================================================

/// `call_builtin print_i64, v` with no dest emits ZERO bytes —
/// the entire instruction collapses to a no-op.
///
/// Trace: const v=5; call_builtin print_i64, v; ret_void
///   0: LDA 5    (v → ACC, owner=v)
///   3: (call_builtin emits 0 bytes — no I/O opcode on this skeleton)
///   3: HLT      (entry function ret_void)
/// Total: 6 bytes (same as the trivial-case ROM).
#[test]
fn call_builtin_no_dest_emits_zero_bytes() {
    let f = IIRFunction::new(
        "print_v",
        vec![],
        "void",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(5)], "i16"),
            IIRInstr::new(
                "call_builtin",
                None,
                vec![Operand::Var("print_i64".into()), Operand::Var("v".into())],
                "void",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x05, // LDA 5
            0x00, 0x00, 0x00, // HLT (entry function ret_void)
        ],
        "call_builtin without dest must emit zero bytes; got: {bytes:02x?}"
    );
    assert_eq!(bytes.len(), 6, "trivial print-of-const should still be 6 bytes");
}

/// `call_builtin input_i64` WITH dest emits `LDA 0` (deterministic
/// placeholder return value), then dest claims ACC.
///
/// Trace: call_builtin x = input_i64; ret x
///   0: LDA 0   (placeholder return value; x → ACC, owner=x)
///   3: HLT     (entry function ret x — x in ACC, no LD needed)
#[test]
fn call_builtin_with_dest_emits_lda_zero() {
    let f = IIRFunction::new(
        "read_x",
        vec![],
        "i64",
        vec![
            IIRInstr::new(
                "call_builtin",
                Some("x".into()),
                vec![Operand::Var("input_i64".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i64"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x00, // LDA 0 (placeholder return)
            0x00, 0x00, 0x00, // HLT (x in ACC)
        ]
    );
}

/// `call_builtin` with an undefined arg var errors crisply.
#[test]
fn call_builtin_undefined_arg_errors() {
    let f = IIRFunction::new(
        "print_undef",
        vec![],
        "void",
        vec![
            IIRInstr::new(
                "call_builtin",
                None,
                vec![Operand::Var("print_i64".into()), Operand::Var("never_bound".into())],
                "void",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("call_builtin with undefined arg must error");
    match err {
        IIRGe225Error::UndefinedVariable { name, .. } => assert_eq!(name, "never_bound"),
        other => panic!("expected UndefinedVariable, got {other:?}"),
    }
}

/// `call_builtin` with no srcs (missing builtin name) errors with
/// `InvalidOperand`.
#[test]
fn call_builtin_no_srcs_errors() {
    let f = IIRFunction::new(
        "bad",
        vec![],
        "void",
        vec![
            IIRInstr::new("call_builtin", None, vec![], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("call_builtin without builtin name must error");
    assert!(matches!(err, IIRGe225Error::InvalidOperand { .. }));
}

/// `call_builtin` evicts a live ACC owner when a dest is bound
/// (so the LDA 0 doesn't lose existing state).
///
/// Trace: const a=5; call_builtin x = input_i64; ret a
///   0: LDA 5     (a → ACC)
///   3: STA r0    (evict a → r0 because call_builtin with dest emits LDA 0)
///   6: LDA 0     (x → ACC)
///   9: LD r0     (reload a for ret)
///   12: HLT
#[test]
fn call_builtin_with_dest_evicts_acc_owner() {
    let f = IIRFunction::new(
        "save_a",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(5)], "i16"),
            IIRInstr::new(
                "call_builtin",
                Some("x".into()),
                vec![Operand::Var("input_i64".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("a".into())], "i64"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x05, // LDA 5
            0x02, 0x00, 0x00, // STA r0 (evict a)
            0x01, 0x00, 0x00, // LDA 0 (x placeholder)
            0x03, 0x00, 0x00, // LD r0 (reload a for ret)
            0x00, 0x00, 0x00, // HLT
        ]
    );
}

/// Unsupported op (e.g., `mul`) still errors.
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
