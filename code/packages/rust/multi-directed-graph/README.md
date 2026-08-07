# Multi Directed Graph

A generic Rust multi-directed graph implementing
[DT02](../../../specs/DT02-multi-directed-graph.md). It keeps insertion-ordered
nodes, stable edge IDs, parallel directed edges, optional self-loops, numeric
edge weights, and metadata property bags on the graph, nodes, and edges.

The package is domain-neutral. The `neural-network` crate uses it as a graph
substrate while keeping inputs, constants, weighted sums, activations, and
outputs in the neural layer.

```rust
use multi_directed_graph::{MultiDirectedGraph, PropertyBag};

let mut graph = MultiDirectedGraph::new();
let left = graph
    .add_edge("x", "sum", 0.25, PropertyBag::new(), Some("w0".into()))
    .unwrap();
let right = graph
    .add_edge("x", "sum", 0.75, PropertyBag::new(), Some("w1".into()))
    .unwrap();

assert_eq!((left.as_str(), right.as_str()), ("w0", "w1"));
assert_eq!(graph.edges_between(&"x", &"sum").unwrap().len(), 2);
```

## API

| Operation | Description |
| --- | --- |
| `add_node`, `remove_node`, `has_node`, `nodes` | Manage generic `Eq + Hash + Clone` node values. |
| `add_edge`, `remove_edge`, `has_edge`, `edge`, `edges` | Manage directed edges by stable ID. |
| `edges_between` | Return every parallel edge for one ordered node pair. |
| `outgoing_edges`, `incoming_edges` | Return incident edge records in insertion order. |
| `successors`, `predecessors` | Return unique neighboring nodes. |
| `edge_weight` | Read the canonical numeric edge weight. |
| `graph_properties`, `node_properties`, `edge_properties` | Read cloned metadata bags. |
| `set_*_property`, `remove_*_property` | Mutate graph, node, or edge metadata. |
| `topological_sort`, `has_cycle`, `independent_groups` | Run multiplicity-aware DAG algorithms. |

All fallible operations return `GraphError`. Setting the `weight` edge property
updates `edge_weight`; removing it resets the weight to `1.0`.
