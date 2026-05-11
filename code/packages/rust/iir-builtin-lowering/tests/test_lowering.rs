//! Comprehensive tests for `iir-builtin-lowering`.
//!
//! # Organisation
//!
//! Tests are grouped by concern:
//!
//! 1–18:   All 18 numeric builtins — each lowers to the correct opcode.
//! 19–22:  Binary op invariants: dest preserved, srcs stripped, type_hint preserved.
//! 23–24:  Unary ops (neg, not): 1 arg, dest preserved.
//! 25–26:  Unknown builtins are left unchanged as `call_builtin`.
//! 27–28:  Non-`call_builtin` instructions are left unchanged.
//! 29–30:  `may_alloc` is always cleared to `false` after lowering.
//! 31–32:  Error — WrongArity (binary op called with wrong arg count).
//! 33–34:  Error — UntypedBuiltin (type_hint = "any" on a known builtin).
//! 35–37:  Modules with multiple functions — all functions lowered.
//! 38:     Empty module (no functions) — no panic.
//! 39–40:  Mixed call_builtin and non-call_builtin in same function.
//! 41–42:  `lower_builtins_cloned` returns copy, original unchanged.
//! 43:     `lower_builtins_checked` returns Ok on success.
//! 44:     `lower_builtins_checked` returns Err on failure.
//! 45:     Profiles fields (observation_count, observed_type) preserved after lowering.
//! 46:     ic_slot preserved after lowering.
//! 47–48:  Functions with empty instruction lists — no panic.
//! 49–50:  Multiple errors accumulate rather than stopping at first.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_builtin_lowering::{lower_builtins, lower_builtins_cloned, lower_builtins_checked, BuiltinLoweringError};

// ===========================================================================
// Test helpers
// ===========================================================================

/// Build an `IIRModule` with a single zero-parameter function named `"main"`.
fn make_module(instrs: Vec<IIRInstr>) -> IIRModule {
    let fn_ = IIRFunction::new("main", vec![], "i64", instrs);
    IIRModule {
        name: "test".into(),
        functions: vec![fn_],
        entry_point: Some("main".into()),
        language: "twig".into(),
    }
}

/// Build a `call_builtin` with 2 argument operands and a given type_hint.
fn binary_call(name: &str, type_hint: &str) -> IIRInstr {
    IIRInstr::new(
        "call_builtin",
        Some("%r0".into()),
        vec![
            Operand::Var(name.into()),
            Operand::Var("%a".into()),
            Operand::Var("%b".into()),
        ],
        type_hint,
    )
}

/// Build a `call_builtin` with 1 argument operand and a given type_hint.
fn unary_call(name: &str, type_hint: &str) -> IIRInstr {
    IIRInstr::new(
        "call_builtin",
        Some("%r0".into()),
        vec![Operand::Var(name.into()), Operand::Var("%x".into())],
        type_hint,
    )
}

/// Shorthand: get the first instruction from the first function.
fn first_instr(m: &IIRModule) -> &IIRInstr {
    &m.functions[0].instructions[0]
}

// ===========================================================================
// Tests 1–18: All 18 numeric builtins lower to the correct op
// ===========================================================================

