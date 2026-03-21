# directed-graph

A directed graph library with algorithms for topological sorting, cycle detection, and parallel execution level computation. Part of the [coding-adventures](https://github.com/your-repo/coding-adventures) educational project.

## What is a directed graph?

A directed graph (or "digraph") is a set of nodes connected by edges, where each edge has a direction -- it goes FROM one node TO another. Think of it like a one-way street map: you can travel from A to B, but that doesn't mean you can travel from B to A.

In a build system, nodes are packages and edges are dependencies: if package A depends on package B, there's an edge from B to A (B must be built before A).

## Features

- **Topological sort** (Kahn's algorithm) -- order nodes so every dependency comes before the things that depend on it
- **Cycle detection** (DFS three-color algorithm) -- detect circular dependencies
- **Transitive closure** -- find all nodes reachable from a given node
- **Affected nodes** -- given a set of changed nodes, find everything that needs rebuilding
- **Independent groups** -- partition nodes into levels for parallel execution

## Usage

```rust
use directed_graph::Graph;

let mut g = Graph::new();

// Build a diamond dependency graph:
//   A -> B -> D
//   A -> C -> D
g.add_edge("A", "B").unwrap();
g.add_edge("A", "C").unwrap();
g.add_edge("B", "D").unwrap();
g.add_edge("C", "D").unwrap();

// Topological sort gives a valid build order
let order = g.topological_sort().unwrap();
assert_eq!(order, vec!["A", "B", "C", "D"]);

// Independent groups show what can run in parallel
let groups = g.independent_groups().unwrap();
// Level 0: [A]      -- no dependencies
// Level 1: [B, C]   -- can run in parallel
// Level 2: [D]      -- depends on B and C
assert_eq!(groups[1], vec!["B", "C"]);
```

## How it fits in the stack

This is a foundational data structure library used by the build tool to manage package dependency graphs. It sits alongside `logic-gates`, `arithmetic`, and other educational packages in the coding-adventures project.

The build tool uses:
- `add_edge` to construct the dependency graph from BUILD files
- `has_cycle` to verify no circular dependencies exist
- `affected_nodes` to determine what needs rebuilding after a change
- `independent_groups` to parallelize builds

## Internal design

The graph maintains two adjacency maps (forward and reverse) for efficient lookups in both directions. This doubles memory but makes all traversals O(V+E).

## Error handling

All fallible operations return `Result<T, GraphError>`. The error variants are:
- `GraphError::CycleError` -- graph contains a cycle
- `GraphError::NodeNotFound(node)` -- referenced node doesn't exist
- `GraphError::EdgeNotFound(from, to)` -- referenced edge doesn't exist
- `GraphError::SelfLoop(node)` -- self-loops are not allowed
