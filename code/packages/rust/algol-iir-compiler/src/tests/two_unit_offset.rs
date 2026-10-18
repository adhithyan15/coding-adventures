//! New two-unit composition stays outside the old exp/log proof domain.
use super::*;

#[test]
fn canonical_two_unit_forms_keep_one_real_formatter_and_legacy_arm_forms() {
    for expression in ["entier(sign(pick()) + 2)","entier(sign(pick()) - 2)",
        "entier((sign(pick()) + 1) + 1)","entier(1 + (1 + sign(pick())))",
        "entier(abs((sign(pick()) + 1) + 1))","entier(entier(sign(pick()) + 1) + 1)",
        "entier(abs(sign(pick()) + 1) + 1)","entier(-(sign(pick()) + 2.0))"] {
        let source=format!("begin real procedure pick; pick := -2.25; real saved; saved := {expression}; output(saved) end");
        let module=compile_source(&source,"two_unit_positive").unwrap_or_else(|e|panic!("{source}: {e}"));
        assert_eq!(module.get_function("main").unwrap().instructions.iter().filter(|i|i.op=="call"&&i.srcs.first().and_then(Operand::as_var)==Some("__basic_print_real")).count(),1,"{source}");
    }
    for condition in ["true","false"] {for units in ["2","(0.5 + 0.5)"] {
        let (inner,outer)=if units=="2"{("1","1")}else{("1.0","0.0")};
        let source=format!("begin boolean procedure choose; choose := {condition}; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := relay(entier(if choose() then (entier(sign(left)) + {units}) else ((entier(sign(right)) + {inner}) + {outer}))); output(saved) end");
        compile_source(&source,"two_unit_legacy_arm").unwrap_or_else(|e|panic!("{source}: {e}"));
    }}
}

#[test]
fn two_unit_refusals_keep_exp_ranges_and_cumulative_wrapper_budget() {
    // All authored loop configurations are explicit; mixed guards stay exact.
    for (case_id, source) in [
        (323, "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 25); output(saved) end"),
        (324, "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (32 + sign(pick()))); output(saved) end"),
        (325, "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 32); output(saved) end"),
        (326, "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 31); output(saved) end"),
        (327, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 2.5); output(saved) end"),
        (328, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + sign(pick()) + 1); output(saved) end"),
        (329, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + ((9007199254740992 + 1) - 9007199254740992) + 2); output(saved) end"),
        (330, "begin real procedure pick; pick := 3.5; real saved; saved := entier(ln(exp(exp(exp(sign(pick()) + 2))))); output(saved) end"),
        (331, "begin real procedure pick; pick := 3.5; real saved; saved := entier(ln(exp(exp(exp((sign(pick()) + 1) + 1))))); output(saved) end"),
        (332, "begin integer procedure sign(x); value x; real x; sign := 1; real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 1) + 1); output(saved) end"),
        (333, "begin integer procedure abs(x); value x; real x; abs := 1; real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 1) + 1); output(saved) end"),
        (334, "begin integer procedure entier(x); value x; real x; entier := 1; real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 1) + 1); output(saved) end"),
    ] {
        assert!(matches!(compile_source(source, "cap24_refusal"), Err(CompileError::Type(message)) if message.contains("cannot print a real value")), "original case {case_id}: {source}");
        eprintln!("cap23-corpus REFUSED original-case={case_id}");
    }
}

#[test]
fn current_cap23_preserves_exact_sources_for_two_unit_refusals_keep_exp_ranges_and_cumulative_wrapper_budget() {
    // Exact former sources retain their original producer/declarations/consumer.
    for (case_id, source) in [
        (323, "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 8) + 14); output(saved) end"),
        (324, "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (21 + sign(pick()))); output(saved) end"),
        (325, "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 21); output(saved) end"),
        (326, "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 20); output(saved) end"),
    ] {
        let module = compile_source(source, "retained_original").unwrap_or_else(|error| panic!("original case {case_id}: {source}: {error:?}"));
        let instructions = &module.get_function("main").expect("main").instructions;
        assert_eq!(instructions.iter().filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "original case {case_id}: {source}");
        assert_eq!(instructions.iter().filter(|i| i.op == "call_builtin" && i.srcs.first().and_then(Operand::as_var) == Some("print_i64")).count(), 0, "original case {case_id}: {source}");
        eprintln!("cap23-corpus ADMITTED original-case={case_id}");
    }
}

#[test]
fn two_unit_conditional_fronts_keep_alias_effects_and_discovery_limit() {
    struct Guard;impl Drop for Guard {fn drop(&mut self){SELECTOR_CLASSIFIER_BUDGET.with(|c|c.set(None));}}
    for condition in ["true","false"] {for root in ["left","right"] {for value in [false,true] {
        let mode=if value{"value t; "}else{""};
        let source=format!("begin boolean procedure choose(t); {mode}real t; begin t := 0.0; choose := {condition} end; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := relay(entier(if choose({root}) then (entier(sign(left)) + 2) else ((entier(sign(right)) + 1) + 1))); output(saved) end");
        SELECTOR_CLASSIFIER_BUDGET.with(|c|c.set(Some((0,8192))));let _guard=Guard;
        let result=compile_source(&source,"two_unit_effect");
        if value {result.unwrap_or_else(|e|panic!("{source}: {e}"));}
        else {assert!(matches!(result,Err(CompileError::Type(m)) if m.contains("cannot print a real value")),"{source}");}
        let (visits,limit)=SELECTOR_CLASSIFIER_BUDGET.with(Cell::get).unwrap();assert!(visits>0&&visits<=limit);
        eprintln!("two-unit condition={condition} root={root} value={value} visits={visits} limit={limit}");
    }}}
}

// Every formerly refused full authored source stays observable, including its
// producer value/declarations. Numerical cap migration cannot erase a source.
#[test]
fn current_main_preserves_all_former_nineteen_unit_authored_sources() {
    for source in [
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
