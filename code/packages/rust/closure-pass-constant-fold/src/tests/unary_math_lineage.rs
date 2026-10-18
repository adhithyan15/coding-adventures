//! Fixed five-slot Math evidence starts at the actual replacement, never a
//! token that merely happens to exist elsewhere in the graph.
use super::*;
use coding_adventures_correlation_vector::GraphLimits;
use serde_json::{json, Value};
use std::collections::BTreeSet;

struct Case {
    method: &'static str,
    input: f64,
    output: f64,
    raw: &'static str,
    before: &'static str,
}

fn cases() -> [Case; 6] {
    [
        Case {
            method: "abs",
            input: -5.25,
            output: 5.25,
            raw: "5.25",
            before: "Math.abs(-5.25)",
        },
        Case {
            method: "floor",
            input: 4.75,
            output: 4.0,
            raw: "4",
            before: "Math.floor(4.75)",
        },
        Case {
            method: "ceil",
            input: 4.25,
            output: 5.0,
            raw: "5",
            before: "Math.ceil(4.25)",
        },
        Case {
            method: "round",
            input: -2.5,
            output: -2.0,
            raw: "-2",
            before: "Math.round(-2.5)",
        },
        Case {
            method: "trunc",
            input: -4.75,
            output: -4.0,
            raw: "-4",
            before: "Math.trunc(-4.75)",
        },
        Case {
            method: "sign",
            input: -4.25,
            output: -1.0,
            raw: "-1",
            before: "Math.sign(-4.25)",
        },
    ]
}

fn named(name: &str, cv: Option<&str>) -> Expression {
    Expression::Identifier(Identifier {
        cv: cv.map(str::to_owned),
        name: name.into(),
    })
}

// Slot order is declared independently of any production frontier collector.
fn call(method: &str, slots: [Option<&str>; 5], argument: Expression) -> Expression {
    Expression::CallExpression(CallExpression {
        cv: slots[0].map(str::to_owned),
        callee: Box::new(Expression::MemberExpression(MemberExpression {
            cv: slots[1].map(str::to_owned),
            object: Box::new(named("Math", slots[2])),
            property: Box::new(named(method, slots[3])),
            computed: false,
        })),
        arguments: vec![argument],
    })
}

fn plain_call(case: &Case, slots: [Option<&str>; 5]) -> Expression {
    call(case.method, slots, num(case.input, slots[4]))
}

fn run(expr: Expression, cv: &mut CVLog) -> PassOutput {
    let p = program_with_expr(expr, false);
    ConstantFoldPass::new()
        .run(PassContext {
            program: &p,
            sidecar: &Sidecar::new(),
            cv,
        })
        .unwrap()
}

fn snapshot(cv: &CVLog) -> Value {
    // JournalRecord deliberately is not Serialize. Use the real public export.
    serde_json::from_str(&cv.to_json_string().unwrap()).unwrap()
}

fn roots(cv: &mut CVLog, count: usize) -> Vec<String> {
    (0..count).map(|_| cv.try_create(None).unwrap()).collect()
}

fn state(cv: &mut CVLog) -> FoldState<'_> {
    FoldState {
        cv,
        contributions: vec![],
        changed: false,
        nodes_touched: 0,
        cv_error: None,
    }
}

fn ancestors(cv: &CVLog, root: &str) -> BTreeSet<String> {
    // Independent small-fixture DFS also rejects dangling parents and cycles.
    fn visit(cv: &CVLog, id: &str, active: &mut BTreeSet<String>, done: &mut BTreeSet<String>) {
        assert!(active.insert(id.to_owned()), "cycle at {id}");
        let entry = cv.get(id).expect("every traversed parent exists");
        for parent in &entry.parent_ids {
            if !done.contains(parent) {
                visit(cv, parent, active, done);
            }
        }
        active.remove(id);
        done.insert(id.to_owned());
    }
    let mut done = BTreeSet::new();
    visit(cv, root, &mut BTreeSet::new(), &mut done);
    done.remove(root);
    done
}

