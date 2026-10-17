//! CLOC36 starts from the returned Program, not a convenient journal record.
use super::*;
use coding_adventures_correlation_vector::GraphLimits;
use serde_json::{json, Value};

#[path = "selected_subtree_frontier.rs"]
mod selected_subtree_frontier;

#[path = "literal_conditional_root.rs"]
mod literal_conditional_root;

#[path = "equal_conditional_frontier.rs"]
mod equal_conditional_frontier;

// Test-only wire fixtures independently enumerate the actual AST union. No
// production visitor/serialization is added to discover or stamp root CVs.
fn shapes(cv: Option<&str>) -> Vec<(String, Expression)> {
    let x = json!({"type":"Identifier","name":"x"});
    let y = json!({"type":"Identifier","name":"y"});
    let call = json!({"type":"CallExpression","callee":x,"arguments":[]});
    let member =
        json!({"type":"OptionalMemberExpression","object":x,"property":y,"computed":false});
    let template = json!({"quasis":[{"raw":"a","cooked":"a","tail":false},{"raw":"b","cooked":"b","tail":true}],"expressions":[x]});
    let mut values = vec![
        json!({"type":"Identifier","name":"slot"}),
        json!({"type":"NumericLiteral","value":-0.0,"raw":"-0"}),
        json!({"type":"StringLiteral","value":"😀","raw":"\"\\uD83D\\uDE00\""}),
        json!({"type":"BooleanLiteral","value":false}),
        json!({"type":"NullLiteral"}),
        json!({"type":"BigIntLiteral","value":"123","raw":"123n"}),
        json!({"type":"UndefinedLiteral"}),
        json!({"type":"RegExpLiteral","pattern":"ab+","flags":"gi"}),
        json!({"type":"BinaryExpression","operator":"+","left":x,"right":y}),
        json!({"type":"LogicalExpression","operator":"&&","left":x,"right":y}),
        json!({"type":"UnaryExpression","operator":"delete","prefix":true,"argument":x}),
        json!({"type":"AssignmentExpression","operator":"=","left":{"name":"slot"},"right":y}),
        json!({"type":"ConditionalExpression","test":x,"consequent":x,"alternate":y}),
        call.clone(),
        json!({"type":"MemberExpression","object":x,"property":y,"computed":false}),
        member.clone(),
        json!({"type":"OptionalCallExpression","callee":x,"arguments":[]}),
        json!({"type":"ChainExpression","expression":member}),
        json!({"type":"ArrayExpression","elements":[x]}),
        json!({"type":"ObjectExpression","properties":[]}),
        json!({"type":"FunctionExpression","params":[],"body":{"body":[]},"generator":false,"async":false}),
        json!({"type":"ArrowFunctionExpression","params":[],"body":x,"async":false}),
        json!({"type":"ClassExpression","body":[]}),
        json!({"type":"TemplateLiteral","quasis":template["quasis"],"expressions":template["expressions"]}),
        json!({"type":"UpdateExpression","operator":"++","prefix":false,"argument":x}),
        json!({"type":"NewExpression","callee":{"type":"Identifier","name":"Ctor"},"arguments":[]}),
        json!({"type":"SequenceExpression","expressions":[call, {"type":"CallExpression","callee":y,"arguments":[]}]}),
        json!({"type":"TaggedTemplateExpression","tag":x,"quasi":template}),
        json!({"type":"SpreadElement","argument":x}),
        json!({"type":"YieldExpression","delegate":false,"argument":x}),
        json!({"type":"AwaitExpression","argument":x}),
        json!({"type":"ThisExpression"}),
        json!({"type":"Super"}),
        json!({"type":"NewTarget"}),
        json!({"type":"ImportMeta"}),
        json!({"type":"ImportExpression","source":x}),
    ];
    assert_eq!(values.len(), 36);
    values
        .iter_mut()
        .map(|value| {
            let name = value["type"].as_str().unwrap().to_owned();
            value["cv"] = json!(cv);
            let expression =
                serde_json::from_value(value.clone()).unwrap_or_else(|e| panic!("{name}: {e}"));
            (name, expression)
        })
        .collect()
}

