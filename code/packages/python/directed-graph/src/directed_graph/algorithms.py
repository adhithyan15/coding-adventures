"""
algorithms.py — Pure Graph Algorithms for DirectedGraph[T]
===========================================================

All functions here are pure — they take a DirectedGraph as input and
return a result without mutating the graph.

Why module-level functions instead of methods on DirectedGraph?
---------------------------------------------------------------
The DT01 spec separates algorithms from the data structure.  This mirrors
how the graph package (DT00) works — algorithms live in algorithms.py while
the data structure lives in graph.py.  The separation has several benefits:

  1. Single-responsibility: the data structure focuses on storage and
     mutation; algorithms focus on computation.
  2. Testability: pure functions are trivial to test in isolation.
  3. Extensibility: new algorithms can be added without touching DirectedGraph.
  4. Reuse: callers can import just the algorithms they need.

Algorithms provided:
  topological_sort        — Kahn's BFS-based algorithm
  has_cycle               — 3-color iterative DFS
  transitive_closure      — BFS over forward edges
  transitive_dependents   — BFS over reverse edges
  independent_groups      — Kahn's grouped by wave (parallel levels)
  affected_nodes          — union of transitive_dependents
  strongly_connected_components — Kosaraju's two-pass iterative DFS

All algorithms are O(V + E) unless otherwise noted.


Topological Sort (Kahn's Algorithm)
------------------------------------
A topological ordering is a linear sequence where for every directed edge
u → v, u appears before v.  This only exists for DAGs (no cycles).

Kahn's algorithm maintains in-degree counts:

    1. Find all nodes with in-degree == 0 (no dependencies).
    2. Add them to a queue.
    3. Remove them one by one from the "virtual" graph (decrement in-degrees
       of their successors).
    4. When a successor's in-degree drops to 0, add it to the queue.
    5. Repeat until the queue is empty.
    6. If any nodes remain (in-degree > 0), there's a cycle.

Example — a dependency chain::

    parse → compile → link → package

    topological_sort(g) → ["parse", "compile", "link", "package"]


Cycle Detection (3-Color DFS)
------------------------------
Uses three "colors" to track node state during DFS:

  WHITE (0) — not yet visited
  GRAY  (1) — currently being explored (on the stack)
  BLACK (2) — fully explored (all descendants visited)

A directed cycle exists iff DFS finds a GRAY → GRAY edge (a back edge).
A GRAY → BLACK edge is a cross-edge or forward-edge and does NOT indicate
a cycle (the black node was already fully explored in another branch).

IMPORTANT: We use iterative DFS (explicit stack) rather than recursive DFS
to avoid Python's default recursion limit (~1000 frames).  Large dependency
graphs can easily have chains deeper than 1000 nodes.

The iterative 3-color algorithm uses a stack of (node, iterator) pairs.
When we push a node we color it GRAY; when we exhaust its successors we
color it BLACK and pop it.  If a successor is already GRAY, we found a cycle.

Example — cross-edge trap::

    A → B → C
    D → C       ← D visits C which is already BLACK (not GRAY): no cycle!

    has_cycle(g)  # False


Strongly Connected Components (Kosaraju's Algorithm)
------------------------------------------------------
An SCC is a maximal set of nodes where every node is reachable from every
other node (following directed edges).

Kosaraju's two-pass algorithm:

    Pass 1: DFS on the ORIGINAL graph, push nodes onto a stack in
            finish-time order (last-finished = top of stack).

    Pass 2: DFS on the REVERSED graph, popping nodes from the finish-time
            stack.  Each DFS in pass 2 finds exactly one SCC.

Why does this work?  The node that finishes last in pass 1 (top of stack)
must be in the "source" SCC — the one with no incoming edges from other SCCs.
In the reversed graph, this source SCC becomes a sink (no outgoing edges to
other SCCs), so a DFS from it can only reach nodes in the same SCC.

Both passes use iterative DFS to avoid stack overflow.

Example::

    Nodes: A B C D E
    Edges: A→B, B→C, C→A, C→D, D→E, E→D

    SCCs: {A,B,C}, {D,E}

    strongly_connected_components(g)
    # [frozenset({"A","B","C"}), frozenset({"D","E"})]   (order may vary)
"""

from __future__ import annotations

from collections import deque
from typing import TypeVar

from directed_graph.directed_graph import DirectedGraph

T = TypeVar("T")

# ---------------------------------------------------------------------------
# Color constants for 3-color DFS
# ---------------------------------------------------------------------------

WHITE = 0  # Not yet visited
GRAY = 1   # Currently on the DFS stack (being explored)
BLACK = 2  # Fully explored (all descendants visited)


# ---------------------------------------------------------------------------
# topological_sort
# ---------------------------------------------------------------------------


