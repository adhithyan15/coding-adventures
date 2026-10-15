//! Public compiler oracles for isolating the one-unit outer-entier slice.
use super::*;

// This oracle examines the semantic call boundary, not the spelling of a
// classifier token: only a sole read of the name actual can lose its thunk.
#[test]
fn conditional_identity_specialization_keeps_nonidentity_calls() {
    for (heading, body, specialized) in [
        ("real procedure relay(x); real x;", "relay := x", true),
        ("real procedure relay(x); real x;", "relay := x + x", false),
        ("real procedure relay(x); real x;", "relay := -x", false),
        ("real procedure relay(x); real x;", "begin relay := x end", false),
        ("real procedure relay(x); real x;", "begin x := 0.0; relay := x end", false),
        ("real procedure relay(x); real x;", "begin relay := x; x := 0.0 end", false),
        ("real procedure relay(x); real x;", "labelled: relay := x", false),
        ("real procedure relay(x); value x; real x;", "relay := x", false),
        ("integer procedure relay(x); real x;", "relay := entier(x)", false),
    ] {
        let source = format!("begin boolean procedure choose; choose := true; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; {heading} {body}; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := relay(entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 1))) end");
        let result = compile_source(&source, "identity_boundary");
        if body.contains("x := 0.0") {
            assert!(matches!(result, Err(CompileError::Type(message)) if message.contains("requires an assignable variable actual")), "write must retain ordinary name-actual validation: {source}");
            continue;
        }
        let module = result.unwrap_or_else(|e| panic!("{source}: {e}"));
        let relay_calls = module.get_function("main").unwrap().instructions.iter().filter(|i| {
            i.op == "call" && i.srcs.first().and_then(Operand::as_var)
                .is_some_and(|name| name == "relay" || name.starts_with("__algol_by_name_relay_"))
        }).count();
        assert_eq!(relay_calls, usize::from(!specialized), "{source}");
    }
}

#[test]
fn identity_declaration_guard_checks_exact_tree_signature_and_resolved_identity() {
    fn declaration(node: &GrammarASTNode) -> &GrammarASTNode {
        if node.rule_name == "procedure_decl" { return node; }
        direct_nodes(node).into_iter().find_map(|child| {
            fn find(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
                if node.rule_name == "procedure_decl" { return Some(node); }
                direct_nodes(node).into_iter().find_map(find)
            }
            find(child)
        }).expect("procedure declaration")
    }
    let ast = parse_algol("begin real procedure relay(x); real x; relay := x; real saved; saved := 0.0 end");
    let decl = declaration(&ast).clone();
    let mut compiler = Compiler::default();
    compiler.register_proc_sig(&decl).unwrap();
    let identity = compiler.resolve_procedure_identity("relay");
    let sig = compiler.proc_sigs[&identity].clone();
    assert!(compiler.is_read_once_real_name_identity(&identity, &sig), "{decl:#?}");
    fn find_mut<'a>(node: &'a mut GrammarASTNode, rule: &str) -> Option<&'a mut GrammarASTNode> {
        if node.rule_name == rule { return Some(node); }
        node.children.iter_mut().find_map(|child| match child {
            ASTNodeOrToken::Node(child) => find_mut(child, rule), _ => None,
        })
    }
    for mutation in 0..9 {
        let mut damaged = decl.clone();
        match mutation {
            0 => damaged.children.push(ASTNodeOrToken::Node(first_direct_node(&decl, "proc_body").unwrap().clone())),
            1 => {
                let ASTNodeOrToken::Node(body) = damaged.children.iter_mut().find(|child| matches!(child,ASTNodeOrToken::Node(n) if n.rule_name=="proc_body")).unwrap() else { unreachable!() };
                body.children.push(body.children[0].clone());
            }
            2 => damaged.rule_name = "not_a_procedure".into(),
            3 => {
                let ASTNodeOrToken::Node(body) = damaged.children.iter_mut().find(|child| matches!(child,ASTNodeOrToken::Node(n) if n.rule_name=="proc_body")).unwrap() else { unreachable!() };
                body.children.clear();
            }
            4 => damaged.children.retain(|child| !matches!(child,ASTNodeOrToken::Node(n) if n.rule_name=="formal_params")),
            5 | 6 => find_mut(&mut damaged, "expr_eqv").unwrap().rule_name =
                if mutation == 5 { "proc_call" } else { "unknown_wrapper" }.into(),
            7 => {
                let variable = find_mut(&mut damaged, "expression").and_then(|rhs| find_mut(rhs, "variable")).unwrap();
                variable.children.push(variable.children[0].clone());
            }
            _ => {
                let wrapper = find_mut(&mut damaged, "expr_eqv").unwrap();
                let ASTNodeOrToken::Node(child) = wrapper.children.remove(0) else { unreachable!() };
                *wrapper = child;
            }
        }
        compiler.proc_decls.insert(identity.clone(), damaged);
        assert!(!compiler.is_read_once_real_name_identity(&identity, &sig), "damaged tree {mutation}");
    }
    compiler.proc_decls.insert(identity.clone(), decl);
    let mut wrong_sig = sig.clone(); wrong_sig.ret = Some(ScalarType::Integer);
    assert!(!compiler.is_read_once_real_name_identity(&identity, &wrong_sig));
    wrong_sig = sig.clone(); wrong_sig.params[0].mode = ProcedureParamMode::Value;
    assert!(!compiler.is_read_once_real_name_identity(&identity, &wrong_sig));
    compiler.proc_scope_names.push(HashSet::new());
    let other = parse_algol("begin real procedure relay(x); real x; relay := x + x; real saved; saved := 0.0 end");
    compiler.register_proc_sig(declaration(&other)).unwrap();
    let shadow = compiler.resolve_procedure_identity("relay");
    assert_ne!(shadow, identity);
    assert!(!compiler.is_read_once_real_name_identity(&shadow, &compiler.proc_sigs[&shadow]));
    assert!(compiler.is_read_once_real_name_identity(&identity, &sig));
}

