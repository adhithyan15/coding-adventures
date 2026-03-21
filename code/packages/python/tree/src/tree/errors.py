"""
errors.py -- Custom Exceptions for the Tree Library
=====================================================

Trees impose strict structural constraints on top of directed graphs. When those
constraints are violated, we need clear, specific errors rather than generic
``ValueError`` or ``KeyError``. Each exception class here corresponds to one
particular kind of violation:

- ``TreeError`` -- the base class for all tree-specific errors. You can catch
  this to handle any tree error generically, or catch a more specific subclass
  when you want to handle one case differently.

- ``NodeNotFoundError`` -- raised when you reference a node that doesn't exist
  in the tree. This is the tree-level equivalent of the directed graph's
  ``NodeNotFoundError``, but we define our own so that callers can catch
  tree errors without importing the graph library.

- ``DuplicateNodeError`` -- raised when you try to add a node that already
  exists. In a tree, every node name must be unique because each node has
  exactly one position in the hierarchy. (This is different from a general
  graph where duplicate ``add_node`` calls are silently ignored.)

- ``RootRemovalError`` -- raised when you try to remove the root node. The root
  is the anchor of the entire tree; removing it would leave a disconnected
  collection of subtrees, which is no longer a tree.
"""


class TreeError(Exception):
    """Base exception for all tree-related errors.

    This exists so callers can write ``except TreeError`` to catch any tree
    error without listing every subclass. It also serves as documentation:
    if you see ``TreeError`` in a traceback, you know the problem is with
    tree structure, not with the underlying graph.
    """

    pass


class NodeNotFoundError(TreeError):
    """Raised when an operation references a node not in the tree.

    The ``node`` attribute carries the missing node's name, so error messages
    can tell you exactly what was missing.

    Example::

        try:
            tree.parent("nonexistent")
        except NodeNotFoundError as e:
            print(f"Missing: {e.node}")  # Missing: nonexistent
    """

    def __init__(self, node: str) -> None:
        super().__init__(f"Node not found in tree: {node!r}")
        self.node = node


class DuplicateNodeError(TreeError):
    """Raised when trying to add a node that already exists in the tree.

    In a tree, every node occupies a unique position. If you could add a
    node twice, it would have two parents -- violating the tree invariant
    that every non-root node has exactly one parent.

    The ``node`` attribute carries the duplicate node's name.
    """

    def __init__(self, node: str) -> None:
        super().__init__(f"Node already exists in tree: {node!r}")
        self.node = node


class RootRemovalError(TreeError):
    """Raised when trying to remove the root node.

    The root is special: it's the only node with no parent, and every other
    node is reachable from it. Removing the root would destroy the tree's
    connected structure.

    If you want to replace the entire tree, create a new ``Tree`` instead.
    """

    def __init__(self) -> None:
        super().__init__("Cannot remove the root node")
