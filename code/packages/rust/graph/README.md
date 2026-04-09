# graph

An undirected graph data structure library. Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational project.

## What is an undirected graph?

An undirected graph is a set of nodes connected by edges, where each edge has no direction — it connects two nodes symmetrically. Think of it like a two-way street map: if you can travel from A to B, you can also travel from B to A.

In an undirected graph, edges are mutual relationships. For example:
- In a social network, friendships are mutual (if Alice is friends with Bob, Bob is friends with Alice)
- In a road network, roads go both ways
- In a peer-to-peer network, connections are symmetric

## Features

- **Graph construction** -- add and remove nodes and edges
- **Neighbor queries** -- find all neighbors of a node
- **Degree queries** -- find how many neighbors a node has
- **Traversal ready** -- foundation for BFS and DFS exploration

## Usage

```rust
use graph::Graph;

let mut g = Graph::new();

// Add edges (creates nodes automatically)
g.add_edge("A", "B").unwrap();
g.add_edge("B", "C").unwrap();
g.add_edge("A", "C").unwrap();

// Query the graph
assert!(g.has_edge("A", "B"));
assert!(g.has_edge("B", "A")); // undirected: both directions

// Find neighbors
let neighbors = g.neighbors("A").unwrap();
assert_eq!(neighbors.len(), 2);

// Degree (number of neighbors)
assert_eq!(g.degree("A").unwrap(), 2);
```

## How it fits in the stack

This undirected graph is a foundational data structure used alongside the directed-graph package. While directed-graph is used for build system dependency resolution, the undirected graph is useful for:
- Social networks and connection analysis
- Game map pathfinding
- Network topology analysis
- Peer-to-peer network modeling

## Error handling

All fallible operations return `Result<T, GraphError>`. The error variants are:
- `GraphError::NodeNotFound(node)` -- referenced node doesn't exist
- `GraphError::EdgeNotFound(from, to)` -- referenced edge doesn't exist
- `GraphError::SelfLoop(node)` -- self-loops are not allowed