#[test]
fn nested_offsets_and_shifted_narrow_range_proofs_have_exact_typed_refusals() {
    let mut failures = Vec::new();
    for (name, expression) in [
        ("nested-right", "entier((sign(pick()) + 3) + 2)"),
        ("nested-left", "entier(1 + (4 + sign(pick())))"),
        ("entier-wrapped", "entier(entier(sign(pick()) + 1) + 4)"),
        ("abs-wrapped", "entier(abs(sign(pick()) + 2) + 3)"),
        ("shifted-triple-exp", "entier(ln(exp(exp(exp(sign(pick()) + 1)))))"),
        ("flat-five", "entier(sign(pick()) + 3 + 2)"),
        ("non-unit", "entier(sign(pick()) + 5)"),
    ] {
        let source = format!("begin real procedure pick; pick := 3.5; real saved; saved := {expression}; output(saved) end");
        match compile_source(&source, "unit_offset_refusal") {
            Err(CompileError::Type(message)) if message.contains("cannot print a real value") => {}
            Ok(_) => failures.push((name, "unexpectedly accepted".to_owned())),
            Err(error) => failures.push((name, format!("wrong refusal: {error:?}"))),
        }
    }
    assert!(failures.is_empty(), "exact Type/formatter refusal failures: {failures:?}");
}

#[test]
fn flat_units_and_original_zero_exp_keep_real_formatter_admission() {
    for expression in [
        "entier(sign(pick()) + 1)",
        "entier(sign(pick()) - 1)",
        "entier(1 + sign(pick()))",
        "entier(1 - sign(pick()))",
        "entier((entier(sign(pick())) + 1))",
        "entier(-(sign(pick()) + 1))",
        "entier(ln(exp(exp(exp(sign(pick()) + 0)))))",
    ] {
        let source = format!("begin real procedure pick; pick := 3.5; real saved; saved := {expression}; output(saved) end");
        let module = compile_source(&source, "unit_offset_preservation").unwrap_or_else(|e| panic!("{source}: {e}"));
        let main = module.get_function("main").unwrap();
        assert!(main.instructions.iter().any(|i| i.op=="call" && i.srcs.first().and_then(Operand::as_var)==Some("__basic_print_real")), "{source}");
    }
}

