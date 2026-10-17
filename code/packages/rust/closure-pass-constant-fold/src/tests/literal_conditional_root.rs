//! CLOC38 proves identity ownership at the actual returned conditional choice.
use super::*;
use std::collections::BTreeSet;

fn choose(
    test: Expression,
    chosen: Expression,
    other: Expression,
    truthy: bool,
    parent: Option<&str>,
) -> Program {
    let (consequent, alternate) = if truthy {
        (chosen, other)
    } else {
        (other, chosen)
    };
    program_with_expr(
        Expression::ConditionalExpression(ConditionalExpression {
            cv: parent.map(str::to_owned),
            test: Box::new(test),
            consequent: Box::new(consequent),
            alternate: Box::new(alternate),
        }),
        false,
    )
}

fn boolean(value: bool, cv: Option<&str>) -> Expression {
    Expression::BooleanLiteral(BooleanLiteral {
        cv: cv.map(str::to_owned),
        value,
    })
}

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

#[test]
fn c38_actual_native_root_shapes_own_both_choices_with_or_without_conditional_id() {
    let mut cases = 0;
    for truthy in [false, true] {
        for parent_present in [false, true] {
            for (name, _) in shapes(None) {
                let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
                let ids: Vec<_> = (0..4).map(|_| cv.try_create(None).unwrap()).collect();
                let selected = shapes(Some(&ids[2]))
                    .into_iter()
                    .find(|(n, _)| n == &name)
                    .unwrap()
                    .1;
                let expected_payload = selected.clone();
                let after = literal_label_for_expr(&selected);
                let input = choose(
                    boolean(truthy, Some(&ids[1])),
                    selected,
                    Expression::Identifier(Identifier {
                        cv: Some(ids[3].clone()),
                        name: "unselected".into(),
                    }),
                    truthy,
                    parent_present.then_some(ids[0].as_str()),
                );
                let out = run(&input, &mut cv);
                let result = extract_expr(&out.program);
                assert!(result.eq_ignoring_cv(&expected_payload), "{name} payload");
                let mut a = serde_json::to_value(result).unwrap();
                let mut b = serde_json::to_value(expected_payload).unwrap();
                a.as_object_mut().unwrap().remove("cv");
                b.as_object_mut().unwrap().remove("cv");
                assert_eq!(a, b, "{name} nested identity and raw payload");
                let mut expected = Vec::new();
                if parent_present {
                    expected.push(ids[0].clone());
                }
                expected.extend(ids[1..3].iter().cloned());
                owned(
                    &cv,
                    result,
                    &expected,
                    if truthy {
                        "(true) ? ... : ..."
                    } else {
                        "(false) ? ... : ..."
                    },
                    &after,
                );
                assert!(!reaches(&cv, &root_cv(result).unwrap(), &ids[3]));
                cases += 1;
            }
        }
    }
    // Native accessor coverage includes Spread; this does not admit it as a
    // standalone JavaScript conditional branch through the source parser.
    assert_eq!(cases, 144);
}

#[test]
fn c38_missing_duplicate_single_and_no_ids_preserve_exact_frontier_behavior() {
    for truthy in [false, true] {
        for mask in 0..8 {
            let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
            let id = cv.try_create(None).unwrap();
            let other = cv.try_create(None).unwrap();
            let present = |bit| (mask & (1 << bit) != 0).then_some(id.as_str());
            let selected = Expression::Identifier(Identifier {
                cv: present(2).map(str::to_owned),
                name: "chosen".into(),
            });
            let input = choose(
                boolean(truthy, present(1)),
                selected,
                Expression::Identifier(Identifier {
                    cv: Some(other.clone()),
                    name: "other".into(),
                }),
                truthy,
                present(0),
            );
            let out = run(&input, &mut cv);
            let result = extract_expr(&out.program);
            let expected = if mask == 0 { vec![] } else { vec![id] };
            owned(
                &cv,
                result,
                &expected,
                if truthy {
                    "(true) ? ... : ..."
                } else {
                    "(false) ? ... : ..."
                },
                "chosen",
            );
            if mask == 0 {
                assert!(out.contributions.is_empty());
            } else {
                assert!(!reaches(&cv, &root_cv(result).unwrap(), &other));
            }
        }
    }
}

