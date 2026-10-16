//! The new eighteen-unit proof stays separate from shared exponential ranges.
use super::*;

#[test]
fn canonical_eighteen_unit_forms_have_typed_output_routes_and_independent_arms() {
    // These include every formerly refused eighteen-unit boundary shape.
    for expression in [
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
fn eighteen_unit_proofs_refuse_nineteen_units_rounding_traps_and_exp_widening() {
    for expression in [
        "entier(sign(pick()) + 19)",
        "entier((sign(pick()) + 9) + 10)",
        "entier(1 + (18 + sign(pick())))",
        "entier(entier(sign(pick()) + 1) + 18)",
        "entier(abs(sign(pick()) + 2) + 17)",
        "entier(sign(pick()) + 15.5)",
        "entier(sign(pick()) + 16.5)",
        "entier(sign(pick()) + 17.5)",
        "entier(sign(pick()) + 18.5)",
        "entier(sign(pick()) + sign(pick()) + 8 + 8)",
        "entier(sign(pick()) + ((9007199254740993 - 9007199254740992) + 16))",
        "entier(sign(pick()) + ((9007199254740992 + 1 - 9007199254740992) + 16))",
        "entier(ln(exp(exp(exp(sign(pick()) + 16)))))",
        "entier(ln(exp(exp(sign(pick()) + 16))))",
        "entier((sign(pick()) + 16) * 1)",
        "entier(exp(sign(pick())) + 16)",
        "entier(exp(sign(pick()) + 16))",
        "entier(sign(pick()) + ((9007199254740992 + 1 - 9007199254740992) + 17))",
        "entier(ln(exp(exp(exp(sign(pick()) + 17)))))",
        "entier(ln(exp(exp(sign(pick()) + 17))))",
        "entier((sign(pick()) + 17) * 1)",
        "entier(exp(sign(pick())) + 17)",
        "entier(exp(sign(pick()) + 17))",
        "entier(sign(pick()) + ((9007199254740992 + 1 - 9007199254740992) + 18))",
        "entier(ln(exp(exp(exp(sign(pick()) + 18)))))",
        "entier(ln(exp(exp(sign(pick()) + 18))))",
        "entier((sign(pick()) + 18) * 1)",
        "entier(exp(sign(pick())) + 18)",
        "entier(exp(sign(pick()) + 18))",
        "entier(sign(pick()) + 18 - 1)",
        "entier(abs(entier(sign(pick()) + 18) + 1))",
        "entier(entier(abs(sign(pick()) + 18) + 1))",
    ] {
        let source = format!("begin real procedure pick; pick := 3.5; real saved; saved := {expression}; output(saved) end");
        assert!(matches!(compile_source(&source, "eighteen_unit_refusal"), Err(CompileError::Type(m)) if m.contains("cannot print a real value")), "{source}");
    }
    for name in ["sign", "abs", "entier"] {
        let source = format!("begin integer procedure {name}(x); value x; real x; {name} := 1; real procedure pick; pick := 3.5; real saved; saved := entier(abs(sign(pick()) + 1) + 17); output(saved) end");
        assert!(matches!(compile_source(&source, "eighteen_unit_shadow"), Err(CompileError::Type(m)) if m.contains("cannot print a real value")), "{source}");
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
fn eighteen_unit_unselected_nineteen_unit_arms_are_semantic_refusals() {
    for condition in ["true", "false"] {
        for (left, right) in [("19", "1"), ("1", "19")] {
            let source = format!("begin boolean procedure choose; choose := {condition}; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := entier(if choose() then (entier(sign(left)) + {left}) else (entier(sign(right)) + {right})); output(saved) end");
            assert!(matches!(compile_source(&source, "eighteen_unit_oversized_arm"), Err(CompileError::Type(m)) if m.contains("cannot print a real value")), "{source}");
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
