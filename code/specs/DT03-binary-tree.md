# DT03 — Binary Tree

## Overview

A binary tree is a tree (DT02) where every node has **at most 2 children**, named
`left` and `right`. This seemingly small constraint — naming the children instead
of just numbering them — unlocks an enormous family of specialized data structures:
binary search trees (DT07), AVL trees (DT08), red-black trees (DT09), and heaps
(DT04) all depend on this left/right distinction to encode ordering.

Think of a regular tree (DT02) as an organizational chart where a manager can
have any number of direct reports. A binary tree is like a tournament bracket —
every match has exactly one winner (left) and one loser (right). The bracket
structure itself becomes meaningful, not just the connections.

### Why "At Most 2"?

The "at most" matters: leaf nodes (nodes with no children) count as valid binary
tree nodes. An empty subtree is represented as `None`/`null`/`nil` — the absence
of a node. This makes recursive algorithms elegant: "process this node, recurse
left, recurse right; base case is None."

### Key Additions Over DT02 (Tree)

DT02 gave us the general tree with unlabeled children. DT03 adds:

- **Typed accessors** — `left_child(node)` / `right_child(node)` rather than
  `children[0]` / `children[1]`
- **Typed mutators** — `add_left_child(parent, child)` / `add_right_child(parent, child)`
- **Shape predicates** — `is_full`, `is_complete`, `is_perfect`
- **Traversal quartet** — `inorder`, `preorder`, `postorder`, `level_order`
- **Array representation** — a bijection between the tree and a flat array,
  critical for heaps (DT04)

## Layer Position

```
DT00: graph (undirected)
DT01: directed-graph
DT02: tree
DT03: binary-tree          ← [YOU ARE HERE]
  ├── DT04: heap
  ├── DT07: binary-search-tree
  │     ├── DT08: avl-tree
  │     ├── DT09: red-black-tree
  │     └── DT10: treap
  ├── DT11: b-tree
  │     └── DT12: b-plus-tree
  └── DT05: segment-tree
        └── DT06: fenwick-tree (logically adjacent)

DT13 onward: trie, radix, suffix, rope, hash structures, skip-list, sketches
DT23-DT25: protocol and application layer (RESP, TCP, mini-Redis)
```

**Depends on:** DT02 (Tree).
**Used by:** DT04 (Heap), DT07 (BST) and all BST variants, DT05 (SegmentTree).

## Concepts

### What a Binary Tree Looks Like

```
          A              ← root (depth 0)
         / \
        B   C            ← depth 1
       / \   \
      D   E   F          ← depth 2
     /
    G                    ← depth 3 (leaf)
```

Each node has:
- A **value** (the data stored at that position)
- A **left** child (or None)
- A **right** child (or None)

Nodes D and G illustrate that "at most 2 children" includes 0 or 1. D has only a
left child. E and F have no children — they are **leaves**.

### Vocabulary

| Term | Meaning |
|---|---|
| Root | The single topmost node (A above) |
| Leaf | A node with no children (G, E, F above) |
| Internal node | A node with at least one child (A, B, C, D) |
| Height | Length of the longest root-to-leaf path. Tree above: 3 (A→B→D→G) |
| Depth of node X | Number of edges from root to X. G has depth 3 |
| Level k | All nodes at depth k |
| Subtree rooted at X | X and all its descendants |

### Full, Complete, and Perfect Trees

These three terms are frequently confused. Here they are side by side:

#### Full Binary Tree: every node has 0 or 2 children (never 1)

```
      A
     / \
    B   C
   / \
  D   E
```

Node C is a leaf (0 children — OK). Nodes A and B each have 2 children (OK).
No node has exactly 1 child. This IS a full binary tree.

```
      A
     / \
    B   C
   /        ← only left child: NOT full
  D
```

Node B has only one child. This is NOT full.

**Why full trees matter:** A full binary tree with n internal nodes has exactly
n+1 leaves. This is a useful invariant for Huffman coding and expression trees
(where every internal node is an operator and every leaf is an operand).

#### Complete Binary Tree: all levels except possibly the last are fully filled;
the last level is filled left-to-right with no gaps

```
Complete:           Not complete (gap before filling right):
      A                   A
     / \                 / \
    B   C               B   C
   / \ /               / \   \
  D  E F              D   E   F
                 ↑ gap here: D's right child missing but F exists
```

The rule: scan the last level left-to-right; once you see a missing child, all
remaining positions must also be empty.