#[test]
fn c38_existing_truthiness_and_labels_include_nan_infinities_and_signed_zero() {
    let cases = vec![
        (boolean(true, None), true, "true"),
        (boolean(false, None), false, "false"),
        (num(1.0, None), true, "1"),
        (num(-1.0, None), true, "-1"),
        (num(0.0, None), false, "0"),
        (num(-0.0, None), false, "0"),
        (num(f64::NAN, None), false, "NaN"),
        (num(f64::INFINITY, None), true, "Infinity"),
        (num(f64::NEG_INFINITY, None), true, "-Infinity"),
        (string("", None), false, "\"\""),
        (string("😀", None), true, "\"😀\""),
        (
            Expression::NullLiteral(NullLiteral { cv: None }),
            false,
            "null",
        ),
    ];
    assert_eq!(cases.len(), 12);
    for (mut test, truthy, label) in cases {
        let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
        let ids: Vec<_> = (0..4).map(|_| cv.try_create(None).unwrap()).collect();
        match &mut test {
            Expression::BooleanLiteral(n) => n.cv = Some(ids[1].clone()),
            Expression::NumericLiteral(n) => n.cv = Some(ids[1].clone()),
            Expression::StringLiteral(n) => n.cv = Some(ids[1].clone()),
            Expression::NullLiteral(n) => n.cv = Some(ids[1].clone()),
            _ => unreachable!(),
        }
        let input = choose(
            test,
            Expression::Identifier(Identifier {
                cv: Some(ids[2].clone()),
                name: "chosen".into(),
            }),
            Expression::Identifier(Identifier {
                cv: Some(ids[3].clone()),
                name: "other".into(),
            }),
            truthy,
            Some(&ids[0]),
        );
        let out = run(&input, &mut cv);
        owned(
            &cv,
            extract_expr(&out.program),
            &ids[..3],
            &format!("({label}) ? ... : ..."),
            "chosen",
        );
    }
}

fn find_fold(cv: &CVLog, contributions: &[Contribution], after: &str) -> String {
    let rows: Vec<_> = contributions
        .iter()
        .filter(|c| c.source == "constant-fold" && c.tag == "folded" && c.meta["after"] == after)
        .collect();
    assert_eq!(rows.len(), 1, "exact earlier fold to {after}");
    let id = rows[0].meta["new_cv"].as_str().unwrap().to_owned();
    assert_eq!(cv.get(&id).unwrap().contributions[0].meta["new_cv"], id);
    id
}

#[test]
fn c38_prior_test_and_chosen_child_rewrites_are_parents_unchosen_activity_remains() {
    for truthy in [false, true] {
        for parent_present in [false, true] {
            let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
            let ids: Vec<_> = (0..9).map(|_| cv.try_create(None).unwrap()).collect();
            let add = |a: f64, b: f64, left: usize, right: usize| {
                Expression::BinaryExpression(BinaryExpression {
                    cv: None,
                    operator: BinaryOperator::Add,
                    left: Box::new(num(a, Some(&ids[left]))),
                    right: Box::new(num(b, Some(&ids[right]))),
                })
            };
            let chosen = Expression::CallExpression(CallExpression {
                cv: Some(ids[3].clone()),
                callee: Box::new(Expression::Identifier(Identifier {
                    cv: Some(ids[4].clone()),
                    name: "observe".into(),
                })),
                arguments: vec![add(1.0, 2.0, 5, 6)],
            });
            let input = choose(
                add(if truthy { 1.0 } else { 0.0 }, 0.0, 1, 2),
                chosen,
                add(4.0, 5.0, 7, 8),
                truthy,
                parent_present.then_some(ids[0].as_str()),
            );
            let out = run(&input, &mut cv);
            let result = extract_expr(&out.program);
            let Expression::CallExpression(call) = result else {
                panic!("chosen call")
            };
            let Expression::NumericLiteral(argument) = &call.arguments[0] else {
                panic!("rewritten child")
            };
            assert_eq!(argument.value.to_bits(), 3.0f64.to_bits());
            let child = argument.cv.as_ref().unwrap();
            let test = find_fold(&cv, &out.contributions, if truthy { "1" } else { "0" });
            let unchosen = find_fold(&cv, &out.contributions, "9");
            let mut expected = Vec::new();
            if parent_present {
                expected.push(ids[0].clone());
            }
            expected.extend([test, ids[3].clone(), ids[4].clone(), child.clone()]);
            owned(
                &cv,
                result,
                &expected,
                if truthy {
                    "(1) ? ... : ..."
                } else {
                    "(0) ? ... : ..."
                },
                "<expr>",
            );
            let root = root_cv(result).unwrap();
            assert!(reaches(&cv, &root, child));
            assert!(!reaches(&cv, &root, &unchosen));
            assert_eq!(
                cv.get(&cv.get(child).unwrap().parent_ids[0])
                    .unwrap()
                    .parent_ids,
                ids[5..7]
            );
            assert_eq!(
                cv.get(&cv.get(&unchosen).unwrap().parent_ids[0])
                    .unwrap()
                    .parent_ids,
                ids[7..9]
            );
        }
    }
}