fn owned(
    cv: &CVLog,
    expr: &Expression,
    expected: &[String],
    before: &str,
    value: f64,
    raw: &str,
) -> Option<String> {
    let Expression::NumericLiteral(n) = expr else {
        panic!("numeric result: {expr:?}");
    };
    assert_eq!(n.value.to_bits(), value.to_bits());
    assert_eq!(n.raw, raw);
    let mut data = json!({"type": "NumericLiteral", "value": value, "raw": raw});
    if let Some(id) = &n.cv {
        data.as_object_mut().unwrap().insert("cv".into(), json!(id));
    }
    assert_eq!(serde_json::to_value(expr).unwrap(), data);
    if expected.is_empty() {
        assert!(n.cv.is_none(), "no manufactured identity");
        return None;
    }
    let id =
        n.cv.as_ref()
            .expect("actual returned replacement owns its event");
    assert!(!expected.contains(id), "not an old operand identity");
    let entry = cv.get(id).unwrap();
    assert_eq!(entry.parent_ids.len(), 1);
    assert_eq!(entry.contributions.len(), 1);
    let fact = &entry.contributions[0];
    assert_eq!(fact.source, "constant-fold");
    assert_eq!(fact.tag, "folded");
    assert_eq!(fact.meta["before"], before);
    assert_eq!(fact.meta["after"], raw);
    assert_eq!(fact.meta["new_cv"], id.as_str());
    assert_eq!(fact.meta["parent_cv"], entry.parent_ids[0]);
    if expected.len() == 1 {
        assert_eq!(entry.parent_ids, expected);
    } else {
        let merged = cv.get(&entry.parent_ids[0]).unwrap();
        assert_eq!(merged.parent_ids, expected);
        assert!(merged.contributions.is_empty());
    }
    cv.validate_graph().unwrap();
    Some(id.clone())
}

#[test]
fn c41_absent_call_identity_retains_all_six_numeric_results() {
    let mut completed = 0;
    for case in cases() {
        let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
        let ids = roots(&mut cv, 5);
        let slots = [
            None,
            Some(ids[0].as_str()),
            Some(ids[1].as_str()),
            Some(ids[2].as_str()),
            Some(ids[3].as_str()),
        ];
        let out = run(plain_call(&case, slots), &mut cv);
        let result = owned(
            &cv,
            extract_expr(&out.program),
            &ids[..4],
            case.before,
            case.output,
            case.raw,
        )
        .unwrap();
        assert_eq!(out.contributions.len(), 1);
        let originals: BTreeSet<_> = ids.iter().cloned().collect();
        let expected: BTreeSet<_> = ids[..4].iter().cloned().collect();
        assert_eq!(
            ancestors(&cv, &result)
                .intersection(&originals)
                .cloned()
                .collect::<BTreeSet<_>>(),
            expected
        );
        assert!(
            !ancestors(&cv, &result).contains(&ids[4]),
            "unrelated origin excluded"
        );
        completed += 1;
    }
    assert_eq!(completed, 6);
}

#[test]
fn c41_every_slot_mask_has_exact_order_and_zero_single_multiple_behavior() {
    let mut completed = 0;
    for case in cases() {
        for mask in 0..32 {
            let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
            let ids = roots(&mut cv, 6);
            let slots = std::array::from_fn(|i| (mask & (1 << i) != 0).then_some(ids[i].as_str()));
            let expected: Vec<_> = (0..5)
                .filter(|i| mask & (1 << i) != 0)
                .map(|i| ids[i].clone())
                .collect();
            let old = cv.entries().len();
            let out = run(plain_call(&case, slots), &mut cv);
            let root = owned(
                &cv,
                extract_expr(&out.program),
                &expected,
                case.before,
                case.output,
                case.raw,
            );
            assert!(out.changed);
            assert_eq!(out.contributions.len(), usize::from(mask != 0));
            assert_eq!(
                cv.entries().len() - old,
                if expected.is_empty() {
                    0
                } else if expected.len() == 1 {
                    1
                } else {
                    2
                }
            );
            if let Some(id) = root {
                assert!(!ancestors(&cv, &id).contains(&ids[5]));
                for (i, origin) in ids[..5].iter().enumerate() {
                    assert_eq!(ancestors(&cv, &id).contains(origin), mask & (1 << i) != 0);
                }
            }
            completed += 1;
        }
    }
    assert_eq!(completed, 192);
}

