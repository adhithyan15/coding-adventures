"""Generic multi-directed graph with metadata property bags."""

from multi_directed_graph.multi_directed_graph import (
    DuplicateEdgeIdError,
    EdgeNotFoundError,
    GraphPropertyBag,
    GraphPropertyValue,
    MultiDirectedEdge,
    MultiDirectedGraph,
    MultiDirectedGraphCycleError,
    NodeNotFoundError,
)

__version__ = "0.1.0"

__all__ = [
    "DuplicateEdgeIdError",
    "EdgeNotFoundError",
    "GraphPropertyBag",
    "GraphPropertyValue",
    "MultiDirectedEdge",
    "MultiDirectedGraph",
    "MultiDirectedGraphCycleError",
    "NodeNotFoundError",
]
