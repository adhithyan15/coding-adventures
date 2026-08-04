//! Integration tests for `iir-to-armv7` v0.1.0 (A3 skeleton).
//!
//! Note: this crate is deprecated as of v0.5.0 (Phase 5 of the
//! historical-arch backend migration).  Tests still exercise the
//! deprecated API as a regression invariant.
#![allow(deprecated)]
//!
//! Smoke-level — confirms the validator stub, the emitter shape, and
//! the exact encoded `BKPT #0xFFFF` word (`0xE12FFF7F`).

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_armv7::{
    lower_iir_to_armv7, validate_for_armv7,
    IIRArmv7Config, IIRArmv7Error,
    ADC_REG_BASE, ADD_REG_BASE, AND_REG_BASE, B_BASE, B_EQ_BASE, B_NE_BASE,
    BKPT, BL_BASE, BX_LR, CMP_IMM_ZERO_BASE, CMP_REG_BASE, EOR_REG_BASE,
    MOV_IMM_CC_BASE, MOV_IMM_CS_BASE, MOV_IMM_EQ_BASE, MOV_IMM_HI_BASE,
    MOV_IMM_LS_BASE, MOV_IMM_NE_BASE, MOV_IMM_R0_BASE, MOV_REG_BASE,
    ORR_REG_BASE, SBC_REG_BASE, SUB_REG_BASE,
};

