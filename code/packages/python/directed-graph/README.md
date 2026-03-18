# coding-adventures-directed-graph

A directed graph library with topological sort, cycle detection, transitive closure, and parallel execution level computation. Built for use in build systems, dependency resolution, and task scheduling.

## Where it fits in the stack

This package provides the graph data structure that underpins dependency resolution across the coding-adventures project. Any package that needs to reason about "what depends on what" -- build ordering, incremental rebuilds, parallel execution planning -- uses this library.

## Installation

```bash
pip install coding-adventures-directed-graph
```

For development:

```bash
uv pip install -e ".[dev]"
```

## Quick Start

```python
from directed_graph import DirectedGraph

# Build a dependency graph
g = DirectedGraph()
g.add_edge("compile", "parse")    # compile depends on parse
g.add_edge("compile", "typecheck") # compile depends on typecheck
g.add_edge("link", "compile")      # link depends on compile
g.add_edge("package", "link")      # package depends on link

# Get a valid build order
print(g.topological_sort())
# ['parse', 'typecheck', 'compile', 'link', 'package']

# Find what can run in parallel
print(g.independent_groups())
# [['parse', 'typecheck'], ['compile'], ['link'], ['package']]

# What's affected if we change 'parse'?
print(g.affected_nodes({"parse"}))
# {'parse', 'compile', 'link', 'package'}
```

## API Reference

### DirectedGraph

#### Core Methods

| Method | Description |
|--------|-------------|
| `add_node(node)` | Add a node (no-op if exists) |
| `add_edge(from, to)` | Add directed edge; implicitly adds nodes; raises `ValueError` on self-loop |
| `remove_node(node)` | Remove node and all edges; raises `NodeNotFoundError` |
| `remove_edge(from, to)` | Remove edge; raises `EdgeNotFoundError` |
| `has_node(node)` | Check if node exists |
| `has_edge(from, to)` | Check if edge exists |
| `nodes()` | List all nodes |
| `edges()` | List all edges as tuples |
| `predecessors(node)` | Direct parents of node |
| `successors(node)` | Direct children of node |
| `len(g)` | Number of nodes |
| `node in g` | Same as `has_node` |

#### Algorithm Methods

| Method | Description |
|--------|-------------|
| `topological_sort()` | Kahn's algorithm; raises `CycleError` |
| `has_cycle()` | DFS three-color cycle detection |
| `transitive_closure(node)` | All nodes reachable downstream |
| `transitive_dependents(node)` | All nodes that depend on node (reverse) |
| `independent_groups()` | Partition into parallel execution levels |
| `affected_nodes(changed)` | Changed nodes + all transitive dependents |

### Exceptions

- `CycleError` -- includes `.cycle` attribute with the cycle path
- `NodeNotFoundError` -- includes `.node` attribute
- `EdgeNotFoundError` -- includes `.from_node` and `.to_node` attributes

## Internal Design

The graph uses two adjacency dictionaries:

- `_forward[u]` = set of nodes that u points to (successors)
- `_reverse[v]` = set of nodes that point to v (predecessors)

This dual-dict design makes both forward and reverse traversals O(1) per edge, which is essential for algorithms like `transitive_dependents` that walk edges backwards.

## Running Tests

```bash
uv run pytest tests/ -v
```

Tests require 95%+ coverage (enforced by pytest-cov configuration).
