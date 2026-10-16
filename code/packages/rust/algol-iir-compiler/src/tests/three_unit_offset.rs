//! The new three-unit proof stays separate from shared exponential ranges.
use super::*;

#[test]
fn canonical_three_unit_forms_have_one_real_formatter_and_independent_arms() {
    // These include every formerly refused three-unit boundary shape.
    for expression in [
        "entier(sign(pick()) + 3)", "entier(sign(pick()) - 3)",
        "entier((sign(pick()) + 2) + 1)", "entier(1 + (2 + sign(pick())))",
        "entier(sign(pick()) + 2 + 1)", "entier(sign(pick()) + 1 + 1 + 1)",
        "entier(entier(sign(pick()) + 1) + 2)",
        "entier(abs(sign(pick()) + 2) + 1)",
        "entier(abs((sign(pick()) + 1) + 2))",
        "entier(-(sign(pick()) + 3.0))",
    ] {
        let source = format!("begin real procedure pick; pick := -2.25; real saved; saved := {expression}; output(saved) end");
        let module = compile_source(&source, "three_unit_positive").unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(module.get_function("main").unwrap().instructions.iter()
            .filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
    }
    for condition in ["true", "false"] {
        // Separate paths may each spend three units. Their costs are not added.
        for (left, right) in [("3", "3"), ("3", "1"), ("1", "3")] {
            let source = format!("begin boolean procedure choose; choose := {condition}; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := relay(entier(if choose() then (entier(sign(left)) + {left}) else (entier(sign(right)) + {right}))); output(saved) end");
            let module = compile_source(&source, "three_unit_arms").unwrap_or_else(|e| panic!("{source}: {e}"));
            assert_eq!(module.get_function("main").unwrap().instructions.iter()
                .filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
        }
    }
}

#[test]
fn three_unit_proofs_refuse_four_units_rounding_traps_and_exp_widening() {
    for expression in [
        "entier(sign(pick()) + 17)", "entier((sign(pick()) + 8) + 9)",
        "entier(1 + (16 + sign(pick())))", "entier(entier(sign(pick()) + 1) + 16)",
        "entier(abs(sign(pick()) + 2) + 15)", "entier(sign(pick()) + 3.5)",
        "entier(sign(pick()) + sign(pick()) + 1 + 2)",
        "entier(sign(pick()) + ((9007199254740992 + 1) - 9007199254740992) + 3)",
        "entier(ln(exp(exp(exp(sign(pick()) + 3)))))",
        "entier(ln(exp(exp(exp((sign(pick()) + 1) + 2)))))",
        "entier((sign(pick()) + 3) * 1)", "entier(exp(sign(pick())) + 3)",
    ] {
        let source = format!("begin real procedure pick; pick := 3.5; real saved; saved := {expression}; output(saved) end");
        assert!(matches!(compile_source(&source, "three_unit_refusal"), Err(CompileError::Type(m)) if m.contains("cannot print a real value")), "{source}");
    }
    for name in ["sign", "abs", "entier"] {
        let source = format!("begin integer procedure {name}(x); value x; real x; {name} := 1; real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 1) + 2); output(saved) end");
        assert!(matches!(compile_source(&source, "three_unit_shadow"), Err(CompileError::Type(m)) if m.contains("cannot print a real value")), "{source}");
    }
}

#[test]
fn three_unit_conditional_alias_effects_obey_the_discovery_limit() {
    struct Guard;
    impl Drop for Guard { fn drop(&mut self) { SELECTOR_CLASSIFIER_BUDGET.with(|c| c.set(None)); } }
    for condition in ["true", "false"] { for root in ["left", "right"] { for value in [false, true] {
        let mode = if value { "value t; " } else { "" };
        let source = format!("begin boolean procedure choose(t); {mode}real t; begin t := 0.0; choose := {condition} end; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := relay(entier(if choose({root}) then (entier(sign(left)) + 3) else ((entier(sign(right)) + 1) + 2))); output(saved) end");
        SELECTOR_CLASSIFIER_BUDGET.with(|c| c.set(Some((0, 8192)))); let _guard = Guard;
        let result = compile_source(&source, "three_unit_effect");
        if value { result.unwrap_or_else(|e| panic!("{source}: {e}")); }
        else { assert!(matches!(result, Err(CompileError::Type(m)) if m.contains("cannot print a real value")), "{source}"); }
        let (visits, limit) = SELECTOR_CLASSIFIER_BUDGET.with(Cell::get).unwrap();
        assert!(visits > 0 && visits <= limit);
        eprintln!("three-unit condition={condition} root={root} value={value} visits={visits} limit={limit}");
    } } }
}
