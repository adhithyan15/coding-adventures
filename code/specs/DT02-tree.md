# DT02 — Tree

## Overview

A tree is a directed graph with three structural invariants enforced by construction:
exactly one root node (the only node with no parent), every non-root node has exactly
one parent, and there are no cycles. These constraints arise naturally in hierarchical
data — file systems, HTML documents, abstract syntax trees, organization charts — where
every element belongs to exactly one parent and the hierarchy has a single top-level entry
point. DT02 builds on DT01 (DirectedGraph) and adds the parent/child API, tree-specific
traversals, and subtree operations.

## Layer Position

```
DT00 graph (undirected base)
        │
        ▼
DT01 directed-graph
        │
        ▼
[YOU ARE HERE: DT02 tree]
        │
        ▼
DT03 binary-tree      (tree: max 2 children, left/right semantics)
       ├── DT04 heap             (binary-tree + heap property)
       ├── DT05 segment-tree     (binary-tree + range aggregates)
       ├── DT07 binary-search-tree (binary-tree + ordering invariant)
       │        ├── DT08 avl-tree
       │        ├── DT09 red-black-tree
       │        └── DT10 treap
       └── DT11 b-tree (M-way search tree)
               └── DT12 b-plus-tree
DT06 fenwick-tree     (array-backed implicit tree, not a subtype of DT02)
DT13 trie             (tree where edges are characters)
     └── DT14 radix-tree ── DT15 suffix-tree
DT16 rope             (binary-tree for strings)
```

## Concepts

### What is a tree?

A tree is a hierarchy. One node is at the top — the **root**. Every other node has
exactly one node above it — its **parent**. A node can have zero or more nodes below
it — its **children**. Nodes with no children are called **leaves**.

```
                    [root]
                   /      \
            [child-1]    [child-2]
            /      \          \
     [gc-1]   [gc-2]        [gc-3]
```

This is the same shape as a family tree, a corporate org chart, or the folders on
your computer. The constraint "every node has exactly one parent" is what makes it a
tree and not a general directed graph.

### The three invariants, stated precisely

```
Invariant 1 — Single root:
    Exactly one node r exists such that predecessors(r) = ∅.
    All other nodes have predecessors(v) = {parent(v)}.

Invariant 2 — One parent per non-root:
    For every node v ≠ r: |predecessors(v)| = 1.

Invariant 3 — No cycles:
    Follows from invariants 1 and 2, but worth stating:
    there is no path from any node back to itself.
```

These invariants are checked on every `add_child` call and enforced structurally:
you cannot create a node without specifying its parent (except the root), and you
cannot add an edge that would give a node two parents.

### Why trees appear everywhere

The tree shape emerges any time you have a "belongs to exactly one" relationship:

```
File system:
    /
    ├── home/
    │   └── alice/
    │       ├── documents/
    │       └── photos/
    └── etc/
        └── hosts

HTML document object model (DOM):
    <html>
    ├── <head>
    │   └── <title>My Page</title>
    └── <body>
        ├── <h1>Hello</h1>
        └── <p>World</p>

Abstract Syntax Tree for  x = 1 + 2:
    Assign
    ├── Name("x")     ← left child: the target
    └── BinOp("+")   ← right child: the value
        ├── Num(1)
        └── Num(2)

Organization chart:
    CEO
    ├── CTO
    │   ├── Engineering Manager
    │   └── DevOps Lead
    └── CFO
        └── Controller
```

Notice that the file system does not use a directed graph — a directory cannot appear
in two places at once (hard links aside). The one-parent constraint is what gives trees
their predictability.

### Worked example: AST for `x = 1 + 2`

When a compiler parses the expression `x = 1 + 2`, it builds this tree:

```
        Assign
       /      \
    Name      BinOp
    "x"         "+"
               / \
            Num   Num
             1     2
```

Reading properties from this tree:
- `root` → Assign
- `children(Assign)` → [Name("x"), BinOp("+")]
- `parent(Num(1))` → BinOp("+")
- `depth(Num(1))` → 2  (root is depth 0; BinOp is depth 1; Num(1) is depth 2)
- `height(tree)` → 2  (longest path from root to any leaf)
- `leaves(tree)` → [Name("x"), Num(1), Num(2)]
- `is_leaf(Name("x"))` → True
- `path_to(Num(1))` → [Assign, BinOp("+"), Num(1)]

### Vocabulary

```
Root:       the unique node with no parent
Leaf:       a node with no children (also called external node)
Internal:   a node with at least one child
Parent:     the single node directly above a given node
Children:   all nodes directly below a given node
Siblings:   nodes that share the same parent
Depth:      number of edges from the root to this node
            (root has depth 0)
Height:     longest path from a node down to any leaf below it
            (a leaf has height 0; the tree's height = root's height)
Level:      all nodes at the same depth form a level
Subtree:    a node and all its descendants, which together form a valid tree
            rooted at that node
Ancestor:   any node on the path from the root to this node (not including self)
Descendant: any node reachable by following child edges from this node
```

