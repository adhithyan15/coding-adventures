//! Integration tests for `iir-to-intel4004` v0.1.0 (A4 skeleton).
//!
//! Note: this crate is deprecated as of v0.4.0 (Phase 4 of the
//! historical-arch backend migration).  Tests still exercise the
//! deprecated API as a regression invariant — `#![allow(deprecated)]`
//! suppresses the build warnings.
#![allow(deprecated)]
//!
//! Smoke-level — confirms the validator stub, the emitter shape,
//! and the exact encoded `JUN 0x000` byte pair (`0x40 0x00`).

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_intel4004::{
    lower_iir_to_intel4004, validate_for_intel4004,
    IIRIntel4004Config, IIRIntel4004Error,
    HALT_LOOP, LDM_OPCODE, LD_OPCODE, XCH_OPCODE,
};

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
    assert!(validate_for_intel4004(&empty_module()).is_empty());
}

// ===========================================================================
// 2. Lowering shape and the exact `JUN 0x000` encoding
// ===========================================================================

#[test]
fn lower_emits_exactly_two_bytes() {
    let bytes = lower_iir_to_intel4004(&empty_module(), &IIRIntel4004Config::default())
        .expect("lowering");
    assert_eq!(bytes.len(), 2,
        "v0.1.0 must emit exactly two bytes (the JUN 0x000 sentinel); got: {bytes:02x?}");
}

/// The emitted bytes are `0x40 0x00` — the canonical 4004-ROM
/// `JUN 0x000` halt idiom.
///
/// Bit layout (JUN = `0100 aaaa aaaaaaaa`):
///
/// ```text
/// byte 1: 0100 0000 = 0x40   (JUN opcode + high nibble of 12-bit addr = 0)
/// byte 2: 0000 0000 = 0x00   (low byte of address = 0)
/// ```
///
/// Pinning the exact constant guards against any future change in
/// 4004 simulator decoders that would break the encoding.
#[test]
fn lower_emits_the_canonical_jun_self_bytes() {
    let bytes = lower_iir_to_intel4004(&empty_module(), &IIRIntel4004Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0x40, 0x00],
        "expected canonical JUN 0x000 encoding [0x40, 0x00]; got: {bytes:02x?}");
    assert_eq!(bytes.as_slice(), &HALT_LOOP,
        "the emitted bytes should equal the exported HALT_LOOP constant");
}

#[test]
fn halt_loop_constant_pinned_to_40_00() {
    // Sanity: the HALT_LOOP constant matches the canonical 4004
    // JUN-0x000 documented encoding.
    assert_eq!(HALT_LOOP, [0x40, 0x00],
        "JUN 0x000 should be [0x40, 0x00] (opcode 0100, addr 0x000)");
}

// ===========================================================================
// 3. Config defaults
// ===========================================================================

#[test]
fn default_config_has_nonempty_module_name() {
    let cfg = IIRIntel4004Config::default();
    assert!(!cfg.module_name.is_empty());
}

#[test]
fn new_sets_module_name() {
    let cfg = IIRIntel4004Config::new("custom");
    assert_eq!(cfg.module_name, "custom");
}

// ===========================================================================
// 4. Error display
// ===========================================================================

#[test]
fn errors_display_without_panic() {
    let _ = format!("{}", IIRIntel4004Error::ValidationFailed(vec!["x".into()]));
    let _ = format!("{}", IIRIntel4004Error::UnsupportedOp {
        function: "f".into(), op: "weird".into(),
    });
    let _ = format!("{}", IIRIntel4004Error::UnsupportedType {
        function: "f".into(), type_hint: "weird".into(),
    });
    let _ = format!("{}", IIRIntel4004Error::InvalidOperand {
        function: "f".into(), detail: "bad".into(),
    });
}

// ===========================================================================
// 5. A4+ (v0.2.0) — `const` lowers to `LDM n`; `ret` → JUN-self (halt)
// ===========================================================================
//
// The accumulator-only first slice: every `const` goes into the
// accumulator via `LDM`.  `ret`/`ret_void` both emit the 2-byte
// JUN 0x000 halt sentinel because real RET (via BBL + the 4004's
// 3-deep internal stack) needs A4++'s call/return discipline.

#[test]
fn ldm_opcode_pinned_to_0xd0() {
    // Sanity: the LDM_OPCODE high nibble matches the canonical
    // 4004-documented encoding (1101_xxxx).
    assert_eq!(LDM_OPCODE, 0xD0,
        "LDM opcode high nibble should be 0xD0 (1101_0000)");
}

#[test]
fn const_5_then_ret_lowers_to_ldm_5_then_jun_self() {
    // const v=5; ret v
    //   LDM 5     = 0xD5
    //   JUN 0x000 = 0x40 0x00
    let f = IIRFunction::new("five", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(5)], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0xD5, 0x40, 0x00],
        "expected LDM 5 (0xD5) + JUN 0x000 (0x40 0x00); got: {bytes:02x?}");
}

