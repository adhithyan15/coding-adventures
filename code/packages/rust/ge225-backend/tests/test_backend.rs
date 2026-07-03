// Tests for ge225-backend.
//
// These pin the SAME byte sequences `iir-to-ge225` v0.9.0 pinned,
// but build CIR programs (not IIR) — proving the migration
// preserves output byte-for-byte while moving the dispatch to the
// proper architectural layer.
//
// Every "trivial ROM" size from the iir-to-ge225 README is
// re-pinned here:
//   * `const + ret_<ty>` (entry function): 6 bytes
//   * `const + const + add + ret`: 21 bytes
//   * `const + const + cmp_lt + ret`: 33 bytes
//   * `const + neg + ret`: 15 bytes

use ge225_backend::{compile, BackendError, Ge225Backend};
use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn ctx<'a>(name: &'a str, params: &'a [(String, String)], ret_ty: &'a str) -> FunctionContext<'a> {
    FunctionContext {
        name,
        params,
        return_type: ret_ty,
    }
}

fn ci(op: &str, dest: Option<&str>, srcs: Vec<CIROperand>, ty: &str) -> CIRInstr {
    CIRInstr::new(op, dest, srcs, ty)
}

// ===========================================================================
// §1. Empty CIR + Backend trait basics
// ===========================================================================

#[test]
fn empty_cir_emits_halt() {
    let bytes = compile(&ctx("empty", &[], "void"), &[]).expect("lowering");
    assert_eq!(bytes, vec![0x00, 0x00, 0x00]);
}

#[test]
fn backend_name_is_ge225() {
    assert_eq!(Ge225Backend.name(), "ge225");
}

#[test]
fn backend_compile_returns_some_on_valid_input() {
    let cir = vec![ci("ret_void", None, vec![], "void")];
    assert_eq!(
        Ge225Backend.compile(&cir),
        Some(vec![0x00, 0x00, 0x00]),
        "Backend::compile must return Some for valid input"
    );
}

#[test]
fn backend_compile_returns_none_on_unsupported_op() {
    let cir = vec![ci("mul_i64", Some("z"), vec![], "i64")];
    assert_eq!(
        Ge225Backend.compile(&cir),
        None,
        "Backend::compile must return None for unsupported op"
    );
}

#[test]
#[should_panic(expected = "ge225 backend is emit-only")]
fn backend_run_panics_per_spec() {
    Ge225Backend.run(&[0x00, 0x00, 0x00], &[]);
}

// ===========================================================================
// §2. Trivial ROM regressions — byte-for-byte parity with iir-to-ge225 v0.9.0
// ===========================================================================

/// `const_i64 v=5; ret_i64 v` — the canonical 6-byte ROM.
#[test]
fn trivial_const_ret_is_six_bytes_for_i64() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(5)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("five", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x01, 0x00, 0x05, 0x00, 0x00, 0x00]);
}

/// All const_* type variants flow through the same LDA + HLT path
/// (GE-225 has only one accumulator width).
#[test]
fn trivial_const_ret_six_bytes_for_every_int_type() {
    for ty in &["i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64"] {
        let const_op = format!("const_{ty}");
        let ret_op = format!("ret_{ty}");
        let cir = vec![
            ci(&const_op, Some("v"), vec![CIROperand::Int(5)], ty),
            ci(&ret_op, None, vec![CIROperand::Var("v".into())], ty),
        ];
        let bytes = compile(&ctx("test", &[], ty), &cir).expect("lowering");
        assert_eq!(
            bytes.len(),
            6,
            "trivial ROM should be 6 bytes for {ty}; got {bytes:02x?}"
        );
    }
}

/// `const_bool b=true; ret_bool b` — Bool literals lower the same way.
#[test]
fn const_bool_true_lowers_to_lda_1() {
    let cir = vec![
        ci("const_bool", Some("b"), vec![CIROperand::Bool(true)], "bool"),
        ci("ret_bool", None, vec![CIROperand::Var("b".into())], "bool"),
    ];
    let bytes = compile(&ctx("btrue", &[], "bool"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0x01, 0x00, 0x01, 0x00, 0x00, 0x00]);
}

/// `const a=3; const b=4; add c, a, b; ret c` → 21 bytes.
#[test]
fn trivial_add_is_twentyone_bytes() {
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(3)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(4)], "i64"),
        ci(
            "add_i64",
            Some("c"),
            vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
            "i64",
        ),
        ci("ret_i64", None, vec![CIROperand::Var("c".into())], "i64"),
    ];
    let bytes = compile(&ctx("add", &[], "i64"), &cir).expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x03, // LDA 3
            0x02, 0x00, 0x00, // STA r0 (evict a)
            0x01, 0x00, 0x04, // LDA 4
            0x02, 0x00, 0x01, // STA r1 (evict b)
            0x03, 0x00, 0x00, // LD r0
            0x04, 0x00, 0x01, // ADD r1
            0x00, 0x00, 0x00, // HLT
        ]
    );
}

