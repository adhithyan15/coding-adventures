// Tests for iir-to-ge225 v0.6.0 (A5++++++ — call/return JSR/RTS + BMI).
//
// Mirrors iir-to-intel8008 v0.3.9 (module-level call backpatching)
// in spirit, adapted for GE-225's 20-bit-word / 3-bytes-per-word
// packing.
//
// Coverage:
//   §1 — validator stub
//   §2 — opcode constant pinning (incl. new JSR/RTS/BMI + RTS_WORD)
//   §3 — Config defaults
//   §4 — Error Display (10 variants now)
//   §5 — v0.2.0 / v0.3.0 / v0.4.0 / v0.5.0 regressions
//   §6 — v0.6.0 NEW: call / RTS / entry-vs-non-entry HLT discrimination

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

/// Build a module from multiple functions; first function's name is
/// the entry point unless explicitly overridden.
fn module_with_many(fs: Vec<IIRFunction>, entry: Option<&str>) -> IIRModule {
    let entry = entry.map(|s| s.to_string()).or_else(|| fs.first().map(|f| f.name.clone()));
    IIRModule {
        name: "test".into(),
        functions: fs,
        entry_point: entry,
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
fn rts_word_constant_pinned() {
    assert_eq!(RTS_WORD, [0x0A, 0x00, 0x00]);
}

#[test]
fn opcode_nibbles_pinned_through_v0_6_0() {
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
    assert_eq!(BMI_OPCODE_NIBBLE, 0xB);
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
        IIRGe225Error::UndefinedFunction {
            caller: "main".into(),
            callee: "missing".into(),
        },
        IIRGe225Error::CallTargetOutOfRange {
            caller: "main".into(),
            callee: "tail".into(),
            offset: 100_000,
        },
    ];
    for err in errs {
        assert!(!format!("{err}").is_empty());
    }
}

// ===========================================================================
// §5. Regressions from v0.2.0 / v0.3.0 / v0.4.0 / v0.5.0
// ===========================================================================

#[test]
fn empty_module_still_emits_halt() {
    let bytes = lower_iir_to_ge225(&empty_module(), &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0x00, 0x00, 0x00]);
}

#[test]
fn trivial_rom_in_entry_function_still_six_bytes() {
    // Single-function module with entry=Some("trivial") — ret in
    // the entry function should still emit HLT, preserving the
    // canonical 6-byte trivial-case ROM from v0.2.0.
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
        assert_eq!(bytes.len(), 6, "entry-function trivial ROM stays 6 bytes for n={n}");
        // Last 3 bytes must be HLT (not RTS).
        assert_eq!(&bytes[3..], &HALT_WORD,
            "entry function ret must emit HLT, not RTS, for n={n}");
    }
}

#[test]
fn trivial_add_still_works() {
    // const a=3; const b=4; add c, a, b; ret c — 21 bytes unchanged.
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
    assert_eq!(bytes.len(), 21);
}

#[test]
fn trivial_branch_still_works() {
    // jmp x; label x; ret_void — same 6-byte sequence as v0.5.0.
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
            0x06, 0x00, 0x03, // BR 3
            0x00, 0x00, 0x00, // HLT (entry function)
        ]
    );
}

// ===========================================================================
// §6. v0.6.0 NEW — call / RTS / entry-vs-non-entry HLT discrimination
// ===========================================================================

/// In a NON-entry function, `ret_void` must emit `RTS`, not `HLT`.
///
/// Module: { fn main { ret_void }, fn helper { ret_void } } with
/// entry=main.
///
/// Bytes:
///   0: main's ret_void → HLT [0x00, 0x00, 0x00]
///   3: helper's ret_void → RTS [0x0A, 0x00, 0x00]
#[test]
fn non_entry_ret_void_emits_rts_not_halt() {
    let main = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    );
    let helper = IIRFunction::new(
        "helper",
        vec![],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    );
    let module = module_with_many(vec![main, helper], Some("main"));
    let bytes = lower_iir_to_ge225(&module, &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x00, 0x00, 0x00, // main: HLT (entry)
            0x0A, 0x00, 0x00, // helper: RTS (non-entry)
        ]
    );
}

