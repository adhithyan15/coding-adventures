//! End-to-end byte and simulator tests for the RV32I CIR backend.

use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use riscv_backend::{compile, run_binary, BackendError, Riscv32Backend};
use vm_core::value::Value;

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

fn compile_and_run(cir: &[CIRInstr]) -> i32 {
    let binary = compile(&ctx("main", &[], "i32"), cir).expect("lowering");
    let run = run_binary(&binary, &[]).expect("simulator execution");
    assert!(run.halted);
    assert!(run.steps > 0);
    run.return_value
}

#[test]
fn empty_cir_emits_canonical_ret() {
    let bytes = compile(&ctx("empty", &[], "void"), &[]).expect("lowering");
    assert_eq!(bytes, vec![0x67, 0x80, 0x00, 0x00]);
}

#[test]
fn backend_name_is_riscv32() {
    assert_eq!(Riscv32Backend.name(), "riscv32");
}

#[test]
fn backend_run_executes_the_binary_in_the_simulator() {
    let binary = compile(
        &ctx("answer", &[], "i32"),
        &[
            ci(
                "const_i32",
                Some("answer"),
                vec![CIROperand::Int(42)],
                "i32",
            ),
            ci(
                "ret_i32",
                None,
                vec![CIROperand::Var("answer".into())],
                "i32",
            ),
        ],
    )
    .unwrap();
    assert_eq!(Riscv32Backend.run(&binary, &[]), Value::Int(42));
}

#[test]
fn canonical_twig_42_bytes_are_preserved_and_execute() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("lowering");
    assert_eq!(
        bytes,
        vec![0x93, 0x02, 0xA0, 0x02, 0x13, 0x85, 0x02, 0x00, 0x67, 0x80, 0x00, 0x00,]
    );
    assert_eq!(run_binary(&bytes, &[]).unwrap().return_value, 42);
}

#[test]
fn executes_large_32_bit_constants() {
    let cir = vec![
        ci(
            "const_i32",
            Some("value"),
            vec![CIROperand::Int(1_000_000)],
            "i32",
        ),
        ci(
            "ret_i32",
            None,
            vec![CIROperand::Var("value".into())],
            "i32",
        ),
    ];
    assert_eq!(compile_and_run(&cir), 1_000_000);
}

#[test]
fn executes_parameterized_cir_functions_via_the_rv32i_abi() {
    let params = vec![
        ("left".to_owned(), "i32".to_owned()),
        ("right".to_owned(), "i32".to_owned()),
    ];
    let cir = vec![
        ci(
            "add_i32",
            Some("sum"),
            vec![
                CIROperand::Var("left".into()),
                CIROperand::Var("right".into()),
            ],
            "i32",
        ),
        ci("ret_i32", None, vec![CIROperand::Var("sum".into())], "i32"),
    ];
    let binary = compile(&ctx("sum", &params, "i32"), &cir).expect("lowering");
    let result =
        run_binary(&binary, &[Value::Int(19), Value::Int(23)]).expect("simulator execution");
    assert_eq!(result.return_value, 42);
}

#[test]
fn masks_u16_results_without_sign_extending_the_mask() {
    let cir = vec![
        ci("const_u16", Some("a"), vec![CIROperand::Int(65_535)], "u16"),
        ci("const_u16", Some("b"), vec![CIROperand::Int(1)], "u16"),
        ci(
            "add_u16",
            Some("sum"),
            vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
            "u16",
        ),
        ci("ret_u16", None, vec![CIROperand::Var("sum".into())], "u16"),
    ];
    assert_eq!(compile_and_run(&cir), 0);
}