**Why complete trees matter:** They can be stored as flat arrays with no wasted
space and no pointers. This is the foundation of heaps (DT04).

#### Perfect Binary Tree: all internal nodes have 2 children AND all leaves are at the same depth

```
Perfect:
      A
     / \
    B   C
   / \ / \
  D  E F  G
```

A perfect binary tree with height h has exactly `2^(h+1) - 1` nodes. Every
level is completely filled. A perfect tree is both full and complete — but a
full tree is not necessarily perfect, and a complete tree is not necessarily full.

```
Summary matrix:

Property      | Full? | Complete? | Perfect?
--------------|-------|-----------|----------
Every node    | YES   | Not req'd | YES
has 0 or 2    |       |           |
children      |       |           |
              |       |           |
All levels    | Not   | YES (last | YES
fully filled  | req'd | can have  | (all levels)
              |       | gaps)     |
              |       |           |
All leaves    | Not   | Not req'd | YES
at same depth | req'd |           |

Perfect ⊂ Complete ⊂ (not quite right — perfect ⊂ full AND complete)
```

### Array Representation of a Complete Binary Tree

This is the single most important concept in this spec, because it is the
foundation of heaps. A complete binary tree can be stored as a flat array with
a simple index formula — no pointers, no node objects, just numbers in a list.

```
Tree:
          A           (index 0)
         / \
        B   C         (index 1, 2)
       / \ / \
      D  E F  G       (index 3, 4, 5, 6)
     /
    H                 (index 7)

Array: [A, B, C, D, E, F, G, H]
Index:  0  1  2  3  4  5  6  7
```

The formulas:

```
For node at index i (0-indexed):
  Left child:  2*i + 1
  Right child: 2*i + 2
  Parent:      floor((i - 1) / 2)

Let's verify:
  Node B is at index 1.
    Left child of B:  2*1 + 1 = 3  → D  ✓
    Right child of B: 2*1 + 2 = 4  → E  ✓

  Node E is at index 4.
    Parent of E: floor((4-1)/2) = floor(1.5) = 1 → B  ✓

  Node G is at index 6.
    Left child of G:  2*6 + 1 = 13 → beyond array length → no left child  ✓
    Right child of G: 2*6 + 2 = 14 → beyond array length → no right child ✓
```

**Why does this work?** Because the tree is complete — there are no gaps in the
last level (when reading left-to-right). If any position were missing, the indices
of all subsequent nodes would shift, breaking the formula.

**Why is this important?** Heaps (DT04) store their data in exactly this format.
When you "push" an element to a heap, you append it to the end of the array (next
position in level order). When you "pop" the root, you move the last element to
index 0 and restore the heap property. No pointers, no allocations — just array
index arithmetic.

### The Four Traversals

Traversals define an ordering over all nodes. They're all O(n) — every node is
visited exactly once. The difference is WHEN you process the current node relative
to its children.

```
Tree used for examples:
          1
         / \
        2   3
       / \   \
      4   5   6
```

#### Inorder: Left → Root → Right

Process the left subtree first, then the current node, then the right subtree.
Recursively: visit all left descendants, then self, then all right descendants.

```
inorder([1, 2, 3, 4, 5, 6]):
  inorder(2's left = 4) → 4
  visit 2              → 2
  inorder(2's right = 5) → 5
  → left subtree yields [4, 2, 5]
  visit 1              → 1
  inorder(3's left = None) → []
  visit 3              → 3
  inorder(3's right = 6) → 6
  → right subtree yields [3, 6]

Result: [4, 2, 5, 1, 3, 6]
```

**Critical property:** For a Binary Search Tree (DT07), inorder traversal yields
elements in sorted ascending order. This is THE defining property that makes BSTs
useful: the left/right placement of nodes encodes the sorted order of the data.

Why? A BST maintains the invariant: all nodes in a node's left subtree have
smaller values, all nodes in the right subtree have larger values. Inorder visits
"all smaller things first, then me, then all larger things" — which is exactly
sorted order. (More on this in DT07.)

#### Preorder: Root → Left → Right

Process the current node first, then recurse.

```
preorder([1, 2, 3, 4, 5, 6]):
  visit 1              → 1
  preorder(left=2):
    visit 2            → 2
    preorder(left=4):
      visit 4          → 4
    preorder(right=5):
      visit 5          → 5
  preorder(right=3):
    visit 3            → 3
    preorder(left=None):
    preorder(right=6):
      visit 6          → 6

Result: [1, 2, 4, 5, 3, 6]
```

