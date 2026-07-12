//! Integration tests for `iir-to-riscv`.
//!
//! Note: this crate is deprecated as of v0.4.0 (Phase 7 of the
//! historical-arch backend migration, the FINAL lane).  Tests still
//! exercise the deprecated API as a regression invariant.
#![allow(deprecated)]
//!
//! Test groups (grow with each release):
//!
//! 1. Validator behaviour
//! 2. Empty module emits nothing
//! 3. `ret_void`-only function — just the canonical 0x0000_8067
//! 4. const + ret (A1+ — pinned exact addi encoding)
//! 5. add / sub (A1+ — R-type opcodes)
//! 6. mov (A1+ — addi rd, rs1, 0 idiom)
//! 7. Param → return value (a0 stays in a0; no extra `mv`)
//! 8. Out-of-range immediate is rejected
//! 9. Register-pool exhaustion is rejected
//! 10. Too many params is rejected
//! 11. Config + error display smoke

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_riscv::{lower_iir_to_riscv, validate_for_riscv, IIRRiscvConfig, IIRRiscvError};
use riscv_simulator::encoding::{encode_add, encode_addi, encode_sub};

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

fn lower(module: &IIRModule) -> Vec<u32> {
    lower_iir_to_riscv(module, &IIRRiscvConfig::default()).expect("lowering should succeed")
}

// Register-name constants used by the assertions below.
const X0:  u32 = 0;
// Kept for completeness of the RISC-V register-name table even though no
// current assertion references x1 directly.
#[allow(dead_code)]
const X1:  u32 = 1;
const A0:  u32 = 10;
const A1:  u32 = 11;
const T0:  u32 = 5;
const T1:  u32 = 6;

const CANONICAL_RET: u32 = 0x0000_8067;

// ===========================================================================
// 1. Validator behaviour
// ===========================================================================

#[test]
fn validate_returns_empty_for_empty_module() {
    assert!(validate_for_riscv(&empty_module()).is_empty());
}

#[test]
fn validate_accepts_supported_ret_void_function() {
    let f = IIRFunction::new("main", vec![], "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    assert!(validate_for_riscv(&module_with(f)).is_empty());
}

#[test]
fn validate_rejects_unsupported_op() {
    let f = IIRFunction::new("main", vec![], "void", vec![
        IIRInstr::new("safepoint", None, vec![], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let errors = validate_for_riscv(&module_with(f));
    assert!(errors.iter().any(|e| e.contains("UnsupportedOp")),
        "expected UnsupportedOp for `safepoint`; got: {errors:?}");
}

#[test]
fn validate_rejects_unsupported_type() {
    let f = IIRFunction::new("main", vec![], "f64",
        vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    let errors = validate_for_riscv(&module_with(f));
    assert!(errors.iter().any(|e| e.contains("UnsupportedType")),
        "expected UnsupportedType for f64 ret; got: {errors:?}");
}

#[test]
fn validate_rejects_too_many_params() {
    let params: Vec<(String, String)> = (0..9)
        .map(|i| (format!("p{i}"), "i32".into())).collect();
    let f = IIRFunction::new("main", params, "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    let errors = validate_for_riscv(&module_with(f));
    assert!(errors.iter().any(|e| e.contains("TooManyParams")),
        "expected TooManyParams; got: {errors:?}");
}

// ===========================================================================
// 2. Empty module emits nothing
// ===========================================================================

#[test]
fn empty_module_emits_no_words() {
    let words = lower(&empty_module());
    assert!(words.is_empty(),
        "empty module should produce empty word list; got: {words:?}");
}

// ===========================================================================
// 3. ret_void-only function
// ===========================================================================

#[test]
fn ret_void_only_function_emits_just_the_canonical_ret() {
    let f = IIRFunction::new("main", vec![], "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    let words = lower(&module_with(f));
    assert_eq!(words, vec![CANONICAL_RET],
        "expected just the canonical 0x0000_8067; got: {words:#x?}");
}

// ===========================================================================
// 4. const + ret — pinned exact encoding
// ===========================================================================

/// `fn answer() -> i32 { const v = 7; ret v }`
/// expected sequence:
///   addi t0, x0, 7       ; bind v ← 7
///   addi a0, t0, 0       ; mv a0, v
///   jalr x0, x1, 0       ; ret
#[test]
fn const_plus_ret_emits_three_pinned_words() {
    let f = IIRFunction::new("answer", vec![], "i32", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "i32"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "i32"),
    ]);
    let words = lower(&module_with(f));
    assert_eq!(words.len(), 3, "expected 3 words; got {words:#x?}");
    assert_eq!(words[0], encode_addi(T0, X0, 7), "first word should be `addi t0, x0, 7`");
    assert_eq!(words[1], encode_addi(A0, T0, 0), "second word should be `addi a0, t0, 0` (mv)");
    assert_eq!(words[2], CANONICAL_RET);
}

#[test]
fn const_of_negative_small_int_is_supported() {
    let f = IIRFunction::new("f", vec![], "i32", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(-2048)], "i32"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "i32"),
    ]);
    let words = lower(&module_with(f));
    assert_eq!(words[0], encode_addi(T0, X0, -2048),
        "minimum 12-bit signed imm (-2048) must encode cleanly");
}

#[test]
fn const_out_of_i32_range_is_rejected() {
    // After A1++ added lui+addi, the rejection threshold moved up from
    // i12::MAX to i32::MAX.  Values that fit in i32 lower cleanly via
    // the wide-immediate idiom; values outside i32 still need 64-bit
    // pair handling (A1++.5).
    let v = (i32::MAX as i64) + 1;
    let f = IIRFunction::new("f", vec![], "i32", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(v)], "i32"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "i32"),
    ]);
    let err = lower_iir_to_riscv(&module_with(f), &IIRRiscvConfig::default())
        .expect_err("oversized (>i32) const should fail");
    match err {
        IIRRiscvError::ImmediateOutOfRange { value, .. } => assert_eq!(value, v),
        other => panic!("expected ImmediateOutOfRange, got: {other:?}"),
    }
}

