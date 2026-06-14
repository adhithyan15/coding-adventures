"""
directed_graph.py — DirectedGraph: A Directed Network Extending Graph[T]
=========================================================================

A directed graph G = (V, E) where every edge has a direction:

    add_edge(u, v)   means  u → v   (u points TO v)

This is in contrast to the undirected Graph[T] base class where edges
have no direction and {u, v} == {v, u}.

Directed graphs are everywhere in computing:

  - Dependency graphs:   "A depends on B"  means  B → A
  - Build systems:       "compile before link"  means  compile → link
  - State machines:      "pressing button X goes to state Y"
  - Web graphs:          "page A links to page B"

Inheritance Design
------------------
``DirectedGraph[T]`` inherits from ``Graph[T]`` (DT00).  The base class
provides all node operations and stores forward edges in ``self._adj``.
We extend it with:

  ``self._reverse``  — a parallel adjacency dict for reverse edges.
                        _reverse[v][u] = weight   means  u → v exists.

Why store _reverse at all?  Because several critical algorithms need to
walk edges backwards:

  - ``predecessors(v)``        — who points TO v?
  - ``transitive_dependents``  — what depends on this node?
  - ``in_degree(v)``           — how many edges arrive at v?
  - Kahn's topological sort    — needs in-degree counts

With _reverse these are all O(degree), without it they'd be O(V + E).

Key Invariant
-------------
Every node that exists in the graph has an entry in BOTH ``self._adj``
(inherited from Graph) AND ``self._reverse``.  These two dicts are always
kept in sync by the overridden add_edge, remove_edge, add_node, and
remove_node methods.

Overriding ``neighbors()``
--------------------------
The base Graph.neighbors(u) returns ALL neighbours (both directions for
undirected graphs).  For a directed graph, algorithms like bfs and dfs
should only follow forward (outgoing) edges.  So we override neighbors()
to return only successors:

    neighbors(u)  ≡  successors(u)  ≡  frozenset(_adj[u])

This means bfs/dfs imported from the graph package work correctly on
DirectedGraph without any modification — they naturally traverse only
forward edges.

Self-loops
----------
By default self-loops are forbidden.  Pass ``allow_self_loops=True`` to
allow ``add_edge(u, u)``.  A self-loop creates an obvious cycle, so
``has_cycle()`` will return True and ``topological_sort()`` will raise
ValueError.

LabeledDirectedGraph
--------------------
A separate class (by composition, not inheritance) that attaches a string
label to each edge.  Useful for state machines (transition labels) and
annotated dependency graphs (e.g., "requires", "imports", "extends").
"""

from __future__ import annotations

from typing import Generic, TypeVar

from graph import Graph, GraphRepr, PropertyBag, PropertyValue

# ---------------------------------------------------------------------------
# Type variable
# ---------------------------------------------------------------------------

T = TypeVar("T")  # Node type — must be hashable


# ---------------------------------------------------------------------------
# DirectedGraph
# ---------------------------------------------------------------------------


