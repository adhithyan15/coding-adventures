"""
Directed Graph — Native Extension (Rust-backed via python-bridge)
==================================================================

A drop-in alternative to ``coding-adventures-directed-graph`` backed by
a Rust implementation for better performance on large graphs.

All algorithms (topological sort, cycle detection, transitive closure,
independent groups, affected nodes) run in Rust. Only the method call
boundary crosses between Python and Rust.
"""

# The native .so/.dylib/.pyd is compiled from Rust and placed in this
# directory by the build process. It exports DirectedGraph class and
# exception types via PyInit_directed_graph_native.
from directed_graph_native.directed_graph_native import (  # type: ignore[import]
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