#[test]
fn executes_integer_arithmetic_and_bitwise_ops() {
    let cir = vec![
        ci("const_i32", Some("a"), vec![CIROperand::Int(40)], "i32"),
        ci("const_i32", Some("b"), vec![CIROperand::Int(2)], "i32"),
        ci(
            "add_i32",
            Some("sum"),
            vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
            "i32",
        ),
        ci("const_i32", Some("mask"), vec![CIROperand::Int(15)], "i32"),
        ci(
            "xor_i32",
            Some("mixed"),
            vec![
                CIROperand::Var("sum".into()),
                CIROperand::Var("mask".into()),
            ],
            "i32",
        ),
        ci(
            "ret_i32",
            None,
            vec![CIROperand::Var("mixed".into())],
            "i32",
        ),
    ];
    assert_eq!(compile_and_run(&cir), 37);
}

#[test]
fn executes_signed_and_unsigned_comparisons() {
    let signed = vec![
        ci("const_i32", Some("a"), vec![CIROperand::Int(-3)], "i32"),
        ci("const_i32", Some("b"), vec![CIROperand::Int(2)], "i32"),
        ci(
            "cmp_lt_i32",
            Some("result"),
            vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
            "bool",
        ),
        ci(
            "ret_bool",
            None,
            vec![CIROperand::Var("result".into())],
            "bool",
        ),
    ];
    assert_eq!(compile_and_run(&signed), 1);

    let unsigned = vec![
        ci("const_u32", Some("a"), vec![CIROperand::Int(-1)], "u32"),
        ci("const_u32", Some("b"), vec![CIROperand::Int(1)], "u32"),
        ci(
            "cmp_gt_u32",
            Some("result"),
            vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
            "bool",
        ),
        ci(
            "ret_bool",
            None,
            vec![CIROperand::Var("result".into())],
            "bool",
        ),
    ];
    assert_eq!(compile_and_run(&unsigned), 1);
}

#[test]
fn preserves_narrow_unsigned_wrap_semantics() {
    let cir = vec![
        ci("const_u8", Some("a"), vec![CIROperand::Int(250)], "u8"),
        ci("const_u8", Some("b"), vec![CIROperand::Int(10)], "u8"),
        ci(
            "add_u8",
            Some("sum"),
            vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
            "u8",
        ),
        ci("ret_u8", None, vec![CIROperand::Var("sum".into())], "u8"),
    ];
    assert_eq!(compile_and_run(&cir), 4);
}

#[test]
fn executes_wide_integer_addition_and_subtraction_with_register_pairs() {
    let add = vec![
        ci(
            "const_i64",
            Some("base"),
            vec![CIROperand::Int(4_294_967_296)],
            "i64",
        ),
        ci(
            "const_i64",
            Some("offset"),
            vec![CIROperand::Int(42)],
            "i64",
        ),
        ci(
            "add_i64",
            Some("sum"),
            vec![
                CIROperand::Var("base".into()),
                CIROperand::Var("offset".into()),
            ],
            "i64",
        ),
        ci("ret_i64", None, vec![CIROperand::Var("sum".into())], "i64"),
    ];
    let add_bytes = compile(&ctx("wide_add", &[], "i64"), &add).expect("wide add lowering");
    let add_result = run_binary(&add_bytes, &[]).expect("wide add execution");
    assert_eq!(add_result.return_value as u32, 42);
    assert_eq!(add_result.return_value_high, 1);

    let subtract = vec![
        ci(
            "const_i64",
            Some("base"),
            vec![CIROperand::Int(4_294_967_296)],
            "i64",
        ),
        ci("const_i64", Some("one"), vec![CIROperand::Int(1)], "i64"),
        ci(
            "sub_i64",
            Some("difference"),
            vec![
                CIROperand::Var("base".into()),
                CIROperand::Var("one".into()),
            ],
            "i64",
        ),
        ci(
            "ret_i64",
            None,
            vec![CIROperand::Var("difference".into())],
            "i64",
        ),
    ];
    let subtract_bytes =
        compile(&ctx("wide_sub", &[], "i64"), &subtract).expect("wide sub lowering");
    let subtract_result = run_binary(&subtract_bytes, &[]).expect("wide sub execution");
    assert_eq!(subtract_result.return_value as u32, u32::MAX);
    assert_eq!(subtract_result.return_value_high, 0);

    let signed = vec![
        ci(
            "const_i64",
            Some("minus_one"),
            vec![CIROperand::Int(-1)],
            "i64",
        ),
        ci("const_i64", Some("one"), vec![CIROperand::Int(1)], "i64"),
        ci(
            "add_i64",
            Some("zero"),
            vec![
                CIROperand::Var("minus_one".into()),
                CIROperand::Var("one".into()),
            ],
            "i64",
        ),
        ci("ret_i64", None, vec![CIROperand::Var("zero".into())], "i64"),
    ];
    let signed_bytes =
        compile(&ctx("signed_wide_add", &[], "i64"), &signed).expect("signed wide add lowering");
    let signed_result = run_binary(&signed_bytes, &[]).expect("signed wide add execution");
    assert_eq!(signed_result.return_value, 0);
    assert_eq!(signed_result.return_value_high, 0);
}

