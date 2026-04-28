"""
graph.py — Graph: An Undirected Network of Nodes and Edges
===========================================================

A graph G = (V, E) is a pair of sets:

  V  — vertices (nodes): anything hashable (strings, ints, objects)
  E  — edges: unordered pairs {u, v} — no direction, {u,v} == {v,u}

Every more specialised data structure in the DT series (directed graph, tree,
binary tree, heap, trie, …) is a graph with additional structural constraints.

Two Representations
-------------------
We support two internal representations, selectable at construction time:

  ADJACENCY_LIST (default):
    A dict mapping each node to a dict of its neighbours with edge weights.

        adj[u][v] = weight    (and adj[v][u] = weight for undirected)

    Space: O(V + E)  — only stores existing edges.
    Edge lookup: O(degree(u))  — scan neighbour dict.
    Best for SPARSE graphs (most real-world graphs).

  ADJACENCY_MATRIX:
    A V×V float matrix where matrix[i][j] = weight (0.0 means no edge).
    Nodes are mapped to integer indices for row/column addressing.

        matrix[idx[u]][idx[v]] = weight  (symmetric for undirected)

    Space: O(V²)  — allocates a slot for every possible edge.
    Edge lookup: O(1)  — single array read.
    Best for DENSE graphs or when O(1) edge lookup is critical.

Both representations expose the same public API.  Every algorithm in
algorithms.py works unchanged on either.

Undirected Edge Symmetry
-------------------------
Because edges have no direction, every operation maintains symmetry:

    add_edge(u, v, w)    stores   adj[u][v] = w  AND  adj[v][u] = w
    remove_edge(u, v)    removes  adj[u][v]       AND  adj[v][u]
    edges()              returns each edge ONCE as (min(u,v), max(u,v), w)
                         so the frozenset contains no duplicates.

This is the key invariant that makes Graph undirected.
"""

from __future__ import annotations

from enum import Enum
from typing import Generic, TypeAlias, TypeVar

T = TypeVar("T")  # Node type — must be hashable
PropertyValue: TypeAlias = str | int | float | bool | None
PropertyBag: TypeAlias = dict[str, PropertyValue]


def _edge_key(u: T, v: T) -> tuple[T, T]:
    return (u, v) if repr(u) <= repr(v) else (v, u)


# ---------------------------------------------------------------------------
# GraphRepr — choose internal storage at construction time
# ---------------------------------------------------------------------------


class GraphRepr(Enum):
    """Which internal data structure backs the Graph.

    ADJACENCY_LIST is the default and best for sparse graphs.
    ADJACENCY_MATRIX gives O(1) edge lookup at the cost of O(V²) space.
    """

    ADJACENCY_LIST = "adjacency_list"
    ADJACENCY_MATRIX = "adjacency_matrix"


# ---------------------------------------------------------------------------
# Graph
# ---------------------------------------------------------------------------