#[test]
fn c41_alias_patterns_preserve_first_seen_order_and_distinct_same_spelling_ids() {
    // Expected root indices are literal tables, not computed by dedup code.
    let tables: [([usize; 5], &[usize]); 6] = [
        ([0, 0, 0, 0, 0], &[0]),
        ([2, 0, 2, 1, 0], &[2, 0, 1]),
        ([4, 3, 2, 1, 0], &[4, 3, 2, 1, 0]),
        ([1, 2, 1, 2, 3], &[1, 2, 3]),
        ([0, 1, 2, 3, 4], &[0, 1, 2, 3, 4]),
        ([3, 3, 1, 1, 2], &[3, 1, 2]),
    ];
    let mut completed = 0;
    for case in cases() {
        for (indices, order) in tables {
            let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
            let ids = roots(&mut cv, 5);
            let slots = indices.map(|i| Some(ids[i].as_str()));
            let expected: Vec<_> = order.iter().map(|i| ids[*i].clone()).collect();
            let out = run(plain_call(&case, slots), &mut cv);
            owned(
                &cv,
                extract_expr(&out.program),
                &expected,
                case.before,
                case.output,
                case.raw,
            );
            completed += 1;
        }
    }
    assert_eq!(completed, 36);
}

#[test]
fn c41_positive_zero_bits_and_canonical_raw_survive_all_six_methods() {
    for case in cases() {
        let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
        let id = cv.try_create(None).unwrap();
        let expression = call(
            case.method,
            [None, None, None, None, Some(&id)],
            num(0.0, Some(&id)),
        );
        let out = run(expression, &mut cv);
        owned(
            &cv,
            extract_expr(&out.program),
            &[id],
            &format!("Math.{}(0)", case.method),
            0.0,
            "0",
        );
        assert_eq!(out.contributions.len(), 1);
    }
}

#[test]
fn c41_sign_infinities_keep_existing_admission_and_exact_five_parent_lineage() {
    // Literal expectations preserve the two original native NumericLiteral
    // inputs; this does not claim a JavaScript parser admits an infinity token.
    let cases = [
        (f64::INFINITY, "inf", "Math.sign(inf)", 1.0f64, "1"),
        (f64::NEG_INFINITY, "-inf", "Math.sign(-inf)", -1.0f64, "-1"),
    ];
    let mut completed = 0;
    for (input, input_raw, before, output, raw) in cases {
        let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
        let ids = roots(&mut cv, 6);
        let slots = std::array::from_fn(|i| Some(ids[i].as_str()));
        let expression = call("sign", slots, num(input, slots[4]));
        let Expression::CallExpression(c) = &expression else {
            unreachable!()
        };
        let Expression::NumericLiteral(n) = &c.arguments[0] else {
            unreachable!()
        };
        assert_eq!(n.value.to_bits(), input.to_bits());
        assert_eq!(n.raw, input_raw);
        let out = run(expression, &mut cv);
        let root = owned(
            &cv,
            extract_expr(&out.program),
            &ids[..5],
            before,
            output,
            raw,
        )
        .unwrap();
        assert_eq!(out.contributions.len(), 1);
        let history = ancestors(&cv, &root);
        assert!(ids[..5].iter().all(|id| history.contains(id)));
        assert!(!history.contains(&ids[5]));
        completed += 1;
    }
    assert_eq!(completed, 2);
}