// ===========================================================================
// 5. add / sub — R-type opcodes
// ===========================================================================

/// `fn f(a: i32, b: i32) -> i32 { v = a + b; ret v }`
/// expected sequence:
///   add t0, a0, a1       ; v ← a + b
///   addi a0, t0, 0       ; mv a0, v
///   ret
#[test]
fn add_two_params_emits_r_type_add_then_mv_then_ret() {
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("add", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
        ],
    );
    let words = lower(&module_with(f));
    assert_eq!(words.len(), 3);
    assert_eq!(words[0], encode_add(T0, A0, A1),
        "first word should be `add t0, a0, a1`");
    assert_eq!(words[1], encode_addi(A0, T0, 0));
    assert_eq!(words[2], CANONICAL_RET);
}

#[test]
fn sub_two_params_emits_r_type_sub() {
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("sub", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
        ],
    );
    let words = lower(&module_with(f));
    assert_eq!(words[0], encode_sub(T0, A0, A1));
}

// ===========================================================================
// 6. mov — `addi rd, rs1, 0`
// ===========================================================================

#[test]
fn mov_emits_addi_zero_canonical_move() {
    let f = IIRFunction::new(
        "f",
        vec![("x".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("mov", Some("y".into()),
                vec![Operand::Var("x".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("y".into())], "i32"),
        ],
    );
    let words = lower(&module_with(f));
    // mov y, x  ⇒  addi t0, a0, 0
    assert_eq!(words[0], encode_addi(T0, A0, 0),
        "mov should be canonical `addi rd, rs1, 0`");
}

// ===========================================================================
// 7. Identity: ret of param[0] skips the redundant `mv a0, a0`
// ===========================================================================

#[test]
fn ret_of_first_param_skips_redundant_mv() {
    let f = IIRFunction::new(
        "identity",
        vec![("x".into(), "i32".into())],
        "i32",
        vec![IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i32")],
    );
    let words = lower(&module_with(f));
    // Just the ret — no `mv a0, a0` since x already lives in a0.
    assert_eq!(words, vec![CANONICAL_RET],
        "identity(x) should emit only ret; got: {words:#x?}");
}

// ===========================================================================
// 8. Register-pool exhaustion is rejected
// ===========================================================================

#[test]
fn out_of_registers_when_pool_exhausted() {
    // 8 consts in a row — TEMP_REGISTERS holds only 7.
    let mut body = vec![];
    for i in 0..8 {
        body.push(IIRInstr::new(
            "const",
            Some(format!("v{i}")),
            vec![Operand::Int(i as i64)],
            "i32",
        ));
    }
    body.push(IIRInstr::new("ret_void", None, vec![], "void"));
    let f = IIRFunction::new("greedy", vec![], "void", body);

    let err = lower_iir_to_riscv(&module_with(f), &IIRRiscvConfig::default())
        .expect_err("8 locals should exhaust the pool");
    match err {
        IIRRiscvError::OutOfRegisters { .. } => {}
        other => panic!("expected OutOfRegisters, got: {other:?}"),
    }
}

// ===========================================================================
// 9. Config + error display smoke
// ===========================================================================

#[test]
fn default_config_has_nonempty_module_name() {
    assert!(!IIRRiscvConfig::default().module_name.is_empty());
}

#[test]
fn errors_display_without_panic() {
    let _ = format!("{}", IIRRiscvError::ValidationFailed(vec!["x".into()]));
    let _ = format!("{}", IIRRiscvError::UnsupportedOp {
        function: "f".into(), op: "weird".into(),
    });
    let _ = format!("{}", IIRRiscvError::UnsupportedType {
        function: "f".into(), type_hint: "weird".into(),
    });
    let _ = format!("{}", IIRRiscvError::InvalidOperand {
        function: "f".into(), detail: "bad".into(),
    });
    let _ = format!("{}", IIRRiscvError::UndefinedVariable {
        function: "f".into(), name: "ghost".into(),
    });
    let _ = format!("{}", IIRRiscvError::TooManyParams {
        function: "f".into(), count: 9,
    });
    let _ = format!("{}", IIRRiscvError::OutOfRegisters {
        function: "f".into(), name: "v8".into(),
    });
    let _ = format!("{}", IIRRiscvError::ImmediateOutOfRange {
        function: "f".into(), value: 99_999,
    });
}

// ===========================================================================
// 12. A1++ — wide constants via lui+addi
// ===========================================================================
//
// After A1++ shipped lui+addi, `const v = 4096` no longer rejects — it
// lowers to `lui + (optionally) addi` for the upper 20 / lower 12 split.
// Carry handling: when low12 is negative (top bit set) we add 1 to
// upper20.  The two tests below pin both the no-addi and with-addi
// paths.

#[test]
fn const_4096_lowers_via_lui_then_addi_skipped() {
    // 4096 == 0x1000 — exactly upper20=1, lower12=0 → just lui, no addi.
    let f = IIRFunction::new("f", vec![], "i32", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(4096)], "i32"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "i32"),
    ]);
    let words = lower(&module_with(f));
    // Expected: lui t0, 1 ;  mv a0, t0 (addi) ;  ret
    assert_eq!(words.len(), 3,
        "4096 should lower as `lui t0, 1` + mv + ret (no extra addi); got {words:#x?}");
}