fn multi_fn_module(entry: &str, functions: Vec<IIRFunction>) -> IIRModule {
    IIRModule {
        name: "test".into(),
        functions,
        entry_point: Some(entry.into()),
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

// ===========================================================================
// 5. A3+ (v0.2.0) — `const` lowers to `MOV r0, #imm8`; `ret` → `BX LR`
// ===========================================================================
//
// The accumulator-only first slice: every `const` goes into `r0`;
// multi-register allocation (r1..r12) lands in A3++.  `ret`/`ret_void`
// both lower to `bx lr` — the AAPCS return convention.  No staging
// MOV needed since the value is already in r0 by construction.

#[test]
fn bx_lr_constant_pinned_to_e12fff1e() {
    // BX LR = 0xE12FFF1E.  Bit-7 distinguishes this from BKPT (which
    // is 0xE12FFF7F) — both share the same 12F_FF family bits, so
    // the bit-7 nibble difference is the canonical confusion point.
    assert_eq!(BX_LR, 0xE12F_FF1E,
        "BX LR should be 0xE12FFF1E (cond=AL, Rm=lr=14); got 0x{:08x}", BX_LR);
}

#[test]
fn mov_imm_r0_base_pinned_to_0xe3a00000() {
    // MOV r0, #0 (no rotation, S=0) = 0xE3A00000.  Subsequent tests
    // OR in (Rd << 12) | imm8 to form the full instruction.
    assert_eq!(MOV_IMM_R0_BASE, 0xE3A0_0000,
        "MOV r0, #0 base should be 0xE3A00000; got 0x{:08x}", MOV_IMM_R0_BASE);
}

#[test]
fn const_42_then_ret_lowers_to_mov_r0_42_then_bx_lr() {
    // const v=42 → r0; ret v → bx lr (v is in r0)
    //
    //   00:  MOV r0, #42   = 0xE3A0_002A
    //   04:  BX LR          = 0xE12F_FF1E
    let f = IIRFunction::new("answer", vec![], "i32", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "i32"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "i32"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![0xE3A0_002A, 0xE12F_FF1E],
        "expected MOV r0, #42 (0xE3A0_002A) + BX LR (0xE12F_FF1E); got: {words:08x?}");
}

#[test]
fn const_negative_uses_twos_complement_byte() {
    // -1 → 0xFF via two's-complement reinterpretation, then MOV r0, #0xFF.
    let f = IIRFunction::new("f", vec![], "i8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(-1)], "i8"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![0xE3A0_00FF, BX_LR],
        "expected MOV r0, #0xFF + BX LR for `const -1`; got: {words:08x?}");
}

#[test]
fn const_out_of_byte_range_is_rejected() {
    let f = IIRFunction::new("f", vec![], "i16", vec![
        // 1000 fits in i16 but exceeds the 8-bit MOV immediate.
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(1000)], "i16"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let err = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect_err("1000 should overflow the 8-bit immediate");
    match err {
        IIRArmv7Error::InvalidOperand { detail, .. } => {
            assert!(detail.contains("8-bit"),
                "expected message naming the 8-bit limit; got: {detail}");
        }
        other => panic!("expected InvalidOperand, got: {other:?}"),
    }
}

#[test]
fn ret_void_alone_emits_just_bx_lr() {
    let f = IIRFunction::new("main", vec![], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![BX_LR],
        "ret_void-only function should emit just BX LR; got: {words:08x?}");
}

#[test]
fn unsupported_op_is_rejected_with_function_name() {
    let f = IIRFunction::new("boom", vec![], "void", vec![
        IIRInstr::new("safepoint", None, vec![], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let err = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect_err("`safepoint` should be UnsupportedOp");
    match err {
        IIRArmv7Error::UnsupportedOp { function, op } => {
            assert_eq!(function, "boom");
            assert_eq!(op, "safepoint");
        }
        other => panic!("expected UnsupportedOp, got: {other:?}"),
    }
}

// ===========================================================================
// 6. A3++ (v0.3.0) — multi-register allocator + `mov` + ret-value staging
// ===========================================================================
//
// `const` rounds out a register from REGISTER_POOL.  r0 is handed out
// first so the trivial `const v; ret v` case keeps its 2-word shape
// (no redundant MOV r0, X round-trip).  Subsequent consts spill into
// r1, r2, ..., r12 in order.

#[test]
fn mov_reg_base_pinned_to_0xe1a00000() {
    // MOV r0, r0 (no shift, S=0) = 0xE1A00000.  Subsequent tests OR
    // in (Rd << 12) | Rm to form the full instruction.
    assert_eq!(MOV_REG_BASE, 0xE1A0_0000,
        "MOV Rd, Rm base should be 0xE1A00000; got 0x{:08x}", MOV_REG_BASE);
}

#[test]
fn two_consts_use_r0_then_r1_then_mov_r0_r1_before_bx_lr() {
    // const v=1; const w=2; ret w
    //   MOV r0, #1   = 0xE3A0_0001
    //   MOV r1, #2   = 0xE3A0_1002   (Rd=1)
    //   MOV r0, r1   = 0xE1A0_0001   ← stage w into r0
    //   BX LR        = 0xE12F_FF1E
    let f = IIRFunction::new("f", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(1)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(2)], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("w".into())], "u8"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![
        0xE3A0_0001,    // MOV r0, #1   (v)
        0xE3A0_1002,    // MOV r1, #2   (w)  — Rd=1 → bits 15..12 = 0001
        0xE1A0_0001,    // MOV r0, r1   (stage w into r0)
        BX_LR,
    ], "expected 4-word sequence; got: {words:08x?}");
}

#[test]
fn ret_of_first_const_omits_the_redundant_mov() {
    // Regression for the v0.2.0 pinned 2-word shape: when the value
    // being returned is already in r0, no `MOV r0, X` is emitted.
    let f = IIRFunction::new("f", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "u8"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![0xE3A0_002A, BX_LR],
        "r0-first allocator should keep the trivial case at 2 words; got: {words:08x?}");
}

#[test]
fn mov_lowers_to_canonical_mov_rd_rm() {
    // const v=7; mov w=v; ret w
    // v is in r0 (first allocated). w is in r1 (next pool slot).
    //   MOV r0, #7   = 0xE3A0_0007
    //   MOV r1, r0   = 0xE1A0_1000   (Rd=1, Rm=0)
    //   MOV r0, r1   = 0xE1A0_0001   ← stage w back into r0 for ret
    //   BX LR
    let f = IIRFunction::new("f", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "u8"),
        IIRInstr::new("mov",   Some("w".into()), vec![Operand::Var("v".into())], "u8"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("w".into())], "u8"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![
        0xE3A0_0007,    // MOV r0, #7
        0xE1A0_1000,    // MOV r1, r0   (Rd=1, Rm=0 → bits 15..12 = 0001, bits 3..0 = 0000)
        0xE1A0_0001,    // MOV r0, r1   (Rd=0, Rm=1)
        BX_LR,
    ], "expected MOV r0,#7 + MOV r1,r0 + MOV r0,r1 + BX LR; got: {words:08x?}");
}

#[test]
fn allocator_exhaustion_yields_out_of_registers() {
    // 13 consts fill the pool [r0..r12]; the 14th triggers OutOfRegisters.
    let mut body = Vec::new();
    for i in 0..14 {
        body.push(IIRInstr::new(
            "const",
            Some(format!("v{i}")),
            vec![Operand::Int((i % 256) as i64)],
            "u8",
        ));
    }
    body.push(IIRInstr::new("ret_void", None, vec![], "void"));
    let f = IIRFunction::new("greedy", vec![], "void", body);
    let err = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect_err("14 consts should exhaust the 13-register pool");
    match err {
        IIRArmv7Error::OutOfRegisters { function, name } => {
            assert_eq!(function, "greedy");
            assert_eq!(name, "v13", "should fail on the 14th local");
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
    let err = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect_err("mov from undefined var should fail");
    match err {
        IIRArmv7Error::UndefinedVariable { name, .. } => {
            assert_eq!(name, "ghost");
        }
        other => panic!("expected UndefinedVariable, got: {other:?}"),
    }
}

#[test]
fn errors_for_new_variants_display_without_panic() {
    let _ = format!("{}", IIRArmv7Error::UndefinedVariable {
        function: "f".into(), name: "ghost".into(),
    });
    let _ = format!("{}", IIRArmv7Error::OutOfRegisters {
        function: "greedy".into(), name: "v13".into(),
    });
}

// ===========================================================================
// 7. A3++.5 (v0.4.0) — data-processing-register ALU (ADD, SUB)
// ===========================================================================
//
// Unlike the 8008's `ADD r` (accumulator-anchored — `A = A + r`),
// ARMv7's `ADD Rd, Rn, Rm` is a 3-register operation: any pair of
// source registers can produce any destination register in a single
// instruction.  No staging MOVs.  Same shape as RV32I's `add rd, rs1,
// rs2`.

#[test]
fn add_reg_base_pinned_to_0xe0800000() {
    // ADD r0, r0, r0 (no shift, S=0) = 0xE0800000.
    assert_eq!(ADD_REG_BASE, 0xE080_0000,
        "ADD Rd, Rn, Rm base should be 0xE0800000; got 0x{:08x}", ADD_REG_BASE);
}

#[test]
fn sub_reg_base_pinned_to_0xe0400000() {
    // SUB r0, r0, r0 (no shift, S=0) = 0xE0400000.
    assert_eq!(SUB_REG_BASE, 0xE040_0000,
        "SUB Rd, Rn, Rm base should be 0xE0400000; got 0x{:08x}", SUB_REG_BASE);
}

#[test]
fn add_three_consts_emits_single_instruction_no_staging() {
    // const v=3; const w=4; add r v w; ret r
    //
    // Allocator: v→r0, w→r1, r→r2.
    //   MOV r0, #3       = 0xE3A0_0003
    //   MOV r1, #4       = 0xE3A0_1004
    //   ADD r2, r0, r1   = 0xE080_0000 | (0<<16) | (2<<12) | 1
    //                     = 0xE080_2001    ← Rn=0, Rd=2, Rm=1
    //   MOV r0, r2       = 0xE1A0_0002    ← stage r into r0 for ret
    //   BX LR            = 0xE12F_FF1E
    let f = IIRFunction::new("add3plus4", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(3)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(4)], "u8"),
        IIRInstr::new("add", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![
        0xE3A0_0003,    // MOV r0, #3
        0xE3A0_1004,    // MOV r1, #4
        0xE080_2001,    // ADD r2, r0, r1
        0xE1A0_0002,    // MOV r0, r2
        BX_LR,
    ], "add 3+4 expected; got: {words:08x?}");
}

#[test]
fn sub_three_consts_emits_sub_instruction_with_correct_opcode_field() {
    // const v=10; const w=4; sub r v w; ret r
    //   SUB r2, r0, r1 = 0xE040_0000 | (0<<16) | (2<<12) | 1 = 0xE040_2001
    let f = IIRFunction::new("ten_minus_four", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(10)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(4)], "u8"),
        IIRInstr::new("sub", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![
        0xE3A0_000A,    // MOV r0, #10
        0xE3A0_1004,    // MOV r1, #4
        0xE040_2001,    // SUB r2, r0, r1
        0xE1A0_0002,    // MOV r0, r2
        BX_LR,
    ], "sub 10-4 expected; got: {words:08x?}");
}

/// `add r v v` — same register used as both Rn and Rm.  The 8008
/// equivalent (`add r v v` where v is already in A) skips the
/// leading staging MOV; ARMv7 simply uses the same register for
/// both source slots in the single ADD instruction.
#[test]
fn add_with_same_register_uses_it_as_both_rn_and_rm() {
    // const v=5; add r v v; ret r
    //
    // Allocator: v→r0, r→r1.
    //   MOV r0, #5       = 0xE3A0_0005
    //   ADD r1, r0, r0   = 0xE080_0000 | (0<<16) | (1<<12) | 0 = 0xE080_1000
    //   MOV r0, r1       = 0xE1A0_0001
    //   BX LR
    let f = IIRFunction::new("double", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(5)], "u8"),
        IIRInstr::new("add", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("v".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![
        0xE3A0_0005,    // MOV r0, #5
        0xE080_1000,    // ADD r1, r0, r0
        0xE1A0_0001,    // MOV r0, r1
        BX_LR,
    ], "self-add expected; got: {words:08x?}");
}

/// If the destination register matches the AAPCS return register r0,
/// the `ret r` stage-MOV is elided (just like the v0.2.0
/// `ret_of_first_const_omits_the_redundant_mov` regression).
///
/// We engineer this by burning r0 with the first const, then
/// re-using r0... but actually under the linear allocator the dest
/// will be the *next* slot, not r0.  To pin "dest_reg is r0" via
/// this slice we'd need register coalescing.  For now this test is
/// a placeholder that confirms the simple case works.
// ===========================================================================
// 8. A3++.5.5 first slice — bitwise data-processing-register ALU
// ===========================================================================
//
// Identical 3-register shape to add/sub.  Only the 4-bit opcode
// field (bits 24..21) changes:
//
//   AND = 0000 → 0xE000_0000 base
//   ORR = 1100 → 0xE180_0000 base   (ARM's "OR Register" mnemonic)
//   EOR = 0001 → 0xE020_0000 base   (ARM's "Exclusive OR" mnemonic)
//
// For each op below, the IIR sequence is `const v; const w; OP r v w; ret r`,
// the allocator places v→r0, w→r1, r→r2, and the emitted word stream
// follows the canonical:
//
//   MVI r0, v_imm
//   MVI r1, w_imm
//   OP  r2, r0, r1             ← the only word that varies between the three tests
//   MOV r0, r2                 ← stage r into r0 for ret
//   BX LR

#[test]
fn and_reg_base_pinned_to_0xe0000000() {
    assert_eq!(AND_REG_BASE, 0xE000_0000,
        "AND Rd, Rn, Rm base should be 0xE0000000; got 0x{:08x}", AND_REG_BASE);
}

#[test]
fn orr_reg_base_pinned_to_0xe1800000() {
    assert_eq!(ORR_REG_BASE, 0xE180_0000,
        "ORR Rd, Rn, Rm base should be 0xE1800000; got 0x{:08x}", ORR_REG_BASE);
}

#[test]
fn eor_reg_base_pinned_to_0xe0200000() {
    assert_eq!(EOR_REG_BASE, 0xE020_0000,
        "EOR Rd, Rn, Rm base should be 0xE0200000; got 0x{:08x}", EOR_REG_BASE);
}

#[test]
fn and_three_consts_emits_single_and_instruction() {
    // const v=0x0F; const w=0x33; and r v w; ret r
    //   AND r2, r0, r1 = 0xE000_0000 | (0<<16) | (2<<12) | 1 = 0xE000_2001
    let f = IIRFunction::new("and_fn", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0x0F)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(0x33)], "u8"),
        IIRInstr::new("and", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![
        0xE3A0_000F,    // MOV r0, #0x0F
        0xE3A0_1033,    // MOV r1, #0x33
        0xE000_2001,    // AND r2, r0, r1
        0xE1A0_0002,    // MOV r0, r2
        BX_LR,
    ], "and 0x0F & 0x33 expected; got: {words:08x?}");
}

#[test]
fn or_three_consts_emits_single_orr_instruction() {
    // const v=0x0F; const w=0xF0; or r v w; ret r
    //   ORR r2, r0, r1 = 0xE180_0000 | (0<<16) | (2<<12) | 1 = 0xE180_2001
    let f = IIRFunction::new("or_fn", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0x0F)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(0xF0)], "u8"),
        IIRInstr::new("or", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![
        0xE3A0_000F,    // MOV r0, #0x0F
        0xE3A0_10F0,    // MOV r1, #0xF0
        0xE180_2001,    // ORR r2, r0, r1
        0xE1A0_0002,    // MOV r0, r2
        BX_LR,
    ], "or 0x0F | 0xF0 expected; got: {words:08x?}");
}

