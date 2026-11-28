# graph

Weighted undirected graph for Rust with both adjacency-list and adjacency-matrix storage.

## What It Provides

- `Graph` with `GraphRepr::AdjacencyList` and `GraphRepr::AdjacencyMatrix`
- Weighted undirected edges, including self-loops
- `bfs`, `dfs`, `is_connected`, `connected_components`, `has_cycle`
- `shortest_path` and `minimum_spanning_tree`
- `TraversalGraph` for reusing BFS/DFS helpers from other graph-shaped crates

## Usage

```rust
use graph::{minimum_spanning_tree, shortest_path, Graph, GraphRepr};

let mut g = Graph::new(GraphRepr::AdjacencyList);
g.add_node_with_properties("London", [(
    "kind".to_string(),
    graph::GraphPropertyValue::String("city".to_string()),
)].into_iter().collect());
g.add_edge("London", "Paris", 300.0);
g.add_edge("London", "Amsterdam", 520.0);
g.add_edge("Amsterdam", "Berlin", 655.0);

assert_eq!(
    g.edge_properties("Paris", "London").unwrap().get("weight"),
    Some(&graph::GraphPropertyValue::Number(300.0))
);

assert_eq!(
    shortest_path(&g, "London", "Berlin"),
    vec!["London".to_string(), "Amsterdam".to_string(), "Berlin".to_string()]
);

let mst = minimum_spanning_tree(&g).unwrap();
assert!(!mst.is_empty());
```

Graph, node, and edge property bags are cloned on read. The canonical edge
property `weight` is kept in sync with `edge_weight` and weighted algorithms.

## Checked shortest paths

Stored weights must be finite and nonnegative for Dijkstra. The legacy
`shortest_path` function keeps its `Vec<String>` signature and returns an empty
vector if a stored edge violates that precondition. Call `try_shortest_path` to
receive `InvalidPathWeight` and distinguish invalid graph weights from a missing
or unreachable route:

```rust
use graph::{try_shortest_path, Graph, GraphRepr, InvalidPathWeight};

fn main() -> Result<(), InvalidPathWeight> {
    let mut graph = Graph::new(GraphRepr::AdjacencyList);
    graph.add_edge("A", "B", 1.0);
    let _path = try_shortest_path(&graph, "A", "B")?;
    Ok(())
}
```

The query checks all stored edges before endpoint shortcuts. It does not reject
weights during mutation; the infallible add APIs remain source compatible.
Minimum spanning trees continue to accept negative finite weights. Sums of large
finite edge weights can still overflow; that behavior is a separate follow-up.

The verification suite uses the repository's seeded property engine and
byte-replayable fuzz engine. Property seed `0xD70010012026` generates 512
four-node graphs with each edge absent or weighted 0, 1, or 2, then checks both
representations against Floyd-Warshall for every ordered pair. Fuzz seed
`0xD70010022026` runs 2,048 mutations of bounded graph bytes. Its adapter reports
semantic input categories; it does not claim native source-branch coverage. A
bounded SAT miter checks all 64 three-node graphs with each edge absent or weighted
0, 1, or 2, in both representations and all nine ordered queries, enumerating
simple direct and one-middle paths. It replays SAT witnesses and includes a
wrong-cost control. UNSAT answers are bounded CDCL results, not certificate-checked
E5 proofs.

## Building and Testing

```bash
cargo test -p graph -- --nocapture
```