#[test]
fn c38_selected_function_body_preserves_nested_cvs_and_reaches_child_rewrite() {
    for truthy in [false, true] {
        let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
        let ids: Vec<_> = (0..10).map(|_| cv.try_create(None).unwrap()).collect();
        let selected = Expression::FunctionExpression(FunctionExpression {
            cv: Some(ids[2].clone()),
            id: None,
            params: vec![],
            generator: false,
            is_async: false,
            body: BlockStatement {
                cv: Some(ids[3].clone()),
                body: vec![Statement::expression_statement(ExpressionStatement {
                    cv: Some(ids[4].clone()),
                    expression: Expression::CallExpression(CallExpression {
                        cv: Some(ids[5].clone()),
                        callee: Box::new(Expression::Identifier(Identifier {
                            cv: Some(ids[6].clone()),
                            name: "observe".into(),
                        })),
                        arguments: vec![Expression::BinaryExpression(BinaryExpression {
                            cv: None,
                            operator: BinaryOperator::Add,
                            left: Box::new(num(1.0, Some(&ids[7]))),
                            right: Box::new(num(2.0, Some(&ids[8]))),
                        })],
                    }),
                })],
            },
        });
        let input = choose(
            boolean(truthy, Some(&ids[1])),
            selected,
            Expression::Identifier(Identifier {
                cv: Some(ids[9].clone()),
                name: "other".into(),
            }),
            truthy,
            Some(&ids[0]),
        );
        let out = run(&input, &mut cv);
        let result = extract_expr(&out.program);
        let Expression::FunctionExpression(function) = result else {
            panic!("chosen function")
        };
        assert_eq!(function.body.cv.as_ref(), Some(&ids[3]));
        let Statement::Tagged(TaggedStatement::ExpressionStatement(statement)) =
            &function.body.body[0]
        else {
            panic!("body")
        };
        assert_eq!(statement.cv.as_ref(), Some(&ids[4]));
        let Expression::CallExpression(call) = &statement.expression else {
            panic!("call")
        };
        assert_eq!(call.cv.as_ref(), Some(&ids[5]));
        let Expression::NumericLiteral(argument) = &call.arguments[0] else {
            panic!("folded argument")
        };
        assert_eq!(argument.value.to_bits(), 3.0f64.to_bits());
        let child = argument.cv.as_ref().unwrap();
        let mut expected = ids[..7].to_vec();
        expected.push(child.clone());
        owned(
            &cv,
            result,
            &expected,
            if truthy {
                "(true) ? ... : ..."
            } else {
                "(false) ? ... : ..."
            },
            "<expr>",
        );
        assert!(reaches(&cv, &root_cv(result).unwrap(), child));
        assert!(!reaches(&cv, &root_cv(result).unwrap(), &ids[9]));
    }
}

