"""
tree.py -- A Rooted Tree Backed by a Directed Graph
=====================================================

What Is a Tree?
---------------

A **tree** is one of the most fundamental data structures in computer science.
You encounter trees everywhere:

- File systems: directories contain files and subdirectories
- HTML/XML: elements contain child elements
- Programming languages: Abstract Syntax Trees (ASTs) represent code structure
- Organization charts: managers have direct reports

Formally, a tree is a connected, acyclic graph where:

1. There is exactly **one root** node (a node with no parent).
2. Every other node has exactly **one parent**.
3. There are **no cycles** -- you can never follow edges and return to where
   you started.

These constraints mean a tree with N nodes always has exactly N-1 edges.

    Tree vs. Graph
    ~~~~~~~~~~~~~~

    A tree IS a graph (specifically, a directed acyclic graph with the
    single-parent constraint). We leverage this by building our Tree on top
    of the ``DirectedGraph`` class from the directed-graph package. The
    ``DirectedGraph`` handles all the low-level node/edge storage, while
    this ``Tree`` class enforces the tree invariants and provides
    tree-specific operations like traversals, depth calculation, and
    lowest common ancestor.

    Edges point from parent to child:

        Program
        ├── Assignment    (edge: Program → Assignment)
        │   ├── Name      (edge: Assignment → Name)
        │   └── BinaryOp  (edge: Assignment → BinaryOp)
        └── Print         (edge: Program → Print)


Tree Terminology
----------------

Let's define the vocabulary we'll use throughout this module:

- **Root**: The topmost node. It has no parent. Every tree has exactly one.
- **Parent**: The node directly above another node. Every non-root node has
  exactly one parent.
- **Child**: A node directly below another node. A node can have zero or more
  children.
- **Siblings**: Nodes that share the same parent.
- **Leaf**: A node with no children (a "dead end" in the tree).
- **Internal node**: A node that has at least one child.
- **Depth**: The number of edges from the root to a node. The root has depth 0.
- **Height**: The maximum depth of any node in the tree.
- **Subtree**: A node together with all its descendants forms a smaller tree.
- **Path**: The sequence of nodes from the root to a given node.
- **Lowest Common Ancestor (LCA)**: The deepest node that is an ancestor of
  both node A and node B.

Implementation Strategy
-----------------------

We store the tree as a ``DirectedGraph`` with edges pointing parent → child.
This means:

- ``graph.successors(node)`` returns the children
- ``graph.predecessors(node)`` returns a list with zero or one element
  (the parent, or empty for the root)

We maintain the tree invariants by checking them in ``add_child``:

- The parent must already exist in the tree
- The child must NOT already exist (no duplicate nodes)
- Since we only add one parent edge per child, cycles are impossible

This means we never need to check for cycles explicitly -- the single-parent
invariant prevents them.
"""

from __future__ import annotations

from collections import deque

from directed_graph import DirectedGraph

from tree.errors import DuplicateNodeError, NodeNotFoundError, RootRemovalError