/// `const a=10; const b=3; sub c, a, b; ret c` → 21 bytes with SUB.
#[test]
fn trivial_sub_is_twentyone_bytes() {
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(10)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(3)], "i64"),
        ci(
            "sub_i64",
            Some("c"),
            vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
            "i64",
        ),
        ci("ret_i64", None, vec![CIROperand::Var("c".into())], "i64"),
    ];
    let bytes = compile(&ctx("sub", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes[15], 0x05, "SUB opcode at byte 15");
    assert_eq!(bytes.len(), 21);
}

/// `const v=5; neg w, v; ret w` → 15 bytes via the LDA 0 + SUB pattern.
#[test]
fn trivial_neg_is_fifteen_bytes() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(5)], "i64"),
        ci("neg_i64", Some("w"), vec![CIROperand::Var("v".into())], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("w".into())], "i64"),
    ];
    let bytes = compile(&ctx("neg", &[], "i64"), &cir).expect("lowering");
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
}

/// `const a=2; const b=5; cmp_lt c, a, b; ret c` — the 33-byte
/// canonical comparison ROM.
#[test]
fn trivial_cmp_lt_is_thirtythree_bytes() {
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(2)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(5)], "i64"),
        ci(
            "cmp_lt_i64",
            Some("c"),
            vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
            "bool",
        ),
        ci("ret_bool", None, vec![CIROperand::Var("c".into())], "bool"),
    ];
    let bytes = compile(&ctx("cmp_lt", &[], "bool"), &cir).expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x02, // LDA 2
            0x02, 0x00, 0x00, // STA r0 (evict a)
            0x01, 0x00, 0x05, // LDA 5
            0x02, 0x00, 0x01, // STA r1 (evict b)
            0x03, 0x00, 0x00, // LD r0
            0x05, 0x00, 0x01, // SUB r1
            0x0B, 0x00, 0x1B, // BMI 27 (true target)
            0x01, 0x00, 0x00, // LDA 0
            0x06, 0x00, 0x1E, // BR 30 (end target)
            0x01, 0x00, 0x01, // LDA 1
            0x00, 0x00, 0x00, // HLT
        ]
    );
}

/// `cmp_eq` swaps BMI for BZ at offset 18.
#[test]
fn cmp_eq_uses_bz_at_offset_18() {
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(3)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(3)], "i64"),
        ci(
            "cmp_eq_i64",
            Some("c"),
            vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
            "bool",
        ),
        ci("ret_bool", None, vec![CIROperand::Var("c".into())], "bool"),
    ];
    let bytes = compile(&ctx("cmp_eq", &[], "bool"), &cir).expect("lowering");
    assert_eq!(bytes[18], 0x08, "cmp_eq must use BZ (0x08) at offset 18");
}

/// `cmp_gt` swaps operands before lowering (cmp_gt a, b == cmp_lt b, a).
#[test]
fn cmp_gt_swaps_operands() {
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(2)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(5)], "i64"),
        ci(
            "cmp_gt_i64",
            Some("c"),
            vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
            "bool",
        ),
        ci("ret_void", None, vec![], "void"),
    ];
    let bytes = compile(&ctx("cmp_gt", &[], "void"), &cir).expect("lowering");
    // After swap: LD r1 (b) then SUB r0 (a) at offsets 12..18.
    assert_eq!(&bytes[12..15], &[0x03, 0x00, 0x01], "LD r1 (b after swap)");
    assert_eq!(&bytes[15..18], &[0x05, 0x00, 0x00], "SUB r0 (a after swap)");
    assert_eq!(bytes[18], 0x0B, "cmp_gt uses BMI after swap");
}

/// `cmp_le` is the double-test (BMI + BZ) variant.
#[test]
fn cmp_le_uses_double_test() {
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(2)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(5)], "i64"),
        ci(
            "cmp_le_i64",
            Some("c"),
            vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
            "bool",
        ),
        ci("ret_void", None, vec![], "void"),
    ];
    let bytes = compile(&ctx("cmp_le", &[], "void"), &cir).expect("lowering");
    assert_eq!(bytes[18], 0x0B, "cmp_le's first test is BMI");
    assert_eq!(bytes[21], 0x08, "cmp_le's second test is BZ");
}

// ===========================================================================
// §3. Control flow — label / jmp / jmp_if_* with backpatching
// ===========================================================================

