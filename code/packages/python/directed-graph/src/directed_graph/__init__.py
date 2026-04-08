"""
directed_graph — DT01: Directed Graph extending Graph[T]
=========================================================

A directed graph that inherits from the DT00 ``Graph[T]`` base class.

The ``DirectedGraph[T]`` class extends ``Graph[T]`` with:

- Directed edges (u → v is stored; v → u is NOT implied)
- A reverse adjacency dict ``_reverse`` for efficient predecessor lookups
- ``successors``, ``predecessors``, ``out_degree``, ``in_degree`` methods
- Overridden ``neighbors()`` returning successors only — so bfs/dfs from
  the graph package work correctly on DirectedGraph

A separate ``LabeledDirectedGraph[T]`` (by composition) attaches a string
label to each edge.

Pure module-level algorithms:

- ``topological_sort``  — Kahn's BFS-based; raises ValueError on cycle
- ``has_cycle``         — iterative 3-color DFS
- ``transitive_closure``      — BFS over forward edges
- ``transitive_dependents``   — BFS over reverse edges
- ``independent_groups``      — Kahn's grouped by wave (parallel levels)
- ``affected_nodes``          — union of transitive_dependents
- ``strongly_connected_components`` — Kosaraju's two-pass iterative DFS

Quick start::

    from directed_graph import DirectedGraph, topological_sort, has_cycle

    g = DirectedGraph()
    g.add_edge("parse", "compile")
    g.add_edge("compile", "link")
    g.add_edge("compile", "typecheck")

    topological_sort(g)   # ["parse", "typecheck", "compile", "link"]
    has_cycle(g)           # False

    from directed_graph import bfs
    # Import from graph package — works because neighbors() returns successors:
    from graph import bfs
    bfs(g, "parse")       # ["parse", "compile", "link", "typecheck"]

Error handling — algorithms raise ValueError on cycle::

    g2 = DirectedGraph()
    g2.add_edge("A", "B")
    g2.add_edge("B", "A")   # cycle!

    has_cycle(g2)           # True
    topological_sort(g2)    # raises ValueError

Custom exception for self-loops::

    g3 = DirectedGraph()            # allow_self_loops=False by default
    g3.add_edge("A", "A")           # raises ValueError
    g4 = DirectedGraph(allow_self_loops=True)
    g4.add_edge("A", "A")           # OK
"""

from directed_graph.directed_graph import DirectedGraph, LabeledDirectedGraph
from directed_graph.algorithms import (
    affected_nodes,
    has_cycle,
    independent_groups,
    strongly_connected_components,
    topological_sort,
    transitive_closure,
    transitive_dependents,
)

# CycleError is now just ValueError (per DT01 spec — algorithms raise ValueError).
# We keep a CycleError alias for backwards compatibility with code that caught it.
CycleError = ValueError

__version__ = "0.1.0"

__all__ = [
    # Data structures
    "DirectedGraph",
    "LabeledDirectedGraph",
    # Algorithms
    "topological_sort",
    "has_cycle",
    "transitive_closure",
    "transitive_dependents",
    "independent_groups",
    "affected_nodes",
    "strongly_connected_components",
    # Exceptions
    "CycleError",
]