#[test]
fn c41_zero_and_single_parent_operation_clocks_are_not_fake_merges() {
    for present in [false, true] {
        let mut cv = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
        let id = cv.try_create(None).unwrap();
        let old = snapshot(&cv);
        let slots = [None, None, None, None, present.then_some(id.as_str())];
        let case = &cases()[0];
        let out = run(plain_call(case, slots), &mut cv);
        let expected = if present { vec![id] } else { vec![] };
        let root = owned(
            &cv,
            extract_expr(&out.program),
            &expected,
            case.before,
            case.output,
            case.raw,
        );
        let now = snapshot(&cv);
        let old_events = old["journal"]["events"].as_array().unwrap();
        let events = now["journal"]["events"].as_array().unwrap();
        assert_eq!(&events[..old_events.len()], old_events);
        let added = &events[old_events.len()..];
        if let Some(root) = root {
            assert_eq!(added.len(), 2);
            assert_eq!(added[0]["event"]["kind"], "derive");
            assert_eq!(added[1]["event"]["kind"], "contribution");
            assert_eq!(added[0]["event"]["entity"], root);
            assert_eq!(added[1]["event"]["entity"], root);
        } else {
            assert!(added.is_empty());
            assert_eq!(now, old);
        }
    }
}

#[test]
fn c41_nested_unary_and_binary_children_keep_result_ancestry_and_real_pass_clocks() {
    for unary in [true, false] {
        let mut cv = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
        let ids = roots(&mut cv, 8);
        let (argument, method, child_before, child_after, before, output, raw, child_parents) =
            if unary {
                (
                    Expression::UnaryExpression(UnaryExpression {
                        cv: Some(ids[4].clone()),
                        operator: UnaryOperator::Negate,
                        argument: Box::new(num(5.0, Some(&ids[5]))),
                        prefix: true,
                    }),
                    "abs",
                    "-(5)",
                    "-5",
                    "Math.abs(-5)",
                    5.0,
                    "5",
                    vec![ids[4].clone(), ids[5].clone()],
                )
            } else {
                (
                    Expression::BinaryExpression(BinaryExpression {
                        cv: Some(ids[4].clone()),
                        operator: BinaryOperator::Add,
                        left: Box::new(num(1.0, Some(&ids[5]))),
                        right: Box::new(num(2.75, Some(&ids[6]))),
                    }),
                    "floor",
                    "(1) + (2.75)",
                    "3.75",
                    "Math.floor(3.75)",
                    3.0,
                    "3",
                    vec![ids[4].clone(), ids[5].clone(), ids[6].clone()],
                )
            };
        let slots = [
            Some(ids[0].as_str()),
            Some(ids[1].as_str()),
            Some(ids[2].as_str()),
            Some(ids[3].as_str()),
            None,
        ];
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(ConstantFoldPass::new()));
        let out = pipeline
            .run(
                program_with_expr(call(method, slots, argument), false),
                &Sidecar::new(),
                &mut cv,
            )
            .unwrap();
        let children: Vec<_> = cv
            .entries()
            .values()
            .filter(|entry| {
                entry.contributions.iter().any(|c| {
                    c.source == "constant-fold"
                        && c.tag == "folded"
                        && c.meta["before"] == child_before
                        && c.meta["after"] == child_after
                })
            })
            .collect();
        assert_eq!(children.len(), 1);
        let child = &children[0].id;
        assert_eq!(children[0].contributions[0].meta["new_cv"], child.as_str());
        assert_eq!(
            cv.get(&children[0].parent_ids[0]).unwrap().parent_ids,
            child_parents
        );
        let mut expected = ids[..4].to_vec();
        expected.push(child.clone());
        let root = owned(
            &cv,
            extract_expr(&out.program),
            &expected,
            before,
            output,
            raw,
        )
        .unwrap();
        let history = ancestors(&cv, &root);
        assert!(history.contains(child));
        assert!(child_parents.iter().all(|id| history.contains(id)));
        assert!(!history.contains(&ids[7]));
        let s = snapshot(&cv);
        let events = s["journal"]["events"].as_array().unwrap();
        let event = |kind: &str, id: &str| {
            events
                .iter()
                .position(|e| e["event"]["kind"] == kind && e["event"]["entity"] == id)
                .unwrap()
        };
        let merged = &cv.get(&root).unwrap().parent_ids[0];
        assert!(event("contribution", child) < event("merge", merged));
        assert!(event("merge", merged) < event("derive", &root));
        assert!(event("derive", &root) < event("contribution", &root));
        let contribution = &events[event("contribution", &root)];
        let pass = events
            .iter()
            .find(|e| e["sequence"] == contribution["context"])
            .unwrap();
        assert_eq!(pass["event"]["scope"]["kind"], "pass");
        assert_eq!(pass["event"]["scope"]["sweep"], "0000000000000000");
        let schedule = events
            .iter()
            .find(|e| e["event"]["kind"] == "schedule" && e["context"] == pass["context"])
            .unwrap();
        let slot =
            usize::from_str_radix(pass["event"]["scope"]["slot"].as_str().unwrap(), 16).unwrap();
        assert_eq!(schedule["event"]["passes"][slot]["name"], "constant-fold");
        for (kind, id) in [
            ("contribution", child.as_str()),
            ("merge", merged.as_str()),
            ("derive", root.as_str()),
        ] {
            assert_eq!(events[event(kind, id)]["context"], contribution["context"]);
        }
    }
}

