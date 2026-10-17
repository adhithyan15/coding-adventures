//! CLOC40 starts at the actual returned logical-choice node.
use super::*;
use std::collections::BTreeSet;
fn owned(cv: &CVLog, result: &Expression, expected: &[String], before: &str, after: &str) {
    if expected.is_empty() {
        assert!(root_cv(result).is_none());
        return;
    }
    let id = root_cv(result).expect("actual selected root owns its rewrite");
    assert!(
        !expected.contains(&id),
        "old branch CV cannot own new selection"
    );
    let entry = cv.get(&id).unwrap();
    assert_eq!(entry.contributions.len(), 1);
    let contribution = &entry.contributions[0];
    assert_eq!(contribution.source, "constant-fold");
    assert_eq!(contribution.tag, "folded");
    assert_eq!(contribution.meta["before"], before);
    assert_eq!(contribution.meta["after"], after);
    assert_eq!(contribution.meta["new_cv"], id);
    assert_eq!(entry.parent_ids.len(), 1);
    assert_eq!(contribution.meta["parent_cv"], entry.parent_ids[0]);
    if expected.len() == 1 {
        assert_eq!(entry.parent_ids, expected);
    } else {
        assert_eq!(cv.get(&entry.parent_ids[0]).unwrap().parent_ids, expected);
    }
}

fn reaches(cv: &CVLog, root: &str, wanted: &str) -> bool {
    let mut pending = vec![root.to_owned()];
    let mut seen = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if id == wanted {
            return true;
        }
        if seen.insert(id.clone()) {
            pending.extend(cv.get(&id).unwrap().parent_ids.iter().cloned());
        }
    }
    false
}

fn boolean(value: bool, cv: Option<&str>) -> Expression {
    Expression::BooleanLiteral(BooleanLiteral {
        cv: cv.map(str::to_owned),
        value,
    })
}

fn named(name: &str, cv: Option<&str>) -> Expression {
    Expression::Identifier(Identifier {
        cv: cv.map(str::to_owned),
        name: name.into(),
    })
}

fn left_for(op: LogicalOperator, keep_right: bool, cv: Option<&str>) -> Expression {
    match op {
        LogicalOperator::And => boolean(keep_right, cv),
        LogicalOperator::Or => boolean(!keep_right, cv),
        LogicalOperator::NullishCoalescing if keep_right => Expression::NullLiteral(NullLiteral {
            cv: cv.map(str::to_owned),
        }),
        LogicalOperator::NullishCoalescing => num(-0.0, cv),
    }
}

fn ops() -> [LogicalOperator; 3] {
    [
        LogicalOperator::And,
        LogicalOperator::Or,
        LogicalOperator::NullishCoalescing,
    ]
}

fn input(
    op: LogicalOperator,
    left: Expression,
    right: Expression,
    parent: Option<&str>,
) -> Program {
    program_with_expr(
        Expression::LogicalExpression(LogicalExpression {
            cv: parent.map(str::to_owned),
            operator: op,
            left: Box::new(left),
            right: Box::new(right),
        }),
        false,
    )
}

fn add(a: f64, b: f64, left: Option<&str>, right: Option<&str>) -> Expression {
    Expression::BinaryExpression(BinaryExpression {
        cv: None,
        operator: BinaryOperator::Add,
        left: Box::new(num(a, left)),
        right: Box::new(num(b, right)),
    })
}

fn call(root: Option<&str>, callee: Option<&str>, argument: Expression) -> Expression {
    Expression::CallExpression(CallExpression {
        cv: root.map(str::to_owned),
        callee: Box::new(named("observe", callee)),
        arguments: vec![argument],
    })
}

fn snapshot(cv: &CVLog) -> Value {
    serde_json::from_str(&cv.to_json_string().unwrap()).unwrap()
}

fn labels(op: LogicalOperator, chosen: &Expression) -> (String, String) {
    (
        format!("({}) {} (...)", lit_label(chosen), logical_op_label(op)),
        literal_label_for_expr(chosen),
    )
}