#[test]
fn const_15_then_ret_lowers_to_ldm_15_then_jun_self() {
    // const v=15 (the max 4-bit value); ret v
    //   LDM 15    = 0xDF
    //   JUN 0x000 = 0x40 0x00
    let f = IIRFunction::new("max", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(15)], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0xDF, 0x40, 0x00],
        "expected LDM 15 (0xDF) + JUN 0x000; got: {bytes:02x?}");
}

#[test]
fn const_0_then_ret_emits_ldm_0() {
    // const v=0; ret v
    //   LDM 0 = 0xD0 — pinned because OR-ing with 0 should leave
    //   the LDM opcode high nibble visibly intact.
    let f = IIRFunction::new("zero", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0)], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0xD0, 0x40, 0x00],
        "expected LDM 0 (0xD0) + JUN 0x000; got: {bytes:02x?}");
}

#[test]
fn const_negative_uses_twos_complement_nibble() {
    // -1 → 0xF via 4-bit two's-complement reinterpretation.
    // LDM 0xF = 0xDF.
    let f = IIRFunction::new("minus_one", vec![], "i8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(-1)], "i8"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let bytes = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0xDF, 0x40, 0x00],
        "expected LDM 0xF + JUN 0x000 for `const -1`; got: {bytes:02x?}");
}

#[test]
fn const_negative_minus_eight_uses_8_nibble() {
    // -8 → 0x8 (the minimum signed 4-bit value).  LDM 0x8 = 0xD8.
    let f = IIRFunction::new("minus_eight", vec![], "i8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(-8)], "i8"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let bytes = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0xD8, 0x40, 0x00],
        "expected LDM 0x8 + JUN 0x000 for `const -8`; got: {bytes:02x?}");
}

#[test]
fn const_out_of_nibble_range_is_rejected() {
    let f = IIRFunction::new("oversized", vec![], "i16", vec![
        // 16 is just past the 4-bit range.
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(16)], "i16"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let err = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect_err("16 should overflow the 4-bit immediate");
    match err {
        IIRIntel4004Error::InvalidOperand { detail, .. } => {
            assert!(detail.contains("4-bit"),
                "expected message naming the 4-bit limit; got: {detail}");
        }
        other => panic!("expected InvalidOperand, got: {other:?}"),
    }
}

#[test]
fn ret_void_alone_emits_just_jun_self() {
    let f = IIRFunction::new("main", vec![], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let bytes = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0x40, 0x00],
        "ret_void-only function should emit just JUN 0x000; got: {bytes:02x?}");
}

#[test]
fn unsupported_op_is_rejected_with_function_name() {
    let f = IIRFunction::new("boom", vec![], "void", vec![
        IIRInstr::new("safepoint", None, vec![], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let err = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect_err("`safepoint` should be UnsupportedOp");
    match err {
        IIRIntel4004Error::UnsupportedOp { function, op } => {
            assert_eq!(function, "boom");
            assert_eq!(op, "safepoint");
        }
        other => panic!("expected UnsupportedOp, got: {other:?}"),
    }
}

// ===========================================================================
// 6. A4++ (v0.3.0) — ACC-first allocator + `mov` + ret-value staging
// ===========================================================================
//
// The 4004's ALU is accumulator-anchored.  The ACC-first allocator
// keeps the first var in the accumulator, only spilling to r0..r15
// on contention.  This preserves v0.2.0's 3-byte shape for the
// trivial `const v; ret v` case.

#[test]
fn ld_opcode_pinned_to_0xa0() {
    assert_eq!(LD_OPCODE, 0xA0,
        "LD r opcode high nibble should be 0xA0 (1010_0000)");
}

#[test]
fn xch_opcode_pinned_to_0xb0() {
    assert_eq!(XCH_OPCODE, 0xB0,
        "XCH r opcode high nibble should be 0xB0 (1011_0000)");
}

#[test]
fn ret_of_first_const_omits_xch_and_ld() {
    // Regression for v0.2.0's 3-byte shape: when v is the sole var,
    // it stays in ACC and ret v needs no staging LD.
    let f = IIRFunction::new("trivial", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![0xD7, 0x40, 0x00],
        "ACC-first allocator should keep the trivial case at 3 bytes; got: {bytes:02x?}");
}

#[test]
fn two_consts_use_acc_then_xch_to_r0_for_eviction() {
    // const v=5; const w=7; ret w
    //   LDM 5    = 0xD5     (v → ACC)
    //   XCH r0   = 0xB0     (evict v to r0; ACC ← junk)
    //   LDM 7    = 0xD7     (w → ACC)
    //   JUN 0x000           (w is in ACC, no LD before JUN)
    let f = IIRFunction::new("two", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(5)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(7)], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("w".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        0xD5,           // LDM 5 (v → ACC)
        0xB0,           // XCH r0 (evict v to r0)
        0xD7,           // LDM 7 (w → ACC)
        0x40, 0x00,     // JUN 0x000 (w already in ACC)
    ], "expected LDM 5 + XCH r0 + LDM 7 + JUN; got: {bytes:02x?}");
}

#[test]
fn ret_of_evicted_var_emits_ld_before_jun() {
    // const v=5; const w=7; ret v
    // v gets evicted to r0 when w arrives.  Then ret v needs LD r0.
    let f = IIRFunction::new("ret_v", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(5)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(7)], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        0xD5,           // LDM 5 (v → ACC)
        0xB0,           // XCH r0 (evict v to r0)
        0xD7,           // LDM 7 (w → ACC)
        0xA0,           // LD r0 (stage v back into ACC for ret)
        0x40, 0x00,     // JUN 0x000
    ], "expected LDM 5 + XCH r0 + LDM 7 + LD r0 + JUN; got: {bytes:02x?}");
}