#[test]
fn c41_each_missing_frontier_parent_rejects_the_pass_without_outer_graph_work() {
    let mut completed = 0;
    for case in cases() {
        // Start with an operand-only missing identity, so old production's
        // ignored member CV yields an actual false acceptance, not a label diff.
        for missing in [1, 2, 3, 4, 0] {
            let mut cv = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
            let ids = roots(&mut cv, 5);
            let slots = std::array::from_fn(|i| {
                Some(if i == missing {
                    "absent-math-parent"
                } else {
                    ids[i].as_str()
                })
            });
            let p = program_with_expr(plain_call(&case, slots), false);
            let before = snapshot(&cv);
            let error = ConstantFoldPass::new()
                .run(PassContext {
                    program: &p,
                    sidecar: &Sidecar::new(),
                    cv: &mut cv,
                })
                .unwrap_err();
            assert_eq!(error.pass_name, "constant-fold");
            assert_eq!(
                error.message,
                "CV fold merge failed: unknown checked CV parent"
            );
            assert_eq!(snapshot(&cv), before, "failed merge must be atomic");
            completed += 1;
        }
    }
    assert_eq!(completed, 30);
}

#[test]
fn c41_checked_outer_merge_derive_event_and_payload_limits_reject_candidates() {
    let mut completed = 0;
    for case in cases() {
        // Five roots; independently predict which outer operation is refused.
        for (limits, message, added) in [
            (
                GraphLimits {
                    max_edges: 5,
                    ..Default::default()
                },
                "CV fold derivation failed: CV parent edges limit exceeded",
                1,
            ),
            (
                GraphLimits {
                    max_nodes: 5,
                    ..Default::default()
                },
                "CV fold merge failed: CV nodes limit exceeded",
                0,
            ),
            (
                GraphLimits {
                    max_nodes: 6,
                    ..Default::default()
                },
                "CV fold derivation failed: CV nodes limit exceeded",
                1,
            ),
            (
                GraphLimits {
                    max_edges: 0,
                    ..Default::default()
                },
                "CV fold merge failed: CV parent edges limit exceeded",
                0,
            ),
            (
                GraphLimits {
                    max_events: 0,
                    ..Default::default()
                },
                "CV fold contribution failed: CV events limit exceeded",
                2,
            ),
            (
                GraphLimits {
                    max_metadata_bytes: 0,
                    ..Default::default()
                },
                "CV fold contribution failed: CV metadata bytes limit exceeded",
                2,
            ),
        ] {
            let mut cv = CVLog::new_checked_compact(limits).unwrap();
            let ids = roots(&mut cv, 5);
            let slots = std::array::from_fn(|i| Some(ids[i].as_str()));
            let p = program_with_expr(plain_call(&case, slots), false);
            let old = snapshot(&cv);
            let error = ConstantFoldPass::new()
                .run(PassContext {
                    program: &p,
                    sidecar: &Sidecar::new(),
                    cv: &mut cv,
                })
                .unwrap_err();
            assert_eq!(error.pass_name, "constant-fold");
            assert_eq!(error.message, message);
            assert_eq!(cv.entries().len(), 5 + added);
            assert!(cv.entries().values().all(|e| e.contributions.is_empty()));
            let now = snapshot(&cv);
            for id in &ids {
                assert_eq!(now["entries"][id], old["entries"][id]);
            }
            cv.validate_graph().unwrap();
            completed += 1;
        }
        for (limits, message, added) in [
            (
                GraphLimits {
                    max_nodes: 1,
                    ..Default::default()
                },
                "CV fold derivation failed: CV nodes limit exceeded",
                0,
            ),
            (
                GraphLimits {
                    max_edges: 0,
                    ..Default::default()
                },
                "CV fold derivation failed: CV parent edges limit exceeded",
                0,
            ),
            (
                GraphLimits {
                    max_events: 0,
                    ..Default::default()
                },
                "CV fold contribution failed: CV events limit exceeded",
                1,
            ),
            (
                GraphLimits {
                    max_metadata_bytes: 0,
                    ..Default::default()
                },
                "CV fold contribution failed: CV metadata bytes limit exceeded",
                1,
            ),
        ] {
            let mut cv = CVLog::new_checked_compact(limits).unwrap();
            let id = cv.try_create(None).unwrap();
            let p = program_with_expr(
                plain_call(&case, [None, None, None, None, Some(&id)]),
                false,
            );
            let error = ConstantFoldPass::new()
                .run(PassContext {
                    program: &p,
                    sidecar: &Sidecar::new(),
                    cv: &mut cv,
                })
                .unwrap_err();
            assert_eq!(error.message, message);
            assert_eq!(cv.entries().len(), 1 + added);
            assert!(cv.entries().values().all(|e| e.contributions.is_empty()));
            cv.validate_graph().unwrap();
            completed += 1;
        }
    }
    assert_eq!(completed, 60);
}