#[test]
fn variable_terms_and_overridden_sign_keep_exact_typed_refusals() {
    for source in [
        "begin real procedure pick; pick := 3.5; real unit, saved; unit := 1.0; saved := entier(sign(pick()) + unit); output(saved) end",
        "begin integer procedure sign(x); value x; real x; sign := 1; real procedure pick; pick := 3.5; real saved; saved := entier(sign(pick()) + 1); output(saved) end",
    ] {
        assert!(matches!(compile_source(source,"unit_offset_identity"),Err(CompileError::Type(message)) if message.contains("cannot print a real value")), "{source}");
    }
}

#[test]
fn conditional_unit_fronts_preserve_both_branches_with_bounded_discovery() {
    struct Guard;
    impl Drop for Guard {fn drop(&mut self) {SELECTOR_CLASSIFIER_BUDGET.with(|c|c.set(None));}}
    for condition in ["true","false"] {
        for wrappers in 0..=2 {
            let expression=format!("{}if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 1){}","(".repeat(wrappers),")".repeat(wrappers));
            let source=format!("begin boolean procedure choose; choose := {condition}; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := relay(entier({expression})); output(saved) end");
            SELECTOR_CLASSIFIER_BUDGET.with(|c|c.set(Some((0,8192))));let _guard=Guard;
            let module=compile_source(&source,"conditional_unit_front").unwrap_or_else(|e|panic!("condition={condition} wrappers={wrappers}: {e}"));
            let main=module.get_function("main").unwrap();
            assert_eq!(main.instructions.iter().filter(|i|i.op=="call" && i.srcs.first().and_then(Operand::as_var)==Some("__basic_print_real")).count(),1);
            let (visits,limit)=SELECTOR_CLASSIFIER_BUDGET.with(Cell::get).unwrap();
            assert!(visits>0 && visits<=limit);
            eprintln!("conditional unit condition={condition} wrappers={wrappers} visits={visits} limit={limit}");
        }
    }
}

#[test]
fn conditional_front_second_and_nonunit_offsets_keep_typed_refusals() {
    for condition in ["true","false"] {
        for expression in [
            "entier((if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 1)) + 1)",
            "entier(if choose() then (entier(sign(left)) + 4) else (entier(sign(right)) + 1))",
            "entier(if choose() then (entier(sign(left)) + 1) else (entier(sign(right)) + 4))",
        ] {
            let source=format!("begin boolean procedure choose; choose := {condition}; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real left, right, unit, saved; left := leftvalue(); right := rightvalue(); unit := 1.0; saved := {expression}; output(saved) end");
            assert!(matches!(compile_source(&source,"conditional_unit_refusal"),Err(CompileError::Type(message)) if message.contains("cannot print a real value")),"condition={condition}: {source}");
        }
    }
}

#[test]
fn conditional_front_condition_name_writes_refuse_and_value_writes_preserve_proof() {
    for root in ["left","right"] {
        for condition in ["true","false"] {
            for value_parameter in [false,true] {
                let mode=if value_parameter {"value t; "} else {""};
                let source=format!("begin boolean procedure choose(t); {mode}real t; begin t := 0.0; choose := {condition} end; real procedure leftvalue; leftvalue := -2.25; real procedure rightvalue; rightvalue := 3.5; real procedure relay(x); real x; relay := x; real left, right, saved; left := leftvalue(); right := rightvalue(); saved := relay(entier(if choose({root}) then (entier(sign(left)) + 1) else (entier(sign(right)) + 1))); output(saved) end");
                let result=compile_source(&source,"conditional_unit_effect");
                if value_parameter {
                    let module=result.unwrap_or_else(|e|panic!("value condition root={root} result={condition}: {e}"));
                    assert_eq!(module.get_function("main").unwrap().instructions.iter().filter(|i|i.op=="call" && i.srcs.first().and_then(Operand::as_var)==Some("__basic_print_real")).count(),1);
                } else {
                    assert!(matches!(result,Err(CompileError::Type(message)) if message.contains("cannot print a real value")),"name condition root={root} result={condition}: {source}");
                }
            }
        }
    }
}
