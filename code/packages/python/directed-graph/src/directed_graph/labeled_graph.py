"""
labeled_graph.py -- Directed Graph with Labeled Edges
======================================================

A labeled directed graph extends the basic directed graph by attaching **labels**
(arbitrary strings) to edges. This is useful when you need to distinguish
*different kinds* of relationships between the same pair of nodes.

Real-World Examples
-------------------

Consider a knowledge graph:

    Alice --[friend]--> Bob
    Alice --[coworker]--> Bob
    Alice --[friend]--> Carol

Alice and Bob have TWO relationships (friend and coworker), while Alice and
Carol have ONE. A basic directed graph can only say "Alice connects to Bob"
-- the labeled graph says *how*.

Other examples:
- **Build systems**: package A depends on package B for "compile" and "test"
- **State machines**: state X transitions to state Y on input "a" and "b"
- **RDF graphs**: subject-predicate-object triples (the predicate is the label)

Architecture
------------

Rather than reimplementing the graph from scratch, ``LabeledDirectedGraph``
wraps an inner ``DirectedGraph`` and adds a label dictionary on top::

    Inner graph: A -> B        (tracks connectivity and runs algorithms)
    Label dict:  (A, B) -> {"friend", "coworker"}   (tracks labels)

This composition approach means all the graph algorithms (topological sort,
cycle detection, etc.) work automatically via delegation. We only need to
add label-aware logic for edge operations.

The inner graph always has ``allow_self_loops=True`` because labeled graphs
commonly need self-referential edges (e.g., a state that loops back to itself
on certain inputs).

Label Storage
-------------

Labels are stored in a dictionary mapping ``(from_node, to_node)`` tuples
to sets of label strings::

    _labels = {
        ("Alice", "Bob"): {"friend", "coworker"},
        ("Alice", "Carol"): {"friend"},
    }

The inner graph tracks the structural edge (A -> B exists or not), while
``_labels`` tracks which labels are on that edge. This separation means:

- Adding a second label to an existing edge doesn't duplicate the structural edge
- Removing a label only removes the structural edge when no labels remain
- Graph algorithms see the simplified structure (just nodes and edges, no labels)
"""

from __future__ import annotations

from directed_graph.graph import DirectedGraph, EdgeNotFoundError, NodeNotFoundError


