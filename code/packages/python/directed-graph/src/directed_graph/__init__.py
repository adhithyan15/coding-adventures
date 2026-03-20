"""
Directed Graph Library
======================

A directed graph implementation with topological sort, cycle detection, transitive
closure, and parallel execution level computation. Built for use in build systems,
dependency resolution, and task scheduling.

The library provides a single ``DirectedGraph`` class that stores nodes and directed
edges using a pair of adjacency dictionaries (forward and reverse). All graph algorithms
-- topological sort, cycle detection, transitive closure, independent grouping -- are
methods on the graph object itself, so you never need to import a separate module.

Quick start::

    from directed_graph import DirectedGraph

    g = DirectedGraph()
    g.add_edge("A", "B")
    g.add_edge("B", "C")

    print(g.topological_sort())   # ['A', 'B', 'C']
    print(g.independent_groups()) # [['A'], ['B'], ['C']]

Error classes are available at the top level too::

    from directed_graph import CycleError, NodeNotFoundError, EdgeNotFoundError
"""

from directed_graph.graph import (
    CycleError,
    DirectedGraph,
    EdgeNotFoundError,
    NodeNotFoundError,
)
from directed_graph.labeled_graph import LabeledDirectedGraph

__all__ = [
    "DirectedGraph",
    "LabeledDirectedGraph",
    "CycleError",
    "NodeNotFoundError",
    "EdgeNotFoundError",
]
