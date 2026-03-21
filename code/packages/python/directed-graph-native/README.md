# coding-adventures-directed-graph-native

A Rust-backed directed graph library for Python via PyO3. Drop-in replacement for `coding-adventures-directed-graph` with the same API but native performance.

## What is this?

This package wraps the Rust `directed-graph` crate and exposes it to Python as a native extension module. All algorithms (topological sort, cycle detection, transitive closure, independent groups, affected nodes) run in Rust — only the method call boundary crosses between Python and Rust.

## Installation

```bash
pip install coding-adventures-directed-graph-native
```

If a prebuilt wheel isn't available for your platform, pip will build from source (requires Rust toolchain).

## Usage

```python
from directed_graph_native import DirectedGraph, CycleError

g = DirectedGraph()
g.add_edge("compile", "link")
g.add_edge("link", "package")

# Topological sort (Kahn's algorithm)
print(g.topological_sort())    # ['compile', 'link', 'package']

# Parallel execution levels
print(g.independent_groups())  # [['compile'], ['link'], ['package']]

# Cycle detection
print(g.has_cycle())           # False

# Incremental builds: what's affected by a change?
affected = g.affected_nodes({"compile"})
print(affected)                # {'compile', 'link', 'package'}
```

## API

The API is identical to `coding-adventures-directed-graph` (pure Python):

| Method | Description |
|--------|-------------|
| `add_node(name)` | Add a node |
| `remove_node(name)` | Remove a node and its edges |
| `has_node(name)` | Check if node exists |
| `nodes()` | Sorted list of all nodes |
| `add_edge(from, to)` | Add directed edge |
| `remove_edge(from, to)` | Remove edge |
| `has_edge(from, to)` | Check if edge exists |
| `edges()` | Sorted list of `(from, to)` tuples |
| `predecessors(node)` | Nodes pointing to this node |
| `successors(node)` | Nodes this node points to |
| `topological_sort()` | Kahn's algorithm |
| `has_cycle()` | DFS 3-color cycle detection |
| `transitive_closure(node)` | All reachable nodes |
| `affected_nodes(changed)` | Changed + transitive dependents |
| `independent_groups()` | Parallel execution levels |

## How it fits in the stack

This is the first native extension package in the coding-adventures project. It demonstrates the pattern of wrapping a Rust core library with PyO3 to bring memory-safe, high-performance implementations to Python.

## Development

```bash
# Install maturin
pip install maturin

# Build and install in development mode
maturin develop

# Run tests
pytest tests/ -v
```