#[test]
fn executes_unsigned_64_bit_wraparound() {
    let cir = vec![
        ci("const_u64", Some("max"), vec![CIROperand::Int(-1)], "u64"),
        ci("const_u64", Some("one"), vec![CIROperand::Int(1)], "u64"),
        ci(
            "add_u64",
            Some("wrapped"),
            vec![CIROperand::Var("max".into()), CIROperand::Var("one".into())],
            "u64",
        ),
        ci(
            "ret_u64",
            None,
            vec![CIROperand::Var("wrapped".into())],
            "u64",
        ),
    ];
    let bytes = compile(&ctx("wide_wrap", &[], "u64"), &cir).expect("wide add lowering");
    let result = run_binary(&bytes, &[]).expect("wide add execution");
    assert_eq!(result.return_value, 0);
    assert_eq!(result.return_value_high, 0);
}

#[test]
fn executes_pair_aware_64_bit_multiplication() {
    let unsigned = vec![
        ci("const_u64", Some("left"), vec![CIROperand::Int(4_294_967_297)], "u64"),
        ci("const_u64", Some("right"), vec![CIROperand::Int(4_294_967_297)], "u64"),
        ci(
            "mul_u64",
            Some("product"),
            vec![CIROperand::Var("left".into()), CIROperand::Var("right".into())],
            "u64",
        ),
        ci("ret_u64", None, vec![CIROperand::Var("product".into())], "u64"),
    ];
    let bytes = compile(&ctx("wide_unsigned_mul", &[], "u64"), &unsigned)
        .expect("wide unsigned multiplication lowering");
    let result = run_binary(&bytes, &[]).expect("wide unsigned multiplication execution");
    assert_eq!(result.return_value as u32, 1);
    assert_eq!(result.return_value_high, 2);

    let signed = vec![
        ci("const_i64", Some("left"), vec![CIROperand::Int(-2)], "i64"),
        ci("const_i64", Some("right"), vec![CIROperand::Int(2)], "i64"),
        ci(
            "mul_i64",
            Some("product"),
            vec![CIROperand::Var("left".into()), CIROperand::Var("right".into())],
            "i64",
        ),
        ci("ret_i64", None, vec![CIROperand::Var("product".into())], "i64"),
    ];
    let bytes = compile(&ctx("wide_signed_mul", &[], "i64"), &signed)
        .expect("wide signed multiplication lowering");
    let result = run_binary(&bytes, &[]).expect("wide signed multiplication execution");
    assert_eq!(result.return_value, -4);
    assert_eq!(result.return_value_high, u32::MAX);
}