def topological_sort(graph: DirectedGraph[T]) -> list[T]:
    """Return a topological ordering of all nodes in the graph.

    Uses Kahn's BFS-based algorithm.  The ordering is deterministic:
    when multiple nodes are eligible (zero in-degree), they are processed
    in sorted order (by ``repr``).

    Raises ValueError if the graph contains a cycle (topological ordering
    only exists for DAGs).

    Returns an empty list for an empty graph.

    Time complexity: O(V + E).

    Example::

        g = DirectedGraph()
        g.add_edge("parse", "compile")
        g.add_edge("compile", "link")
        g.add_edge("compile", "typecheck")
        topological_sort(g)
        # ["parse", "typecheck", "compile", "link"]  (one valid ordering)

    ASCII diagram::

        parse → compile → link
                       ↘ typecheck
    """
    # In-degree = number of incoming edges.
    # We copy into a mutable dict so we can "virtually" remove nodes.
    in_degree: dict[T, int] = {
        node: len(graph._reverse[node]) for node in graph.nodes()
    }

    # Seed the queue with all nodes that have no dependencies (in-degree 0).
    # Sort for deterministic output when multiple nodes qualify.
    queue: deque[T] = deque(
        sorted((n for n, d in in_degree.items() if d == 0), key=repr)
    )

    result: list[T] = []

    while queue:
        node = queue.popleft()
        result.append(node)

        # "Remove" node: decrement in-degree of all its successors.
        for successor in sorted(graph._adj[node].keys(), key=repr):
            in_degree[successor] -= 1
            if in_degree[successor] == 0:
                queue.append(successor)

    # If we processed fewer nodes than exist, there's a cycle.
    total = len(graph)
    if len(result) != total:
        raise ValueError(
            f"Graph contains a cycle — topological sort impossible. "
            f"Processed {len(result)}/{total} nodes."
        )

    return result


# ---------------------------------------------------------------------------
# has_cycle
# ---------------------------------------------------------------------------


def has_cycle(graph: DirectedGraph[T]) -> bool:
    """Return True if the graph contains at least one directed cycle.

    Uses an iterative 3-color DFS:
    - WHITE (0): not yet visited
    - GRAY  (1): on the current DFS stack
    - BLACK (2): fully explored

    A cycle is detected when DFS finds an edge to a GRAY node (back edge).
    An edge to a BLACK node is a cross-edge and does NOT indicate a cycle.

    This iterative version avoids Python's recursion limit, which is essential
    for large graphs (e.g., a 10,000-node dependency graph).

    Time complexity: O(V + E).

    Example — 3-node cycle::

        A → B → C → A   ← cycle!
        has_cycle(g)     # True

    Example — cross-edge (NOT a cycle)::

        A → B → C
        D → C           ← cross-edge: C is BLACK when D finds it
        has_cycle(g)     # False
    """
    color: dict[T, int] = {node: WHITE for node in graph.nodes()}

    for start in graph.nodes():
        if color[start] != WHITE:
            continue

        # Iterative DFS using an explicit stack.
        # Each entry is (node, iterator_over_successors).
        # When we push a node, we color it GRAY.
        # When we exhaust its successors, we color it BLACK and pop.
        stack: list[tuple[T, object]] = []
        color[start] = GRAY
        stack.append((start, iter(graph._adj[start])))

        while stack:
            node, it = stack[-1]
            try:
                successor = next(it)  # type: ignore[arg-type]
                if color[successor] == GRAY:
                    # Back edge: successor is still on the stack → cycle!
                    return True
                if color[successor] == WHITE:
                    # Tree edge: explore this successor.
                    color[successor] = GRAY
                    stack.append((successor, iter(graph._adj[successor])))
                # If BLACK: cross-edge or forward-edge — not a cycle, skip.
            except StopIteration:
                # All successors of `node` explored → color it BLACK.
                color[node] = BLACK
                stack.pop()

    return False


# ---------------------------------------------------------------------------
# transitive_closure
# ---------------------------------------------------------------------------


def transitive_closure(graph: DirectedGraph[T], node: T) -> frozenset[T]:
    """Return all nodes reachable from ``node`` by following forward edges.

    The starting ``node`` itself is NOT included in the result (only the
    nodes it can reach).

    Uses BFS over the forward adjacency (_adj).

    Raises KeyError if the node does not exist.

    Time complexity: O(V + E).

    Example::

        g.add_edge("A", "B")
        g.add_edge("B", "C")
        g.add_edge("A", "D")

        transitive_closure(g, "A")  # frozenset({"B", "C", "D"})
        transitive_closure(g, "B")  # frozenset({"C"})
        transitive_closure(g, "C")  # frozenset()

    ASCII diagram::

        A → B → C
        A → D
        transitive_closure(A) = {B, C, D}
    """
    if not graph.has_node(node):
        raise KeyError(node)

    visited: set[T] = set()
    queue: deque[T] = deque()

    # Seed with direct successors of the starting node.
    for successor in graph._adj[node]:
        if successor not in visited:
            visited.add(successor)
            queue.append(successor)

    while queue:
        current = queue.popleft()
        for successor in graph._adj[current]:
            if successor not in visited:
                visited.add(successor)
                queue.append(successor)

    return frozenset(visited)


