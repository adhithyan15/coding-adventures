//! CLOC39 starts from actual returned roots and exact ordered graph parents.
use super::*;

fn equal_input(
    test: Expression,
    consequent: Expression,
    alternate: Expression,
    cv: Option<&str>,
) -> Program {
    program_with_expr(
        Expression::ConditionalExpression(ConditionalExpression {
            cv: cv.map(str::to_owned),
            test: Box::new(test),
            consequent: Box::new(consequent),
            alternate: Box::new(alternate),
        }),
        false,
    )
}
fn named(name: &str, cv: Option<&str>) -> Expression {
    Expression::Identifier(Identifier {
        cv: cv.map(str::to_owned),
        name: name.into(),
    })
}
fn opaque(
    name: &str,
    cv: Option<&str>,
    callee: Option<&str>,
    arguments: Vec<Expression>,
) -> Expression {
    Expression::CallExpression(CallExpression {
        cv: cv.map(str::to_owned),
        callee: Box::new(named(name, callee)),
        arguments,
    })
}
fn test_root(impure: bool, cv: Option<&str>) -> Expression {
    if impure {
        opaque("check", cv, None, vec![])
    } else {
        named("gate", cv)
    }
}
fn add(a: f64, b: f64, left: Option<&str>, right: Option<&str>) -> Expression {
    Expression::BinaryExpression(BinaryExpression {
        cv: None,
        operator: BinaryOperator::Add,
        left: Box::new(num(a, left)),
        right: Box::new(num(b, right)),
    })
}
fn own(cv: &CVLog, result: &Expression, expected: &[String], impure: bool) {
    let Some(id) = root_cv(result) else {
        assert!(expected.is_empty());
        return;
    };
    assert!(!expected.is_empty() && !expected.contains(&id));
    let entry = cv.get(&id).unwrap();
    assert_eq!(entry.contributions.len(), 1);
    let c = &entry.contributions[0];
    assert_eq!(c.source, "constant-fold");
    assert_eq!(c.tag, "folded");
    assert_eq!(c.meta["before"], "t ? X : X");
    assert_eq!(c.meta["after"], if impure { "(t,X)" } else { "X" });
    assert_eq!(c.meta["new_cv"], id);
    assert_eq!(entry.parent_ids.len(), 1);
    assert_eq!(c.meta["parent_cv"], entry.parent_ids[0]);
    if expected.len() == 1 {
        assert_eq!(entry.parent_ids, expected);
    } else {
        assert_eq!(cv.get(&entry.parent_ids[0]).unwrap().parent_ids, expected);
    }
}
fn snapshot(cv: &CVLog) -> Value {
    serde_json::from_str(&cv.to_json_string().unwrap()).unwrap()
}

#[test]
fn c39_actual_native_shapes_join_both_equal_roots_and_own_pure_or_sequence_result() {
    let mut cases = 0;
    for impure in [false, true] {
        for parent in [false, true] {
            for (name, _) in shapes(None) {
                let mut cv = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
                let ids: Vec<_> = (0..4).map(|_| cv.try_create(None).unwrap()).collect();
                let branch = |id: &str| {
                    shapes(Some(id))
                        .into_iter()
                        .find(|(n, _)| n == &name)
                        .unwrap()
                        .1
                };
                let chosen = branch(&ids[2]);
                let alternate = branch(&ids[3]);
                let test = test_root(impure, Some(&ids[1]));
                let input = equal_input(
                    test.clone(),
                    chosen.clone(),
                    alternate,
                    parent.then_some(ids[0].as_str()),
                );
                let out = run(&input, &mut cv);
                let result = extract_expr(&out.program);
                let selected = if impure {
                    let Expression::SequenceExpression(sequence) = result else {
                        panic!("{name}: actual sequence root")
                    };
                    assert_eq!(sequence.expressions.len(), 2);
                    assert_eq!(
                        serde_json::to_value(&sequence.expressions[0]).unwrap(),
                        serde_json::to_value(test).unwrap()
                    );
                    assert_eq!(
                        serde_json::to_value(&sequence.expressions[1]).unwrap(),
                        serde_json::to_value(&chosen).unwrap()
                    );
                    &sequence.expressions[1]
                } else {
                    result
                };
                let mut actual = serde_json::to_value(selected).unwrap();
                let mut wanted = serde_json::to_value(chosen).unwrap();
                actual.as_object_mut().unwrap().remove("cv");
                wanted.as_object_mut().unwrap().remove("cv");
                assert_eq!(actual, wanted, "{name}: nested CVs/raw payload");
                let mut expected = Vec::new();
                if parent {
                    expected.push(ids[0].clone());
                }
                expected.extend(ids[1..4].iter().cloned());
                own(&cv, result, &expected, impure);
                cases += 1;
            }
        }
    }
    // Native shape coverage includes Spread; no standalone JS admission claim.
    assert_eq!(cases, 144);
}