#[test]
fn xor_three_consts_emits_single_eor_instruction() {
    // const v=0xFF; const w=0x55; xor r v w; ret r
    //   EOR r2, r0, r1 = 0xE020_0000 | (0<<16) | (2<<12) | 1 = 0xE020_2001
    let f = IIRFunction::new("xor_fn", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0xFF)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(0x55)], "u8"),
        IIRInstr::new("xor", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![
        0xE3A0_00FF,    // MOV r0, #0xFF
        0xE3A0_1055,    // MOV r1, #0x55
        0xE020_2001,    // EOR r2, r0, r1
        0xE1A0_0002,    // MOV r0, r2
        BX_LR,
    ], "xor 0xFF ^ 0x55 expected; got: {words:08x?}");
}

// ===========================================================================
// 9. A3++.5.5 second slice — carry-chained DP-register ALU (ADC, SBC)
// ===========================================================================
//
// Same 3-register shape as add/sub.  Only the 4-bit `opcode` field
// changes:
//
//   ADC = 0101 → 0xE0A0_0000 base   (add with carry-in)
//   SBC = 0110 → 0xE0C0_0000 base   (sub with borrow-in)
//
// ADC/SBC consume the C flag set by a PRIOR flag-affecting ALU op.
// This crate emits the non-S form by default — front-ends arrange
// for the producer to use the S-suffix variant so the carry chain
// survives.  The S-suffix variants land alongside `cmp` in v0.4.3.

