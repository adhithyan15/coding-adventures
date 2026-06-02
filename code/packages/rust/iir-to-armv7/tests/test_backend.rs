//! Integration tests for `iir-to-armv7` v0.1.0 (A3 skeleton).
//!
//! Smoke-level — confirms the validator stub, the emitter shape, and
//! the exact encoded `BKPT #0xFFFF` word (`0xE12FFF7F`).

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_armv7::{
    lower_iir_to_armv7, validate_for_armv7,
    IIRArmv7Config, IIRArmv7Error,
    ADD_REG_BASE, BKPT, BX_LR, MOV_IMM_R0_BASE, MOV_REG_BASE, SUB_REG_BASE,
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
