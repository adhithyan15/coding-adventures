//! The new six-unit proof stays separate from shared exponential ranges.
use super::*;

#[test]
fn canonical_six_unit_forms_have_one_real_formatter_and_independent_arms() {
    // These include every formerly refused six-unit boundary shape.
    for expression in [
        "entier(sign(pick()) + 6)",
        "entier(sign(pick()) - 6)",
        "entier((sign(pick()) + 3) + 3)",
        "entier(1 + (5 + sign(pick())))",
        "entier(sign(pick()) + 3 + 3)",
        "entier(sign(pick()) + 1 + 5)",
        "entier(sign(pick()) + 1 + 1 + 1 + 1 + 1 + 1)",
        "entier(entier(sign(pick()) + 1) + 5)",
        "entier(abs(sign(pick()) + 2) + 4)",
        "entier(abs((sign(pick()) + 1) + 5))",
        "entier(-(sign(pick()) + 6.0))",
    ] {
        let source = format!("begin real procedure pick; pick := -2.25; real saved; saved := {expression}; output(saved) end");
        let module = compile_source(&source, "six_unit_positive").unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(module.get_function("main").unwrap().instructions.iter()
            .filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
    }
    for condition in ["true", "false"] {
        // Separate paths may each spend six units. Their costs are not added.
        for (left, right) in [("6", "6"), ("6", "1"), ("1", "6")] {
            let source = format!("begin boolean procedure choose; choose := {condition}; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := relay(entier(if choose() then (entier(sign(left)) + {left}) else (entier(sign(right)) + {right}))); output(saved) end");
            let module = compile_source(&source, "six_unit_arms").unwrap_or_else(|e| panic!("{source}: {e}"));
            assert_eq!(module.get_function("main").unwrap().instructions.iter()
                .filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
        }
    }
    // Direct conditional arms have the same independent budgets as relay arms.
    for condition in ["true", "false"] {
        for (left, right) in [("6", "1"), ("1", "6")] {
            let source = format!("begin boolean procedure choose; choose := {condition}; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + {left}) else (entier(sign(right)) + {right})); output(saved) end");
            let module = compile_source(&source, "six_unit_direct_arms").unwrap_or_else(|e| panic!("{source}: {e}"));
            assert_eq!(module.get_function("main").unwrap().instructions.iter()
                .filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
        }
    }

}

#[test]
fn six_unit_proofs_refuse_eleven_units_rounding_traps_and_exp_widening() {
    for expression in [
        "entier(sign(pick()) + 11)",
        "entier((sign(pick()) + 5) + 6)",
        "entier(1 + (10 + sign(pick())))",
        "entier(entier(sign(pick()) + 1) + 10)",
        "entier(abs(sign(pick()) + 2) + 9)",
        "entier(sign(pick()) + 6.5)",
        "entier(sign(pick()) + sign(pick()) + 1 + 5)",
        "entier(sign(pick()) + ((9007199254740992 + 1) - 9007199254740992) + 6)",
        "entier(ln(exp(exp(exp(sign(pick()) + 6)))))",
        "entier(ln(exp(exp(exp((sign(pick()) + 1) + 5)))))",
        "entier((sign(pick()) + 6) * 1)",
        "entier(exp(sign(pick())) + 6)",
    ] {
        let source = format!("begin real procedure pick; pick := 3.5; real saved; saved := {expression}; output(saved) end");
        assert!(matches!(compile_source(&source, "six_unit_refusal"), Err(CompileError::Type(m)) if m.contains("cannot print a real value")), "{source}");
    }
    for name in ["sign", "abs", "entier"] {
        let source = format!("begin integer procedure {name}(x); value x; real x; {name} := 1; real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 1) + 5); output(saved) end");
        assert!(matches!(compile_source(&source, "six_unit_shadow"), Err(CompileError::Type(m)) if m.contains("cannot print a real value")), "{source}");
    }
}

#[test]
fn six_unit_conditional_alias_effects_obey_the_discovery_limit() {
    struct Guard;
    impl Drop for Guard { fn drop(&mut self) { SELECTOR_CLASSIFIER_BUDGET.with(|c| c.set(None)); } }
    for condition in ["true", "false"] { for root in ["left", "right"] { for value in [false, true] {
        let mode = if value { "value t; " } else { "" };
        let source = format!("begin boolean procedure choose(t); {mode}real t; begin t := 0.0; choose := {condition} end; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := relay(entier(if choose({root}) then (entier(sign(left)) + 6) else ((entier(sign(right)) + 1) + 5))); output(saved) end");
        SELECTOR_CLASSIFIER_BUDGET.with(|c| c.set(Some((0, 8192)))); let _guard = Guard;
        let result = compile_source(&source, "six_unit_effect");
        if value { result.unwrap_or_else(|e| panic!("{source}: {e}")); }
        else { assert!(matches!(result, Err(CompileError::Type(m)) if m.contains("cannot print a real value")), "{source}"); }
        let (visits, limit) = SELECTOR_CLASSIFIER_BUDGET.with(Cell::get).unwrap();
        assert!(visits > 0 && visits <= limit);
        eprintln!("six-unit condition={condition} root={root} value={value} visits={visits} limit={limit}");
    } } }
}