#[test]
fn const_4097_lowers_via_lui_plus_addi() {
    // 4097 == 0x1001 — upper20=1, lower12=1 (positive 1), no carry.
    let f = IIRFunction::new("f", vec![], "i32", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(4097)], "i32"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "i32"),
    ]);
    let words = lower(&module_with(f));
    // Expected: lui t0, 1 ;  addi t0, t0, 1 ;  mv a0, t0 ;  ret
    assert_eq!(words.len(), 4,
        "4097 should lower as lui + addi + mv + ret; got {words:#x?}");
}

// ===========================================================================
// 13. A1++ — comparison ops produce 0/1 in a register
// ===========================================================================

#[test]
fn cmp_lt_signed_emits_slt() {
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("lt", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
        ],
    );
    let words = lower(&module_with(f));
    let expected = riscv_simulator::encoding::encode_slt(T0, A0, A1);
    assert_eq!(words[0], expected,
        "expected slt t0, a0, a1; got 0x{:08x}", words[0]);
}

#[test]
fn cmp_lt_unsigned_emits_sltu() {
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "u32".into()), ("b".into(), "u32".into())],
        "u32",
        vec![
            IIRInstr::new("lt", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "u32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "u32"),
        ],
    );
    let words = lower(&module_with(f));
    let expected = riscv_simulator::encoding::encode_sltu(T0, A0, A1);
    assert_eq!(words[0], expected,
        "expected sltu t0, a0, a1; got 0x{:08x}", words[0]);
}