```
         A           ← root; depth=0; height=3
        / \
       B   C         ← depth=1
      / \   \
     D   E   F       ← depth=2
    /
   G                 ← leaf; depth=3; height=0

height(D) = 1  (D has child G, so path D→G has length 1)
height(B) = 2  (longest path: B→D→G)
height(A) = 3  (longest path: A→B→D→G)
siblings(D) = [E]  (D and E share parent B)
ancestors(G) = [D, B, A]  (path from root excluding G itself)
path_to(G) = [A, B, D, G]
```

### Depth vs height (common point of confusion)

Depth is measured **downward from the root**: how many steps from the root to get to
this node. Height is measured **upward from the leaves**: the longest path from this
node down to any leaf below it.

```
Depth:  root is 0, increases as you go DOWN
Height: leaves are 0, increases as you go UP

A leaf has depth = (distance from root) and height = 0.
The root has depth = 0 and height = (height of the tallest subtree).
```

### Lowest Common Ancestor (LCA)

The **lowest common ancestor** of two nodes a and b is the deepest node that is an
ancestor of both. In a family tree, the LCA of two cousins is their shared grandparent.

```
         A
        / \
       B   C
      / \   \
     D   E   F
    /
   G

LCA(G, E) = B    (B is ancestor of both; no deeper common ancestor exists)
LCA(D, F) = A    (only A is ancestor of both D and F)
LCA(G, D) = D    (D is an ancestor of G, so LCA is D itself)
```

Algorithm: find the path from root to a, and the path from root to b. Walk both paths
together until they diverge — the last shared node is the LCA.

```
lca(tree, a, b):
    path_a = path_to(a)   # [root, ..., a]
    path_b = path_to(b)   # [root, ..., b]

    last_common = root
    for (x, y) in zip(path_a, path_b):
        if x == y:
            last_common = x
        else:
            break
    return last_common
```

Time: O(depth(a) + depth(b)). For balanced trees, this is O(log V). For degenerate
trees (a line), it is O(V). More efficient LCA algorithms exist (DT05 segment tree
can answer LCA in O(log V) after O(V log V) preprocessing) but are deferred to DT05.

### Tree traversals

A traversal visits every node in the tree exactly once. Unlike graph traversal, tree
traversal has well-defined orderings because children are ordered:

#### Preorder: root first, then children (left to right)

"Visit me, then visit each of my subtrees."
Used by: copying a tree, serializing a tree, prefix expression notation.

```
preorder(tree from above example):
A → B → D → G → E → C → F

Rule: visit node, then recursively preorder each child
```

#### Postorder: children first, then root

"Finish all my subtrees, then visit me."
Used by: deleting a tree (can only safely delete a node after its children are deleted),
evaluating an AST (must evaluate operands before operator), dependency compilation
(build dependencies before dependents).

```
postorder:
G → D → E → B → F → C → A

Rule: recursively postorder each child, then visit node
```

#### Level-order (BFS): one full level before the next

"Visit all nodes at depth 0, then depth 1, then depth 2, ..."
Used by: printing a tree level by level, finding the shallowest node with some property,
building a heap from an array.

```
level-order:
A → B → C → D → E → F → G

Level 0: [A]
Level 1: [B, C]
Level 2: [D, E, F]
Level 3: [G]
```

### ASCII visualization

Trees are much easier to understand visually. The `render(tree)` pure function produces
the classic Unix `tree` command format:

```
root
├── child-1
│   ├── grandchild-1
│   └── grandchild-2
└── child-2
    └── grandchild-3
```

Rules for drawing the ASCII tree:
- Each node is on its own line, indented by its depth.
- The last child of a parent uses `└──` (corner). All others use `├──` (tee).
- Descendants of non-last children are indented with `│   ` (vertical bar + spaces).
- Descendants of last children are indented with `    ` (4 spaces — no bar needed).

This visualization function is a pure function that returns a string. It does not
print anything itself — the caller decides what to do with the output.

### The `add_child` contract

The only way to add edges to a tree is through `add_child(parent, child)`. This is
stricter than `DirectedGraph.add_edge(u, v)` — you cannot call `add_edge` directly
because it would bypass the invariant checks:

```
add_child(parent, child) enforces:
  1. parent must already exist in the tree
  2. child must NOT already exist in the tree
     (a node with an existing parent would get a second parent — forbidden)
  3. child cannot be an ancestor of parent
     (would create a cycle)
```

If any of these checks fail, `add_child` raises a ValueError with a descriptive message.

