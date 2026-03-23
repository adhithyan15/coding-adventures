"""
Directed Graph — Native Extension (Rust-backed via PyO3)
=========================================================

A drop-in alternative to ``coding-adventures-directed-graph`` backed by
a Rust implementation for better performance on large graphs.

The API is identical to the pure Python version::

    from directed_graph_native import DirectedGraph, CycleError

    g = DirectedGraph()
    g.add_edge("A", "B")
    g.add_edge("B", "C")

    print(g.topological_sort())    # ['A', 'B', 'C']
    print(g.independent_groups())  # [['A'], ['B'], ['C']]

All algorithms (topological sort, cycle detection, transitive closure,
independent groups, affected nodes) run in Rust. Only the method call
boundary crosses between Python and Rust — the hot loops stay in native
code.
"""

# The actual classes and exceptions are defined in Rust (src/lib.rs) and
# compiled into a native extension module by maturin. This __init__.py
# re-exports them so that `from directed_graph_native import DirectedGraph`
# works naturally.
from directed_graph_native.directed_graph_native import (
    CycleError,
    DirectedGraph,
    EdgeNotFoundError,
    NodeNotFoundError,
)

__all__ = [
    "DirectedGraph",
    "CycleError",
    "NodeNotFoundError",
    "EdgeNotFoundError",
]