**Use cases:** Preorder produces a "prefix" ordering. It's used for:
- Copying a tree (create parent before children)
- Serializing a tree (write root first so you can reconstruct it)
- Expression trees: preorder of `(a + b) * c` gives `* + a b c` (Polish notation)

#### Postorder: Left → Right → Root

Process children before the current node.

```
postorder([1, 2, 3, 4, 5, 6]):
  postorder(left=2):
    postorder(left=4):
      visit 4          → 4
    postorder(right=5):
      visit 5          → 5
    visit 2            → 2
  postorder(right=3):
    postorder(left=None):
    postorder(right=6):
      visit 6          → 6
    visit 3            → 3
  visit 1              → 1

Result: [4, 5, 2, 6, 3, 1]
```

**Use cases:** Postorder processes children before parents — perfect for:
- Deleting a tree (free children before parent)
- Evaluating expression trees (compute operands before operator)
- Computing subtree sizes (need children's sizes to compute parent's size)

#### Level Order: Breadth-First (left-to-right per level)

Visit all nodes at depth 0, then all at depth 1, then depth 2, etc.
This is NOT recursive — it uses a queue.

```
level_order([1, 2, 3, 4, 5, 6]):
  Queue: [1]
  Dequeue 1, visit 1. Enqueue children 2, 3. Queue: [2, 3]
  Dequeue 2, visit 2. Enqueue children 4, 5. Queue: [3, 4, 5]
  Dequeue 3, visit 3. Enqueue children (only 6). Queue: [4, 5, 6]
  Dequeue 4, visit 4. No children. Queue: [5, 6]
  Dequeue 5, visit 5. No children. Queue: [6]
  Dequeue 6, visit 6. No children. Queue: []

Result: [1, 2, 3, 4, 5, 6]
```

Notice: level order of a complete binary tree gives exactly the array representation
discussed above. That's not a coincidence — level order and array storage are the
same thing.

**Use cases:** Level order is used for:
- Building the array representation of a heap
- Finding the minimum depth of a tree (stop as soon as you hit a leaf)
- Connecting nodes at the same level ("next right pointer" problems)
- Printing a tree level by level for visualization

## Representation

### Node-Pointer Representation

The natural representation uses node objects with left/right pointers:

```
class BinaryTreeNode:
    value: Any
    left:  BinaryTreeNode | None
    right: BinaryTreeNode | None
```

**Advantages:**
- Natural recursive structure
- Easy to insert/delete at arbitrary positions
- Works for any binary tree (full, partial, unbalanced)

**Disadvantages:**
- Memory overhead per node (two pointers + value + object header)
- Poor cache locality (nodes scattered in heap memory)
- Pointer chasing on every traversal step

### Array Representation (Complete Binary Trees Only)

As described above: store nodes in level-order in a flat array. Index arithmetic
replaces pointer chasing.

```python
array[0]          # root
array[2*i + 1]    # left child of node i
array[2*i + 2]    # right child of node i
array[(i-1) // 2] # parent of node i
```

**Advantages:**
- Cache-friendly: children are at predictable nearby indices
- No pointer overhead: just a list of values
- O(1) random access by index

**Disadvantages:**
- Only works for complete binary trees
- "Empty" positions in partial levels waste space
- Insertion/deletion requires careful bookkeeping

**When to use which:** Use node-pointer for BSTs and other search trees where you
need arbitrary insertion/deletion. Use array representation for heaps (DT04) and
segment trees (DT05).

## Algorithms (Pure Functions)

All algorithms take the tree (or its root node) as input and return a result
without modifying the input.

