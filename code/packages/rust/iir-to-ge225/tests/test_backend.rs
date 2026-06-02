// Tests for iir-to-ge225 v0.3.0 (A5++ — ACC-first GP register
// allocator + mov + STA/LD opcodes).
//
// Mirrors iir-to-intel4004 v0.3.0's test set in spirit, adapted for
// GE-225's 20-bit-word / 3-bytes-per-word packing.
//
// Coverage:
//   §1 — validator stub
//   §2 — HLT shape + constant pinning (regressions from v0.1.0 / v0.2.0)
//   §3 — Config defaults
//   §4 — Error Display (all 6 variants now)
//   §5 — v0.2.0 regressions (trivial 6-byte ROM, LDA opcode)
//   §6 — v0.3.0 NEW: ACC-first allocator + mov + STA/LD

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_ge225::{
    lower_iir_to_ge225, validate_for_ge225, IIRGe225Config, IIRGe225Error, HALT_WORD,
    LDA_OPCODE_NIBBLE, LD_OPCODE_NIBBLE, STA_OPCODE_NIBBLE,
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
// §2. v0.1.0 HLT contract (regression)
// ===========================================================================

#[test]
fn empty_module_still_emits_the_canonical_halt_word() {
    let bytes = lower_iir_to_ge225(&empty_module(), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0x00, 0x00, 0x00]);
}

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
// §5. v0.2.0 regressions — trivial-case ROM size, single-const LDA
// ===========================================================================

#[test]
fn trivial_rom_is_still_six_bytes() {
    // const v=N; ret v — one const, no eviction.
    // LDA N + HLT = 6 bytes, unchanged from v0.2.0.
    for &n in &[0i64, 1, 42, 32767, -1, -32768] {
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
            "trivial ROM for n={n} should stay 6 bytes; got: {bytes:02x?}"
        );
        assert_eq!(bytes[0], 0x01, "first byte must be LDA opcode for n={n}");
        assert_eq!(&bytes[3..], &HALT_WORD, "tail must be HLT for n={n}");
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

// ===========================================================================
// §6. v0.3.0 NEW — ACC-first allocator + STA/LD + mov
// ===========================================================================

/// `const a=1; const b=2; ret b` — b is the current ACC owner, so
/// ret needs no `LD` reload.  The second const evicts `a` to r0 via
/// `STA r0` before its `LDA 2` would clobber ACC.
///
/// Word sequence: `LDA 1` + `STA r0` + `LDA 2` + `HLT` = 4 words = 12 bytes.
#[test]
fn two_consts_then_ret_of_current_acc_evicts_first_to_r0() {
    let f = IIRFunction::new(
        "two_consts_ret_b",
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
            0x01, 0x00, 0x01, // LDA 1   (ACC = a)
            0x02, 0x00, 0x00, // STA r0  (evict a → r0)
            0x01, 0x00, 0x02, // LDA 2   (ACC = b)
            0x00, 0x00, 0x00, // HLT
        ],
        "expected LDA+STA+LDA+HLT (4 words = 12 bytes); got: {bytes:02x?}"
    );
}

/// `const a=1; const b=2; ret a` — a was evicted to r0; ret needs
/// `LD r0` to reload it into ACC before halting.
///
/// Word sequence: `LDA 1` + `STA r0` + `LDA 2` + `LD r0` + `HLT` = 5 words = 15 bytes.
#[test]
fn ret_of_evicted_var_emits_ld_to_reload() {
    let f = IIRFunction::new(
        "two_consts_ret_a",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i16"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(2)], "i16"),
            IIRInstr::new("ret", None, vec![Operand::Var("a".into())], "i16"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x01, // LDA 1   (ACC = a)
            0x02, 0x00, 0x00, // STA r0  (evict a → r0)
            0x01, 0x00, 0x02, // LDA 2   (ACC = b)
            0x03, 0x00, 0x00, // LD r0   (reload a into ACC)
            0x00, 0x00, 0x00, // HLT
        ],
        "expected reload-via-LD pattern; got: {bytes:02x?}"
    );
}

/// `const a=7; mov b, a; ret b` — `mov` when src lives in ACC:
/// evict src first (STA r0), LD r0 (refresh ACC with src), STA r1
/// (copy ACC into r1 for dest).  Then ret b is `LD r1` + `HLT`.
///
/// Word sequence: LDA 7 + STA r0 + LD r0 + STA r1 + LD r1 + HLT = 6 words = 18 bytes.
#[test]
fn mov_when_src_in_acc_evicts_then_ld_sta() {
    let f = IIRFunction::new(
        "mov_from_acc",
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
            0x01, 0x00, 0x07, // LDA 7   (ACC = a)
            0x02, 0x00, 0x00, // STA r0  (evict a → r0)
            0x03, 0x00, 0x00, // LD r0   (reload a into ACC for mov source)
            0x02, 0x00, 0x01, // STA r1  (XCH ACC↔r1 → r1 = a; ACC = junk)
            0x03, 0x00, 0x01, // LD r1   (reload b into ACC for ret)
            0x00, 0x00, 0x00, // HLT
        ]
    );
}

