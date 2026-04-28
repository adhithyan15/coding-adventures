# graph — DT00

Undirected graph with two internal representations (adjacency list and adjacency
matrix) and a suite of classic graph algorithms as pure functions.

This is the foundation of the entire DT series. Every specialised structure
(directed graph, tree, binary tree, heap, trie, …) is a graph with additional
constraints imposed on top.

## What it does

```python
from graph import Graph, GraphRepr, bfs, shortest_path, minimum_spanning_tree

# Build a city graph
g: Graph[str] = Graph()
g.add_node("London", {"kind": "city"})
g.add_edge("London", "Paris", weight=300, properties={"route": "train"})
g.add_edge("London", "Amsterdam", weight=520)
g.add_edge("Paris", "Berlin", weight=878)
g.add_edge("Amsterdam", "Berlin", weight=655)
g.add_edge("Amsterdam", "Brussels", weight=180)

print(g.edge_properties("Paris", "London"))
# → {"route": "train", "weight": 300}

# Breadth-first traversal from London
print(bfs(g, "London"))
# → ['London', 'Paris', 'Amsterdam', 'Berlin', 'Brussels']

# Shortest (cheapest) route London → Berlin
print(shortest_path(g, "London", "Berlin"))
# → ['London', 'Amsterdam', 'Berlin']   (cost 1175 vs 1178 via Paris)

# Minimum spanning tree
for u, v, w in minimum_spanning_tree(g):
    print(f"  {u} — {v}  ({w})")
```

## Internal representations

Choose at construction time:

```python
from graph import Graph, GraphRepr

g_list   = Graph(repr=GraphRepr.ADJACENCY_LIST)    # default; O(V+E) space
g_matrix = Graph(repr=GraphRepr.ADJACENCY_MATRIX)  # O(V²) space; O(1) edge lookup
```

Both expose exactly the same public API — every algorithm works on either.

## Properties

Graphs, nodes, and edges can carry portable property bags:

```python
g.set_graph_property("name", "city-map")
g.add_node("Amsterdam", {"kind": "city"})
g.add_edge("Amsterdam", "Berlin", weight=655, properties={"route": "rail"})

assert g.edge_properties("Berlin", "Amsterdam")["weight"] == 655
```

Property bags are copied on read. Edge weights are also exposed through the
canonical `weight` edge property.

## Where it fits

```
DT00 graph  ← you are here
     │
     ▼
DT01 directed-graph
     │
     ▼
DT02 tree
     │
    ... (DT03–DT16)
```

## Development

```bash
cd code/packages/python/graph
bash BUILD
```
