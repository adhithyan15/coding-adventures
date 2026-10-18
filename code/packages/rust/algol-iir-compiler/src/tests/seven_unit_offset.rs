//! The new seven-unit proof stays separate from shared exponential ranges.
use super::*;

#[test]
fn canonical_seven_unit_forms_have_one_real_formatter_and_independent_arms() {
    // These include every formerly refused seven-unit boundary shape.
    for expression in [
        "entier(sign(pick()) + 7)",
        "entier(sign(pick()) - 7)",
        "entier((sign(pick()) + 3) + 4)",
        "entier(1 + (6 + sign(pick())))",
        "entier(sign(pick()) + 3 + 4)",
        "entier(sign(pick()) + 2 + 5)",
        "entier(sign(pick()) + 1 + 1 + 1 + 1 + 1 + 1 + 1)",
        "entier(entier(sign(pick()) + 1) + 6)",
        "entier(abs(sign(pick()) + 2) + 5)",
        "entier(abs((sign(pick()) + 1) + 6))",
        "entier(-(sign(pick()) + 7.0))",
    ] {
        let source = format!("begin real procedure pick; pick := -2.25; real saved; saved := {expression}; output(saved) end");
        let module = compile_source(&source, "seven_unit_positive").unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(module.get_function("main").unwrap().instructions.iter()
            .filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
    }
    for condition in ["true", "false"] {
        // Separate paths may each spend seven units. Their costs are not added.
        for (left, right) in [("7", "7"), ("7", "1"), ("1", "7")] {
            let source = format!("begin boolean procedure choose; choose := {condition}; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := relay(entier(if choose() then (entier(sign(left)) + {left}) else (entier(sign(right)) + {right}))); output(saved) end");
            let module = compile_source(&source, "seven_unit_arms").unwrap_or_else(|e| panic!("{source}: {e}"));
            assert_eq!(module.get_function("main").unwrap().instructions.iter()
                .filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
        }
    }
    // Direct conditional arms have the same independent budgets as relay arms.
    for condition in ["true", "false"] {
        for (left, right) in [("7", "1"), ("1", "7")] {
            let source = format!("begin boolean procedure choose; choose := {condition}; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + {left}) else (entier(sign(right)) + {right})); output(saved) end");
            let module = compile_source(&source, "seven_unit_direct_arms").unwrap_or_else(|e| panic!("{source}: {e}"));
            assert_eq!(module.get_function("main").unwrap().instructions.iter()
                .filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
        }
    }

}

#[test]
fn seven_unit_proofs_refuse_twenty_two_units_rounding_traps_and_exp_widening() {
    // All authored loop configurations are explicit; mixed guards stay exact.
    for (case_id, source) in [
        (197, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 32); output(saved) end"),
        (198, "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 24); output(saved) end"),
        (199, "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (31 + sign(pick()))); output(saved) end"),
        (200, "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 31); output(saved) end"),
        (201, "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 30); output(saved) end"),
        (202, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 7.5); output(saved) end"),
        (203, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + sign(pick()) + 2 + 5); output(saved) end"),
        (204, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + ((9007199254740992 + 1) - 9007199254740992) + 7); output(saved) end"),
        (205, "begin real procedure pick; pick := 3.5; real saved; saved := entier(ln(exp(exp(exp(sign(pick()) + 7))))); output(saved) end"),
        (206, "begin real procedure pick; pick := 3.5; real saved; saved := entier(ln(exp(exp(exp((sign(pick()) + 1) + 6))))); output(saved) end"),
        (207, "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 7) * 1); output(saved) end"),
        (208, "begin real procedure pick; pick := 3.5; real saved; saved := entier(exp(sign(pick())) + 7); output(saved) end"),
        (209, "begin integer procedure sign(x); value x; real x; sign := 1; real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 1) + 6); output(saved) end"),
        (210, "begin integer procedure abs(x); value x; real x; abs := 1; real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 1) + 6); output(saved) end"),
        (211, "begin integer procedure entier(x); value x; real x; entier := 1; real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 1) + 6); output(saved) end"),
    ] {
        assert!(matches!(compile_source(source, "cap24_refusal"), Err(CompileError::Type(message)) if message.contains("cannot print a real value")), "original case {case_id}: {source}");
        eprintln!("cap23-corpus REFUSED original-case={case_id}");
    }
}

#[test]
fn current_cap23_preserves_exact_sources_for_seven_unit_proofs_refuse_twenty_two_units_rounding_traps_and_exp_widening() {
    // Exact former sources retain their original producer/declarations/consumer.
    for (case_id, source) in [
        (197, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 22); output(saved) end"),
        (198, "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 14); output(saved) end"),
        (199, "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (21 + sign(pick()))); output(saved) end"),
        (200, "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 21); output(saved) end"),
        (201, "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 20); output(saved) end"),
    ] {
        let module = compile_source(source, "retained_original").unwrap_or_else(|error| panic!("original case {case_id}: {source}: {error:?}"));
        let instructions = &module.get_function("main").expect("main").instructions;
        assert_eq!(instructions.iter().filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "original case {case_id}: {source}");
        assert_eq!(instructions.iter().filter(|i| i.op == "call_builtin" && i.srcs.first().and_then(Operand::as_var) == Some("print_i64")).count(), 0, "original case {case_id}: {source}");
        eprintln!("cap23-corpus ADMITTED original-case={case_id}");
    }
}

#[test]
fn seven_unit_conditional_alias_effects_obey_the_discovery_limit() {
    struct Guard;
    impl Drop for Guard { fn drop(&mut self) { SELECTOR_CLASSIFIER_BUDGET.with(|c| c.set(None)); } }
    for condition in ["true", "false"] { for root in ["left", "right"] { for value in [false, true] {
        let mode = if value { "value t; " } else { "" };
        let source = format!("begin boolean procedure choose(t); {mode}real t; begin t := 0.0; choose := {condition} end; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := relay(entier(if choose({root}) then (entier(sign(left)) + 7) else ((entier(sign(right)) + 1) + 6))); output(saved) end");
        SELECTOR_CLASSIFIER_BUDGET.with(|c| c.set(Some((0, 8192)))); let _guard = Guard;
        let result = compile_source(&source, "seven_unit_effect");
        if value { result.unwrap_or_else(|e| panic!("{source}: {e}")); }
        else { assert!(matches!(result, Err(CompileError::Type(m)) if m.contains("cannot print a real value")), "{source}"); }
        let (visits, limit) = SELECTOR_CLASSIFIER_BUDGET.with(Cell::get).unwrap();
        assert!(visits > 0 && visits <= limit);
        eprintln!("seven-unit condition={condition} root={root} value={value} visits={visits} limit={limit}");
    } } }
}

// Every formerly refused full authored source stays observable, including its
// producer value/declarations. Numerical cap migration cannot erase a source.
#[test]
fn current_main_preserves_all_former_nineteen_unit_authored_sources() {
    for source in [
        "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 19); output(saved) end",
        "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 11); output(saved) end",
        "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (18 + sign(pick()))); output(saved) end",
        "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 18); output(saved) end",
        "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 17); output(saved) end",

    ] {
        let module=compile_source(source,"former_nineteen_source").unwrap_or_else(|e|panic!("{source}: {e}"));
        let instructions=&module.get_function("main").unwrap().instructions;
        assert_eq!(instructions.iter().filter(|i|i.op=="call"&&i.srcs.first().and_then(Operand::as_var)==Some("__basic_print_real")).count(),1,"{source}");
        assert_eq!(instructions.iter().filter(|i|i.op=="call_builtin"&&i.srcs.first().and_then(Operand::as_var)==Some("print_i64")).count(),0,"{source}");
    }
}
