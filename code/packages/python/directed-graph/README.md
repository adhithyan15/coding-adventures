# coding-adventures-directed-graph

DT01 — A directed graph library extending `Graph[T]` (DT00). Provides directed edges, reverse adjacency for efficient predecessor lookups, topological sort, cycle detection, transitive closure, parallel execution level computation, and Strongly Connected Components.

This package also carries the DT00 graph/node/edge property-bag contract into
directed graphs. Directed edge metadata is keyed by ordered edge identity, so
`(u, v)` and `(v, u)` can hold independent weights, training flags, gradient
slots, labels, and runtime annotations.

## Where it fits in the stack

This package is the directed graph layer in the DT data structure series:

```
DT00: Graph[T]          — undirected weighted graph (base class)
DT01: DirectedGraph[T]  — directed graph, extends Graph[T]    ← this package
```

`DirectedGraph` inherits from `Graph[T]` and is used anywhere directed relationships
are needed: build systems (dependency ordering), task schedulers (parallel execution
planning), state machines, and compiler IR analysis.

## Installation

```bash
pip install coding-adventures-directed-graph
```

For development:

```bash
uv pip install -e "../graph"   # install DT00 base class first
uv pip install -e ".[dev]"
```

## Quick Start

```python
from directed_graph import (
    DirectedGraph,
    topological_sort,
    has_cycle,
    independent_groups,
    affected_nodes,
    strongly_connected_components,
)

# Build a dependency graph
# Edge A → B means "A depends on B" (B must be built before A)
g = DirectedGraph()
g.add_edge("compile", "parse")
g.add_edge("compile", "typecheck")
g.add_edge("link", "compile")
g.add_edge("package", "link")
g.add_node("compile", {"kind": "task"})
g.set_edge_property("link", "compile", "weight", 2.0)

# Valid build order
topological_sort(g)
# ["parse", "typecheck", "compile", "link", "package"]  (one valid ordering)

# Find parallelism
independent_groups(g)
# [["parse", "typecheck"], ["compile"], ["link"], ["package"]]
# parse and typecheck can run in parallel!

# If parse changes, what must be rebuilt?
affected_nodes(g, frozenset({"parse"}))
# frozenset({"parse", "compile", "link", "package"})

# Detect cycles
has_cycle(g)   # False — this is a DAG

g2 = DirectedGraph()
g2.add_edge("A", "B")
g2.add_edge("B", "A")
has_cycle(g2)  # True
```

## API Reference

### `DirectedGraph[T](Graph[T])`

Extends `Graph[T]` from DT00. Always uses adjacency list internally.

#### Inherited from Graph[T]

| Method | Description |
|--------|-------------|
| `add_node(node)` | Add a node (no-op if exists) |
| `remove_node(node)` | Remove node and all incident edges; raises `KeyError` |
| `has_node(node)` | Check if node exists |
| `nodes() -> frozenset[T]` | All nodes as a frozenset |
| `has_edge(u, v)` | Check if directed edge u→v exists |
| `edge_weight(u, v)` | Weight of edge u→v; raises `KeyError` if missing |
| `len(g)` | Number of nodes |
| `node in g` | Same as `has_node` |

#### Property methods

| Method | Description |
|--------|-------------|
| `graph_properties()` | Copy of graph-level metadata |
| `set_graph_property(key, value)` | Set graph-level metadata |
| `remove_graph_property(key)` | Remove graph-level metadata |
| `node_properties(node)` | Copy of node metadata |
| `set_node_property(node, key, value)` | Set node metadata |
| `remove_node_property(node, key)` | Remove node metadata |
| `edge_properties(u, v)` | Copy of directed edge metadata, always including `weight` |
| `set_edge_property(u, v, key, value)` | Set edge metadata; `weight` updates edge weight |
| `remove_edge_property(u, v, key)` | Remove edge metadata; `weight` resets to `1.0` |

#### Overridden

| Method | Description |
|--------|-------------|
| `add_edge(u, v, weight=1.0)` | Add directed edge u→v; both nodes auto-added; raises `ValueError` on self-loop |
| `remove_edge(u, v)` | Remove directed edge u→v; raises `KeyError` if missing |
| `neighbors(node) -> frozenset[T]` | Returns successors only (enables bfs/dfs from graph package) |
| `edges() -> frozenset[tuple[T,T,float]]` | All directed edges as (u, v, weight) triples |

#### New in DirectedGraph

| Method | Description |
|--------|-------------|
| `successors(node) -> frozenset[T]` | Nodes that `node` points TO |
| `predecessors(node) -> frozenset[T]` | Nodes that point TO `node` |
| `out_degree(node) -> int` | Number of outgoing edges |
| `in_degree(node) -> int` | Number of incoming edges |

### `LabeledDirectedGraph[T]`

A directed graph (by composition over DirectedGraph) where every edge carries a
required string label. Useful for state machines and annotated dependency graphs.

| Method | Description |
|--------|-------------|
| `add_node(node)` | Add a node |
| `remove_node(node)` | Remove node and all its labeled edges |
| `has_node(node)` | Check existence |
| `nodes() -> frozenset[T]` | All nodes |
| `add_edge(u, v, label, weight=1.0)` | Add labeled directed edge |
| `remove_edge(u, v)` | Remove edge and its label |
| `has_edge(u, v)` | Check edge existence |
| `edge_label(u, v) -> str` | Get label; raises `KeyError` if missing |
| `edges_labeled() -> frozenset[tuple[T,T,str,float]]` | All edges with labels and weights |
| `successors(node) -> frozenset[T]` | Forward neighbors |
| `predecessors(node) -> frozenset[T]` | Reverse neighbors |

### Pure Algorithms

All algorithms are module-level functions — import them from `directed_graph`:

| Function | Description |
|----------|-------------|
| `topological_sort(graph)` | Kahn's algorithm; raises `ValueError` if cycle |
| `has_cycle(graph)` | Iterative 3-color DFS; returns `bool` |
| `transitive_closure(graph, node)` | All nodes reachable FROM node (BFS forward) |
| `transitive_dependents(graph, node)` | All nodes that depend on node (BFS reverse) |
| `independent_groups(graph)` | Parallel execution levels; raises `ValueError` if cycle |
| `affected_nodes(graph, changed)` | `changed` + all transitive dependents |
| `strongly_connected_components(graph)` | Kosaraju's two-pass; returns `list[frozenset[T]]` |

## Internal Design

`DirectedGraph` stores two adjacency dicts:

- `_adj[u][v] = weight` — forward edges (u → v); inherited from `Graph[T]`
- `_reverse[v][u] = weight` — reverse edges; maintained by overridden `add_edge`/`remove_edge`

The dual-dict design makes forward AND reverse traversals O(1) per edge:
- `successors(u)` → `frozenset(_adj[u])`
- `predecessors(v)` → `frozenset(_reverse[v])`
- Kahn's in-degree counting → `len(_reverse[node])`

`neighbors()` is overridden to return successors only.  This makes `bfs` and `dfs`
from the `graph` package work correctly on `DirectedGraph` without modification:

```python
from graph import bfs, dfs
from directed_graph import DirectedGraph

g = DirectedGraph()
g.add_edge("A", "B")
g.add_edge("B", "C")
bfs(g, "A")   # ["A", "B", "C"]  — follows forward edges only
```

## Running Tests

```bash
uv run python -m pytest tests/ -v
```

Tests require 95%+ coverage (enforced by pytest-cov configuration).