### Subtree extraction

`subtree(tree, node)` returns a new, independent tree rooted at `node`, containing
`node` and all its descendants. The original tree is not modified.

```
Original tree:
    A
    ├── B
    │   ├── D
    │   └── E
    └── C

subtree(tree, B) returns a new tree:
    B
    ├── D
    └── E
```

### remove_subtree

`remove_subtree(node)` removes a node and all its descendants from the tree.
If node is the root, the tree becomes empty. This is the safe way to remove a branch —
you cannot "dangle" nodes by removing only a parent.

## Representation

Tree builds on DirectedGraph. The additional state is minimal:

```
inner:  DirectedGraph[T]   # edges are parent → child
root:   Optional[T]        # None if tree is empty
```

The forward adjacency in `inner` maps parent → children.
The reverse adjacency in `inner` maps child → parent (always size 0 or 1).

There is no separate parent map — `parent(v)` is `list(predecessors(v))[0]` or None.
This keeps the representation DRY at a tiny O(1) lookup cost.

Children are stored in **insertion order** (Python dict ordering, linked list in Rust/Go).
Ordered children are important for deterministic traversals and for rendering.

## Algorithms (Pure Functions)

All algorithms are pure functions. None mutate the tree.

```
# Navigation
parent(tree, node)     → Optional[T]          immediate parent, or None for root
children(tree, node)   → list[T]              ordered list of direct children
siblings(tree, node)   → list[T]              other children of same parent
is_leaf(tree, node)    → bool                 no children
is_root(tree, node)    → bool                 no parent

# Metrics
depth(tree, node)      → int                  edges from root to node
height(tree)           → int                  longest root-to-leaf path
subtree_size(tree, node) → int                count of node + all descendants

# Search
leaves(tree)           → list[T]              all nodes with no children
path_to(tree, node)    → list[T]              [root, ..., node]
lca(tree, a, b)        → T                    lowest common ancestor

# Traversals (all return ordered lists)
preorder(tree)         → list[T]              root, then children L→R recursively
postorder(tree)        → list[T]              children L→R recursively, then root
level_order(tree)      → list[T]              BFS by depth level

# Subtree operations
subtree(tree, node)    → Tree[T]              new independent tree rooted at node
ancestors(tree, node)  → list[T]              [root, ..., parent] (excludes node)

# Visualization
render(tree)           → str                  ASCII tree in `tree` command format
```

## Public API

```python
from typing import Generic, TypeVar, Optional

T = TypeVar("T")  # node type — must be hashable

class Tree(DirectedGraph[T]):
    """
    A tree: directed graph with single root, unique parents, no cycles.

    In Python/Ruby/TypeScript: inherits from DirectedGraph.
    In Rust/Go/Elixir/Lua/Perl/Swift: wraps DirectedGraph via composition.
    """

    def __init__(self) -> None: ...
        # Creates an empty tree with no root.

    @property
    def root(self) -> Optional[T]: ...
        # The unique root node, or None if the tree is empty.

    def set_root(self, node: T) -> None: ...
        # Set the root of an empty tree. Raises if tree is not empty.

    # ── Mutation (all invariant-checked) ──────────────────────────────────

    def add_child(self, parent: T, child: T) -> None: ...
        # Add child as a child of parent.
        # Raises ValueError if parent not in tree, child already in tree,
        # or child is an ancestor of parent (would create cycle).

    def remove_subtree(self, node: T) -> None: ...
        # Remove node and all its descendants. If node is root, tree becomes empty.

    # ── DISALLOWED mutations (inherited from Graph/DirectedGraph) ─────────

    def add_edge(self, u: T, v: T, weight: float = 1.0) -> None: ...
        # RAISES NotImplementedError: use add_child instead.

    def remove_edge(self, u: T, v: T) -> None: ...
        # RAISES NotImplementedError: use remove_subtree instead.

    def add_node(self, node: T) -> None: ...
        # RAISES NotImplementedError: use set_root or add_child instead.
        # (Isolated nodes violate the tree invariant — every non-root needs a parent.)

# ─── Pure function algorithms ────────────────────────────────────────────────

def parent(tree: Tree[T], node: T) -> Optional[T]: ...
def children(tree: Tree[T], node: T) -> list[T]: ...
def siblings(tree: Tree[T], node: T) -> list[T]: ...
def is_leaf(tree: Tree[T], node: T) -> bool: ...
def is_root(tree: Tree[T], node: T) -> bool: ...

def depth(tree: Tree[T], node: T) -> int: ...
def height(tree: Tree[T]) -> int: ...
def subtree_size(tree: Tree[T], node: T) -> int: ...

def leaves(tree: Tree[T]) -> list[T]: ...
def path_to(tree: Tree[T], node: T) -> list[T]: ...
def lca(tree: Tree[T], a: T, b: T) -> T: ...
def ancestors(tree: Tree[T], node: T) -> list[T]: ...

def preorder(tree: Tree[T]) -> list[T]: ...
def postorder(tree: Tree[T]) -> list[T]: ...
def level_order(tree: Tree[T]) -> list[T]: ...

def subtree(tree: Tree[T], node: T) -> Tree[T]: ...

def render(tree: Tree[T]) -> str: ...
    # Returns multi-line string in tree(1) format.
    # Nodes are rendered using str(node).
```

