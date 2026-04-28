# DT01 — Directed Graph

## Overview

A directed graph (digraph) is a graph where every edge has a direction: an edge A→B means
"A points to B" but says nothing about whether B points back to A. This asymmetry makes
directed graphs the right model for any relationship that flows one way: dependencies,
imports, causality, state transitions. DT01 builds on DT00 (Graph) by adding a second
adjacency map that tracks predecessors, enabling O(1) lookup in both directions.

DT01 inherits the DT00 property model. Nodes, directed edges, and the graph itself can
carry property bags. This is deliberately part of the graph foundation rather than a
neural-network-specific layer, so packages such as state machines, build graphs, graph
visualizers, and future neural graphs all share the same metadata contract.

## Layer Position

```
DT00 graph (undirected base)
        │
        ▼
[YOU ARE HERE: DT01 directed-graph]
        │
        ▼
DT02 tree             (directed-graph: single root, no cycles)
        │
        ▼
DT03 binary-tree      (tree with max 2 children)
       ├── DT04 heap
       ├── DT05 segment-tree
       ├── DT07 binary-search-tree ── DT08/DT09/DT10
       └── DT11 b-tree ── DT12
DT06 fenwick-tree
DT13 trie ── DT14 radix-tree ── DT15 suffix-tree
DT16 rope
DT17–DT22 (hash structures, probabilistic)
DT23–DT25 (Redis protocol stack)
```

## Concepts

### Why direction matters

In an undirected graph, the road London—Paris goes both ways. But most interesting
relationships in software are **one-directional**:

```
"package A imports package B"  ≠  "package B imports package A"
"task A must run before B"     ≠  "task B must run before A"
"user A follows user B"        ≠  "user B follows user A"
"state A transitions to B"     ≠  "state B transitions to A"
```

A directed graph captures this asymmetry. Each edge is an **ordered pair** (u, v)
rather than an unordered pair {u, v}.

### Real-world directed graphs

```
Build system (this repo):
    main-program ──→ parser ──→ lexer
                 ──→ codegen ──→ lexer

Git commit history:
    commit-C ──→ commit-B ──→ commit-A  (each commit points to its parent)

Web links:
    Wikipedia/Python ──→ Wikipedia/Guido
    Wikipedia/Python ──→ Wikipedia/CPython
    (but Wikipedia/Guido may not link back)

Spreadsheet cells:
    C1 = A1 + B1   means C1 ──→ A1, C1 ──→ B1
    (changing A1 must recalculate C1 — dependency flows backwards)
```

### Successors and predecessors

In a directed graph, "neighbors" splits into two concepts:

```
Given edge A ──→ B:

  A is a PREDECESSOR of B   (also called: parent, dependency, source)
  B is a SUCCESSOR  of A   (also called: child, dependent, target)

successors(A)   = all nodes that A points TO    = {B, ...}
predecessors(B) = all nodes that point TO B     = {A, ...}
```

To make both directions O(1), we maintain **two adjacency maps**:

```
forward[u]  = {v | edge u→v exists}   # successors of u
reverse[v]  = {u | edge u→v exists}   # predecessors of v
```

When you add edge A→B:
- Add B to forward[A]
- Add A to reverse[B]

When you remove edge A→B:
- Remove B from forward[A]
- Remove A from reverse[B]

This doubles memory usage but pays off enormously: many graph algorithms need to
walk edges in reverse (e.g., finding everything that depends on a changed file).

### Directed edge properties

Directed edge properties are attached to the ordered edge `(u, v)`. Unlike DT00,
the reverse edge `(v, u)` is different and has its own property bag:

```
graph.add_edge("A", "B", properties: {"role": "forward"})
graph.add_edge("B", "A", properties: {"role": "reverse"})

graph.edge_properties("A", "B")["role"] == "forward"
graph.edge_properties("B", "A")["role"] == "reverse"
```

This distinction matters for neural graphs:

```
InputA -> Hidden1   weight: 0.42, trainable: true
Hidden1 -> Output   weight: 1.20, trainable: true
```

Both edges may connect related concepts, but their direction, weights, gradients, and
runtime traces are independent.