#[test]
fn c39_absent_duplicate_single_and_no_ids_have_exact_owned_frontiers() {
    for impure in [false, true] {
        for mask in 0..16 {
            let mut cv = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
            let id = cv.try_create(None).unwrap();
            let present = |bit| (mask & (1 << bit) != 0).then_some(id.as_str());
            let input = equal_input(
                test_root(impure, present(1)),
                named("chosen", present(2)),
                named("chosen", present(3)),
                present(0),
            );
            let before = snapshot(&cv);
            let out = run(&input, &mut cv);
            let expected = if mask == 0 { vec![] } else { vec![id.clone()] };
            own(&cv, extract_expr(&out.program), &expected, impure);
            if mask == 0 {
                assert!(out.contributions.is_empty());
                assert_eq!(snapshot(&cv), before);
            }
        }
    }
}

#[test]
fn c39_both_rewritten_arms_and_all_test_descendants_are_exact_ordered_parents() {
    for impure in [false, true] {
        let mut cv = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
        let ids: Vec<_> = (0..13).map(|_| cv.try_create(None).unwrap()).collect();
        let test_child = add(4.0, 0.0, Some(&ids[3]), Some(&ids[4]));
        let test = if impure {
            opaque("check", Some(&ids[1]), Some(&ids[2]), vec![test_child])
        } else {
            Expression::MemberExpression(MemberExpression {
                cv: Some(ids[1].clone()),
                object: Box::new(named("guard", Some(&ids[2]))),
                property: Box::new(test_child),
                computed: true,
            })
        };
        let branch = |root: usize, callee: usize, left: usize, right: usize| {
            opaque(
                "observe",
                Some(&ids[root]),
                Some(&ids[callee]),
                vec![add(1.0, 2.0, Some(&ids[left]), Some(&ids[right]))],
            )
        };
        let input = equal_input(
            test,
            branch(5, 6, 7, 8),
            branch(9, 10, 11, 12),
            Some(&ids[0]),
        );
        let out = run(&input, &mut cv);
        let result = extract_expr(&out.program);
        let folded: Vec<_> = out
            .contributions
            .iter()
            .filter(|c| {
                c.source == "constant-fold"
                    && c.meta["before"] == "(1) + (2)"
                    && c.meta["after"] == "3"
            })
            .map(|c| c.meta["new_cv"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(folded.len(), 2);
        let test_id = out
            .contributions
            .iter()
            .find(|c| {
                c.source == "constant-fold"
                    && c.meta["before"] == "(4) + (0)"
                    && c.meta["after"] == "4"
            })
            .unwrap()
            .meta["new_cv"]
            .as_str()
            .unwrap()
            .to_owned();
        let expected = vec![
            ids[0].clone(),
            ids[1].clone(),
            ids[2].clone(),
            test_id.clone(),
            ids[5].clone(),
            ids[6].clone(),
            folded[0].clone(),
            ids[9].clone(),
            ids[10].clone(),
            folded[1].clone(),
        ];
        own(&cv, result, &expected, impure);
        let selected = if impure {
            let Expression::SequenceExpression(s) = result else {
                panic!("sequence")
            };
            assert_eq!(root_cv(&s.expressions[0]), Some(ids[1].clone()));
            assert_eq!(root_cv(&s.expressions[1]), Some(ids[5].clone()));
            &s.expressions[1]
        } else {
            result
        };
        let Expression::CallExpression(call) = selected else {
            panic!("chosen opaque call")
        };
        assert_eq!(root_cv(&call.callee), Some(ids[6].clone()));
        assert_eq!(root_cv(&call.arguments[0]), Some(folded[0].clone()));
        let value = snapshot(&cv);
        let events = value["journal"]["events"].as_array().unwrap();
        let root = root_cv(result).unwrap();
        let merged = cv.get(&root).unwrap().parent_ids[0].as_str();
        let event = |kind: &str, id: &str| {
            events
                .iter()
                .find(|e| e["event"]["kind"] == kind && e["event"]["entity"] == id)
                .unwrap()
        };
        let merge = event("merge", merged);
        let derive = event("derive", &root);
        let own_event = event("contribution", &root);
        // Fixed-width sequence strings have their actual numeric clock order.
        assert!(merge["sequence"].as_str().unwrap() < derive["sequence"].as_str().unwrap());
        assert!(derive["sequence"].as_str().unwrap() < own_event["sequence"].as_str().unwrap());
        let a = event("contribution", &test_id);
        let b = event("contribution", &folded[0]);
        let c = event("contribution", &folded[1]);
        assert!(a["sequence"].as_str().unwrap() < b["sequence"].as_str().unwrap());
        assert!(b["sequence"].as_str().unwrap() < c["sequence"].as_str().unwrap());
        assert!(c["sequence"].as_str().unwrap() < merge["sequence"].as_str().unwrap());
    }
}

#[test]
fn c39_checked_child_failure_origins_keep_prior_activity_and_latch_before_outer_work() {
    for impure in [false, true] {
        for origin in 0..3 {
            for missing in [false, true] {
                let limits = if missing {
                    GraphLimits::default()
                } else {
                    GraphLimits {
                        max_nodes: 13 + 2 * origin,
                        ..Default::default()
                    }
                };
                let mut cv = CVLog::new_checked_chronology(limits).unwrap();
                let ids: Vec<_> = (0..13).map(|_| cv.try_create(None).unwrap()).collect();
                let right = |stage: usize, index: usize| {
                    if missing && stage == origin {
                        "missing-parent"
                    } else {
                        ids[index].as_str()
                    }
                };
                let child = add(1.0, 2.0, Some(&ids[3]), Some(right(0, 4)));
                let test = if impure {
                    opaque("check", Some(&ids[1]), Some(&ids[2]), vec![child])
                } else {
                    Expression::MemberExpression(MemberExpression {
                        cv: Some(ids[1].clone()),
                        object: Box::new(named("guard", Some(&ids[2]))),
                        property: Box::new(child),
                        computed: true,
                    })
                };
                let branch =
                    |stage: usize, root: usize, callee: usize, left: usize, right_index: usize| {
                        opaque(
                            "observe",
                            Some(&ids[root]),
                            Some(&ids[callee]),
                            vec![add(
                                4.0,
                                5.0,
                                Some(&ids[left]),
                                Some(right(stage, right_index)),
                            )],
                        )
                    };
                let input = equal_input(
                    test,
                    branch(1, 5, 6, 7, 8),
                    branch(2, 9, 10, 11, 12),
                    Some(&ids[0]),
                );
                let before = snapshot(&cv);
                let error = ConstantFoldPass::new()
                    .run(PassContext {
                        program: &input,
                        sidecar: &Sidecar::new(),
                        cv: &mut cv,
                    })
                    .unwrap_err();
                assert_eq!(error.pass_name, "constant-fold");
                let after = snapshot(&cv);
                let left_index = [3, 7, 11][origin];
                let right_index = [4, 8, 12][origin];
                let expected = cv
                    .try_merge(
                        &[ids[left_index].as_str(), right(origin, right_index)],
                        None,
                    )
                    .unwrap_err();
                assert_eq!(snapshot(&cv), after);
                assert_eq!(error.message, format!("CV fold merge failed: {expected}"));
                let old = before["entries"].as_object().unwrap();
                let now = after["entries"].as_object().unwrap();
                for (id, entry) in old {
                    assert_eq!(&now[id], entry);
                }
                assert_eq!(now.len() - old.len(), 2 * origin);
                let old_events = before["journal"]["events"].as_array().unwrap();
                let events = after["journal"]["events"].as_array().unwrap();
                assert_eq!(&events[..old_events.len()], old_events);
                let new_events = &events[old_events.len()..];
                assert_eq!(new_events.len(), 3 * origin);
                for (stage, chunk) in new_events.chunks_exact(3).enumerate() {
                    assert_eq!(chunk[0]["event"]["kind"], "merge");
                    assert_eq!(chunk[1]["event"]["kind"], "derive");
                    assert_eq!(chunk[2]["event"]["kind"], "contribution");
                    let child = chunk[1]["event"]["entity"].as_str().unwrap();
                    let merged = chunk[0]["event"]["entity"].as_str().unwrap();
                    assert_eq!(now[child]["parent_ids"], json!([merged]));
                    let pair = if stage == 0 { [3, 4] } else { [7, 8] };
                    assert_eq!(
                        now[merged]["parent_ids"],
                        json!([ids[pair[0]], ids[pair[1]]])
                    );
                    let contribution = &now[child]["contributions"][0];
                    assert_eq!(contribution["source"], "constant-fold");
                    assert_eq!(contribution["tag"], "folded");
                    assert_eq!(
                        contribution["meta"]["before"],
                        if stage == 0 { "(1) + (2)" } else { "(4) + (5)" }
                    );
                    assert_eq!(
                        contribution["meta"]["after"],
                        if stage == 0 { "3" } else { "9" }
                    );
                }
                if missing {
                    let outer = cv
                        .try_merge(
                            &[
                                ids[0].as_str(),
                                ids[1].as_str(),
                                ids[5].as_str(),
                                ids[9].as_str(),
                            ],
                            None,
                        )
                        .unwrap();
                    cv.try_derive(&outer, None).unwrap();
                }
            }
        }
    }
}

#[test]
fn c39_outer_merge_derive_and_event_refusals_reject_both_return_forms() {
    for impure in [false, true] {
        for limits in [
            GraphLimits {
                max_nodes: 4,
                ..Default::default()
            },
            GraphLimits {
                max_nodes: 5,
                ..Default::default()
            },
            GraphLimits {
                max_edges: 0,
                ..Default::default()
            },
            GraphLimits {
                max_edges: 3,
                ..Default::default()
            },
            GraphLimits {
                max_events: 0,
                ..Default::default()
            },
        ] {
            let mut cv = CVLog::new_checked_compact(limits).unwrap();
            let ids: Vec<_> = (0..4).map(|_| cv.try_create(None).unwrap()).collect();
            let input = equal_input(
                test_root(impure, Some(&ids[1])),
                opaque("observe", Some(&ids[2]), None, vec![]),
                opaque("observe", Some(&ids[3]), None, vec![]),
                Some(&ids[0]),
            );
            let error = ConstantFoldPass::new()
                .run(PassContext {
                    program: &input,
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
