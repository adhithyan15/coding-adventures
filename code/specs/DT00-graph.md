# DT00 — Graph (Undirected)

## Overview

A graph is the most general data structure for representing pairwise relationships between
things. It consists of nodes (also called vertices) and edges that connect pairs of nodes.
In an undirected graph, every edge is symmetric: if A is connected to B, then B is also
connected to A. The graph package is the foundation of the entire DT series — every
specialized structure (trees, heaps, tries) is a graph with additional constraints.

## Layer Position

```
[YOU ARE HERE: DT00 graph]
        │
        ▼
DT01 directed-graph  (adds direction to edges)
        │
        ▼
DT02 tree            (directed-graph with single root, no cycles)
        │
        ▼
DT03 binary-tree     (tree with max 2 children)
       ├── DT04 heap
       ├── DT05 segment-tree
       ├── DT07 binary-search-tree
       │        ├── DT08 avl-tree
       │        ├── DT09 red-black-tree
       │        └── DT10 treap
       └── DT11 b-tree
               └── DT12 b-plus-tree
DT06 fenwick-tree    (array-backed, no explicit tree structure)
DT13 trie            (prefix tree)
     └── DT14 radix-tree
             └── DT15 suffix-tree
DT16 rope            (binary-tree for strings)
DT17 hash-functions → DT18 hash-map → DT19 hash-set
DT20 skip-list
DT21 hyperloglog
DT22 bloom-filter
DT23 resp-protocol → DT24 tcp-server → DT25 mini-redis
```

## Concepts

### What is a graph?

Imagine a map of cities connected by roads. Each city is a **node**. Each road is an
**edge**. The entire road network is a **graph**. This mental model works for almost
every graph you will encounter:

```
           [Paris]
          /       \
    [London]    [Berlin]
          \       /
          [Amsterdam]
              |
           [Brussels]
```

Here there are 5 nodes and 5 edges. London—Amsterdam—Berlin forms a path. Paris and
Brussels are connected through intermediate nodes. This is an undirected graph because
every road goes both ways — if you can drive London→Amsterdam you can also drive
Amsterdam→London.

### Formal definition

A graph G = (V, E) where:
- V is a set of vertices (nodes): {London, Paris, Berlin, Amsterdam, Brussels}
- E is a set of edges, each edge being an unordered pair of vertices: {{London, Paris}, {Paris, Berlin}, ...}

An edge {u, v} is equivalent to {v, u} — there is no "direction."

### Weighted edges

Sometimes edges have a **weight** — a number attached to them. On a road map, weights
might represent distances or travel times. On a social network, weights might represent
how often two people interact.

```
           [Paris]
       300 /       \ 878
    [London]      [Berlin]
       520 \       / 655
          [Amsterdam]
           |  180
        [Brussels]
```

The weight is stored alongside the edge. Algorithms that care about weights (like Dijkstra's
shortest path) use them; algorithms that don't (like BFS) ignore them and treat all edges
as weight 1.

### Node and edge properties

Graphs also need metadata. A compiler graph may need to know that a node is a parser
stage. A neural graph may need to know that a node is an activation function and that an
edge is trainable. A build graph may want to tag edges as `runtime`, `test`, or
`compile`.

The base graph package owns this capability directly:

```
node_properties[node]       = PropertyBag
edge_properties[{u, v}]     = PropertyBag
graph_properties            = PropertyBag
```

A **PropertyBag** is a string-keyed map whose values are portable JSON-like scalars:

```
PropertyValue = string | number | boolean | null
PropertyBag   = Map<string, PropertyValue>
```

Language implementations should use the closest idiomatic representation while preserving
the same semantics:

```
TypeScript: Record<string, string | number | boolean | null>
Python:     dict[str, str | int | float | bool | None]
Go:         map[string]any, restricted by tests/documentation to scalar values
Rust:       enum GraphPropertyValue { String, Number, Bool, Null }
Ruby/Perl/Lua/Elixir: native map/hash/table values, restricted to scalar values
Java/Kotlin/C#/F#: Map/Dictionary<string, object?>
Swift:      enum GraphPropertyValue
Dart:       Map<String, Object?>
Haskell:    data GraphPropertyValue = PropertyString | PropertyNumber | PropertyBool | PropertyNull
```

Properties are not the runtime. They are facts attached to the graph's structure. A future
runtime can interpret them, trace them, lower them to matrix operations, or ignore them.

#### The `weight` property

`weight` is the canonical edge property for weighted algorithms. Existing `add_edge(...,
weight)` and `edge_weight(...)` APIs remain valid, but implementations should also expose
the weight through the edge property bag:

```
graph.add_edge("London", "Paris", weight: 300)
graph.edge_properties("London", "Paris")["weight"] == 300
```

When callers set the `weight` edge property to a number, `edge_weight` and weighted
algorithms must observe the same value. If `weight` is absent, the default is `1.0`.
Non-numeric `weight` values are invalid.

### Two ways to store a graph

This is one of the most important design decisions in graph programming. There are two
classic representations, and choosing the wrong one can make your program 1000x slower.

#### Representation 1: Adjacency List

For each node, store the list (or set) of its neighbors.

```
London     → {Paris, Amsterdam}
Paris      → {London, Berlin}
Amsterdam  → {London, Berlin, Brussels}
Berlin     → {Paris, Amsterdam}
Brussels   → {Amsterdam}
```

In code, this is a `Map<Node, Set<Node>>`. If edges are weighted, use
`Map<Node, Map<Node, Weight>>` so you can look up the weight of any specific edge.

Memory usage: O(V + E) — you only store entries for edges that exist.

Edge lookup: O(degree(v)) — to check if London—Berlin is an edge, scan London's neighbors.

This is efficient when the graph is **sparse** (few edges relative to nodes). Most
real-world graphs are sparse: social networks, road maps, dependency graphs, the web.

#### Representation 2: Adjacency Matrix

Create a V×V boolean matrix. `matrix[i][j] = true` means there is an edge between
node i and node j.

```
          London  Paris  Amsterdam  Berlin  Brussels
London  [  -       T        T        F        F    ]
Paris   [  T       -        F        T        F    ]
Amsterdam[ T       F        -        T        T    ]
Berlin  [  F       T        T        -        F    ]
Brussels[  F       F        T        F        -    ]
```

Memory usage: O(V²) — you allocate a slot for every possible edge even if it doesn't exist.

Edge lookup: O(1) — checking if London—Berlin is an edge is a single array read.

This is efficient when the graph is **dense** (most possible edges exist, or V is small).
Good for social networks among small groups, game boards, transition matrices.

#### Trade-off summary

```
Operation              │ Adjacency List     │ Adjacency Matrix
───────────────────────┼────────────────────┼──────────────────
Space                  │ O(V + E)           │ O(V²)
Add node               │ O(1)               │ O(V²) — resize matrix
Remove node            │ O(V + E)           │ O(V²) — remove row/col
Add edge               │ O(1)               │ O(1)
Remove edge            │ O(degree)          │ O(1)
Has edge (u, v)?       │ O(degree(u))       │ O(1)
List all neighbors     │ O(degree(v))       │ O(V)
Iterate all edges      │ O(V + E)           │ O(V²)
───────────────────────┼────────────────────┼──────────────────
Best for               │ Sparse graphs      │ Dense graphs
                       │ E << V²            │ E ≈ V²
```

**The rule of thumb:** Use adjacency list by default. Switch to adjacency matrix only if
your graph is dense (E > V²/4 or so) AND you need O(1) edge lookup.

### Nodes are generic

Nodes can be anything: integers, strings, objects. In Python/Ruby, any hashable value
works. In TypeScript, Rust, and Go, the node type is a generic parameter `T` that must
be hashable/comparable.