Directed graph implementations must expose the same property operations as DT00 using
language-idiomatic names:

```
add_node(node, properties = {})
add_edge(from, to, weight = 1.0, properties = {})

graph_properties() -> PropertyBag
node_properties(node) -> PropertyBag
edge_properties(from, to) -> PropertyBag

set_graph_property(key, value)
set_node_property(node, key, value)
set_edge_property(from, to, key, value)

remove_graph_property(key)
remove_node_property(node, key)
remove_edge_property(from, to, key)
```

If a directed-graph package internally composes or inherits from DT00 graph, it should
delegate this behavior to DT00 rather than duplicating a second metadata system. If a
language currently has a standalone directed graph implementation, it must still match
the DT00 property semantics so higher-level packages can target one graph contract.

### Directed Acyclic Graph (DAG)

A special and extremely common case: a directed graph with **no cycles**.

```
DAG example (build dependencies):
    app ──→ server ──→ db-driver
    app ──→ auth   ──→ db-driver
    server ──→ logger
    auth   ──→ logger
```

DAGs are everywhere:
- Build systems (packages depending on packages)
- Task schedulers (tasks with prerequisites)
- Git history (commits pointing to parents)
- Spreadsheet formulas
- Neural network layers
- Package managers (npm, pip, cargo)

DAGs allow **topological sorting** — ordering nodes such that all edges point forward.

### Self-loops

A self-loop is an edge from a node to itself: A→A. In most dependency graphs, self-loops
are meaningless or errors. The `allow_self_loops` flag controls whether they are permitted.

```
allow_self_loops = False (default):
    add_edge(A, A) → raises ValueError

allow_self_loops = True:
    add_edge(A, A) → allowed (useful for state machines with self-transitions)
```

### The 3-color DFS and cycle detection

For undirected graphs, cycle detection only needs a "visited/unvisited" distinction.
For directed graphs, we need three colors because an edge to a previously-visited node
is only a cycle if that node is still on the **current DFS path**:

```
WHITE = 0 = not yet visited
GRAY  = 1 = currently on the DFS stack (being explored)
BLACK = 2 = fully explored, no longer on the stack
```

Why three colors? Consider:
```
    A ──→ B ──→ C
              ↗
          D ─┘
```
When DFS finishes exploring B→C and returns to D, then D→C, node C is already BLACK
(fully explored). That is NOT a cycle — D just happens to also point to C.

But if we see:
```
    A ──→ B ──→ C ──→ B   ← B is GRAY (still on stack)
```
Then C→B, where B is GRAY, IS a cycle. B is an ancestor of C in the current path.

Two colors cannot distinguish "visited but finished" (BLACK) from "visited and still
on stack" (GRAY). Three colors can.

### Topological sort: Kahn's algorithm

Topological sort orders nodes such that every edge u→v has u before v in the output.
Only possible if the graph has no cycles (i.e., it is a DAG).

Kahn's algorithm uses the concept of **in-degree** — the number of incoming edges:

```
Kahn's algorithm:

1. Compute in_degree[v] for every node v
2. Initialize queue with all nodes where in_degree[v] == 0
   (nodes with no prerequisites — they can run immediately)
3. While queue is not empty:
     node = dequeue()
     add node to result
     for each successor s of node:
         in_degree[s] -= 1       ← remove the dependency on node
         if in_degree[s] == 0:
             enqueue(s)          ← s is now ready to run
4. If len(result) != len(graph.nodes()):
     cycle detected! (some nodes were never added)
```

Worked example:
```
Graph:  app ──→ server ──→ db-driver
        app ──→ auth   ──→ db-driver
        server ──→ logger
        auth   ──→ logger

In-degrees:  app=0, server=1, auth=1, logger=2, db-driver=2

Queue starts with: [app]

Step 1: dequeue app    → result=[app]
        app's successors: server(1→0), auth(1→0)
        Queue: [server, auth]

Step 2: dequeue server → result=[app, server]
        server's successors: db-driver(2→1), logger(2→1)
        Queue: [auth]

Step 3: dequeue auth   → result=[app, server, auth]
        auth's successors: db-driver(1→0), logger(1→0)
        Queue: [db-driver, logger]

Step 4: dequeue db-driver → result=[app, server, auth, db-driver]
Step 5: dequeue logger    → result=[app, server, auth, db-driver, logger]

Valid build order: app → server → auth → db-driver → logger
```