/// In a non-entry function, `ret <var>` must stage var into ACC and
/// emit `RTS`, not `HLT`.
///
/// Module: { fn main { ret_void }, fn helper { const v=7; ret v } }
/// with entry=main.
///
/// helper's `ret v` (v is ACC owner) → just RTS (no LD needed).
#[test]
fn non_entry_ret_with_var_emits_rts() {
    let main = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    );
    let helper = IIRFunction::new(
        "helper",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "i16"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i16"),
        ],
    );
    let module = module_with_many(vec![main, helper], Some("main"));
    let bytes = lower_iir_to_ge225(&module, &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x00, 0x00, 0x00, // main: HLT
            0x01, 0x00, 0x07, // helper: LDA 7
            0x0A, 0x00, 0x00, // helper: RTS
        ]
    );
}

/// `call helper` from main with backpatched address.
///
/// Module: { fn main { call helper; ret_void }, fn helper { ret_void } }
/// with entry=main.
///
/// Bytes:
///   0: main: JSR <helper>     [0x09, hi=0x00, lo=0x06]
///   3: main: HLT               [0x00, 0x00, 0x00]
///   6: helper: RTS             [0x0A, 0x00, 0x00]
///
/// helper's entry is at byte 6, so JSR slot bytes 1..3 get 0x00 0x06.
#[test]
fn trivial_call_no_return_emits_jsr_then_helper_rts() {
    let main = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![
            IIRInstr::new("call", None, vec![Operand::Var("helper".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let helper = IIRFunction::new(
        "helper",
        vec![],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    );
    let module = module_with_many(vec![main, helper], Some("main"));
    let bytes = lower_iir_to_ge225(&module, &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x09, 0x00, 0x06, // main: JSR 6  (helper's entry)
            0x00, 0x00, 0x00, // main: HLT
            0x0A, 0x00, 0x00, // helper: RTS
        ]
    );
}

/// `call dest = helper` captures the callee's return value (in ACC)
/// into dest.  Then `ret dest` finds dest as the current ACC owner
/// and emits just HLT (no LD needed).
///
/// Module: { fn main { call x = helper; ret x }, fn helper {
/// const v=42; ret v } } with entry=main.
///
/// main bytes:
///   0: JSR <helper>            [0x09, hi=0x00, lo=0x06]
///   3: HLT (entry, ACC has x)  [0x00, 0x00, 0x00]
/// helper bytes:
///   6: LDA 42                   [0x01, 0x00, 0x2A]
///   9: RTS                      [0x0A, 0x00, 0x00]
#[test]
fn call_with_return_captures_value() {
    let main = IIRFunction::new(
        "main",
        vec![],
        "i16",
        vec![
            IIRInstr::new("call", Some("x".into()), vec![Operand::Var("helper".into())], "i16"),
            IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i16"),
        ],
    );
    let helper = IIRFunction::new(
        "helper",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "i16"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i16"),
        ],
    );
    let module = module_with_many(vec![main, helper], Some("main"));
    let bytes = lower_iir_to_ge225(&module, &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x09, 0x00, 0x06, // main: JSR 6 (helper)
            0x00, 0x00, 0x00, // main: HLT (x is ACC owner from JSR's return)
            0x01, 0x00, 0x2A, // helper: LDA 42
            0x0A, 0x00, 0x00, // helper: RTS
        ]
    );
}

/// `call` to a function that doesn't exist anywhere in the module
/// errors with `UndefinedFunction`.
#[test]
fn call_to_undefined_function_errors() {
    let main = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![
            IIRInstr::new("call", None, vec![Operand::Var("does_not_exist".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let module = module_with_many(vec![main], Some("main"));
    let err = lower_iir_to_ge225(&module, &IIRGe225Config::default())
        .expect_err("call to undefined function must error");
    match err {
        IIRGe225Error::UndefinedFunction { caller, callee } => {
            assert_eq!(caller, "main");
            assert_eq!(callee, "does_not_exist");
        }
        other => panic!("expected UndefinedFunction, got {other:?}"),
    }
}

/// Multiple calls in sequence all backpatch to their correct
/// addresses.  Module ordering: main, a, b.  main calls a then b.
#[test]
fn multiple_calls_resolve_independently() {
    let main = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![
            IIRInstr::new("call", None, vec![Operand::Var("a".into())], "void"),
            IIRInstr::new("call", None, vec![Operand::Var("b".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let a = IIRFunction::new(
        "a",
        vec![],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    );
    let b = IIRFunction::new(
        "b",
        vec![],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    );
    let module = module_with_many(vec![main, a, b], Some("main"));
    let bytes = lower_iir_to_ge225(&module, &IIRGe225Config::default())
        .expect("lowering");
    // Layout:
    //   0:  main: JSR a (placeholder)
    //   3:  main: JSR b (placeholder)
    //   6:  main: HLT
    //   9:  a: RTS
    //   12: b: RTS
    // Backpatched: a at 9, b at 12.
    assert_eq!(
        bytes,
        vec![
            0x09, 0x00, 0x09, // main: JSR 9 (a)
            0x09, 0x00, 0x0C, // main: JSR 12 (b)
            0x00, 0x00, 0x00, // main: HLT
            0x0A, 0x00, 0x00, // a: RTS
            0x0A, 0x00, 0x00, // b: RTS
        ]
    );
}

/// Forward `call` — callee defined LATER in the module than the
/// caller — works because backpatching happens after all functions
/// are emitted.
#[test]
fn forward_call_resolves_via_backpatching() {
    let main = IIRFunction::new(
        "main",
        vec![],
        "void",
        vec![
            IIRInstr::new("call", None, vec![Operand::Var("later".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let later = IIRFunction::new(
        "later",
        vec![],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    );
    let module = module_with_many(vec![main, later], Some("main"));
    let bytes = lower_iir_to_ge225(&module, &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(bytes[0], 0x09, "JSR opcode at byte 0");
    assert_eq!(bytes[1], 0x00, "JSR hi byte");
    assert_eq!(bytes[2], 0x06, "JSR lo byte → later at offset 6");
    assert_eq!(&bytes[3..6], &HALT_WORD);
    assert_eq!(&bytes[6..9], &RTS_WORD);
}

/// `call` evicts a live ACC owner first — the callee will clobber
/// ACC, so any prior value must be saved.
///
/// Module: { fn main { const x=5; call helper; ret x }, fn helper { ret_void } }
///
/// main bytes:
///   0: LDA 5                  (x → ACC, owner=x)
///   3: STA r0                 (evict x → r0 before JSR)
///   6: JSR <helper>           (placeholder, backpatched to helper)
///   9: LD r0                  (reload x for ret)
///   12: HLT                    (entry → HLT)
/// helper:
///   15: RTS
///
/// Backpatched: JSR at byte 7-8 holds (hi, lo) of helper offset 15.
#[test]
fn call_evicts_live_acc_owner_before_jsr() {
    let main = IIRFunction::new(
        "main",
        vec![],
        "i16",
        vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(5)], "i16"),
            IIRInstr::new("call", None, vec![Operand::Var("helper".into())], "void"),
            IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i16"),
        ],
    );
    let helper = IIRFunction::new(
        "helper",
        vec![],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    );
    let module = module_with_many(vec![main, helper], Some("main"));
    let bytes = lower_iir_to_ge225(&module, &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x05, // LDA 5
            0x02, 0x00, 0x00, // STA r0 (evict x before JSR)
            0x09, 0x00, 0x0F, // JSR 15 (helper)
            0x03, 0x00, 0x00, // LD r0 (reload x for ret)
            0x00, 0x00, 0x00, // HLT (entry function)
            0x0A, 0x00, 0x00, // helper: RTS
        ]
    );
}

/// When entry_point is `None`, ALL functions emit RTS for ret (no
/// HLT).  Conservative: the IR author opted out of an entry, so
/// no function gets the program-halt treatment.
#[test]
fn no_entry_point_means_all_functions_use_rts() {
    let only_fn = IIRFunction::new(
        "lone",
        vec![],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    );
    let module = IIRModule {
        name: "test".into(),
        functions: vec![only_fn],
        entry_point: None,
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let bytes = lower_iir_to_ge225(&module, &IIRGe225Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0x0A, 0x00, 0x00]);
}

/// `mul` (still unsupported in v0.6.0) errors with `UnsupportedOp`.
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