#[test]
fn cmp_eq_synthesizes_xor_plus_sltiu() {
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("eq", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
        ],
    );
    let words = lower(&module_with(f));
    let xor = riscv_simulator::encoding::encode_xor(T0, A0, A1);
    let sltiu_ = riscv_simulator::encoding::encode_sltiu(T0, T0, 1);
    assert_eq!(words[0], xor);
    assert_eq!(words[1], sltiu_);
}

#[test]
fn cmp_ne_synthesizes_xor_plus_sltu_with_x0() {
    let f = IIRFunction::new(
        "f",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("ne", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
        ],
    );
    let words = lower(&module_with(f));
    let xor = riscv_simulator::encoding::encode_xor(T0, A0, A1);
    let sltu_x0 = riscv_simulator::encoding::encode_sltu(T0, X0, T0);
    assert_eq!(words[0], xor);
    assert_eq!(words[1], sltu_x0);
}

#[test]
fn cmp_prefixed_alias_lowers_like_naked() {
    // `cmp_lt` should produce identical bytes to `lt`.
    let f1 = IIRFunction::new(
        "g",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("cmp_lt", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
        ],
    );
    let w1 = lower(&module_with(f1));
    let f2 = IIRFunction::new(
        "g",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("lt", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
        ],
    );
    let w2 = lower(&module_with(f2));
    assert_eq!(w1, w2, "`cmp_lt` should lower identically to `lt`");
}

// ===========================================================================
// 14. A1++ — call_builtin "print_i64" → ecall on RV32I
// ===========================================================================