```
graph of strings:    nodes are "London", "Paris", ...
graph of integers:   nodes are 0, 1, 2, 3, ...
graph of structs:    nodes are {id: 42, name: "Alice", ...}
```

Internally, each node must be storable in a set and usable as a map key. Python uses
`__hash__` and `__eq__`. Rust requires `Hash + Eq`. Go requires comparable types.

### Graph vocabulary

Before diving into algorithms, here are the terms you'll see everywhere:

```
Degree of a node:     number of edges connected to it
                      (London has degree 2: Paris and Amsterdam)

Path:                 a sequence of nodes where consecutive nodes share an edge
                      London → Amsterdam → Berlin → Paris

Path length:          number of edges in the path (above: length 3)

Cycle:                a path that starts and ends at the same node
                      London → Amsterdam → Brussels → Amsterdam  ← NOT a cycle (repeats Amsterdam)
                      London → Paris → Berlin → Amsterdam → London  ← IS a cycle

Connected graph:      every node can reach every other node via some path

Component:            a maximal connected subgraph
                      if the graph has isolated clusters, each cluster is a component

Sparse graph:         E << V² (most edges missing)
Dense graph:          E ≈ V² (most edges present)
```

## Representation

The graph stores an internal representation type chosen at construction time:

```
GraphRepr = "adjacency_list" | "adjacency_matrix"
```

For adjacency list:
```
nodes: Set<T>
adj:   Map<T, Map<T, float>>   # neighbor → edge_weight (default 1.0)
node_properties: Map<T, PropertyBag>
edge_properties: Map<EdgeKey<T>, PropertyBag>
graph_properties: PropertyBag
```

For adjacency matrix:
```
nodes:   List<T>               # ordered list for index mapping
index:   Map<T, int>           # node → row/col index
matrix:  List<List<float>>     # V×V matrix; 0.0 means no edge
node_properties: Map<T, PropertyBag>
edge_properties: Map<EdgeKey<T>, PropertyBag>
graph_properties: PropertyBag
```

Both expose the same public API so every algorithm works on either representation
without modification.

Edge property keys for undirected graphs are canonicalized because `{u, v}` and `{v, u}`
are the same edge. Implementations may use tuple keys, stable string keys, or internal
edge IDs, but the public API must treat both endpoint orders identically.

## Public API

Existing graph APIs remain source-compatible. Property support extends the surface with
the following common operations:

```
add_node(node, properties = {})
add_edge(left, right, weight = 1.0, properties = {})

graph_properties() -> PropertyBag
set_graph_property(key, value)
remove_graph_property(key)

node_properties(node) -> PropertyBag
set_node_property(node, key, value)
remove_node_property(node, key)

edge_properties(left, right) -> PropertyBag
set_edge_property(left, right, key, value)
remove_edge_property(left, right, key)
```

API naming may follow each language's style (`addNode`, `AddNode`, `add_node`,
`setNodeProperty`), but semantics must match:

- Returned property bags are copies or read-only views unless the language deliberately
  documents live mutation.
- Adding an existing node merges new properties into the existing property bag.
- Adding an existing edge updates its weight and merges new properties into the existing
  edge property bag.
- Removing a node removes its node properties and all incident edge properties.
- Removing an edge removes its edge properties.
- Algorithms must not mutate graph, node, or edge properties.
- Property insertion order is not semantically meaningful.
- Unknown properties are preserved by copy and serialization operations.

## Algorithms (Pure Functions)

All algorithms are pure functions — they take a `Graph` as input and return a result.
They never mutate the graph and never live as methods on the class.

### `bfs(graph, start) → List[T]`

**Breadth-First Search** — explore nodes level by level, starting from `start`.
Think of it as dropping a stone in a pond: the ripples spread outward one ring at a time.

Pseudocode:
```
bfs(graph, start):
    visited = {start}
    queue   = deque([start])
    result  = []

    while queue is not empty:
        node = queue.popleft()
        result.append(node)

        for neighbor in graph.neighbors(node):
            if neighbor not in visited:
                visited.add(neighbor)
                queue.append(neighbor)

    return result
```