#[test]
fn adc_reg_base_pinned_to_0xe0a00000() {
    assert_eq!(ADC_REG_BASE, 0xE0A0_0000,
        "ADC Rd, Rn, Rm base should be 0xE0A00000; got 0x{:08x}", ADC_REG_BASE);
}

#[test]
fn sbc_reg_base_pinned_to_0xe0c00000() {
    assert_eq!(SBC_REG_BASE, 0xE0C0_0000,
        "SBC Rd, Rn, Rm base should be 0xE0C00000; got 0x{:08x}", SBC_REG_BASE);
}

#[test]
fn adc_three_consts_emits_single_adc_instruction() {
    // const v=0x10; const w=0x20; adc r v w; ret r
    //   ADC r2, r0, r1 = 0xE0A0_0000 | (0<<16) | (2<<12) | 1 = 0xE0A0_2001
    let f = IIRFunction::new("adc_fn", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0x10)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(0x20)], "u8"),
        IIRInstr::new("adc", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![
        0xE3A0_0010,    // MOV r0, #0x10
        0xE3A0_1020,    // MOV r1, #0x20
        0xE0A0_2001,    // ADC r2, r0, r1
        0xE1A0_0002,    // MOV r0, r2
        BX_LR,
    ], "adc expected; got: {words:08x?}");
}

#[test]
fn sbb_three_consts_emits_single_sbc_instruction() {
    // const v=0x80; const w=0x01; sbb r v w; ret r
    //   SBC r2, r0, r1 = 0xE0C0_0000 | (0<<16) | (2<<12) | 1 = 0xE0C0_2001
    let f = IIRFunction::new("sbb_fn", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0x80)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(0x01)], "u8"),
        IIRInstr::new("sbb", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![
        0xE3A0_0080,    // MOV r0, #0x80
        0xE3A0_1001,    // MOV r1, #0x01
        0xE0C0_2001,    // SBC r2, r0, r1
        0xE1A0_0002,    // MOV r0, r2
        BX_LR,
    ], "sbb expected; got: {words:08x?}");
}

// ===========================================================================
// 10. A3++.5.5 third slice — `cmp` equality with flag-to-bool capture
// ===========================================================================
//
// ARMv7's KEY architectural feature — the 4-bit `cond` field at
// bits 31..28 of every A32 instruction — makes the equality capture
// remarkably clean:
//
//   CMP   rn, rm        ; sets Z if rn == rm
//   MOV   dest, #0      ; default false (cond = AL = 0xE)
//   MOVEQ dest, #1      ; if Z=1, overwrite to true (cond = EQ = 0x0)
//
// 4 words.  No address backpatching (the 8008 needed an inline
// JFZ + 2-byte address slot), no synthetic labels.

#[test]
fn cmp_reg_base_pinned_to_0xe1500000() {
    // CMP r0, r0 (S=1 forced, no Rd) = 0xE150_0000.
    assert_eq!(CMP_REG_BASE, 0xE150_0000,
        "CMP Rn, Rm base should be 0xE1500000; got 0x{:08x}", CMP_REG_BASE);
}