/// Two-step mov where the second src already lives in a register:
/// `const a; mov b,a; mov c,b; ret c`.  The second mov skips the
/// eviction step (b is already in r1, not ACC).
///
/// LDA a + STA r0 + LD r0 + STA r1   (mov b,a — first mov needs eviction)
/// + LD r1 + STA r2                  (mov c,b — b not in ACC, no eviction)
/// + LD r2 + HLT                     (ret c)
/// = 8 words = 24 bytes.
#[test]
fn mov_when_src_already_in_register_skips_eviction() {
    let f = IIRFunction::new(
        "chained_mov",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(9)], "i16"),
            IIRInstr::new("mov", Some("b".into()), vec![Operand::Var("a".into())], "i16"),
            IIRInstr::new("mov", Some("c".into()), vec![Operand::Var("b".into())], "i16"),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i16"),
        ],
    );
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x09, // LDA 9   (a → ACC)
            0x02, 0x00, 0x00, // STA r0  (evict a → r0)
            0x03, 0x00, 0x00, // LD r0   (a → ACC for mov b,a)
            0x02, 0x00, 0x01, // STA r1  (b = ACC → r1)
            0x03, 0x00, 0x01, // LD r1   (b → ACC for mov c,b — no eviction needed)
            0x02, 0x00, 0x02, // STA r2  (c = ACC → r2)
            0x03, 0x00, 0x02, // LD r2   (c → ACC for ret)
            0x00, 0x00, 0x00, // HLT
        ]
    );
    assert_eq!(bytes.len(), 24);
}

/// Filling all 17 slots (ACC + r0..r15) is fine; the 18th const
/// exhausts the pool because eviction has nowhere left to spill.
#[test]
fn allocator_exhausts_on_eighteenth_const() {
    // 18 consts named v0..v17 + ret_void.
    let mut instrs = Vec::with_capacity(19);
    for i in 0..18 {
        instrs.push(IIRInstr::new(
            "const",
            Some(format!("v{i}")),
            vec![Operand::Int(i as i64)],
            "i16",
        ));
    }
    instrs.push(IIRInstr::new("ret_void", None, vec![], "void"));
    let f = IIRFunction::new("too_many", vec![], "i16", instrs);
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("18 consts must overflow the 17-slot pool");
    match err {
        IIRGe225Error::OutOfRegisters { name, .. } => {
            // The eviction happens *before* the 18th const's LDA;
            // it fails trying to spill the 17th const's value
            // (which is v16, the current ACC owner).
            assert_eq!(name, "v16",
                "expected eviction of v16 (the 17th const, current ACC owner) \
                 to fail; got name {name:?}");
        }
        other => panic!("expected OutOfRegisters, got {other:?}"),
    }
}

/// 17 consts + a ret_void is right at the pool limit and must
/// succeed.  Output: 16 (LDA+STA) pairs + final LDA (no eviction
/// before the 17th since ACC is still ownable) + HLT.
#[test]
fn allocator_at_seventeenth_const_still_succeeds() {
    let mut instrs = Vec::with_capacity(18);
    for i in 0..17 {
        instrs.push(IIRInstr::new(
            "const",
            Some(format!("v{i}")),
            vec![Operand::Int(i as i64)],
            "i16",
        ));
    }
    instrs.push(IIRInstr::new("ret_void", None, vec![], "void"));
    let f = IIRFunction::new("right_at_limit", vec![], "i16", instrs);
    let bytes = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect("17 consts must fit in the 17-slot pool");
    // Word count: 17 consts → 16 evictions (LDA+STA) + 1 final LDA + 1 HLT.
    //           = 16 LDA + 16 STA + 1 LDA + 1 HLT
    //           = 17 LDA + 16 STA + 1 HLT
    //           = 34 words = 102 bytes
    assert_eq!(bytes.len(), 34 * 3);
}

/// `mov` of a never-bound source errors crisply with
/// `UndefinedVariable`.
#[test]
fn mov_from_undefined_src_errors() {
    let f = IIRFunction::new(
        "mov_undef",
        vec![],
        "i16",
        vec![
            IIRInstr::new("mov", Some("b".into()), vec![Operand::Var("a".into())], "i16"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("mov from unbound src must error");
    match err {
        IIRGe225Error::UndefinedVariable { name, .. } => assert_eq!(name, "a"),
        other => panic!("expected UndefinedVariable, got {other:?}"),
    }
}

/// Op not in v0.3.0's supported set (e.g., `add`) errors with
/// `UnsupportedOp` carrying the op name.
#[test]
fn unsupported_op_add_errors() {
    let f = IIRFunction::new(
        "with_add",
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
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let err = lower_iir_to_ge225(&module_with(f), &IIRGe225Config::default())
        .expect_err("add is not yet supported");
    match err {
        IIRGe225Error::UnsupportedOp { op, .. } => assert_eq!(op, "add"),
        other => panic!("expected UnsupportedOp, got {other:?}"),
    }
}

/// const Int(-1) still rewrites via two's complement under the new
/// allocator (regression from v0.2.0 — single const, no eviction).
#[test]
fn const_negative_one_still_uses_twos_complement() {
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

/// 16-bit immediate overflow still rejected (regression from v0.2.0).
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
