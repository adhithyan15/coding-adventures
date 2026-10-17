//! CLOC37 inspects the actual returned root and its retained child rewrites.
use super::*;
use std::collections::BTreeSet;

fn immediate<'a>(cv: &'a CVLog, result: &Expression) -> &'a [String] {
    let root = root_cv(result).expect("actual selection root");
    let entry = cv.get(&root).unwrap();
    assert_eq!(entry.contributions.len(), 1);
    assert_eq!(entry.contributions[0].meta["new_cv"], root);
    assert_eq!(entry.parent_ids.len(), 1);
    &cv.get(&entry.parent_ids[0]).unwrap().parent_ids
}

fn reaches(cv: &CVLog, start: &str, target: &str) -> bool {
    // Bounded test fixture, independent of the production AST traversal.
    let mut pending = vec![start.to_owned()];
    let mut seen = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if id == target {
            return true;
        }
        if seen.insert(id.clone()) {
            pending.extend(cv.get(&id).unwrap().parent_ids.iter().cloned());
        }
    }
    false
}

#[test]
fn c37_actual_selected_call_root_reaches_retained_argument_rewrite_in_exact_order() {
    for string_key in [false, true] {
        for root_present in [false, true] {
            let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
            let ids: Vec<_> = (0..8).map(|_| cv.try_create(None).unwrap()).collect();
            let selected = Expression::CallExpression(CallExpression {
                cv: root_present.then(|| ids[3].clone()),
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
            let input = pick(
                vec![Some(selected), Some(num(9.0, Some(&ids[7])))],
                if string_key {
                    string("0", Some(&ids[2]))
                } else {
                    num(0.0, Some(&ids[2]))
                },
                &ids,
            );
            let out = run(&input, &mut cv);
            let result = extract_expr(&out.program);
            let Expression::CallExpression(call) = result else {
                panic!("call preserved")
            };
            let Expression::Identifier(callee) = call.callee.as_ref() else {
                panic!("callee")
            };
            let Expression::NumericLiteral(argument) = &call.arguments[0] else {
                panic!("argument folded")
            };
            assert_eq!(callee.cv.as_deref(), Some(ids[4].as_str()));
            assert_eq!(callee.name, "observe");
            assert_eq!(argument.value.to_bits(), 3.0f64.to_bits());
            let child = argument
                .cv
                .as_ref()
                .expect("actual retained derived argument");
            let child_entry = cv.get(child).unwrap();
            assert_eq!(child_entry.contributions.len(), 1);
            assert_eq!(child_entry.contributions[0].meta["new_cv"], *child);
            assert_eq!(
                cv.get(&child_entry.parent_ids[0]).unwrap().parent_ids,
                ids[5..7]
            );
            let mut expected = ids[..3].to_vec();
            if root_present {
                expected.push(ids[3].clone());
            }
            expected.extend([ids[4].clone(), child.clone()]);
            assert_eq!(
                immediate(&cv, result),
                expected,
                "member,array,actual key,selected root,callee,actual rewritten argument"
            );
            let root = root_cv(result).unwrap();
            assert!(
                reaches(&cv, &root, child),
                "returned root reaches actual child rewrite"
            );
            assert!(
                !reaches(&cv, &root, &ids[7]),
                "unselected native semantic root excluded"
            );
        }
    }
}

fn stamp_all_native_objects(value: &mut Value, cv: &mut CVLog) {
    match value {
        Value::Object(object) => {
            for child in object.values_mut() {
                stamp_all_native_objects(child, cv);
            }
            // Test-only wire construction: roundtrip below discards fields on
            // objects that do not actually carry a native CV. The oracle reads
            // back the actual typed subtree, never assumes insertion succeeded.
            object.insert("cv".into(), json!(cv.try_create(None).unwrap()));
        }
        Value::Array(items) => {
            for child in items {
                stamp_all_native_objects(child, cv);
            }
        }
        _ => {}
    }
}

fn identity_set(value: &Value, ids: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            if let Some(id) = object.get("cv").and_then(Value::as_str) {
                ids.insert(id.into());
            }
            for child in object.values() {
                identity_set(child, ids);
            }
        }
        Value::Array(items) => {
            for child in items {
                identity_set(child, ids);
            }
        }
        _ => {}
    }
}

#[test]
fn c37_nonempty_native_expression_census_preserves_and_joins_all_retained_ids() {
    let mut selected_shapes = 0;
    let mut positive_nested_shapes = 0;
    for string_key in [false, true] {
        for (name, expression) in shapes(None) {
            if name == "SpreadElement" {
                continue;
            } // Existing eligibility unchanged.
            let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
            let operands: Vec<_> = (0..4).map(|_| cv.try_create(None).unwrap()).collect();
            let mut wire = serde_json::to_value(expression).unwrap();
            stamp_all_native_objects(&mut wire, &mut cv);
            let selected: Expression = serde_json::from_value(wire).unwrap();
            let before = serde_json::to_value(&selected).unwrap();
            let mut selected_ids = BTreeSet::new();
            identity_set(&before, &mut selected_ids);
            assert!(!selected_ids.is_empty(), "{name} actually stamped");
            positive_nested_shapes += usize::from(selected_ids.len() > 1);
            let input = pick(
                vec![Some(selected), Some(num(9.0, Some(&operands[3])))],
                if string_key {
                    string("0", Some(&operands[2]))
                } else {
                    num(0.0, Some(&operands[2]))
                },
                &operands,
            );
            let out = run(&input, &mut cv);
            let result = extract_expr(&out.program);
            let mut after = serde_json::to_value(result).unwrap();
            let mut expected_payload = before;
            after.as_object_mut().unwrap().remove("cv");
            expected_payload.as_object_mut().unwrap().remove("cv");
            assert_eq!(
                after, expected_payload,
                "{name} nested CV and payload conservation"
            );
            let mut expected = selected_ids.clone();
            expected.extend(operands[..3].iter().cloned());
            let frontier = immediate(&cv, result);
            assert_eq!(
                frontier.len(),
                expected.len(),
                "{name} no duplicate parents"
            );
            assert_eq!(
                frontier.iter().cloned().collect::<BTreeSet<_>>(),
                expected,
                "{name} exact selected ID set"
            );
            assert_eq!(&frontier[..3], &operands[..3], "fixed operand prefix");
            let root = root_cv(result).unwrap();
            assert!(
                selected_ids.iter().all(|id| reaches(&cv, &root, id)),
                "{name} actual root reachability"
            );
            selected_shapes += 1;
        }
    }
    assert_eq!(selected_shapes, 70);
    assert!(
        positive_nested_shapes >= 40,
        "nonvacuous stamped composite census"
    );
}
