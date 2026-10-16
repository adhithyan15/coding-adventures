//! Array counts must retain their available operand frontier, not just tokens
//! elsewhere in the log. Composite occurrence coverage remains a separate gap.
use super::*;
use coding_adventures_correlation_vector::GraphLimits;

fn length(
    computed: bool,
    member: Option<&str>,
    array: Option<&str>,
    property: Option<&str>,
    elements: Vec<Option<Expression>>,
) -> Program {
    let property = if computed {
        string("length", property)
    } else {
        Expression::Identifier(Identifier {
            cv: property.map(str::to_owned),
            name: "length".into(),
        })
    };
    program_with_expr(
        Expression::MemberExpression(MemberExpression {
            cv: member.map(str::to_owned),
            object: Box::new(Expression::ArrayExpression(ArrayExpression {
                cv: array.map(str::to_owned),
                elements,
            })),
            property: Box::new(property),
            computed,
        }),
        false,
    )
}

#[test]
fn ordered_frontier_keeps_holes_and_deduplicates_shared_identities() {
    for computed in [false, true] {
        let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
        let ids: Vec<_> = (0..5).map(|_| cv.try_create(None).unwrap()).collect();
        let prog = length(computed, Some(&ids[0]), Some(&ids[1]), Some(&ids[2]), vec![
            Some(num(8.0, Some(&ids[3]))), None,
            Some(num(9.0, Some(&ids[0]))), // Reused member, not a new parent.
            Some(num(10.0, None)), Some(num(11.0, Some(&ids[4]))),
        ]);
        let out = ConstantFoldPass::new().run(PassContext {
            program: &prog, sidecar: &Sidecar::new(), cv: &mut cv,
        }).unwrap();
        let Expression::NumericLiteral(n) = extract_expr(&out.program) else { panic!("count"); };
        assert_eq!(n.value, 5.0, "a hole still counts");
        let id = n.cv.as_deref().expect("result-owned identity");
        let result = cv.get(id).unwrap();
        assert_eq!(result.contributions.len(), 1);
        let fold = &result.contributions[0];
        assert_eq!(fold.source, "constant-fold");
        assert_eq!(fold.tag, "folded");
        assert_eq!(fold.meta["after"], "5");
        assert_eq!(fold.meta["new_cv"], id);
        assert_eq!(result.parent_ids.len(), 1);
        assert_eq!(fold.meta["parent_cv"], result.parent_ids[0]);
        assert_eq!(cv.get(&result.parent_ids[0]).unwrap().parent_ids, ids);
    }
}

#[test]
fn one_primitive_parent_derives_directly_and_missing_parents_stay_untraced() {
    for computed in [false, true] {
        for traced in [false, true] {
            let mut cv = CVLog::new(true);
            let parent = cv.try_create(None).unwrap();
            let prog = length(computed, None, None, None, vec![
                Some(num(7.0, traced.then_some(parent.as_str()))), None,
            ]);
            let out = ConstantFoldPass::new().run(PassContext {
                program: &prog, sidecar: &Sidecar::new(), cv: &mut cv,
            }).unwrap();
            let Expression::NumericLiteral(n) = extract_expr(&out.program) else { panic!("count"); };
            assert_eq!(n.value, 2.0);
            if traced {
                let entry = cv.get(n.cv.as_deref().expect("primitive parent retained")).unwrap();
                assert_eq!(entry.parent_ids, [parent]);
                assert_eq!(entry.contributions.len(), 1);
            } else {
                assert!(n.cv.is_none(), "no invented source identity");
                assert!(out.contributions.is_empty());
            }
        }
    }
}

#[test]
fn wide_frontier_retains_every_parent_in_source_order() {
    let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
    let ids: Vec<_> = (0..1024).map(|_| cv.try_create(None).unwrap()).collect();
    let prog = length(false, None, None, None,
        ids.iter().map(|id| Some(num(7.0, Some(id)))).collect());
    let out = ConstantFoldPass::new().run(PassContext {
        program: &prog, sidecar: &Sidecar::new(), cv: &mut cv,
    }).unwrap();
    let Expression::NumericLiteral(n) = extract_expr(&out.program) else { panic!("count"); };
    assert_eq!(n.value, 1024.0);
    let entry = cv.get(n.cv.as_deref().expect("wide result identity")).unwrap();
    assert_eq!(cv.get(&entry.parent_ids[0]).unwrap().parent_ids, ids);
}

#[test]
fn checked_array_frontier_exhaustion_rejects_the_candidate() {
    // Exercise merge allocation, replacement allocation, merge edges,
    // replacement edges and replacement contribution independently.
    for computed in [false, true] {
        for limits in [
            GraphLimits { max_nodes: 2, ..Default::default() },
            GraphLimits { max_nodes: 3, ..Default::default() },
            GraphLimits { max_edges: 0, ..Default::default() },
            GraphLimits { max_edges: 2, ..Default::default() },
            GraphLimits { max_events: 0, ..Default::default() },
        ] {
            let mut cv = CVLog::new_checked_compact(limits).unwrap();
            let a = cv.try_create(None).unwrap();
            let b = cv.try_create(None).unwrap();
            let prog = length(computed, None, None, None,
                vec![Some(num(1.0, Some(&a))), Some(num(2.0, Some(&b)))]);
            let error = ConstantFoldPass::new().run(PassContext {
                program: &prog, sidecar: &Sidecar::new(), cv: &mut cv,
            }).unwrap_err();
            assert_eq!(error.pass_name, "constant-fold");
            assert!(error.message.contains("CV") && error.message.contains("limit"), "{error}");
        }
    }
}

#[test]
fn declined_lengths_record_no_fictitious_outer_rewrite() {
    for computed in [false, true] {
        for spread in [false, true] {
            let mut cv = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
            let member = cv.try_create(None).unwrap();
            let element = if spread {
                Expression::SpreadElement(SpreadElement { cv: None, argument: Box::new(ident("x")) })
            } else {
                Expression::CallExpression(CallExpression { cv: None, callee: Box::new(ident("g")), arguments: vec![] })
            };
            let prog = length(computed, Some(&member), None, None, vec![Some(element)]);
            let out = ConstantFoldPass::new().run(PassContext {
                program: &prog, sidecar: &Sidecar::new(), cv: &mut cv,
            }).unwrap();
            assert!(matches!(extract_expr(&out.program), Expression::MemberExpression(_)));
            assert!(!out.changed);
            assert!(out.contributions.is_empty());
            assert!(cv.get(&member).unwrap().contributions.is_empty());
        }
    }
}
