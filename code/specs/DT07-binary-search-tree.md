# DT07 — Binary Search Tree

## Overview

A Binary Search Tree (BST) is a binary tree (DT03) where every node obeys one
simple rule — the **BST invariant**:

> For every node N:
> - Every value in N's **left** subtree is **strictly less than** N's value
> - Every value in N's **right** subtree is **strictly greater than** N's value

This invariant is what transforms an ordinary binary tree into a search engine.
Instead of scanning all nodes (O(n)), you can navigate directly to any value in
O(log n) time on average — the same way you'd find a word in a dictionary by
opening to the middle, deciding "too early / too late", and halving the search
space each time.

### Where BSTs Sit in the Data Structures Landscape

BSTs are the foundation of a whole family of self-balancing trees:

```
DT02: tree           (general, unordered)
DT03: binary-tree    (at most 2 children, unordered)
DT07: binary-search-tree   [YOU ARE HERE]  ← ordered, O(log n) average
  ├── DT08: avl-tree        ← self-balancing, O(log n) guaranteed
  ├── DT09: red-black-tree  ← self-balancing, fewer rotations
  └── DT10: treap           ← randomized balance
DT11: b-tree         (generalization to m children, used in databases)
```

The BST is the conceptual entry point. Understanding it is prerequisite for
AVL, Red-Black, and Treap trees.

---

## Layer Position

```
DT00: graph
DT01: directed-graph
DT02: tree
DT03: binary-tree         ← parent: BST is a binary tree with an ordering rule
DT04: heap                ← sibling: heap has a different ordering rule
DT05: segment-tree
DT06: fenwick-tree
DT07: binary-search-tree  ← [YOU ARE HERE]
DT08: avl-tree            ← child: BST + height balancing
DT09: red-black-tree      ← child: BST + color-based balancing
DT10: treap               ← child: BST + heap + randomization
```

A **heap** (DT04) and a **BST** are often confused because both maintain an
ordering property. The key difference:

- **Heap**: parent is greater than (or less than) BOTH children. Great for
  finding the max/min in O(1). Terrible for searching an arbitrary value.
- **BST**: left child < parent < right child. Great for searching any value in
  O(log n). Inorder traversal yields sorted output.

---

## Concepts

### The BST Invariant — Valid vs Invalid

```
VALID BST:                         INVALID BST:

        8                                  8
       / \                                / \
      3   10                             3   10
     / \    \                           / \    \
    1   6    14                        1   7    14
       / \                                / \
      4   7                              4   6    ← INVALID! 6 < 7 but is in
                                                    7's right subtree
```

In the invalid tree, `6` appears in the right subtree of `7`. But `6 < 7`, so
it should be in `7`'s LEFT subtree. The invariant is broken.

A common mistake when verifying BSTs: you cannot just check that each node's
left child is smaller and right child is larger. You must check that ALL
descendants on the left are smaller and ALL descendants on the right are larger.

```
TRICKY INVALID BST (passes naive check, fails correct check):

        5
       / \
      3   7
       \
        6     ← 6 > 3 (passes local check) but 6 > 5 (should be in right subtree of 5!)
```

The correct way to verify is to track the valid range `(min_bound, max_bound)`
for each node (see `is_valid_bst` in the Algorithms section).

---

### Why Inorder Traversal Yields Sorted Output

Inorder traversal visits nodes in left → node → right order. For a BST, this
means visiting all values smaller than the current node first, then the current
node, then all values larger. That IS sorted order.

**Informal proof by induction:**
- Base case: a single node. Inorder traversal outputs just that node. Trivially sorted.
- Inductive step: assume inorder of any subtree smaller than T is sorted.
  For tree T with root R:
  - Everything in the left subtree < R (BST invariant)
  - Inorder of left subtree produces sorted output (inductive hypothesis)
  - Then we visit R
  - Everything in the right subtree > R (BST invariant)
  - Inorder of right subtree produces sorted output (inductive hypothesis)
  - So the full output is: sorted_left_values < R < sorted_right_values → sorted ✓

```
      8
     / \
    3   10
   / \    \
  1   6    14
     / \
    4   7

Inorder: 1, 3, 4, 6, 7, 8, 10, 14  ← sorted! ✓
```

---

### Search

Start at the root. At each node, you have three choices:

```
search(node, target):
  if node is None → not found
  if target == node.value → found!
  if target < node.value  → go LEFT (target must be in left subtree)
  if target > node.value  → go RIGHT (target must be in right subtree)
```