#[test]
fn c38_unsupported_test_families_keep_unequal_conditional_branches() {
    let tests = vec![
        ident("unknown"),
        Expression::UndefinedLiteral(UndefinedLiteral { cv: None }),
        Expression::BigIntLiteral(coding_adventures_javascript_ast::BigIntLiteral {
            cv: None,
            value: "1".into(),
            raw: "1n".into(),
        }),
        Expression::RegExpLiteral(coding_adventures_javascript_ast::RegExpLiteral {
            cv: None,
            pattern: "a".into(),
            flags: "".into(),
        }),
        Expression::ArrayExpression(ArrayExpression {
            cv: None,
            elements: vec![],
        }),
        Expression::ObjectExpression(ObjectExpression {
            cv: None,
            properties: vec![],
        }),
    ];
    for test in tests {
        let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
        let id = cv.try_create(None).unwrap();
        let input = choose(test, ident("a"), ident("b"), true, Some(&id));
        let out = run(&input, &mut cv);
        assert!(matches!(
            extract_expr(&out.program),
            Expression::ConditionalExpression(_)
        ));
        assert!(extract_expr(&out.program).eq_ignoring_cv(extract_expr(&input)));
        assert!(out.contributions.is_empty());
    }
}

#[test]
fn c38_unchosen_child_checked_refusals_latch_before_outer_selection() {
    for truthy in [false, true] {
        for missing_parent in [false, true] {
            // The missing-parent control leaves ample capacity for the outer
            // selection; the other control refuses a genuine child node budget.
            let limits = if missing_parent {
                GraphLimits::default()
            } else {
                GraphLimits {
                    max_nodes: 5,
                    ..Default::default()
                }
            };
            let mut cv = CVLog::new_checked_chronology(limits).unwrap();
            let ids: Vec<_> = (0..5).map(|_| cv.try_create(None).unwrap()).collect();
            let right = if missing_parent {
                "missing-parent"
            } else {
                &ids[4]
            };
            let entries_before = serde_json::to_value(cv.entries()).unwrap();
            let journal_before = serde_json::to_value(cv.journal().unwrap().events()).unwrap();
            let expected = cv.try_merge(&[ids[3].as_str(), right], None).unwrap_err();
            assert_eq!(serde_json::to_value(cv.entries()).unwrap(), entries_before);
            assert_eq!(
                serde_json::to_value(cv.journal().unwrap().events()).unwrap(),
                journal_before
            );
            let input = choose(
                boolean(truthy, Some(&ids[1])),
                Expression::Identifier(Identifier {
                    cv: Some(ids[2].clone()),
                    name: "chosen".into(),
                }),
                Expression::BinaryExpression(BinaryExpression {
                    cv: None,
                    operator: BinaryOperator::Add,
                    left: Box::new(num(4.0, Some(&ids[3]))),
                    right: Box::new(num(5.0, Some(right))),
                }),
                truthy,
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
            assert_eq!(error.message, format!("CV fold merge failed: {expected}"));
            // Neither the refused child nor the later outer selection may add
            // identities, journal events, or contributions after the latch.
            assert_eq!(serde_json::to_value(cv.entries()).unwrap(), entries_before);
            assert_eq!(
                serde_json::to_value(cv.journal().unwrap().events()).unwrap(),
                journal_before
            );
            if missing_parent {
                let outer = cv
                    .try_merge(&[ids[0].as_str(), ids[1].as_str(), ids[2].as_str()], None)
                    .unwrap();
                cv.try_derive(&outer, None).unwrap();
            }
        }
    }
}

#[test]
fn c38_checked_outer_merge_derive_and_event_refusals_reject_the_candidate() {
    for truthy in [false, true] {
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
                max_edges: 2,
                ..Default::default()
            },
            GraphLimits {
                max_events: 0,
                ..Default::default()
            },
        ] {
            let mut cv = CVLog::new_checked_compact(limits).unwrap();
            let ids: Vec<_> = (0..3).map(|_| cv.try_create(None).unwrap()).collect();
            let input = choose(
                boolean(truthy, Some(&ids[1])),
                Expression::Identifier(Identifier {
                    cv: Some(ids[2].clone()),
                    name: "chosen".into(),
                }),
                ident("other"),
                truthy,
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