#[test]
fn executes_pair_aware_signed_and_unsigned_64_bit_comparisons() {
    let signed = vec![
        ci(
            "const_i64",
            Some("low"),
            vec![CIROperand::Int(-4_294_967_296)],
            "i64",
        ),
        ci("const_i64", Some("high"), vec![CIROperand::Int(-1)], "i64"),
        ci(
            "cmp_lt_i64",
            Some("result"),
            vec![
                CIROperand::Var("low".into()),
                CIROperand::Var("high".into()),
            ],
            "bool",
        ),
        ci(
            "ret_bool",
            None,
            vec![CIROperand::Var("result".into())],
            "bool",
        ),
    ];
    assert_eq!(compile_and_run(&signed), 1);

    let unsigned = vec![
        ci(
            "const_u64",
            Some("large"),
            vec![CIROperand::Int(4_294_967_296)],
            "u64",
        ),
        ci("const_u64", Some("small"), vec![CIROperand::Int(1)], "u64"),
        ci(
            "cmp_gt_u64",
            Some("result"),
            vec![
                CIROperand::Var("large".into()),
                CIROperand::Var("small".into()),
            ],
            "bool",
        ),
        ci(
            "ret_bool",
            None,
            vec![CIROperand::Var("result".into())],
            "bool",
        ),
    ];
    assert_eq!(compile_and_run(&unsigned), 1);

    let equality = vec![
        ci(
            "const_u64",
            Some("left"),
            vec![CIROperand::Int(4_294_967_297)],
            "u64",
        ),
        ci(
            "const_u64",
            Some("right"),
            vec![CIROperand::Int(4_294_967_297)],
            "u64",
        ),
        ci(
            "cmp_eq_u64",
            Some("result"),
            vec![
                CIROperand::Var("left".into()),
                CIROperand::Var("right".into()),
            ],
            "bool",
        ),
        ci(
            "ret_bool",
            None,
            vec![CIROperand::Var("result".into())],
            "bool",
        ),
    ];
    assert_eq!(compile_and_run(&equality), 1);

    for (relation, left_value, right_value) in [
        ("cmp_ne_u64", 4_294_967_296, 4_294_967_297),
        ("cmp_le_u64", 4_294_967_296, 4_294_967_297),
        ("cmp_ge_u64", 4_294_967_297, 4_294_967_296),
    ] {
        let cir = vec![
            ci(
                "const_u64",
                Some("left"),
                vec![CIROperand::Int(left_value)],
                "u64",
            ),
            ci(
                "const_u64",
                Some("right"),
                vec![CIROperand::Int(right_value)],
                "u64",
            ),
            ci(
                relation,
                Some("result"),
                vec![
                    CIROperand::Var("left".into()),
                    CIROperand::Var("right".into()),
                ],
                "bool",
            ),
            ci(
                "ret_bool",
                None,
                vec![CIROperand::Var("result".into())],
                "bool",
            ),
        ];
        assert_eq!(compile_and_run(&cir), 1, "{relation} must compare pairs");
    }
}

#[test]
fn executes_pair_aware_64_bit_bitwise_operations() {
    for (op, expected_low, expected_high) in
        [("and_u64", 15, 1), ("or_u64", 255, 1), ("xor_u64", 240, 0)]
    {
        let cir = vec![
            ci(
                "const_u64",
                Some("left"),
                vec![CIROperand::Int(4_294_967_551)],
                "u64",
            ),
            ci(
                "const_u64",
                Some("right"),
                vec![CIROperand::Int(4_294_967_311)],
                "u64",
            ),
            ci(
                op,
                Some("result"),
                vec![
                    CIROperand::Var("left".into()),
                    CIROperand::Var("right".into()),
                ],
                "u64",
            ),
            ci(
                "ret_u64",
                None,
                vec![CIROperand::Var("result".into())],
                "u64",
            ),
        ];
        let bytes = compile(&ctx("wide_bitwise", &[], "u64"), &cir).expect("wide bitwise lowering");
        let result = run_binary(&bytes, &[]).expect("wide bitwise execution");
        assert_eq!(result.return_value as u32, expected_low, "{op} low word");
        assert_eq!(result.return_value_high, expected_high, "{op} high word");
    }

    let complement = vec![
        ci("const_u64", Some("zero"), vec![CIROperand::Int(0)], "u64"),
        ci(
            "not_u64",
            Some("all_ones"),
            vec![CIROperand::Var("zero".into())],
            "u64",
        ),
        ci(
            "ret_u64",
            None,
            vec![CIROperand::Var("all_ones".into())],
            "u64",
        ),
    ];
    let bytes = compile(&ctx("wide_not", &[], "u64"), &complement).expect("wide not lowering");
    let result = run_binary(&bytes, &[]).expect("wide not execution");
    assert_eq!(result.return_value as u32, u32::MAX);
    assert_eq!(result.return_value_high, u32::MAX);
}

