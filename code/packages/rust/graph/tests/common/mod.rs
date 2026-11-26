use graph::{shortest_path, try_shortest_path, Graph, GraphRepr};

#[derive(Clone, Debug)]
pub struct GraphCase {
    pub edges: Vec<(usize, usize, u8)>,
}

pub const NODES: [&str; 4] = ["A", "B", "C", "D"];
pub const PAIRS: [(usize, usize); 6] = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];

pub fn build_graph(case: &GraphCase, repr: GraphRepr) -> Graph {
    let mut graph = Graph::new(repr);
    for node in NODES {
        graph.add_node(node);
    }
    for &(left, right, weight) in &case.edges {
        graph.add_edge(NODES[left], NODES[right], f64::from(weight));
    }
    graph
}

pub fn oracle(case: &GraphCase) -> [[u32; 4]; 4] {
    const INF: u32 = u32::MAX / 4;
    let mut distances = [[INF; 4]; 4];
    for (index, row) in distances.iter_mut().enumerate() {
        row[index] = 0;
    }
    for &(left, right, weight) in &case.edges {
        distances[left][right] = u32::from(weight);
        distances[right][left] = u32::from(weight);
    }
    for middle in 0..4 {
        for start in 0..4 {
            for end in 0..4 {
                let through = distances[start][middle] + distances[middle][end];
                distances[start][end] = distances[start][end].min(through);
            }
        }
    }
    distances
}

pub fn verify_path_claim(
    case: &GraphCase,
    repr: GraphRepr,
    start: usize,
    end: usize,
    path: &[String],
) -> bool {
    let distance = oracle(case)[start][end];
    if distance >= u32::MAX / 4 {
        return path.is_empty();
    }
    if path.first().map(String::as_str) != Some(NODES[start])
        || path.last().map(String::as_str) != Some(NODES[end])
    {
        return false;
    }
    let graph = build_graph(case, repr);
    let mut cost = 0u32;
    for pair in path.windows(2) {
        let Ok(weight) = graph.edge_weight(&pair[0], &pair[1]) else {
            return false;
        };
        cost += weight as u32;
    }
    cost == distance
}

pub fn check_graph_case(case: &GraphCase) -> Result<(), String> {
    for repr in [GraphRepr::AdjacencyList, GraphRepr::AdjacencyMatrix] {
        let graph = build_graph(case, repr);
        for (start, &start_name) in NODES.iter().enumerate() {
            for (end, &end_name) in NODES.iter().enumerate() {
                let checked = try_shortest_path(&graph, start_name, end_name)
                    .map_err(|error| format!("{repr:?} {start}->{end}: {error}"))?;
                let compatibility = shortest_path(&graph, start_name, end_name);
                if checked != compatibility {
                    return Err(format!(
                        "{repr:?} {start}->{end}: checked {checked:?} differs from compatibility {compatibility:?}"
                    ));
                }
                if !verify_path_claim(case, repr, start, end, &checked) {
                    return Err(format!(
                        "{repr:?} {start}->{end}: returned path {checked:?} disagrees with Floyd-Warshall"
                    ));
                }
            }
        }
    }
    Ok(())
}