class LabeledDirectedGraph:
    """A directed graph where each edge carries one or more string labels.

    This class wraps a ``DirectedGraph`` (with self-loops enabled) and adds
    a label layer on top. Each edge between two nodes can have multiple labels,
    and you can filter queries by label.

    Example::

        lg = LabeledDirectedGraph()
        lg.add_edge("Alice", "Bob", "friend")
        lg.add_edge("Alice", "Bob", "coworker")
        lg.add_edge("Alice", "Carol", "friend")

        print(lg.labels("Alice", "Bob"))        # {"friend", "coworker"}
        print(lg.successors("Alice"))            # ["Bob", "Carol"]
        print(lg.successors("Alice", "coworker"))  # ["Bob"]
    """

    # ------------------------------------------------------------------
    # Initialization
    # ------------------------------------------------------------------
    # The inner graph handles all structural operations (node/edge storage,
    # algorithms). The _labels dict adds the label dimension.

    def __init__(self) -> None:
        self._graph: DirectedGraph = DirectedGraph(allow_self_loops=True)
        self._labels: dict[tuple[object, object], set[str]] = {}

    # ------------------------------------------------------------------
    # Node operations -- delegate to inner graph
    # ------------------------------------------------------------------
    # Nodes in a labeled graph behave exactly like nodes in a plain graph.
    # The only extra work is cleaning up labels when a node is removed.

    def add_node(self, node: object) -> None:
        """Add a node to the graph. No-op if it already exists."""
        self._graph.add_node(node)

    def remove_node(self, node: object) -> None:
        """Remove a node and all its edges (including labels).

        This cleans up both the inner graph AND the label dictionary.
        We must remove all label entries where this node appears as
        either the source or target of an edge.

        Raises ``NodeNotFoundError`` if the node doesn't exist.
        """
        if not self._graph.has_node(node):
            raise NodeNotFoundError(node)

        # Collect label keys to remove BEFORE modifying the dict.
        # We can't modify a dict while iterating it, so we collect first.
        keys_to_remove = [
            key for key in self._labels if key[0] == node or key[1] == node
        ]
        for key in keys_to_remove:
            del self._labels[key]

        # Now remove the node from the inner graph (which handles edge cleanup).
        self._graph.remove_node(node)

    def has_node(self, node: object) -> bool:
        """Return True if the node exists in the graph."""
        return self._graph.has_node(node)

    def nodes(self) -> list:
        """Return a list of all nodes."""
        return self._graph.nodes()

    def __len__(self) -> int:
        """Return the number of nodes."""
        return len(self._graph)

    def __contains__(self, node: object) -> bool:
        """Support ``node in graph`` syntax."""
        return self._graph.has_node(node)

    def __repr__(self) -> str:
        edge_count = sum(len(labels) for labels in self._labels.values())
        return (
            f"LabeledDirectedGraph(nodes={len(self)}, "
            f"labeled_edges={edge_count})"
        )

    # ------------------------------------------------------------------
    # Labeled edge operations
    # ------------------------------------------------------------------
    # These are the core of the labeled graph. Each edge is identified by
    # the triple (from_node, to_node, label).

    def add_edge(self, from_node: object, to_node: object, label: str) -> None:
        """Add a labeled edge from ``from_node`` to ``to_node``.

        Both nodes are auto-created if they don't exist. If the structural
        edge doesn't exist yet, it's added to the inner graph. The label is
        then added to the label set for this edge pair.

        Multiple calls with the same (from, to, label) are idempotent.
        Multiple calls with the same (from, to) but different labels add
        multiple labels to the same structural edge.

        Example::

            lg.add_edge("A", "B", "x")   # Creates edge A->B with label "x"
            lg.add_edge("A", "B", "y")   # Adds label "y" to existing A->B
        """
        # Add the structural edge if it doesn't exist yet.
        if not self._graph.has_edge(from_node, to_node):
            self._graph.add_edge(from_node, to_node)
        else:
            # Ensure both nodes exist (they should, since the edge exists).
            self._graph.add_node(from_node)
            self._graph.add_node(to_node)

        # Add the label.
        key = (from_node, to_node)
        if key not in self._labels:
            self._labels[key] = set()
        self._labels[key].add(label)

    def remove_edge(
        self, from_node: object, to_node: object, label: str
    ) -> None:
        """Remove a specific labeled edge.

        This removes the given label from the edge pair (from_node, to_node).
        If this was the LAST label on that edge, the structural edge is also
        removed from the inner graph.

        Raises ``EdgeNotFoundError`` if the edge or label doesn't exist.

        Example::

            lg.add_edge("A", "B", "x")
            lg.add_edge("A", "B", "y")
            lg.remove_edge("A", "B", "x")   # "y" still exists
            lg.remove_edge("A", "B", "y")   # structural edge removed too
        """
        key = (from_node, to_node)
        if key not in self._labels or label not in self._labels[key]:
            raise EdgeNotFoundError(from_node, to_node)

        self._labels[key].discard(label)

        # If no labels remain, remove the structural edge too.
        if not self._labels[key]:
            del self._labels[key]
            self._graph.remove_edge(from_node, to_node)

    def has_edge(
        self,
        from_node: object,
        to_node: object,
        label: str | None = None,
    ) -> bool:
        """Check if an edge exists, optionally with a specific label.

        - ``has_edge("A", "B")``: True if ANY label exists between A and B
        - ``has_edge("A", "B", "x")``: True only if label "x" exists on A->B

        This two-mode API lets you ask both "are these nodes connected?" and
        "are they connected by THIS specific relationship?".
        """
        key = (from_node, to_node)
        if key not in self._labels:
            return False
        if label is None:
            return len(self._labels[key]) > 0
        return label in self._labels[key]

    def edges(self) -> list[tuple[object, object, str]]:
        """Return all edges as (from_node, to_node, label) triples.

        Each label gets its own triple. So if A->B has labels "x" and "y",
        this returns both ("A", "B", "x") and ("A", "B", "y").
        """
        result: list[tuple[object, object, str]] = []
        for (from_node, to_node), labels in self._labels.items():
            for label in sorted(labels):
                result.append((from_node, to_node, label))
        return result

    def labels(self, from_node: object, to_node: object) -> set[str]:
        """Return the set of labels on the edge from_node -> to_node.

        Returns an empty set if no edge exists (rather than raising).
        This is a query method, not a mutation, so being lenient is appropriate.
        """
        key = (from_node, to_node)
        if key not in self._labels:
            return set()
        return set(self._labels[key])

    # ------------------------------------------------------------------
    # Neighbor queries with optional label filtering
    # ------------------------------------------------------------------
    # These methods delegate to the inner graph for the basic query, then
    # optionally filter by label. The label filter is useful when you want
    # to traverse only certain kinds of edges.

    def successors(
        self, node: object, label: str | None = None
    ) -> list:
        """Return successors of a node, optionally filtered by label.

        - ``successors("A")``: all nodes that A has an edge TO
        - ``successors("A", "friend")``: only nodes connected by "friend" label

        Raises ``NodeNotFoundError`` if the node doesn't exist.
        """
        if not self._graph.has_node(node):
            raise NodeNotFoundError(node)

        if label is None:
            return self._graph.successors(node)

        # Filter: only include successors where the specific label exists.
        result = []
        for successor in self._graph.successors(node):
            key = (node, successor)
            if key in self._labels and label in self._labels[key]:
                result.append(successor)
        return result

    def predecessors(
        self, node: object, label: str | None = None
    ) -> list:
        """Return predecessors of a node, optionally filtered by label.

        - ``predecessors("B")``: all nodes that have an edge TO B
        - ``predecessors("B", "friend")``: only nodes connected by "friend" label

        Raises ``NodeNotFoundError`` if the node doesn't exist.
        """
        if not self._graph.has_node(node):
            raise NodeNotFoundError(node)

        if label is None:
            return self._graph.predecessors(node)

        # Filter: only include predecessors where the specific label exists.
        result = []
        for predecessor in self._graph.predecessors(node):
            key = (predecessor, node)
            if key in self._labels and label in self._labels[key]:
                result.append(predecessor)
        return result

    # ------------------------------------------------------------------
    # Algorithm delegation
    # ------------------------------------------------------------------
    # All graph algorithms are delegated to the inner DirectedGraph. The
    # labeled graph adds no new algorithmic behavior -- labels are metadata
    # on edges, not structural properties that affect traversal.

    def topological_sort(self) -> list:
        """Return a topological ordering of all nodes.

        Delegates to the inner graph's Kahn's algorithm implementation.
        Raises ``CycleError`` if the graph contains a cycle.
        """
        return self._graph.topological_sort()

    def has_cycle(self) -> bool:
        """Return True if the graph contains a cycle.

        Delegates to the inner graph's DFS three-color algorithm.
        """
        return self._graph.has_cycle()

    def transitive_closure(self, node: object) -> set:
        """Return all nodes reachable downstream from ``node``.

        Delegates to the inner graph's BFS implementation.
        """
        return self._graph.transitive_closure(node)

    def transitive_dependents(self, node: object) -> set:
        """Return all nodes that transitively depend on ``node``.

        Delegates to the inner graph's reverse BFS implementation.
        """
        return self._graph.transitive_dependents(node)

    # ------------------------------------------------------------------
    # Access to the inner graph
    # ------------------------------------------------------------------

    @property
    def graph(self) -> DirectedGraph:
        """Access the inner DirectedGraph for advanced operations.

        This is useful when you need algorithms not exposed by the labeled
        graph, like ``independent_groups()`` or ``affected_nodes()``.
        """
        return self._graph