#[test]
fn c41_actual_child_failure_and_preexisting_error_prevent_all_outer_activity() {
    for binary in [false, true] {
        let mut cv = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
        let ids = roots(&mut cv, 5);
        let argument = if binary {
            Expression::BinaryExpression(BinaryExpression {
                cv: None,
                operator: BinaryOperator::Add,
                left: Box::new(num(1.0, Some(&ids[4]))),
                right: Box::new(num(2.0, Some("missing-child"))),
            })
        } else {
            Expression::UnaryExpression(UnaryExpression {
                cv: None,
                operator: UnaryOperator::Negate,
                argument: Box::new(num(5.0, Some("missing-child"))),
                prefix: true,
            })
        };
        let slots = [
            Some(ids[0].as_str()),
            Some(ids[1].as_str()),
            Some(ids[2].as_str()),
            Some(ids[3].as_str()),
            None,
        ];
        let p = program_with_expr(call("abs", slots, argument), false);
        let before = snapshot(&cv);
        let error = ConstantFoldPass::new()
            .run(PassContext {
                program: &p,
                sidecar: &Sidecar::new(),
                cv: &mut cv,
            })
            .unwrap_err();
        assert_eq!(
            error.message,
            if binary {
                "CV fold merge failed: unknown checked CV parent"
            } else {
                "CV fold derivation failed: unknown checked CV parent"
            }
        );
        assert_eq!(snapshot(&cv), before);
    }
    // A previously latched error is authoritative even with every outer parent valid.
    let mut cv = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
    let ids = roots(&mut cv, 5);
    let slots = std::array::from_fn(|i| Some(ids[i].as_str()));
    let Expression::CallExpression(c) = plain_call(&cases()[0], slots) else {
        unreachable!()
    };
    let before = snapshot(&cv);
    let mut st = state(&mut cv);
    st.cv_error = Some("earlier checked child failure".into());
    let _ = fold_call(&c, &mut st);
    assert_eq!(
        st.cv_error.as_deref(),
        Some("earlier checked child failure")
    );
    assert!(st.contributions.is_empty());
    assert_eq!(snapshot(st.cv), before);
}

