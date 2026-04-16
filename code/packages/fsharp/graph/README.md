# graph

An undirected weighted graph for F# with two interchangeable storage models:
adjacency lists for sparse graphs and adjacency matrices for dense ones.

## What it provides

- `Graph<'T>` with `GraphRepr.AdjacencyList` and `GraphRepr.AdjacencyMatrix`
- weighted undirected edges, including self-loops
- traversal algorithms: BFS, DFS, connectivity, components, cycle detection
- path and tree helpers: shortest path and minimum spanning tree

## Literate source

The implementation is written in the repo's Knuth-style literate format. The
goal is that someone learning graph theory or data-structure tradeoffs can read
[Graph.fs](./Graph.fs) and understand both the code and the design decisions.

## Development

```bash
bash BUILD
```