Time: O(V + E).

### Multi-directed graph extension

DT01 intentionally keeps one edge per ordered `(from, to)` pair. A future
`multi-directed-graph` package should build on the same concepts but assign each edge a
stable edge id so multiple parallel edges can exist between the same nodes:

```
edge1: A -> B, properties: {"channel": "data", "weight": 0.4}
edge2: A -> B, properties: {"channel": "trace", "weight": 1.0}
```

That package should relax the "one ordered pair, one edge" constraint without changing
the node property, edge property, graph property, traversal, or serialization vocabulary.
In other words: DT01 is the simple directed graph. `multi-directed-graph` is the same
semantic model with edge identity added.

### Strongly Connected Components (Kosaraju's algorithm)

A **strongly connected component** (SCC) is a maximal set of nodes where every node
can reach every other node *following directed edges*.

```
Example:
   A ──→ B ──→ C
   ↑           │
   └───────────┘  ← C points back to A

   D ──→ E

SCCs: {A, B, C}, {D}, {E}
A, B, C form an SCC because you can get from any of them to any other.
D and E are their own SCCs because D→E but E does not point back to D.
```

**Kosaraju's two-pass DFS:**

The insight: if you reverse all edges in the graph, the SCCs are the same (a cycle
in the forward graph is still a cycle in the reversed graph). The algorithm uses this:

```
Pass 1: Run DFS on the original graph.
        Record finish times (push each node onto a stack when DFS finishes it).
        The last node to finish is the one with the "highest reach."

Pass 2: Transpose the graph (reverse all edges).
        Process nodes in reverse finish order (pop from stack).
        Each DFS in pass 2 on the transposed graph visits exactly one SCC.
```

Why does this work? Intuitively: the node that finishes last in pass 1 must be in
a "source SCC" (one with no incoming edges from other SCCs). Reversing edges and
starting DFS from there will only reach nodes in that same SCC, because the path
out of the SCC (in the original) becomes a path in (in the reversed) — but there
is no path in from other SCCs.

```
Kosaraju pseudocode:

kosaraju(graph):
    # Pass 1: finish-time ordering
    visited = set()
    stack   = []
    for node in graph.nodes():
        if node not in visited:
            dfs_pass1(graph, node, visited, stack)

    # Pass 2: SCC extraction on transposed graph
    transposed = transpose(graph)
    visited    = set()
    sccs       = []
    while stack:
        node = stack.pop()
        if node not in visited:
            scc = []
            dfs_pass2(transposed, node, visited, scc)
            sccs.append(frozenset(scc))
    return sccs
```

Time: O(V + E) for each pass → O(V + E) total.

### Independent groups (parallel execution levels)

Given a DAG representing task dependencies, `independent_groups` computes which tasks
can run in parallel at each "wave":

```
Build graph:       app → server → db-driver
                   app → auth   → db-driver
                   server → logger

Groups:
  Wave 0 (no deps):           [db-driver, logger]
  Wave 1 (db-driver + logger done): [server, auth]
  Wave 2 (server + auth done):      [app]

In a parallel build system, all tasks in wave 0 run concurrently,
then all tasks in wave 1, etc.
```

This is computed by repeatedly finding all nodes with in-degree 0, removing them, and
repeating — essentially Kahn's algorithm grouped by iteration.

### Affected nodes (incremental build)

`affected_nodes(graph, changed_set)` answers the question: "if these packages changed,
which packages need to be rebuilt?"

The answer is the union of transitive dependents of all changed packages:

```
affected_nodes(graph, changed):
    result = set()
    for node in changed:
        result |= transitive_dependents(graph, node)
    return result
```

`transitive_dependents` is a BFS/DFS over the **reverse** graph (predecessor edges):
"who depends on me? and who depends on them?" This is where the reverse adjacency map
pays off — without it, finding predecessors would require scanning all edges.

### LabeledDirectedGraph

