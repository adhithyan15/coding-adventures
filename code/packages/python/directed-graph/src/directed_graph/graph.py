"""
graph.py -- Directed Graph with Built-in Algorithms
====================================================

This module contains the entire directed graph implementation: the data structure,
mutation methods, query methods, and graph algorithms. We keep everything in one
class because the algorithms need intimate access to the internal adjacency dicts,
and splitting them into a separate module would just add indirection without any
real benefit.

Internal Storage
----------------

We maintain **two** adjacency dictionaries:

- ``_forward[u]``  = set of nodes that ``u`` points TO   (successors / children)
- ``_reverse[v]``  = set of nodes that point TO ``v``     (predecessors / parents)

Every node that exists in the graph has an entry in both dicts, even if its
adjacency set is empty. This invariant lets us use ``node in self._forward``
as the canonical "does this node exist?" check, and it means we never need
to special-case missing keys.

Why two dicts? Because many of our algorithms need to walk edges in *both*
directions efficiently:

- ``topological_sort`` needs to find nodes with zero in-degree, which means
  checking ``len(self._reverse[node]) == 0`` -- O(1) with the reverse dict.
- ``transitive_dependents`` walks *backwards* from a node, which is just a
  forward traversal on ``_reverse``.
- ``remove_node`` needs to clean up both incoming and outgoing edges, which
  is O(degree) with both dicts but would be O(E) with only one.

The trade-off is that every ``add_edge`` and ``remove_edge`` must update both
dicts, but that's O(1) per operation, so it's a good deal.

Error Classes
-------------

We define three custom exceptions:

- ``CycleError`` -- raised when a topological sort is requested on a graph that
  contains a cycle. It stores the cycle path so the caller can report which
  nodes are involved.
- ``NodeNotFoundError`` -- raised when an operation references a node that
  doesn't exist in the graph (e.g., ``remove_node("X")`` when X was never added).
- ``EdgeNotFoundError`` -- raised when ``remove_edge(u, v)`` is called but
  the edge u -> v doesn't exist.
"""

from __future__ import annotations

from collections import deque
from typing import TypeVar

# ---------------------------------------------------------------------------
# Type variable for generic node types
# ---------------------------------------------------------------------------
# The graph is generic over its node type. In practice most users will use
# strings, but you could use integers, enums, or any hashable type.

T = TypeVar("T")


# ---------------------------------------------------------------------------
# Custom exceptions
# ---------------------------------------------------------------------------
# Each exception carries enough context for the caller to produce a useful
# error message. We inherit from Exception (not RuntimeError) because these
# are expected, recoverable errors -- not bugs in the library.


class CycleError(Exception):
    """Raised when a topological sort encounters a cycle.

    The ``cycle`` attribute contains a list of nodes forming the cycle,
    starting and ending with the same node. For example, if the graph has
    edges A -> B -> C -> A, the cycle might be ``["A", "B", "C", "A"]``.
    """

    def __init__(self, message: str, cycle: list) -> None:
        super().__init__(message)
        self.cycle = cycle


class NodeNotFoundError(Exception):
    """Raised when an operation references a node not in the graph.

    The ``node`` attribute contains the missing node value.
    """

    def __init__(self, node: object) -> None:
        super().__init__(f"Node not found: {node!r}")
        self.node = node


class EdgeNotFoundError(Exception):
    """Raised when ``remove_edge`` targets a nonexistent edge.

    The ``from_node`` and ``to_node`` attributes identify the missing edge.
    """

    def __init__(self, from_node: object, to_node: object) -> None:
        super().__init__(f"Edge not found: {from_node!r} -> {to_node!r}")
        self.from_node = from_node
        self.to_node = to_node


# ---------------------------------------------------------------------------
# The DirectedGraph class
# ---------------------------------------------------------------------------