# ---------------------------------------------------------------------------
# transitive_dependents
# ---------------------------------------------------------------------------


def transitive_dependents(graph: DirectedGraph[T], node: T) -> frozenset[T]:
    """Return all nodes that transitively depend on ``node``.

    "Depends on" means: there is a directed path TO ``node`` from those nodes
    (following edges backwards).  This is the reverse reachability.

    The starting ``node`` itself is NOT included in the result.

    Uses BFS over the reverse adjacency (_reverse).

    Raises KeyError if the node does not exist.

    Time complexity: O(V + E).

    Example — in a build system where A → B means "A depends on B"::

        g.add_edge("compile", "parse")   # compile depends on parse
        g.add_edge("link", "compile")    # link depends on compile
        g.add_edge("package", "link")    # package depends on link

        transitive_dependents(g, "parse")
        # frozenset({"compile", "link", "package"})
        # If parse changes, all three must be rebuilt.

    ASCII diagram::

        package → link → compile → parse
        transitive_dependents(parse) = {compile, link, package}
    """
    if not graph.has_node(node):
        raise KeyError(node)

    visited: set[T] = set()
    queue: deque[T] = deque()

    # Seed with direct predecessors of the starting node.
    for predecessor in graph._reverse[node]:
        if predecessor not in visited:
            visited.add(predecessor)
            queue.append(predecessor)

    while queue:
        current = queue.popleft()
        for predecessor in graph._reverse[current]:
            if predecessor not in visited:
                visited.add(predecessor)
                queue.append(predecessor)

    return frozenset(visited)


# ---------------------------------------------------------------------------
# independent_groups
# ---------------------------------------------------------------------------


def independent_groups(graph: DirectedGraph[T]) -> list[list[T]]:
    """Partition nodes into levels of parallel execution.

    Uses a modified Kahn's algorithm: instead of processing one zero-in-degree
    node at a time, we process ALL of them simultaneously as a "wave".  Each
    wave is a group of nodes that:

    - Have all their dependencies satisfied by earlier waves.
    - Have no dependency on each other (can run in parallel).

    This is the core of parallel build systems — knowing which tasks can run
    at the same time.

    Raises ValueError if the graph contains a cycle.

    Returns an empty list for an empty graph.

    Time complexity: O(V + E).

    Example::

        #          ┌→ B →┐
        #  A →────→┤     ├→ D
        #          └→ C →┘

        independent_groups(g)
        # [["A"], ["B", "C"], ["D"]]
        # Level 0: A (no deps)
        # Level 1: B and C (both depend only on A, which is done)
        # Level 2: D (depends on B and C, which are done)

    In a real build system, level 1 jobs (B and C) can run concurrently.
    """
    in_degree: dict[T, int] = {
        node: len(graph._reverse[node]) for node in graph.nodes()
    }

    # First wave: all nodes with zero in-degree.
    current_wave: list[T] = sorted(
        (n for n, d in in_degree.items() if d == 0), key=repr
    )

    groups: list[list[T]] = []
    processed = 0

    while current_wave:
        groups.append(current_wave)
        processed += len(current_wave)

        next_wave_set: set[T] = set()
        for node in current_wave:
            for successor in graph._adj[node]:
                in_degree[successor] -= 1
                if in_degree[successor] == 0:
                    next_wave_set.add(successor)

        current_wave = sorted(next_wave_set, key=repr)

    if processed != len(graph):
        raise ValueError(
            f"Graph contains a cycle — independent_groups impossible. "
            f"Processed {processed}/{len(graph)} nodes."
        )

    return groups


# ---------------------------------------------------------------------------
# affected_nodes
# ---------------------------------------------------------------------------


def affected_nodes(
    graph: DirectedGraph[T], changed: frozenset[T]
) -> frozenset[T]:
    """Return all nodes affected by changes to the ``changed`` set.

    A node is "affected" if it is in ``changed``, or if it transitively
    depends on any node in ``changed``.  In a build system, these are the
    packages that must be rebuilt when the changed packages are modified.

    Nodes in ``changed`` that don't exist in the graph are silently ignored.

    Time complexity: O(|changed| * (V + E)).

    Example::

        # Build graph (A → B means A depends on B):
        g.add_edge("compile", "parse")
        g.add_edge("link", "compile")
        g.add_edge("package", "link")

        affected_nodes(g, frozenset({"parse"}))
        # frozenset({"parse", "compile", "link", "package"})
        # parse changed → everything downstream must rebuild

        affected_nodes(g, frozenset({"compile"}))
        # frozenset({"compile", "link", "package"})
        # parse is NOT affected — it's upstream, not downstream
    """
    result: set[T] = set()

    for node in changed:
        if graph.has_node(node):
            result.add(node)
            result.update(transitive_dependents(graph, node))

    return frozenset(result)


