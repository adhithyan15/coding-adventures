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

## Weight rules for Python shortest paths

The Python graph container preserves its existing numeric storage behavior:
edges can store finite negative weights, NaN, or infinity. Finite negative
weights remain useful to `minimum_spanning_tree`, which uses Kruskal's
algorithm. Non-finite MST behavior is unchanged and not defined by this
profile.

`shortest_path` has a stricter precondition. It scans all graph adjacencies
before doing a search, including disconnected components and self-loops. Every
weight must be nonnegative and finite after conversion to a binary64 float; an
invalid weight raises `ValueError` before a same-node result or missing-endpoint
search. A Python integer too large to convert to a finite float also raises
`ValueError` when it reaches `shortest_path`. Both `add_edge` representations
retain the integer as supplied; the canonical `weight` property update converts
to float and may raise `OverflowError` earlier. For example, a negative edge
can still be stored and used by an MST, but it cannot be passed to Dijkstra
through `shortest_path`.

With valid weights, existing missing-endpoint behavior is retained: a missing
start raises `KeyError`; a missing end returns `[]` in the all-unit BFS branch
and raises `KeyError` in weighted search. Invalid-weight preflight errors take
precedence over those query outcomes.

This validation is read-only. It adds a full graph scan even for a same-node
query: O(V + E) time and O(V) temporary space for an adjacency list, or O(V²)
time and O(V) temporary space for an adjacency matrix. Path-sum arithmetic is
still binary64 and can overflow even when each edge weight is finite. The
adjacency-matrix representation also uses zero to mean “no edge,” so it cannot
currently represent zero-weight edges; the adjacency-list representation can.
Matrix `Graph.edges()` also currently omits diagonal self-loops. The
shortest-path preflight reads each node's adjacency so it still rejects an
invalid self-loop; `edges()` parity remains a separate representation gap.

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
uv venv .venv --quiet --no-project --clear --python 3.13
uv pip install --python .venv -e .[dev] --quiet
.venv/bin/python -m ruff check src tests
.venv/bin/python -m ruff format --check src tests
.venv/bin/python -m mypy --strict src tests
.venv/bin/python -m pytest tests/ -v
```

`BUILD_windows` provides the same gates through
`.venv\Scripts\python.exe`. Both recipes clear and recreate the package-local
environment, so an immediate repeat is supported.