#[test]
fn trivial_jmp_backpatches_forward_target() {
    let cir = vec![
        ci("jmp", None, vec![CIROperand::Var("x".into())], "void"),
        ci("label", None, vec![CIROperand::Var("x".into())], "void"),
        ci("ret_void", None, vec![], "void"),
    ];
    let bytes = compile(&ctx("jmp", &[], "void"), &cir).expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x06, 0x00, 0x03, // BR 3 (label x)
            0x00, 0x00, 0x00, // HLT
        ]
    );
}

#[test]
fn jmp_to_undefined_label_errors() {
    let cir = vec![
        ci("jmp", None, vec![CIROperand::Var("missing".into())], "void"),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("bad", &[], "void"), &cir)
        .expect_err("jmp to undefined label must error");
    matches!(err, BackendError::UndefinedLabel(s) if s == "missing");
}

#[test]
fn jmp_if_true_with_cond_in_acc_skips_ld_prefix() {
    let cir = vec![
        ci("const_bool", Some("c"), vec![CIROperand::Bool(true)], "bool"),
        ci(
            "jmp_if_true",
            None,
            vec![
                CIROperand::Var("c".into()),
                CIROperand::Var("skip".into()),
            ],
            "void",
        ),
        ci("label", None, vec![CIROperand::Var("skip".into())], "void"),
        ci("ret_void", None, vec![], "void"),
    ];
    let bytes = compile(&ctx("if", &[], "void"), &cir).expect("lowering");
    // After LDA 1, c is ACC owner — no LD before BNZ.
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x01, // LDA 1
            0x07, 0x00, 0x06, // BNZ 6 (skip)
            0x00, 0x00, 0x00, // HLT
        ]
    );
}

// ===========================================================================
// §4. call_builtin no-op
// ===========================================================================

#[test]
fn call_builtin_no_dest_emits_zero_bytes() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(5)], "i64"),
        ci(
            "call_builtin",
            None,
            vec![
                CIROperand::Var("print_i64".into()),
                CIROperand::Var("v".into()),
            ],
            "void",
        ),
        ci("ret_void", None, vec![], "void"),
    ];
    let bytes = compile(&ctx("print", &[], "void"), &cir).expect("lowering");
    // Same as `const v=5; ret_void` — call_builtin emits nothing.
    assert_eq!(bytes, vec![0x01, 0x00, 0x05, 0x00, 0x00, 0x00]);
}

#[test]
fn call_builtin_with_dest_emits_lda_zero() {
    let cir = vec![
        ci(
            "call_builtin",
            Some("x"),
            vec![CIROperand::Var("input_i64".into())],
            "i64",
        ),
        ci("ret_i64", None, vec![CIROperand::Var("x".into())], "i64"),
    ];
    let bytes = compile(&ctx("input", &[], "i64"), &cir).expect("lowering");
    assert_eq!(
        bytes,
        vec![
            0x01, 0x00, 0x00, // LDA 0 (placeholder return)
            0x00, 0x00, 0x00, // HLT
        ]
    );
}

#[test]
fn cross_function_call_returns_unsupported_until_phase_3() {
    let cir = vec![
        ci(
            "call",
            Some("x"),
            vec![CIROperand::Var("helper".into())],
            "i64",
        ),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("main", &[], "void"), &cir)
        .expect_err("cross-function call must be UnsupportedOp in Phase 2");
    matches!(err, BackendError::UnsupportedOp(s) if s.contains("call"));
}

// ===========================================================================
// §5. Error cases
// ===========================================================================

#[test]
fn const_out_of_range_errors() {
    let cir = vec![
        ci(
            "const_i32",
            Some("v"),
            vec![CIROperand::Int(70_000)],
            "i32",
        ),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("big", &[], "void"), &cir)
        .expect_err("70_000 overflows the 16-bit ceiling");
    matches!(err, BackendError::ImmediateOutOfRange(70_000));
}

#[test]
fn undefined_var_in_add_errors() {
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(1)], "i64"),
        ci(
            "add_i64",
            Some("c"),
            vec![
                CIROperand::Var("a".into()),
                CIROperand::Var("undefined".into()),
            ],
            "i64",
        ),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("bad_add", &[], "void"), &cir)
        .expect_err("undefined rhs must error");
    matches!(err, BackendError::UndefinedVariable(s) if s == "undefined");
}

#[test]
fn unsupported_op_returns_err() {
    // CIR `mul_*` isn't yet supported by GE-225.
    let cir = vec![ci(
        "mul_i64",
        Some("c"),
        vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
        "i64",
    )];
    let err = compile(&ctx("nope", &[], "void"), &cir).expect_err("mul_i64 must error");
    matches!(err, BackendError::UnsupportedOp(s) if s == "mul_i64");
}
