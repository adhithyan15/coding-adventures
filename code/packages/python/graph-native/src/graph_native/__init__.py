"""
graph_native - Rust-backed DT00 graph package for Python.
"""

from __future__ import annotations

from enum import Enum
from typing import Any

from graph_native.graph_native import (  # type: ignore[import]
    EdgeNotFoundError,
    Graph as _NativeGraph,
    NodeNotFoundError,
)


class GraphRepr(Enum):
    ADJACENCY_LIST = "adjacency_list"
    ADJACENCY_MATRIX = "adjacency_matrix"


class Graph:
    def __init__(self, repr: GraphRepr = GraphRepr.ADJACENCY_LIST) -> None:
        self._graph = _NativeGraph(repr.value)

    def add_node(self, node: str) -> None:
        self._graph.add_node(node)

    def remove_node(self, node: str) -> None:
        self._graph.remove_node(node)

    def has_node(self, node: str) -> bool:
        return bool(self._graph.has_node(node))

    def nodes(self) -> frozenset[str]:
        return frozenset(self._graph.nodes())

    def add_edge(self, u: str, v: str, weight: float = 1.0) -> None:
        self._graph.add_edge(u, v, weight)

    def remove_edge(self, u: str, v: str) -> None:
        self._graph.remove_edge(u, v)

    def has_edge(self, u: str, v: str) -> bool:
        return bool(self._graph.has_edge(u, v))

    def edges(self) -> frozenset[tuple[str, str, float]]:
        return frozenset(tuple(edge) for edge in self._graph.edges())

    def edge_weight(self, u: str, v: str) -> float:
        return float(self._graph.edge_weight(u, v))

    def neighbors(self, node: str) -> frozenset[str]:
        return frozenset(self._graph.neighbors(node))

    def neighbors_weighted(self, node: str) -> dict[str, float]:
        return dict(self._graph.neighbors_weighted(node))

    def degree(self, node: str) -> int:
        return int(self._graph.degree(node))

    def bfs(self, start: str) -> list[str]:
        return list(self._graph.bfs(start))

    def dfs(self, start: str) -> list[str]:
        return list(self._graph.dfs(start))

    def is_connected(self) -> bool:
        return bool(self._graph.is_connected())

    def connected_components(self) -> list[frozenset[str]]:
        return [frozenset(component) for component in self._graph.connected_components()]

    def has_cycle(self) -> bool:
        return bool(self._graph.has_cycle())

    def shortest_path(self, start: str, end: str) -> list[str]:
        return list(self._graph.shortest_path(start, end))

    def minimum_spanning_tree(self) -> frozenset[tuple[str, str, float]]:
        return frozenset(tuple(edge) for edge in self._graph.minimum_spanning_tree())

    def __len__(self) -> int:
        return len(self._graph)

    def __contains__(self, node: object) -> bool:
        if not isinstance(node, str):
            return False
        return node in self._graph

    def __repr__(self) -> str:
        return repr(self._graph)


def _unwrap_graph(graph: Graph | _NativeGraph) -> Any:
    return graph._graph if isinstance(graph, Graph) else graph


def bfs(graph: Graph | _NativeGraph, start: str) -> list[str]:
    return list(_unwrap_graph(graph).bfs(start))


def dfs(graph: Graph | _NativeGraph, start: str) -> list[str]:
    return list(_unwrap_graph(graph).dfs(start))


def is_connected(graph: Graph | _NativeGraph) -> bool:
    return bool(_unwrap_graph(graph).is_connected())


def connected_components(graph: Graph | _NativeGraph) -> list[frozenset[str]]:
    return [frozenset(component) for component in _unwrap_graph(graph).connected_components()]


def has_cycle(graph: Graph | _NativeGraph) -> bool:
    return bool(_unwrap_graph(graph).has_cycle())


def shortest_path(graph: Graph | _NativeGraph, start: str, end: str) -> list[str]:
    return list(_unwrap_graph(graph).shortest_path(start, end))


def minimum_spanning_tree(
    graph: Graph | _NativeGraph,
) -> frozenset[tuple[str, str, float]]:
    return frozenset(tuple(edge) for edge in _unwrap_graph(graph).minimum_spanning_tree())


__all__ = [
    "EdgeNotFoundError",
    "Graph",
    "GraphRepr",
    "NodeNotFoundError",
    "bfs",
    "connected_components",
    "dfs",
    "has_cycle",
    "is_connected",
    "minimum_spanning_tree",
    "shortest_path",
]
