use cdcl_sat::{Lit, SolveResult, Solver, Var};
use graph::{shortest_path, Graph, GraphRepr};

const LABELS: [&str; 3] = ["A", "B", "C"];
const EDGES: [(usize, usize); 3] = [(0, 1), (0, 2), (1, 2)];

#[derive(Debug)]
struct Route {
    nodes: Vec<usize>,
    cost: u8,
    edges_present: bool,
}

fn routes(weights: &[[Option<u8>; 3]; 3], start: usize, end: usize) -> Vec<Route> {
    let mut routes = Vec::new();
    if start == end {
        routes.push(Route {
            nodes: vec![start],
            cost: 0,
            edges_present: true,
        });
        return routes;
    }
    let mut candidates = vec![vec![start, end]];
    for middle in 0..3 {
        if middle != start && middle != end {
            candidates.push(vec![start, middle, end]);
        }
    }
    for nodes in candidates {
        let mut cost = 0u8;
        let mut edges_present = true;
        for pair in nodes.windows(2) {
            match weights[pair[0]][pair[1]] {
                Some(weight) => cost += weight,
                None => edges_present = false,
            }
        }
        routes.push(Route {
            nodes,
            cost,
            edges_present,
        });
    }
    routes
}

/// A positional bounded miter: route-choice variables denote the ordered
/// positions (start, optional middle, destination); clauses require each
/// adjacent transition to be an edge, select one finite cost bucket, and ask
/// whether any path cost is below `strict_bound`.
fn sat_path_below(
    weights: &[[Option<u8>; 3]; 3],
    start: usize,
    end: usize,
    strict_bound: u8,
) -> (SolveResult, Option<Route>) {
    let candidates = routes(weights, start, end);
    let mut solver = Solver::new();
    let route_vars: Vec<Var> = candidates.iter().map(|_| solver.new_var()).collect();
    let cost_vars: Vec<Var> = (0..=4).map(|_| solver.new_var()).collect();

    solver.add_clause(
        &route_vars
            .iter()
            .map(|var| var.positive())
            .collect::<Vec<Lit>>(),
    );
    for left in 0..route_vars.len() {
        for right in (left + 1)..route_vars.len() {
            solver.add_clause(&[route_vars[left].negative(), route_vars[right].negative()]);
        }
    }
    solver.add_clause(
        &cost_vars
            .iter()
            .map(|var| var.positive())
            .collect::<Vec<Lit>>(),
    );
    for left in 0..cost_vars.len() {
        for right in (left + 1)..cost_vars.len() {
            solver.add_clause(&[cost_vars[left].negative(), cost_vars[right].negative()]);
        }
    }
    for (route, var) in candidates.iter().zip(&route_vars) {
        if route.edges_present {
            solver.add_clause(&[var.negative(), cost_vars[route.cost as usize].positive()]);
        } else {
            solver.add_clause(&[var.negative()]);
        }
    }
    solver.add_clause(
        &cost_vars
            .iter()
            .enumerate()
            .filter(|(cost, _)| *cost < strict_bound as usize)
            .map(|(_, var)| var.positive())
            .collect::<Vec<Lit>>(),
    );

    let result = solver.solve();
    let witness = if result == SolveResult::Sat {
        candidates
            .into_iter()
            .zip(route_vars)
            .find_map(|(route, var)| solver.model_value(var).unwrap_or(false).then_some(route))
    } else {
        None
    };
    (result, witness)
}

fn replay_route(weights: &[[Option<u8>; 3]; 3], route: &Route) -> u8 {
    let mut cost = 0;
    for pair in route.nodes.windows(2) {
        cost += weights[pair[0]][pair[1]].expect("SAT witness transition must be an edge");
    }
    cost
}

fn build_graph(states: &[usize], repr: GraphRepr) -> ([[Option<u8>; 3]; 3], Graph) {
    let mut weights = [[None; 3]; 3];
    let mut graph = Graph::new(repr);
    for label in LABELS {
        graph.add_node(label);
    }
    for (index, (left, right)) in EDGES.iter().copied().enumerate() {
        let state = states[index];
        if state > 0 {
            let weight = (state - 1) as u8;
            weights[left][right] = Some(weight);
            weights[right][left] = Some(weight);
            graph.add_edge(LABELS[left], LABELS[right], f64::from(weight));
        }
    }
    (weights, graph)
}

#[test]
fn bounded_sat_miter_checks_three_node_graphs_and_replays_wrong_claims() {
    for topology in 0..4usize.pow(3) {
        let mut value = topology;
        let mut states = [0usize; 3];
        for state in &mut states {
            *state = value % 4;
            value /= 4;
        }
        for repr in [GraphRepr::AdjacencyList, GraphRepr::AdjacencyMatrix] {
            let (weights, graph) = build_graph(&states, repr);
            for (start, &start_label) in LABELS.iter().enumerate() {
                for (end, &end_label) in LABELS.iter().enumerate() {
                    let path = shortest_path(&graph, start_label, end_label);
                    if path.is_empty() {
                        let (result, witness) = sat_path_below(&weights, start, end, 5);
                        assert_eq!(
                            result,
                            SolveResult::Unsat,
                            "topology={states:?} {repr:?} {start}->{end}"
                        );
                        assert!(witness.is_none());
                    } else {
                        let cost: u8 = path
                            .windows(2)
                            .map(|edge| graph.edge_weight(&edge[0], &edge[1]).unwrap() as u8)
                            .sum();
                        let (result, witness) = sat_path_below(&weights, start, end, cost);
                        assert_eq!(
                            result,
                            SolveResult::Unsat,
                            "topology={states:?} {repr:?} {path:?}"
                        );
                        assert!(witness.is_none());
                    }
                }
            }
        }
    }

    let states = [2, 0, 2]; // A--B=1, A--C absent, B--C=1
    let (weights, _) = build_graph(&states, GraphRepr::AdjacencyList);
    let (result, witness) = sat_path_below(&weights, 0, 2, 3); // deliberately claim cost 3
    assert_eq!(result, SolveResult::Sat);
    let witness = witness.expect("SAT model must select a path");
    assert_eq!(witness.nodes.first(), Some(&0));
    assert_eq!(witness.nodes.last(), Some(&2));
    assert!(witness.edges_present);
    assert_eq!(replay_route(&weights, &witness), witness.cost);
    assert_eq!(witness.cost, 2);
    assert!(witness.cost < 3);
}