## Composition Model

Tree adds invariant enforcement on top of DirectedGraph. The composition strategy
per language:

- **Python, Ruby, TypeScript** — `class Tree(DirectedGraph[T])`. Override `add_edge`,
  `remove_edge`, and `add_node` to raise `NotImplementedError`. Add `set_root` and
  `add_child` which call the parent class's `add_edge` after passing invariant checks.
  Store `_root: Optional[T]` as an instance variable.

- **Rust** — `struct Tree<T>` wraps `DirectedGraph<T>` and exposes no public `add_edge`.
  The only public mutation methods are `set_root` and `add_child`. The Rust type system
  enforces this statically — no runtime checks needed for the "disallowed" methods.

  ```rust
  pub struct Tree<T: Hash + Eq + Clone> {
      inner: DirectedGraph<T>,
      root: Option<T>,
  }

  impl<T: Hash + Eq + Clone> Tree<T> {
      pub fn set_root(&mut self, node: T) -> Result<(), TreeError> { ... }
      pub fn add_child(&mut self, parent: T, child: T) -> Result<(), TreeError> { ... }
  }
  ```

- **Go** — `type Tree[T comparable] struct { inner DirectedGraph[T]; root *T }`.
  Methods `AddChild` and `SetRoot` are the only mutation entry points.

- **Elixir** — `%Tree{inner: directed_graph, root: nil}` struct. Pure functions
  `set_root/2` and `add_child/3` return updated structs and `{:error, reason}` tuples.

- **Lua, Perl** — Tables with metatables. `add_edge` metamethod raises an error.

- **Swift** — `struct Tree<T: Hashable>` with `var inner: DirectedGraph<T>` and
  `var root: T?`. Only `setRoot` and `addChild` are public.

The pure-function algorithms work identically regardless of which language's
composition/inheritance approach is used — they only call the public read API.

## Test Strategy

- Construction: empty tree has root=None, height=0, zero nodes.
- set_root: sets root on empty tree; raises if tree not empty.
- add_child invariants:
  - adding to a non-existent parent raises ValueError
  - adding a node that is already in the tree raises ValueError
  - adding a node that would create a cycle raises ValueError
  - success case: verify has_node, children(parent) updated, parent(child) correct
- remove_subtree:
  - removing a leaf removes only that node
  - removing an internal node removes it and all descendants
  - removing the root leaves an empty tree
- parent: root returns None; non-root returns correct parent.
- children: leaf returns []; internal node returns children in insertion order.
- siblings: only child returns []; one of two siblings returns the other.
- is_leaf, is_root: spot-check several nodes.
- depth: root=0; child of root=1; verify recursively.
- height: leaf=0; known tree with known height.
- subtree_size: leaf=1; root=total node count.
- leaves: verify returns exactly the leaf nodes.
- path_to: root → [root]; leaf → full path from root.
- lca: same-node case (LCA is that node); sibling case; cousin case; ancestor case.
- ancestors: root → []; child of root → [root]; deeper node → full ancestor list.
- preorder: verify root is first; verify parent before children.
- postorder: verify root is last; verify children before parent.
- level_order: verify BFS order; each level fully processed before next.
- subtree: result is a new independent tree; modifying original doesn't affect subtree.
- render: spot-check output string for a small known tree.
- add_edge / add_node: verify these raise NotImplementedError (invariant protection).
- Integration: build the AST example (x = 1 + 2), run all algorithms, verify results.

## Future Extensions

- **DT03 binary-tree** — Tree with at most 2 children per node (named `left` and `right`).
  Adds left/right child access, in-order traversal (left, root, right), and forms the
  base for all the BST-family structures (DT07–DT12).
- **DT13 trie** — Tree where each edge is labeled with a character. Each path from root
  to a marked node spells out a stored string. Uses `LabeledDirectedGraph` from DT01
  as the underlying structure.
- **DT16 rope** — A binary tree (DT03) where internal nodes store length metadata and
  leaves store string fragments. Used for efficient string concatenation and editing.
- **DT05 segment-tree** — A binary tree with range-aggregate metadata at each internal
  node. The O(V log V) LCA preprocessing mentioned in this spec will be implemented there.