#[test]
fn c41_argument_failure_keeps_prior_callee_selection_and_the_first_error() {
    let mut cv = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
    let ids = roots(&mut cv, 5);
    let selected_receiver = Expression::ConditionalExpression(ConditionalExpression {
        cv: Some(ids[2].clone()),
        test: Box::new(boolean(true, None)),
        consequent: Box::new(named("Math", Some(&ids[3]))),
        alternate: Box::new(named("other", None)),
    });
    let input = Expression::CallExpression(CallExpression {
        cv: Some(ids[0].clone()),
        callee: Box::new(Expression::MemberExpression(MemberExpression {
            cv: Some(ids[1].clone()),
            object: Box::new(selected_receiver),
            property: Box::new(named("abs", Some(&ids[4]))),
            computed: false,
        })),
        arguments: vec![Expression::UnaryExpression(UnaryExpression {
            cv: None,
            operator: UnaryOperator::Negate,
            prefix: true,
            argument: Box::new(num(5.0, Some("missing-later-argument"))),
        })],
    });
    let p = program_with_expr(input, false);
    let before = snapshot(&cv);
    let error = ConstantFoldPass::new()
        .run(PassContext {
            program: &p,
            sidecar: &Sidecar::new(),
            cv: &mut cv,
        })
        .unwrap_err();
    assert_eq!(
        error.message,
        "CV fold derivation failed: unknown checked CV parent"
    );
    let after = snapshot(&cv);
    for id in &ids {
        assert_eq!(after["entries"][id], before["entries"][id]);
    }
    assert_eq!(
        cv.entries().len(),
        7,
        "only the successful callee merge and derive survive"
    );
    let old_events = before["journal"]["events"].as_array().unwrap();
    let events = after["journal"]["events"].as_array().unwrap();
    assert_eq!(&events[..old_events.len()], old_events);
    let added = &events[old_events.len()..];
    assert_eq!(added.len(), 3);
    assert_eq!(added[0]["event"]["kind"], "merge");
    assert_eq!(added[1]["event"]["kind"], "derive");
    assert_eq!(added[2]["event"]["kind"], "contribution");
    let merged = added[0]["event"]["entity"].as_str().unwrap();
    let child = added[1]["event"]["entity"].as_str().unwrap();
    assert_eq!(
        cv.get(merged).unwrap().parent_ids,
        [ids[2].clone(), ids[3].clone()]
    );
    assert_eq!(cv.get(child).unwrap().parent_ids, [merged]);
    assert_eq!(cv.get(child).unwrap().contributions.len(), 1);
    assert_eq!(added[2]["event"]["entity"], child);
    assert!(!cv.get(child).unwrap().contributions[0].meta["before"]
        .as_str()
        .unwrap()
        .starts_with("Math."));
    cv.validate_graph().unwrap();
}

