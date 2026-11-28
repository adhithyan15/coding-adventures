mod common;

use coding_adventures_test_bootstrap::{
    pbt::{check_property, PbtOptions},
    prng::SplitMix64,
    Verdict,
};
use common::{check_graph_case, verify_path_claim, GraphCase, NODES, PAIRS};

fn generate_case(rng: &mut SplitMix64) -> Result<GraphCase, String> {
    let mut edges = Vec::new();
    for (left, right) in PAIRS {
        let choice = rng.bounded(4);
        if choice != 0 {
            edges.push((left, right, (choice - 1) as u8));
        }
    }
    Ok(GraphCase { edges })
}

#[test]
fn seeded_graphs_match_independent_floyd_warshall_oracle() {
    const SEED: u64 = 0xD700_1001_2026;
    let options = PbtOptions {
        num_tests: 512,
        max_retries: 64,
        max_shrink_steps: 256,
        is_ci: false,
    };
    let verdict = check_property(
        SEED,
        &options,
        generate_case,
        |case| check_graph_case(case).is_ok(),
        |candidate, property| {
            let mut current = candidate;
            let mut steps = 0;
            let mut index = 0;
            while index < current.edges.len() && steps < 256 {
                let mut smaller = current.clone();
                smaller.edges.remove(index);
                if !property(&smaller) {
                    current = smaller;
                    steps += 1;
                } else {
                    index += 1;
                }
            }
            (current, steps)
        },
    );
    if verdict != Verdict::Pass {
        let mut replay = SplitMix64::new(SEED);
        for iteration in 1..=options.num_tests {
            let case = generate_case(&mut replay).expect("generator is infallible");
            if let Err(detail) = check_graph_case(&case) {
                panic!("seed={SEED:#x} iteration={iteration} graph={case:?}: {detail}");
            }
        }
    }
    assert_eq!(verdict, Verdict::Pass, "seed={SEED:#x}: {verdict:?}");
}

#[test]
fn property_rejects_a_deliberately_nonoptimal_path_claim() {
    let case = GraphCase {
        edges: vec![(0, 1, 1), (1, 2, 1), (0, 2, 3)],
    };
    let wrong_claim = vec![NODES[0].to_string(), NODES[2].to_string()];
    assert!(!verify_path_claim(
        &case,
        graph::GraphRepr::AdjacencyList,
        0,
        2,
        &wrong_claim,
    ));
}
