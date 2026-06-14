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
use iir_builtin_lowering::{lower_builtins, lower_builtins_cloned, lower_builtins_checked, lower_heap_builtins, BuiltinLoweringError};

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
        exports: vec![],
        imports: vec![],
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
fn test_25_cons_lowered_by_phase2() {
    // "cons" is a heap builtin — Phase 1 ignores it, but Phase 2 lowers it.
    // lower_builtins() now runs both phases, so cons IS lowered.
    let instr = IIRInstr::new(
        "call_builtin",
        Some("%cell".into()),
        vec![
            Operand::Var("cons".into()),
            Operand::Var("%head".into()),
            Operand::Var("%tail".into()),
        ],
        "ref<LispyPair>",
    );
    let mut m = make_module(vec![instr]);
    let errors = lower_builtins(&mut m);
    // No error — heap lowering is infallible.
    assert!(errors.is_empty());
    // Phase 2 expanded cons into alloc + field_store + field_store.
    assert_eq!(m.functions[0].instructions.len(), 3);
    assert_eq!(m.functions[0].instructions[0].op, "alloc");
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
        exports: vec![],
        imports: vec![],
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
        exports: vec![],
        imports: vec![],
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
        exports: vec![],
        imports: vec![],
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
        exports: vec![],
        imports: vec![],
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
fn test_40_mixed_known_and_cons_builtins() {
    // call_builtin "+" is lowered by Phase 1 to "add".
    // call_builtin "cons" is lowered by Phase 2 to alloc + 2 field_stores.
    // Together: 1 add + 3 heap ops = 4 instructions.
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
            "ref<LispyPair>",
        ),
    ];
    let mut m = make_module(instrs);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    // Instruction 0: add (from Phase 1 lowering of "+")
    assert_eq!(m.functions[0].instructions[0].op, "add");
    // Instruction 1: alloc (from Phase 2 lowering of cons — step 1 of 3)
    assert_eq!(m.functions[0].instructions[1].op, "alloc");
    // Total: 4 instructions (1 add + 3 heap ops)
    assert_eq!(m.functions[0].instructions.len(), 4);
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
        exports: vec![],
        imports: vec![],
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
        exports: vec![],
        imports: vec![],
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
        exports: vec![],
        imports: vec![],
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
        exports: vec![],
        imports: vec![],
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
        exports: vec![],
        imports: vec![],
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

// ===========================================================================
// Tests 51–74: Phase 2 — heap builtin lowering
// ===========================================================================
//
// These tests verify that the heap lowering pass (lower_heap_builtins and the
// heap phase wired into lower_builtins) correctly rewrites cons/car/cdr/null?/
// make_nil into typed IIR heap ops.
//
// Organisation:
// 51–66: cons lowering details (instruction count, op names, fields).
// 67–68: car → field_load index 0.
// 69–70: cdr → field_load index 1.
// 71–72: null? → is_null.
// 73–74: make_nil → const Int(0) with ref<LispyPair>.
// 75–76: unknown heap builtins (pair?, make_closure) left unchanged.
// 77:    multiple cons in sequence.
// 78:    cons + car + null? in same function.
// 79:    cons + arithmetic in same function (numeric lowering already happened).
// 80:    lower_builtins integrates both phases end-to-end.

// Helpers for heap tests

fn cons_call_instr(dest: &str, head: &str, tail: &str) -> IIRInstr {
    IIRInstr::new(
        "call_builtin",
        Some(dest.into()),
        vec![
            Operand::Var("cons".into()),
            Operand::Var(head.into()),
            Operand::Var(tail.into()),
        ],
        "ref<LispyPair>",
    )
}

fn car_call_instr(dest: &str, pair: &str) -> IIRInstr {
    IIRInstr::new(
        "call_builtin",
        Some(dest.into()),
        vec![Operand::Var("car".into()), Operand::Var(pair.into())],
        "ref<any>",
    )
}

fn cdr_call_instr(dest: &str, pair: &str) -> IIRInstr {
    IIRInstr::new(
        "call_builtin",
        Some(dest.into()),
        vec![Operand::Var("cdr".into()), Operand::Var(pair.into())],
        "ref<any>",
    )
}

