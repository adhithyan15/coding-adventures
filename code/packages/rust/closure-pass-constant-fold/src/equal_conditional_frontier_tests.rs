//! Independent C39 multi-root work/storage formulas, not observed thresholds.
use super::*;
fn leaf(cv: Option<&str>) -> Expression {
    Expression::Identifier(Identifier {
        cv: cv.map(str::to_owned),
        name: "x".into(),
    })
}
fn bytes(count: usize, buckets: usize) -> usize {
    4 * std::mem::size_of::<Frame<'_>>()
        + count * std::mem::size_of::<&str>()
        + buckets * std::mem::size_of::<Option<&str>>()
}
#[test]
fn c39_three_leaf_collision_work_and_combined_storage_are_independently_104() {
    let trees = [leaf(Some("a")), leaf(Some("b")), leaf(Some("c"))];
    let roots = [&trees[0], &trees[1], &trees[2]];
    // Conditional absent: count3*(3prefix+6frame+1slot+1byte)+growth1=34.
    // Combined raw count3 =>8buckets, initialization10. Replay structural33,
    // then constant-hash visits/probes/equality6+9+12=27. Total34+10+60=104.
    let mut allocations = Vec::new();
    assert_eq!(
        collect_roots_with(
            [None; 3],
            &roots,
            104,
            bytes(3, 8),
            |_| 0,
            |n| {
                allocations.push(n);
                Ok(())
            }
        )
        .unwrap(),
        ["a", "b", "c"]
    );
    assert_eq!(
        allocations,
        [
            4 * std::mem::size_of::<Frame<'_>>(),
            3 * std::mem::size_of::<&str>(),
            8 * std::mem::size_of::<Option<&str>>()
        ]
    );
    assert!(
        collect_roots_with([None; 3], &roots, 103, bytes(3, 8), |_| 0, |_| Ok(()))
            .unwrap_err()
            .contains("work")
    );
    assert!(
        collect_roots_with([None; 3], &roots, 104, bytes(3, 8) - 1, |_| 0, |_| Ok(()))
            .unwrap_err()
            .contains("buffer")
    );
}
#[test]
fn c39_conditional_prefix_is_charged_once_per_count_and_replay() {
    let trees = [leaf(Some("a")), leaf(Some("b")), leaf(Some("c"))];
    let roots = [&trees[0], &trees[1], &trees[2]];
    // Present one-byte parent adds one count byte:35. Four IDs =>8buckets+2.
    // Replay structural34 +4 collided visits6+9+12+15=42. Total35+10+76=121.
    assert_eq!(
        collect_roots_with(
            [Some("p"), None, None],
            &roots,
            121,
            bytes(4, 8),
            |_| 0,
            |_| Ok(())
        )
        .unwrap(),
        ["p", "a", "b", "c"]
    );
    assert!(collect_roots_with(
        [Some("p"), None, None],
        &roots,
        120,
        bytes(4, 8),
        |_| 0,
        |_| Ok(())
    )
    .unwrap_err()
    .contains("work"));
}
#[test]
fn c39_three_absent_trees_are_all_counted_in_31_without_dedup_allocations() {
    let trees = [leaf(None), leaf(None), leaf(None)];
    let roots = [&trees[0], &trees[1], &trees[2]];
    let mut allocations = Vec::new();
    // Per tree3prefix+6frames+1slot=10, initial stack allocation setup1.
    assert!(collect_roots_with(
        [None; 3],
        &roots,
        31,
        bytes(0, 0),
        |_| panic!("no IDs to hash"),
        |n| {
            allocations.push(n);
            Ok(())
        }
    )
    .unwrap()
    .is_empty());
    assert_eq!(allocations, [4 * std::mem::size_of::<Frame<'_>>()]);
    assert!(collect_roots_with(
        [None; 3],
        &roots,
        30,
        bytes(0, 0),
        |_| panic!("no IDs"),
        |_| Ok(())
    )
    .unwrap_err()
    .contains("work"));
    let late = leaf(Some("b"));
    assert_eq!(
        collect_roots_with(
            [None; 3],
            &[&trees[0], &late, &trees[2]],
            usize::MAX,
            usize::MAX,
            |_| 0,
            |_| Ok(())
        )
        .unwrap(),
        ["b"]
    );
}
#[test]
fn c39_cross_tree_duplicate_still_purchases_combined_raw_capacity() {
    let trees = [leaf(Some("a")), leaf(Some("a")), leaf(Some("b"))];
    let roots = [&trees[0], &trees[1], &trees[2]];
    let mut allocations = Vec::new();
    // Count34/init10 remain. Replay structural33 + visits6+8+9=23 =>100.
    assert_eq!(
        collect_roots_with(
            [None; 3],
            &roots,
            100,
            bytes(3, 8),
            |_| 0,
            |n| {
                allocations.push(n);
                Ok(())
            }
        )
        .unwrap(),
        ["a", "b"]
    );
    assert_eq!(
        allocations,
        [
            4 * std::mem::size_of::<Frame<'_>>(),
            3 * std::mem::size_of::<&str>(),
            8 * std::mem::size_of::<Option<&str>>()
        ]
    );
    assert!(
        collect_roots_with([None; 3], &roots, 99, bytes(3, 8), |_| 0, |_| Ok(()))
            .unwrap_err()
            .contains("work")
    );
}
#[test]
fn c39_each_individual_allowance_does_not_authorize_three_independent_budgets() {
    let trees = [leaf(Some("a")), leaf(Some("b")), leaf(Some("c"))];
    let roots = [&trees[0], &trees[1], &trees[2]];
    for tree in &trees {
        assert_eq!(
            collect_with([None; 3], tree, 33, bytes(1, 2), |_| 0, |_| Ok(()))
                .unwrap()
                .len(),
            1
        );
    }
    assert!(
        collect_roots_with([None; 3], &roots, 33, usize::MAX, |_| 0, |_| Ok(()))
            .unwrap_err()
            .contains("work")
    );
    assert!(collect_roots_with(
        [None; 3],
        &roots,
        usize::MAX,
        bytes(1, 2),
        |_| 0,
        |_| Ok(())
    )
    .unwrap_err()
    .contains("buffer"));
}
#[test]
fn c39_all_three_aggregate_allocations_refuse_before_use() {
    let trees = [leaf(Some("a")), leaf(Some("b")), leaf(Some("c"))];
    let roots = [&trees[0], &trees[1], &trees[2]];
    for rejected in 0..3 {
        let mut calls = 0;
        let error = collect_roots_with(
            [None; 3],
            &roots,
            104,
            bytes(3, 8),
            |_| 0,
            |_| {
                let index = calls;
                calls += 1;
                if index == rejected {
                    Err(format!("injected allocation {index}"))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert_eq!(error, format!("injected allocation {rejected}"));
        assert_eq!(calls, rejected + 1);
    }
}
fn dispose(expression: Expression) {
    let program = Program::new(
        "C39 fixture".into(),
        coding_adventures_javascript_tokens::EsVersion::Es2025,
        SourceType::Script,
    )
    .with_body(vec![ProgramItem::Statement(
        Statement::expression_statement(ExpressionStatement {
            cv: None,
            expression,
        }),
    )]);
    dispose_program(program);
}
#[test]
fn c39_multi_root_deep_and_wide_scans_reuse_borrowed_frames_on_small_stack() {
    std::thread::Builder::new()
        .stack_size(144 * 1024)
        .spawn(|| {
            let mut deep = leaf(Some("bottom"));
            for _ in 0..4096 {
                deep = Expression::ArrayExpression(ArrayExpression {
                    cv: None,
                    elements: vec![None, Some(deep), None],
                });
            }
            let wide = Expression::ArrayExpression(ArrayExpression {
                cv: Some("root".into()),
                elements: (0..100_000)
                    .map(|i| {
                        if i % 2 == 0 {
                            Some(leaf(Some("same")))
                        } else {
                            None
                        }
                    })
                    .collect(),
            });
            let last = leaf(Some("last"));
            assert_eq!(
                collect_equal_conditional(None, &deep, &wide, &last).unwrap(),
                ["bottom", "root", "same", "last"]
            );
            dispose(deep);
            dispose(wide);
            dispose(last);
        })
        .unwrap()
        .join()
        .unwrap();
}
