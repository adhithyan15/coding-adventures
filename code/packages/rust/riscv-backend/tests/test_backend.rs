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
fn executes_scalar_division_and_modulo() {
    let evaluate = |op: &str, ty: &str, left: i64, right: i64| {
        let cir = vec![
            ci(&format!("const_{ty}"), Some("left"), vec![CIROperand::Int(left)], ty),
            ci(&format!("const_{ty}"), Some("right"), vec![CIROperand::Int(right)], ty),
            ci(
                &format!("{op}_{ty}"),
                Some("result"),
                vec![CIROperand::Var("left".into()), CIROperand::Var("right".into())],
                ty,
            ),
            ci(&format!("ret_{ty}"), None, vec![CIROperand::Var("result".into())], ty),
        ];
        compile_and_run(&cir)
    };

    assert_eq!(evaluate("div", "i32", -20, 6), -3);
    assert_eq!(evaluate("mod", "i32", -20, 6), -2);
    assert_eq!(evaluate("div", "u32", 20, 6), 3);
    assert_eq!(evaluate("mod", "u32", 20, 6), 2);
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
fn executes_pair_aware_unsigned_64_bit_division_and_modulo() {
    let evaluate = |op: &str, dividend: i64, divisor: i64| {
        let cir = vec![
            ci("const_u64", Some("dividend"), vec![CIROperand::Int(dividend)], "u64"),
            ci("const_u64", Some("divisor"), vec![CIROperand::Int(divisor)], "u64"),
            ci(
                op,
                Some("result"),
                vec![
                    CIROperand::Var("dividend".into()),
                    CIROperand::Var("divisor".into()),
                ],
                "u64",
            ),
            ci("ret_u64", None, vec![CIROperand::Var("result".into())], "u64"),
        ];
        let bytes = compile(&ctx("wide_unsigned_divmod", &[], "u64"), &cir)
            .expect("wide unsigned div/mod lowering");
        run_binary(&bytes, &[]).expect("wide unsigned div/mod execution")
    };

    let quotient = evaluate("div_u64", 1_099_511_627_776, 3);
    assert_eq!(quotient.return_value as u32, 0x5555_5555);
    assert_eq!(quotient.return_value_high, 0x55);

    let cross_word_quotient = evaluate("div_u64", 1_311_768_467_463_790_320, 4_294_967_297);
    assert_eq!(cross_word_quotient.return_value as u32, 0x1234_5678);
    assert_eq!(cross_word_quotient.return_value_high, 0);

    for (dividend, divisor) in [(17, 5), (i64::MAX, 4_294_967_297), (-1, 7)] {
        let quotient = evaluate("div_u64", dividend, divisor);
        let remainder = evaluate("mod_u64", dividend, divisor);
        let quotient_bits = (u64::from(quotient.return_value_high) << 32)
            | u64::from(quotient.return_value as u32);
        let remainder_bits = (u64::from(remainder.return_value_high) << 32)
            | u64::from(remainder.return_value as u32);
        assert_eq!(quotient_bits, (dividend as u64) / (divisor as u64));
        assert_eq!(remainder_bits, (dividend as u64) % (divisor as u64));
    }

    let remainder = evaluate("mod_u64", 1_311_768_467_463_790_320, 4_294_967_297);
    assert_eq!(remainder.return_value as u32, 0x8888_8878);
    assert_eq!(remainder.return_value_high, 0);

    let zero_divisor_quotient = evaluate("div_u64", 1_099_511_627_776, 0);
    assert_eq!(zero_divisor_quotient.return_value as u32, u32::MAX);
    assert_eq!(zero_divisor_quotient.return_value_high, u32::MAX);

    let zero_divisor_remainder = evaluate("mod_u64", 1_099_511_627_776, 0);
    assert_eq!(zero_divisor_remainder.return_value, 0);
    assert_eq!(zero_divisor_remainder.return_value_high, 0x100);
}

#[test]
fn executes_pair_aware_signed_64_bit_division_and_modulo() {
    let evaluate = |op: &str, dividend: i64, divisor: i64| {
        let cir = vec![
            ci("const_i64", Some("dividend"), vec![CIROperand::Int(dividend)], "i64"),
            ci("const_i64", Some("divisor"), vec![CIROperand::Int(divisor)], "i64"),
            ci(
                op,
                Some("result"),
                vec![
                    CIROperand::Var("dividend".into()),
                    CIROperand::Var("divisor".into()),
                ],
                "i64",
            ),
            ci("ret_i64", None, vec![CIROperand::Var("result".into())], "i64"),
        ];
        let bytes = compile(&ctx("wide_signed_divmod", &[], "i64"), &cir)
            .expect("wide signed div/mod lowering");
        let result = run_binary(&bytes, &[]).expect("wide signed div/mod execution");
        (u64::from(result.return_value_high) << 32 | u64::from(result.return_value as u32)) as i64
    };

    for (dividend, divisor, quotient, remainder) in [
        (-20, 6, -3, -2),
        (-20, -6, 3, -2),
        (20, -6, -3, 2),
        (-1_099_511_627_776, 3, -366_503_875_925, -1),
        (i64::MIN, -1, i64::MIN, 0),
        (-20, 0, -1, -20),
    ] {
        assert_eq!(evaluate("div_i64", dividend, divisor), quotient);
        assert_eq!(evaluate("mod_i64", dividend, divisor), remainder);
    }
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
fn executes_chained_wide_shifts_without_exhausting_registers() {
    let cir = vec![
        ci("const_u64", Some("one"), vec![CIROperand::Int(1)], "u64"),
        ci("const_u64", Some("left_count"), vec![CIROperand::Int(6)], "u64"),
        ci(
            "shl_u64",
            Some("shifted"),
            vec![
                CIROperand::Var("one".into()),
                CIROperand::Var("left_count".into()),
            ],
            "u64",
        ),
        ci(
            "const_u64",
            Some("right_count"),
            vec![CIROperand::Int(1)],
            "u64",
        ),
        ci(
            "shr_u64",
            Some("result"),
            vec![
                CIROperand::Var("shifted".into()),
                CIROperand::Var("right_count".into()),
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

    let bytes = compile(&ctx("chained_wide_shifts", &[], "u64"), &cir)
        .expect("a dead wide shift result should be reusable by the next shift");
    let result = run_binary(&bytes, &[]).expect("wide shift execution");
    assert_eq!(result.return_value, 32);
    assert_eq!(result.return_value_high, 0);
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
fn spills_live_scalar_values_to_a_stack_frame() {
    let mut cir: Vec<CIRInstr> = (1..=7)
        .map(|value| {
            CIRInstr::new(
                "const_i32",
                Some(format!("v{value}")),
                vec![CIROperand::Int(value)],
                "i32",
            )
        })
        .collect();
    cir.extend([
        ci("add_i32", Some("ab"), vec![CIROperand::Var("v1".into()), CIROperand::Var("v2".into())], "i32"),
        ci("add_i32", Some("cd"), vec![CIROperand::Var("v3".into()), CIROperand::Var("v4".into())], "i32"),
        ci("add_i32", Some("ef"), vec![CIROperand::Var("v5".into()), CIROperand::Var("v6".into())], "i32"),
        ci("add_i32", Some("abcd"), vec![CIROperand::Var("ab".into()), CIROperand::Var("cd".into())], "i32"),
        ci("add_i32", Some("total"), vec![CIROperand::Var("abcd".into()), CIROperand::Var("ef".into())], "i32"),
        ci("add_i32", Some("answer"), vec![CIROperand::Var("total".into()), CIROperand::Var("v7".into())], "i32"),
        ci("ret_i32", None, vec![CIROperand::Var("answer".into())], "i32"),
    ]);
    assert_eq!(compile_and_run(&cir), 28);
}

#[test]
fn allocates_a_wide_pair_by_spilling_live_scalar_values() {
    let mut cir: Vec<CIRInstr> = (1..=6)
        .map(|value| ci("const_i32", Some(&format!("v{value}")), vec![CIROperand::Int(value)], "i32"))
        .collect();
    cir.extend([
        ci(
            "const_u64",
            Some("wide"),
            vec![CIROperand::Int(4_294_967_297)],
            "u64",
        ),
        ci(
            "add_i32",
            Some("answer"),
            vec![CIROperand::Var("v1".into()), CIROperand::Var("v2".into())],
            "i32",
        ),
        ci(
            "ret_i32",
            None,
            vec![CIROperand::Var("answer".into())],
            "i32",
        ),
    ]);
    assert_eq!(compile_and_run(&cir), 3);
}

#[test]
fn spills_live_wide_pairs_to_a_stack_frame() {
    let mut cir: Vec<CIRInstr> = (1..=4)
        .map(|value| {
            ci(
                "const_u64",
                Some(&format!("v{value}")),
                vec![CIROperand::Int(4_294_967_296 + value)],
                "u64",
            )
        })
        .collect();
    cir.extend([
        ci(
            "add_u64",
            Some("first"),
            vec![CIROperand::Var("v1".into()), CIROperand::Var("v4".into())],
            "u64",
        ),
        ci(
            "add_u64",
            Some("answer"),
            vec![CIROperand::Var("first".into()), CIROperand::Var("v2".into())],
            "u64",
        ),
        ci(
            "ret_u64",
            None,
            vec![CIROperand::Var("answer".into())],
            "u64",
        ),
    ]);
    assert_eq!(compile_and_run(&cir), 7);
}

#[test]
fn reloads_spilled_pairs_for_wide_arithmetic_and_bitwise_ops() {
    let mut cir: Vec<CIRInstr> = (1..=4)
        .map(|value| {
            ci(
                "const_u64",
                Some(&format!("v{value}")),
                vec![CIROperand::Int(4_294_967_296 + value)],
                "u64",
            )
        })
        .collect();
    cir.extend([
        ci(
            "sub_u64",
            Some("difference"),
            vec![CIROperand::Var("v1".into()), CIROperand::Var("v4".into())],
            "u64",
        ),
        ci(
            "mul_u64",
            Some("product"),
            vec![CIROperand::Var("v2".into()), CIROperand::Var("v4".into())],
            "u64",
        ),
        ci(
            "xor_u64",
            Some("bits"),
            vec![CIROperand::Var("v2".into()), CIROperand::Var("v1".into())],
            "u64",
        ),
        ci(
            "not_u64",
            Some("answer"),
            vec![CIROperand::Var("v1".into())],
            "u64",
        ),
        ci(
            "ret_u64",
            None,
            vec![CIROperand::Var("answer".into())],
            "u64",
        ),
    ]);
    assert_eq!(compile_and_run(&cir), -2);
}

#[test]
fn shifts_a_spilled_wide_pair_with_a_parameterized_count() {
    let params = vec![("count".to_owned(), "i32".to_owned())];
    let cir = vec![
        ci(
            "const_u64",
            Some("v1"),
            vec![CIROperand::Int(4_294_967_297)],
            "u64",
        ),
        ci(
            "const_u64",
            Some("v2"),
            vec![CIROperand::Int(4_294_967_298)],
            "u64",
        ),
        ci(
            "const_u64",
            Some("v3"),
            vec![CIROperand::Int(4_294_967_299)],
            "u64",
        ),
        ci(
            "shl_u64",
            Some("shifted"),
            vec![
                CIROperand::Var("v1".into()),
                CIROperand::Var("count".into()),
            ],
            "u64",
        ),
        ci(
            "add_u64",
            Some("sum"),
            vec![CIROperand::Var("shifted".into()), CIROperand::Var("v2".into())],
            "u64",
        ),
        ci(
            "add_u64",
            Some("answer"),
            vec![CIROperand::Var("sum".into()), CIROperand::Var("v3".into())],
            "u64",
        ),
        ci(
            "ret_u64",
            None,
            vec![CIROperand::Var("answer".into())],
            "u64",
        ),
    ];

    let bytes = compile(&ctx("spill_shift", &params, "u64"), &cir).expect("wide shift lowering");
    let result = run_binary(&bytes, &[Value::Int(1)]).expect("wide shift execution");
    assert_eq!(result.return_value, 7);
    assert_eq!(result.return_value_high, 4);
}

#[test]
fn divides_spilled_wide_pairs_for_signed_and_unsigned_values() {
    for (name, op, ty, dividend, divisor) in [
        (
            "unsigned_spill_division",
            "div_u64",
            "u64",
            4_294_967_304,
            4_294_967_298,
        ),
        (
            "signed_spill_division",
            "div_i64",
            "i64",
            -4_294_967_304,
            -4_294_967_298,
        ),
    ] {
        let const_op = format!("const_{ty}");
        let ret_op = format!("ret_{ty}");
        let cir = vec![
            ci(&const_op, Some("a"), vec![CIROperand::Int(dividend)], ty),
            ci(&const_op, Some("b"), vec![CIROperand::Int(divisor)], ty),
            ci(&const_op, Some("c"), vec![CIROperand::Int(4_294_967_299)], ty),
            ci(&const_op, Some("d"), vec![CIROperand::Int(4_294_967_300)], ty),
            ci(
                op,
                Some("quotient"),
                vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
                ty,
            ),
            ci(
                &format!("add_{ty}"),
                Some("sink"),
                vec![CIROperand::Var("c".into()), CIROperand::Var("d".into())],
                ty,
            ),
            ci(&ret_op, None, vec![CIROperand::Var("quotient".into())], ty),
        ];
        let bytes = compile(&ctx(name, &[], ty), &cir).expect("spilled wide division lowering");
        let result = run_binary(&bytes, &[]).expect("spilled wide division execution");
        assert_eq!(result.return_value, 1, "{name}");
        assert_eq!(result.return_value_high, 0, "{name}");
    }
}

#[test]
fn compares_two_spilled_wide_pairs_under_mixed_width_pressure() {
    let cir = vec![
        ci("const_u64", Some("v1"), vec![CIROperand::Int(4_294_967_297)], "u64"),
        ci("const_u64", Some("v2"), vec![CIROperand::Int(8_589_934_592)], "u64"),
        ci("const_u64", Some("v3"), vec![CIROperand::Int(4_294_967_299)], "u64"),
        ci("const_u64", Some("v4"), vec![CIROperand::Int(4_294_967_300)], "u64"),
        ci("const_u64", Some("v5"), vec![CIROperand::Int(4_294_967_301)], "u64"),
        ci(
            "cmp_lt_u64",
            Some("answer"),
            vec![CIROperand::Var("v1".into()), CIROperand::Var("v2".into())],
            "bool",
        ),
        ci(
            "add_u64",
            Some("sum34"),
            vec![CIROperand::Var("v3".into()), CIROperand::Var("v4".into())],
            "u64",
        ),
        ci(
            "add_u64",
            Some("sink"),
            vec![CIROperand::Var("sum34".into()), CIROperand::Var("v5".into())],
            "u64",
        ),
        ci(
            "ret_bool",
            None,
            vec![CIROperand::Var("answer".into())],
            "bool",
        ),
    ];
    assert_eq!(compile_and_run(&cir), 1);
}