#[test]
fn test_01_plus_becomes_add() {
    let mut m = make_module(vec![binary_call("+", "i64")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(first_instr(&m).op, "add");
}

#[test]
fn test_02_minus_becomes_sub() {
    let mut m = make_module(vec![binary_call("-", "i64")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(first_instr(&m).op, "sub");
}

#[test]
fn test_03_star_becomes_mul() {
    let mut m = make_module(vec![binary_call("*", "i64")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(first_instr(&m).op, "mul");
}

#[test]
fn test_04_slash_becomes_div() {
    let mut m = make_module(vec![binary_call("/", "i64")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(first_instr(&m).op, "div");
}

#[test]
fn test_05_percent_becomes_mod() {
    let mut m = make_module(vec![binary_call("%", "i64")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(first_instr(&m).op, "mod");
}

#[test]
fn test_06_neg_becomes_neg() {
    let mut m = make_module(vec![unary_call("neg", "i64")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(first_instr(&m).op, "neg");
}

#[test]
fn test_07_eq_becomes_cmp_eq() {
    let mut m = make_module(vec![binary_call("=", "bool")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(first_instr(&m).op, "cmp_eq");
}

#[test]
fn test_08_ne_becomes_cmp_ne() {
    let mut m = make_module(vec![binary_call("!=", "bool")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(first_instr(&m).op, "cmp_ne");
}

#[test]
fn test_09_lt_becomes_cmp_lt() {
    let mut m = make_module(vec![binary_call("<", "bool")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(first_instr(&m).op, "cmp_lt");
}

#[test]
fn test_10_le_becomes_cmp_le() {
    let mut m = make_module(vec![binary_call("<=", "bool")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(first_instr(&m).op, "cmp_le");
}

#[test]
fn test_11_gt_becomes_cmp_gt() {
    let mut m = make_module(vec![binary_call(">", "bool")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(first_instr(&m).op, "cmp_gt");
}

#[test]
fn test_12_ge_becomes_cmp_ge() {
    let mut m = make_module(vec![binary_call(">=", "bool")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(first_instr(&m).op, "cmp_ge");
}

#[test]
fn test_13_and_becomes_and() {
    let mut m = make_module(vec![binary_call("and", "bool")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(first_instr(&m).op, "and");
}

#[test]
fn test_14_or_becomes_or() {
    let mut m = make_module(vec![binary_call("or", "bool")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(first_instr(&m).op, "or");
}

#[test]
fn test_15_not_becomes_not() {
    let mut m = make_module(vec![unary_call("not", "bool")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(first_instr(&m).op, "not");
}

#[test]
fn test_16_shl_becomes_shl() {
    let mut m = make_module(vec![binary_call("shl", "i64")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(first_instr(&m).op, "shl");
}

#[test]
fn test_17_shr_becomes_shr() {
    let mut m = make_module(vec![binary_call("shr", "i64")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(first_instr(&m).op, "shr");
}

#[test]
fn test_18_xor_becomes_xor() {
    let mut m = make_module(vec![binary_call("xor", "i64")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(first_instr(&m).op, "xor");
}

// ===========================================================================
// Tests 19–22: Binary op invariants
// ===========================================================================

#[test]
fn test_19_binary_dest_preserved() {
    // The destination register name must survive lowering unchanged.
    let instr = IIRInstr::new(
        "call_builtin",
        Some("%result_42".into()),
        vec![
            Operand::Var("+".into()),
            Operand::Var("%a".into()),
            Operand::Var("%b".into()),
        ],
        "i64",
    );
    let mut m = make_module(vec![instr]);
    lower_builtins(&mut m);
    assert_eq!(first_instr(&m).dest.as_deref(), Some("%result_42"));
}

#[test]
fn test_20_binary_srcs_stripped_of_name() {
    // After lowering, srcs should only contain the two argument operands,
    // NOT the builtin name operand that was at srcs[0].
    let mut m = make_module(vec![binary_call("+", "i64")]);
    lower_builtins(&mut m);
    let instr = first_instr(&m);
    assert_eq!(instr.srcs.len(), 2);
    assert_eq!(instr.srcs[0], Operand::Var("%a".into()));
    assert_eq!(instr.srcs[1], Operand::Var("%b".into()));
}

#[test]
fn test_21_type_hint_preserved_after_lowering() {
    // type_hint on the lowered instruction must match the original.
    let mut m = make_module(vec![binary_call("*", "f64")]);
    lower_builtins(&mut m);
    assert_eq!(first_instr(&m).type_hint, "f64");
}

#[test]
fn test_22_type_hint_bool_preserved() {
    // bool type_hint should also survive for comparison ops.
    let mut m = make_module(vec![binary_call("=", "bool")]);
    lower_builtins(&mut m);
    assert_eq!(first_instr(&m).type_hint, "bool");
}

// ===========================================================================
// Tests 23–24: Unary op invariants
// ===========================================================================

#[test]
fn test_23_unary_neg_one_src() {
    // After lowering `neg`, there must be exactly 1 source operand.
    let mut m = make_module(vec![unary_call("neg", "i64")]);
    lower_builtins(&mut m);
    let instr = first_instr(&m);
    assert_eq!(instr.op, "neg");
    assert_eq!(instr.srcs.len(), 1);
    assert_eq!(instr.srcs[0], Operand::Var("%x".into()));
}

#[test]
fn test_24_unary_not_one_src_and_dest() {
    // After lowering `not`, there must be exactly 1 source and dest preserved.
    let instr = IIRInstr::new(
        "call_builtin",
        Some("%flag".into()),
        vec![Operand::Var("not".into()), Operand::Var("%cond".into())],
        "bool",
    );
    let mut m = make_module(vec![instr]);
    lower_builtins(&mut m);
    let lowered = first_instr(&m);
    assert_eq!(lowered.op, "not");
    assert_eq!(lowered.srcs.len(), 1);
    assert_eq!(lowered.dest.as_deref(), Some("%flag"));
    assert_eq!(lowered.srcs[0], Operand::Var("%cond".into()));
}

// ===========================================================================
// Tests 25–26: Unknown builtins left unchanged
// ===========================================================================

#[test]
fn test_25_cons_left_as_call_builtin() {
    // "cons" is a heap builtin (Phase 2) and must not be touched.
    let instr = IIRInstr::new(
        "call_builtin",
        Some("%cell".into()),
        vec![
            Operand::Var("cons".into()),
            Operand::Var("%head".into()),
            Operand::Var("%tail".into()),
        ],
        "any",
    );
    let mut m = make_module(vec![instr]);
    let errors = lower_builtins(&mut m);
    // No error — unknown builtins are silently left unchanged.
    assert!(errors.is_empty());
    let i = first_instr(&m);
    assert_eq!(i.op, "call_builtin");
    assert_eq!(i.srcs.len(), 3); // unchanged — still includes the name
}

#[test]
fn test_26_make_closure_left_unchanged() {
    // "make_closure" is not in the numeric table and should be left alone.
    let instr = IIRInstr::new(
        "call_builtin",
        Some("%clos".into()),
        vec![
            Operand::Var("make_closure".into()),
            Operand::Var("%fn_ptr".into()),
            Operand::Var("%env".into()),
        ],
        "any",
    );
    let mut m = make_module(vec![instr]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(first_instr(&m).op, "call_builtin");
    assert_eq!(first_instr(&m).srcs.len(), 3);
}

// ===========================================================================
// Tests 27–28: Non-call_builtin instructions left unchanged
// ===========================================================================

#[test]
fn test_27_add_instr_left_unchanged() {
    // An existing "add" instruction (not call_builtin) must survive verbatim.
    let instr = IIRInstr::new(
        "add",
        Some("%r0".into()),
        vec![Operand::Var("%a".into()), Operand::Var("%b".into())],
        "i64",
    );
    let mut m = make_module(vec![instr]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    let i = first_instr(&m);
    assert_eq!(i.op, "add");
    assert_eq!(i.srcs.len(), 2);
}

#[test]
fn test_28_ret_instr_left_unchanged() {
    // Return instructions must not be modified.
    let instr = IIRInstr::new(
        "ret",
        None,
        vec![Operand::Var("%r0".into())],
        "i64",
    );
    let mut m = make_module(vec![instr]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(first_instr(&m).op, "ret");
}

// ===========================================================================
// Tests 29–30: may_alloc is cleared
// ===========================================================================

#[test]
fn test_29_may_alloc_cleared_for_arithmetic() {
    // Arithmetic ops never allocate; may_alloc must be false after lowering.
    let mut instr = binary_call("+", "i64");
    instr.may_alloc = true; // pretend someone set this incorrectly
    let mut m = make_module(vec![instr]);
    lower_builtins(&mut m);
    assert!(!first_instr(&m).may_alloc);
}

#[test]
fn test_30_may_alloc_cleared_for_comparison() {
    let mut instr = binary_call("<", "bool");
    instr.may_alloc = true;
    let mut m = make_module(vec![instr]);
    lower_builtins(&mut m);
    assert!(!first_instr(&m).may_alloc);
}

// ===========================================================================
// Tests 31–32: Error — WrongArity
// ===========================================================================

#[test]
fn test_31_wrong_arity_binary_given_one_arg() {
    // "+" expects 2 args; give it 1.
    let instr = IIRInstr::new(
        "call_builtin",
        Some("%r".into()),
        vec![Operand::Var("+".into()), Operand::Var("%a".into())],
        "i64",
    );
    let mut m = make_module(vec![instr]);
    let errors = lower_builtins(&mut m);
    assert_eq!(errors.len(), 1);
    match &errors[0] {
        BuiltinLoweringError::WrongArity { builtin_name, expected, found, .. } => {
            assert_eq!(builtin_name, "+");
            assert_eq!(*expected, 2);
            assert_eq!(*found, 1);
        }
        _ => panic!("expected WrongArity, got {:?}", errors[0]),
    }
}

#[test]
fn test_32_wrong_arity_unary_given_two_args() {
    // "neg" expects 1 arg; give it 2.
    let instr = IIRInstr::new(
        "call_builtin",
        Some("%r".into()),
        vec![
            Operand::Var("neg".into()),
            Operand::Var("%a".into()),
            Operand::Var("%b".into()),
        ],
        "i64",
    );
    let mut m = make_module(vec![instr]);
    let errors = lower_builtins(&mut m);
    assert_eq!(errors.len(), 1);
    match &errors[0] {
        BuiltinLoweringError::WrongArity { builtin_name, expected, found, .. } => {
            assert_eq!(builtin_name, "neg");
            assert_eq!(*expected, 1);
            assert_eq!(*found, 2);
        }
        _ => panic!("expected WrongArity, got {:?}", errors[0]),
    }
}

// ===========================================================================
// Tests 33–34: Error — UntypedBuiltin
// ===========================================================================

#[test]
fn test_33_untyped_plus_is_error() {
    // "+" with type_hint="any" means type-checker ran before lowering. Error!
    let instr = binary_call("+", "any");
    let mut m = make_module(vec![instr]);
    let errors = lower_builtins(&mut m);
    assert_eq!(errors.len(), 1);
    match &errors[0] {
        BuiltinLoweringError::UntypedBuiltin { builtin_name, .. } => {
            assert_eq!(builtin_name, "+");
        }
        _ => panic!("expected UntypedBuiltin, got {:?}", errors[0]),
    }
}

#[test]
fn test_34_untyped_comparison_is_error() {
    let instr = binary_call("=", "any");
    let mut m = make_module(vec![instr]);
    let errors = lower_builtins(&mut m);
    assert_eq!(errors.len(), 1);
    match &errors[0] {
        BuiltinLoweringError::UntypedBuiltin { builtin_name, .. } => {
            assert_eq!(builtin_name, "=");
        }
        _ => panic!("expected UntypedBuiltin"),
    }
}

// ===========================================================================
// Tests 35–37: Multiple functions — all are lowered
// ===========================================================================

#[test]
fn test_35_two_functions_both_lowered() {
    // Each function in the module has a call_builtin "+" that should be lowered.
    let fn1 = IIRFunction::new(
        "add",
        vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
        "i64",
        vec![binary_call("+", "i64")],
    );
    let fn2 = IIRFunction::new(
        "sub",
        vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
        "i64",
        vec![binary_call("-", "i64")],
    );
    let mut m = IIRModule {
        name: "test".into(),
        functions: vec![fn1, fn2],
        entry_point: Some("add".into()),
        language: "twig".into(),
    };
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(m.functions[0].instructions[0].op, "add");
    assert_eq!(m.functions[1].instructions[0].op, "sub");
}

#[test]
fn test_36_three_functions_independent() {
    // Three functions, each with a different builtin.
    let fns: Vec<IIRFunction> = [("f1", "*", "i64"), ("f2", "/", "i64"), ("f3", "%", "i64")]
        .iter()
        .map(|(name, op, ty)| {
            IIRFunction::new(
                *name,
                vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
                *ty,
                vec![binary_call(op, ty)],
            )
        })
        .collect();
    let mut m = IIRModule {
        name: "t".into(),
        functions: fns,
        entry_point: Some("f1".into()),
        language: "twig".into(),
    };
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(m.functions[0].instructions[0].op, "mul");
    assert_eq!(m.functions[1].instructions[0].op, "div");
    assert_eq!(m.functions[2].instructions[0].op, "mod");
}

#[test]
fn test_37_error_in_one_function_reports_function_name() {
    // WrongArity error should include the function name for diagnostics.
    let bad_instr = IIRInstr::new(
        "call_builtin",
        Some("%r".into()),
        vec![Operand::Var("+".into()), Operand::Var("%a".into())], // missing second arg
        "i64",
    );
    let fn_ = IIRFunction::new("my_broken_function", vec![], "i64", vec![bad_instr]);
    let mut m = IIRModule {
        name: "t".into(),
        functions: vec![fn_],
        entry_point: Some("my_broken_function".into()),
        language: "twig".into(),
    };
    let errors = lower_builtins(&mut m);
    assert_eq!(errors.len(), 1);
    match &errors[0] {
        BuiltinLoweringError::WrongArity { function_name, .. } => {
            assert_eq!(function_name, "my_broken_function");
        }
        _ => panic!("expected WrongArity"),
    }
}

// ===========================================================================
// Test 38: Empty module — no panic
// ===========================================================================

#[test]
fn test_38_empty_module_no_panic() {
    let mut m = IIRModule {
        name: "empty".into(),
        functions: vec![],
        entry_point: None,
        language: "twig".into(),
    };
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
}

// ===========================================================================
// Tests 39–40: Mixed call_builtin and non-call_builtin in same function
// ===========================================================================

#[test]
fn test_39_mixed_instrs_only_call_builtin_lowered() {
    // A function with: const, call_builtin "+", ret — only the middle is changed.
    let instrs = vec![
        IIRInstr::new("const", Some("%a".into()), vec![Operand::Int(1)], "i64"),
        binary_call("+", "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("%r0".into())], "i64"),
    ];
    let mut m = make_module(instrs);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    let instrs = &m.functions[0].instructions;
    assert_eq!(instrs[0].op, "const");  // unchanged
    assert_eq!(instrs[1].op, "add");    // lowered
    assert_eq!(instrs[2].op, "ret");    // unchanged
    assert_eq!(instrs.len(), 3);         // count preserved
}

#[test]
fn test_40_mixed_known_and_unknown_builtins() {
    // call_builtin "+" is lowered; call_builtin "cons" is left as-is.
    let instrs = vec![
        binary_call("+", "i64"),
        IIRInstr::new(
            "call_builtin",
            Some("%cell".into()),
            vec![
                Operand::Var("cons".into()),
                Operand::Var("%h".into()),
                Operand::Var("%t".into()),
            ],
            "any",
        ),
    ];
    let mut m = make_module(instrs);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(m.functions[0].instructions[0].op, "add");
    assert_eq!(m.functions[0].instructions[1].op, "call_builtin");
}

// ===========================================================================
// Tests 41–42: lower_builtins_cloned — original unchanged
// ===========================================================================

#[test]
fn test_41_cloned_lowers_correctly() {
    let fn_ = IIRFunction::new(
        "f",
        vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
        "i64",
        vec![binary_call("+", "i64")],
    );
    let original = IIRModule {
        name: "t".into(),
        functions: vec![fn_],
        entry_point: None,
        language: "twig".into(),
    };
    let (lowered, errors) = lower_builtins_cloned(&original);
    assert!(errors.is_empty());
    // Lowered copy has "add".
    assert_eq!(lowered.functions[0].instructions[0].op, "add");
}

#[test]
fn test_42_cloned_leaves_original_unchanged() {
    let fn_ = IIRFunction::new(
        "f",
        vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
        "i64",
        vec![binary_call("+", "i64")],
    );
    let original = IIRModule {
        name: "t".into(),
        functions: vec![fn_],
        entry_point: None,
        language: "twig".into(),
    };
    let (_lowered, _errors) = lower_builtins_cloned(&original);
    // Original still shows call_builtin.
    assert_eq!(original.functions[0].instructions[0].op, "call_builtin");
    // And original still has 3 srcs (builtin name + 2 args).
    assert_eq!(original.functions[0].instructions[0].srcs.len(), 3);
}

// ===========================================================================
// Tests 43–44: lower_builtins_checked
// ===========================================================================

#[test]
fn test_43_checked_returns_ok_on_success() {
    let mut m = make_module(vec![binary_call("+", "i64")]);
    let result = lower_builtins_checked(&mut m);
    assert!(result.is_ok());
    assert_eq!(first_instr(&m).op, "add");
}

#[test]
fn test_44_checked_returns_err_on_untyped() {
    let mut m = make_module(vec![binary_call("+", "any")]);
    let result = lower_builtins_checked(&mut m);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert_eq!(errors.len(), 1);
    assert!(matches!(errors[0], BuiltinLoweringError::UntypedBuiltin { .. }));
}

// ===========================================================================
// Test 45: Profiling fields preserved
// ===========================================================================

#[test]
fn test_45_profiling_fields_preserved() {
    // observation_count and observed_type are set by the VM profiler at runtime.
    // The lowering pass must not clear them.
    let mut instr = binary_call("+", "i64");
    instr.record_observation("i64");  // simulate having been profiled
    assert_eq!(instr.observation_count, 1);
    assert_eq!(instr.observed_type.as_deref(), Some("i64"));

    let mut m = make_module(vec![instr]);
    lower_builtins(&mut m);

    let lowered = first_instr(&m);
    assert_eq!(lowered.op, "add");
    // Note: lower_function uses try_lower_instr which mutates in place and
    // preserves the profiling fields because IIRInstr::new is NOT called.
    // The observation fields remain on the same struct.
    assert_eq!(lowered.observation_count, 1);
    assert_eq!(lowered.observed_type.as_deref(), Some("i64"));
}

// ===========================================================================
// Test 46: ic_slot preserved
// ===========================================================================

#[test]
fn test_46_ic_slot_preserved() {
    let mut instr = binary_call("+", "i64");
    instr.ic_slot = Some(7);  // simulate an inline-cache slot assignment

    let mut m = make_module(vec![instr]);
    lower_builtins(&mut m);

    assert_eq!(first_instr(&m).ic_slot, Some(7));
}

// ===========================================================================
// Tests 47–48: Empty instruction lists — no panic
// ===========================================================================

#[test]
fn test_47_function_with_no_instructions() {
    let fn_ = IIRFunction::new("f", vec![], "void", vec![]);
    let mut m = IIRModule {
        name: "t".into(),
        functions: vec![fn_],
        entry_point: Some("f".into()),
        language: "twig".into(),
    };
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert!(m.functions[0].instructions.is_empty());
}

#[test]
fn test_48_multiple_empty_functions() {
    let fns = vec![
        IIRFunction::new("f", vec![], "void", vec![]),
        IIRFunction::new("g", vec![], "void", vec![]),
        IIRFunction::new("h", vec![], "void", vec![]),
    ];
    let mut m = IIRModule {
        name: "t".into(),
        functions: fns,
        entry_point: Some("f".into()),
        language: "twig".into(),
    };
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
}

// ===========================================================================
// Tests 49–50: Multiple errors accumulate
// ===========================================================================

#[test]
fn test_49_multiple_wrong_arity_errors_accumulated() {
    // Two bad instructions in the same function — both errors should be reported.
    let instrs = vec![
        // "+" with 1 arg
        IIRInstr::new(
            "call_builtin",
            Some("%r1".into()),
            vec![Operand::Var("+".into()), Operand::Var("%a".into())],
            "i64",
        ),
        // "-" with 3 args
        IIRInstr::new(
            "call_builtin",
            Some("%r2".into()),
            vec![
                Operand::Var("-".into()),
                Operand::Var("%a".into()),
                Operand::Var("%b".into()),
                Operand::Var("%c".into()),
            ],
            "i64",
        ),
    ];
    let mut m = make_module(instrs);
    let errors = lower_builtins(&mut m);
    assert_eq!(errors.len(), 2);
}

#[test]
fn test_50_mixed_errors_across_functions() {
    // One function has wrong arity, another has untyped builtin.
    let fn1 = IIRFunction::new(
        "fn1",
        vec![],
        "i64",
        vec![IIRInstr::new(
            "call_builtin",
            Some("%r".into()),
            vec![Operand::Var("+".into()), Operand::Var("%a".into())], // 1 arg, expects 2
            "i64",
        )],
    );
    let fn2 = IIRFunction::new(
        "fn2",
        vec![],
        "i64",
        vec![binary_call("*", "any")], // untyped
    );
    let mut m = IIRModule {
        name: "t".into(),
        functions: vec![fn1, fn2],
        entry_point: Some("fn1".into()),
        language: "twig".into(),
    };
    let errors = lower_builtins(&mut m);
    assert_eq!(errors.len(), 2);
    // Check that function names are present in the errors (in any order).
    let has_fn1_err = errors.iter().any(|e| match e {
        BuiltinLoweringError::WrongArity { function_name, .. } => function_name == "fn1",
        _ => false,
    });
    let has_fn2_err = errors.iter().any(|e| match e {
        BuiltinLoweringError::UntypedBuiltin { function_name, .. } => function_name == "fn2",
        _ => false,
    });
    assert!(has_fn1_err, "expected error for fn1");
    assert!(has_fn2_err, "expected error for fn2");
}