fn null_pred_instr(dest: &str, var: &str) -> IIRInstr {
    IIRInstr::new(
        "call_builtin",
        Some(dest.into()),
        vec![Operand::Var("null?".into()), Operand::Var(var.into())],
        "bool",
    )
}

fn make_nil_instr(dest: &str) -> IIRInstr {
    IIRInstr::new(
        "call_builtin",
        Some(dest.into()),
        vec![Operand::Var("make_nil".into())],
        "ref<LispyPair>",
    )
}

// ===========================================================================
// Tests 51–54: cons produces 3 instructions
// ===========================================================================

#[test]
fn test_51_cons_expands_to_three_instructions() {
    // cons %h %t → alloc + field_store + field_store = 3 instructions.
    let mut m = make_module(vec![cons_call_instr("%cell", "%h", "%t")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(m.functions[0].instructions.len(), 3,
        "cons must expand to exactly 3 instructions");
}

#[test]
fn test_52_cons_first_instr_is_alloc() {
    let mut m = make_module(vec![cons_call_instr("%cell", "%h", "%t")]);
    lower_builtins(&mut m);
    assert_eq!(m.functions[0].instructions[0].op, "alloc");
}

#[test]
fn test_53_cons_second_instr_is_field_store() {
    let mut m = make_module(vec![cons_call_instr("%cell", "%h", "%t")]);
    lower_builtins(&mut m);
    assert_eq!(m.functions[0].instructions[1].op, "field_store");
}

#[test]
fn test_54_cons_third_instr_is_field_store() {
    let mut m = make_module(vec![cons_call_instr("%cell", "%h", "%t")]);
    lower_builtins(&mut m);
    assert_eq!(m.functions[0].instructions[2].op, "field_store");
}

// ===========================================================================
// Tests 55–57: alloc instruction properties
// ===========================================================================

#[test]
fn test_55_cons_alloc_gets_original_dest() {
    // The alloc instruction must inherit the original dest name.
    let mut m = make_module(vec![cons_call_instr("my_cell", "%h", "%t")]);
    lower_builtins(&mut m);
    assert_eq!(
        m.functions[0].instructions[0].dest.as_deref(),
        Some("my_cell"),
        "alloc must inherit the original dest"
    );
}

#[test]
fn test_56_cons_alloc_may_alloc_is_true() {
    // alloc is a heap allocation point; the GC must track it.
    let mut m = make_module(vec![cons_call_instr("%cell", "%h", "%t")]);
    lower_builtins(&mut m);
    assert!(m.functions[0].instructions[0].may_alloc,
        "alloc instruction must have may_alloc=true");
}

#[test]
fn test_57_cons_alloc_type_hint_is_ref_lispy_pair() {
    let mut m = make_module(vec![cons_call_instr("%cell", "%h", "%t")]);
    lower_builtins(&mut m);
    assert_eq!(m.functions[0].instructions[0].type_hint, "ref<LispyPair>");
}

// ===========================================================================
// Tests 58–63: field_store operand layout
// ===========================================================================

#[test]
fn test_58_cons_field_store_head_dest_is_none() {
    // Stores never produce a value.
    let mut m = make_module(vec![cons_call_instr("%cell", "%h", "%t")]);
    lower_builtins(&mut m);
    assert!(m.functions[0].instructions[1].dest.is_none(),
        "field_store head must have no dest");
}

#[test]
fn test_59_cons_field_store_tail_dest_is_none() {
    let mut m = make_module(vec![cons_call_instr("%cell", "%h", "%t")]);
    lower_builtins(&mut m);
    assert!(m.functions[0].instructions[2].dest.is_none(),
        "field_store tail must have no dest");
}

#[test]
fn test_60_cons_field_store_head_srcs_index0_is_cell() {
    // field_store srcs[0] is the pair pointer.
    let mut m = make_module(vec![cons_call_instr("%cell", "%h", "%t")]);
    lower_builtins(&mut m);
    let store = &m.functions[0].instructions[1];
    assert_eq!(store.srcs[0], Operand::Var("%cell".into()),
        "field_store head: srcs[0] must be the cell pointer");
}

#[test]
fn test_61_cons_field_store_head_srcs_index1_is_zero() {
    // field_store srcs[1] is the field index: 0 = car slot.
    let mut m = make_module(vec![cons_call_instr("%cell", "%h", "%t")]);
    lower_builtins(&mut m);
    let store = &m.functions[0].instructions[1];
    assert_eq!(store.srcs[1], Operand::Int(0),
        "field_store head: srcs[1] must be Int(0)");
}

#[test]
fn test_62_cons_field_store_head_srcs_index2_is_head() {
    // field_store srcs[2] is the head value.
    let mut m = make_module(vec![cons_call_instr("%cell", "%h", "%t")]);
    lower_builtins(&mut m);
    let store = &m.functions[0].instructions[1];
    assert_eq!(store.srcs[2], Operand::Var("%h".into()),
        "field_store head: srcs[2] must be the head variable");
}

#[test]
fn test_63_cons_field_store_tail_srcs_index1_is_one() {
    // field_store tail: srcs[1] is the field index: 1 = cdr slot.
    let mut m = make_module(vec![cons_call_instr("%cell", "%h", "%t")]);
    lower_builtins(&mut m);
    let store = &m.functions[0].instructions[2];
    assert_eq!(store.srcs[1], Operand::Int(1),
        "field_store tail: srcs[1] must be Int(1)");
}

// ===========================================================================
// Tests 64–65: car lowers to field_load with index 0
// ===========================================================================

#[test]
fn test_64_car_produces_field_load_index_zero() {
    let mut m = make_module(vec![car_call_instr("%head", "%pair")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    let instr = &m.functions[0].instructions[0];
    assert_eq!(instr.op, "field_load");
    assert_eq!(instr.srcs[1], Operand::Int(0),
        "car must use field index 0");
}

#[test]
fn test_65_car_field_load_pair_is_src0() {
    let mut m = make_module(vec![car_call_instr("%head", "%pair")]);
    lower_builtins(&mut m);
    let instr = &m.functions[0].instructions[0];
    assert_eq!(instr.srcs[0], Operand::Var("%pair".into()));
}

// ===========================================================================
// Tests 66–67: cdr lowers to field_load with index 1
// ===========================================================================

#[test]
fn test_66_cdr_produces_field_load_index_one() {
    let mut m = make_module(vec![cdr_call_instr("%tail", "%pair")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    let instr = &m.functions[0].instructions[0];
    assert_eq!(instr.op, "field_load");
    assert_eq!(instr.srcs[1], Operand::Int(1),
        "cdr must use field index 1");
}

#[test]
fn test_67_cdr_field_load_preserves_dest() {
    let mut m = make_module(vec![cdr_call_instr("my_tail", "%pair")]);
    lower_builtins(&mut m);
    let instr = &m.functions[0].instructions[0];
    assert_eq!(instr.dest.as_deref(), Some("my_tail"));
}

// ===========================================================================
// Tests 68–69: null? lowers to is_null
// ===========================================================================

#[test]
fn test_68_null_pred_produces_is_null() {
    let mut m = make_module(vec![null_pred_instr("%r", "%x")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(m.functions[0].instructions[0].op, "is_null");
}

#[test]
fn test_69_null_pred_type_hint_is_bool() {
    let mut m = make_module(vec![null_pred_instr("%r", "%x")]);
    lower_builtins(&mut m);
    assert_eq!(m.functions[0].instructions[0].type_hint, "bool");
}

// ===========================================================================
// Tests 70–71: make_nil lowers to const Int(0) : ref<LispyPair>
// ===========================================================================

#[test]
fn test_70_make_nil_produces_const_zero() {
    let mut m = make_module(vec![make_nil_instr("%nil")]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    let instr = &m.functions[0].instructions[0];
    assert_eq!(instr.op, "const");
    assert_eq!(instr.srcs[0], Operand::Int(0));
}

#[test]
fn test_71_make_nil_type_hint_is_ref_lispy_pair() {
    let mut m = make_module(vec![make_nil_instr("%nil")]);
    lower_builtins(&mut m);
    assert_eq!(m.functions[0].instructions[0].type_hint, "ref<LispyPair>");
}

// ===========================================================================
// Tests 72–73: unknown heap builtins left unchanged
// ===========================================================================

#[test]
fn test_72_pair_pred_left_unchanged() {
    // `pair?` is a type predicate, not a heap operation; it stays as call_builtin.
    let instr = IIRInstr::new(
        "call_builtin",
        Some("%r".into()),
        vec![Operand::Var("pair?".into()), Operand::Var("%x".into())],
        "bool",
    );
    let mut m = make_module(vec![instr]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(m.functions[0].instructions[0].op, "call_builtin",
        "pair? should be left as call_builtin");
}

#[test]
fn test_73_make_closure_unresolvable_left_unchanged() {
    // Phase 4 (LANG34) lowers make_closure only when the fn_name register
    // can be resolved to a compile-time string literal via a preceding const
    // instruction.  When the register (%fn here) has no such const, the
    // instruction is left unchanged so the backend validator or twig-vm
    // fallback can handle it.
    let instr = IIRInstr::new(
        "call_builtin",
        Some("%clos".into()),
        vec![Operand::Var("make_closure".into()), Operand::Var("%fn".into())],
        "any",
    );
    let mut m = make_module(vec![instr]);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(m.functions[0].instructions[0].op, "call_builtin");
}

// ===========================================================================
// Tests 74–75: sequences and mixed instructions
// ===========================================================================

#[test]
fn test_74_multiple_cons_in_sequence() {
    // Two consecutive cons calls → 2 × 3 = 6 instructions.
    let instrs = vec![
        cons_call_instr("%c1", "%h1", "%t1"),
        cons_call_instr("%c2", "%h2", "%t2"),
    ];
    let mut m = make_module(instrs);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    assert_eq!(m.functions[0].instructions.len(), 6,
        "two cons calls should produce 6 instructions");
}

#[test]
fn test_75_cons_then_car_then_null_in_same_function() {
    // Simulate a real list-processing snippet:
    //   %cell = cons(%head, %tail)
    //   %h    = car(%cell)
    //   %nil  = null?(%h)
    let instrs = vec![
        cons_call_instr("%cell", "%head", "%tail"),
        car_call_instr("%h", "%cell"),
        null_pred_instr("%nil", "%h"),
    ];
    let mut m = make_module(instrs);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    // cons→3, car→1, null?→1 = 5
    assert_eq!(m.functions[0].instructions.len(), 5);
    assert_eq!(m.functions[0].instructions[0].op, "alloc");
    assert_eq!(m.functions[0].instructions[3].op, "field_load");
    assert_eq!(m.functions[0].instructions[4].op, "is_null");
}

// ===========================================================================
// Test 76: cons + arithmetic in same function (numeric lowering already done)
// ===========================================================================

#[test]
fn test_76_cons_with_arithmetic_in_same_function() {
    // When both phases run, arithmetic ops and heap ops coexist cleanly.
    let instrs = vec![
        // Phase 1 (numeric) lowers this
        IIRInstr::new(
            "call_builtin",
            Some("%sum".into()),
            vec![
                Operand::Var("+".into()),
                Operand::Var("%a".into()),
                Operand::Var("%b".into()),
            ],
            "i64",
        ),
        // Phase 2 (heap) lowers this
        cons_call_instr("%pair", "%sum", "%nil"),
    ];
    let mut m = make_module(instrs);
    let errors = lower_builtins(&mut m);
    assert!(errors.is_empty());
    // add (1) + alloc + field_store + field_store (3) = 4
    assert_eq!(m.functions[0].instructions.len(), 4);
    assert_eq!(m.functions[0].instructions[0].op, "add",
        "numeric lowering should convert + to add");
    assert_eq!(m.functions[0].instructions[1].op, "alloc",
        "heap lowering should convert cons to alloc");
}

// ===========================================================================
// Test 77: lower_heap_builtins alone (Phase 2 only, via direct call)
// ===========================================================================

#[test]
fn test_77_lower_heap_builtins_direct_call() {
    // Callers can invoke Phase 2 directly via the re-exported symbol.
    let fn_ = IIRFunction::new(
        "f",
        vec![("p".into(), "ref<LispyPair>".into())],
        "ref<any>",
        vec![car_call_instr("%h", "p")],
    );
    let mut m = IIRModule {
        name: "t".into(),
        functions: vec![fn_],
        entry_point: Some("f".into()),
        language: "twig".into(),
        exports: vec![],
        imports: vec![],
    };
    lower_heap_builtins(&mut m);
    assert_eq!(m.functions[0].instructions[0].op, "field_load");
}