# ---------------------------------------------------------------------------
# strongly_connected_components
# ---------------------------------------------------------------------------


def strongly_connected_components(
    graph: DirectedGraph[T],
) -> list[frozenset[T]]:
    """Return all Strongly Connected Components (SCCs) of the graph.

    An SCC is a maximal set of nodes S such that for every pair (u, v) in S,
    there is a directed path from u to v AND from v to u.

    Uses Kosaraju's two-pass algorithm:

    Pass 1 — DFS on original graph, record finish times.
        We push each fully-explored node onto a finish stack.
        The last node to finish sits on top.

    Pass 2 — DFS on REVERSED graph, consuming finish stack.
        Pop the top of the finish stack.  If unvisited, do a DFS on the
        reversed graph from that node.  All nodes reachable in the reversed
        graph form one SCC.

    Why the reversed graph?  If node X is the "root" of an SCC (finishes
    last in pass 1), then in the reversed graph, a DFS from X can reach
    EXACTLY the other members of X's SCC — no more, no less.

    Time complexity: O(V + E).  Two linear DFS passes.

    Example::

        Nodes: A B C D E
        Edges: A→B, B→C, C→A   (cycle: one SCC)
               C→D, D→E, E→D   (D,E cycle: another SCC)

        strongly_connected_components(g)
        # [frozenset({"A","B","C"}), frozenset({"D","E"})]

    A DAG (no cycles) has every node in its own singleton SCC::

        A→B→C   (linear chain)
        strongly_connected_components(g)
        # [frozenset({"A"}), frozenset({"B"}), frozenset({"C"})]

    Note: the order of SCCs in the returned list is not guaranteed.
    """
    nodes = list(graph.nodes())

    if not nodes:
        return []

    # ------------------------------------------------------------------
    # Pass 1: iterative DFS on ORIGINAL graph to determine finish order.
    # ------------------------------------------------------------------
    # We need finish times: the order in which nodes complete DFS.
    # finish_stack[-1] is the last node to finish (highest finish time).

    visited_pass1: set[T] = set()
    finish_stack: list[T] = []

    for start in nodes:
        if start in visited_pass1:
            continue

        # Iterative DFS with post-order tracking.
        # Stack entries: (node, iterator, entered)
        #   entered=False means we just pushed this node (pre-order)
        #   entered=True  means we're returning from children (post-order)
        #
        # We use (node, iterator) pairs and track when iterator is exhausted.
        dfs1_stack: list[tuple[T, object]] = []
        visited_pass1.add(start)
        dfs1_stack.append((start, iter(graph._adj[start])))

        while dfs1_stack:
            node, it = dfs1_stack[-1]
            try:
                successor = next(it)  # type: ignore[arg-type]
                if successor not in visited_pass1:
                    visited_pass1.add(successor)
                    dfs1_stack.append((successor, iter(graph._adj[successor])))
            except StopIteration:
                # Node is fully explored — record finish time.
                finish_stack.append(node)
                dfs1_stack.pop()

    # ------------------------------------------------------------------
    # Pass 2: iterative DFS on REVERSED graph, consuming finish_stack.
    # ------------------------------------------------------------------
    # Process nodes in reverse finish order (last finished first).
    # Each DFS in pass 2 discovers exactly one SCC.

    visited_pass2: set[T] = set()
    sccs: list[frozenset[T]] = []

    while finish_stack:
        start = finish_stack.pop()
        if start in visited_pass2:
            continue

        # BFS/DFS on the reversed graph (follow _reverse edges backwards,
        # which means follow _adj edges on the REVERSED graph).
        # The reversed graph has edge v→u whenever the original has u→v.
        # So from `start` in the reversed graph we follow graph._reverse[start].
        scc_nodes: set[T] = set()
        dfs2_stack: list[tuple[T, object]] = []

        visited_pass2.add(start)
        scc_nodes.add(start)
        dfs2_stack.append((start, iter(graph._reverse[start])))

        while dfs2_stack:
            node, it = dfs2_stack[-1]
            try:
                predecessor = next(it)  # type: ignore[arg-type]
                if predecessor not in visited_pass2:
                    visited_pass2.add(predecessor)
                    scc_nodes.add(predecessor)
                    dfs2_stack.append(
                        (predecessor, iter(graph._reverse[predecessor]))
                    )
            except StopIteration:
                dfs2_stack.pop()

        sccs.append(frozenset(scc_nodes))

    return sccs