class DirectedGraph:
    """A directed graph backed by forward and reverse adjacency dictionaries.

    The graph stores nodes of any hashable type (strings by default). Edges
    are directed: ``add_edge("A", "B")`` means A points to B, so B is a
    *successor* of A and A is a *predecessor* of B.

    By default, self-loops are disallowed -- ``add_edge("A", "A")`` raises
    ``ValueError``. Pass ``allow_self_loops=True`` to permit them.

    When self-loops ARE allowed, a self-loop edge A -> A means A appears in
    its own successor set AND its own predecessor set. This naturally creates
    a cycle, so ``has_cycle()`` returns True and ``topological_sort()`` raises
    ``CycleError`` for any graph containing a self-loop.

    Duplicate edges and nodes are silently ignored (idempotent adds).

    Example::

        g = DirectedGraph()
        g.add_edge("compile", "link")
        g.add_edge("link", "package")
        print(g.topological_sort())   # ['compile', 'link', 'package']

    Example with self-loops::

        g = DirectedGraph(allow_self_loops=True)
        g.add_edge("A", "A")          # OK -- self-loop allowed
        print(g.has_cycle())           # True
        print(g.has_edge("A", "A"))    # True
    """

    # ------------------------------------------------------------------
    # Initialization
    # ------------------------------------------------------------------
    # We start with empty dicts. The invariant is: every node that exists
    # in the graph has a key in BOTH _forward and _reverse.
    #
    # The ``allow_self_loops`` flag controls whether add_edge permits
    # from_node == to_node. When False (the default), the graph behaves
    # as a strict DAG-oriented structure. When True, self-loops are stored
    # like any other edge: the node appears in both its own _forward and
    # _reverse sets.

    def __init__(self, *, allow_self_loops: bool = False) -> None:
        self._forward: dict[object, set[object]] = {}
        self._reverse: dict[object, set[object]] = {}
        self._allow_self_loops: bool = allow_self_loops

    # ------------------------------------------------------------------
    # Node operations
    # ------------------------------------------------------------------

    def add_node(self, node: object) -> None:
        """Add a node to the graph. No-op if the node already exists.

        This is called implicitly by ``add_edge``, so you only need to call
        it directly for isolated nodes (nodes with no edges).
        """
        if node not in self._forward:
            self._forward[node] = set()
            self._reverse[node] = set()

    def remove_node(self, node: object) -> None:
        """Remove a node and all its incoming/outgoing edges.

        Raises ``NodeNotFoundError`` if the node doesn't exist.

        This is O(in-degree + out-degree) because we need to update the
        adjacency sets of all neighbors.
        """
        if node not in self._forward:
            raise NodeNotFoundError(node)

        # Clean up outgoing edges: for each successor, remove `node` from
        # that successor's reverse (predecessor) set.
        for successor in self._forward[node]:
            self._reverse[successor].discard(node)

        # Clean up incoming edges: for each predecessor, remove `node` from
        # that predecessor's forward (successor) set.
        for predecessor in self._reverse[node]:
            self._forward[predecessor].discard(node)

        # Finally, remove the node itself from both dicts.
        del self._forward[node]
        del self._reverse[node]

    def has_node(self, node: object) -> bool:
        """Return True if the node exists in the graph."""
        return node in self._forward

    def nodes(self) -> list:
        """Return a list of all nodes in the graph.

        The order is arbitrary (dict insertion order in CPython 3.7+, but
        we don't guarantee it).
        """
        return list(self._forward.keys())

    # ------------------------------------------------------------------
    # Edge operations
    # ------------------------------------------------------------------

    def add_edge(self, from_node: object, to_node: object) -> None:
        """Add a directed edge from ``from_node`` to ``to_node``.

        Both nodes are implicitly added if they don't exist yet. This means
        you can build a graph entirely with ``add_edge`` calls -- no need
        to call ``add_node`` first.

        Raises ``ValueError`` if ``from_node == to_node`` (self-loops are
        not allowed in a DAG-oriented graph).

        Duplicate edges are silently ignored (sets handle deduplication).
        """
        if from_node == to_node and not self._allow_self_loops:
            raise ValueError(
                f"Self-loops are not allowed: {from_node!r} -> {to_node!r}"
            )

        # Ensure both nodes exist (idempotent).
        self.add_node(from_node)
        self.add_node(to_node)

        # Add the edge to both adjacency dicts.
        self._forward[from_node].add(to_node)
        self._reverse[to_node].add(from_node)

    def remove_edge(self, from_node: object, to_node: object) -> None:
        """Remove the directed edge from ``from_node`` to ``to_node``.

        Raises ``EdgeNotFoundError`` if the edge doesn't exist (including
        if either node doesn't exist).
        """
        if (
            from_node not in self._forward
            or to_node not in self._forward[from_node]
        ):
            raise EdgeNotFoundError(from_node, to_node)

        self._forward[from_node].discard(to_node)
        self._reverse[to_node].discard(from_node)

    def has_edge(self, from_node: object, to_node: object) -> bool:
        """Return True if the directed edge from_node -> to_node exists."""
        return (
            from_node in self._forward and to_node in self._forward[from_node]
        )

    def edges(self) -> list[tuple]:
        """Return a list of all edges as (from_node, to_node) tuples.

        The order is arbitrary.
        """
        result: list[tuple] = []
        for node, successors in self._forward.items():
            for successor in successors:
                result.append((node, successor))
        return result

    # ------------------------------------------------------------------
    # Neighbor queries
    # ------------------------------------------------------------------

    def predecessors(self, node: object) -> list:
        """Return the direct predecessors (parents) of a node.

        These are the nodes that have an edge pointing TO this node.
        Raises ``NodeNotFoundError`` if the node doesn't exist.
        """
        if node not in self._reverse:
            raise NodeNotFoundError(node)
        return list(self._reverse[node])

    def successors(self, node: object) -> list:
        """Return the direct successors (children) of a node.

        These are the nodes that this node points TO.
        Raises ``NodeNotFoundError`` if the node doesn't exist.
        """
        if node not in self._forward:
            raise NodeNotFoundError(node)
        return list(self._forward[node])

    # ------------------------------------------------------------------
    # Dunder methods
    # ------------------------------------------------------------------

    def __len__(self) -> int:
        """Return the number of nodes in the graph."""
        return len(self._forward)

    def __contains__(self, node: object) -> bool:
        """Return True if the node exists in the graph (same as has_node)."""
        return self.has_node(node)

    def __repr__(self) -> str:
        base = f"DirectedGraph(nodes={len(self)}, edges={len(self.edges())})"
        if self._allow_self_loops:
            base = f"DirectedGraph(nodes={len(self)}, edges={len(self.edges())}, allow_self_loops=True)"
        return base

    # ==================================================================
    # ALGORITHMS
    # ==================================================================
    # All algorithms are methods on the graph itself. This keeps the API
    # simple: you just call g.topological_sort() instead of importing a
    # separate module.

    # ------------------------------------------------------------------
    # Topological Sort (Kahn's Algorithm)
    # ------------------------------------------------------------------
    #
    # Kahn's algorithm works by repeatedly removing nodes with zero
    # in-degree from the graph. The order in which we remove them is a
    # valid topological ordering.
    #
    # Why Kahn's instead of DFS-based? Two reasons:
    # 1. It naturally detects cycles (if we can't remove all nodes, there's
    #    a cycle).
    # 2. It's easier to modify for independent_groups (see below).
    #
    # Time complexity: O(V + E) where V = nodes, E = edges.

    def topological_sort(self) -> list:
        """Return a topological ordering of all nodes.

        A topological ordering is a linear sequence where for every edge
        u -> v, u appears before v. This only exists for DAGs (directed
        acyclic graphs).

        Raises ``CycleError`` if the graph contains a cycle. The error
        includes the cycle path.

        For an empty graph, returns an empty list.
        """
        # We work on copies of the in-degree counts so we don't mutate the
        # actual graph. This is a "virtual" removal -- we just decrement
        # counters instead of actually removing nodes.

        in_degree: dict[object, int] = {
            node: len(preds) for node, preds in self._reverse.items()
        }

        # Start with all nodes that have zero in-degree (no dependencies).
        queue: deque[object] = deque()
        for node, degree in in_degree.items():
            if degree == 0:
                queue.append(node)

        result: list = []

        while queue:
            # Pick a node with zero in-degree. Sorting ensures deterministic
            # output when multiple nodes have zero in-degree.
            node = queue.popleft()
            result.append(node)

            # "Remove" this node by decrementing the in-degree of all its
            # successors. If any successor's in-degree drops to zero, it's
            # ready to be processed.
            for successor in sorted(self._forward[node]):
                in_degree[successor] -= 1
                if in_degree[successor] == 0:
                    queue.append(successor)

        # If we couldn't process all nodes, there's a cycle. Find it using
        # DFS so we can report the actual cycle path.
        if len(result) != len(self._forward):
            cycle = self._find_cycle()
            raise CycleError(
                f"Graph contains a cycle: {' -> '.join(str(n) for n in cycle)}",
                cycle=cycle,
            )

        return result

    # ------------------------------------------------------------------
    # Cycle Detection (DFS Three-Color Algorithm)
    # ------------------------------------------------------------------
    #
    # The three-color algorithm uses:
    # - WHITE (0): not yet visited
    # - GRAY  (1): currently being explored (on the recursion stack)
    # - BLACK (2): fully explored
    #
    # If we encounter a GRAY node during DFS, we've found a back edge,
    # which means there's a cycle.

    WHITE, GRAY, BLACK = 0, 1, 2

    def has_cycle(self) -> bool:
        """Return True if the graph contains at least one cycle.

        Uses DFS with three-color marking. This is O(V + E).
        """
        color: dict[object, int] = {node: self.WHITE for node in self._forward}

        def dfs(node: object) -> bool:
            """Return True if a cycle is reachable from this node."""
            color[node] = self.GRAY

            for successor in self._forward[node]:
                if color[successor] == self.GRAY:
                    # Back edge found -- cycle!
                    return True
                if color[successor] == self.WHITE and dfs(successor):
                    return True

            color[node] = self.BLACK
            return False

        # We need to start DFS from every unvisited node because the graph
        # might not be connected.
        for node in self._forward:
            if color[node] == self.WHITE:
                if dfs(node):
                    return True

        return False

    def _find_cycle(self) -> list:
        """Find and return a cycle path using DFS.

        This is a private helper used by ``topological_sort`` to provide
        a useful error message. It returns a list like [A, B, C, A] where
        A -> B -> C -> A is the cycle.
        """
        color: dict[object, int] = {node: self.WHITE for node in self._forward}
        parent: dict[object, object | None] = {
            node: None for node in self._forward
        }

        def dfs(node: object) -> list | None:
            color[node] = self.GRAY

            for successor in sorted(self._forward[node]):
                if color[successor] == self.GRAY:
                    # Found the cycle! Reconstruct the path.
                    cycle = [successor, node]
                    current = node
                    while current != successor:
                        current = parent[current]
                        if current is None:
                            break
                        cycle.append(current)
                    cycle.reverse()
                    cycle.append(successor)  # Close the cycle
                    return cycle
                if color[successor] == self.WHITE:
                    parent[successor] = node
                    result = dfs(successor)
                    if result is not None:
                        return result

            color[node] = self.BLACK
            return None

        for node in sorted(self._forward.keys()):
            if color[node] == self.WHITE:
                result = dfs(node)
                if result is not None:
                    return result

        return []  # Should never reach here if called when a cycle exists

    # ------------------------------------------------------------------
    # Transitive Closure
    # ------------------------------------------------------------------
    #
    # The transitive closure of a node is the set of all nodes reachable
    # from it by following edges forward. We use BFS because it's simple
    # and doesn't risk stack overflow on deep graphs.

    def transitive_closure(self, node: object) -> set:
        """Return all nodes reachable downstream from ``node``.

        This follows edges in the forward direction. The starting node is
        NOT included in the result (only the nodes it can reach).

        Raises ``NodeNotFoundError`` if the node doesn't exist.
        """
        if node not in self._forward:
            raise NodeNotFoundError(node)

        visited: set = set()
        queue: deque[object] = deque(self._forward[node])
        visited.update(self._forward[node])

        while queue:
            current = queue.popleft()
            for successor in self._forward[current]:
                if successor not in visited:
                    visited.add(successor)
                    queue.append(successor)

        return visited

    # ------------------------------------------------------------------
    # Transitive Dependents (Reverse Transitive Closure)
    # ------------------------------------------------------------------
    #
    # This is the mirror of transitive_closure: instead of asking "what
    # does this node depend on?", we ask "what depends on this node?"
    # We just walk the reverse adjacency dict instead of the forward one.

    def transitive_dependents(self, node: object) -> set:
        """Return all nodes that transitively depend on ``node``.

        This follows edges in the REVERSE direction -- it finds everything
        upstream that would be affected if ``node`` changed.

        The starting node is NOT included in the result.

        Raises ``NodeNotFoundError`` if the node doesn't exist.
        """
        if node not in self._reverse:
            raise NodeNotFoundError(node)

        visited: set = set()
        queue: deque[object] = deque(self._reverse[node])
        visited.update(self._reverse[node])

        while queue:
            current = queue.popleft()
            for predecessor in self._reverse[current]:
                if predecessor not in visited:
                    visited.add(predecessor)
                    queue.append(predecessor)

        return visited

    # ------------------------------------------------------------------
    # Independent Groups (Parallel Execution Levels)
    # ------------------------------------------------------------------
    #
    # This is a modified version of Kahn's algorithm. Instead of pulling
    # nodes off the queue one at a time, we pull ALL zero-in-degree nodes
    # at once -- they form one "level" of independent tasks that can run
    # in parallel.
    #
    # For a linear chain A -> B -> C, we get [[A], [B], [C]] (fully serial).
    # For a diamond A -> B, A -> C, B -> D, C -> D, we get
    # [[A], [B, C], [D]] -- B and C can run in parallel.

    def independent_groups(self) -> list[list]:
        """Partition nodes into levels by topological depth.

        Each level contains nodes that have no dependencies on each other
        and whose dependencies have all been satisfied by earlier levels.
        Nodes within a level can be executed in parallel.

        Raises ``CycleError`` if the graph contains a cycle.

        Returns an empty list for an empty graph.
        """
        in_degree: dict[object, int] = {
            node: len(preds) for node, preds in self._reverse.items()
        }

        # Collect the initial set of zero-in-degree nodes.
        current_level: list = sorted(
            node for node, degree in in_degree.items() if degree == 0
        )

        groups: list[list] = []
        processed = 0

        while current_level:
            groups.append(current_level)
            processed += len(current_level)

            next_level_set: set = set()
            for node in current_level:
                for successor in self._forward[node]:
                    in_degree[successor] -= 1
                    if in_degree[successor] == 0:
                        next_level_set.add(successor)

            current_level = sorted(next_level_set)

        if processed != len(self._forward):
            cycle = self._find_cycle()
            raise CycleError(
                f"Graph contains a cycle: {' -> '.join(str(n) for n in cycle)}",
                cycle=cycle,
            )

        return groups

    # ------------------------------------------------------------------
    # Affected Nodes
    # ------------------------------------------------------------------
    #
    # Given a set of "changed" nodes, compute everything that is affected:
    # the changed nodes themselves plus all their transitive dependents.
    # This is useful in build systems to figure out what needs to be rebuilt.

    def affected_nodes(self, changed: set) -> set:
        """Return the changed nodes plus all their transitive dependents.

        For each node in ``changed``, we find everything that depends on it
        (directly or transitively) and include it in the result. The changed
        nodes themselves are always included.

        Nodes in ``changed`` that don't exist in the graph are silently
        ignored (they might have been removed).
        """
        result: set = set()

        for node in changed:
            if node in self._forward:
                result.add(node)
                result.update(self.transitive_dependents(node))

        return result