```python
# Traversals — all return a list of values in visitation order

def inorder(root: BinaryTreeNode | None) -> list:
    # Left, Root, Right
    if root is None:
        return []
    return inorder(root.left) + [root.value] + inorder(root.right)

def preorder(root: BinaryTreeNode | None) -> list:
    # Root, Left, Right
    if root is None:
        return []
    return [root.value] + preorder(root.left) + preorder(root.right)

def postorder(root: BinaryTreeNode | None) -> list:
    # Left, Right, Root
    if root is None:
        return []
    return postorder(root.left) + postorder(root.right) + [root.value]

def level_order(root: BinaryTreeNode | None) -> list:
    # Breadth-first using a queue
    if root is None:
        return []
    result = []
    queue  = deque([root])
    while queue:
        node = queue.popleft()
        result.append(node.value)
        if node.left:
            queue.append(node.left)
        if node.right:
            queue.append(node.right)
    return result

# Shape predicates

def is_full(root: BinaryTreeNode | None) -> bool:
    # Every node has 0 or 2 children (never exactly 1)
    if root is None:
        return True
    both = root.left is not None and root.right is not None
    neither = root.left is None and root.right is None
    if not (both or neither):
        return False
    return is_full(root.left) and is_full(root.right)

def is_complete(root: BinaryTreeNode | None) -> bool:
    # All levels full except last; last level filled left-to-right
    # Strategy: level-order scan; once we see a non-full node,
    # all subsequent nodes must be leaves
    if root is None:
        return True
    queue = deque([root])
    seen_incomplete = False
    while queue:
        node = queue.popleft()
        # Check left child
        if node.left:
            if seen_incomplete:
                return False      # child after a gap — not complete
            queue.append(node.left)
        else:
            seen_incomplete = True  # this node has a missing left child
        # Check right child
        if node.right:
            if seen_incomplete:
                return False
            queue.append(node.right)
        else:
            seen_incomplete = True
    return True

def is_perfect(root: BinaryTreeNode | None) -> bool:
    # All leaves at the same depth, all internal nodes have 2 children
    # A perfect tree with height h has exactly 2^(h+1)-1 nodes
    h = height(root)
    n = size(root)
    return n == (2 ** (h + 1)) - 1

def height(root: BinaryTreeNode | None) -> int:
    # Longest root-to-leaf path length (empty tree has height -1)
    if root is None:
        return -1
    return 1 + max(height(root.left), height(root.right))

def size(root: BinaryTreeNode | None) -> int:
    # Total number of nodes
    if root is None:
        return 0
    return 1 + size(root.left) + size(root.right)

def left_child(root: BinaryTreeNode, value) -> BinaryTreeNode | None:
    # Find the node with this value and return its left child
    node = find(root, value)
    return node.left if node else None

def right_child(root: BinaryTreeNode, value) -> BinaryTreeNode | None:
    node = find(root, value)
    return node.right if node else None
```

## Public API

```python
class BinaryTreeNode:
    """A single node in a binary tree."""
    def __init__(self, value, left=None, right=None): ...

class BinaryTree:
    """
    A binary tree rooted at a single BinaryTreeNode.
    All mutation operations return a new BinaryTree (functional style)
    or mutate in place (imperative style) — implementation choice.
    """

    # --- Construction ---
    def __init__(self, root: BinaryTreeNode | None = None): ...

    @classmethod
    def from_level_order(cls, values: list) -> "BinaryTree":
        """Build from a level-order list. None values represent missing nodes."""
        ...

    # --- Shape queries ---
    def is_full(self) -> bool: ...
    def is_complete(self) -> bool: ...
    def is_perfect(self) -> bool: ...
    def height(self) -> int: ...
    def size(self) -> int: ...

    # --- Child accessors ---
    def left_child(self, node: BinaryTreeNode) -> BinaryTreeNode | None: ...
    def right_child(self, node: BinaryTreeNode) -> BinaryTreeNode | None: ...

    # --- Traversals (all return flat lists of values) ---
    def inorder(self) -> list: ...
    def preorder(self) -> list: ...
    def postorder(self) -> list: ...
    def level_order(self) -> list: ...

    # --- Conversion ---
    def to_array(self) -> list:
        """Level-order array. None for missing positions."""
        ...

    # --- Visualization ---
    def to_ascii(self) -> str:
        """Pretty-print the tree structure."""
        ...
```

## Composition Model

Different languages have different mechanisms for code reuse. The DT series uses
**inheritance** in OO languages and **composition/protocol** in functional or
systems languages.

### Inheritance (Python, Ruby, TypeScript)

`BinaryTree` extends `Tree` (DT02). The parent class provides generic tree
operations; `BinaryTree` overrides child accessors with typed left/right versions
and adds the shape predicates and traversals.

```python
# Python
class BinaryTree(Tree):
    def add_left_child(self, parent, child): ...
    def add_right_child(self, parent, child): ...
    def inorder(self) -> list: ...
    def is_complete(self) -> bool: ...
    # ... etc.
```

```typescript
// TypeScript
class BinaryTree extends Tree {
    addLeftChild(parent: BinaryTreeNode, child: BinaryTreeNode): void { ... }
    addRightChild(parent: BinaryTreeNode, child: BinaryTreeNode): void { ... }
    inorder(): unknown[] { ... }
    isComplete(): boolean { ... }
}
```

