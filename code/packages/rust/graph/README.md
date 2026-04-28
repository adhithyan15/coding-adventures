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

## Building and Testing

```bash
cargo test -p graph -- --nocapture
```