#[test]
fn executes_pair_aware_64_bit_shifts_across_word_boundaries() {
    let left = vec![
        ci("const_u64", Some("one"), vec![CIROperand::Int(1)], "u64"),
        ci("const_u64", Some("count"), vec![CIROperand::Int(40)], "u64"),
        ci(
            "shl_u64",
            Some("result"),
            vec![
                CIROperand::Var("one".into()),
                CIROperand::Var("count".into()),
            ],
            "u64",
        ),
        ci(
            "ret_u64",
            None,
            vec![CIROperand::Var("result".into())],
            "u64",
        ),
    ];
    let left_bytes =
        compile(&ctx("wide_left_shift", &[], "u64"), &left).expect("wide shift lowering");
    let left_result = run_binary(&left_bytes, &[]).expect("wide shift execution");
    assert_eq!(left_result.return_value, 0);
    assert_eq!(left_result.return_value_high, 256);

    let logical_right = vec![
        ci(
            "const_u64",
            Some("value"),
            vec![CIROperand::Int(1_099_511_627_776)],
            "u64",
        ),
        ci("const_u64", Some("count"), vec![CIROperand::Int(32)], "u64"),
        ci(
            "shr_u64",
            Some("result"),
            vec![
                CIROperand::Var("value".into()),
                CIROperand::Var("count".into()),
            ],
            "u64",
        ),
        ci(
            "ret_u64",
            None,
            vec![CIROperand::Var("result".into())],
            "u64",
        ),
    ];
    let right_bytes = compile(&ctx("wide_logical_right_shift", &[], "u64"), &logical_right)
        .expect("wide shift lowering");
    let right_result = run_binary(&right_bytes, &[]).expect("wide shift execution");
    assert_eq!(right_result.return_value, 256);
    assert_eq!(right_result.return_value_high, 0);

    let arithmetic_right = vec![
        ci(
            "const_i64",
            Some("minus_one"),
            vec![CIROperand::Int(-1)],
            "i64",
        ),
        ci("const_i64", Some("count"), vec![CIROperand::Int(40)], "i64"),
        ci(
            "shr_i64",
            Some("result"),
            vec![
                CIROperand::Var("minus_one".into()),
                CIROperand::Var("count".into()),
            ],
            "i64",
        ),
        ci(
            "ret_i64",
            None,
            vec![CIROperand::Var("result".into())],
            "i64",
        ),
    ];
    let arithmetic_bytes = compile(
        &ctx("wide_arithmetic_right_shift", &[], "i64"),
        &arithmetic_right,
    )
    .expect("wide shift lowering");
    let arithmetic_result = run_binary(&arithmetic_bytes, &[]).expect("wide shift execution");
    assert_eq!(arithmetic_result.return_value as u32, u32::MAX);
    assert_eq!(arithmetic_result.return_value_high, u32::MAX);

    let oversized = vec![
        ci("const_u64", Some("one"), vec![CIROperand::Int(1)], "u64"),
        ci("const_u64", Some("count"), vec![CIROperand::Int(64)], "u64"),
        ci(
            "shl_u64",
            Some("result"),
            vec![
                CIROperand::Var("one".into()),
                CIROperand::Var("count".into()),
            ],
            "u64",
        ),
        ci(
            "ret_u64",
            None,
            vec![CIROperand::Var("result".into())],
            "u64",
        ),
    ];
    let oversized_bytes =
        compile(&ctx("wide_oversized_shift", &[], "u64"), &oversized).expect("wide shift lowering");
    let oversized_result = run_binary(&oversized_bytes, &[]).expect("wide shift execution");
    assert_eq!(oversized_result.return_value, 0);
    assert_eq!(oversized_result.return_value_high, 0);
}

