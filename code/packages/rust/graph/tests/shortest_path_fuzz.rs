mod common;

use coding_adventures_test_bootstrap::fuzz::{
    fuzz_target, CoverageBitmap, FuzzOptions, FuzzOutcome,
};
use common::{check_graph_case, GraphCase, NODES, PAIRS};
use graph::{shortest_path, try_shortest_path, GraphRepr};

fn decode(input: &[u8]) -> (GraphCase, Option<(usize, usize, f64)>) {
    let mut edges = Vec::new();
    let mut invalid = None;
    for (index, (left, right)) in PAIRS.into_iter().enumerate() {
        let byte = input.get(index).copied().unwrap_or(0);
        let weight = match byte {
            0 => continue,
            1 => 0.0,
            2 => 1.0,
            3 => 2.0,
            4 => 3.0,
            5 => 4.0,
            6 => -1.0,
            7 => f64::NAN,
            8 => f64::INFINITY,
            9 => f64::NEG_INFINITY,
            10 => -0.0,
            other => f64::from(other % 5),
        };
        if !weight.is_finite() || weight < 0.0 {
            invalid = Some((left, right, weight));
        } else {
            edges.push((left, right, weight as u8));
        }
    }
    (GraphCase { edges }, invalid)
}

#[test]
fn replayable_fuzz_target_checks_valid_and_invalid_graph_queries() {
    const SEED: u64 = 0xD700_1002_2026;
    let corpus = vec![
        vec![0; 6],
        vec![1; 6],
        vec![2; 6],
        vec![3; 6],
        vec![6, 0, 0, 0, 0, 0],
        vec![7, 0, 0, 0, 0, 0],
        vec![8, 0, 0, 0, 0, 0],
        vec![9, 0, 0, 0, 0, 0],
        vec![10, 0, 0, 0, 0, 0],
    ];
    let options = FuzzOptions {
        max_iterations: 2_048,
        seed: SEED,
        max_input_len: 6,
        base_energy: 4,
    };
    let report = fuzz_target(&corpus, &options, |input, coverage: &mut CoverageBitmap| {
        let (case, invalid) = decode(input);
        coverage.record_edge(usize::from(invalid.is_some()));
        coverage.record_edge(2 + usize::from(input.first().copied().unwrap_or(0) >= 6));
        if input.iter().all(|byte| *byte == 0) {
            coverage.record_edge(4);
        }

        if let Some((left, right, weight)) = invalid {
            for repr in [GraphRepr::AdjacencyList, GraphRepr::AdjacencyMatrix] {
                let mut graph = common::build_graph(&case, repr);
                graph.add_edge(NODES[left], NODES[right], weight);
                if try_shortest_path(&graph, NODES[0], NODES[0]).is_ok()
                    || !shortest_path(&graph, NODES[0], NODES[1]).is_empty()
                {
                    return FuzzOutcome::Crash {
                        kind: "invalid-weight-accepted".into(),
                        frames: vec![format!("{repr:?}")],
                    };
                }
            }
        } else if let Err(detail) = check_graph_case(&case) {
            return FuzzOutcome::Crash {
                kind: "oracle-disagreement".into(),
                frames: vec![detail],
            };
        }
        FuzzOutcome::Pass
    });

    assert_eq!(
        report.iterations, options.max_iterations,
        "seed={SEED:#x}; report={report:?}"
    );
    assert!(report.is_clean(), "seed={SEED:#x}; report={report:?}");
    assert!(
        report.total_edges >= 5,
        "semantic feedback categories were not exercised: {report:?}"
    );
    assert!(
        report.admitted_seeds >= 2,
        "valid and invalid seed classes were not retained: {report:?}"
    );
}