#[test]
fn mov_imm_eq_base_pinned_to_0x03a00000() {
    // MOVEQ r0, #0 = 0x03A0_0000.  Identical to MOV_IMM_R0_BASE
    // (0xE3A0_0000) except the top nibble (cond field) is 0 (EQ)
    // instead of E (AL).
    assert_eq!(MOV_IMM_EQ_BASE, 0x03A0_0000,
        "MOVEQ Rd, #imm base should be 0x03A00000; got 0x{:08x}", MOV_IMM_EQ_BASE);
}

#[test]
fn cmp_pins_full_capture_word_stream() {
    // const v=5; const w=5; cmp r v w; ret r
    //
    // Allocator: v→r0, w→r1, r→r2.
    //   MOV r0, #5    = 0xE3A0_0005
    //   MOV r1, #5    = 0xE3A0_1005
    //   CMP r0, r1    = 0xE150_0000 | (0<<16) | 1 = 0xE150_0001
    //   MOV r2, #0    = 0xE3A0_2000
    //   MOVEQ r2, #1  = 0x03A0_2001  ← cond=EQ (0) instead of AL (E)
    //   MOV r0, r2    = 0xE1A0_0002  ← stage r into r0 for ret
    //   BX LR         = 0xE12F_FF1E
    let f = IIRFunction::new("eq", vec![], "bool", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(5)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(5)], "u8"),
        IIRInstr::new("cmp", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "bool"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "bool"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![
        0xE3A0_0005,    // MOV r0, #5
        0xE3A0_1005,    // MOV r1, #5
        0xE150_0001,    // CMP r0, r1
        0xE3A0_2000,    // MOV r2, #0   (default false)
        0x03A0_2001,    // MOVEQ r2, #1 (Z=1 path)
        0xE1A0_0002,    // MOV r0, r2   (stage r into r0 for ret)
        BX_LR,
    ], "cmp expected 7-word sequence; got: {words:08x?}");
}

/// `cmp r v v` — same register as both operands.  The CMP encodes
/// with Rn == Rm; the result is always Z=1 at runtime.  Lowering
/// shape is identical — the optimisation of constant-folding this to
/// `MOV r, #1` is upstream's job.
#[test]
fn cmp_with_same_register_emits_cmp_a_a_then_capture() {
    // const v=42; cmp r v v; ret_void
    //
    // Allocator: v→r0, r→r1.
    //   MOV r0, #42      = 0xE3A0_002A
    //   CMP r0, r0       = 0xE150_0000 | (0<<16) | 0 = 0xE150_0000
    //   MOV r1, #0       = 0xE3A0_1000
    //   MOVEQ r1, #1     = 0x03A0_1001
    //   BX LR
    let f = IIRFunction::new("self_cmp", vec![], "void", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "u8"),
        IIRInstr::new("cmp", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("v".into())], "bool"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![
        0xE3A0_002A,    // MOV r0, #42
        0xE150_0000,    // CMP r0, r0 (Rn=Rm=0)
        0xE3A0_1000,    // MOV r1, #0
        0x03A0_1001,    // MOVEQ r1, #1
        BX_LR,
    ], "self-cmp expected; got: {words:08x?}");
}

// ===========================================================================
// 11. A3++.5.5 fourth slice — cmp_ne / cmp_lt / cmp_gt / cmp_gte / cmp_lte
// ===========================================================================
//
// Same CMP + MOV + MOV<cond> capture skeleton as v0.4.3's `cmp` —
// only the trailing MOV's condition prefix changes.  The (op →
// condition) mapping is:
//
//   cmp_ne  → NE (0x13A0..)
//   cmp_lt  → CC (0x33A0..)
//   cmp_gt  → HI (0x83A0..)
//   cmp_gte → CS (0x23A0..)
//   cmp_lte → LS (0x93A0..)
//
// The byte streams below pin each variant against the canonical
// "v=10, w=20" template.  Allocator: v→r0, w→r1, r→r2.  CMP r0, r1
// = 0xE150_0001 (Rn=0, Rm=1).
//
//   00:  MOV r0, #10               (0xE3A0_000A)
//   01:  MOV r1, #20               (0xE3A0_1014)
//   02:  CMP r0, r1                (0xE150_0001)
//   03:  MOV r2, #0                (0xE3A0_2000)
//   04:  MOV<cond> r2, #1          (<cond_base> | 0x2001)
//   05:  MOV r0, r2                (0xE1A0_0002)
//   06:  BX LR

#[test]
fn mov_imm_ne_base_pinned_to_0x13a00000() {
    assert_eq!(MOV_IMM_NE_BASE, 0x13A0_0000,
        "MOVNE Rd, #imm base should be 0x13A00000; got 0x{:08x}", MOV_IMM_NE_BASE);
}

#[test]
fn mov_imm_cc_base_pinned_to_0x33a00000() {
    assert_eq!(MOV_IMM_CC_BASE, 0x33A0_0000);
}

#[test]
fn mov_imm_cs_base_pinned_to_0x23a00000() {
    assert_eq!(MOV_IMM_CS_BASE, 0x23A0_0000);
}

#[test]
fn mov_imm_hi_base_pinned_to_0x83a00000() {
    assert_eq!(MOV_IMM_HI_BASE, 0x83A0_0000);
}

#[test]
fn mov_imm_ls_base_pinned_to_0x93a00000() {
    assert_eq!(MOV_IMM_LS_BASE, 0x93A0_0000);
}