#[test]
fn c41_declined_arity_receiver_nonliteral_nonfinite_and_signed_zero_keep_data() {
    let mut completed = 0;
    for case in cases() {
        for kind in 0..10 {
            // Existing admission is result-finite, not input-finite: sign(±Inf)
            // is tested separately as the two preserved positive fixtures.
            if case.method == "sign" && (kind == 5 || kind == 6) {
                continue;
            }
            let mut cv = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
            let ids = roots(&mut cv, 5);
            let slots = std::array::from_fn(|i| Some(ids[i].as_str()));
            let Expression::CallExpression(mut c) = plain_call(&case, slots) else {
                unreachable!()
            };
            match kind {
                0 => c.arguments.clear(),
                1 => c.arguments.push(num(9.0, None)),
                2 => c.arguments[0] = named("unknown", Some(&ids[4])),
                3 => {
                    let Expression::MemberExpression(m) = c.callee.as_mut() else {
                        unreachable!()
                    };
                    *m.object = named("notMath", Some(&ids[2]));
                }
                4 => c.arguments[0] = num(f64::NAN, Some(&ids[4])),
                5 => c.arguments[0] = num(f64::INFINITY, Some(&ids[4])),
                6 => c.arguments[0] = num(f64::NEG_INFINITY, Some(&ids[4])),
                7 => c.arguments[0] = num(-0.0, Some(&ids[4])),
                8 => {
                    let Expression::MemberExpression(m) = c.callee.as_mut() else {
                        unreachable!()
                    };
                    m.computed = true;
                }
                9 => {
                    let Expression::MemberExpression(m) = c.callee.as_mut() else {
                        unreachable!()
                    };
                    *m.property = named("notARecognizedMethod", Some(&ids[3]));
                }
                _ => unreachable!(),
            }
            let input = Expression::CallExpression(c);
            let data = serde_json::to_value(&input).unwrap();
            let before = snapshot(&cv);
            let out = run(input, &mut cv);
            let actual = extract_expr(&out.program);
            assert!(matches!(actual, Expression::CallExpression(_)));
            assert_eq!(serde_json::to_value(actual).unwrap(), data);
            if (4..8).contains(&kind) {
                let Expression::CallExpression(c) = actual else {
                    unreachable!()
                };
                let Expression::NumericLiteral(n) = &c.arguments[0] else {
                    unreachable!()
                };
                let expected = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.0][kind - 4];
                assert_eq!(n.value.to_bits(), expected.to_bits());
                assert_eq!(n.raw, expected.to_string());
            }
            assert!(!out.changed);
            assert!(out.contributions.is_empty());
            assert_eq!(snapshot(&cv), before);
            completed += 1;
        }
    }
    // Negative inputs that round/truncate toward zero stay declined too.
    for method in ["ceil", "round", "trunc"] {
        let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
        let id = cv.try_create(None).unwrap();
        let input = call(
            method,
            [Some(&id), None, None, None, None],
            num(-0.25, None),
        );
        let data = serde_json::to_value(&input).unwrap();
        let before = snapshot(&cv);
        let out = run(input, &mut cv);
        assert_eq!(
            serde_json::to_value(extract_expr(&out.program)).unwrap(),
            data
        );
        assert!(!out.changed);
        assert_eq!(snapshot(&cv), before);
    }
    assert_eq!(completed, 58);
}

#[test]
fn c41_declined_effectful_argument_retains_child_work_without_an_outer_fact() {
    let mut cv = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
    let ids = roots(&mut cv, 7);
    let effect = Expression::CallExpression(CallExpression {
        cv: Some(ids[4].clone()),
        callee: Box::new(named("effect", Some(&ids[5]))),
        arguments: vec![Expression::BinaryExpression(BinaryExpression {
            cv: None,
            operator: BinaryOperator::Add,
            left: Box::new(num(1.0, Some(&ids[6]))),
            right: Box::new(num(2.0, None)),
        })],
    });
    let slots = [
        Some(ids[0].as_str()),
        Some(ids[1].as_str()),
        Some(ids[2].as_str()),
        Some(ids[3].as_str()),
        None,
    ];
    let out = run(call("abs", slots, effect), &mut cv);
    let Expression::CallExpression(outer) = extract_expr(&out.program) else {
        panic!("Math call retained");
    };
    assert_eq!(outer.cv.as_ref(), Some(&ids[0]));
    let Expression::CallExpression(effect) = &outer.arguments[0] else {
        panic!("effect call retained");
    };
    assert_eq!(effect.cv.as_ref(), Some(&ids[4]));
    assert_eq!(
        serde_json::to_value(&effect.callee).unwrap(),
        json!({"type":"Identifier","name":"effect","cv":ids[5]})
    );
    let child = owned(&cv, &effect.arguments[0], &ids[6..7], "(1) + (2)", 3.0, "3").unwrap();
    assert_eq!(out.contributions.len(), 1);
    assert_eq!(out.contributions[0].meta["new_cv"], child);
    assert!(cv.get(&ids[0]).unwrap().contributions.is_empty());
    assert!(!out.contributions.iter().any(|c| c.meta["before"]
        .as_str()
        .is_some_and(|s| s.starts_with("Math."))));
    cv.validate_graph().unwrap();
}