class Tree:
    """A rooted tree backed by a DirectedGraph.

    A tree is a directed graph with three constraints:

    1. Exactly one root (no predecessors)
    2. Every non-root node has exactly one parent
    3. No cycles

    Edges point parent → child. The tree is constructed by specifying a root
    node and then adding children one at a time with ``add_child``.

    Example::

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
    """

    # ------------------------------------------------------------------
    # Construction
    # ------------------------------------------------------------------
    # A tree always starts with a root. You can't have an empty tree
    # (that would be a forest, or nothing at all). This simplifies the
    # implementation because we always know the root exists.

    def __init__(self, root: str) -> None:
        """Create a new tree with the given root node.

        Args:
            root: The name of the root node. This will be the ancestor
                  of every other node in the tree.
        """
        self._graph = DirectedGraph()
        self._graph.add_node(root)
        self._root = root

    # ------------------------------------------------------------------
    # Mutation
    # ------------------------------------------------------------------

    def add_child(self, parent: str, child: str) -> None:
        """Add a child node under the given parent.

        This is the primary way to build up a tree. Each call adds one
        new node and one edge (parent → child).

        Args:
            parent: The name of the existing parent node.
            child: The name of the new child node (must not already exist).

        Raises:
            NodeNotFoundError: If ``parent`` is not in the tree.
            DuplicateNodeError: If ``child`` is already in the tree.

        Why not allow adding a node that already exists? Because in a tree,
        every node has exactly one parent. If we allowed adding "X" under
        both "A" and "B", node "X" would have two parents -- violating the
        tree invariant.

        Example::

            t = Tree("root")
            t.add_child("root", "child1")
            t.add_child("root", "child2")
            t.add_child("child1", "grandchild")
        """
        if not self._graph.has_node(parent):
            raise NodeNotFoundError(parent)
        if self._graph.has_node(child):
            raise DuplicateNodeError(child)

        # add_edge implicitly creates the child node and adds the edge
        self._graph.add_edge(parent, child)

    def remove_subtree(self, node: str) -> None:
        """Remove a node and all its descendants from the tree.

        This is a "prune" operation -- it cuts off an entire branch. The node
        and everything below it is removed. The parent of the removed node
        is unaffected.

        Args:
            node: The node to remove (along with its descendants).

        Raises:
            NodeNotFoundError: If ``node`` is not in the tree.
            RootRemovalError: If ``node`` is the root (can't remove the root).

        How it works:
        We do a post-order traversal of the subtree (children before parents)
        and remove each node from the underlying graph. Post-order ensures
        we remove leaves first, working our way up to the target node.

        Example::

            t = Tree("A")
            t.add_child("A", "B")
            t.add_child("B", "C")
            t.add_child("B", "D")
            t.remove_subtree("B")
            # Tree now contains only "A"
        """
        if not self._graph.has_node(node):
            raise NodeNotFoundError(node)
        if node == self._root:
            raise RootRemovalError()

        # Collect all descendants using BFS, then remove in reverse
        # (so we remove children before their parents).
        to_remove = self._collect_subtree_nodes(node)

        # Remove from bottom up. Since _collect_subtree_nodes returns
        # them in BFS order (parent before children), we reverse to get
        # children-first order.
        for n in reversed(to_remove):
            self._graph.remove_node(n)

    def _collect_subtree_nodes(self, node: str) -> list[str]:
        """Collect all nodes in the subtree rooted at ``node`` using BFS.

        Returns a list starting with ``node`` and then all descendants
        in breadth-first order.
        """
        result: list[str] = []
        queue: deque[str] = deque([node])

        while queue:
            current = queue.popleft()
            result.append(current)
            # Sort children for deterministic ordering
            for child in sorted(self._graph.successors(current)):
                queue.append(child)

        return result

    # ------------------------------------------------------------------
    # Queries
    # ------------------------------------------------------------------
    # These methods let you inspect the tree's structure without modifying it.

    @property
    def root(self) -> str:
        """The root node of the tree.

        The root is the unique node with no parent. It's set at construction
        time and never changes.
        """
        return self._root

    def parent(self, node: str) -> str | None:
        """Return the parent of a node, or None if the node is the root.

        In a tree, every non-root node has exactly one parent. The root
        has no parent, so we return None for it.

        Args:
            node: The node whose parent to find.

        Raises:
            NodeNotFoundError: If ``node`` is not in the tree.

        How it works:
        We look up the predecessors of the node in the underlying graph.
        Since we maintain the tree invariant (single parent), there will be
        either 0 predecessors (root) or 1 predecessor (everyone else).
        """
        if not self._graph.has_node(node):
            raise NodeNotFoundError(node)

        predecessors = self._graph.predecessors(node)
        if len(predecessors) == 0:
            return None
        # predecessors() returns a frozenset (not indexable); extract the sole element.
        return next(iter(predecessors))

    def children(self, node: str) -> list[str]:
        """Return the children of a node (sorted alphabetically).

        A node with no children is a leaf. A node with children is an
        internal node.

        Args:
            node: The node whose children to list.

        Raises:
            NodeNotFoundError: If ``node`` is not in the tree.
        """
        if not self._graph.has_node(node):
            raise NodeNotFoundError(node)

        return sorted(self._graph.successors(node))

    def siblings(self, node: str) -> list[str]:
        """Return the siblings of a node (other children of the same parent).

        The root has no siblings (it has no parent). A node whose parent
        has only one child also has no siblings.

        Args:
            node: The node whose siblings to find.

        Raises:
            NodeNotFoundError: If ``node`` is not in the tree.

        Example::

            t = Tree("root")
            t.add_child("root", "A")
            t.add_child("root", "B")
            t.add_child("root", "C")
            t.siblings("A")  # ["B", "C"]
        """
        if not self._graph.has_node(node):
            raise NodeNotFoundError(node)

        parent_node = self.parent(node)
        if parent_node is None:
            # Root has no siblings
            return []

        return [child for child in self.children(parent_node) if child != node]

    def is_leaf(self, node: str) -> bool:
        """Return True if the node has no children.

        Leaves are the "endpoints" of a tree -- they don't branch further.
        In an AST, leaves are typically literals and variable names.

        Raises:
            NodeNotFoundError: If ``node`` is not in the tree.
        """
        if not self._graph.has_node(node):
            raise NodeNotFoundError(node)

        return len(self._graph.successors(node)) == 0

    def is_root(self, node: str) -> bool:
        """Return True if the node is the root of the tree.

        Raises:
            NodeNotFoundError: If ``node`` is not in the tree.
        """
        if not self._graph.has_node(node):
            raise NodeNotFoundError(node)

        return node == self._root

    def depth(self, node: str) -> int:
        """Return the depth of a node (distance from root).

        The depth is the number of edges on the path from the root to this
        node. The root has depth 0, its children have depth 1, and so on.

        This is computed by walking up the parent chain from the node to the
        root, counting steps. For a tree of height H, this is O(H) in the
        worst case.

        Raises:
            NodeNotFoundError: If ``node`` is not in the tree.

        Depth vs. Height
        ~~~~~~~~~~~~~~~~
        Don't confuse depth and height:
        - **Depth** is measured from the TOP (root) DOWN to a node.
        - **Height** is measured from the BOTTOM (deepest leaf) UP.

        In a tree with structure:
            A (depth 0)
            └── B (depth 1)
                └── C (depth 2)

        The height of the tree is 2 (the maximum depth).
        """
        if not self._graph.has_node(node):
            raise NodeNotFoundError(node)

        d = 0
        current = node
        while current != self._root:
            predecessors = self._graph.predecessors(current)
            current = next(iter(predecessors))  # frozenset: extract single element
            d += 1

        return d

    def height(self) -> int:
        """Return the height of the tree (maximum depth of any node).

        A single-node tree has height 0. An empty tree is impossible (we
        always have at least a root).

        This is computed by finding the maximum depth across all nodes.
        We use BFS from the root, tracking the depth at each level.

        Time complexity: O(N) where N is the number of nodes.
        """
        max_depth = 0
        # BFS with depth tracking
        queue: deque[tuple[str, int]] = deque([(self._root, 0)])

        while queue:
            current, d = queue.popleft()
            if d > max_depth:
                max_depth = d
            for child in self._graph.successors(current):
                queue.append((child, d + 1))

        return max_depth

    def size(self) -> int:
        """Return the total number of nodes in the tree.

        This includes the root and all descendants.
        """
        return len(self._graph)

    def nodes(self) -> list[str]:
        """Return a list of all nodes in the tree (sorted alphabetically).

        The order is alphabetical for determinism, not structural.
        """
        return sorted(self._graph.nodes())

    def leaves(self) -> list[str]:
        """Return all leaf nodes (sorted alphabetically).

        Leaves are nodes with no children. In an AST, these are typically
        literals, variable names, and other terminal symbols.
        """
        return sorted(
            node
            for node in self._graph.nodes()
            if len(self._graph.successors(node)) == 0
        )

    def has_node(self, node: str) -> bool:
        """Return True if the node exists in the tree."""
        return self._graph.has_node(node)

    def __len__(self) -> int:
        """Return the number of nodes in the tree (same as ``size()``)."""
        return self.size()

    def __contains__(self, node: str) -> bool:
        """Return True if the node exists in the tree (same as ``has_node()``)."""
        return self.has_node(node)

    # ------------------------------------------------------------------
    # Traversals
    # ------------------------------------------------------------------
    #
    # Tree traversals visit every node exactly once, but in different orders.
    # The three classic traversals are:
    #
    # 1. **Preorder** (root first): Visit a node, then visit all its children.
    #    This gives a "top-down" view. Good for: copying a tree, prefix notation.
    #
    # 2. **Postorder** (root last): Visit all children, then visit the node.
    #    This gives a "bottom-up" view. Good for: computing sizes, deleting trees,
    #    postfix notation, evaluating expressions.
    #
    # 3. **Level-order** (breadth-first): Visit all nodes at depth 0, then
    #    depth 1, then depth 2, etc. Good for: finding shortest paths, printing
    #    by level.
    #
    # For a tree:
    #       A
    #      / \
    #     B   C
    #    / \
    #   D   E
    #
    # Preorder:    A, B, D, E, C
    # Postorder:   D, E, B, C, A
    # Level-order: A, B, C, D, E

    def preorder(self) -> list[str]:
        """Return nodes in preorder (parent before children).

        Preorder traversal visits a node BEFORE any of its descendants.
        Within each level, children are visited in sorted (alphabetical) order.

        Implementation:
        We use an explicit stack (not recursion) to avoid stack overflow on
        deep trees. We push children in reverse sorted order so that they
        come off the stack in sorted order.
        """
        result: list[str] = []
        stack: list[str] = [self._root]

        while stack:
            node = stack.pop()
            result.append(node)
            # Push children in reverse sorted order so smallest pops first
            children = sorted(self._graph.successors(node), reverse=True)
            stack.extend(children)

        return result

    def postorder(self) -> list[str]:
        """Return nodes in postorder (children before parent).

        Postorder traversal visits a node AFTER all of its descendants.
        This is useful for operations that need to process children before
        their parent (e.g., computing subtree sizes, or deleting a tree
        from leaves to root).

        Implementation:
        We use a recursive helper. For most trees (ASTs, file systems),
        the depth is manageable. For extremely deep trees, an iterative
        approach with two stacks would be safer.
        """
        result: list[str] = []
        self._postorder_recursive(self._root, result)
        return result

    def _postorder_recursive(self, node: str, result: list[str]) -> None:
        """Recursive postorder helper.

        Visit all children (in sorted order) before appending the node itself.
        """
        for child in sorted(self._graph.successors(node)):
            self._postorder_recursive(child, result)
        result.append(node)

    def level_order(self) -> list[str]:
        """Return nodes in level-order (breadth-first).

        Level-order visits all nodes at depth 0 first, then depth 1, then
        depth 2, and so on. Within each depth level, nodes are visited in
        sorted (alphabetical) order.

        Implementation:
        Classic BFS using a queue (deque for O(1) popleft).
        """
        result: list[str] = []
        queue: deque[str] = deque([self._root])

        while queue:
            node = queue.popleft()
            result.append(node)
            for child in sorted(self._graph.successors(node)):
                queue.append(child)

        return result

    # ------------------------------------------------------------------
    # Utilities
    # ------------------------------------------------------------------

    def path_to(self, node: str) -> list[str]:
        """Return the path from the root to the given node.

        The path is a list starting with the root and ending with the target
        node, containing every node along the way. For the root itself, the
        path is just ``[root]``.

        This is computed by walking up the parent chain from the node to
        the root, then reversing.

        Raises:
            NodeNotFoundError: If ``node`` is not in the tree.

        Example::

            t = Tree("A")
            t.add_child("A", "B")
            t.add_child("B", "C")
            t.path_to("C")  # ["A", "B", "C"]
        """
        if not self._graph.has_node(node):
            raise NodeNotFoundError(node)

        path: list[str] = []
        current = node

        while current is not None:
            path.append(current)
            current = self.parent(current)

        path.reverse()
        return path

    def lca(self, a: str, b: str) -> str:
        """Return the lowest common ancestor (LCA) of nodes a and b.

        The LCA is the deepest node that is an ancestor of both ``a`` and ``b``.
        If one node is an ancestor of the other, the ancestor is the LCA.
        The LCA of a node with itself is the node itself.

        Algorithm:
        1. Compute the path from root to ``a``.
        2. Compute the path from root to ``b``.
        3. Walk both paths in parallel from the root. The last node where
           both paths agree is the LCA.

        This is the "naive" LCA algorithm with O(depth) time complexity.
        For trees where you need many LCA queries, more sophisticated
        algorithms exist (binary lifting, Euler tour + sparse table), but
        this simple approach is clear and correct.

        Raises:
            NodeNotFoundError: If ``a`` or ``b`` is not in the tree.

        Example::

                    A
                   / \\
                  B   C
                 / \\
                D   E

            lca("D", "E") → "B"  (B is parent of both)
            lca("D", "C") → "A"  (A is the only common ancestor)
            lca("B", "D") → "B"  (B is ancestor of D)
        """
        if not self._graph.has_node(a):
            raise NodeNotFoundError(a)
        if not self._graph.has_node(b):
            raise NodeNotFoundError(b)

        path_a = self.path_to(a)
        path_b = self.path_to(b)

        # Walk both paths from root, finding the last common node
        lca_node = self._root
        for na, nb in zip(path_a, path_b):
            if na == nb:
                lca_node = na
            else:
                break

        return lca_node

    def subtree(self, node: str) -> Tree:
        """Extract the subtree rooted at the given node.

        Returns a NEW ``Tree`` object containing the node and all its
        descendants. The original tree is not modified.

        This is useful for isolating a part of a tree for independent
        processing. For example, in a compiler, you might extract the
        subtree for a function body to compile it separately.

        Raises:
            NodeNotFoundError: If ``node`` is not in the tree.

        How it works:
        1. Create a new tree with ``node`` as root.
        2. BFS through the original tree starting at ``node``.
        3. For each edge (parent → child), add_child in the new tree.
        """
        if not self._graph.has_node(node):
            raise NodeNotFoundError(node)

        new_tree = Tree(node)
        queue: deque[str] = deque([node])

        while queue:
            current = queue.popleft()
            for child in sorted(self._graph.successors(current)):
                new_tree.add_child(current, child)
                queue.append(child)

        return new_tree

    # ------------------------------------------------------------------
    # Visualization
    # ------------------------------------------------------------------

    def to_ascii(self) -> str:
        """Render the tree as an ASCII art diagram.

        Produces output like::

            Program
            ├── Assignment
            │   ├── BinaryOp
            │   └── Name
            └── Print

        The box-drawing characters used are:
        - ``├──`` for a child that has more siblings after it
        - ``└──`` for the last child of its parent
        - ``│   `` for a vertical continuation line
        - ``    `` (spaces) for padding where no continuation is needed

        Children are displayed in sorted (alphabetical) order.

        This is implemented recursively, building up prefix strings as we
        go deeper into the tree. Each level of recursion adds either a
        "│   " or "    " prefix depending on whether the parent has more
        siblings to display.
        """
        lines: list[str] = []
        self._ascii_recursive(self._root, "", "", lines)
        return "\n".join(lines)

    def _ascii_recursive(
        self,
        node: str,
        prefix: str,
        child_prefix: str,
        lines: list[str],
    ) -> None:
        """Recursive helper for ``to_ascii``.

        Args:
            node: The current node to render.
            prefix: The prefix for this node's line (includes the connector).
            child_prefix: The prefix for this node's children (includes
                          continuation lines).
            lines: The accumulator list of output lines.

        How the prefixes work:

        For a tree like:
            A
            ├── B
            │   └── D
            └── C

        When rendering B: prefix = "├── ", child_prefix = "│   "
        When rendering C: prefix = "└── ", child_prefix = "    "
        When rendering D: prefix = "│   └── ", child_prefix = "│       "
        """
        lines.append(prefix + node)
        children = sorted(self._graph.successors(node))

        for i, child in enumerate(children):
            if i < len(children) - 1:
                # Not the last child: use ├── and continue with │
                self._ascii_recursive(
                    child,
                    child_prefix + "├── ",
                    child_prefix + "│   ",
                    lines,
                )
            else:
                # Last child: use └── and continue with spaces
                self._ascii_recursive(
                    child,
                    child_prefix + "└── ",
                    child_prefix + "    ",
                    lines,
                )

    # ------------------------------------------------------------------
    # Graph access
    # ------------------------------------------------------------------

    @property
    def graph(self) -> DirectedGraph:
        """Access the underlying DirectedGraph.

        This is exposed for advanced use cases where you need direct access
        to the graph's algorithms (e.g., transitive closure). Modifying the
        graph directly may violate tree invariants, so use with caution.
        """
        return self._graph

    # ------------------------------------------------------------------
    # String representation
    # ------------------------------------------------------------------

    def __repr__(self) -> str:
        return f"Tree(root={self._root!r}, size={self.size()})"