/// Shared helper used by the five comparison-variant tests below.
/// Builds the canonical "const v=10; const w=20; OP r v w; ret r"
/// IIR sequence, runs lowering, and asserts the 7-word output
/// matches the standard CMP-capture template with the given
/// condition-MOV word at index 4.
fn assert_cmp_variant(op: &str, expected_cond_mov: u32) {
    let f = IIRFunction::new(op, vec![], "bool", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(10)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(20)], "u8"),
        IIRInstr::new(op, Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "bool"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "bool"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![
        0xE3A0_000A,           // MOV r0, #10
        0xE3A0_1014,           // MOV r1, #20
        0xE150_0001,           // CMP r0, r1
        0xE3A0_2000,           // MOV r2, #0
        expected_cond_mov,     // MOV<cond> r2, #1
        0xE1A0_0002,           // MOV r0, r2 (stage r)
        BX_LR,
    ], "{op} expected; got: {words:08x?}");
}

#[test]
fn cmp_ne_uses_movne_for_set_true() {
    // MOVNE r2, #1 = 0x13A0_0000 | (2<<12) | 1 = 0x13A0_2001
    assert_cmp_variant("cmp_ne", 0x13A0_2001);
}

#[test]
fn cmp_lt_uses_movcc_for_set_true() {
    // MOVCC r2, #1 = 0x33A0_0000 | (2<<12) | 1 = 0x33A0_2001
    assert_cmp_variant("cmp_lt", 0x33A0_2001);
}

#[test]
fn cmp_gt_uses_movhi_for_set_true() {
    // MOVHI r2, #1 = 0x83A0_0000 | (2<<12) | 1 = 0x83A0_2001
    assert_cmp_variant("cmp_gt", 0x83A0_2001);
}

#[test]
fn cmp_gte_uses_movcs_for_set_true() {
    // MOVCS r2, #1 = 0x23A0_0000 | (2<<12) | 1 = 0x23A0_2001
    assert_cmp_variant("cmp_gte", 0x23A0_2001);
}

#[test]
fn cmp_lte_uses_movls_for_set_true() {
    // MOVLS r2, #1 = 0x93A0_0000 | (2<<12) | 1 = 0x93A0_2001
    assert_cmp_variant("cmp_lte", 0x93A0_2001);
}

// ===========================================================================
// 12. A3++.5.5 fifth slice — branches (`label` + `jmp` + `jmp_if_*`)
// ===========================================================================
//
// ARMv7's B instruction carries a 24-bit signed PC-relative offset
// in WORDS (shifted left 2 to convert to bytes by the silicon).
// PC at execute time = current_instruction_address + 8 (the classic
// ARM 2-stage pipeline prefetch offset).
//
// For a branch at word index `S` targeting word index `T`:
//   imm24 = T - S - 2     (the -2 = 8 bytes / 4 = 2 words for the
//                          PC prefetch quirk)
//
// The boolean-branch idiom adds a CMP cond_reg, #0 in front to
// provoke the Z flag from the cond register, then uses BNE/BEQ.

#[test]
fn b_base_pinned_to_0xea000000() {
    assert_eq!(B_BASE, 0xEA00_0000,
        "B (cond=AL) base should be 0xEA000000; got 0x{:08x}", B_BASE);
}

#[test]
fn b_ne_base_pinned_to_0x1a000000() {
    assert_eq!(B_NE_BASE, 0x1A00_0000);
}

#[test]
fn b_eq_base_pinned_to_0x0a000000() {
    assert_eq!(B_EQ_BASE, 0x0A00_0000);
}

#[test]
fn cmp_imm_zero_base_pinned_to_0xe3500000() {
    assert_eq!(CMP_IMM_ZERO_BASE, 0xE350_0000,
        "CMP Rn, #0 base should be 0xE3500000; got 0x{:08x}", CMP_IMM_ZERO_BASE);
}

#[test]
fn jmp_to_forward_label_backpatches_correct_offset() {
    // const v=42; jmp end; const w=99; label end; ret v
    //
    // Layout (word indices):
    //   0: MOV r0, #42       (3E A0 002A — wait, 0xE3A0_002A)
    //   1: B end             (0xEA00_???? — backpatched)
    //   2: MOV r1, #99       (0xE3A0_1063)
    //   <-- label "end" at word index 3 -->
    //   3: BX LR             (0xE12F_FF1E)
    //
    // For B at slot 1 targeting word index 3:
    //   imm24 = 3 - 1 - 2 = 0
    // So the B word = 0xEA00_0000 | 0 = 0xEA00_0000.
    let f = IIRFunction::new("fwd", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "u8"),
        IIRInstr::new("jmp",   None,             vec![Operand::Var("end".into())], "void"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(99)], "u8"),
        IIRInstr::new("label", None,             vec![Operand::Var("end".into())], "void"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "u8"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![
        0xE3A0_002A,    // 0: MOV r0, #42  (v)
        B_BASE,     // 1: B 0 (target = current + 8 = +0 words past prefetch)
        0xE3A0_1063,    // 2: MOV r1, #99 (w, unreachable)
        // label "end" at word index 3
        BX_LR,          // 3: BX LR (ret v — v is already in r0)
    ], "forward jmp expected; got: {words:08x?}");
}

