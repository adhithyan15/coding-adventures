"""
algorithms.py — Pure Graph Algorithms
======================================

All functions here are pure — they take a Graph as input and return a result.
They never mutate the graph.  They work identically on both ADJACENCY_LIST and
ADJACENCY_MATRIX graphs because they only call the Graph's public API.

Algorithms provided:
  bfs                   — breadth-first traversal
  dfs                   — depth-first traversal
  is_connected          — does every node reach every other?
  connected_components  — find all isolated clusters
  has_cycle             — does the graph contain a cycle?
  shortest_path         — fewest-hops or lowest-weight path
  minimum_spanning_tree — cheapest set of edges connecting all nodes

Each function has a worked example in its docstring so you can check your
understanding before reading the code.
"""

from __future__ import annotations

import heapq
from collections import deque
from typing import Generic, TypeVar

from graph.graph import Graph

T = TypeVar("T")


# ---------------------------------------------------------------------------
# BFS — Breadth-First Search
# ---------------------------------------------------------------------------
#
# BFS explores a graph level-by-level: first all nodes 1 hop from start,
# then all 2 hops, then 3 hops, etc.  Picture a stone dropped in water:
# the ripple rings expand outward one at a time.
#
#   Queue  (FIFO): nodes to visit, oldest first.
#   Visited set:   prevents revisiting nodes and infinite loops.
#
# Why a queue and not a stack?  A queue enforces level-by-level order.
# With a stack you get DFS instead.


def bfs(graph: Graph[T], start: T) -> list[T]:
    """Return nodes reachable from start in breadth-first order.

    Nodes not reachable from start (in a disconnected graph) are excluded.

    Time: O(V + E).  Space: O(V) for the visited set and queue.

    Example — city graph::

        London → Paris, Amsterdam
        Paris  → London, Berlin
        ...

        bfs(g, "London") → ["London", "Paris", "Amsterdam", "Berlin", "Brussels"]
        (level 0: London; level 1: Paris, Amsterdam; level 2: Berlin, Brussels)
    """
    visited: set[T] = {start}
    queue: deque[T] = deque([start])
    result: list[T] = []

    while queue:
        node = queue.popleft()
        result.append(node)
        # Sort neighbours for deterministic output.
        for neighbour in sorted(graph.neighbors(node), key=repr):
            if neighbour not in visited:
                visited.add(neighbour)
                queue.append(neighbour)

    return result


# ---------------------------------------------------------------------------
# DFS — Depth-First Search
# ---------------------------------------------------------------------------
#
# DFS explores as far as possible down each branch before backtracking.
# Think of solving a maze: go straight until you hit a dead end, back up,
# try the next turn.
#
# We use an explicit stack instead of recursion to avoid Python's default
# recursion limit (usually 1000 frames) on deep graphs.
#
# Key difference from BFS: the stack pops from the END, so the most
# recently added neighbours are explored first (LIFO order).


def dfs(graph: Graph[T], start: T) -> list[T]:
    """Return nodes reachable from start in depth-first order.

    Nodes not reachable from start (in a disconnected graph) are excluded.

    Time: O(V + E).  Space: O(V) for the visited set and stack.

    Example::

        A—B—D
        |
        C

        dfs(g, "A") → ["A", "B", "D", "C"]   (goes deep via B before C)
    """
    visited: set[T] = set()
    # Reverse-sort so that when we push all neighbours the first (alphabetically)
    # is on top — this makes output deterministic and matches intuition.
    stack: list[T] = [start]
    result: list[T] = []

    while stack:
        node = stack.pop()
        if node in visited:
            continue
        visited.add(node)
        result.append(node)
        for neighbour in sorted(graph.neighbors(node), key=repr, reverse=True):
            if neighbour not in visited:
                stack.append(neighbour)

    return result


# ---------------------------------------------------------------------------
# is_connected
# ---------------------------------------------------------------------------
#
# A graph is connected if every node can reach every other node.
# One BFS from any starting node visits ALL nodes iff the graph is connected.


def is_connected(graph: Graph[T]) -> bool:
    """Return True if every node can reach every other node.

    An empty graph is vacuously connected (True).
    A single-node graph is trivially connected (True).

    Time: O(V + E).
    """
    if len(graph) == 0:
        return True
    # Pick any node as the start.
    start = next(iter(graph.nodes()))
    return len(bfs(graph, start)) == len(graph)


# ---------------------------------------------------------------------------
# connected_components
# ---------------------------------------------------------------------------
#
# When a graph is disconnected it consists of several isolated clusters called
# "connected components".  Think of an archipelago: ships can travel between
# ports on the same island, but cannot cross to a different island.
#
# Algorithm: repeatedly BFS from any unvisited node, collecting each component.