Worked example on the city graph (starting from London):
```
Level 0:   London                      (start)
Level 1:   Paris, Amsterdam            (London's neighbors)
Level 2:   Berlin, Brussels            (new neighbors of Paris and Amsterdam)

BFS order: [London, Paris, Amsterdam, Berlin, Brussels]
```

Time: O(V + E). Space: O(V) for the visited set and queue.

BFS is the right algorithm when you want the **shortest path** in an unweighted graph,
or when you want to explore "nearby" nodes before "distant" ones.

### `dfs(graph, start) → List[T]`

**Depth-First Search** — go as deep as possible down one path before backtracking.
Think of it as exploring a maze: walk forward until you hit a dead end, then back up
and try a different turn.

Pseudocode (iterative with explicit stack):
```
dfs(graph, start):
    visited = set()
    stack   = [start]
    result  = []

    while stack is not empty:
        node = stack.pop()
        if node in visited:
            continue
        visited.add(node)
        result.append(node)

        for neighbor in graph.neighbors(node):
            if neighbor not in visited:
                stack.append(neighbor)

    return result
```

Time: O(V + E). Space: O(V) for the visited set and call stack.

DFS is the right algorithm for cycle detection, topological sorting, finding connected
components, and maze solving.

### `is_connected(graph) → bool`

A graph is connected if every node can reach every other node. Check by running BFS
(or DFS) from any node and seeing if all nodes were visited.

Pseudocode:
```
is_connected(graph):
    if graph has no nodes:
        return True           # vacuously true
    start = any node in graph
    visited = bfs(graph, start)
    return len(visited) == len(graph.nodes())
```

Time: O(V + E).

### `connected_components(graph) → List[Set[T]]`

Returns the list of all connected components — groups of nodes that can reach each
other but cannot reach nodes in other groups.

Think of an archipelago: each island is a connected component. Ships can travel between
ports on the same island, but not to a different island (no bridge).

Pseudocode:
```
connected_components(graph):
    unvisited = set(graph.nodes())
    components = []

    while unvisited is not empty:
        start = any node in unvisited
        component = set(bfs(graph, start))
        components.append(component)
        unvisited -= component

    return components
```

Worked example:
```
Graph: A—B—C   D—E   F

Components: [{A, B, C}, {D, E}, {F}]   ← three disconnected islands
```

Time: O(V + E).

### `has_cycle(graph) → bool`

An undirected graph has a cycle if DFS finds a "back edge" — an edge to a node already
in the current visited set, not counting the parent node (every edge appears twice in
an undirected graph; we must not count the edge we came from as a cycle).

Pseudocode:
```
has_cycle(graph):
    visited = set()

    dfs_cycle(node, parent):
        visited.add(node)
        for neighbor in graph.neighbors(node):
            if neighbor not in visited:
                if dfs_cycle(neighbor, node):
                    return True
            elif neighbor != parent:   ← back edge: neighbor visited and not our parent
                return True
        return False

    for node in graph.nodes():
        if node not in visited:
            if dfs_cycle(node, None):
                return True
    return False
```

Example:
```
A—B—C—A   ← triangle: has a cycle
A—B—C     ← path: no cycle
```

Time: O(V + E).

### `shortest_path(graph, start, end) → List[T]`

Returns the shortest path (fewest edges for unweighted, lowest total weight for
weighted) from `start` to `end`. Returns an empty list if no path exists.

**Unweighted graphs — BFS approach:**

BFS naturally finds the shortest path in terms of edge count. While running BFS,
record the "parent" of each node (which node we came from). Then trace back from
`end` to `start` using the parent map.

```
shortest_path_unweighted(graph, start, end):
    parent = {start: None}
    queue  = deque([start])

    while queue:
        node = queue.popleft()
        if node == end:
            break
        for neighbor in graph.neighbors(node):
            if neighbor not in parent:
                parent[neighbor] = node
                queue.append(neighbor)

    if end not in parent:
        return []   # no path

    # trace back
    path = []
    node = end
    while node is not None:
        path.append(node)
        node = parent[node]
    return reversed(path)
```