class Graph(Generic[T]):
    """Undirected weighted graph.

    Nodes can be any hashable type: strings, integers, tuples, etc.
    Edges are unordered pairs (u, v) with an optional float weight (default 1.0).

    Choose the internal representation at construction::

        g = Graph()                                   # adjacency list (default)
        g = Graph(repr=GraphRepr.ADJACENCY_MATRIX)    # adjacency matrix

    Both forms expose the identical public API.
    """

    def __init__(self, repr: GraphRepr = GraphRepr.ADJACENCY_LIST) -> None:
        self._repr = repr
        self._graph_properties: PropertyBag = {}
        self._node_properties: dict[T, PropertyBag] = {}
        self._edge_properties: dict[tuple[T, T], PropertyBag] = {}

        if repr is GraphRepr.ADJACENCY_LIST:
            # adj[u][v] = weight for every edge {u, v}.
            # Both directions are stored: adj[u][v] and adj[v][u].
            # Nodes without any edges are tracked in _isolated.
            self._adj: dict[T, dict[T, float]] = {}
        else:
            # Ordered list of nodes for matrix index mapping.
            self._node_list: list[T] = []
            # node → row/col index in _matrix.
            self._node_idx: dict[T, int] = {}
            # V×V matrix; 0.0 means no edge.
            self._matrix: list[list[float]] = []

    # ------------------------------------------------------------------
    # Node operations
    # ------------------------------------------------------------------

    def add_node(self, node: T, properties: PropertyBag | None = None) -> None:
        """Add a node to the graph.  No-op if the node already exists."""
        if self._repr is GraphRepr.ADJACENCY_LIST:
            if node not in self._adj:
                self._adj[node] = {}
                self._node_properties[node] = {}
        else:
            if node not in self._node_idx:
                idx = len(self._node_list)
                self._node_list.append(node)
                self._node_idx[node] = idx
                self._node_properties[node] = {}
                # Add a new row and column of zeros.
                for row in self._matrix:
                    row.append(0.0)
                self._matrix.append([0.0] * (idx + 1))
        if properties is not None:
            self._node_properties[node].update(properties)

    def remove_node(self, node: T) -> None:
        """Remove a node and all edges incident to it.

        Raises KeyError if the node does not exist.
        """
        if self._repr is GraphRepr.ADJACENCY_LIST:
            if node not in self._adj:
                raise KeyError(node)
            # Remove all edges that touch this node.
            for neighbour in list(self._adj[node]):
                del self._adj[neighbour][node]
                self._edge_properties.pop(_edge_key(node, neighbour), None)
            del self._adj[node]
            del self._node_properties[node]
        else:
            if node not in self._node_idx:
                raise KeyError(node)
            for other in list(self._node_list):
                self._edge_properties.pop(_edge_key(node, other), None)
            idx = self._node_idx.pop(node)
            self._node_list.pop(idx)
            del self._node_properties[node]
            # Update indices for nodes that shifted down.
            for n in self._node_list[idx:]:
                self._node_idx[n] -= 1
            # Remove the row.
            self._matrix.pop(idx)
            # Remove the column from every remaining row.
            for row in self._matrix:
                row.pop(idx)

    def has_node(self, node: T) -> bool:
        """Return True if node is in the graph."""
        if self._repr is GraphRepr.ADJACENCY_LIST:
            return node in self._adj
        return node in self._node_idx

    def nodes(self) -> frozenset[T]:
        """Return all nodes as an immutable frozenset."""
        if self._repr is GraphRepr.ADJACENCY_LIST:
            return frozenset(self._adj)
        return frozenset(self._node_list)

    # ------------------------------------------------------------------
    # Edge operations
    # ------------------------------------------------------------------

    def add_edge(
        self,
        u: T,
        v: T,
        weight: float = 1.0,
        properties: PropertyBag | None = None,
    ) -> None:
        """Add an undirected edge between u and v with the given weight.

        Both nodes are added automatically if they do not already exist.
        If the edge already exists its weight is updated.
        """
        self.add_node(u)
        self.add_node(v)
        self._validate_weight(weight)

        if self._repr is GraphRepr.ADJACENCY_LIST:
            self._adj[u][v] = weight
            self._adj[v][u] = weight
        else:
            i, j = self._node_idx[u], self._node_idx[v]
            self._matrix[i][j] = weight
            self._matrix[j][i] = weight
        merged = dict(properties or {})
        merged["weight"] = weight
        self._edge_properties.setdefault(_edge_key(u, v), {}).update(merged)

    def remove_edge(self, u: T, v: T) -> None:
        """Remove the edge between u and v.

        Raises KeyError if either node or the edge does not exist.
        """
        if self._repr is GraphRepr.ADJACENCY_LIST:
            if u not in self._adj or v not in self._adj[u]:
                raise KeyError((u, v))
            del self._adj[u][v]
            del self._adj[v][u]
            self._edge_properties.pop(_edge_key(u, v), None)
        else:
            if u not in self._node_idx or v not in self._node_idx:
                raise KeyError((u, v))
            i, j = self._node_idx[u], self._node_idx[v]
            if self._matrix[i][j] == 0.0:
                raise KeyError((u, v))
            self._matrix[i][j] = 0.0
            self._matrix[j][i] = 0.0
            self._edge_properties.pop(_edge_key(u, v), None)

    def has_edge(self, u: T, v: T) -> bool:
        """Return True if an edge exists between u and v."""
        if self._repr is GraphRepr.ADJACENCY_LIST:
            return u in self._adj and v in self._adj[u]
        if u not in self._node_idx or v not in self._node_idx:
            return False
        i, j = self._node_idx[u], self._node_idx[v]
        return self._matrix[i][j] != 0.0

    def edges(self) -> frozenset[tuple[T, T, float]]:
        """Return all edges as a frozenset of (u, v, weight) triples.

        Each undirected edge appears exactly once.  The two endpoint nodes
        are ordered so the triple is canonical (smaller-repr node first when
        possible, otherwise insertion-order tiebreak).
        """
        result: set[tuple[T, T, float]] = set()

        if self._repr is GraphRepr.ADJACENCY_LIST:
            for u, neighbours in self._adj.items():
                for v, w in neighbours.items():
                    # Canonical ordering: use repr() to get a consistent sort key.
                    a, b = (u, v) if repr(u) <= repr(v) else (v, u)
                    result.add((a, b, w))
        else:
            n = len(self._node_list)
            for i in range(n):
                for j in range(i + 1, n):
                    w = self._matrix[i][j]
                    if w != 0.0:
                        result.add((self._node_list[i], self._node_list[j], w))

        return frozenset(result)

    def edge_weight(self, u: T, v: T) -> float:
        """Return the weight of edge (u, v).

        Raises KeyError if the edge does not exist.
        """
        if self._repr is GraphRepr.ADJACENCY_LIST:
            if u not in self._adj or v not in self._adj[u]:
                raise KeyError((u, v))
            return self._adj[u][v]
        if u not in self._node_idx or v not in self._node_idx:
            raise KeyError((u, v))
        i, j = self._node_idx[u], self._node_idx[v]
        w = self._matrix[i][j]
        if w == 0.0:
            raise KeyError((u, v))
        return w

    # ------------------------------------------------------------------
    # Property bags
    # ------------------------------------------------------------------

    def graph_properties(self) -> PropertyBag:
        """Return a copy of graph-level properties."""
        return dict(self._graph_properties)

    def set_graph_property(self, key: str, value: PropertyValue) -> None:
        """Set one graph-level property."""
        self._graph_properties[key] = value

    def remove_graph_property(self, key: str) -> None:
        """Remove one graph-level property if present."""
        self._graph_properties.pop(key, None)

    def node_properties(self, node: T) -> PropertyBag:
        """Return a copy of properties attached to ``node``."""
        if not self.has_node(node):
            raise KeyError(node)
        return dict(self._node_properties[node])

    def set_node_property(self, node: T, key: str, value: PropertyValue) -> None:
        """Set one property on ``node``."""
        if not self.has_node(node):
            raise KeyError(node)
        self._node_properties[node][key] = value

    def remove_node_property(self, node: T, key: str) -> None:
        """Remove one property from ``node`` if present."""
        if not self.has_node(node):
            raise KeyError(node)
        self._node_properties[node].pop(key, None)

    def edge_properties(self, u: T, v: T) -> PropertyBag:
        """Return a copy of properties attached to edge {u, v}."""
        if not self.has_edge(u, v):
            raise KeyError((u, v))
        properties = dict(self._edge_properties.get(_edge_key(u, v), {}))
        properties["weight"] = self.edge_weight(u, v)
        return properties

    def set_edge_property(
        self,
        u: T,
        v: T,
        key: str,
        value: PropertyValue,
    ) -> None:
        """Set one property on edge {u, v}."""
        if not self.has_edge(u, v):
            raise KeyError((u, v))
        if key == "weight":
            if not isinstance(value, (int, float)) or isinstance(value, bool):
                raise ValueError("edge property 'weight' must be numeric")
            self._set_edge_weight(u, v, float(value))
        self._edge_properties.setdefault(_edge_key(u, v), {})[key] = value

    def remove_edge_property(self, u: T, v: T, key: str) -> None:
        """Remove one property from edge {u, v} if present."""
        if not self.has_edge(u, v):
            raise KeyError((u, v))
        if key == "weight":
            self._set_edge_weight(u, v, 1.0)
            self._edge_properties.setdefault(_edge_key(u, v), {})["weight"] = 1.0
            return
        self._edge_properties.setdefault(_edge_key(u, v), {}).pop(key, None)

    # ------------------------------------------------------------------
    # Neighbourhood queries
    # ------------------------------------------------------------------

    def neighbors(self, node: T) -> frozenset[T]:
        """Return all neighbours of node as a frozenset.

        Raises KeyError if the node does not exist.
        """
        if self._repr is GraphRepr.ADJACENCY_LIST:
            if node not in self._adj:
                raise KeyError(node)
            return frozenset(self._adj[node])
        if node not in self._node_idx:
            raise KeyError(node)
        idx = self._node_idx[node]
        return frozenset(
            self._node_list[j]
            for j, w in enumerate(self._matrix[idx])
            if w != 0.0
        )

    def neighbors_weighted(self, node: T) -> dict[T, float]:
        """Return {neighbour: weight} for all neighbours of node.

        Raises KeyError if the node does not exist.
        """
        if self._repr is GraphRepr.ADJACENCY_LIST:
            if node not in self._adj:
                raise KeyError(node)
            return dict(self._adj[node])
        if node not in self._node_idx:
            raise KeyError(node)
        idx = self._node_idx[node]
        return {
            self._node_list[j]: w
            for j, w in enumerate(self._matrix[idx])
            if w != 0.0
        }

    def degree(self, node: T) -> int:
        """Return the degree of node (number of incident edges).

        Raises KeyError if the node does not exist.
        """
        return len(self.neighbors(node))

    def _validate_weight(self, weight: float) -> None:
        if not isinstance(weight, (int, float)) or isinstance(weight, bool):
            raise ValueError("edge weight must be numeric")

    def _set_edge_weight(self, u: T, v: T, weight: float) -> None:
        self._validate_weight(weight)
        if self._repr is GraphRepr.ADJACENCY_LIST:
            self._adj[u][v] = weight
            self._adj[v][u] = weight
            return
        i, j = self._node_idx[u], self._node_idx[v]
        self._matrix[i][j] = weight
        self._matrix[j][i] = weight

    # ------------------------------------------------------------------
    # Python dunder methods
    # ------------------------------------------------------------------

    def __len__(self) -> int:
        """Return the number of nodes in the graph."""
        if self._repr is GraphRepr.ADJACENCY_LIST:
            return len(self._adj)
        return len(self._node_list)

    def __contains__(self, node: object) -> bool:
        """Return True if node is in the graph  (supports ``node in graph``)."""
        return self.has_node(node)  # type: ignore[arg-type]

    def __repr__(self) -> str:
        return (
            f"Graph(nodes={len(self)}, "
            f"edges={len(self.edges())}, "
            f"repr={self._repr.value})"
        )
