"""
Undirected Graph Library
========================

An undirected graph implementation from scratch with comprehensive algorithms.

Quick start::

    from coding_adventures_graph import Graph, bfs, shortest_path

    g = Graph()
    g.add_edge("A", "B")
    g.add_edge("B", "C")

    print(g.nodes())       # frozenset({"A", "B", "C"})
    print(g.edges())       # frozenset({("A", "B", 1.0), ("B", "C", 1.0)})
    print(bfs(g, "A"))     # ["A", "B", "C"]
    print(shortest_path(g, "A", "C"))  # ["A", "B", "C"]
"""

from coding_adventures_graph.graph import (
    Graph,
    GraphRepr,
    NodeNotFoundError,
    EdgeNotFoundError,
    bfs,
    dfs,
    is_connected,
    connected_components,
    has_cycle,
    shortest_path,
    minimum_spanning_tree,
    UnionFind,
)

__all__ = [
    "Graph",
    "GraphRepr",
    "NodeNotFoundError",
    "EdgeNotFoundError",
    "bfs",
    "dfs",
    "is_connected",
    "connected_components",
    "has_cycle",
    "shortest_path",
    "minimum_spanning_tree",
    "UnionFind",
]