#[test]
fn rejects_calls_until_the_linker_and_frame_abi_exist() {
    let cir = vec![ci(
        "call",
        Some("result"),
        vec![CIROperand::Var("helper".into())],
        "i32",
    )];
    let err = compile(&ctx("main", &[], "i32"), &cir).expect_err("calls are a later backend slice");
    assert!(matches!(err, BackendError::UnsupportedOp(op) if op == "call"));
}

// ---------------------------------------------------------------------------
// Floating point: a refusal with a reason, never bytes.
//
// RV32I is the RISC-V *base integer* ISA — 32 integer registers, no `f0`..`f31`
// bank, no `fadd.d`.  Single/double precision live in the optional `F`/`D`
// extensions (RV32F/RV32D).  So a float here is not "a lowering nobody wrote
// yet"; it is a value the target cannot hold, and the two have opposite fixes
// (implement an op vs. retarget or soft-float).  `UnsupportedFloat` says which
// one the reader is looking at, and names the site so a multi-function module
// (Dartmouth BASIC drags in its whole `__basic_print_*` runtime) points at the
// exact instruction.  Truncating the double to an integer to "make it work"
// would be a silent wrong answer — never acceptable.
// ---------------------------------------------------------------------------

#[test]
fn rejects_a_float_constant_with_a_reason_not_bytes() {
    let cir = vec![ci(
        "const_f64",
        Some("x"),
        vec![CIROperand::Float(42.0)],
        "f64",
    )];
    let err = compile(&ctx("main", &[], "i32"), &cir)
        .expect_err("RV32I has no floating-point registers");
    assert_eq!(
        err,
        BackendError::UnsupportedFloat {
            site: "op \"const_f64\"".to_owned(),
            ty: "f64".to_owned(),
        }
    );
    let msg = err.to_string();
    assert!(msg.contains("no floating-point registers"), "got: {msg}");
    assert!(msg.contains("RV32F/RV32D"), "got: {msg}");
}

#[test]
fn rejects_float_arithmetic_comparison_return_and_parameters() {
    // Arithmetic: `add_f64` never reaches an `add` encoding.
    let arithmetic = vec![ci(
        "add_f64",
        Some("sum"),
        vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
        "f64",
    )];
    assert_eq!(
        compile(&ctx("main", &[], "i32"), &arithmetic).expect_err("no f registers"),
        BackendError::UnsupportedFloat {
            site: "op \"add_f64\"".to_owned(),
            ty: "f64".to_owned(),
        }
    );

    // Comparison: `cmp_lt_f64` is not an integer `slt` in disguise.
    let comparison = vec![ci(
        "cmp_lt_f64",
        Some("less"),
        vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
        "bool",
    )];
    assert_eq!(
        compile(&ctx("main", &[], "bool"), &comparison).expect_err("no f registers"),
        BackendError::UnsupportedFloat {
            site: "op \"cmp_lt_f64\"".to_owned(),
            ty: "f64".to_owned(),
        }
    );

    // Return: an f64 cannot ride home in `a0`.
    let returned = vec![ci(
        "ret_f32",
        None,
        vec![CIROperand::Var("x".into())],
        "f32",
    )];
    assert_eq!(
        compile(&ctx("main", &[], "f32"), &returned).expect_err("no f registers"),
        BackendError::UnsupportedFloat {
            site: "op \"ret_f32\"".to_owned(),
            ty: "f32".to_owned(),
        }
    );

    // Parameter: the refusal happens before a single word is emitted, and it
    // names the parameter (this is the `__basic_print_real(x : f64)` shape).
    let params = vec![("mag".to_owned(), "f64".to_owned())];
    assert_eq!(
        compile(&ctx("__basic_print_real", &params, "i64"), &[])
            .expect_err("an f64 argument has no RV32I register to arrive in"),
        BackendError::UnsupportedFloat {
            site: "parameter \"mag\"".to_owned(),
            ty: "f64".to_owned(),
        }
    );
}