### Composition (Rust, Go)

No inheritance. Instead, `BinaryTree` contains a generic `Tree` and implements
the `BinaryTree` interface (Go) or the `BinaryTree` trait (Rust).

```go
// Go
type BinaryTree struct {
    root *BinaryTreeNode
}

// BinaryTree does NOT embed Tree — it reuses algorithms from the tree package
// as standalone functions: tree.Height(node), tree.Size(node)

func Inorder(root *BinaryTreeNode) []interface{} { ... }
func IsComplete(root *BinaryTreeNode) bool { ... }
```

```rust
// Rust
pub struct BinaryTree<T> {
    root: Option<Box<BinaryTreeNode<T>>>,
}

// Pure functions in the same module:
pub fn inorder<T: Clone>(root: &Option<Box<BinaryTreeNode<T>>>) -> Vec<T> { ... }
pub fn is_complete<T>(root: &Option<Box<BinaryTreeNode<T>>>) -> bool { ... }
```

### Pure Module (Elixir, Lua, Perl)

No objects at all. The "tree" is a nested tuple/map, and all operations are
functions in a module.

```elixir
# Elixir — trees as {:node, value, left, right} tuples
defmodule BinaryTree do
  def inorder(nil), do: []
  def inorder({:node, val, left, right}) do
    inorder(left) ++ [val] ++ inorder(right)
  end

  def is_complete(root), do: ...
end
```

### Protocol Extension (Swift)

```swift
// Swift — BinaryTreeNode struct + standalone functions
struct BinaryTreeNode<T> {
    let value: T
    let left:  BinaryTreeNode<T>?
    let right: BinaryTreeNode<T>?
}

func inorder<T>(_ root: BinaryTreeNode<T>?) -> [T] { ... }
func isComplete<T>(_ root: BinaryTreeNode<T>?) -> Bool { ... }
```

## Test Strategy

### Shape Predicate Tests

Test `is_full`, `is_complete`, `is_perfect` against hand-crafted examples:

```
Test cases for is_complete:

Complete:             Not complete:
    1                     1
   / \                   / \
  2   3                 2   3
 / \                     \
4   5                     4   ← gap: 3 has no left child but has right child
                               (actually this violates left-to-right fill)

Edge cases: empty tree, single node, two nodes (left only), two nodes (right only)
```

### Traversal Tests

```
For tree:
      1
     / \
    2   3
   / \
  4   5

Expected:
  inorder:    [4, 2, 5, 1, 3]
  preorder:   [1, 2, 4, 5, 3]
  postorder:  [4, 5, 2, 3, 1]
  level_order:[1, 2, 3, 4, 5]
```

### Array Representation Round-Trip

```python
tree = BinaryTree.from_level_order([1, 2, 3, 4, 5, 6, 7])
assert tree.to_array() == [1, 2, 3, 4, 5, 6, 7]
assert tree.level_order() == [1, 2, 3, 4, 5, 6, 7]
```

### Property-Based Tests

- `inorder(tree)` has the same elements as `preorder(tree)` and `postorder(tree)`
  (just different orderings)
- `size(tree) == len(inorder(tree))`
- A perfect tree with height h has `2^(h+1) - 1` nodes
- `is_perfect(tree)` implies `is_complete(tree)` implies `is_full(tree)` — wait,
  that's NOT true (complete does not imply full). Write a test that catches this.

### Coverage Targets

- 95%+ line coverage
- Every branch of `is_complete` covered (full node, left-only, right-only, leaf,
  after-gap scenarios)
- Traversals tested on: empty tree, single node, left-skewed, right-skewed, perfect

## Future Extensions

- **Serialization / deserialization** — serialize to/from JSON, bracket notation,
  level-order list. Needed for DT07 (BST) range queries that return subtrees.
- **Threaded binary trees** — store inorder successor pointers in the null left/right
  fields, enabling O(n) inorder traversal without a call stack or explicit stack.
- **Morris traversal** — O(1) space inorder traversal by temporarily modifying the
  tree and restoring it. Surprising and elegant.
- **Mirror / invert** — swap every left and right child. Useful for DT04 interview
  problems and tree symmetry checks.
- **Lowest common ancestor (LCA)** — given two nodes, find their deepest common
  ancestor. O(h) naive, O(log n) with sparse table preprocessing.
- **Diameter** — longest path between any two nodes (not necessarily through root).
  O(n) with a single postorder pass.