A `LabeledDirectedGraph` wraps `DirectedGraph` and adds a string label to each edge.
Used by state machines: nodes are states, edges are transitions, labels are the events
that trigger each transition.

```
State machine for a traffic light:

Nodes:  {RED, GREEN, YELLOW}
Edges:  RED ──"timer"──→ GREEN
        GREEN ──"timer"──→ YELLOW
        YELLOW ──"timer"──→ RED

LabeledDirectedGraph stores:
  labels: Map<(u, v), str>
```

## Representation

DirectedGraph stores two sets of adjacency maps for O(1) lookup in both directions:

```
nodes:    Set[T]
forward:  Map[T, Map[T, float]]   # u → {v: weight} (successors)
reverse:  Map[T, Map[T, float]]   # v → {u: weight} (predecessors)
allow_self_loops: bool
```

The `forward` map is the source of truth for edges. The `reverse` map is derived
and kept in sync on every add/remove operation.

For `LabeledDirectedGraph`:
```
inner:  DirectedGraph[T]
labels: Map[Tuple[T, T], str]     # (u, v) → label string
```

## Algorithms (Pure Functions)

```
topological_sort(graph)          → List[T]          Kahn's; raises if cycle
has_cycle(graph)                 → bool              3-color DFS
transitive_closure(graph, node)  → frozenset[T]      BFS forward (all reachable)
transitive_dependents(graph, node) → frozenset[T]    BFS reverse (all that depend)
independent_groups(graph)        → List[List[T]]     Kahn's grouped by wave
affected_nodes(graph, changed)   → frozenset[T]      union of transitive_dependents
strongly_connected_components(graph) → List[frozenset[T]]  Kosaraju's
```

All algorithms accept a `DirectedGraph[T]` and return plain Python values.
None of them mutate the graph.

## Public API

```python
from typing import Generic, TypeVar, Optional

T = TypeVar("T")  # node type — must be hashable

class DirectedGraph(Graph[T]):
    """
    Directed graph built on top of Graph (DT00).

    In Python/Ruby/TypeScript: inherits from Graph.
    In Rust/Go/Elixir/Lua/Perl/Swift: wraps Graph via composition.
    """

    def __init__(self, allow_self_loops: bool = False) -> None: ...

    # Inherited from Graph (work on directed edges):
    # add_node, remove_node, has_node, nodes
    # add_edge(u, v, weight) — directed: u→v only
    # remove_edge(u, v)
    # has_edge(u, v)
    # edges() → frozenset of (u, v, weight)
    # edge_weight(u, v)

    # New: directional neighborhood
    def successors(self, node: T) -> frozenset[T]: ...
    def predecessors(self, node: T) -> frozenset[T]: ...
    def out_degree(self, node: T) -> int: ...
    def in_degree(self, node: T) -> int: ...

    # neighbors(node) is kept as an alias for successors(node)
    # for compatibility with Graph-based algorithms

class LabeledDirectedGraph(Generic[T]):
    """
    Wraps DirectedGraph and attaches a string label to each edge.
    Used for state machines.
    """

    def __init__(self, allow_self_loops: bool = False) -> None: ...

    # Delegates all DirectedGraph operations to inner graph
    def add_node(self, node: T) -> None: ...
    def remove_node(self, node: T) -> None: ...
    def has_node(self, node: T) -> bool: ...
    def nodes(self) -> frozenset[T]: ...
    def add_edge(self, u: T, v: T, label: str, weight: float = 1.0) -> None: ...
    def remove_edge(self, u: T, v: T) -> None: ...
    def has_edge(self, u: T, v: T) -> bool: ...
    def edge_label(self, u: T, v: T) -> str: ...       # raises KeyError if missing
    def edges_labeled(self) -> frozenset[tuple[T, T, str, float]]: ...
    def successors(self, node: T) -> frozenset[T]: ...
    def predecessors(self, node: T) -> frozenset[T]: ...

# ─── Pure function algorithms ────────────────────────────────────────────────

def topological_sort(graph: DirectedGraph[T]) -> list[T]: ...
    # Raises ValueError if graph contains a cycle.

def has_cycle(graph: DirectedGraph[T]) -> bool: ...
    # 3-color DFS. O(V + E).

def transitive_closure(graph: DirectedGraph[T], node: T) -> frozenset[T]: ...
    # All nodes reachable FROM node (following forward edges). Excludes node itself.

def transitive_dependents(graph: DirectedGraph[T], node: T) -> frozenset[T]: ...
    # All nodes that can reach node (following forward edges in reverse).
    # i.e., everything that depends on node.

def independent_groups(graph: DirectedGraph[T]) -> list[list[T]]: ...
    # Groups nodes into parallel execution waves. Requires DAG.

def affected_nodes(
    graph: DirectedGraph[T],
    changed: frozenset[T],
) -> frozenset[T]: ...
    # Union of transitive_dependents for all nodes in changed.

def strongly_connected_components(
    graph: DirectedGraph[T],
) -> list[frozenset[T]]: ...
    # Kosaraju's algorithm. O(V + E).
```