#[test]
fn non_float_unsupported_types_keep_the_generic_refusal() {
    // The float refusal must not swallow the ordinary "no lowering yet" case:
    // a `str` is unsupported for a different reason and keeps its own error.
    let cir = vec![ci(
        "const_str",
        Some("s"),
        vec![CIROperand::Var("hello".into())],
        "str",
    )];
    assert_eq!(
        compile(&ctx("main", &[], "i32"), &cir).expect_err("no string lowering"),
        BackendError::UnsupportedType("str".to_owned())
    );
}

#[test]
fn executes_conditional_and_unconditional_control_flow() {
    let conditional = vec![
        ci(
            "const_bool",
            Some("condition"),
            vec![CIROperand::Bool(true)],
            "bool",
        ),
        ci(
            "jmp_if_false",
            None,
            vec![
                CIROperand::Var("condition".into()),
                CIROperand::Var("otherwise".into()),
            ],
            "void",
        ),
        ci(
            "const_i32",
            Some("answer"),
            vec![CIROperand::Int(42)],
            "i32",
        ),
        ci(
            "ret_i32",
            None,
            vec![CIROperand::Var("answer".into())],
            "i32",
        ),
        ci(
            "label",
            None,
            vec![CIROperand::Var("otherwise".into())],
            "void",
        ),
        ci("const_i32", Some("wrong"), vec![CIROperand::Int(0)], "i32"),
        ci(
            "ret_i32",
            None,
            vec![CIROperand::Var("wrong".into())],
            "i32",
        ),
    ];
    assert_eq!(compile_and_run(&conditional), 42);

    let jump = vec![
        ci("jmp", None, vec![CIROperand::Var("end".into())], "void"),
        ci("const_i32", Some("dead"), vec![CIROperand::Int(0)], "i32"),
        ci("label", None, vec![CIROperand::Var("end".into())], "void"),
        ci(
            "const_i32",
            Some("answer"),
            vec![CIROperand::Int(42)],
            "i32",
        ),
        ci(
            "ret_i32",
            None,
            vec![CIROperand::Var("answer".into())],
            "i32",
        ),
    ];
    assert_eq!(compile_and_run(&jump), 42);
}

#[test]
fn rejects_control_flow_to_an_undefined_label() {
    let cir = vec![ci(
        "jmp",
        None,
        vec![CIROperand::Var("missing".into())],
        "void",
    )];
    let err = compile(&ctx("bad_jump", &[], "void"), &cir)
        .expect_err("an unresolved label must be reported");
    assert_eq!(err, BackendError::UndefinedLabel("missing".to_owned()));
}

#[test]
fn executes_jmp_if_true_when_its_branch_is_taken() {
    let cir = vec![
        ci(
            "const_bool",
            Some("condition"),
            vec![CIROperand::Bool(true)],
            "bool",
        ),
        ci(
            "jmp_if_true",
            None,
            vec![
                CIROperand::Var("condition".into()),
                CIROperand::Var("taken".into()),
            ],
            "void",
        ),
        ci("const_i32", Some("missed"), vec![CIROperand::Int(0)], "i32"),
        ci(
            "ret_i32",
            None,
            vec![CIROperand::Var("missed".into())],
            "i32",
        ),
        ci("label", None, vec![CIROperand::Var("taken".into())], "void"),
        ci(
            "const_i32",
            Some("answer"),
            vec![CIROperand::Int(42)],
            "i32",
        ),
        ci(
            "ret_i32",
            None,
            vec![CIROperand::Var("answer".into())],
            "i32",
        ),
    ];
    assert_eq!(compile_and_run(&cir), 42);
}

#[test]
fn rejects_more_live_values_than_the_starter_allocator_can_hold() {
    let cir: Vec<CIRInstr> = (0..7)
        .map(|index| {
            CIRInstr::new(
                "const_i32",
                Some(format!("v{index}")),
                vec![CIROperand::Int(index)],
                "i32",
            )
        })
        .collect();
    let err = compile(&ctx("many", &[], "void"), &cir).expect_err("six value registers only");
    assert_eq!(err, BackendError::OutOfRegisters);
}
