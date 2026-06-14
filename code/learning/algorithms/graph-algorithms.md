# Graph Algorithms -- From Theory to Build Systems

This document covers the graph algorithms used in the coding-adventures
repository: how they work, why they matter, and where to find them in the
code. Every algorithm here has a real job -- it powers the build system that
decides what to compile, in what order, and what can run in parallel.

**Implementations referenced in this document:**

- Python: `code/packages/python/directed-graph/src/directed_graph/graph.py`
- Go: `code/packages/go/directed-graph/graph.go`
- Build tool (Go): `code/programs/go/build-tool/`

---

## Table of Contents

1. [What Is a Directed Graph?](#1-what-is-a-directed-graph)
2. [Representing a Graph: The Adjacency List](#2-representing-a-graph-the-adjacency-list)
3. [Kahn's Algorithm for Topological Sort](#3-kahns-algorithm-for-topological-sort)
4. [DFS-Based Topological Sort](#4-dfs-based-topological-sort)
5. [3-Color Cycle Detection (WHITE/GRAY/BLACK)](#5-3-color-cycle-detection-whitegrayblack)
6. [Independent Groups / Parallel Levels](#6-independent-groups--parallel-levels)
7. [Affected Nodes](#7-affected-nodes)
8. [Transitive Closure](#8-transitive-closure)
9. [Complexity Summary](#9-complexity-summary)
10. [Real-World Application: The Build Tool](#10-real-world-application-the-build-tool)

---

## 1. What Is a Directed Graph?

A **directed graph** (or "digraph") is a collection of **nodes** (also called
vertices) connected by **edges** (also called arcs), where every edge has a
direction -- it goes FROM one node TO another.

Think of it like a one-way street map. You can drive from A to B, but that
does not mean you can drive from B to A.

```
    Undirected:          Directed:

    A --- B              A ---> B
    |     |              |      |
    C --- D              v      v
                         C ---> D

  "A is connected       "A points to B and C.
   to B, C, etc."        B points to D.
                         C points to D."
```

### Nodes and Edges

A directed graph G = (V, E) consists of:

- **V** -- a set of nodes (vertices). In our build system, each node is a
  package name like `"python/logic-gates"` or `"go/build-tool"`.
- **E** -- a set of directed edges. Each edge is an ordered pair (u, v)
  meaning "u points to v". In our build system, an edge from A to B means
  "A must be built before B" (B depends on A).

### Key Vocabulary

| Term            | Meaning                                          |
|-----------------|--------------------------------------------------|
| **Successor**   | Node that u points TO (u -> v, so v is a successor of u) |
| **Predecessor** | Node that points TO u (w -> u, so w is a predecessor of u) |
| **In-degree**   | Number of edges pointing INTO a node             |
| **Out-degree**  | Number of edges pointing OUT of a node           |
| **Path**        | A sequence of nodes connected by edges           |
| **Cycle**       | A path that starts and ends at the same node     |
| **DAG**         | Directed Acyclic Graph -- a directed graph with no cycles |

### Why DAGs Matter for Build Systems

Build dependencies must form a DAG. If package A depends on B, and B depends
on A, you have a circular dependency -- neither can be built first. That is a
**cycle**, and it is an error. Every build system must detect cycles and
reject them.

---

## 2. Representing a Graph: The Adjacency List

There are two common ways to store a graph in memory:

1. **Adjacency matrix** -- a V x V grid where cell (i,j) = 1 if there is
   an edge from node i to node j. Simple but wastes space for sparse graphs.
2. **Adjacency list** -- each node stores a list (or set) of its neighbors.
   Space-efficient for sparse graphs, which is what dependency graphs are.

Our implementation uses **two adjacency dictionaries**: one for forward edges
(successors) and one for reverse edges (predecessors).

```
  Example graph:
                                  Forward dict:          Reverse dict:
    A ---> B ---> D               A: {B, C}              A: {}
    |             ^               B: {D}                 B: {A}
    +----> C -----+               C: {D}                 C: {A}
                                  D: {}                  D: {B, C}
```

### Why Two Dictionaries?

Many algorithms need to walk edges in both directions:

| Operation                | Needs         | With two dicts |
|--------------------------|---------------|----------------|
| "What does A depend on?" | predecessors  | O(1) lookup    |
| "What depends on A?"     | successors    | O(1) lookup    |
| Topological sort         | in-degrees    | O(1) per node  |
| Remove a node            | both dirs     | O(degree)      |

Without the reverse dict, finding predecessors would require scanning ALL
edges -- O(E) instead of O(1). The trade-off is that `add_edge` and
`remove_edge` must update both dicts, but that is O(1) per operation.

### Python Implementation

From `code/packages/python/directed-graph/src/directed_graph/graph.py`:

```python
class DirectedGraph:
    def __init__(self) -> None:
        self._forward: dict[object, set[object]] = {}  # node -> successors
        self._reverse: dict[object, set[object]] = {}  # node -> predecessors

    def add_node(self, node: object) -> None:
        if node not in self._forward:
            self._forward[node] = set()
            self._reverse[node] = set()

    def add_edge(self, from_node: object, to_node: object) -> None:
        if from_node == to_node:
            raise ValueError(f"Self-loops are not allowed")
        self.add_node(from_node)
        self.add_node(to_node)
        self._forward[from_node].add(to_node)
        self._reverse[to_node].add(from_node)
```

### Go Implementation

From `code/packages/go/directed-graph/graph.go`:

```go
type Graph struct {
    forward map[string]map[string]bool  // node -> set of successors
    reverse map[string]map[string]bool  // node -> set of predecessors
}
```

Go uses `map[string]bool` instead of a `set` type (Go does not have built-in
sets). Checking `forward["A"]["B"]` returns `true` if the edge A->B exists,
or `false` (the zero value) if it does not.

### Invariant

Both implementations maintain the same invariant: **every node that exists in
the graph has an entry in BOTH the forward and reverse dicts**, even if its
adjacency set is empty. This means `node in self._forward` is always a valid
"does this node exist?" check, and we never need to handle missing keys.

---

## 3. Kahn's Algorithm for Topological Sort

### What Is Topological Sort?

A **topological sort** (or "topo sort") produces a linear ordering of nodes
such that for every edge u -> v, u appears before v. In build system terms:
every dependency is built before the things that depend on it.

```
  Graph:        A ---> B ---> D
                |             ^
                +----> C -----+

  Valid topo orders:
    [A, B, C, D]    (B before C)
    [A, C, B, D]    (C before B)

  Invalid:
    [B, A, C, D]    (B appears before A, but A -> B)
```

A topological ordering only exists for DAGs. If the graph has a cycle, no
valid ordering exists (you cannot put all of A before B and B before A at the
same time).

### How Kahn's Algorithm Works

Kahn's algorithm (published by Arthur B. Kahn in 1962) works by repeatedly
removing nodes that have no incoming edges:

```
  ALGORITHM Kahn's Topological Sort:
    1. Compute in-degree for every node
    2. Put all nodes with in-degree 0 into a queue
    3. While the queue is not empty:
       a. Remove a node from the queue, add it to the result
       b. For each successor of that node:
          - Decrement the successor's in-degree by 1
          - If the successor's in-degree is now 0, add it to the queue
    4. If result contains all nodes: success (return the result)
       If some nodes are missing: the graph has a cycle (error)
```

### Step-by-Step Example

Let us trace Kahn's algorithm on this graph:

```
  A ---> B ---> D ---> E
  |             ^
  +----> C -----+
```

Edges: A->B, A->C, B->D, C->D, D->E

**Step 0: Compute in-degrees**

```
  Node:       A    B    C    D    E
  In-degree:  0    1    1    2    1
                   ^    ^    ^    ^
                   |    |    |    |
                  (A)  (A) (B,C) (D)
```

**Step 1: Queue all nodes with in-degree 0**

```
  Queue:  [A]
  Result: []
```

**Step 2: Process A**

Remove A from queue. Add to result. Decrement successors B and C.

```
  Queue:  [B, C]     (both now have in-degree 0)
  Result: [A]

  Node:       A    B    C    D    E
  In-degree:  -    0    0    2    1
```

**Step 3: Process B**

Remove B from queue. Add to result. Decrement successor D.

```
  Queue:  [C]
  Result: [A, B]

  Node:       A    B    C    D    E
  In-degree:  -    -    0    1    1
```

**Step 4: Process C**

Remove C from queue. Add to result. Decrement successor D.

```
  Queue:  [D]        (D now has in-degree 0)
  Result: [A, B, C]

  Node:       A    B    C    D    E
  In-degree:  -    -    -    0    1
```

**Step 5: Process D**

Remove D from queue. Add to result. Decrement successor E.

```
  Queue:  [E]
  Result: [A, B, C, D]

  Node:       A    B    C    D    E
  In-degree:  -    -    -    -    0
```

**Step 6: Process E**

Remove E from queue. Add to result. No successors.

```
  Queue:  []
  Result: [A, B, C, D, E]
```

All 5 nodes are in the result. Success! The topological order is
`[A, B, C, D, E]`.

### Visual Walkthrough

```
  Start:       A -----> B -----> D -----> E
               |                 ^
               +-------> C ------+

  Step 1:      [A]  (in-degree 0 -- no arrows pointing in)
               Process A. Decrement B and C.

  Step 2:      [B, C]  (both now have in-degree 0)
               Process B. Decrement D.

  Step 3:      [C]
               Process C. Decrement D (now 0).

  Step 4:      [D]
               Process D. Decrement E.

  Step 5:      [E]
               Process E. Queue empty. Done!

  Result:      A, B, C, D, E
```

### Cycle Detection with Kahn's

If the algorithm terminates but has NOT processed all nodes, the remaining
nodes form one or more cycles. Those nodes all have in-degree >= 1, meaning
they all have at least one predecessor that is also unprocessed -- a circular
dependency.

```
  Graph with cycle:    A ---> B ---> C ---> B  (C points back to B)

  In-degrees:   A: 0    B: 2    C: 1

  Step 1: Process A. Decrement B (now 1).
  Step 2: Queue is empty! B and C still have in-degree > 0.
  Result has 1 node but graph has 3. Cycle detected!
```

### Complexity

- **Time: O(V + E)** -- We visit every node once and examine every edge once.
- **Space: O(V)** -- The in-degree array and queue each hold at most V entries.

---

## 4. DFS-Based Topological Sort

There is an alternative approach using depth-first search. It is not used in
our codebase (we use Kahn's everywhere), but understanding it helps clarify
the relationship between DFS and topological ordering.

### How It Works

```
  ALGORITHM DFS-Based Topological Sort:
    1. For each unvisited node, run DFS
    2. When DFS finishes visiting a node (all successors explored),
       push it onto a stack (or prepend to result)
    3. The final stack/list is the topological order
```

The key insight: a node is added to the result only AFTER all of its
descendants have been added. This guarantees that if u -> v, then v is added
before u -- and since we reverse the order at the end, u appears before v.

### Example

```
  Graph:   A ---> B ---> C

  DFS from A:
    Visit A
      Visit B
        Visit C
        C has no successors. Push C. Stack: [C]
      B done. Push B. Stack: [C, B]
    A done. Push A. Stack: [C, B, A]

  Reverse: [A, B, C]   -- valid topological order
```

### How It Differs from Kahn's

| Feature            | Kahn's                          | DFS-based                    |
|--------------------|---------------------------------|------------------------------|
| Approach           | Remove zero in-degree nodes     | Post-order DFS + reverse     |
| Cycle detection    | Built-in (incomplete result)    | Requires extra color tracking|
| Parallel levels    | Easy to modify for groups       | Not natural                  |
| Implementation     | Queue-based, iterative          | Recursion or explicit stack  |
| Our choice         | Used for topo sort + groups     | Not used (but cycle detection uses DFS) |

We chose Kahn's for two reasons:
1. It naturally detects cycles (if we cannot process all nodes, there is a
   cycle).
2. It is easy to modify for independent groups (see Section 6).

---

## 5. 3-Color Cycle Detection (WHITE/GRAY/BLACK)

### The Problem

We need to determine whether a directed graph contains any cycles. A cycle
means there is a path from some node back to itself. In a build system, a
cycle means circular dependencies -- an error that must be reported.

### The Three Colors

During a DFS traversal, each node is assigned one of three colors:

```
  +-------+--------------------------------------------------+
  | Color | Meaning                                          |
  +-------+--------------------------------------------------+
  | WHITE | Not yet visited                                  |
  | GRAY  | Currently being explored (on the recursion stack)|
  | BLACK | Fully explored (all descendants visited)         |
  +-------+--------------------------------------------------+
```

The rule: **if during DFS we encounter a GRAY node, we have found a cycle.**

Why? A GRAY node is one we started exploring but have not finished. If we
reach it again, we have found a path from that node back to itself -- that
is a cycle (a "back edge" in DFS terminology).

### Example WITHOUT a Cycle (DAG)

```
  Graph:   A ---> B ---> C
           |             ^
           +----> D -----+

  DFS from A:

  Step 1: Visit A. Color A = GRAY
          +-------+-------+-------+-------+
          |   A   |   B   |   C   |   D   |
          | GRAY  | WHITE | WHITE | WHITE |
          +-------+-------+-------+-------+

  Step 2: Visit B (successor of A). Color B = GRAY
          +-------+-------+-------+-------+
          |   A   |   B   |   C   |   D   |
          | GRAY  | GRAY  | WHITE | WHITE |
          +-------+-------+-------+-------+

  Step 3: Visit C (successor of B). Color C = GRAY
          +-------+-------+-------+-------+
          |   A   |   B   |   C   |   D   |
          | GRAY  | GRAY  | GRAY  | WHITE |
          +-------+-------+-------+-------+

  Step 4: C has no successors. Color C = BLACK. Backtrack.
          +-------+-------+-------+-------+
          |   A   |   B   |   C   |   D   |
          | GRAY  | GRAY  | BLACK | WHITE |
          +-------+-------+-------+-------+

  Step 5: B fully explored. Color B = BLACK. Backtrack.
          +-------+-------+-------+-------+
          |   A   |   B   |   C   |   D   |
          | GRAY  | BLACK | BLACK | WHITE |
          +-------+-------+-------+-------+

  Step 6: Visit D (successor of A). Color D = GRAY
          +-------+-------+-------+-------+
          |   A   |   B   |   C   |   D   |
          | GRAY  | BLACK | BLACK | GRAY  |
          +-------+-------+-------+-------+

  Step 7: D's successor is C. C is BLACK (not GRAY). No cycle.
          Color D = BLACK. Backtrack.
          +-------+-------+-------+-------+
          |   A   |   B   |   C   |   D   |
          | GRAY  | BLACK | BLACK | BLACK |
          +-------+-------+-------+-------+

  Step 8: A fully explored. Color A = BLACK.
          All nodes are BLACK. No cycle found!
```

### Example WITH a Cycle

```
  Graph:   A ---> B ---> C ---> B   (C points back to B!)
                               ^--- this creates a cycle

  DFS from A:

  Step 1: Visit A. Color A = GRAY
          +-------+-------+-------+
          |   A   |   B   |   C   |
          | GRAY  | WHITE | WHITE |
          +-------+-------+-------+

  Step 2: Visit B. Color B = GRAY
          +-------+-------+-------+
          |   A   |   B   |   C   |
          | GRAY  | GRAY  | WHITE |
          +-------+-------+-------+

  Step 3: Visit C. Color C = GRAY
          +-------+-------+-------+
          |   A   |   B   |   C   |
          | GRAY  | GRAY  | GRAY  |
          +-------+-------+-------+

  Step 4: C's successor is B. B is GRAY!
          +-------+-------+-------+
          |   A   |   B   |   C   |
          | GRAY  | GRAY  | GRAY  |
          +-------+-------+-------+
                    ^^^^
                    GRAY node encountered during DFS!
                    This is a back edge. CYCLE DETECTED!
                    Cycle path: B -> C -> B
```

### Why BLACK Nodes Are Safe

When we encounter a BLACK node, it means we already fully explored that node
and all its descendants -- and found no cycle involving it. So reaching a
BLACK node from a different path is perfectly fine. It does NOT create a cycle.

### Truth Table for Edge Classification

When DFS at node u encounters a successor v:

```
  +------------------+-----------+-----------------------------------+
  | Color of v       | Edge type | Action                            |
  +------------------+-----------+-----------------------------------+
  | WHITE            | Tree edge | Continue DFS into v               |
  | GRAY             | Back edge | CYCLE FOUND! Return true.         |
  | BLACK            | Cross/Fwd | Skip v (already fully explored)   |
  +------------------+-----------+-----------------------------------+
```

### Python Implementation

From `code/packages/python/directed-graph/src/directed_graph/graph.py`:

```python
WHITE, GRAY, BLACK = 0, 1, 2

def has_cycle(self) -> bool:
    color = {node: self.WHITE for node in self._forward}

    def dfs(node):
        color[node] = self.GRAY
        for successor in self._forward[node]:
            if color[successor] == self.GRAY:
                return True              # Back edge -- cycle!
            if color[successor] == self.WHITE and dfs(successor):
                return True
        color[node] = self.BLACK
        return False

    for node in self._forward:
        if color[node] == self.WHITE:
            if dfs(node):
                return True
    return False
```

Note that we start DFS from every unvisited node. The graph might not be
connected -- there could be isolated subgraphs that we would miss if we only
started from one node.

### Complexity

- **Time: O(V + E)** -- Each node and edge is visited exactly once.
- **Space: O(V)** -- The color array plus recursion stack depth (at most V).

---

## 6. Independent Groups / Parallel Levels

### The Problem

Topological sort gives us a valid build order, but it is strictly sequential.
In reality, many packages have no dependency on each other and can be built
simultaneously. We want to partition the DAG into **levels** where everything
at the same level can run in parallel.

### The Idea

This is a modification of Kahn's algorithm. Instead of pulling nodes off the
queue one at a time, we pull ALL zero-in-degree nodes at once. They form one
"level" of independent tasks.

```
  ALGORITHM Independent Groups:
    1. Compute in-degree for every node
    2. Collect all nodes with in-degree 0 -- this is Level 0
    3. While there are nodes at the current level:
       a. Record the current level
       b. For each node in the current level:
          - Decrement in-degree of all its successors
       c. Collect all nodes whose in-degree just dropped to 0 -- next level
    4. If all nodes are accounted for: success
       Otherwise: cycle error
```

### Example: Diamond Dependency

```
  Graph:    A ----> B ----> D
            |               ^
            +-----> C ------+

  Edges: A->B, A->C, B->D, C->D
```

**Processing:**

```
  In-degrees:  A:0  B:1  C:1  D:2

  Level 0:  [A]           (in-degree 0)
            Decrement B (now 0), C (now 0)

  Level 1:  [B, C]        (both in-degree 0)
            Decrement D twice (now 0)

  Level 2:  [D]           (in-degree 0)

  Result:   [[A], [B, C], [D]]
```

**What this means for the build tool:**

```
  Time ------>

  Level 0:   [  build A  ]
  Level 1:   [  build B  ] [  build C  ]    <-- parallel!
  Level 2:   [  build D  ]

  Total time = time(A) + max(time(B), time(C)) + time(D)
  Instead of = time(A) + time(B) + time(C) + time(D)
```

### Example: Linear Chain (No Parallelism)

```
  Graph:   A ---> B ---> C ---> D

  Result:  [[A], [B], [C], [D]]
```

Every node depends on the previous one. No parallelism is possible.

### Example: Wide Tree (Maximum Parallelism)

```
  Graph:   A ---> B
           A ---> C
           A ---> D
           A ---> E

  Result:  [[A], [B, C, D, E]]
```

After A is built, B, C, D, and E can all run simultaneously.

### How the Build Tool Uses This

The executor at `code/programs/go/build-tool/internal/executor/executor.go`
calls `graph.IndependentGroups()` to get the level structure. For each level,
it launches one goroutine per package, limited by a semaphore:

```go
// Simplified from executor.go
groups, _ := graph.IndependentGroups()

for _, level := range groups {
    semaphore := make(chan struct{}, maxJobs)
    var wg sync.WaitGroup

    for _, pkg := range level {
        wg.Add(1)
        go func(p Package) {
            defer wg.Done()
            semaphore <- struct{}{}        // acquire slot
            defer func() { <-semaphore }() // release slot
            runPackageBuild(p)
        }(pkg)
    }
    wg.Wait()  // finish this level before starting the next
}
```

### Complexity

- **Time: O(V + E)** -- Same as Kahn's algorithm (same work, different grouping).
- **Space: O(V)** -- In-degree counts plus the level lists.

---

## 7. Affected Nodes

### The Problem

When you change a file in `logic-gates/`, which packages need to be rebuilt?
Obviously `logic-gates` itself, but also every package that depends on it
(directly or transitively).

### The Algorithm

```
  ALGORITHM Affected Nodes:
    Input: a set of "changed" nodes
    Output: the changed nodes + all their transitive dependents

    1. For each changed node:
       a. Add it to the result set
       b. Find all transitive dependents (BFS on forward edges)
       c. Add them to the result set
    2. Return the result set
```

### Example

```
  Graph:   logic-gates ---> arithmetic ---> cpu-simulator
                |
                +---------> alu
```

If `logic-gates` changes:

```
  Changed:     {logic-gates}
  Dependents:  {arithmetic, cpu-simulator, alu}
  Affected:    {logic-gates, arithmetic, cpu-simulator, alu}
```

All four packages need rebuilding. But if only `arithmetic` changes:

```
  Changed:     {arithmetic}
  Dependents:  {cpu-simulator}
  Affected:    {arithmetic, cpu-simulator}
```

Only two packages need rebuilding. `logic-gates` and `alu` are not affected
because they do not depend on `arithmetic`.

### Why "Forward" Edges Give Dependents

In our graph convention, edges go FROM dependency TO dependent:

```
  logic-gates ---edge---> arithmetic
  "logic-gates must be built before arithmetic"
  "arithmetic depends on logic-gates"
```

So to find everything that depends on `logic-gates`, we follow edges
**forward** from `logic-gates`. This is exactly what `TransitiveDependents`
does -- it is a BFS on the forward adjacency list.

### Python Implementation

From `code/packages/python/directed-graph/src/directed_graph/graph.py`:

```python
def affected_nodes(self, changed: set) -> set:
    result: set = set()
    for node in changed:
        if node in self._forward:
            result.add(node)
            result.update(self.transitive_dependents(node))
    return result
```

### How the Build Tool Uses This

The git-diff mode (`code/programs/go/build-tool/internal/gitdiff/gitdiff.go`)
maps changed files to packages, then calls `graph.AffectedNodes(changed)` to
find everything that needs rebuilding:

```
  git diff --name-only origin/main...HEAD
       |
       v
  changed files: [code/packages/python/logic-gates/src/and_gate.py]
       |
       v
  MapFilesToPackages: {python/logic-gates}
       |
       v
  graph.AffectedNodes: {python/logic-gates, python/arithmetic, ...}
       |
       v
  Build only these packages
```

### Complexity

- **Time: O(C * (V + E))** where C is the number of changed nodes. In the
  worst case (all nodes changed), this is O(V * (V + E)). In practice, C is
  small (a few changed packages), so it is effectively O(V + E).
- **Space: O(V)** -- The visited set and BFS queue.

---

## 8. Transitive Closure

### What Is Transitive Closure?

The **transitive closure** of a node is the set of all nodes reachable from
it by following directed edges. If you can get from A to B by any chain of
edges (A -> X -> Y -> B), then B is in A's transitive closure.

```
  Graph:   A ---> B ---> C ---> D

  Transitive closure of A: {B, C, D}
  Transitive closure of B: {C, D}
  Transitive closure of C: {D}
  Transitive closure of D: {}    (nothing reachable)
```

### Transitive Closure vs. Transitive Dependents

In our codebase, both operations follow forward edges. They are the same
underlying BFS, just named differently for clarity:

```
  transitive_closure(A)    = "everything downstream of A"
  transitive_dependents(A) = "everything that depends on A"
```

In our graph convention (edges go from dependency to dependent), these are
identical. The Go implementation makes this explicit:

```go
// From code/packages/go/directed-graph/graph.go
func (g *Graph) TransitiveDependents(node string) (map[string]bool, error) {
    return g.TransitiveClosure(node)
}
```

### The "Reverse" Direction

Sometimes you need the opposite: "what does node X depend on?" (its
transitive dependencies, not dependents). This is the transitive closure on
the **reverse** graph -- following predecessor edges instead of successor
edges.

The Python implementation provides this as `transitive_dependents()` (which
confusingly walks the reverse dict), while the build tool's hasher implements
its own `collectTransitivePredecessors()`.

### Algorithm

```
  ALGORITHM Transitive Closure (BFS):
    Input: a starting node
    Output: set of all reachable nodes

    1. Initialize a visited set and a queue with the start node's successors
    2. While the queue is not empty:
       a. Dequeue a node
       b. For each of its successors:
          - If not already visited, mark as visited and enqueue
    3. Return the visited set
```

We use BFS rather than DFS because:
1. It is simple and iterative (no recursion, no stack overflow risk).
2. It visits nodes level-by-level, which is easy to reason about.
3. Performance is identical -- both are O(V + E).

### When You Need It

- **Build systems**: "If I change package X, what else needs rebuilding?"
- **Garbage collection**: "What objects are reachable from the root set?"
- **Database queries**: "Find all descendants of this node in a hierarchy."
- **Access control**: "Can user X reach resource Y through any chain of
  permissions?"

### Complexity

- **Time: O(V + E)** -- BFS visits every reachable node and edge once.
- **Space: O(V)** -- The visited set and queue.

---

## 9. Complexity Summary

| Algorithm               | Time      | Space  | Used In             |
|-------------------------|-----------|--------|---------------------|
| Kahn's topological sort | O(V + E)  | O(V)   | Build ordering      |
| DFS cycle detection     | O(V + E)  | O(V)   | Validation          |
| Independent groups      | O(V + E)  | O(V)   | Parallel execution  |
| Transitive closure      | O(V + E)  | O(V)   | Dependency analysis |
| Affected nodes          | O(C*(V+E))| O(V)   | Change detection    |

Where:
- V = number of nodes (packages)
- E = number of edges (dependency relationships)
- C = number of changed nodes (typically small)

All algorithms are linear in the size of the graph, which means they scale
well. Even a monorepo with 1000 packages and 5000 dependency edges would
process in milliseconds.

---

## 10. Real-World Application: The Build Tool

The build tool at `code/programs/go/build-tool/` ties all these algorithms
together into a complete incremental build system.

### The Full Pipeline

```
  +-------------------+
  | 1. DISCOVERY      |   Scan for BUILD files, determine language
  | discovery.go      |   and build commands for each package.
  +-------------------+
           |
           v
  +-------------------+
  | 2. RESOLUTION     |   Read pyproject.toml / go.mod / .gemspec
  | resolver.go       |   to find dependencies. Build the directed
  |                   |   graph with edges: dep -> dependent.
  +-------------------+
           |
           v
  +-------------------+
  | 3. CHANGE DETECT  |   Two modes:
  | gitdiff.go /      |   a) git diff: find changed files, map to
  | hasher.go         |      packages, use AffectedNodes()
  |                   |   b) cache: hash files, compare to cache
  +-------------------+
           |
           v
  +-------------------+
  | 4. PLANNING       |   Call IndependentGroups() to get parallel
  | executor.go       |   build levels. Filter each level by
  |                   |   the affected/changed set.
  +-------------------+
           |
           v
  +-------------------+
  | 5. EXECUTION      |   For each level, launch goroutines for
  | executor.go       |   each package (semaphore-limited).
  |                   |   If a build fails, mark all transitive
  |                   |   dependents as "dep-skipped".
  +-------------------+
           |
           v
  +-------------------+
  | 6. CACHE UPDATE   |   Record new hashes for built packages.
  | cache.go          |   Save to .build-cache.json atomically.
  +-------------------+
```

### Which Graph Algorithm Is Used Where

| Step                | Algorithm             | Purpose                          |
|---------------------|-----------------------|----------------------------------|
| Cycle validation    | 3-color DFS           | Reject circular dependencies     |
| Build ordering      | Kahn's topo sort      | Ensure deps built first          |
| Parallel levels     | Independent groups    | Maximize build parallelism       |
| Change propagation  | Affected nodes (BFS)  | Find what to rebuild             |
| Dep hashing         | Transitive predecessors | Hash all transitive deps       |
| Failure propagation | Transitive predecessors | Skip dependents of failures    |

### Example: A Real Build

Imagine this dependency graph for the coding-adventures repo:

```
  logic-gates -----> arithmetic -----> cpu-simulator
       |                                    ^
       +-----------> alu -------------------+
       |
       +-----------> memory
```

If you edit a file in `logic-gates/`:

1. **git diff** detects the changed file
2. **MapFilesToPackages** maps it to `{logic-gates}`
3. **AffectedNodes** computes `{logic-gates, arithmetic, alu, memory, cpu-simulator}`
4. **IndependentGroups** produces:
   ```
   Level 0: [logic-gates]
   Level 1: [arithmetic, alu, memory]     <-- parallel
   Level 2: [cpu-simulator]
   ```
5. **Executor** builds level 0, then level 1 in parallel, then level 2
6. **Cache** records new hashes for all five packages

If you only edit a file in `alu/`:

1. **AffectedNodes** computes `{alu, cpu-simulator}`
2. Only two packages rebuilt instead of five
3. `logic-gates`, `arithmetic`, and `memory` are skipped entirely

This is the power of graph algorithms in a build system: they turn a naive
"rebuild everything" into a precise, parallel, incremental build.