## Composition Model

DirectedGraph adds a `reverse` adjacency map on top of Graph's storage. The composition
model per language:

- **Python, Ruby, TypeScript** — `class DirectedGraph(Graph[T])`. The subclass calls
  `super().add_edge(u, v, weight)` then also updates `self._reverse`. The `neighbors()`
  method is overridden to return successors (forward edges only), preserving compatibility
  with Graph-based algorithms like `bfs` and `dfs`.

- **Rust** — `DirectedGraph<T>` holds an `inner: Graph<T>` for the forward adjacency
  and adds `reverse: HashMap<T, HashSet<T>>`. The `Graph` trait is implemented by
  delegating to `inner` for read operations.

- **Go** — `type DirectedGraph[T comparable] struct { inner Graph[T]; reverse map[T]map[T]float64 }`.
  Methods on `DirectedGraph` call `inner.AddEdge` then update `reverse`.

- **Elixir** — `DirectedGraph` is a struct with `%{inner: Graph.t(), reverse: map()}`.
  Pure functions operate on this struct and return updated copies.

- **Lua, Perl** — Tables with metatables delegating to the inner graph table.

- **Swift** — `struct DirectedGraph<T: Hashable>` embeds `var inner: Graph<T>` and adds
  `var reverse: [T: [T: Double]]`.

`LabeledDirectedGraph` always uses composition in all languages, wrapping `DirectedGraph`.

## Test Strategy

- Basic directed operations: add_edge(A, B), verify has_edge(A, B) is True,
  has_edge(B, A) is False (undirected Graph would return True for both).
- Successors / predecessors: build a diamond graph, verify predecessors and successors
  at each node.
- In-degree / out-degree: verify against manually computed values.
- Self-loops: verify allow_self_loops=False rejects A→A; allow_self_loops=True accepts it.
- topological_sort: DAG with known expected order; cycle graph raises ValueError.
  Verify that all edges point forward in the returned order.
- has_cycle: acyclic DAG → False; graph with one back edge → True.
  Test the 3-color logic: a "cross edge" to a BLACK node must not be treated as a cycle.
- transitive_closure: node with no successors → empty set; node at root of deep chain →
  all descendants.
- transitive_dependents: verify BFS over reverse edges; node at a leaf → all ancestors.
- independent_groups: verify the diamond example above gives exactly 3 waves.
- affected_nodes: change one deep node; verify all its transitive dependents are returned.
- strongly_connected_components: graph with 2 SCCs; single-node graph; fully cyclic graph.
- LabeledDirectedGraph: add edges with labels, retrieve labels, verify delegation.
- 100% API compatibility: all algorithms from DT00 (bfs, dfs, is_connected, etc.) must work
  unchanged on a DirectedGraph since it is a subtype of Graph.

## Future Extensions

- **DT02 tree** — DirectedGraph with invariants: single root, every non-root has exactly one
  parent, no cycles. Adds parent/child navigation and tree-specific traversals.
- **DT13 trie** — a tree where edges are labeled with characters from a string. Uses
  `LabeledDirectedGraph` internally.
- The build system in `code/programs/go/build-tool/` already uses the directed graph
  structure described here (Go implementation). The Python/Ruby/TypeScript/Rust packages
  will be the first publishable implementations of DT01.
