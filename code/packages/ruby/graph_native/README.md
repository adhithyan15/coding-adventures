# coding_adventures_graph_native

Rust-backed DT00 graph package for Ruby.

It mirrors the pure Ruby graph API while delegating storage and algorithms to
the shared `rust/graph` crate.

## Features

- Adjacency-list and adjacency-matrix representations
- Weighted undirected edges
- BFS, DFS, connectivity, connected components, cycle detection
- Shortest path and minimum spanning tree

## Example

```ruby
require "coding_adventures_graph_native"

graph = CodingAdventures::GraphNative::Graph.new
graph.add_edge("London", "Paris", 300.0)
graph.add_edge("Paris", "Berlin", 878.0)

p CodingAdventures::GraphNative.shortest_path(graph, "London", "Berlin")
```