**Weighted graphs — Dijkstra's algorithm:**

When edges have different weights, BFS no longer works (it treats all edges as equal).
Dijkstra's algorithm uses a priority queue to always expand the closest unvisited node.

Think of it like water flowing: water flows through the cheapest paths first, spreading
outward until it reaches the destination.

```
dijkstra(graph, start, end):
    dist   = {node: infinity for node in graph.nodes()}
    parent = {}
    dist[start] = 0
    pq = MinHeap([(0, start)])   # (distance, node)

    while pq is not empty:
        d, node = pq.pop_min()
        if d > dist[node]:
            continue             # stale entry, skip
        if node == end:
            break
        for neighbor, weight in graph.neighbors_weighted(node):
            new_dist = dist[node] + weight
            if new_dist < dist[neighbor]:
                dist[neighbor] = new_dist
                parent[neighbor] = node
                pq.push((new_dist, neighbor))

    return trace_path(parent, start, end)
```

Time: O((V + E) log V) with a binary heap.

### `minimum_spanning_tree(graph) → Set[Tuple[T, T, float]]`

A **spanning tree** of a connected graph is a subset of edges that connects all nodes
with no cycles. A **minimum** spanning tree uses the subset with the lowest total weight.

Example: you need to lay cables to connect 5 cities. A spanning tree means every city
is reachable. A minimum spanning tree minimizes the total cable length.

**Kruskal's Algorithm:**

Sort all edges by weight. Greedily add the cheapest edge that does not create a cycle.
The cycle check uses **Union-Find** (also called Disjoint Set Union):

```
Union-Find data structure:
- parent[v] = which "representative" node v belongs to
- Initially every node is its own representative: parent[v] = v
- find(v):  walk up parent pointers to the root (with path compression)
- union(a, b): merge the sets containing a and b

Path compression trick: when walking up to find the root,
update every node along the path to point directly to the root.
This makes future finds nearly O(1).
```

Kruskal pseudocode:
```
kruskal(graph):
    edges  = sorted(graph.edges(), key=lambda e: e.weight)
    uf     = UnionFind(graph.nodes())
    result = set()

    for (u, v, weight) in edges:
        if uf.find(u) != uf.find(v):    ← different components → no cycle
            uf.union(u, v)
            result.add((u, v, weight))
        if len(result) == len(graph.nodes()) - 1:
            break                        ← MST complete (V-1 edges)

    return result
```

Worked example:
```
Graph:
  A —3— B
  |       \
  1         4
  |           \
  C —2— D —5— E

Edges sorted by weight: (A,C,1), (C,D,2), (A,B,3), (B,E,4), (D,E,5)

Step 1: Add (A,C,1)  → {A,C} connected
Step 2: Add (C,D,2)  → {A,C,D} connected
Step 3: Add (A,B,3)  → {A,C,D,B} connected
Step 4: Add (B,E,4)  → {A,C,D,B,E} connected — MST complete!

MST edges: (A,C,1), (C,D,2), (A,B,3), (B,E,4)   total weight = 10
Skipped:   (D,E,5) would create a cycle
```

Time: O(E log E) for sorting + O(E · α(V)) for Union-Find (α is the inverse Ackermann
function — effectively O(1) in practice).

## Public API