Visual example: searching for `4` in the tree above.

```
Start at 8: 4 < 8, go left
  At 3: 4 > 3, go right
    At 6: 4 < 6, go left
      At 4: 4 == 4, FOUND! ✓
```

We visited 4 nodes out of 8 — roughly O(log n).

---

### Insert

Insertion uses the same traversal as search. When you fall off the edge of the
tree (reach a `None` child), that's where the new node goes.

```
Insert 5 into the tree:

Start at 8: 5 < 8, go left
  At 3: 5 > 3, go right
    At 6: 5 < 6, go left
      At 4: 5 > 4, go right
        → None! Place new node here.

Result:
      8
     / \
    3   10
   / \    \
  1   6    14
     / \
    4   7
     \
      5   ← newly inserted
```

In an immutable (functional) implementation, we create new node objects along
the path from root to insertion point, leaving the rest of the tree shared.

---

### Delete — Three Cases

Deletion is the trickiest BST operation because removing a node can disrupt
the tree structure. There are three cases:

**Case 1: Deleting a leaf node (no children)**

Simply remove it. Nothing else changes.

```
Delete 1 from:          Result:
      8                       8
     / \                     / \
    3   10                  3   10
   / \    \                  \    \
  1   6    14                 6    14
     / \                     / \
    4   7                   4   7

Just remove 1. ✓
```

**Case 2: Deleting a node with one child**

Replace the node with its only child. The subtree "slides up".

```
Delete 10 from:         Result:
      8                       8
     / \                     / \
    3   10                  3   14
   / \    \                / \
  1   6    14             1   6
     / \                     / \
    4   7                   4   7

10 had one child (14), so 14 takes 10's place. ✓
```

**Case 3: Deleting a node with two children**

This is the hard case. We cannot just remove the node — the two subtrees need
a new parent.