class DirectedGraph(Graph[T]):
    """A directed weighted graph extending Graph[T] from DT00.

    Edges are directed: ``add_edge(u, v)`` creates the edge u → v but NOT
    v → u.  The graph stores a reverse adjacency dict ``_reverse`` in addition
    to the inherited forward adjacency dict ``_adj``.

    Inherits from Graph[T] (DT00 — undirected graph base).  Overrides:

    - ``add_node``    — also initialises ``_reverse[node]``
    - ``remove_node`` — also cleans up ``_reverse`` entries
    - ``add_edge``    — only stores u→v (not v→u); updates ``_reverse``
    - ``remove_edge`` — removes from both ``_adj`` and ``_reverse``
    - ``neighbors``   — returns successors only (forward edges)

    New methods:

    - ``successors(node)``   — nodes this node points TO
    - ``predecessors(node)`` — nodes that point TO this node
    - ``out_degree(node)``   — number of outgoing edges
    - ``in_degree(node)``    — number of incoming edges

    Example — a simple dependency chain::

        g = DirectedGraph()
        g.add_edge("parse", "compile")    # parse → compile
        g.add_edge("compile", "link")     # compile → link
        g.add_edge("compile", "typecheck")

        g.successors("compile")   # frozenset({"link", "typecheck"})
        g.predecessors("compile") # frozenset({"parse"})
        g.out_degree("compile")   # 2
        g.in_degree("compile")    # 1

    ASCII diagram::

        parse → compile → link
                       ↘ typecheck

    Compatibility with Graph algorithms::

        from graph import bfs, dfs
        bfs(g, "parse")   # ["parse", "compile", "link", "typecheck"]
        # bfs/dfs call neighbors() which returns successors — works correctly!
    """

    def __init__(self, allow_self_loops: bool = False) -> None:
        # Always use ADJACENCY_LIST — we can't use ADJACENCY_MATRIX because
        # the base class's matrix is symmetric (undirected), but our edges
        # are directed.  Using adjacency list lets us store u→v without v→u.
        super().__init__(repr=GraphRepr.ADJACENCY_LIST)

        # Reverse adjacency: _reverse[v][u] = weight means edge u → v exists.
        # This mirrors _adj but in the reverse direction.
        # Invariant: every node in _adj has a corresponding key in _reverse.
        self._reverse: dict[T, dict[T, float]] = {}

        self._allow_self_loops: bool = allow_self_loops

    # ------------------------------------------------------------------
    # Node operations (overrides)
    # ------------------------------------------------------------------

    def add_node(self, node: T, properties: PropertyBag | None = None) -> None:
        """Add a node to the graph.  No-op if the node already exists.

        Overrides Graph.add_node to also initialise the reverse adjacency
        entry for this node.

        Example::

            g = DirectedGraph()
            g.add_node("A", {"kind": "input"})
            g.has_node("A")  # True
            g.successors("A")    # frozenset()
            g.predecessors("A")  # frozenset()
        """
        existed = node in self._adj
        super().add_node(node, properties)

        if existed:
            return

        if node not in self._adj:
            return

        # Mirror the entry in _reverse.
        self._reverse[node] = {}

    def remove_node(self, node: T) -> None:
        """Remove a node and all its incident edges (both in and out).

        Overrides Graph.remove_node to also clean up _reverse entries.

        Raises KeyError if the node does not exist (base class behaviour).

        Example::

            g = DirectedGraph()
            g.add_edge("A", "B")
            g.add_edge("C", "B")
            g.remove_node("B")
            g.has_node("B")  # False
            g.has_edge("A", "B")  # False
            g.has_edge("C", "B")  # False
        """
        if node not in self._adj:
            raise KeyError(node)

        # Clean up _reverse: for each successor v of `node`,
        # remove `node` from _reverse[v] (i.e., node no longer points to v).
        for successor in list(self._adj[node]):
            del self._reverse[successor][node]
            self._edge_properties.pop((node, successor), None)

        # Clean up _adj of predecessors: for each predecessor u of `node`,
        # remove the forward edge u → node.
        for predecessor in list(self._reverse[node]):
            del self._adj[predecessor][node]
            self._edge_properties.pop((predecessor, node), None)

        # Remove the node's own entries.
        del self._adj[node]
        del self._reverse[node]
        del self._node_properties[node]

    # ------------------------------------------------------------------
    # Edge operations (overrides)
    # ------------------------------------------------------------------

    def add_edge(
        self,
        u: T,
        v: T,
        weight: float = 1.0,
        properties: PropertyBag | None = None,
    ) -> None:
        """Add a directed edge u → v with the given weight.

        Overrides Graph.add_edge to store a DIRECTED edge only (u→v, not v→u).
        Also updates ``_reverse[v][u]`` so reverse lookups are O(1).

        Both nodes are added automatically if they do not exist.
        If the edge already exists its weight is updated (idempotent).

        Raises ValueError if u == v and allow_self_loops is False.

        Example::

            g = DirectedGraph()
            g.add_edge("A", "B", weight=2.5)
            g.has_edge("A", "B")  # True
            g.has_edge("B", "A")  # False  ← directed!
            g._adj["A"]["B"]      # 2.5
            g._reverse["B"]["A"]  # 2.5
        """
        if u == v and not self._allow_self_loops:
            raise ValueError(
                f"Self-loops are not allowed: {u!r} -> {v!r}. "
                f"Pass allow_self_loops=True to permit them."
            )
        self._validate_weight(weight)

        # Ensure both nodes exist (with their _reverse entries).
        self.add_node(u)
        self.add_node(v)

        # Store ONLY the forward edge u→v.
        # The base class add_edge would store both u→v AND v→u (undirected).
        # We bypass it and write directly to the internal dict.
        self._adj[u][v] = weight
        # Update reverse dict.
        self._reverse[v][u] = weight

        merged = dict(properties or {})
        merged["weight"] = weight
        self._edge_properties.setdefault((u, v), {}).update(merged)

    def remove_edge(self, u: T, v: T) -> None:
        """Remove the directed edge u → v.

        Overrides Graph.remove_edge to only remove the forward edge and its
        corresponding reverse entry.  Does NOT remove the reverse direction
        (v → u stays, if it exists as a separate edge).

        Raises KeyError if either node or the edge does not exist.

        Example::

            g = DirectedGraph()
            g.add_edge("A", "B")
            g.add_edge("B", "A")   # two separate edges
            g.remove_edge("A", "B")
            g.has_edge("A", "B")  # False
            g.has_edge("B", "A")  # True  ← other direction still exists
        """
        if u not in self._adj or v not in self._adj[u]:
            raise KeyError((u, v))

        del self._adj[u][v]
        del self._reverse[v][u]
        self._edge_properties.pop((u, v), None)

    # ------------------------------------------------------------------
    # Neighbor queries (override + new methods)
    # ------------------------------------------------------------------

    def neighbors(self, node: T) -> frozenset[T]:
        """Return the forward neighbors (successors) of node.

        Overrides Graph.neighbors to return ONLY outgoing edges (successors),
        not incoming edges.  This makes bfs/dfs from the graph package work
        correctly: they traverse forward edges only.

        Raises KeyError if the node does not exist.

        Example::

            g = DirectedGraph()
            g.add_edge("A", "B")
            g.add_edge("C", "A")
            g.neighbors("A")  # frozenset({"B"})  — only forward, not C
        """
        if node not in self._adj:
            raise KeyError(node)
        return frozenset(self._adj[node])

    def successors(self, node: T) -> frozenset[T]:
        """Return the nodes that ``node`` points TO.

        These are the direct forward neighbors of ``node`` — the nodes you
        reach by following the outgoing edges from ``node``.

        In a dependency graph where A → B means "A depends on B":
        successors(A) = the things A directly depends on.

        Raises KeyError if the node does not exist.

        Example::

            #  A → B → D
            #  A → C
            g.successors("A")  # frozenset({"B", "C"})
            g.successors("B")  # frozenset({"D"})
            g.successors("D")  # frozenset()
        """
        if node not in self._adj:
            raise KeyError(node)
        return frozenset(self._adj[node])

    def predecessors(self, node: T) -> frozenset[T]:
        """Return the nodes that point TO ``node``.

        These are the nodes with an outgoing edge arriving at ``node``.

        In a dependency graph where A → B means "A depends on B":
        predecessors(B) = the things that directly depend on B.

        Raises KeyError if the node does not exist.

        Example::

            #  A → C
            #  B → C
            g.predecessors("C")  # frozenset({"A", "B"})
            g.predecessors("A")  # frozenset()
        """
        if node not in self._reverse:
            raise KeyError(node)
        return frozenset(self._reverse[node])

    def out_degree(self, node: T) -> int:
        """Return the number of outgoing edges from ``node``.

        Raises KeyError if the node does not exist.

        Example::

            g.add_edge("A", "B")
            g.add_edge("A", "C")
            g.out_degree("A")  # 2
            g.out_degree("B")  # 0
        """
        return len(self.successors(node))

    def in_degree(self, node: T) -> int:
        """Return the number of incoming edges to ``node``.

        Raises KeyError if the node does not exist.

        Example::

            g.add_edge("A", "C")
            g.add_edge("B", "C")
            g.in_degree("C")  # 2
            g.in_degree("A")  # 0
        """
        return len(self.predecessors(node))

    # ------------------------------------------------------------------
    # Edge queries (override for directed semantics)
    # ------------------------------------------------------------------

    def has_edge(self, u: T, v: T) -> bool:
        """Return True if the directed edge u → v exists.

        Note: has_edge(u, v) and has_edge(v, u) are independent —
        they check two different directed edges.

        Example::

            g.add_edge("A", "B")
            g.has_edge("A", "B")  # True
            g.has_edge("B", "A")  # False
        """
        return u in self._adj and v in self._adj[u]

    def edges(self) -> frozenset[tuple[T, T, float]]:
        """Return all directed edges as a frozenset of (u, v, weight) triples.

        Each directed edge appears exactly once.  Unlike the undirected base
        class, we do NOT deduplicate: (A, B, 1.0) and (B, A, 1.0) are two
        separate directed edges and both appear.

        Example::

            g.add_edge("A", "B", 1.0)
            g.add_edge("B", "A", 2.0)   # a separate reverse edge
            g.edges()
            # frozenset({("A", "B", 1.0), ("B", "A", 2.0)})
        """
        result: set[tuple[T, T, float]] = set()
        for u, neighbours in self._adj.items():
            for v, w in neighbours.items():
                result.add((u, v, w))
        return frozenset(result)

    # ------------------------------------------------------------------
    # Property bags (override for directed edge identity)
    # ------------------------------------------------------------------

    def edge_properties(self, u: T, v: T) -> PropertyBag:
        """Return a copy of properties attached to directed edge u → v."""
        if not self.has_edge(u, v):
            raise KeyError((u, v))
        properties = dict(self._edge_properties.get((u, v), {}))
        properties["weight"] = self.edge_weight(u, v)
        return properties

    def set_edge_property(
        self,
        u: T,
        v: T,
        key: str,
        value: PropertyValue,
    ) -> None:
        """Set one property on directed edge u → v.

        Setting ``weight`` also updates both the forward and reverse adjacency
        maps while preserving directed semantics.
        """
        if not self.has_edge(u, v):
            raise KeyError((u, v))
        if key == "weight":
            if not isinstance(value, (int, float)) or isinstance(value, bool):
                raise ValueError("edge property 'weight' must be numeric")
            self._set_directed_edge_weight(u, v, float(value))
        self._edge_properties.setdefault((u, v), {})[key] = value

    def remove_edge_property(self, u: T, v: T, key: str) -> None:
        """Remove one property from directed edge u → v if present."""
        if not self.has_edge(u, v):
            raise KeyError((u, v))
        if key == "weight":
            self._set_directed_edge_weight(u, v, 1.0)
            self._edge_properties.setdefault((u, v), {})["weight"] = 1.0
            return
        self._edge_properties.setdefault((u, v), {}).pop(key, None)

    def _set_directed_edge_weight(self, u: T, v: T, weight: float) -> None:
        self._validate_weight(weight)
        self._adj[u][v] = weight
        self._reverse[v][u] = weight

    # ------------------------------------------------------------------
    # Dunder methods
    # ------------------------------------------------------------------

    def __repr__(self) -> str:
        loops = ", allow_self_loops=True" if self._allow_self_loops else ""
        return (
            f"DirectedGraph(nodes={len(self)}, "
            f"edges={len(self.edges())}{loops})"
        )


