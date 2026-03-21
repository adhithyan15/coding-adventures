"""
Tree Library
============

A rooted tree data structure backed by a directed graph. Provides tree traversals,
lowest common ancestor (LCA), subtree extraction, and ASCII visualization.

The tree enforces three invariants:

1. Exactly one root (no predecessors)
2. Every non-root node has exactly one parent
3. No cycles

Quick start::

    from tree import Tree

    t = Tree("Program")
    t.add_child("Program", "Assignment")
    t.add_child("Program", "Print")
    t.add_child("Assignment", "Name")
    t.add_child("Assignment", "BinaryOp")

    print(t.to_ascii())
    # Program
    # ├── Assignment
    # │   ├── BinaryOp
    # │   └── Name
    # └── Print

    print(t.preorder())    # ['Program', 'Assignment', 'BinaryOp', 'Name', 'Print']
    print(t.lca("Name", "BinaryOp"))  # 'Assignment'

Error classes are available at the top level too::

    from tree import TreeError, NodeNotFoundError, DuplicateNodeError, RootRemovalError
"""

from tree.errors import (
    DuplicateNodeError,
    NodeNotFoundError,
    RootRemovalError,
    TreeError,
)
from tree.tree import Tree

__all__ = [
    "Tree",
    "TreeError",
    "NodeNotFoundError",
    "DuplicateNodeError",
    "RootRemovalError",
]