#[test]
fn c40_right_selected_native_shapes_own_actual_root_with_or_without_logical_id() {
    let mut cases = 0;
    for op in ops() {
        for parent in [false, true] {
            for (name, _) in shapes(None) {
                let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
                let ids: Vec<_> = (0..3).map(|_| cv.try_create(None).unwrap()).collect();
                let chosen = shapes(Some(&ids[2]))
                    .into_iter()
                    .find(|(n, _)| n == &name)
                    .unwrap()
                    .1;
                let payload = serde_json::to_value(&chosen).unwrap();
                // JSON number equality does not establish the sign bit of -0.
                // Capture this actual right operand before ownership moves it.
                let numeric_payload = match &chosen {
                    Expression::NumericLiteral(n) => Some((n.value.to_bits(), n.raw.clone())),
                    _ => None,
                };
                let (before, after) = labels(op, &chosen);
                let p = input(
                    op,
                    left_for(op, true, Some(&ids[1])),
                    chosen,
                    parent.then_some(ids[0].as_str()),
                );
                let out = run(&p, &mut cv);
                let result = extract_expr(&out.program);
                if let Some((bits, raw)) = numeric_payload {
                    let Expression::NumericLiteral(n) = result else {
                        panic!("selected numeric right operand")
                    };
                    assert_eq!(n.value.to_bits(), bits);
                    assert_eq!(n.raw, raw);
                }
                let mut actual = serde_json::to_value(result).unwrap();
                let mut expected_payload = payload;
                actual.as_object_mut().unwrap().remove("cv");
                expected_payload.as_object_mut().unwrap().remove("cv");
                assert_eq!(
                    actual, expected_payload,
                    "{name} nested identity/raw payload"
                );
                let mut expected = Vec::new();
                if parent {
                    expected.push(ids[0].clone());
                }
                expected.extend(ids[1..3].iter().cloned());
                owned(&cv, result, &expected, &before, &after);
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 216); // Native Spread coverage is not source admission.
}

#[test]
fn c40_literal_guard_table_preserves_selected_bits_raw_and_chosen_labels() {
    // An explicit oracle keeps the existing predicates out of the expected-side
    // calculation. Each row lists right-selection for &&, ||, ?? respectively.
    let rows = vec![
        (boolean(false, None), [false, true, false]),
        (boolean(true, None), [true, false, false]),
        (num(0.0, None), [false, true, false]),
        (num(-0.0, None), [false, true, false]),
        (num(f64::NAN, None), [false, true, false]),
        (num(1.0, None), [true, false, false]),
        (num(-1.0, None), [true, false, false]),
        (num(f64::INFINITY, None), [true, false, false]),
        (num(f64::NEG_INFINITY, None), [true, false, false]),
        (string("", None), [false, true, false]),
        (string("😀", None), [true, false, false]),
        (
            Expression::NullLiteral(NullLiteral { cv: None }),
            [false, true, true],
        ),
    ];
    assert_eq!(rows.len(), 12);
    for (left, sides) in rows {
        for (index, op) in ops().into_iter().enumerate() {
            let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
            let ids: Vec<_> = (0..3).map(|_| cv.try_create(None).unwrap()).collect();
            let mut left = left.clone();
            *selected_root_identity::slot(&mut left) = Some(ids[1].clone());
            let right = named("chosen", Some(&ids[2]));
            let chosen = if sides[index] { &right } else { &left };
            let (before, after) = labels(op, chosen);
            let expected_payload = chosen.clone();
            let out = run(&input(op, left, right, Some(&ids[0])), &mut cv);
            let result = extract_expr(&out.program);
            let expected = if sides[index] { &ids[..3] } else { &ids[..2] };
            owned(&cv, result, expected, &before, &after);
            let mut payload = result.clone();
            *selected_root_identity::slot(&mut payload) = root_cv(&expected_payload);
            if let (Expression::NumericLiteral(a), Expression::NumericLiteral(b)) =
                (&payload, &expected_payload)
            {
                assert_eq!(a.value.to_bits(), b.value.to_bits());
                assert_eq!(a.raw, b.raw);
                assert_eq!(a.cv, b.cv);
            } else {
                assert_eq!(payload, expected_payload);
            }
            if !sides[index] {
                assert!(!reaches(&cv, &root_cv(result).unwrap(), &ids[2]));
            }
        }
    }
}

#[test]
fn c40_availability_duplicates_and_chosen_left_alias_have_exact_frontiers() {
    let mut cases = 0;
    for op in ops() {
        for right_selected in [false, true] {
            for mask in 0..8 {
                let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
                let id = cv.try_create(None).unwrap();
                let present = |bit| (mask & (1 << bit) != 0).then_some(id.as_str());
                let left = left_for(op, right_selected, present(1));
                let right = named("chosen", present(2));
                let chosen = if right_selected { &right } else { &left };
                let (before, after) = labels(op, chosen);
                let old_entries = cv.entries().len();
                let out = run(&input(op, left, right, present(0)), &mut cv);
                let has_id = mask & (if right_selected { 7 } else { 3 }) != 0;
                let expected = if has_id { vec![id] } else { vec![] };
                owned(&cv, extract_expr(&out.program), &expected, &before, &after);
                assert_eq!(cv.entries().len() - old_entries, usize::from(has_id));
                assert_eq!(out.contributions.len(), usize::from(has_id));
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 48);
}

#[test]
fn c40_selected_call_descendants_reach_actual_rewrites_after_folded_left() {
    for op in ops() {
        let mut cv = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
        let ids: Vec<_> = (0..8).map(|_| cv.try_create(None).unwrap()).collect();
        // For ??, a folded conditional yields null without widening the
        // nullish predicate; for &&/|| a numeric sum produces the deciding left.
        let left = if op == LogicalOperator::NullishCoalescing {
            Expression::ConditionalExpression(ConditionalExpression {
                cv: Some(ids[1].clone()),
                test: Box::new(boolean(true, Some(&ids[2]))),
                consequent: Box::new(Expression::NullLiteral(NullLiteral {
                    cv: Some(ids[3].clone()),
                })),
                alternate: Box::new(named("unused", None)),
            })
        } else {
            add(
                if op == LogicalOperator::And { 1.0 } else { 0.0 },
                0.0,
                Some(&ids[2]),
                Some(&ids[3]),
            )
        };
        let right = call(
            Some(&ids[4]),
            Some(&ids[5]),
            add(1.0, 2.0, Some(&ids[6]), Some(&ids[7])),
        );
        let out = run(&input(op, left, right, Some(&ids[0])), &mut cv);
        let result = extract_expr(&out.program);
        let Expression::CallExpression(c) = result else {
            panic!("selected call")
        };
        assert_eq!(root_cv(&c.callee), Some(ids[5].clone()));
        let Expression::NumericLiteral(n) = &c.arguments[0] else {
            panic!("rewritten child")
        };
        assert_eq!(n.value.to_bits(), 3.0f64.to_bits());
        let child = n.cv.as_ref().unwrap();
        let left_fold = out
            .contributions
            .iter()
            .find(|c| {
                c.meta["after"]
                    == if op == LogicalOperator::NullishCoalescing {
                        "null"
                    } else if op == LogicalOperator::And {
                        "1"
                    } else {
                        "0"
                    }
            })
            .unwrap();
        let left_id = left_fold.meta["new_cv"].as_str().unwrap();
        let expected = vec![
            ids[0].clone(),
            left_id.to_owned(),
            ids[4].clone(),
            ids[5].clone(),
            child.clone(),
        ];
        owned(
            &cv,
            result,
            &expected,
            &format!("(<expr>) {} (...)", logical_op_label(op)),
            "<expr>",
        );
        let root = root_cv(result).unwrap();
        assert!(reaches(&cv, &root, child));
        let child_merge = &cv.get(child).unwrap().parent_ids[0];
        assert_eq!(cv.get(child_merge).unwrap().parent_ids, ids[6..8]);
        let s = snapshot(&cv);
        let events = s["journal"]["events"].as_array().unwrap();
        let event = |kind: &str, id: &str| {
            events
                .iter()
                .position(|e| e["event"]["kind"] == kind && e["event"]["entity"] == id)
                .unwrap()
        };
        let merged = &cv.get(&root).unwrap().parent_ids[0];
        assert!(event("contribution", left_id) < event("contribution", child));
        assert!(event("contribution", child) < event("merge", merged));
        assert!(event("merge", merged) < event("derive", &root));
        assert!(event("derive", &root) < event("contribution", &root));
    }
}

#[test]
fn c40_selected_function_preserves_populated_body_and_child_identity() {
    for op in ops() {
        let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
        let ids: Vec<_> = (0..9).map(|_| cv.try_create(None).unwrap()).collect();
        let selected: Expression = serde_json::from_value(json!({
            "type":"FunctionExpression", "cv":ids[2], "params":[],
            "generator":false,"async":false,"body":{"cv":ids[3],"body":[{
                "type":"ExpressionStatement","cv":ids[4],"expression":{
                    "type":"CallExpression","cv":ids[5],
                    "callee":{"type":"Identifier","name":"observe","cv":ids[6]},
                    "arguments":[{"type":"BinaryExpression","operator":"+",
                        "left":{"type":"NumericLiteral","value":1.0,"raw":"1","cv":ids[7]},
                        "right":{"type":"NumericLiteral","value":2.0,"raw":"2","cv":ids[8]}}]
                }
            }]}
        }))
        .unwrap();
        let out = run(
            &input(
                op,
                left_for(op, true, Some(&ids[1])),
                selected,
                Some(&ids[0]),
            ),
            &mut cv,
        );
        let result = extract_expr(&out.program);
        let Expression::FunctionExpression(f) = result else {
            panic!("selected function")
        };
        assert_eq!(f.body.cv.as_ref(), Some(&ids[3]));
        let Statement::Tagged(TaggedStatement::ExpressionStatement(s)) = &f.body.body[0] else {
            panic!("statement")
        };
        assert_eq!(s.cv.as_ref(), Some(&ids[4]));
        let Expression::CallExpression(c) = &s.expression else {
            panic!("call")
        };
        assert_eq!(c.cv.as_ref(), Some(&ids[5]));
        assert_eq!(root_cv(&c.callee), Some(ids[6].clone()));
        let Expression::NumericLiteral(n) = &c.arguments[0] else {
            panic!("child fold")
        };
        assert_eq!(n.value.to_bits(), 3.0f64.to_bits());
        let child = n.cv.as_ref().unwrap();
        let mut expected = ids[..7].to_vec();
        expected.push(child.clone());
        owned(
            &cv,
            result,
            &expected,
            &format!("(<expr>) {} (...)", logical_op_label(op)),
            "<expr>",
        );
        assert!(reaches(&cv, &root_cv(result).unwrap(), child));
        assert_eq!(
            cv.get(&cv.get(child).unwrap().parent_ids[0])
                .unwrap()
                .parent_ids,
            ids[7..9]
        );
    }
}

#[test]
fn c40_unchosen_right_success_stays_in_journal_but_outside_selected_parents() {
    for op in ops() {
        let mut cv = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
        let ids: Vec<_> = (0..4).map(|_| cv.try_create(None).unwrap()).collect();
        let left = left_for(op, false, Some(&ids[1]));
        let (before, after) = labels(op, &left);
        let out = run(
            &input(
                op,
                left,
                add(4.0, 5.0, Some(&ids[2]), Some(&ids[3])),
                Some(&ids[0]),
            ),
            &mut cv,
        );
        let result = extract_expr(&out.program);
        owned(&cv, result, &ids[..2], &before, &after);
        let child = out
            .contributions
            .iter()
            .find(|c| c.meta["after"] == "9")
            .unwrap()
            .meta["new_cv"]
            .as_str()
            .unwrap();
        let root = root_cv(result).unwrap();
        assert!(!reaches(&cv, &root, child));
        let s = snapshot(&cv);
        let events = s["journal"]["events"].as_array().unwrap();
        let position = |id: &str| {
            events
                .iter()
                .position(|e| e["event"]["kind"] == "contribution" && e["event"]["entity"] == id)
                .unwrap()
        };
        assert!(position(child) < position(&root));
    }
}

#[test]
fn c40_checked_child_origins_preserve_first_error_and_prior_left_activity() {
    // Left-origin errors prevent all later work. Right-origin cases exercise
    // both selected and unchosen right, with a successful left arithmetic fold
    // wherever the existing admission permits that deciding literal.
    for op in ops() {
        for right_selected in [false, true] {
            for origin in 0..2 {
                for missing in [false, true] {
                    let left_rewrites = origin == 0
                        || !(op == LogicalOperator::NullishCoalescing && right_selected);
                    let prior = usize::from(origin == 1 && left_rewrites);
                    let limits = if missing {
                        GraphLimits::default()
                    } else {
                        GraphLimits {
                            max_nodes: 5 + 2 * prior,
                            ..Default::default()
                        }
                    };
                    let mut cv = CVLog::new_checked_chronology(limits).unwrap();
                    let ids: Vec<_> = (0..5).map(|_| cv.try_create(None).unwrap()).collect();
                    let failing = |stage, id: usize| {
                        if missing && stage == origin {
                            "missing-parent"
                        } else {
                            ids[id].as_str()
                        }
                    };
                    let value = if op == LogicalOperator::And {
                        (if right_selected { 1.0 } else { 0.0 })
                    } else if op == LogicalOperator::Or {
                        (if right_selected { 0.0 } else { 1.0 })
                    } else {
                        0.0
                    };
                    let left = if left_rewrites {
                        add(value, 0.0, Some(&ids[1]), Some(failing(0, 2)))
                    } else {
                        left_for(op, right_selected, Some(&ids[1]))
                    };
                    let right = add(4.0, 5.0, Some(&ids[3]), Some(failing(1, 4)));
                    let before = snapshot(&cv);
                    let p = input(op, left, right, Some(&ids[0]));
                    let error = ConstantFoldPass::new()
                        .run(PassContext {
                            program: &p,
                            sidecar: &Sidecar::new(),
                            cv: &mut cv,
                        })
                        .unwrap_err();
                    let after = snapshot(&cv);
                    let pair = if origin == 0 { [1, 2] } else { [3, 4] };
                    let expected = cv
                        .try_merge(&[ids[pair[0]].as_str(), failing(origin, pair[1])], None)
                        .unwrap_err();
                    assert_eq!(error.pass_name, "constant-fold");
                    assert_eq!(error.message, format!("CV fold merge failed: {expected}"));
                    assert_eq!(snapshot(&cv), after);
                    let old = before["entries"].as_object().unwrap();
                    let now = after["entries"].as_object().unwrap();
                    for (id, entry) in old {
                        assert_eq!(&now[id], entry);
                    }
                    assert_eq!(now.len() - old.len(), 2 * prior);
                    let old_events = before["journal"]["events"].as_array().unwrap();
                    let events = after["journal"]["events"].as_array().unwrap();
                    assert_eq!(&events[..old_events.len()], old_events);
                    let added = &events[old_events.len()..];
                    assert_eq!(added.len(), 3 * prior);
                    if prior == 1 {
                        assert_eq!(added[0]["event"]["kind"], "merge");
                        assert_eq!(added[1]["event"]["kind"], "derive");
                        assert_eq!(added[2]["event"]["kind"], "contribution");
                        let merged = added[0]["event"]["entity"].as_str().unwrap();
                        let child = added[1]["event"]["entity"].as_str().unwrap();
                        assert_eq!(now[merged]["parent_ids"], json!([ids[1], ids[2]]));
                        assert_eq!(now[child]["parent_ids"], json!([merged]));
                        assert_eq!(
                            now[child]["contributions"][0]["meta"]["before"],
                            format!("({value}) + (0)")
                        );
                    }
                    if missing {
                        let outer = cv
                            .try_merge(&[ids[0].as_str(), ids[1].as_str(), ids[3].as_str()], None)
                            .unwrap();
                        cv.try_derive(&outer, None).unwrap();
                    }
                }
            }
        }
    }
}

#[test]
fn c40_outer_checked_merge_derive_and_event_refusals_reject_every_side_operator() {
    for op in ops() {
        for right_selected in [false, true] {
            for limits in [
                GraphLimits {
                    max_nodes: 3,
                    ..Default::default()
                },
                GraphLimits {
                    max_nodes: 4,
                    ..Default::default()
                },
                GraphLimits {
                    max_edges: 0,
                    ..Default::default()
                },
                GraphLimits {
                    max_edges: if right_selected { 3 } else { 2 },
                    ..Default::default()
                },
                GraphLimits {
                    max_events: 0,
                    ..Default::default()
                },
            ] {
                let mut cv = CVLog::new_checked_compact(limits).unwrap();
                let ids: Vec<_> = (0..3).map(|_| cv.try_create(None).unwrap()).collect();
                let p = input(
                    op,
                    left_for(op, right_selected, Some(&ids[1])),
                    named("chosen", Some(&ids[2])),
                    Some(&ids[0]),
                );
                let error = ConstantFoldPass::new()
                    .run(PassContext {
                        program: &p,
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
}

#[test]
fn c40_declined_left_families_keep_the_logical_expression() {
    for op in ops() {
        for (_, left) in shapes(None).into_iter().filter(|(name, _)| {
            [
                "Identifier",
                "UndefinedLiteral",
                "BigIntLiteral",
                "RegExpLiteral",
                "ArrayExpression",
                "ObjectExpression",
            ]
            .contains(&name.as_str())
        }) {
            let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
            let parent = cv.try_create(None).unwrap();
            let p = input(op, left, named("other", None), Some(&parent));
            let out = run(&p, &mut cv);
            assert_eq!(extract_expr(&out.program), extract_expr(&p));
            assert!(out.contributions.is_empty());
        }
    }
}