#[test]
fn jmp_to_backward_label_emits_negative_offset() {
    // label loop; const v=1; jmp loop; ret_void
    //
    //   <-- label "loop" at word index 0 -->
    //   0: MOV r0, #1
    //   1: B loop  (target = 0; imm24 = 0 - 1 - 2 = -3)
    //   2: BX LR
    //
    // imm24 = -3 = 0xFFFFFD in signed 24-bit.  Masked to 24 bits and
    // OR'd into B_BASE: 0xEA00_0000 | 0xFFFFFD = 0xEAFFFFFD.
    let f = IIRFunction::new("backward", vec![], "void", vec![
        IIRInstr::new("label", None,             vec![Operand::Var("loop".into())], "void"),
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(1)], "u8"),
        IIRInstr::new("jmp",   None,             vec![Operand::Var("loop".into())], "void"),
        IIRInstr::new("ret_void", None,          vec![], "void"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![
        0xE3A0_0001,    // 0: MOV r0, #1
        0xEAFF_FFFD,    // 1: B loop (imm24 = -3)
        BX_LR,          // 2: BX LR
    ], "backward jmp expected; got: {words:08x?}");
}

#[test]
fn jmp_to_undefined_label_is_rejected() {
    let f = IIRFunction::new("dangling", vec![], "void", vec![
        IIRInstr::new("jmp", None, vec![Operand::Var("nowhere".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let err = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect_err("jmp to nonexistent label should fail");
    match err {
        IIRArmv7Error::UndefinedLabel { function, label } => {
            assert_eq!(function, "dangling");
            assert_eq!(label, "nowhere");
        }
        other => panic!("expected UndefinedLabel, got: {other:?}"),
    }
}

#[test]
fn jmp_if_true_emits_cmp_zero_then_bne() {
    // const cond=1; jmp_if_true cond, end; const x=0; label end; ret_void
    //
    //   0: MOV r0, #1         (cond → r0)
    //   1: CMP r0, #0         (0xE350_0000 | (0<<16) = 0xE350_0000)
    //   2: BNE end            (target = word 5; imm24 = 5-2-2 = 1)
    //   3: MOV r1, #0         (x — unreachable)
    //   <-- label end at word 4 -->
    //   4: BX LR
    //
    // Wait, the count: words[0]=MOV r0, words[1]=CMP, words[2]=BNE, words[3]=MOV r1.
    // Then label "end" at word index 4. Then BX LR at word 4.
    // imm24 = target - slot - 2 = 4 - 2 - 2 = 0.
    let f = IIRFunction::new("if_true", vec![], "void", vec![
        IIRInstr::new("const", Some("cond".into()), vec![Operand::Int(1)], "bool"),
        IIRInstr::new("jmp_if_true", None,
            vec![Operand::Var("cond".into()), Operand::Var("end".into())], "void"),
        IIRInstr::new("const", Some("x".into()), vec![Operand::Int(0)], "u8"),
        IIRInstr::new("label", None, vec![Operand::Var("end".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![
        0xE3A0_0001,    // 0: MOV r0, #1 (cond)
        0xE350_0000,    // 1: CMP r0, #0
        B_NE_BASE,  // 2: BNE end (imm24 = 4 - 2 - 2 = 0)
        0xE3A0_1000,    // 3: MOV r1, #0 (x — unreachable)
        BX_LR,          // 4: BX LR
    ], "jmp_if_true expected; got: {words:08x?}");
}

#[test]
fn jmp_if_false_emits_cmp_zero_then_beq() {
    // Same layout as jmp_if_true but with BEQ.
    let f = IIRFunction::new("if_false", vec![], "void", vec![
        IIRInstr::new("const", Some("cond".into()), vec![Operand::Int(0)], "bool"),
        IIRInstr::new("jmp_if_false", None,
            vec![Operand::Var("cond".into()), Operand::Var("end".into())], "void"),
        IIRInstr::new("const", Some("x".into()), vec![Operand::Int(99)], "u8"),
        IIRInstr::new("label", None, vec![Operand::Var("end".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    assert_eq!(words, vec![
        0xE3A0_0000,    // 0: MOV r0, #0 (cond)
        0xE350_0000,    // 1: CMP r0, #0
        B_EQ_BASE,  // 2: BEQ end
        0xE3A0_1063,    // 3: MOV r1, #99 (unreachable)
        BX_LR,          // 4: BX LR
    ], "jmp_if_false expected; got: {words:08x?}");
}

#[test]
fn errors_for_branch_variants_display_without_panic() {
    let _ = format!("{}", IIRArmv7Error::UndefinedLabel {
        function: "f".into(), label: "ghost".into(),
    });
    let _ = format!("{}", IIRArmv7Error::BranchOutOfRange {
        function: "huge".into(), target: 10_000_000, current: 0,
    });
}

// ===========================================================================
// 13. A3++.6 — real `call` via BL with module-level backpatching
// ===========================================================================
//
// BL (branch with link) has bit 24 SET vs B's bit 24 CLEAR — the
// same family-bit difference as the 8008's JMP ↔ CAL (0x7C ↔ 0x7E).
// The silicon writes PC+4 into LR before branching, so a subsequent
// BX LR in the callee returns to the next instruction.
//
// Module-level resolution mirrors the 8008's v0.3.9: function_addrs
// records each function's start word index; pending_calls records
// (slot, callee, caller); the post-loop pass walks pending_calls,
// resolves callees, range-checks, and OR-encodes the BL word.

#[test]
fn bl_base_pinned_to_0xeb000000() {
    // BL = B with bit 24 set.  B_BASE = 0xEA00_0000 → BL_BASE =
    // 0xEB00_0000.
    assert_eq!(BL_BASE, 0xEB00_0000,
        "BL (cond=AL) base should be 0xEB000000; got 0x{:08x}", BL_BASE);
}

#[test]
fn call_emits_bl_with_backpatched_pc_relative_offset() {
    // Two functions: `main` (entry) calls `helper`.
    //
    // Layout (word indices):
    //   main:
    //     0: BL helper        (placeholder; backpatched in pass 2)
    //     1: BX LR
    //   helper:
    //     2: MOV r0, #7
    //     3: BX LR
    //
    // After backpatching, the BL at slot 0 targets helper at word 2.
    // imm24 = 2 - 0 - 2 = 0. So BL = 0xEB00_0000 | 0 = 0xEB00_0000.
    //
    // r is bound by the `call dest, helper` IIR op; the allocator
    // picks r0 for it (first local in main).  Since dest_reg == r0,
    // no capture MOV is emitted.
    let main_fn = IIRFunction::new("main", vec![], "u8", vec![
        IIRInstr::new("call", Some("r".into()),
            vec![Operand::Var("helper".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let helper = IIRFunction::new("helper", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "u8"),
    ]);
    let words = lower_iir_to_armv7(
        &multi_fn_module("main", vec![main_fn, helper]),
        &IIRArmv7Config::default(),
    ).expect("lowering");
    assert_eq!(words, vec![
        BL_BASE,    // 0:  BL helper (imm24 = 0)
        BX_LR,          // 1:  BX LR  (ret r — r is in r0 from the call's dest_reg)
        0xE3A0_0007,    // 2:  MOV r0, #7   (helper: const v)
        BX_LR,          // 3:  BX LR  (helper: ret v)
    ], "call + BX LR expected; got: {words:08x?}");
}

#[test]
fn call_with_helper_before_main_emits_negative_offset() {
    // helper is defined FIRST, then main calls it (backward call).
    //
    //   helper:
    //     0: MOV r0, #5
    //     1: BX LR
    //   main:
    //     2: BL helper        (imm24 = 0 - 2 - 2 = -4)
    //     3: BX LR
    //
    // imm24 = -4 = 0xFFFFFC in 24-bit two's-complement.  BL word =
    // 0xEB00_0000 | 0xFFFFFC = 0xEBFFFFFC.
    let helper = IIRFunction::new("helper", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(5)], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "u8"),
    ]);
    let main_fn = IIRFunction::new("main", vec![], "u8", vec![
        IIRInstr::new("call", Some("r".into()),
            vec![Operand::Var("helper".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let words = lower_iir_to_armv7(
        &multi_fn_module("main", vec![helper, main_fn]),
        &IIRArmv7Config::default(),
    ).expect("lowering");
    assert_eq!(words, vec![
        0xE3A0_0005,    // 0:  MOV r0, #5  (helper: const v)
        BX_LR,          // 1:  BX LR       (helper: ret v)
        0xEBFF_FFFC,    // 2:  BL helper   (imm24 = -4)
        BX_LR,          // 3:  BX LR       (main: ret r)
    ], "backward call expected; got: {words:08x?}");
}

#[test]
fn call_to_undefined_function_is_rejected() {
    let main_fn = IIRFunction::new("main", vec![], "void", vec![
        IIRInstr::new("call", None, vec![Operand::Var("ghost".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let err = lower_iir_to_armv7(
        &multi_fn_module("main", vec![main_fn]),
        &IIRArmv7Config::default(),
    ).expect_err("call to undefined function should fail");
    match err {
        IIRArmv7Error::UndefinedFunction { caller, callee } => {
            assert_eq!(caller, "main");
            assert_eq!(callee, "ghost");
        }
        other => panic!("expected UndefinedFunction, got: {other:?}"),
    }
}

#[test]
fn call_with_no_dest_discards_return_value() {
    // Void-call shape: no register allocated for the return value.
    let main_fn = IIRFunction::new("main", vec![], "void", vec![
        IIRInstr::new("call", None, vec![Operand::Var("helper".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let helper = IIRFunction::new("helper", vec![], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let words = lower_iir_to_armv7(
        &multi_fn_module("main", vec![main_fn, helper]),
        &IIRArmv7Config::default(),
    ).expect("lowering");
    assert_eq!(words, vec![
        BL_BASE,    // 0:  BL helper (imm24 = 0, helper at word 2)
        BX_LR,          // 1:  BX LR (main ret_void)
        BX_LR,          // 2:  BX LR (helper ret_void)
    ], "void call expected; got: {words:08x?}");
}

#[test]
fn errors_for_undefined_function_display_without_panic() {
    let _ = format!("{}", IIRArmv7Error::UndefinedFunction {
        caller: "main".into(), callee: "ghost".into(),
    });
}

#[test]
fn add_then_ret_into_non_r0_register_emits_staging_mov() {
    // Regression for: the v0.3.0 ret-staging logic still fires for
    // ALU dest registers.
    let f = IIRFunction::new("f", vec![], "u8", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(1)], "u8"),
        IIRInstr::new("const", Some("w".into()), vec![Operand::Int(2)], "u8"),
        IIRInstr::new("add", Some("r".into()),
            vec![Operand::Var("v".into()), Operand::Var("w".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ]);
    let words = lower_iir_to_armv7(&module_with(f), &IIRArmv7Config::default())
        .expect("lowering");
    // 5 words: 2 MVI + 1 ADD + 1 stage MOV + BX LR.
    assert_eq!(words.len(), 5);
    // The 4th word (index 3) is the stage MOV r0, r2 = 0xE1A0_0002.
    assert_eq!(words[3], 0xE1A0_0002,
        "expected stage MOV r0, r2 at index 3; got 0x{:08x}", words[3]);
    assert_eq!(words[4], BX_LR);
}
