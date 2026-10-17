//! Tests-first budget models for CLOC37. Wired by the later implementation.
use super::*;

#[path = "selected_subtree_field_fixtures.rs"]
mod populated_fields;

fn leaf(cv: Option<&str>) -> Expression {
    Expression::Identifier(Identifier {
        cv: cv.map(str::to_owned),
        name: "x".into(),
    })
}

#[test]
fn c37_leaf_budget_is_derived_from_both_scans_not_measured_from_success() {
    // Three absent prefix slots. First walk: initial frame growth setup1,
    // two pushes2, two pops2, two dispatches2, root slot1, ID byte1.
    // Dedup setup2 + two initialized buckets2. Replay: same three prefix
    // slots, two pushes/pops/dispatches6, root slot1 + byte1, hash setup4
    // + byte1 + one empty probe1. Total 3+9+4+3+14 =33.
    let work = 33;
    let bytes = 4 * std::mem::size_of::<Frame<'_>>()
        + std::mem::size_of::<&str>()
        + 2 * std::mem::size_of::<Option<&str>>();
    let expression = leaf(Some("a"));
    let mut allocations = Vec::new();
    let result = collect_with(
        [None; 3],
        &expression,
        work,
        bytes,
        |_| 0,
        |n| {
            allocations.push(n);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(result, ["a"]);
    assert_eq!(
        allocations,
        [
            4 * std::mem::size_of::<Frame<'_>>(),
            std::mem::size_of::<&str>(),
            2 * std::mem::size_of::<Option<&str>>()
        ]
    );
    assert!(
        collect_with([None; 3], &expression, work - 1, bytes, |_| 0, |_| Ok(()))
            .unwrap_err()
            .contains("work")
    );
    assert!(
        collect_with([None; 3], &expression, work, bytes - 1, |_| 0, |_| Ok(()))
            .unwrap_err()
            .contains("buffer")
    );
}

#[test]
fn c37_absent_ids_still_charge_visits_and_allocate_no_dedup_buffers() {
    // First scan only: three prefix slots + growth1 + pushes2 + pops2 +
    // dispatches2 + absent root slot1 =11; no replay/hash buffers.
    let expression = leaf(None);
    let bytes = 4 * std::mem::size_of::<Frame<'_>>();
    let mut allocations = Vec::new();
    assert!(collect_with(
        [None; 3],
        &expression,
        11,
        bytes,
        |_| panic!("no available IDs"),
        |n| {
            allocations.push(n);
            Ok(())
        }
    )
    .unwrap()
    .is_empty());
    assert_eq!(allocations, [bytes]);
    assert!(collect_with([None; 3], &expression, 10, bytes, |_| 0, |_| Ok(())).is_err());
}

#[test]
fn c37_frame_growth_counts_old_and_new_requested_storage() {
    let mut expression = leaf(Some("a"));
    for _ in 0..4 {
        expression = Expression::ArrayExpression(ArrayExpression {
            cv: None,
            elements: vec![Some(expression)],
        });
    }
    // Four resumable lists plus the leaf require five live frames. The 4->8
    // migration has twelve requested frames alive simultaneously. Replay
    // reuses eight frames; one ordered ref + two buckets fits the lower peak.
    let frame = std::mem::size_of::<Frame<'_>>();
    let peak = 12 * frame;
    assert!(
        8 * frame + std::mem::size_of::<&str>() + 2 * std::mem::size_of::<Option<&str>>() <= peak
    );
    let mut allocations = Vec::new();
    assert_eq!(
        collect_with(
            [None; 3],
            &expression,
            10_000,
            peak,
            |_| 0,
            |n| {
                allocations.push(n);
                Ok(())
            }
        )
        .unwrap(),
        ["a"]
    );
    assert_eq!(
        allocations,
        [
            4 * frame,
            8 * frame,
            std::mem::size_of::<&str>(),
            2 * std::mem::size_of::<Option<&str>>()
        ]
    );
    assert!(
        collect_with([None; 3], &expression, 10_000, peak - 1, |_| 0, |_| Ok(()))
            .unwrap_err()
            .contains("buffer")
    );
}

#[test]
fn c37_forced_collisions_have_exact_first_seen_order_and_allocation_refusals() {
    let expression = Expression::ArrayExpression(ArrayExpression {
        cv: Some("b".into()),
        elements: vec![None, Some(leaf(Some("a"))), Some(leaf(Some("b")))],
    });
    let expected = ["p", "b", "a"];
    // Independent cell-by-cell model: scan1 prefix5 + walk44 =49;
    // initialization16+2 =18; scan2 prefix5 + walk43 (frames reused)
    // + insertion costs6,8,9,12,11 =94. Walk includes the absent hole and
    // terminal iterator advance. Every collision equality costs both ID bytes.
    let exact_work = 49 + 18 + 94;
    let exact_bytes = 4 * std::mem::size_of::<Frame<'_>>()
        + 5 * std::mem::size_of::<&str>()
        + 16 * std::mem::size_of::<Option<&str>>();
    assert_eq!(
        collect_with(
            [Some("p"), None, Some("p")],
            &expression,
            exact_work,
            exact_bytes,
            |_| 0,
            |_| Ok(())
        )
        .unwrap(),
        expected
    );
    assert!(collect_with(
        [Some("p"), None, Some("p")],
        &expression,
        exact_work - 1,
        exact_bytes,
        |_| 0,
        |_| Ok(())
    )
    .unwrap_err()
    .contains("work"));
    assert!(collect_with(
        [Some("p"), None, Some("p")],
        &expression,
        exact_work,
        exact_bytes - 1,
        |_| 0,
        |_| Ok(())
    )
    .unwrap_err()
    .contains("buffer"));
    assert_eq!(
        collect_with(
            [Some("p"), None, Some("p")],
            &expression,
            10_000,
            100_000,
            |_| 0,
            |_| Ok(())
        )
        .unwrap(),
        expected
    );
    // Frame + ordered refs + hash table each has a real allocation gate.
    // Every gate refuses before hash setup or outer graph mutation can follow.
    for reject in 0..3 {
        let mut calls = 0;
        let error = collect_with(
            [Some("p"), None, Some("p")],
            &expression,
            10_000,
            100_000,
            |_| panic!("allocation refusal precedes replay hashing"),
            |_| {
                let current = calls;
                calls += 1;
                if current == reject {
                    Err("injected allocation refusal".into())
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert!(error.contains("allocation"));
        assert_eq!(calls, reject + 1);
    }
}

#[test]
fn c37_combined_storage_overflow_refuses_before_allocation() {
    let expression = leaf(Some("x"));
    let mut stack = Stack::new();
    stack.fixed_bytes = usize::MAX;
    let mut budget = Budget {
        work: usize::MAX,
        bytes: usize::MAX,
    };
    let error = stack
        .push(Frame::Expression(&expression), &mut budget, &mut |_| {
            panic!("combined arithmetic is checked first")
        })
        .unwrap_err();
    assert!(error.contains("overflow"));
    assert!(stack.frames.is_empty());
}

fn dispose_expression(expression: Expression) {
    let program = Program::new(
        "fixture".into(),
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
fn c37_deep_and_wide_collections_use_bounded_list_frames_on_a_small_stack() {
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
            assert_eq!(collect([None; 3], &deep).unwrap(), ["bottom"]);
            dispose_expression(deep);
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
            assert_eq!(collect([None; 3], &wide).unwrap(), ["root", "same"]);
            dispose_expression(wide);
        })
        .unwrap()
        .join()
        .unwrap();
}
