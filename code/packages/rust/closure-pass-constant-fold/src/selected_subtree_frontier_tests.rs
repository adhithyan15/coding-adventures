//! Tests-first budget models for CLOC37. Wired by the later implementation.
use super::*;

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
