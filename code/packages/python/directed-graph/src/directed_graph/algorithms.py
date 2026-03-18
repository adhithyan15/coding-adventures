"""
algorithms.py -- Algorithm Documentation and Re-exports
========================================================

All graph algorithms are implemented directly on the ``DirectedGraph`` class
in ``graph.py``. This module exists as a documentation hub that explains the
algorithmic ideas in more depth and provides convenient re-exports.

Why are algorithms on the class instead of in a separate module?
----------------------------------------------------------------

In many graph libraries, algorithms live in their own namespace (e.g.,
``networkx.algorithms.shortest_path``). That makes sense for a library with
hundreds of algorithms, but our graph has exactly five algorithmic methods:

1. ``topological_sort()``  -- Kahn's algorithm
2. ``has_cycle()``         -- DFS three-color marking
3. ``transitive_closure()`` -- BFS forward reachability
4. ``transitive_dependents()`` -- BFS reverse reachability
5. ``independent_groups()`` -- Modified Kahn's for parallel levels
6. ``affected_nodes()``    -- Union of transitive dependents

All of these need access to the internal ``_forward`` and ``_reverse`` dicts,
so making them instance methods is the most natural and Pythonic choice.

Algorithm Details
-----------------

**Kahn's Algorithm (topological_sort, independent_groups)**

Kahn's algorithm maintains a count of incoming edges (in-degree) for each
node. It repeatedly finds nodes with in-degree zero, removes them from the
graph (conceptually), and decrements the in-degree of their successors.

The key insight for ``independent_groups`` is that instead of processing
zero-in-degree nodes one at a time, we can process ALL of them as a batch.
Each batch forms a "level" -- nodes at the same level have no dependencies
on each other and can run in parallel.

**DFS Three-Color (has_cycle)**

The three colors represent the state of each node during DFS:
- WHITE: unvisited
- GRAY: currently on the recursion stack (being explored)
- BLACK: fully explored (all descendants visited)

A cycle exists if and only if we encounter a GRAY node during DFS. A GRAY
node means we've found a path back to a node that's still being explored --
that's a back edge, which means a cycle.

**BFS Reachability (transitive_closure, transitive_dependents)**

Both methods use simple BFS. The only difference is which adjacency dict
they traverse: ``_forward`` for transitive_closure (downstream) and
``_reverse`` for transitive_dependents (upstream).
"""

# Re-export everything from graph.py for convenience. Users who prefer
# ``from directed_graph.algorithms import ...`` can do so.
from directed_graph.graph import (
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
