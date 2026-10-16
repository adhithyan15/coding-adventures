//! Exact old source/consumer contexts, restored at the validated cap21.
//! Baseline all refusals before migrating only quantitative oracles.
use super::*;

// Original owned_cap21: al4_additive_entier_sign_widening_rejects_unproven_terms.
// pure_quantitative_budget22; full declaration and consumer remain unchanged.
#[test]
fn retained_original_context_01() {
    let source = "begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 22); output(result) end";
    let module = compile_source(source, "test").unwrap_or_else(|error| panic!("exact original quantitative context {source}: {error:?}"));
    let instructions = &module.get_function("main").expect("main").instructions;
    assert_eq!(instructions.iter().filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
    assert_eq!(instructions.iter().filter(|i| i.op == "call_builtin" && i.srcs.first().and_then(Operand::as_var) == Some("print_i64")).count(), 0, "{source}");
    eprintln!("retained-cap23 ADMITTED context=1");
}

// Original owned_cap21: al4_additive_entier_sign_widening_rejects_unproven_terms.
// pure_quantitative_budget22; full declaration and consumer remain unchanged.
#[test]
fn retained_original_context_02() {
    let source = "begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 21 + 1); output(result) end";
    let module = compile_source(source, "test").unwrap_or_else(|error| panic!("exact original quantitative context {source}: {error:?}"));
    let instructions = &module.get_function("main").expect("main").instructions;
    assert_eq!(instructions.iter().filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
    assert_eq!(instructions.iter().filter(|i| i.op == "call_builtin" && i.srcs.first().and_then(Operand::as_var) == Some("print_i64")).count(), 0, "{source}");
    eprintln!("retained-cap23 ADMITTED context=2");
}

// Original owned_cap21: al4_additive_entier_sign_widening_rejects_unproven_terms.
// pure_quantitative_budget22; full declaration and consumer remain unchanged.
#[test]
fn retained_original_context_03() {
    let source = "begin real procedure pick; pick := -2.25; real result; result := entier((sign(pick()) + 21) + 1); output(result) end";
    let module = compile_source(source, "test").unwrap_or_else(|error| panic!("exact original quantitative context {source}: {error:?}"));
    let instructions = &module.get_function("main").expect("main").instructions;
    assert_eq!(instructions.iter().filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
    assert_eq!(instructions.iter().filter(|i| i.op == "call_builtin" && i.srcs.first().and_then(Operand::as_var) == Some("print_i64")).count(), 0, "{source}");
    eprintln!("retained-cap23 ADMITTED context=3");
}

// Original owned_cap21: al4_additive_entier_sign_widening_rejects_unproven_terms.
// pure_quantitative_budget22; full declaration and consumer remain unchanged.
#[test]
fn retained_original_context_04() {
    let source = "begin real procedure pick; pick := -2.25; real result; result := entier(abs((sign(pick()) + 11) + 11)); output(result) end";
    let module = compile_source(source, "test").unwrap_or_else(|error| panic!("exact original quantitative context {source}: {error:?}"));
    let instructions = &module.get_function("main").expect("main").instructions;
    assert_eq!(instructions.iter().filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
    assert_eq!(instructions.iter().filter(|i| i.op == "call_builtin" && i.srcs.first().and_then(Operand::as_var) == Some("print_i64")).count(), 0, "{source}");
    eprintln!("retained-cap23 ADMITTED context=4");
}

// Original incoming_cap22_intermediate: al4_runtime_real_conditional_selector_calls_reject_unsafe_local_branches.
// pure_quantitative_budget23; full declaration and consumer remain unchanged.
#[test]
fn retained_original_context_05() {
    let source = "begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 23)) else relay(entier(entier(sign(right)) + 23)); output(result) end";
    let module = compile_source(source, "test").unwrap_or_else(|error| panic!("exact original quantitative context {source}: {error:?}"));
    let instructions = &module.get_function("main").expect("main").instructions;
    assert_eq!(instructions.iter().filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
    assert_eq!(instructions.iter().filter(|i| i.op == "call_builtin" && i.srcs.first().and_then(Operand::as_var) == Some("print_i64")).count(), 0, "{source}");
    eprintln!("retained-cap23 ADMITTED context=5");
}

// Original incoming_cap22_intermediate: al4_runtime_real_conditional_selector_calls_reject_unsafe_local_branches.
// pure_quantitative_budget23; full declaration and consumer remain unchanged.
#[test]
fn retained_original_context_06() {
    let source = "begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := if choose() then relay(entier(entier(sign(left)) + 22 + 1)) else relay(entier(entier(sign(right)) + 1 + 22)); output(result) end";
    let module = compile_source(source, "test").unwrap_or_else(|error| panic!("exact original quantitative context {source}: {error:?}"));
    let instructions = &module.get_function("main").expect("main").instructions;
    assert_eq!(instructions.iter().filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
    assert_eq!(instructions.iter().filter(|i| i.op == "call_builtin" && i.srcs.first().and_then(Operand::as_var) == Some("print_i64")).count(), 0, "{source}");
    eprintln!("retained-cap23 ADMITTED context=6");
}

// Original incoming_cap22_intermediate: al4_runtime_real_conditional_selector_calls_reject_unsafe_local_branches.
// structural_conditional_under_addition_keep_negative; full declaration and consumer remain unchanged.
#[test]
fn retained_original_context_07() {
    let source = "begin boolean procedure choose; choose := true; real procedure pick; pick := 2.25; real procedure relay(x); real x; relay := x; real left, right, result; left := pick(); right := pick(); result := relay(entier((if choose() then (entier(sign(left)) + 22) else (entier(sign(right)) + 22)) + 1)); output(result) end";
    let error = compile_source(source, "test").expect_err("cap21 exact original source remains refused");
    assert!(format!("{error:?}").contains("cannot print a real value"), "{source}: {error:?}");
    eprintln!("retained-cap21 REFUSED context=7");
}

// Original incoming_cap22_intermediate: al4_additive_entier_sign_widening_rejects_unproven_terms.
// pure_quantitative_budget23; full declaration and consumer remain unchanged.
#[test]
fn retained_original_context_08() {
    let source = "begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 23); output(result) end";
    let module = compile_source(source, "test").unwrap_or_else(|error| panic!("exact original quantitative context {source}: {error:?}"));
    let instructions = &module.get_function("main").expect("main").instructions;
    assert_eq!(instructions.iter().filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
    assert_eq!(instructions.iter().filter(|i| i.op == "call_builtin" && i.srcs.first().and_then(Operand::as_var) == Some("print_i64")).count(), 0, "{source}");
    eprintln!("retained-cap23 ADMITTED context=8");
}

// Original incoming_cap22_intermediate: al4_additive_entier_sign_widening_rejects_unproven_terms.
// pure_quantitative_budget23; full declaration and consumer remain unchanged.
#[test]
fn retained_original_context_09() {
    let source = "begin real procedure pick; pick := -2.25; real result; result := entier(sign(pick()) + 22 + 1); output(result) end";
    let module = compile_source(source, "test").unwrap_or_else(|error| panic!("exact original quantitative context {source}: {error:?}"));
    let instructions = &module.get_function("main").expect("main").instructions;
    assert_eq!(instructions.iter().filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
    assert_eq!(instructions.iter().filter(|i| i.op == "call_builtin" && i.srcs.first().and_then(Operand::as_var) == Some("print_i64")).count(), 0, "{source}");
    eprintln!("retained-cap23 ADMITTED context=9");
}

// Original incoming_cap22_intermediate: al4_additive_entier_sign_widening_rejects_unproven_terms.
// pure_quantitative_budget23; full declaration and consumer remain unchanged.
#[test]
fn retained_original_context_10() {
    let source = "begin real procedure pick; pick := -2.25; real result; result := entier((sign(pick()) + 22) + 1); output(result) end";
    let module = compile_source(source, "test").unwrap_or_else(|error| panic!("exact original quantitative context {source}: {error:?}"));
    let instructions = &module.get_function("main").expect("main").instructions;
    assert_eq!(instructions.iter().filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
    assert_eq!(instructions.iter().filter(|i| i.op == "call_builtin" && i.srcs.first().and_then(Operand::as_var) == Some("print_i64")).count(), 0, "{source}");
    eprintln!("retained-cap23 ADMITTED context=10");
}

// Original incoming_cap22_intermediate: al4_additive_entier_sign_widening_rejects_unproven_terms.
// pure_quantitative_budget23; full declaration and consumer remain unchanged.
#[test]
fn retained_original_context_11() {
    let source = "begin real procedure pick; pick := -2.25; real result; result := entier(abs((sign(pick()) + 11) + 12)); output(result) end";
    let module = compile_source(source, "test").unwrap_or_else(|error| panic!("exact original quantitative context {source}: {error:?}"));
    let instructions = &module.get_function("main").expect("main").instructions;
    assert_eq!(instructions.iter().filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
    assert_eq!(instructions.iter().filter(|i| i.op == "call_builtin" && i.srcs.first().and_then(Operand::as_var) == Some("print_i64")).count(), 0, "{source}");
    eprintln!("retained-cap23 ADMITTED context=11");
}

// Individually admitted 2^53 leaves must not hide a unit via intermediate rounding.
#[test]
fn current_cap_precision_and_shared_exp_guards_remain_refused() {
    for expression in [
        "entier(sign(pick()) + ((9007199254740992 + 1) - 9007199254740992 + 23))",
        "entier(sign(pick()) + ((9007199254740992 + 1) - 9007199254740992 + 22))",
        "entier(sign(pick()) + ((9007199254740993 - 9007199254740992) + 23))",
        "entier(ln(exp(exp(sign(pick()) + 23))))",
        "entier(ln(exp(exp(sign(pick()) + 22))))",
    ] {
        let source = format!("begin real procedure pick; pick := 3.5; real saved; saved := {expression}; output(saved) end");
        assert!(matches!(compile_source(&source, "precision_guard"), Err(CompileError::Type(message)) if message.contains("cannot print a real value")), "{source}");
    }
}

#[test]
fn canonical_twenty_four_units_stay_outside_the_dedicated_budget() {
    // Exact complete generated sources; quantitative boundary is now25.
    for source in [
        "begin real procedure pick; pick := -2.25; real saved; saved := entier(sign(pick()) + 25); output(saved) end",
        "begin real procedure pick; pick := -2.25; real saved; saved := entier((sign(pick()) + 12) + 13); output(saved) end",
        "begin real procedure pick; pick := -2.25; real saved; saved := entier(entier(sign(pick()) + 1) + 24); output(saved) end",
        "begin real procedure pick; pick := -2.25; real saved; saved := entier(abs(sign(pick()) + 12) + 13); output(saved) end",
    ] {
        assert!(matches!(compile_source(source, "current_cap_refusal"), Err(CompileError::Type(message)) if message.contains("cannot print a real value")), "{source}");
    }
}
