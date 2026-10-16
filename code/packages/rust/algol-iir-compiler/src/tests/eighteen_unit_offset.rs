//! The new eighteen-unit proof stays separate from shared exponential ranges.
use super::*;

#[test]
fn canonical_eighteen_unit_forms_have_typed_output_routes_and_independent_arms() {
    // These include every formerly refused eighteen-unit boundary shape.
    for expression in [
        // Former absolute-budget nineteen refusals become exact positives.
        "entier(sign(pick()) + 19)",
        "entier((sign(pick()) + 9) + 10)",
        "entier(1 + (18 + sign(pick())))",
        "entier(entier(sign(pick()) + 1) + 18)",
        "entier(abs(sign(pick()) + 2) + 17)",
        "entier(sign(pick()) + 18 - 1)",
        "entier(abs(entier(sign(pick()) + 18) + 1))",
        "entier(entier(abs(sign(pick()) + 18) + 1))",
        "entier(sign(pick()) + 17)",
        "entier(sign(pick()) - 17)",
        "entier((sign(pick()) + 8) + 9)",
        "entier(1 + (16 + sign(pick())))",
        "entier(sign(pick()) + 8 + 9)",
        "entier(sign(pick()) + 1 + 16)",
        "entier(sign(pick()) + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1)",
        "entier(entier(sign(pick()) + 1) + 16)",
        "entier(abs(sign(pick()) + 2) + 15)",
        "entier(abs((sign(pick()) + 1) + 16))",
        "entier(-(sign(pick()) + 17.0))",
        "entier(abs(entier(sign(pick()) + 17)))",
        "entier(entier(abs(sign(pick()) + 17)))",
        "entier(sign(pick()) + 18)",
        "entier(sign(pick()) - 18)",
        "entier((sign(pick()) + 8) + 10)",
        "entier(1 + (17 + sign(pick())))",
        "entier(sign(pick()) + 8 + 10)",
        "entier(sign(pick()) + 1 + 17)",
        "entier(sign(pick()) + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1)",
        "entier(entier(sign(pick()) + 1) + 17)",
        "entier(abs(sign(pick()) + 2) + 16)",
        "entier(abs((sign(pick()) + 1) + 17))",
        "entier(-(sign(pick()) + 18.0))",
        "entier(abs(entier(sign(pick()) + 18)))",
        "entier(entier(abs(sign(pick()) + 18)))",
        "entier(sign(pick()) + 16 - 1)",
        "entier(abs(entier(sign(pick()) + 16) + 1))",
        "entier(entier(abs(sign(pick()) + 16) + 1))",
        "entier(sign(pick()) + 17 - 1)",
        "entier(abs(entier(sign(pick()) + 17) + 1))",
        "entier(entier(abs(sign(pick()) + 17) + 1))",
    ] {
        for (statement, real_count, integer_count) in [(format!("saved := {expression}; output(saved)"),1,0),
            (format!("output({expression})"),0,1), (format!("saved := relay({expression}); output(saved)"),1,0)] {
        let source = format!("begin real procedure pick; pick := -2.25; real procedure relay(x); real x; relay := x; real saved; {statement} end");
        let module = compile_source(&source, "eighteen_unit_positive").unwrap_or_else(|e| panic!("{source}: {e}"));
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
        // Separate paths may each spend eighteen units. Their costs are not added.
        for (left, right) in [("17", "18"), ("18", "17"), ("18", "18"), ("18", "1"), ("1", "18")] {
            let source = format!("begin boolean procedure choose; choose := {condition}; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := relay(entier(if choose() then (entier(sign(left)) + {left}) else (entier(sign(right)) + {right}))); output(saved) end");
            let module = compile_source(&source, "eighteen_unit_arms").unwrap_or_else(|e| panic!("{source}: {e}"));
            assert_eq!(module.get_function("main").unwrap().instructions.iter()
                .filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
        }
    }
    // Direct conditional arms have the same independent budgets as relay arms.
    for condition in ["true", "false"] {
        for (left, right) in [("17", "18"), ("18", "17"), ("18", "18"), ("18", "1"), ("1", "18")] {
            let source = format!("begin boolean procedure choose; choose := {condition}; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + {left}) else (entier(sign(right)) + {right})); output(saved) end");
            let module = compile_source(&source, "eighteen_unit_direct_arms").unwrap_or_else(|e| panic!("{source}: {e}"));
            assert_eq!(module.get_function("main").unwrap().instructions.iter()
                .filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "{source}");
        }
    }

}

#[test]
fn eighteen_unit_proofs_refuse_twenty_two_units_rounding_traps_and_exp_widening() {
    // All authored loop configurations are explicit; mixed guards stay exact.
    for (case_id, source) in [
        (47, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 24); output(saved) end"),
        (48, "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 11) + 13); output(saved) end"),
        (49, "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (23 + sign(pick()))); output(saved) end"),
        (50, "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 23); output(saved) end"),
        (51, "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 22); output(saved) end"),
        (52, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 15.5); output(saved) end"),
        (53, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 16.5); output(saved) end"),
        (54, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 17.5); output(saved) end"),
        (55, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 18.5); output(saved) end"),
        (56, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + sign(pick()) + 8 + 8); output(saved) end"),
        (57, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + ((9007199254740993 - 9007199254740992) + 16)); output(saved) end"),
        (58, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + ((9007199254740992 + 1 - 9007199254740992) + 16)); output(saved) end"),
        (59, "begin real procedure pick; pick := 3.5; real saved; saved := entier(ln(exp(exp(exp(sign(pick()) + 16))))); output(saved) end"),
        (60, "begin real procedure pick; pick := 3.5; real saved; saved := entier(ln(exp(exp(sign(pick()) + 16)))); output(saved) end"),
        (61, "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 16) * 1); output(saved) end"),
        (62, "begin real procedure pick; pick := 3.5; real saved; saved := entier(exp(sign(pick())) + 16); output(saved) end"),
        (63, "begin real procedure pick; pick := 3.5; real saved; saved := entier(exp(sign(pick()) + 16)); output(saved) end"),
        (64, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + ((9007199254740992 + 1 - 9007199254740992) + 17)); output(saved) end"),
        (65, "begin real procedure pick; pick := 3.5; real saved; saved := entier(ln(exp(exp(exp(sign(pick()) + 17))))); output(saved) end"),
        (66, "begin real procedure pick; pick := 3.5; real saved; saved := entier(ln(exp(exp(sign(pick()) + 17)))); output(saved) end"),
        (67, "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 17) * 1); output(saved) end"),
        (68, "begin real procedure pick; pick := 3.5; real saved; saved := entier(exp(sign(pick())) + 17); output(saved) end"),
        (69, "begin real procedure pick; pick := 3.5; real saved; saved := entier(exp(sign(pick()) + 17)); output(saved) end"),
        (70, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + ((9007199254740992 + 1 - 9007199254740992) + 18)); output(saved) end"),
        (71, "begin real procedure pick; pick := 3.5; real saved; saved := entier(ln(exp(exp(exp(sign(pick()) + 18))))); output(saved) end"),
        (72, "begin real procedure pick; pick := 3.5; real saved; saved := entier(ln(exp(exp(sign(pick()) + 18)))); output(saved) end"),
        (73, "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 18) * 1); output(saved) end"),
        (74, "begin real procedure pick; pick := 3.5; real saved; saved := entier(exp(sign(pick())) + 18); output(saved) end"),
        (75, "begin real procedure pick; pick := 3.5; real saved; saved := entier(exp(sign(pick()) + 18)); output(saved) end"),
        (76, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 23 - 1); output(saved) end"),
        (77, "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(entier(sign(pick()) + 23) + 1)); output(saved) end"),
        (78, "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(abs(sign(pick()) + 23) + 1)); output(saved) end"),
        (79, "begin integer procedure sign(x); value x; real x; sign := 1; real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 1) + 17); output(saved) end"),
        (80, "begin integer procedure abs(x); value x; real x; abs := 1; real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 1) + 17); output(saved) end"),
        (81, "begin integer procedure entier(x); value x; real x; entier := 1; real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 1) + 17); output(saved) end"),
    ] {
        assert!(matches!(compile_source(source, "cap24_refusal"), Err(CompileError::Type(message)) if message.contains("cannot print a real value")), "original case {case_id}: {source}");
        eprintln!("cap23-corpus REFUSED original-case={case_id}");
    }
}

#[test]
fn current_cap23_preserves_exact_sources_for_eighteen_unit_proofs_refuse_twenty_two_units_rounding_traps_and_exp_widening() {
    // Exact former sources retain their original producer/declarations/consumer.
    for (case_id, source) in [
        (47, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 22); output(saved) end"),
        (48, "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 11) + 11); output(saved) end"),
        (49, "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (21 + sign(pick()))); output(saved) end"),
        (50, "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 21); output(saved) end"),
        (51, "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 20); output(saved) end"),
        (76, "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 21 - 1); output(saved) end"),
        (77, "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(entier(sign(pick()) + 21) + 1)); output(saved) end"),
        (78, "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(abs(sign(pick()) + 21) + 1)); output(saved) end"),
    ] {
        let module = compile_source(source, "retained_original").unwrap_or_else(|error| panic!("original case {case_id}: {source}: {error:?}"));
        let instructions = &module.get_function("main").expect("main").instructions;
        assert_eq!(instructions.iter().filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "original case {case_id}: {source}");
        assert_eq!(instructions.iter().filter(|i| i.op == "call_builtin" && i.srcs.first().and_then(Operand::as_var) == Some("print_i64")).count(), 0, "original case {case_id}: {source}");
        eprintln!("cap23-corpus ADMITTED original-case={case_id}");
    }
}

#[test]
fn eighteen_unit_conditional_alias_effects_obey_the_discovery_limit() {
    struct Guard;
    impl Drop for Guard { fn drop(&mut self) { SELECTOR_CLASSIFIER_BUDGET.with(|c| c.set(None)); } }
    for condition in ["true", "false"] { for root in ["left", "right"] { for value in [false, true] {
        let mode = if value { "value t; " } else { "" };
        let source = format!("begin boolean procedure choose(t); {mode}real t; begin t := 0.0; choose := {condition} end; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := relay(entier(if choose({root}) then (entier(sign(left)) + 18) else ((entier(sign(right)) + 1) + 17))); output(saved) end");
        SELECTOR_CLASSIFIER_BUDGET.with(|c| c.set(Some((0, 8192)))); let _guard = Guard;
        let result = compile_source(&source, "eighteen_unit_effect");
        if value { result.unwrap_or_else(|e| panic!("{source}: {e}")); }
        else { assert!(matches!(result, Err(CompileError::Type(m)) if m.contains("cannot print a real value")), "{source}"); }
        let (visits, limit) = SELECTOR_CLASSIFIER_BUDGET.with(Cell::get).unwrap();
        assert!(visits > 0 && visits <= limit);
        eprintln!("eighteen-unit condition={condition} root={root} value={value} visits={visits} limit={limit}");
    } } }
}

// Both arms must prove their own budget, including an unselected oversized arm.
#[test]
fn twenty_one_unit_unselected_twenty_two_unit_arms_are_semantic_refusals() {
    // All authored loop configurations are explicit; mixed guards stay exact.
    for (case_id, source) in [
        (82, "begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 24) else (entier(sign(right)) + 1)); output(saved) end"),
        (83, "begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 24)); output(saved) end"),
        (84, "begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 24) else (entier(sign(right)) + 1)); output(saved) end"),
        (85, "begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 24)); output(saved) end"),
    ] {
        assert!(matches!(compile_source(source, "cap24_refusal"), Err(CompileError::Type(message)) if message.contains("cannot print a real value")), "original case {case_id}: {source}");
        eprintln!("cap23-corpus REFUSED original-case={case_id}");
    }
}

#[test]
fn current_cap23_preserves_exact_sources_for_twenty_one_unit_unselected_twenty_two_unit_arms_are_semantic_refusals() {
    // Exact former sources retain their original producer/declarations/consumer.
    for (case_id, source) in [
        (82, "begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 22) else (entier(sign(right)) + 1)); output(saved) end"),
        (83, "begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 22)); output(saved) end"),
        (84, "begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 22) else (entier(sign(right)) + 1)); output(saved) end"),
        (85, "begin boolean procedure choose; choose := false; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 22)); output(saved) end"),
    ] {
        let module = compile_source(source, "retained_original").unwrap_or_else(|error| panic!("original case {case_id}: {source}: {error:?}"));
        let instructions = &module.get_function("main").expect("main").instructions;
        assert_eq!(instructions.iter().filter(|i| i.op == "call" && i.srcs.first().and_then(Operand::as_var) == Some("__basic_print_real")).count(), 1, "original case {case_id}: {source}");
        assert_eq!(instructions.iter().filter(|i| i.op == "call_builtin" && i.srcs.first().and_then(Operand::as_var) == Some("print_i64")).count(), 0, "original case {case_id}: {source}");
        eprintln!("cap23-corpus ADMITTED original-case={case_id}");
    }
}

// Preserve every exact former nineteen-unit refusal as a typed-real positive.
#[test]
fn former_nineteen_unit_conditional_arm_refusals_are_exact_positives() {
    for condition in ["true", "false"] {
        for (left, right) in [("19", "1"), ("1", "19")] {
            let source = format!("begin boolean procedure choose; choose := {condition}; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + {left}) else (entier(sign(right)) + {right})); output(saved) end");
            let module=compile_source(&source,"former_nineteen_unit_arm").unwrap_or_else(|e|panic!("{source}: {e}"));
            assert_eq!(module.get_function("main").unwrap().instructions.iter().filter(|i|i.op=="call"&&i.srcs.first().and_then(Operand::as_var)==Some("__basic_print_real")).count(),1,"{source}");
        }
    }
}

/// The independent16 control supplies complete IIR shape. Equal-width literal
/// spellings retain locations; only declared authored leaf operands may change.
#[test]
fn native18_complete_serialized_modules_preserve_lineage_except_authored_offset_leaves() {
    // Exhaustive patterns deliberately have no rest: a future IR field needs
    // an explicit assertion. The wire serializer omits these source/side tables.
    fn complete_metadata(actual:&IIRModule,expected:&IIRModule) {
        let IIRModule{name:an,functions:af,entry_point:ae,language:al,exports:ax,imports:ai}=actual;
        let IIRModule{name:en,functions:ef,entry_point:ee,language:el,exports:ex,imports:ei}=expected;
        assert_eq!((an,ae,al,ax,ai),(en,ee,el,ex,ei),"complete module metadata");
        assert_eq!(af.len(),ef.len(),"ordered function count");
        let main=actual.get_function("main").expect("source main");
        assert_eq!(main.source_map.len(),main.instructions.len(),"AST main source-map lockstep");
        assert!(main.source_map.iter().any(|loc|loc.line>0&&loc.column>0),"real main source positions must be observable");
        for (a,e) in af.iter().zip(ef) {
            let IIRFunction{name:an,params:ap,return_type:ar,instructions:ai,register_count:ac,type_status:at,call_count:acc,feedback_slots:afs,source_map:asm,param_refinements:apr,return_refinement:arr}=a;
            let IIRFunction{name:en,params:ep,return_type:er,instructions:ei,register_count:ec,type_status:et,call_count:ecc,feedback_slots:efs,source_map:esm,param_refinements:epr,return_refinement:err}=e;
            assert_eq!((an,ap,ar,ac,at,acc,afs,asm,apr,arr),(en,ep,er,ec,et,ecc,efs,esm,epr,err),"complete function metadata");
            assert!(asm.is_empty()||asm.len()==ai.len(),"nonempty retained source maps stay in lockstep");
            assert_eq!(ai.len(),ei.len(),"ordered instruction count");
            for (a,e) in ai.iter().zip(ei) {
                let IIRInstr{op:ao,dest:ad,srcs:asrc,type_hint:at,may_alloc:am,observed_slot:aos,observed_type:aot,observation_count:aoc,deopt_anchor:ada,ic_slot:aic}=a;
                let IIRInstr{op:eo,dest:ed,srcs:esrc,type_hint:et,may_alloc:em,observed_slot:eos,observed_type:eot,observation_count:eoc,deopt_anchor:eda,ic_slot:eic}=e;
                assert_eq!((ao,ad,asrc,at,am,aot,aoc,ada,aic),(eo,ed,esrc,et,em,eot,eoc,eda,eic),"complete instruction metadata and operands");
                assert!(aos.is_none()&&eos.is_none(),"freshly compiled instructions have no observed profiling slots");
            }
        }
    }
    // The former control is deliberately refused: literal assignments do not
    // establish the retained runtime-real root proof, even at offset16.
    let literal_root_control = "begin boolean procedure choose; choose := true; real procedure pick; pick := -2.25; real procedure relay(x); real x; relay := x; real left, right, saved; left := -2.25; right := 3.5; saved := relay(entier(if choose() then (entier(sign(left)) + 16) else (entier(sign(right)) + 16))); output(saved) end";
    assert!(matches!(compile_source(literal_root_control,"native18_lineage"),Err(CompileError::Type(m)) if m.contains("cannot print a real value")),"literal-root offset16 remains refused");
    for amount in [17_i64,18] {
        for (case,(template,leaves)) in [
            ("saved := entier(sign(pick()) + OFFSET); output(saved)",1),
            ("output(entier(sign(pick()) + OFFSET))",1),
            ("saved := relay(entier(sign(pick()) + OFFSET)); output(saved)",1),
            ("saved := entier(abs(entier(sign(pick()) + OFFSET))); output(saved)",1),
            ("saved := relay(entier(if choose() then (entier(sign(left)) + OFFSET) else (entier(sign(right)) + OFFSET))); output(saved)",2),
        ].into_iter().enumerate() {
            let source=|offset:&str| format!("begin boolean procedure choose; choose := true; real procedure pick; pick := -2.25; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); {} end",template.replace("OFFSET",offset));
            let mut expected=compile_source(&source("16"),"native18_lineage").unwrap_or_else(|e|panic!("accepted16 complete control case={case} {template}: {e}"));
            let mut changed=0;
            for function in &mut expected.functions {for instruction in &mut function.instructions {for operand in &mut instruction.srcs {
                match operand {
                    Operand::Int(value) if *value==16=>{*value=amount;changed+=1;},
                    Operand::Float(value) if *value==16.0=>{*value=amount as f64;changed+=1;},
                    _=>{},
                }
            }}}
            assert_eq!(changed,leaves,"independent authored leaf inventory: {template}");
            let actual=compile_source(&source(&amount.to_string()),"native18_lineage").unwrap_or_else(|e|panic!("{amount} {template}: {e}"));
            assert_eq!(interpreter_ir::serialise::serialise(&actual),interpreter_ir::serialise::serialise(&expected),"whole wire module instructions/types {amount} {template}");
            complete_metadata(&actual,&expected);
            eprintln!("NATIVE18-IIR-COMPLETE offset={amount} case={case} fullwire/metadata/source-map");
        }
    }
}

// The former negative contexts use pick:=3.5 and no relay declaration. Keep
// them in addition to the existing negative-input, direct and relay routes.
#[test]
fn current_main_preserves_exact_eight_eighteenth_boundary_source_contexts() {
    for source in [
        "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 19); output(saved) end",
        "begin real procedure pick; pick := 3.5; real saved; saved := entier((sign(pick()) + 9) + 10); output(saved) end",
        "begin real procedure pick; pick := 3.5; real saved; saved := entier(1 + (18 + sign(pick()))); output(saved) end",
        "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(sign(pick()) + 1) + 18); output(saved) end",
        "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 2) + 17); output(saved) end",
        "begin real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 18 - 1); output(saved) end",
        "begin real procedure pick; pick := 3.5; real saved; saved := entier(abs(entier(sign(pick()) + 18) + 1)); output(saved) end",
        "begin real procedure pick; pick := 3.5; real saved; saved := entier(entier(abs(sign(pick()) + 18) + 1)); output(saved) end",

    ] {
        let module=compile_source(source,"former_eighteenth_context").unwrap_or_else(|e|panic!("{source}: {e}"));
        let instructions=&module.get_function("main").unwrap().instructions;
        assert_eq!(instructions.iter().filter(|i|i.op=="call"&&i.srcs.first().and_then(Operand::as_var)==Some("__basic_print_real")).count(),1,"{source}");
        assert_eq!(instructions.iter().filter(|i|i.op=="call_builtin"&&i.srcs.first().and_then(Operand::as_var)==Some("print_i64")).count(),0,"{source}");
    }
}