def connected_components(graph: Graph[T]) -> list[frozenset[T]]:
    """Return a list of connected components, each as a frozenset of nodes.

    Example::

        Graph: A—B—C   D—E   F

        connected_components(g) → [{A, B, C}, {D, E}, {F}]

    Time: O(V + E).
    """
    unvisited = set(graph.nodes())
    components: list[frozenset[T]] = []

    while unvisited:
        start = next(iter(unvisited))
        component = frozenset(bfs(graph, start))
        components.append(component)
        unvisited -= component

    return components


# ---------------------------------------------------------------------------
# has_cycle
# ---------------------------------------------------------------------------
#
# An undirected graph has a cycle if DFS finds a "back edge" — an edge to a
# node already in the visited set that is NOT the node we came from.
#
# The "not our parent" check is essential: in an undirected graph every edge
# appears twice (u→v and v→u).  Without the parent check, the return edge
# would always look like a back edge and every edge would falsely indicate a
# cycle.
#
# Example:
#   A—B—C—A  (triangle)  → has_cycle = True
#   A—B—C    (path)       → has_cycle = False


def has_cycle(graph: Graph[T]) -> bool:
    """Return True if the graph contains any cycle.

    Uses iterative DFS (avoids Python's recursion limit on large graphs).

    Key insight: an undirected graph has a cycle iff DFS finds a "back edge" —
    an edge to an already-visited node that is NOT the node we came from.
    The parent check prevents counting the return edge (u→v, v→u) as a cycle.

    Time: O(V + E).
    """
    visited: set[T] = set()

    for start in graph.nodes():
        if start in visited:
            continue
        # Stack holds (node, parent) pairs.
        stack: list[tuple[T, T | None]] = [(start, None)]
        while stack:
            node, par = stack.pop()
            if node in visited:
                # Already processed from a different push; skip.
                continue
            visited.add(node)
            for neighbour in graph.neighbors(node):
                if neighbour not in visited:
                    stack.append((neighbour, node))
                elif neighbour != par:
                    # Back edge: visited neighbour that isn't our parent → cycle.
                    return True
    return False


# ---------------------------------------------------------------------------
# shortest_path
# ---------------------------------------------------------------------------
#
# Two strategies depending on edge weights:
#
#   All weights equal (or all 1.0):
#     BFS finds the shortest path in O(V + E).  BFS naturally explores
#     nodes in order of hop-count, so the first time it reaches the
#     destination it has found the shortest route.
#
#   Variable weights (Dijkstra's algorithm):
#     A priority queue (min-heap) always expands the cheapest unvisited
#     node.  Think of water flowing: it always takes the cheapest path
#     available, spreading outward until it reaches the destination.
#
# We detect "all weights equal" by checking if every edge weight == 1.0.
# If so, we use BFS.  Otherwise, Dijkstra.


def shortest_path(graph: Graph[T], start: T, end: T) -> list[T]:
    """Return the shortest (lowest-weight) path from start to end.

    Returns an empty list if no path exists.

    For unweighted graphs (all weights 1.0) uses BFS — O(V + E).
    For weighted graphs uses Dijkstra's algorithm — O((V + E) log V).

    Example (unweighted)::

        A—B—C—D

        shortest_path(g, "A", "D") → ["A", "B", "C", "D"]

    Example (weighted, Dijkstra)::

        A —1— B —10— D
          \\         /
            ——3—— C —3—

        shortest_path(g, "A", "D") → ["A", "C", "D"]   (cost 6 vs 11)
    """
    if start == end:
        return [start] if graph.has_node(start) else []

    # Decide strategy: BFS if all weights are 1.0, else Dijkstra.
    all_unit = all(w == 1.0 for _, _, w in graph.edges())

    if all_unit:
        return _bfs_path(graph, start, end)
    return _dijkstra(graph, start, end)


def _bfs_path(graph: Graph[T], start: T, end: T) -> list[T]:
    """BFS shortest path (for unweighted graphs)."""
    parent: dict[T, T | None] = {start: None}
    queue: deque[T] = deque([start])

    while queue:
        node = queue.popleft()
        if node == end:
            break
        for neighbour in graph.neighbors(node):
            if neighbour not in parent:
                parent[neighbour] = node
                queue.append(neighbour)

    if end not in parent:
        return []  # No path exists.

    # Trace back from end to start via parent pointers.
    path: list[T] = []
    cur: T | None = end
    while cur is not None:
        path.append(cur)
        cur = parent[cur]
    path.reverse()
    return path


def _dijkstra(graph: Graph[T], start: T, end: T) -> list[T]:
    """Dijkstra's algorithm for weighted shortest path."""
    INF = float("inf")
    dist: dict[T, float] = {node: INF for node in graph.nodes()}
    parent: dict[T, T | None] = {}
    dist[start] = 0.0

    # Min-heap entries: (distance, counter, node).
    # The counter breaks ties and avoids comparing T values directly.
    counter = 0
    heap: list[tuple[float, int, T]] = [(0.0, counter, start)]

    while heap:
        d, _, node = heapq.heappop(heap)
        if d > dist[node]:
            continue  # Stale entry — a shorter path was already found.
        if node == end:
            break
        for neighbour, weight in graph.neighbors_weighted(node).items():
            new_dist = dist[node] + weight
            if new_dist < dist[neighbour]:
                dist[neighbour] = new_dist
                parent[neighbour] = node
                counter += 1
                heapq.heappush(heap, (new_dist, counter, neighbour))

    if dist[end] == INF:
        return []  # No path exists.

    # Trace back.
    path: list[T] = []
    cur: T | None = end
    while cur is not None:
        path.append(cur)
        cur = parent.get(cur)
    path.reverse()
    return path


