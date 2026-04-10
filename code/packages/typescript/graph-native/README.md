# @coding-adventures/graph-native

Rust-backed DT00 graph package for Node.js and TypeScript.

This package wraps the shared `rust/graph` crate and exposes the same
high-level graph operations as the pure TypeScript graph package, with
string-backed nodes.

## Features

- Adjacency-list and adjacency-matrix storage
- Weighted undirected edges
- BFS, DFS, connectivity, connected components, cycle detection
- Shortest path and minimum spanning tree

## Example

```ts
import { Graph, GraphRepr, shortestPath } from "@coding-adventures/graph-native";

const graph = new Graph(GraphRepr.ADJACENCY_LIST);
graph.addEdge("London", "Paris", 300);
graph.addEdge("Paris", "Berlin", 878);

console.log(shortestPath(graph, "London", "Berlin"));
```