#[test]
fn mov_lowers_to_ld_then_xch() {
    // const v=3; mov w=v; ret w
    //
    // v → ACC.  mov w=v: v is in ACC, evict v to r0 first (LDM
    // already happened, just need XCH); then LD r0 stages src
    // value into ACC; XCH r1 puts it into r1 (= w).  After: w in
    // r1, ACC has junk.
    //
    // For ret w: LD r1.
    //
    //   LDM 3    = 0xD3   (v → ACC)
    //   XCH r0   = 0xB0   (evict v from ACC to r0)
    //   LD  r0   = 0xA0   (ACC ← v's value)
    //   XCH r1   = 0xB1   (r1 = v's value = w; ACC = junk)
    //   LD  r1   = 0xA1   (ACC ← w's value for ret)
    //   JUN 0x000
    let f = IIRFunction::new("mov_fn", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(3)], "u8"),
        IIRInstr::new("mov",   Some("w".into()), vec![Operand::Var("v".into())], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("w".into())], "u8"),
    ]);
    let bytes = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect("lowering");
    assert_eq!(bytes, vec![
        0xD3,           // LDM 3
        0xB0,           // XCH r0 (evict v from ACC)
        0xA0,           // LD r0  (stage v's value back to ACC for mov)
        0xB1,           // XCH r1 (w = v's value)
        0xA1,           // LD r1  (stage w into ACC for ret)
        0x40, 0x00,     // JUN 0x000
    ], "mov expected; got: {bytes:02x?}");
}

#[test]
fn allocator_exhaustion_yields_out_of_registers() {
    // Capacity: 1 var in ACC + 16 vars in r0..r15 = 17 total.  The
    // 18th const tries to evict v16 (currently in ACC) to a real
    // register, but next_reg is already at 16 — OutOfRegisters fires
    // on v16's eviction.
    let mut body = Vec::new();
    for i in 0..18 {
        body.push(IIRInstr::new(
            "const",
            Some(format!("v{i}")),
            vec![Operand::Int((i % 16) as i64)],
            "u8",
        ));
    }
    body.push(IIRInstr::new("ret_void", None, vec![], "void"));
    let f = IIRFunction::new("greedy", vec![], "void", body);
    let err = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect_err("18 consts should exhaust the 17-slot pool (ACC + 16 GP)");
    match err {
        IIRIntel4004Error::OutOfRegisters { function, name } => {
            assert_eq!(function, "greedy");
            // v16 currently lives in ACC; the 18th const (v17) tries
            // to evict it and runs out of GP registers.
            assert_eq!(name, "v16");
        }
        other => panic!("expected OutOfRegisters, got: {other:?}"),
    }
}

#[test]
fn undefined_variable_in_mov_is_rejected() {
    let f = IIRFunction::new("f", vec![], "void", vec![
        IIRInstr::new("mov", Some("w".into()),
            vec![Operand::Var("ghost".into())], "u8"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let err = lower_iir_to_intel4004(&module_with(f), &IIRIntel4004Config::default())
        .expect_err("mov from undefined var should fail");
    match err {
        IIRIntel4004Error::UndefinedVariable { name, .. } => {
            assert_eq!(name, "ghost");
        }
        other => panic!("expected UndefinedVariable, got: {other:?}"),
    }
}

#[test]
fn errors_for_new_variants_display_without_panic() {
    let _ = format!("{}", IIRIntel4004Error::UndefinedVariable {
        function: "f".into(), name: "ghost".into(),
    });
    let _ = format!("{}", IIRIntel4004Error::OutOfRegisters {
        function: "greedy".into(), name: "v15".into(),
    });
}
