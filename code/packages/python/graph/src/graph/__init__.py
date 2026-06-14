"""
graph — DT00: Undirected Graph
===============================

The foundation of the entire DT data-structure series.

Public API::

    from graph import Graph, GraphRepr
    from graph import bfs, dfs, is_connected, connected_components
    from graph import has_cycle, shortest_path, minimum_spanning_tree
"""

from graph.graph import Graph, GraphRepr, PropertyBag, PropertyValue
from graph.algorithms import (
    bfs,
    connected_components,
    dfs,
    has_cycle,
    is_connected,
    minimum_spanning_tree,
    shortest_path,
)

__version__ = "0.1.0"

__all__ = [
    "Graph",
    "GraphRepr",
    "PropertyBag",
    "PropertyValue",
    "bfs",
    "connected_components",
    "dfs",
    "has_cycle",
    "is_connected",
    "minimum_spanning_tree",
    "shortest_path",
]