fn root_cv(expr: &Expression) -> Option<String> {
    // Read the actual returned typed node's root field. This cannot choose an
    // unrelated folded entry merely because that entry names itself new_cv.
    serde_json::to_value(expr)
        .unwrap()
        .get("cv")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn pick(elements: Vec<Option<Expression>>, key: Expression, ids: &[String]) -> Program {
    program_with_expr(
        Expression::MemberExpression(MemberExpression {
            cv: ids.first().cloned(),
            object: Box::new(Expression::ArrayExpression(ArrayExpression {
                cv: ids.get(1).cloned(),
                elements,
            })),
            property: Box::new(key),
            computed: true,
        }),
        false,
    )
}

fn run(prog: &Program, cv: &mut CVLog) -> coding_adventures_closure_pass_pipeline::PassOutput {
    ConstantFoldPass::new()
        .run(PassContext {
            program: prog,
            sidecar: &Sidecar::new(),
            cv,
        })
        .unwrap()
}

fn assert_owned(cv: &CVLog, result: &Expression, expected: &[String]) {
    let id = root_cv(result).expect("actual selected replacement must own an ID");
    assert!(
        !expected.contains(&id),
        "old selected/member IDs cannot own the replacement"
    );
    let entry = cv.get(&id).unwrap();
    assert_eq!(
        entry.contributions.len(),
        1,
        "actual root owns exactly its selection rewrite"
    );
    let contribution = &entry.contributions[0];
    assert_eq!(contribution.source, "constant-fold");
    assert_eq!(contribution.tag, "folded");
    assert_eq!(contribution.meta["before"], "array-literal[2][0]");
    assert_eq!(contribution.meta["after"], "element[0]");
    assert_eq!(contribution.meta["new_cv"], id);
    assert_eq!(entry.parent_ids.len(), 1);
    assert_eq!(contribution.meta["parent_cv"], entry.parent_ids[0]);
    if expected.len() == 1 {
        assert_eq!(entry.parent_ids, expected);
    } else {
        assert_eq!(cv.get(&entry.parent_ids[0]).unwrap().parent_ids, expected);
    }
}

#[test]
fn c36_fixture_census_is_36_distinct_native_variants() {
    let rows = shapes(None);
    let names: std::collections::BTreeSet<_> = rows.iter().map(|(name, _)| name).collect();
    assert_eq!(names.len(), 36);
    for (name, expression) in rows {
        assert_eq!(serde_json::to_value(expression).unwrap()["type"], name);
    }
}

#[test]
fn c36_actual_selected_roots_preserve_shapes_and_own_exact_operands() {
    for string_key in [false, true] {
        for (name, _) in shapes(None) {
            let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
            let ids: Vec<_> = (0..5).map(|_| cv.try_create(None).unwrap()).collect();
            let selected = shapes(Some(&ids[3]))
                .into_iter()
                .find(|(n, _)| n == &name)
                .unwrap()
                .1;
            let expected = selected.clone();
            let key = if string_key {
                string("0", Some(&ids[2]))
            } else {
                num(0.0, Some(&ids[2]))
            };
            let prog = pick(
                vec![Some(selected), Some(num(9.0, Some(&ids[4])))],
                key,
                &ids,
            );
            let out = run(&prog, &mut cv);
            let result = extract_expr(&out.program);
            if name == "SpreadElement" {
                assert!(
                    matches!(result, Expression::MemberExpression(_)),
                    "spread eligibility unchanged"
                );
                assert!(out
                    .contributions
                    .iter()
                    .all(|c| c.meta["after"] != "element[0]"));
                continue;
            }
            assert!(
                result.eq_ignoring_cv(&expected),
                "{name} non-CV payload changed"
            );
            // Exact field equality after deleting ONLY the root identity also
            // preserves all nested identities, raw strings and signed zero.
            let mut a = serde_json::to_value(result).unwrap();
            let mut b = serde_json::to_value(&expected).unwrap();
            a.as_object_mut().unwrap().remove("cv");
            b.as_object_mut().unwrap().remove("cv");
            assert_eq!(a, b, "{name} nested CV/payload changed");
            assert_owned(&cv, result, &ids[..4]);
        }
    }
}

#[test]
fn c36_available_positions_deduplicate_and_missing_positions_stay_untraced() {
    for string_key in [false, true] {
        for mask in 0..16 {
            let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
            let source = cv.try_create(None).unwrap();
            let present = |bit| (mask & (1 << bit) != 0).then_some(source.as_str());
            let prog = program_with_expr(
                Expression::MemberExpression(MemberExpression {
                    cv: present(0).map(str::to_owned),
                    object: Box::new(Expression::ArrayExpression(ArrayExpression {
                        cv: present(1).map(str::to_owned),
                        elements: vec![Some(num(7.0, present(3))), Some(num(9.0, None))],
                    })),
                    property: Box::new(if string_key {
                        string("0", present(2))
                    } else {
                        num(0.0, present(2))
                    }),
                    computed: true,
                }),
                false,
            );
            let out = run(&prog, &mut cv);
            if mask == 0 {
                assert!(root_cv(extract_expr(&out.program)).is_none());
                assert!(out.contributions.is_empty());
            } else {
                assert_owned(&cv, extract_expr(&out.program), &[source]);
            }
        }
    }
}

#[test]
fn c36_selected_and_actual_key_rewrites_are_immediate_parents() {
    for string_key in [false, true] {
        let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
        let ids: Vec<_> = (0..7).map(|_| cv.try_create(None).unwrap()).collect();
        let selected = Expression::BinaryExpression(BinaryExpression {
            cv: None,
            operator: BinaryOperator::Add,
            left: Box::new(num(1.0, Some(&ids[3]))),
            right: Box::new(num(2.0, Some(&ids[4]))),
        });
        let key = Expression::BinaryExpression(BinaryExpression {
            cv: None,
            operator: BinaryOperator::Add,
            left: Box::new(if string_key {
                string("0", Some(&ids[5]))
            } else {
                num(0.0, Some(&ids[5]))
            }),
            right: Box::new(if string_key {
                string("", Some(&ids[6]))
            } else {
                num(0.0, Some(&ids[6]))
            }),
        });
        let prog = pick(
            vec![Some(selected), Some(num(9.0, Some(&ids[2])))],
            key,
            &ids,
        );
        let out = run(&prog, &mut cv);
        let selected_fold = out
            .contributions
            .iter()
            .find(|c| c.meta["after"] == "3")
            .unwrap();
        let key_fold = out
            .contributions
            .iter()
            .find(|c| {
                c.meta["after"]
                    == if string_key {
                        json!("\"0\"")
                    } else {
                        json!("0")
                    }
            })
            .unwrap();
        let expected = vec![
            ids[0].clone(),
            ids[1].clone(),
            key_fold.meta["new_cv"].as_str().unwrap().to_owned(),
            selected_fold.meta["new_cv"].as_str().unwrap().to_owned(),
        ];
        assert_owned(&cv, extract_expr(&out.program), &expected);
        for (fold, parents) in [
            (selected_fold, vec![ids[3].clone(), ids[4].clone()]),
            (key_fold, vec![ids[5].clone(), ids[6].clone()]),
        ] {
            let node = cv.get(fold.meta["new_cv"].as_str().unwrap()).unwrap();
            assert_eq!(cv.get(&node.parent_ids[0]).unwrap().parent_ids, parents);
        }
    }
}

#[test]
fn c36_selection_checked_failures_reject_the_typed_candidate() {
    for string_key in [false, true] {
        for limits in [
            GraphLimits {
                max_nodes: 2,
                ..Default::default()
            },
            GraphLimits {
                max_nodes: 3,
                ..Default::default()
            },
            GraphLimits {
                max_edges: 0,
                ..Default::default()
            },
            GraphLimits {
                max_edges: 2,
                ..Default::default()
            },
            GraphLimits {
                max_events: 0,
                ..Default::default()
            },
        ] {
            let mut cv = CVLog::new_checked_compact(limits).unwrap();
            let selected = cv.try_create(None).unwrap();
            let key_cv = cv.try_create(None).unwrap();
            let prog = pick(
                vec![Some(num(7.0, Some(&selected))), Some(num(9.0, None))],
                if string_key {
                    string("0", Some(&key_cv))
                } else {
                    num(0.0, Some(&key_cv))
                },
                &[],
            );
            let error = ConstantFoldPass::new()
                .run(PassContext {
                    program: &prog,
                    sidecar: &Sidecar::new(),
                    cv: &mut cv,
                })
                .unwrap_err();
            assert_eq!(error.pass_name, "constant-fold");
            assert!(
                error.message.contains("CV") && error.message.contains("limit"),
                "{error}"
            );
        }
    }
}

#[test]
fn c36_opaque_selected_call_keeps_nonempty_nested_and_derived_child_cvs() {
    for string_key in [false, true] {
        let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
        let ids: Vec<_> = (0..7).map(|_| cv.try_create(None).unwrap()).collect();
        let selected = Expression::CallExpression(CallExpression {
            cv: Some(ids[3].clone()),
            callee: Box::new(Expression::Identifier(Identifier {
                cv: Some(ids[4].clone()),
                name: "observe".into(),
            })),
            arguments: vec![Expression::BinaryExpression(BinaryExpression {
                cv: None,
                operator: BinaryOperator::Add,
                left: Box::new(num(1.0, Some(&ids[5]))),
                right: Box::new(num(2.0, Some(&ids[6]))),
            })],
        });
        let prog = pick(
            vec![Some(selected), Some(num(9.0, None))],
            if string_key {
                string("0", Some(&ids[2]))
            } else {
                num(0.0, Some(&ids[2]))
            },
            &ids,
        );
        let out = run(&prog, &mut cv);
        let result = extract_expr(&out.program);
        let Expression::CallExpression(call) = result else {
            panic!("selected effect must remain a call");
        };
        let Expression::Identifier(callee) = call.callee.as_ref() else {
            panic!("callee");
        };
        assert_eq!(callee.cv.as_ref(), Some(&ids[4]));
        assert_eq!(callee.name, "observe");
        assert_eq!(call.arguments.len(), 1);
        let Expression::NumericLiteral(argument) = &call.arguments[0] else {
            panic!("folded argument");
        };
        assert_eq!(argument.value.to_bits(), 3.0f64.to_bits());
        let child = cv
            .get(
                argument
                    .cv
                    .as_deref()
                    .expect("derived child retained in actual subtree"),
            )
            .unwrap();
        assert_eq!(child.contributions.len(), 1);
        assert_eq!(child.contributions[0].meta["after"], "3");
        assert_eq!(cv.get(&child.parent_ids[0]).unwrap().parent_ids, ids[5..7]);
        // CLOC37 extends CLOC36's four-parent contract with the selected
        // callee and actual derived argument, preserving the original child
        // ownership/payload assertions above rather than weakening them.
        let mut expected = ids[..5].to_vec();
        expected.push(argument.cv.as_ref().unwrap().clone());
        assert_owned(&cv, result, &expected);
    }
}

#[test]
fn c36_numeric_string_spellings_keep_the_original_string_key_identity() {
    for spelling in ["0", "00", " 0 ", "0x0", "0e0", "", "-0"] {
        let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
        let ids: Vec<_> = (0..4).map(|_| cv.try_create(None).unwrap()).collect();
        let prog = pick(
            vec![Some(num(7.0, Some(&ids[3]))), Some(num(9.0, None))],
            string(spelling, Some(&ids[2])),
            &ids,
        );
        let out = run(&prog, &mut cv);
        assert_owned(&cv, extract_expr(&out.program), &ids);
    }
}

#[test]
fn c36_sibling_effect_spread_and_fractional_refusals_keep_semantics() {
    for case in 0..4 {
        let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
        let ids: Vec<_> = (0..4).map(|_| cv.try_create(None).unwrap()).collect();
        let sibling = match case {
            0 => Expression::CallExpression(CallExpression {
                cv: None,
                callee: Box::new(ident("effect")),
                arguments: vec![],
            }),
            1 => Expression::SpreadElement(SpreadElement {
                cv: None,
                argument: Box::new(ident("unknown")),
            }),
            _ => num(9.0, None),
        };
        let key = if case == 3 {
            string("0.5", Some(&ids[2]))
        } else {
            num(if case == 2 { 0.5 } else { 0.0 }, Some(&ids[2]))
        };
        let prog = pick(
            vec![Some(num(7.0, Some(&ids[3]))), Some(sibling)],
            key,
            &ids,
        );
        let out = run(&prog, &mut cv);
        assert!(matches!(
            extract_expr(&out.program),
            Expression::MemberExpression(_)
        ));
        assert!(extract_expr(&out.program).eq_ignoring_cv(extract_expr(&prog)));
        assert!(out
            .contributions
            .iter()
            .all(|c| c.meta["after"] != "element[0]"));
    }
}
