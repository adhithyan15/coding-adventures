//! The new fourteen-unit proof stays separate from shared exponential ranges.
use super::*;

#[test]
fn canonical_fourteen_unit_forms_have_typed_output_routes_and_independent_arms() {
    // These include every formerly refused fourteen-unit boundary shape.
    for expression in [
        "entier(sign(pick()) + 13 - 1)",
        "entier(abs(entier(sign(pick()) + 13) + 1))",
        "entier(entier(abs(sign(pick()) + 13) + 1))",
        "entier(sign(pick()) + 14)",
        "entier(sign(pick()) - 14)",
        "entier((sign(pick()) + 7) + 7)",
        "entier(1 + (13 + sign(pick())))",
        "entier(sign(pick()) + 7 + 7)",
        "entier(sign(pick()) + 1 + 13)",
        "entier(sign(pick()) + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1)",
        "entier(entier(sign(pick()) + 1) + 13)",
        "entier(abs(sign(pick()) + 2) + 12)",
        "entier(abs((sign(pick()) + 1) + 13))",
        "entier(-(sign(pick()) + 14.0))",
        "entier(abs(entier(sign(pick()) + 14)))",
        "entier(entier(abs(sign(pick()) + 14)))",
    ] {
        for (statement, real_count, integer_count) in [(format!("saved := {expression}; output(saved)"),1,0),
            (format!("output({expression})"),0,1), (format!("saved := relay({expression}); output(saved)"),1,0)] {
        let source = format!("begin real procedure pick; pick := -2.25; real procedure relay(x); real x; relay := x; real saved; {statement} end");
        let module = compile_source(&source, "fourteen_unit_positive").unwrap_or_else(|e| panic!("{source}: {e}"));
        let instructions=&module.get_function("main").unwrap().instructions;
        assert_eq!(instructions.iter()
            .filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), real_count, "{source}");
        assert_eq!(instructions.iter()
            .filter(|i| i.op == "call_builtin" && i.srcs.first().and_then(Operand::as_var) == Some("print_i64")).count(), integer_count, "{source}");
        if integer_count==1 {
            let calls=instructions.iter().filter(|i|i.op=="call").collect::<Vec<_>>();
            assert_eq!(calls.len(),1,"direct output evaluates its real producer once: {source}");
            assert!(calls[0].srcs.first().and_then(Operand::as_var).is_some_and(|name|name.contains("pick")),"{source}");
        }
    }
    }
    for condition in ["true", "false"] {
        // Separate paths may each spend fourteen units. Their costs are not added.
        for (left, right) in [("14", "14"), ("14", "1"), ("1", "14")] {
            let source = format!("begin boolean procedure choose; choose := {condition}; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := relay(entier(if choose() then (entier(sign(left)) + {left}) else (entier(sign(right)) + {right}))); output(saved) end");
            let module = compile_source(&source, "fourteen_unit_arms").unwrap_or_else(|e| panic!("{source}: {e}"));
            assert_eq!(module.get_function("main").unwrap().instructions.iter()
                .filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
        }
    }
    // Direct conditional arms have the same independent budgets as relay arms.
    for condition in ["true", "false"] {
        for (left, right) in [("14", "14"), ("14", "1"), ("1", "14")] {
            let source = format!("begin boolean procedure choose; choose := {condition}; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + {left}) else (entier(sign(right)) + {right})); output(saved) end");
            let module = compile_source(&source, "fourteen_unit_direct_arms").unwrap_or_else(|e| panic!("{source}: {e}"));
            assert_eq!(module.get_function("main").unwrap().instructions.iter()
                .filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
        }
    }

}

#[test]
fn fourteen_unit_proofs_refuse_seventeen_units_rounding_traps_and_exp_widening() {
    for expression in [
        "entier(sign(pick()) + 17)",
        "entier((sign(pick()) + 8) + 9)",
        "entier(1 + (16 + sign(pick())))",
        "entier(entier(sign(pick()) + 1) + 16)",
        "entier(abs(sign(pick()) + 2) + 15)",
        "entier(sign(pick()) + 13.5)",
        "entier(sign(pick()) + 14.5)",
        "entier(sign(pick()) + sign(pick()) + 7 + 7)",
        "entier(sign(pick()) + ((9007199254740993 - 9007199254740992) + 14))",
        "entier(sign(pick()) + ((9007199254740992 + 1 - 9007199254740992) + 14))",
        "entier(ln(exp(exp(exp(sign(pick()) + 14)))))",
        "entier(ln(exp(exp(exp((sign(pick()) + 7) + 7)))))",
        "entier((sign(pick()) + 14) * 1)",
        "entier(exp(sign(pick())) + 14)",
        "entier(exp(sign(pick()) + 14))",
        "entier(sign(pick()) + 16 - 1)",
        "entier(abs(entier(sign(pick()) + 16) + 1))",
        "entier(entier(abs(sign(pick()) + 16) + 1))",
    ] {
        let source = format!("begin real procedure pick; pick := 3.5; real saved; saved := {expression}; output(saved) end");
        assert!(matches!(compile_source(&source, "fourteen_unit_refusal"), Err(CompileError::Type(m)) if m.contains("cannot print a real value")), "{source}");
    }
    for name in ["sign", "abs", "entier"] {
        let source = format!("begin integer procedure {name}(x); value x; real x; {name} := 1; real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 1) + 13); output(saved) end");
        assert!(matches!(compile_source(&source, "fourteen_unit_shadow"), Err(CompileError::Type(m)) if m.contains("cannot print a real value")), "{source}");
    }
}

#[test]
fn fourteen_unit_conditional_alias_effects_obey_the_discovery_limit() {
    struct Guard;
    impl Drop for Guard { fn drop(&mut self) { SELECTOR_CLASSIFIER_BUDGET.with(|c| c.set(None)); } }
    for condition in ["true", "false"] { for root in ["left", "right"] { for value in [false, true] {
        let mode = if value { "value t; " } else { "" };
        let source = format!("begin boolean procedure choose(t); {mode}real t; begin t := 0.0; choose := {condition} end; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := relay(entier(if choose({root}) then (entier(sign(left)) + 14) else ((entier(sign(right)) + 1) + 13))); output(saved) end");
        SELECTOR_CLASSIFIER_BUDGET.with(|c| c.set(Some((0, 8192)))); let _guard = Guard;
        let result = compile_source(&source, "fourteen_unit_effect");
        if value { result.unwrap_or_else(|e| panic!("{source}: {e}")); }
        else { assert!(matches!(result, Err(CompileError::Type(m)) if m.contains("cannot print a real value")), "{source}"); }
        let (visits, limit) = SELECTOR_CLASSIFIER_BUDGET.with(Cell::get).unwrap();
        assert!(visits > 0 && visits <= limit);
        eprintln!("fourteen-unit condition={condition} root={root} value={value} visits={visits} limit={limit}");
    } } }
}

// Both arms must prove their own budget, including an unselected oversized arm.
#[test]
fn fourteen_unit_unselected_seventeen_unit_arms_are_semantic_refusals() {
    for condition in ["true", "false"] {
        for (left, right) in [("17", "1"), ("1", "17")] {
            let source = format!("begin boolean procedure choose; choose := {condition}; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + {left}) else (entier(sign(right)) + {right})); output(saved) end");
            assert!(matches!(compile_source(&source, "fourteen_unit_oversized_arm"), Err(CompileError::Type(m)) if m.contains("cannot print a real value")), "{source}");
        }
    }
}