# ---------------------------------------------------------------------------
# LabeledDirectedGraph
# ---------------------------------------------------------------------------


class LabeledDirectedGraph(Generic[T]):
    """A directed graph where every edge carries a string label.

    Implemented by COMPOSITION over DirectedGraph (not inheritance), because
    we need to enrich the edge representation beyond what the base class stores.

    Use cases:

    - State machines:    label = transition name / event
    - Grammars:          label = production rule name
    - Annotated deps:    label = relationship type ("imports", "extends", …)

    Internally we store labels in a separate dict::

        _labels[(u, v)] = label_string

    while weights are delegated to the underlying DirectedGraph.

    Example — a state machine::

        sm = LabeledDirectedGraph()
        sm.add_edge("idle",    "running", label="start",  weight=1.0)
        sm.add_edge("running", "idle",    label="stop",   weight=1.0)
        sm.add_edge("running", "paused",  label="pause",  weight=1.0)
        sm.add_edge("paused",  "running", label="resume", weight=1.0)

        sm.edge_label("running", "paused")   # "pause"
        sm.successors("running")             # frozenset({"idle", "paused"})

    ASCII diagram::

        idle ──start──▶ running ──pause──▶ paused
              ◀──stop──          ◀──resume──
    """

    def __init__(self, allow_self_loops: bool = False) -> None:
        self._graph: DirectedGraph[T] = DirectedGraph(
            allow_self_loops=allow_self_loops
        )
        # Maps (u, v) edge tuples to their string labels.
        self._labels: dict[tuple[T, T], str] = {}

    # ------------------------------------------------------------------
    # Node operations
    # ------------------------------------------------------------------

    def add_node(self, node: T) -> None:
        """Add a node to the graph.  No-op if the node already exists."""
        self._graph.add_node(node)

    def remove_node(self, node: T) -> None:
        """Remove a node and all its incident labeled edges.

        Raises KeyError if the node does not exist.
        """
        if not self._graph.has_node(node):
            raise KeyError(node)

        # Clean up all labels for edges incident to this node BEFORE removing.
        edges_to_remove = [
            (u, v) for (u, v) in self._labels if u == node or v == node
        ]
        for edge in edges_to_remove:
            del self._labels[edge]

        self._graph.remove_node(node)

    def has_node(self, node: T) -> bool:
        """Return True if the node exists in the graph."""
        return self._graph.has_node(node)

    def nodes(self) -> frozenset[T]:
        """Return all nodes as an immutable frozenset."""
        return self._graph.nodes()

    # ------------------------------------------------------------------
    # Edge operations
    # ------------------------------------------------------------------

    def add_edge(self, u: T, v: T, label: str, weight: float = 1.0) -> None:
        """Add a directed labeled edge u → v.

        The label is a mandatory string annotation (e.g., "imports", "start").
        If the edge already exists its label and weight are updated.

        Raises ValueError if u == v and self-loops are not allowed.

        Example::

            g.add_edge("A", "B", label="depends_on", weight=1.0)
            g.edge_label("A", "B")  # "depends_on"
        """
        self._graph.add_edge(u, v, weight)
        self._labels[(u, v)] = label

    def remove_edge(self, u: T, v: T) -> None:
        """Remove the directed edge u → v.

        Raises KeyError if the edge does not exist.
        """
        if not self._graph.has_edge(u, v):
            raise KeyError((u, v))
        self._graph.remove_edge(u, v)
        del self._labels[(u, v)]

    def has_edge(self, u: T, v: T) -> bool:
        """Return True if the directed edge u → v exists."""
        return self._graph.has_edge(u, v)

    def edge_label(self, u: T, v: T) -> str:
        """Return the label of the directed edge u → v.

        Raises KeyError if the edge does not exist.

        Example::

            g.add_edge("A", "B", label="imports")
            g.edge_label("A", "B")  # "imports"
            g.edge_label("B", "A")  # KeyError — edge doesn't exist
        """
        if (u, v) not in self._labels:
            raise KeyError((u, v))
        return self._labels[(u, v)]

    def edges_labeled(self) -> frozenset[tuple[T, T, str, float]]:
        """Return all edges as (u, v, label, weight) tuples.

        Example::

            g.add_edge("A", "B", "imports", 1.0)
            g.add_edge("B", "C", "extends", 2.0)
            g.edges_labeled()
            # frozenset({("A", "B", "imports", 1.0), ("B", "C", "extends", 2.0)})
        """
        result: set[tuple[T, T, str, float]] = set()
        for (u, v), label in self._labels.items():
            weight = self._graph._adj[u][v]
            result.add((u, v, label, weight))
        return frozenset(result)

    def successors(self, node: T) -> frozenset[T]:
        """Return the nodes that ``node`` points TO.

        Raises KeyError if the node does not exist.
        """
        return self._graph.successors(node)

    def predecessors(self, node: T) -> frozenset[T]:
        """Return the nodes that point TO ``node``.

        Raises KeyError if the node does not exist.
        """
        return self._graph.predecessors(node)

    def __repr__(self) -> str:
        return (
            f"LabeledDirectedGraph(nodes={len(self._graph)}, "
            f"edges={len(self._labels)})"
        )
