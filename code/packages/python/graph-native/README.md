# coding-adventures-graph-native

Rust-backed DT00 graph for Python.

This package mirrors the public API of the pure Python `graph` package while
executing the storage layer and graph algorithms inside the Rust `graph` crate.

## Usage

```python
from graph_native import Graph, GraphRepr, bfs, minimum_spanning_tree

g = Graph(repr=GraphRepr.ADJACENCY_LIST)
g.add_edge("London", "Paris", 300.0)
g.add_edge("London", "Amsterdam", 520.0)
g.add_edge("Paris", "Berlin", 878.0)
g.add_edge("Amsterdam", "Berlin", 655.0)

print(bfs(g, "London"))
print(minimum_spanning_tree(g))
```

## Development

```bash
cargo build --release
PYTHONPATH=src python -m pytest tests -v
```
