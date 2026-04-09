# coding-adventures-graph

A complete undirected graph data structure implementation from scratch, supporting both adjacency list and adjacency matrix representations with comprehensive graph algorithms.

## Where it fits in the stack

This is **DT00** (Data Structures layer 0) — the foundational graph data structure that all specialized structures (DT01–DT16) build upon. Trees, tries, heaps, and every other graph-based structure inherit from or compose this Graph class.

## Features

- **Two representations**: adjacency list (default, O(V+E) space) and adjacency matrix (O(V²) space)
- **Weighted edges**: optional weights on edges (default 1.0)
- **Generic nodes**: any hashable type (strings, integers, tuples, etc.)
- **Core algorithms**:
  - BFS/DFS for traversal
  - Shortest path (BFS for unweighted, Dijkstra for weighted)
  - Cycle detection
  - Connected components
  - Minimum spanning tree (Kruskal's algorithm)
  - Union-Find (disjoint set union) data structure

## Installation

```bash
pip install coding-adventures-graph
```

For development:

```bash
uv pip install -e ".[dev]"
```

## Quick Start

### Building a graph

```python
from coding_adventures_graph import Graph

# Create a graph
g = Graph()

# Add edges (nodes are created automatically)
g.add_edge("A", "B", weight=1.0)
g.add_edge("B", "C", weight=2.0)
g.add_edge("C", "A", weight=3.0)

print(g.nodes())    # frozenset({"A", "B", "C"})
print(g.edges())    # frozenset({("A", "B", 1.0), ("B", "C", 2.0), ("C", "A", 3.0)})
print(g.degree("A"))  # 2
print(g.neighbors("A"))  # frozenset({"B", "C"})
```

### Running algorithms

```python
from coding_adventures_graph import (
    bfs, dfs, shortest_path, has_cycle,
    minimum_spanning_tree, connected_components
)

# Traversal
print(bfs(g, "A"))        # ["A", "B", "C"]
print(dfs(g, "A"))        # ["A", "C", "B"] or similar

# Path finding
path = shortest_path(g, "A", "C")
print(path)  # ["A", "C"] (direct edge, weight 3)

# Graph properties
print(has_cycle(g))       # True (triangle)
print(connected_components(g))  # [frozenset({"A", "B", "C"})]

# Spanning tree
mst = minimum_spanning_tree(g)
print(mst)  # frozenset({("A", "B", 1.0), ("B", "C", 2.0)})
```

### Using different representations

```python
from coding_adventures_graph import Graph, GraphRepr

# Adjacency list (default): O(V + E) space, good for sparse graphs
g_list = Graph(GraphRepr.ADJACENCY_LIST)

# Adjacency matrix: O(V²) space, O(1) edge lookup, good for dense graphs
g_matrix = Graph(GraphRepr.ADJACENCY_MATRIX)

# Both produce identical results for all operations
```

## API Reference

### Graph class

**Construction:**
- `Graph(repr=GraphRepr.ADJACENCY_LIST)` — Create empty graph

**Node operations:**
- `add_node(node)` — Add a node
- `remove_node(node)` — Remove a node and its edges
- `has_node(node)` — Check if node exists
- `nodes()` — Return all nodes as frozenset
- `len(graph)` — Number of nodes

**Edge operations:**
- `add_edge(u, v, weight=1.0)` — Add undirected edge
- `remove_edge(u, v)` — Remove edge
- `has_edge(u, v)` — Check if edge exists
- `edge_weight(u, v)` — Get edge weight
- `edges()` — Return all edges as frozenset of (u, v, weight)

**Neighborhood:**
- `neighbors(node)` — Get adjacent nodes as frozenset
- `neighbors_weighted(node)` — Get neighbors with weights as dict
- `degree(node)` — Count neighbors

### Algorithms (pure functions)

All algorithms are pure functions that don't modify the graph:

- `bfs(graph, start)` → list[T] — Breadth-first search
- `dfs(graph, start)` → list[T] — Depth-first search
- `is_connected(graph)` → bool — Check if connected
- `connected_components(graph)` → list[frozenset[T]] — Find all components
- `has_cycle(graph)` → bool — Detect cycles
- `shortest_path(graph, start, end)` → list[T] — Find shortest path
- `minimum_spanning_tree(graph)` → frozenset[tuple] — Kruskal's MST

### Exception classes

- `NodeNotFoundError` — Raised when operating on missing node
- `EdgeNotFoundError` — Raised when operating on missing edge

## Design Notes

### Why two representations?

| Operation | Adjacency List | Adjacency Matrix |
|-----------|---|---|
| Space | O(V + E) | O(V²) |
| Add edge | O(1) | O(1) |
| Remove edge | O(degree) | O(1) |
| Check edge | O(degree) | O(1) |
| List neighbors | O(degree) | O(V) |
| List all edges | O(V + E) | O(V²) |

Use adjacency list by default (sparse graphs). Switch to matrix only for dense graphs where E > V²/4.

### Weighted vs. unweighted

By default, edges have weight 1.0. When you add an edge with a different weight, algorithms that care about weights (Dijkstra, MST) use them. Algorithms that don't (BFS, DFS, cycle detection) ignore them.

## Running Tests

```bash
# Run all tests with coverage
uv run pytest tests/ -v

# Run specific test class
uv run pytest tests/test_graph.py::TestBFS -v

# Run with coverage report
uv run pytest tests/ --cov=coding_adventures_graph --cov-report=term-missing
```

Current coverage: **97.38%** (133 tests, exceeding 95% requirement)