```python
from enum import Enum
from typing import Generic, TypeVar, Optional

T = TypeVar("T")  # node type — must be hashable

class GraphRepr(Enum):
    ADJACENCY_LIST   = "adjacency_list"
    ADJACENCY_MATRIX = "adjacency_matrix"

class Graph(Generic[T]):
    def __init__(
        self,
        repr: GraphRepr = GraphRepr.ADJACENCY_LIST,
    ) -> None: ...

    # Node operations
    def add_node(self, node: T) -> None: ...
    def remove_node(self, node: T) -> None: ...
    def has_node(self, node: T) -> bool: ...
    def nodes(self) -> frozenset[T]: ...

    # Edge operations
    def add_edge(self, u: T, v: T, weight: float = 1.0) -> None: ...
    def remove_edge(self, u: T, v: T) -> None: ...
    def has_edge(self, u: T, v: T) -> bool: ...
    def edges(self) -> frozenset[tuple[T, T, float]]: ...  # (u, v, weight)
    def edge_weight(self, u: T, v: T) -> float: ...        # raises KeyError if missing

    # Neighborhood
    def neighbors(self, node: T) -> frozenset[T]: ...
    def neighbors_weighted(self, node: T) -> dict[T, float]: ...
    def degree(self, node: T) -> int: ...

    def __len__(self) -> int: ...     # number of nodes
    def __contains__(self, node: T) -> bool: ...

# ─── Pure function algorithms ───────────────────────────────────────────────

def bfs(graph: Graph[T], start: T) -> list[T]: ...
def dfs(graph: Graph[T], start: T) -> list[T]: ...
def is_connected(graph: Graph[T]) -> bool: ...
def connected_components(graph: Graph[T]) -> list[frozenset[T]]: ...
def has_cycle(graph: Graph[T]) -> bool: ...
def shortest_path(graph: Graph[T], start: T, end: T) -> list[T]: ...
def minimum_spanning_tree(graph: Graph[T]) -> frozenset[tuple[T, T, float]]: ...
```

## Composition Model

The Graph class is the root of the DT hierarchy. All specializations build on it.

- **Python, Ruby, TypeScript** — Use inheritance. `DirectedGraph(Graph)`, `Tree(DirectedGraph)`, etc.
  Subclasses call `super().__init__()` and may override or augment methods.

- **Rust, Go, Elixir, Lua, Perl, Swift** — Use composition. `DirectedGraph` wraps a `Graph`
  internally and delegates to it, adding extra state (e.g., the reverse adjacency map).

  ```rust
  // Rust example
  pub struct DirectedGraph<T> {
      inner: Graph<T>,          // delegate for shared operations
      reverse: HashMap<T, HashSet<T>>,  // extra: predecessors
  }
  ```

  The wrapped inner graph handles storage; the outer type enforces the new constraints
  and exposes the augmented API.

## Test Strategy

- Construction: build graphs with both `ADJACENCY_LIST` and `ADJACENCY_MATRIX` repr.
  Verify all operations produce identical results.
- Node operations: add, remove, has_node, check that remove cleans up all edges.
- Edge operations: add undirected edge, verify both (u,v) and (v,u) are present.
  Test weighted edges, default weight of 1.0.
- Degree: verify degree(node) == number of neighbors.
- BFS / DFS: use a known graph with a known expected traversal order.
  Test disconnected graphs (unreachable nodes should not appear).
- is_connected: test connected and disconnected graphs, including single-node graph.
- connected_components: graph with 3 isolated clusters; verify 3 components returned.
- has_cycle: triangle → True; path → False; single node → False.
- shortest_path: unweighted (BFS) and weighted (Dijkstra), including no-path case.
- minimum_spanning_tree: verify result has exactly V-1 edges, all nodes covered,
  total weight is minimal, and no cycles.
- Edge cases: empty graph, single-node graph, complete graph (all nodes connected).
- Performance: create a 1000-node sparse graph; verify all algorithms complete quickly.

## Future Extensions

- **DT01 directed-graph** — adds edge direction (A→B ≠ B→A), predecessors/successors,
  topological sort, strongly connected components.
- **DT02 tree** — directed-graph with single root, no cycles, parent/child relationships.
- The entire DT03–DT16 series inherits transitively from this base.
- **DT25 mini-redis** — uses DT20 skip-list and DT18 hash-map, both of which are
  built on the hashing primitives in DT17, not on the graph hierarchy.