#[test]
fn call_builtin_print_i64_emits_ecall_with_a7_syscall() {
    use riscv_simulator::encoding::{encode_addi, encode_ecall};
    const A7: u32 = 17;
    let f = IIRFunction::new("main", vec![], "void", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "i32"),
        IIRInstr::new("call_builtin", None,
            vec![Operand::Var("print_i64".into()), Operand::Var("v".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let words = lower(&module_with(f));
    // Sequence:
    //   addi t0, x0, 42       ; v = 42
    //   addi a0, t0, 0        ; mv a0, v
    //   addi a7, x0, 1        ; syscall #1 (print_i64)
    //   ecall
    //   ret
    let expected = vec![
        encode_addi(T0, X0, 42),
        encode_addi(A0, T0, 0),
        encode_addi(A7, X0, 1),
        encode_ecall(),
        CANONICAL_RET,
    ];
    assert_eq!(words, expected,
        "print_i64 sequence mismatch; got {words:#x?}, expected {expected:#x?}");
}

#[test]
fn call_builtin_unknown_name_is_unsupported() {
    let f = IIRFunction::new("main", vec![], "void", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(0)], "i32"),
        IIRInstr::new("call_builtin", None,
            vec![Operand::Var("not_a_real_builtin".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let err = lower_iir_to_riscv(&module_with(f), &IIRRiscvConfig::default())
        .expect_err("unknown builtin should fail");
    match err {
        IIRRiscvError::UnsupportedOp { op, .. } => {
            assert!(op.contains("not_a_real_builtin"),
                "error should mention the unknown name; got: {op}");
        }
        other => panic!("expected UnsupportedOp, got: {other:?}"),
    }
}

// ===========================================================================
// 15. A1++.5 — control flow within a function
// ===========================================================================
//
// `label "L"` records a byte offset; `jmp "L"` → `jal x0, +offset`;
// `jmp_if_true cond, L` → `bne cond, x0, +offset`;
// `jmp_if_false cond, L` → `beq cond, x0, +offset`.
//
// Offsets are resolved in a second pass after every label is known.

#[test]
fn jmp_around_a_dead_block_patches_jal_with_real_offset() {
    use riscv_simulator::encoding::encode_jal;
    // Module:
    //   const v = 1
    //   jmp "L_end"
    //   const dead = 99    ; unreachable but kept to grow the gap
    //   label "L_end"
    //   ret v
    let f = IIRFunction::new("f", vec![], "i32", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(1)], "i32"),
        IIRInstr::new("jmp",   None,             vec![Operand::Var("L_end".into())], "void"),
        IIRInstr::new("const", Some("dead".into()), vec![Operand::Int(99)], "i32"),
        IIRInstr::new("label", None, vec![Operand::Var("L_end".into())], "void"),
        IIRInstr::new("ret",   None, vec![Operand::Var("v".into())], "i32"),
    ]);
    let words = lower(&module_with(f));
    // Expected layout:
    //   [0] addi t0, x0, 1      ; v = 1                    @ byte 0
    //   [1] jal  x0, +8         ; jmp to L_end             @ byte 4
    //   [2] addi t1, x0, 99     ; dead = 99                @ byte 8
    //   [3] (label L_end here)  ; addi a0, t0, 0  @ byte 12  ← target
    //   [4] jalr x0, x1, 0      ; ret                      @ byte 16
    //
    // So jal at byte 4 should jump +8 bytes (target byte 12).
    assert_eq!(words[1], encode_jal(X0, 8),
        "jal at byte 4 should encode +8 offset; got 0x{:08x}", words[1]);
}

#[test]
fn jmp_if_true_emits_bne_with_resolved_offset() {
    use riscv_simulator::encoding::encode_bne;
    // const cond = 1
    // jmp_if_true cond, "L_end"
    // const dead = 99
    // label "L_end"
    // ret_void
    let f = IIRFunction::new("f", vec![], "void", vec![
        IIRInstr::new("const", Some("cond".into()), vec![Operand::Int(1)], "i32"),
        IIRInstr::new("jmp_if_true", None,
            vec![Operand::Var("cond".into()), Operand::Var("L_end".into())], "i32"),
        IIRInstr::new("const", Some("dead".into()), vec![Operand::Int(99)], "i32"),
        IIRInstr::new("label", None, vec![Operand::Var("L_end".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let words = lower(&module_with(f));
    // [0] addi t0, x0, 1     @ byte 0   (cond = 1, t0)
    // [1] bne  t0, x0, +8    @ byte 4   ← jmp_if_true
    // [2] addi t1, x0, 99    @ byte 8   (dead)
    // [3] jalr x0, x1, 0     @ byte 12  ← L_end, ret
    assert_eq!(words[1], encode_bne(T0, X0, 8),
        "bne t0, x0, +8 expected at byte 4; got 0x{:08x}", words[1]);
}

#[test]
fn jmp_if_false_emits_beq_with_resolved_offset() {
    use riscv_simulator::encoding::encode_beq;
    // Same shape as jmp_if_true but with the opposite branch.
    let f = IIRFunction::new("f", vec![], "void", vec![
        IIRInstr::new("const", Some("cond".into()), vec![Operand::Int(0)], "i32"),
        IIRInstr::new("jmp_if_false", None,
            vec![Operand::Var("cond".into()), Operand::Var("L_end".into())], "i32"),
        IIRInstr::new("const", Some("dead".into()), vec![Operand::Int(99)], "i32"),
        IIRInstr::new("label", None, vec![Operand::Var("L_end".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let words = lower(&module_with(f));
    assert_eq!(words[1], encode_beq(T0, X0, 8),
        "beq t0, x0, +8 expected at byte 4; got 0x{:08x}", words[1]);
}

#[test]
fn backward_jmp_emits_negative_offset() {
    use riscv_simulator::encoding::encode_jal;
    // Trivial infinite loop:
    //   label "top"
    //   jmp "top"
    //   ret_void   (unreachable)
    let f = IIRFunction::new("f", vec![], "void", vec![
        IIRInstr::new("label", None, vec![Operand::Var("top".into())], "void"),
        IIRInstr::new("jmp", None, vec![Operand::Var("top".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let words = lower(&module_with(f));
    // label "top" sits at byte 0, jal is at byte 0 (first emitted word).
    // Wait — label emits 0 words, so jal is at word index 0 (byte 0).
    // Target byte = 0, source byte = 0, offset = 0.
    // That's `jal x0, +0` — encoding 0x6F.
    assert_eq!(words[0], encode_jal(X0, 0),
        "jal x0, +0 expected for label-at-jmp; got 0x{:08x}", words[0]);
}

#[test]
fn undefined_label_is_rejected() {
    let f = IIRFunction::new("f", vec![], "void", vec![
        IIRInstr::new("jmp", None, vec![Operand::Var("nowhere".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let err = lower_iir_to_riscv(&module_with(f), &IIRRiscvConfig::default())
        .expect_err("undefined label should fail");
    match err {
        IIRRiscvError::UndefinedLabel { label, .. } => assert_eq!(label, "nowhere"),
        other => panic!("expected UndefinedLabel, got: {other:?}"),
    }
}

// ===========================================================================
// 16. A1++.5.5 — cross-function call (0-arg, void only in this slice)
// ===========================================================================
//
// Two-function modules exercise the module-level call-site resolver:
// pass 1 lowers each function and records call sites; pass 2 patches
// the placeholder `jal ra, 0` words with PC-relative offsets to the
// callee's start byte.
//
// Leaf functions emit no prologue/epilogue (preserves the existing
// single-word `ret` shape).  Functions that contain at least one call
// emit a 16-byte frame around the body: `addi sp, sp, -16; sw ra,
// 12(sp); … body …; lw ra, 12(sp); addi sp, sp, 16; ret`.

#[test]
fn cross_function_void_call_resolves_jal_offset() {
    use riscv_simulator::encoding::{encode_addi, encode_lw, encode_jal, encode_sw};
    // callee() void { ret_void }                          ; leaf, single ret word
    // caller() void { call callee(); ret_void }           ; has prologue/epilogue
    let callee = IIRFunction::new("callee", vec![], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let caller = IIRFunction::new("caller", vec![], "void", vec![
        IIRInstr::new("call", None, vec![Operand::Var("callee".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let module = IIRModule {
        name: "test".into(),
        functions: vec![callee, caller],
        entry_point: Some("caller".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let words = lower_iir_to_riscv(&module, &IIRRiscvConfig::default())
        .expect("lowering");

    // Layout (byte offsets):
    //   callee:
    //     [0] jalr x0, x1, 0                  ; byte 0
    //   caller:
    //     [1] addi sp, sp, -16                ; byte 4  ← prologue
    //     [2] sw   ra, 12(sp)                 ; byte 8
    //     [3] jal  ra, +offset (to callee=0)  ; byte 12  ← call site
    //     [4] lw   ra, 12(sp)                 ; byte 16  ← epilogue
    //     [5] addi sp, sp, 16                 ; byte 20
    //     [6] jalr x0, x1, 0                  ; byte 24  ← ret
    //
    // call site byte = 12, callee start byte = 0, offset = -12.
    const SP: u32 = 2;
    const RA: u32 = 1;
    assert_eq!(words[0], CANONICAL_RET, "callee should be a single ret word");
    assert_eq!(words[1], encode_addi(SP, SP, -16), "caller prologue: addi sp, sp, -16");
    assert_eq!(words[2], encode_sw(RA, SP, 12), "caller prologue: sw ra, 12(sp)");
    assert_eq!(words[3], encode_jal(RA, -12),
        "call site: jal ra, -12 (back to callee at byte 0); got 0x{:08x}", words[3]);
    assert_eq!(words[4], encode_lw(RA, SP, 12), "caller epilogue: lw ra, 12(sp)");
    assert_eq!(words[5], encode_addi(SP, SP, 16), "caller epilogue: addi sp, sp, 16");
    assert_eq!(words[6], CANONICAL_RET, "caller ret");
}

#[test]
fn leaf_function_still_omits_prologue() {
    // Sanity: a function with no `call` continues to emit just the body.
    let f = IIRFunction::new("leaf", vec![], "void", vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let words = lower(&module_with(f));
    assert_eq!(words, vec![CANONICAL_RET],
        "leaf function should be one-word; got {words:#x?}");
}

// ===========================================================================
// 17. A1++.5.5.5 — call args + non-void return
// ===========================================================================
//
// Two-phase move-through-temp avoids swap-clobbering when an arg's
// source register coincides with another arg's target a-register.
// Phase 1 reads all sources into disjoint scratch temps; Phase 2 writes
// from temps to a0..a{n-1}.

#[test]
fn call_with_one_const_arg_emits_arg_setup() {
    use riscv_simulator::encoding::{encode_addi, encode_jal};
    // square(x: i32) -> i32 { ret x }     ; trivial
    // f() void { v = const 5; call _ = square(v); ret_void }
    let callee = IIRFunction::new(
        "square",
        vec![("x".into(), "i32".into())],
        "i32",
        vec![IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i32")],
    );
    let caller = IIRFunction::new("f", vec![], "void", vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(5)], "i32"),
        IIRInstr::new("call", None,
            vec![Operand::Var("square".into()), Operand::Var("v".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let module = IIRModule {
        name: "test".into(),
        functions: vec![callee, caller],
        entry_point: Some("f".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let words = lower_iir_to_riscv(&module, &IIRRiscvConfig::default())
        .expect("lowering should succeed");
    // square emits: addi a0, a0, 0 ?  No — `ret x` where x is a0, so we
    //   skip the mv and emit just one ret word.
    // square layout:
    //   [0] jalr x0, x1, 0
    // caller layout:
    //   [1] addi sp, sp, -16     ; prologue
    //   [2] sw   ra, 12(sp)
    //   [3] addi t0, x0, 5       ; const v = 5 → t0
    //   [4] addi t1, t0, 0       ; phase 1: arg0 → scratch t1
    //   [5] addi a0, t1, 0       ; phase 2: t1 → a0
    //   [6] jal  ra, +offset     ; call square (will be patched)
    //   [7] lw   ra, 12(sp)
    //   [8] addi sp, sp, 16
    //   [9] jalr x0, x1, 0
    assert_eq!(words[0], CANONICAL_RET, "square should be one ret word");
    // The two-phase moves should both be present:
    assert_eq!(words[4], encode_addi(T1, T0, 0),
        "phase 1: addi t1, t0, 0 (scratch copy); got 0x{:08x}", words[4]);
    assert_eq!(words[5], encode_addi(A0, T1, 0),
        "phase 2: addi a0, t1, 0; got 0x{:08x}", words[5]);
    // The jal targets square at byte 0 from caller byte (6*4) - 1*4 prologue = 24.
    // Caller starts at byte 4 (after square's 1 word). Jal site is the 7th word
    // in `words` (index 6), i.e. byte 24. Target byte = 0. Offset = -24.
    assert_eq!(words[6], encode_jal(/*ra*/ 1, -24),
        "jal ra, -24 (call square at byte 0 from call site at byte 24); got 0x{:08x}", words[6]);
}

#[test]
fn call_with_non_void_return_binds_dest_from_a0() {
    use riscv_simulator::encoding::encode_addi;
    // g() -> i32 { ret_void  } — bogus body, just need a stub
    // f() void { call r = g(); ret_void }
    let callee = IIRFunction::new("g", vec![], "i32",
        vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    let caller = IIRFunction::new("f", vec![], "void", vec![
        IIRInstr::new("call", Some("r".into()),
            vec![Operand::Var("g".into())], "i32"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let module = IIRModule {
        name: "test".into(),
        functions: vec![callee, caller],
        entry_point: Some("f".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let words = lower_iir_to_riscv(&module, &IIRRiscvConfig::default())
        .expect("lowering should succeed");
    // Layout:
    //   [0] g: jalr x0, x1, 0
    //   [1] f: addi sp, sp, -16
    //   [2] f: sw   ra, 12(sp)
    //   [3] f: jal  ra, -12               ; call site
    //   [4] f: addi t0, a0, 0             ; bind r ← a0
    //   [5] f: lw   ra, 12(sp)
    //   [6] f: addi sp, sp, 16
    //   [7] f: jalr x0, x1, 0
    assert_eq!(words[4], encode_addi(T0, A0, 0),
        "expected addi t0, a0, 0 binding return value to r; got 0x{:08x}", words[4]);
}

#[test]
fn call_too_many_args_is_rejected_as_unsupported_shape() {
    // 9 args > 8 (a0..a7) → UnsupportedCallShape.
    let params: Vec<(String, String)> = (0..8)
        .map(|i| (format!("p{i}"), "i32".into())).collect();
    let callee = IIRFunction::new("g", params, "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    // Caller has 9 args — exceeds a0..a7.  Validator caps callee at 8
    // params, but the call instr itself may pass more srcs.
    let mut call_srcs = vec![Operand::Var("g".into())];
    for _ in 0..9 {
        // Each arg references a fresh local; we'll only define a few since
        // the lower will hit the > 8 check before lookup.
        call_srcs.push(Operand::Int(0));
    }
    let caller = IIRFunction::new("f", vec![], "void", vec![
        IIRInstr::new("call", None, call_srcs, "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let module = IIRModule {
        name: "test".into(),
        functions: vec![callee, caller],
        entry_point: None,
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let err = lower_iir_to_riscv(&module, &IIRRiscvConfig::default())
        .expect_err("9-arg call should be UnsupportedCallShape");
    match err {
        IIRRiscvError::UnsupportedCallShape { detail, .. } => {
            assert!(detail.contains("9 args") || detail.contains("up to"),
                "expected message naming arg-count restriction; got: {detail}");
        }
        other => panic!("expected UnsupportedCallShape, got: {other:?}"),
    }
}

#[test]
fn call_with_too_many_scratch_temps_needed_is_rejected() {
    // Pool has 7 temps. Caller pre-allocates 5 locals (t0..t4), leaving
    // 2 scratch slots; call with 3 args → OutOfRegisters.
    let callee_params: Vec<(String, String)> = (0..3)
        .map(|i| (format!("p{i}"), "i32".into())).collect();
    let callee = IIRFunction::new("g", callee_params, "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")]);
    let mut caller_body = Vec::new();
    for i in 0..5 {
        caller_body.push(IIRInstr::new("const", Some(format!("v{i}")),
            vec![Operand::Int(i as i64)], "i32"));
    }
    caller_body.push(IIRInstr::new("call", None, vec![
        Operand::Var("g".into()),
        Operand::Var("v0".into()), Operand::Var("v1".into()), Operand::Var("v2".into()),
    ], "void"));
    caller_body.push(IIRInstr::new("ret_void", None, vec![], "void"));
    let caller = IIRFunction::new("f", vec![], "void", caller_body);
    let module = IIRModule {
        name: "test".into(),
        functions: vec![callee, caller],
        entry_point: None,
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let err = lower_iir_to_riscv(&module, &IIRRiscvConfig::default())
        .expect_err("scratch-overflow should be OutOfRegisters");
    match err {
        IIRRiscvError::OutOfRegisters { .. } => {}
        other => panic!("expected OutOfRegisters, got: {other:?}"),
    }
}

#[test]
fn undefined_callee_is_rejected_at_module_level() {
    let f = IIRFunction::new("f", vec![], "void", vec![
        IIRInstr::new("call", None, vec![Operand::Var("ghost".into())], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let err = lower_iir_to_riscv(&module_with(f), &IIRRiscvConfig::default())
        .expect_err("undefined callee should fail");
    match err {
        IIRRiscvError::UndefinedCallee { callee, .. } => assert_eq!(callee, "ghost"),
        other => panic!("expected UndefinedCallee, got: {other:?}"),
    }
}
