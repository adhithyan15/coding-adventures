//! The new eight-unit proof stays separate from shared exponential ranges.
use super::*;

#[test]
fn canonical_eight_unit_forms_have_typed_output_routes_and_independent_arms() {
    // These include every formerly refused eight-unit boundary shape.
    for expression in [
        "entier(sign(pick()) + 8)",
        "entier(sign(pick()) - 8)",
        "entier((sign(pick()) + 4) + 4)",
        "entier(1 + (7 + sign(pick())))",
        "entier(sign(pick()) + 4 + 4)",
        "entier(sign(pick()) + 3 + 5)",
        "entier(sign(pick()) + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1)",
        "entier(entier(sign(pick()) + 1) + 7)",
        "entier(abs(sign(pick()) + 2) + 6)",
        "entier(abs((sign(pick()) + 1) + 7))",
        "entier(-(sign(pick()) + 8.0))",
    ] {
        for (statement, real_count, integer_count) in [(format!("saved := {expression}; output(saved)"),1,0),
            (format!("output({expression})"),0,1), (format!("saved := relay({expression}); output(saved)"),1,0)] {
        let source = format!("begin real procedure pick; pick := -2.25; real procedure relay(x); real x; relay := x; real saved; {statement} end");
        let module = compile_source(&source, "eight_unit_positive").unwrap_or_else(|e| panic!("{source}: {e}"));
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
        // Separate paths may each spend eight units. Their costs are not added.
        for (left, right) in [("8", "8"), ("8", "1"), ("1", "8")] {
            let source = format!("begin boolean procedure choose; choose := {condition}; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := relay(entier(if choose() then (entier(sign(left)) + {left}) else (entier(sign(right)) + {right}))); output(saved) end");
            let module = compile_source(&source, "eight_unit_arms").unwrap_or_else(|e| panic!("{source}: {e}"));
            assert_eq!(module.get_function("main").unwrap().instructions.iter()
                .filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
        }
    }
    // Direct conditional arms have the same independent budgets as relay arms.
    for condition in ["true", "false"] {
        for (left, right) in [("8", "8"), ("8", "1"), ("1", "8")] {
            let source = format!("begin boolean procedure choose; choose := {condition}; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + {left}) else (entier(sign(right)) + {right})); output(saved) end");
            let module = compile_source(&source, "eight_unit_direct_arms").unwrap_or_else(|e| panic!("{source}: {e}"));
            assert_eq!(module.get_function("main").unwrap().instructions.iter()
                .filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
        }
    }

}

#[test]
fn eight_unit_proofs_refuse_twenty_two_units_rounding_traps_and_exp_widening() {
    // All authored loop configurations are explicit; mixed guards stay exact.
    for (case_id, source) in [
        (32, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 24); output(saved) end"),
        (33, "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 16); output(saved) end"),
        (34, "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (23 + sign(pick()))); output(saved) end"),
        (35, "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 23); output(saved) end"),
        (36, "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 22); output(saved) end"),
        (37, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 8.5); output(saved) end"),
        (38, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + sign(pick()) + 3 + 5); output(saved) end"),
        (39, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + ((9007199254740992 + 1) - 9007199254740992) + 8); output(saved) end"),
        (40, "begin real procedure pick; pick := 3.5; real saved; saved := entier(ln(exp(exp(exp(sign(pick()) + 8))))); output(saved) end"),
        (41, "begin real procedure pick; pick := 3.5; real saved; saved := entier(ln(exp(exp(exp((sign(pick()) + 3) + 5))))); output(saved) end"),
        (42, "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) * 1); output(saved) end"),
        (43, "begin real procedure pick; pick := 3.5; real saved; saved := entier(exp(sign(pick())) + 8); output(saved) end"),
        (44, "begin integer procedure sign(x); value x; real x; sign := 1; real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 1) + 7); output(saved) end"),
        (45, "begin integer procedure abs(x); value x; real x; abs := 1; real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 1) + 7); output(saved) end"),
        (46, "begin integer procedure entier(x); value x; real x; entier := 1; real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 1) + 7); output(saved) end"),
    ] {
        assert!(matches!(compile_source(source, "cap24_refusal"), Err(CompileError::Type(message)) if message.contains("cannot print a real value")), "original case {case_id}: {source}");
        eprintln!("cap23-corpus REFUSED original-case={case_id}");
    }
}

#[test]
fn current_cap23_preserves_exact_sources_for_eight_unit_proofs_refuse_twenty_two_units_rounding_traps_and_exp_widening() {
    // Exact former sources retain their original producer/declarations/consumer.
    for (case_id, source) in [
        (32, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 22); output(saved) end"),
        (33, "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 14); output(saved) end"),
        (34, "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (21 + sign(pick()))); output(saved) end"),
        (35, "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 21); output(saved) end"),
        (36, "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 20); output(saved) end"),
    ] {
        let module = compile_source(source, "retained_original").unwrap_or_else(|error| panic!("original case {case_id}: {source}: {error:?}"));
        let instructions = &module.get_function("main").expect("main").instructions;
        assert_eq!(instructions.iter().filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "original case {case_id}: {source}");
        assert_eq!(instructions.iter().filter(|i| i.op == "call_builtin" && i.srcs.first().and_then(Operand::as_var) == Some("print_i64")).count(), 0, "original case {case_id}: {source}");
        eprintln!("cap23-corpus ADMITTED original-case={case_id}");
    }
}

#[test]
fn eight_unit_conditional_alias_effects_obey_the_discovery_limit() {
    struct Guard;
    impl Drop for Guard { fn drop(&mut self) { SELECTOR_CLASSIFIER_BUDGET.with(|c| c.set(None)); } }
    for condition in ["true", "false"] { for root in ["left", "right"] { for value in [false, true] {
        let mode = if value { "value t; " } else { "" };
        let source = format!("begin boolean procedure choose(t); {mode}real t; begin t := 0.0; choose := {condition} end; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := relay(entier(if choose({root}) then (entier(sign(left)) + 8) else ((entier(sign(right)) + 1) + 7))); output(saved) end");
        SELECTOR_CLASSIFIER_BUDGET.with(|c| c.set(Some((0, 8192)))); let _guard = Guard;
        let result = compile_source(&source, "eight_unit_effect");
        if value { result.unwrap_or_else(|e| panic!("{source}: {e}")); }
        else { assert!(matches!(result, Err(CompileError::Type(m)) if m.contains("cannot print a real value")), "{source}"); }
        let (visits, limit) = SELECTOR_CLASSIFIER_BUDGET.with(Cell::get).unwrap();
        assert!(visits > 0 && visits <= limit);
        eprintln!("eight-unit condition={condition} root={root} value={value} visits={visits} limit={limit}");
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