# ---------------------------------------------------------------------------
# minimum_spanning_tree — Kruskal's algorithm + Union-Find
# ---------------------------------------------------------------------------
#
# A spanning tree connects all V nodes with exactly V-1 edges and no cycles.
# The MINIMUM spanning tree does so with the lowest possible total weight.
#
# Real-world use: lay cables to connect N cities using the least total wire.
#
# Kruskal's algorithm:
#   1. Sort all edges by weight (cheapest first).
#   2. Greedily add each edge IF it doesn't create a cycle.
#      Cycle check: use Union-Find.  If both endpoints are already in the
#      same component, adding the edge would create a cycle — skip it.
#   3. Stop when we have V-1 edges (the spanning tree is complete).
#
# Union-Find (Disjoint Set Union):
#   - parent[v] = representative of v's component.
#   - find(v): walk parent pointers up to the root.
#   - union(a, b): merge the components of a and b.
#   - Path compression: during find, point every visited node directly to
#     the root.  This makes future finds nearly O(1) amortised.
#
# Example:
#   A —3— B
#   |       \
#   1         4
#   |           \
#   C —2— D —5— E
#
#   Sorted edges: (A,C,1), (C,D,2), (A,B,3), (B,E,4), (D,E,5)
#   Add (A,C,1) → MST: {(A,C,1)}
#   Add (C,D,2) → MST: {(A,C,1),(C,D,2)}
#   Add (A,B,3) → MST: {(A,C,1),(C,D,2),(A,B,3)}
#   Add (B,E,4) → MST complete!  (V-1 = 4 edges)
#   Skip (D,E,5) → would create a cycle


def minimum_spanning_tree(graph: Graph[T]) -> frozenset[tuple[T, T, float]]:
    """Return the minimum spanning tree as a frozenset of (u, v, weight) triples.

    Returns an empty frozenset if the graph is empty or has no edges.
    Raises ValueError if the graph is not connected (no spanning tree exists).

    Time: O(E log E) for sorting + O(E · α(V)) for Union-Find.

    Example::

        A—1—B, A—3—C, B—2—C   → MST = {(A,B,1), (B,C,2)}   (total weight 3)
    """
    all_nodes = list(graph.nodes())
    if not all_nodes:
        return frozenset()

    # Sort edges by weight.
    sorted_edges = sorted(graph.edges(), key=lambda e: e[2])

    uf = _UnionFind(all_nodes)
    mst: set[tuple[T, T, float]] = set()

    for u, v, w in sorted_edges:
        if uf.find(u) != uf.find(v):
            uf.union(u, v)
            mst.add((u, v, w))
            if len(mst) == len(all_nodes) - 1:
                break  # MST is complete.

    if len(mst) < len(all_nodes) - 1 and len(all_nodes) > 1:
        raise ValueError(
            "minimum_spanning_tree: graph is not connected — no spanning tree exists"
        )

    return frozenset(mst)


# ---------------------------------------------------------------------------
# Union-Find (helper for Kruskal's algorithm)
# ---------------------------------------------------------------------------
#
# Tracks which "component" (group) each node belongs to.
# Two nodes are in the same component iff find(a) == find(b).
#
# Path compression:  When we walk up the parent chain to find the root,
# we update every visited node to point DIRECTLY to the root.  This
# "flattens" the tree and makes future finds very fast.
#
#   Before find(E):   E → D → B → A (root)
#   After find(E):    E → A, D → A  (all point directly to root)


class _UnionFind(Generic[T]):
    """Union-Find with path compression for Kruskal's MST algorithm."""

    def __init__(self, nodes: list[T]) -> None:
        self._parent: dict[T, T] = {n: n for n in nodes}
        self._rank: dict[T, int] = {n: 0 for n in nodes}

    def find(self, x: T) -> T:
        """Return the representative (root) of x's component."""
        if self._parent[x] != x:
            self._parent[x] = self.find(self._parent[x])  # path compression
        return self._parent[x]

    def union(self, a: T, b: T) -> None:
        """Merge the components of a and b (union by rank)."""
        ra, rb = self.find(a), self.find(b)
        if ra == rb:
            return
        # Attach the shorter tree under the taller tree.
        if self._rank[ra] < self._rank[rb]:
            ra, rb = rb, ra
        self._parent[rb] = ra
        if self._rank[ra] == self._rank[rb]:
            self._rank[ra] += 1