The trick: find the **inorder successor** — the smallest node in the right
subtree (equivalently, the leftmost node in the right subtree). This value is:
- Larger than everything in the left subtree (it's in the right subtree)
- Smaller than everything else in the right subtree (it's the minimum there)
- Therefore it can legally take the deleted node's place!

Step-by-step: delete `3` from the original tree.

```
Step 1: We want to delete 3. It has two children (1 and 6).

        8
       / \
      3   10        ← want to delete 3
     / \    \
    1   6    14
       / \
      4   7

Step 2: Find the inorder successor of 3.
        That's the minimum of 3's right subtree.
        Go right to 6, then go left as far as possible: 4.
        Inorder successor = 4.

Step 3: Copy the inorder successor's value into the node we're "deleting".

        8
       / \
      4   10        ← copied 4's value here (node was "3")
     / \    \
    1   6    14
       / \
      4   7         ← now we must delete THIS 4

Step 4: Delete the inorder successor from its original position.
        The inorder successor (4) is either a leaf or has a right child only
        (it has no left child, because if it did, that left child would be
        the minimum, not 4). So this is Case 1 or Case 2.

        8
       / \
      4   10        ← node formerly known as 3
     / \    \
    1   6    14
         \
          7         ← 4 has been removed from here ✓
```

We could also use the **inorder predecessor** (largest value in the left
subtree) — either approach maintains the BST invariant.

---

### The Degenerate Case — Why We Need Self-Balancing Trees

What happens when you insert values in sorted order?

```
Insert [1, 2, 3, 4, 5] in order:

1
 \
  2
   \
    3
     \
      4
       \
        5
```

The tree degenerates into a linked list! Search, insert, and delete are all
O(n) — no better than a linear scan. The BST invariant is satisfied, but the
tree provides zero performance benefit.

This is the fundamental weakness of the plain BST. To guarantee O(log n), we
need self-balancing trees:
- **AVL tree** (DT08): maintains height balance after every operation
- **Red-Black tree** (DT09): looser balance guarantee, fewer rotations
- **Treap** (DT10): randomized balance, expected O(log n)

---

## Representation

A BST node stores a value (the key), optional left and right child pointers,
and optionally a parent pointer (useful for traversal without recursion).

```
Node:
  value:  any comparable type
  left:   Node | None
  right:  Node | None

BST:
  root: Node | None
```

The tree is entirely defined by its root node. An empty BST has `root = None`.

For an augmented BST (order statistics tree), each node also stores:
- `size`: count of nodes in this subtree (enables `kth_smallest` and `rank` in O(log n))
- `height`: longest path to a leaf (used by AVL tree, DT08)

In a purely functional (immutable) implementation, "modifying" a node means
creating a new node with the changed fields, and creating new copies of all
ancestor nodes up to the root. The unchanged subtrees are shared between the
old and new tree.

---

## Algorithms (Pure Functions)

All functions below treat the BST as immutable — insert and delete return new
trees rather than modifying in place. This is the right design for Rust, Go,
Elixir, and functional-style implementations.

### search

```
search(node, value):
  if node is None:
    return None
  if value == node.value:
    return node
  if value < node.value:
    return search(node.left, value)
  else:
    return search(node.right, value)
```

Time: O(h) where h is tree height. Average O(log n), worst O(n).

### insert

```
insert(node, value):
  if node is None:
    return Node(value, left=None, right=None)
  if value < node.value:
    return Node(node.value, left=insert(node.left, value), right=node.right)
  if value > node.value:
    return Node(node.value, left=node.left, right=insert(node.right, value))
  else:
    return node  # duplicate — return unchanged (or update, your choice)
```

Time: O(h). Note how we copy nodes along the path (functional style).

### min_value / max_value

```
min_value(node):
  if node is None:
    raise EmptyTreeError
  if node.left is None:
    return node.value   # leftmost node has minimum value
  return min_value(node.left)

max_value(node):
  if node is None:
    raise EmptyTreeError
  if node.right is None:
    return node.value   # rightmost node has maximum value
  return max_value(node.right)
```

Time: O(h).

### delete

```
delete(node, value):
  if node is None:
    return None  # value not found

  if value < node.value:
    return Node(node.value, left=delete(node.left, value), right=node.right)
  if value > node.value:
    return Node(node.value, left=node.left, right=delete(node.right, value))

  # Found the node to delete (value == node.value)
  # Case 1: leaf node
  if node.left is None and node.right is None:
    return None
  # Case 2a: only right child
  if node.left is None:
    return node.right
  # Case 2b: only left child
  if node.right is None:
    return node.left
  # Case 3: two children — find inorder successor
  successor_value = min_value(node.right)
  return Node(
    successor_value,
    left=node.left,
    right=delete(node.right, successor_value)
  )
```

Time: O(h).

### predecessor / successor

The **successor** of a value V is the smallest value in the BST that is
strictly greater than V. The **predecessor** is the largest value strictly
less than V.

```
successor(node, value):
  # Approach: search down the tree, tracking the last time we went left
  result = None
  current = node
  while current is not None:
    if value < current.value:
      result = current.value  # current could be the answer
      current = current.left  # look for something smaller but still > value
    else:
      current = current.right  # need to go right to find larger values

  return result  # None if no value in tree is greater than `value`

predecessor(node, value):
  result = None
  current = node
  while current is not None:
    if value > current.value:
      result = current.value  # current could be the answer
      current = current.right  # look for something larger but still < value
    else:
      current = current.left  # need to go left to find smaller values

  return result  # None if no value in tree is smaller than `value`
```

Time: O(h).

### kth_smallest

Find the k-th smallest value (1-indexed: k=1 returns the minimum).

The naive approach: inorder traversal collecting all values, return index k-1.
Time: O(n), Space: O(n).

The augmented approach (if nodes store subtree size):

```
kth_smallest(node, k):
  if node is None:
    raise IndexError("k out of bounds")
  left_size = size(node.left)  # 0 if node.left is None
  if k == left_size + 1:
    return node.value   # this node is the k-th
  if k <= left_size:
    return kth_smallest(node.left, k)
  else:
    return kth_smallest(node.right, k - left_size - 1)
```

Time: O(h) with size augmentation.

### rank

Find how many values in the BST are strictly less than the given value.

```
rank(node, value):
  if node is None:
    return 0
  if value <= node.value:
    return rank(node.left, value)
  else:
    return size(node.left) + 1 + rank(node.right, value)
    #       ↑ all left nodes   ↑ current node   ↑ matching nodes in right subtree
```

Time: O(h).

### is_valid_bst

Check whether a binary tree satisfies the BST invariant.

The naive approach (check each node against its immediate children) FAILS for
tricky cases like the one shown in the Concepts section. The correct approach
threads a `(min_bound, max_bound)` through the recursion.

```
is_valid_bst(node, min_bound=-∞, max_bound=+∞):
  if node is None:
    return True   # empty tree / leaf boundary is valid

  # The node's value must be strictly within (min_bound, max_bound)
  if not (min_bound < node.value < max_bound):
    return False

  # Left subtree: all values must be < node.value (tighten max_bound)
  # Right subtree: all values must be > node.value (tighten min_bound)
  return (
    is_valid_bst(node.left,  min_bound, node.value) and
    is_valid_bst(node.right, node.value, max_bound)
  )
```

Why this works: every node gets checked against the range of values that are
legally allowed at its position. The constraints propagate down:

```
        5
       / \
      3   7
       \
        6   ← is this valid?

is_valid_bst(5, -∞, +∞)
  → is_valid_bst(3, -∞, 5)      3 is in (-∞, 5) ✓
    → is_valid_bst(6, 3, 5)     6 is NOT in (3, 5) ✗  ← caught!
  → is_valid_bst(7, 5, +∞)      7 is in (5, +∞) ✓
```

Time: O(n) — must visit every node.

### from_sorted_array

Build a balanced BST from a sorted array by always choosing the middle element
as the root. This minimizes height: the root splits the array into two equal
halves, each of which becomes a balanced subtree.

```
from_sorted_array(array, lo=0, hi=len(array)-1):
  if lo > hi:
    return None
  mid = (lo + hi) // 2
  node = Node(array[mid])
  node.left  = from_sorted_array(array, lo, mid - 1)
  node.right = from_sorted_array(array, mid + 1, hi)
  return node
```

Example: `[1, 3, 4, 6, 7, 8, 10, 14]` → balanced BST:

```
         6
       /   \
      3     10
     / \   /  \
    1   4 8    14
         \  \
          7   (empty)
     Wait, let me redo this properly.

Array: [1, 3, 4, 6, 7, 8, 10, 14]  (indices 0–7)
mid = 3 → value 6 (root)

Left half:  [1, 3, 4]  (indices 0–2), mid=1 → value 3
  Left:  [1]           (index 0),  mid=0 → value 1 (leaf)
  Right: [4]           (index 2),  mid=2 → value 4 (leaf)

Right half: [7, 8, 10, 14] (indices 4–7), mid=5 → value 8
  Left:  [7]           (index 4),  mid=4 → value 7 (leaf)
  Right: [10, 14]      (indices 6–7), mid=6 → value 10
    Right: [14]        (index 7),  mid=7 → value 14 (leaf)

Result:
         6
       /   \
      3     8
     / \   / \
    1   4 7   10
               \
               14
```

Height = 3 for 8 elements. A degenerate BST from sorted insertion would have
height 7. This is the maximum compression possible.

Time: O(n). Space: O(n).

### to_sorted_array

Simply perform inorder traversal and collect values.

```
to_sorted_array(node, result=[]):
  if node is None:
    return
  to_sorted_array(node.left, result)
  result.append(node.value)
  to_sorted_array(node.right, result)
  return result
```

Time: O(n). Space: O(n).

---

## Public API (Python-style pseudocode)

```python
class BSTNode:
    value: Any
    left: Optional[BSTNode]
    right: Optional[BSTNode]

class BST:
    root: Optional[BSTNode]

    @classmethod
    def empty(cls) -> BST: ...

    @classmethod
    def from_sorted_array(cls, array: list) -> BST: ...

    def insert(self, value: Any) -> BST:
        """Return a new BST with value inserted."""

    def delete(self, value: Any) -> BST:
        """Return a new BST with value removed (unchanged if not found)."""

    def search(self, value: Any) -> Optional[BSTNode]:
        """Return the node with the given value, or None."""

    def contains(self, value: Any) -> bool:
        """Return True if value is in the BST."""

    def min_value(self) -> Any:
        """Return the smallest value. Raises EmptyTreeError if empty."""

    def max_value(self) -> Any:
        """Return the largest value. Raises EmptyTreeError if empty."""

    def predecessor(self, value: Any) -> Optional[Any]:
        """Return the largest value strictly less than `value`."""

    def successor(self, value: Any) -> Optional[Any]:
        """Return the smallest value strictly greater than `value`."""

    def kth_smallest(self, k: int) -> Any:
        """Return the k-th smallest value (1-indexed)."""

    def rank(self, value: Any) -> int:
        """Return count of values strictly less than `value`."""

    def to_sorted_array(self) -> list:
        """Return all values in sorted order via inorder traversal."""

    def is_valid(self) -> bool:
        """Return True if BST invariant holds throughout the tree."""

    def height(self) -> int:
        """Return the height of the tree (0 for single node, -1 for empty)."""

    def size(self) -> int:
        """Return the total number of nodes."""
```

---

## Composition Model

### Inheritance languages (Python, Ruby, TypeScript)

BST extends BinaryTree (DT03), which extends Tree (DT02):

```
Tree (DT02)
  └── BinaryTree (DT03)
        └── BST (DT07)
              ├── AVLTree (DT08)
              └── ... (DT09, DT10)
```

BST inherits traversal methods (`inorder`, `preorder`, `postorder`, `height`,
`size`) from BinaryTree. It adds ordering-aware methods: `search`, `insert`,
`delete`, `min_value`, `max_value`, `predecessor`, `successor`, etc.

AVLTree (DT08) extends BST and overrides `insert` and `delete` to include
rebalancing. It also adds `balance_factor`, `rotate_left`, `rotate_right`.

### Composition languages (Rust, Go, Elixir, Lua, Perl, Swift)

These languages use composition or protocols instead of inheritance:

**Rust**: `BSTNode<T: Ord>` is a struct. Functions like `bst_insert`,
`bst_delete`, `bst_search` take `Option<Box<BSTNode<T>>>` and return the
same type. The BST "interface" is a trait `SearchTree<T>` that AVL and
Red-Black also implement.

**Go**: `BSTNode[T constraints.Ordered]` struct with `Insert`, `Delete`,
`Search` functions. An interface `SearchTree[T]` with those methods. AVL
wraps BST logic and adds rotation state.

**Elixir**: Pure functions on a `%BSTNode{value, left, right}` struct.
Pattern matching makes the three deletion cases elegant.

**Swift**: `BSTNode<T: Comparable>` class with mutating methods, or a
functional variant using value types and returning new trees.

---

## Test Strategy

### Property-based tests (most important)

1. **BST invariant after every operation**: after any sequence of inserts and
   deletes, `is_valid_bst(tree)` must return `True`.

2. **Round-trip via sorted array**: `to_sorted_array(from_sorted_array(arr))`
   must equal `arr` for any sorted input.

3. **Sort equivalence**: `to_sorted_array(insert_all(empty, values))`
   must equal `sorted(values)` for any list of distinct values.

4. **Rank and kth_smallest inverse**: `kth_smallest(tree, rank(tree, v) + 1) == v`
   for any value v in the tree.

5. **Delete then search**: after `tree2 = delete(tree, v)`, `search(tree2, v)`
   must return `None`.

### Unit tests

- `search` on empty tree → `None`
- `search` for present value → correct node
- `search` for absent value → `None`
- `insert` duplicate → tree unchanged (or same size)
- `delete` leaf → parent's child pointer is `None`
- `delete` one-child node → child slides up correctly
- `delete` two-child node → inorder successor takes its place
- `min_value` / `max_value` on empty tree → error
- `min_value` on single node → that node's value
- Degenerate tree (sorted insertion): `height` equals `n - 1`
- `from_sorted_array([1..n])` produces a tree of height `floor(log2(n))`
- `is_valid_bst` on a manually constructed invalid tree → `False`
- `predecessor` / `successor` on minimum / maximum → `None`
- `kth_smallest(tree, 1)` equals `min_value(tree)`
- `kth_smallest(tree, size(tree))` equals `max_value(tree)`

### Coverage targets

Target ≥ 95% line and branch coverage. Every deletion case (leaf, one child,
two children) must be exercised. Both left-heavy and right-heavy trees must be
tested.

---

## Future Extensions

1. **Order-statistics tree**: augment each node with `size` (count of nodes in
   subtree) to support `kth_smallest` and `rank` in O(log n) without scanning.

2. **Interval tree**: store intervals `[lo, hi]` instead of scalar values.
   Augment with `max_hi` per subtree. Enables O(log n) stabbing queries
   ("which intervals contain point x?").

3. **Persistent BST**: fully functional/immutable implementation enables O(log n)
   path-copying persistence — you keep old versions of the tree after mutations.
   Foundation for persistent data structures in functional languages.

4. **Threaded BST**: add "thread" pointers so inorder traversal can be done
   without a stack or recursion (each leaf's right pointer points to its inorder
   successor instead of None).

5. **Self-balancing variants**: see DT08 (AVL), DT09 (Red-Black), DT10 (Treap)
   for trees that guarantee O(log n) by maintaining height balance invariants.
