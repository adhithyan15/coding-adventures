//! Integration tests for `iir-to-riscv`.
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
use riscv_simulator::encoding::{encode_add, encode_addi, encode_jalr, encode_sub};

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
        IIRInstr::new("call_builtin", None, vec![], "void"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let errors = validate_for_riscv(&module_with(f));
    assert!(errors.iter().any(|e| e.contains("UnsupportedOp")),
        "expected UnsupportedOp for `call_builtin`; got: {errors:?}");
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
fn const_out_of_imm12_range_is_rejected() {
    let f = IIRFunction::new("f", vec![], "i32", vec![
        // 0x1000 = 4096 — outside [-2048, 2047].
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(4096)], "i32"),
        IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "i32"),
    ]);
    let err = lower_iir_to_riscv(&module_with(f), &IIRRiscvConfig::default())
        .expect_err("oversized const should fail");
    match err {
        IIRRiscvError::ImmediateOutOfRange { value, .. } => assert_eq!(value, 4096),
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
