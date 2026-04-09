# DT08 — AVL Tree

## Overview

An AVL tree is a self-balancing binary search tree where the heights of the
left and right subtrees of **every node** differ by at most 1. This invariant
is maintained automatically after every insert and delete using **rotations** —
local tree restructurings that preserve the BST property while fixing height
imbalances.

Named after its inventors: **A**delson-**V**elsky and **L**andis, who published
it in 1962. It was the first self-balancing BST ever invented.

The critical guarantee: an AVL tree with n nodes has height at most
`1.44 × log₂(n)`. This means search, insert, and delete are ALL O(log n) in
the **worst case** — unlike the plain BST (DT07) which degenerates to O(n).

### Why Doesn't a Plain BST Guarantee O(log n)?

```
Inserting [1, 2, 3, 4, 5] into a plain BST:

1
 \
  2
   \
    3           ← height = 4, not log₂(5) ≈ 2.3
     \
      4
       \
        5

Searching for 5 requires visiting 5 nodes — O(n), not O(log n).
```

An AVL tree would rebalance after each insert, keeping height ≤ 3 for 5 nodes.

---

## Layer Position

```
DT02: tree
DT03: binary-tree
DT07: binary-search-tree    ← parent: AVL is a BST with the balance invariant
DT08: avl-tree              ← [YOU ARE HERE]
DT09: red-black-tree        ← sibling: different balance mechanism
DT10: treap                 ← sibling: randomized balance
DT11: b-tree                ← cousin: generalization to m children
```

AVL and Red-Black trees (DT09) solve the same problem — guaranteed O(log n) —
but with different trade-offs:

| Property           | AVL (DT08)         | Red-Black (DT09)       |
|--------------------|--------------------|------------------------|
| Balance strictness | Strict (|bf| ≤ 1)  | Loose (≤ 2× log n)     |
| Search speed       | Slightly faster    | Slightly slower        |
| Insert/Delete cost | More rotations     | Fewer rotations        |
| Use case           | Read-heavy         | Write-heavy            |

---

## Concepts

### Height and Balance Factor

The **height** of a node is the length of the longest path from that node down
to any leaf. We define height of an empty tree (None) as -1. A leaf node has
height 0.

```
Height examples:

  4          ← height 2
 / \
2   5        ← height 1 each
 \
  3          ← height 0 (leaf)

height(4) = 1 + max(height(2), height(5))
           = 1 + max(1, 0)
           = 2
```

The **balance factor** of a node is:

```
balance_factor(node) = height(node.left) - height(node.right)
```

- `0`: perfectly balanced at this node
- `+1`: left is one taller (okay)
- `-1`: right is one taller (okay)
- `+2`: left is two taller → **LEFT HEAVY**, needs right rotation
- `-2`: right is two taller → **RIGHT HEAVY**, needs left rotation

An AVL tree requires that every node has balance factor in `{-1, 0, +1}`.

```
VALID AVL (balance factors shown):

        8  [0]
       / \
    3[0]  10[1]
   / \      \
  1   6    14    ← 14 is leaf
     / \
    4   7

height(10) = 1 + max(height(None), height(14)) = 1 + max(-1, 0) = 1
balance_factor(10) = height(None) - height(14) = -1 - 0 = -1  ✓

INVALID (balance factor = +2):

    3  [+2]
   /
  2  [+1]
 /
1  [0]

balance_factor(3) = height(subtree_of_2) - height(None)
                  = 1 - (-1) = 2  ← TOO IMBALANCED! Need a rotation.
```

### Storing Height

Each node caches its own height as an integer field. After any insert or delete,
we recompute the height for every node on the path from the changed node back
to the root, and rebalance any node whose balance factor leaves `{-1, 0, +1}`.

```
Node:
  value:  comparable
  left:   Node | None
  right:  Node | None
  height: int   ← cached, updated after every structural change
```

Recomputing height is O(1) given children's heights:

```
update_height(node):
  node.height = 1 + max(height(node.left), height(node.right))
  # height(None) = -1
```

---

### The Four Rotation Cases

When an imbalance is detected (balance factor reaches ±2), we apply one of
four rotations. The choice depends on **where** the imbalance originated.

Think of it as a 2×2 matrix:

```
         Imbalance side    Insertion side    Rotation needed
Row 1:   Left (+2)         Left subtree      Right rotation (single)
Row 2:   Left (+2)         Right subtree     Left-Right rotation (double)
Row 3:   Right (-2)        Right subtree     Left rotation (single)
Row 4:   Right (-2)        Left subtree      Right-Left rotation (double)
```

#### Case 1: Right Rotation (Left-Left imbalance)

The imbalance is at a node Z, and the "heavy" path goes left, then left again.
We call this the "LL" case.

```
BEFORE rotation:          AFTER right rotation:

      Z [+2]                    Y [0]
     / \                       / \
    Y  [+1]  T3               X   Z [0]
   / \                       / \ / \
  X   T2                    T1 T2 T3
 / \
T1  (T2 is empty or shorter)
```

Step by step: rotate Z to the right, making Y the new root of this subtree.
- Y's right child (T2) becomes Z's left child
- Z becomes Y's right child
- Y becomes the new subtree root

```
Concrete example: nodes 3, 2, 1 inserted in order

After inserting 3, 2:           Insert 1 → imbalance at 3:

    3 [+1]                          3 [+2]   ← balance factor 2!
   /                               /
  2 [0]                           2 [+1]
                                 /
                                1 [0]

Apply right rotation at 3:

    2 [0]                ← 2 is new root
   / \
  1   3                 ← 1 and 3 are leaves, both height 0
```

#### Case 3: Left Rotation (Right-Right imbalance)

Mirror image of Case 1. The heavy path goes right, then right again.

```
BEFORE rotation:          AFTER left rotation:

  Z [-2]                        Y [0]
 / \                           / \
T1   Y [-1]                   Z   X [0]
    / \                      / \ / \
   T2   X                   T1 T2 T3
       / \
      T3  (T3 is empty or shorter)
```

Step by step: rotate Z to the left, making Y the new root.
- Y's left child (T2) becomes Z's right child
- Z becomes Y's left child
- Y becomes the new subtree root

```
Concrete example: nodes 1, 2, 3 inserted in order

After inserting 1, 2:           Insert 3 → imbalance at 1:

  1 [-1]                          1 [-2]   ← balance factor -2!
   \                               \
    2 [0]                           2 [-1]
                                     \
                                      3 [0]

Apply left rotation at 1:

    2 [0]                ← 2 is new root
   / \
  1   3                 ← 1 and 3 are leaves
```

#### Case 2: Left-Right Rotation (Left-Right imbalance)

The imbalance is at Z, the heavy path goes left, then right. A single rotation
won't fix it — we need TWO rotations.

```
BEFORE:                   AFTER left rotate Y:        AFTER right rotate Z:

    Z [+2]                    Z [+2]                       X [0]
   / \                       / \                          / \
  Y [-1] T4                 X   T4                       Y   Z
 / \                       / \                          / \ / \
T1   X                    Y   T3                       T1 T2 T3 T4
    / \                  / \
   T2  T3               T1  T2
```

Step 1: Left-rotate Y (making X the new left child of Z)
Step 2: Right-rotate Z (making X the new root)

```
Concrete example: insert [3, 1, 2]

Insert 3:    Insert 1:       Insert 2 → imbalance at 3:

3 [0]         3 [+1]              3 [+2]   ← LR case! (left child 1 is right-heavy)
             /                   /
            1 [0]               1 [-1]
                                 \
                                  2 [0]

Step 1: Left-rotate 1 (the left child of 3):

    3 [+2]
   /
  2 [+1]     ← 2 is now left child of 3, 1 is left child of 2
 /
1 [0]        ← now this is the LL case!

Step 2: Right-rotate 3:

    2 [0]
   / \
  1   3      ← balanced! ✓
```

#### Case 4: Right-Left Rotation (Right-Left imbalance)

Mirror image of Case 2. Heavy path goes right, then left.

```
BEFORE:                   AFTER right rotate Y:       AFTER left rotate Z:

  Z [-2]                    Z [-2]                       X [0]
 / \                       / \                          / \
T1   Y [+1]               T1   X                       Z   Y
    / \                       / \                     / \ / \
   X   T4                    T2   Y                  T1 T2 T3 T4
  / \                            / \
 T2  T3                         T3  T4
```

Step 1: Right-rotate Y (making X the new right child of Z)
Step 2: Left-rotate Z (making X the new root)

```
Concrete example: insert [1, 3, 2]

Insert 1:    Insert 3:       Insert 2 → imbalance at 1:

1 [0]         1 [-1]              1 [-2]   ← RL case! (right child 3 is left-heavy)
               \                   \
                3 [0]               3 [+1]
                                   /
                                  2 [0]

Step 1: Right-rotate 3 (the right child of 1):

  1 [-2]
   \
    2 [-1]    ← 2 is now right child of 1, 3 is right child of 2
     \
      3 [0]   ← now this is the RR case!

Step 2: Left-rotate 1:

    2 [0]
   / \
  1   3       ← balanced! ✓
```

---

### The Rebalance Procedure

After inserting or deleting a node, walk back up the tree from the affected
node to the root. At each ancestor, update its height and check its balance
factor. If the balance factor is ±2, apply the appropriate rotation.

```
Which rotation to apply:

balance_factor(node) == +2 (left-heavy):
  if balance_factor(node.left) >= 0:    → LL case → rotate_right(node)
  if balance_factor(node.left) < 0:     → LR case → rotate_left(node.left),
                                                      then rotate_right(node)

balance_factor(node) == -2 (right-heavy):
  if balance_factor(node.right) <= 0:   → RR case → rotate_left(node)
  if balance_factor(node.right) > 0:    → RL case → rotate_right(node.right),
                                                      then rotate_left(node)
```

After a single insertion, at most ONE rotation (or one double rotation) is
needed anywhere on the path. After a deletion, rotations may propagate all
the way to the root — but still O(log n) work total.

---

## Representation

```
AVLNode:
  value:  comparable
  left:   AVLNode | None
  right:  AVLNode | None
  height: int   ← must always equal 1 + max(height(left), height(right))
```

Height is an **invariant** on each node. Any function that modifies the tree
must recompute heights on the way back up the call stack (or after the
structural change). Never read a stale height.

The balance factor is derived from height and never stored separately:

```
balance_factor(node):
  return height(node.left) - height(node.right)

height(node):
  if node is None: return -1
  return node.height
```

---

## Algorithms (Pure Functions)

### rotate_right

```
rotate_right(z):
  # z is the imbalanced node (balance_factor == +2)
  # y is z's left child (will become new root)
  y = z.left
  t2 = y.right         # y's right subtree moves to z's left

  # Create new z with t2 as left child
  new_z = AVLNode(z.value, left=t2, right=z.right)
  update_height(new_z)

  # Create new y with new_z as right child
  new_y = AVLNode(y.value, left=y.left, right=new_z)
  update_height(new_y)

  return new_y  # new subtree root
```

### rotate_left

```
rotate_left(z):
  # z is the imbalanced node (balance_factor == -2)
  y = z.right
  t2 = y.left          # y's left subtree moves to z's right

  new_z = AVLNode(z.value, left=z.left, right=t2)
  update_height(new_z)

  new_y = AVLNode(y.value, left=new_z, right=y.right)
  update_height(new_y)

  return new_y
```

### rebalance

```
rebalance(node):
  update_height(node)
  bf = balance_factor(node)

  # Left heavy
  if bf == +2:
    if balance_factor(node.left) < 0:
      # LR case: first fix the left child
      node = AVLNode(node.value, left=rotate_left(node.left), right=node.right)
    return rotate_right(node)  # LL or fixed LR

  # Right heavy
  if bf == -2:
    if balance_factor(node.right) > 0:
      # RL case: first fix the right child
      node = AVLNode(node.value, left=node.left, right=rotate_right(node.right))
    return rotate_left(node)  # RR or fixed RL

  return node  # already balanced
```

### insert

```
insert(node, value):
  if node is None:
    return AVLNode(value, left=None, right=None, height=0)
  if value < node.value:
    new_node = AVLNode(node.value, left=insert(node.left, value), right=node.right)
  elif value > node.value:
    new_node = AVLNode(node.value, left=node.left, right=insert(node.right, value))
  else:
    return node  # duplicate

  return rebalance(new_node)  # rebalance on the way back up
```

### delete

```
delete(node, value):
  if node is None:
    return None  # not found

  if value < node.value:
    new_node = AVLNode(node.value, left=delete(node.left, value), right=node.right)
  elif value > node.value:
    new_node = AVLNode(node.value, left=node.left, right=delete(node.right, value))
  else:
    # Found the node to delete
    if node.left is None:
      return node.right
    if node.right is None:
      return node.left
    # Two children: replace with inorder successor
    succ_value = min_value(node.right)
    new_node = AVLNode(
      succ_value,
      left=node.left,
      right=delete(node.right, succ_value)
    )

  return rebalance(new_node)  # rebalance on the way back up
```

Note: rebalancing happens recursively on the way back up, potentially all
the way to the root. Each `rebalance` call is O(1), and there are O(log n)
levels, so the total work per operation is O(log n).

### balance_factor

```
balance_factor(node):
  if node is None:
    return 0
  return height(node.left) - height(node.right)
```

---

## Public API (Python-style pseudocode)

```python
class AVLNode:
    value: Any
    left: Optional[AVLNode]
    right: Optional[AVLNode]
    height: int

class AVLTree:
    root: Optional[AVLNode]

    # Inherited from BST (DT07) — all still work unchanged:
    def search(self, value: Any) -> Optional[AVLNode]: ...
    def contains(self, value: Any) -> bool: ...
    def min_value(self) -> Any: ...
    def max_value(self) -> Any: ...
    def predecessor(self, value: Any) -> Optional[Any]: ...
    def successor(self, value: Any) -> Optional[Any]: ...
    def kth_smallest(self, k: int) -> Any: ...
    def rank(self, value: Any) -> int: ...
    def to_sorted_array(self) -> list: ...
    def is_valid_bst(self) -> bool: ...

    # AVL-specific overrides (add rebalancing):
    def insert(self, value: Any) -> AVLTree:
        """Return new AVL tree with value inserted and tree rebalanced."""

    def delete(self, value: Any) -> AVLTree:
        """Return new AVL tree with value removed and tree rebalanced."""

    # AVL-specific additions:
    def balance_factor(self, node: AVLNode) -> int:
        """Return height(left) - height(right) for the given node."""

    def is_valid_avl(self) -> bool:
        """Return True if BST invariant AND balance invariant hold."""

    def height(self) -> int:
        """Return the height of the root node (-1 for empty tree)."""
```

---

## Composition Model

### Inheritance languages (Python, Ruby, TypeScript)

AVLTree extends BST (DT07), which extends BinaryTree (DT03):

```
BinaryTree (DT03)
  └── BST (DT07)
        └── AVLTree (DT08)    ← overrides insert, delete; adds rotation methods
              └── ... (could extend further, but usually not)
```

`AVLTree.insert` calls `super().insert` internally is NOT the right pattern
here — instead, AVLTree completely reimplements insert and delete with
rebalancing baked in. The BST's insert/delete logic is replicated in the
recursive helpers.

The BST's search, min, max, predecessor, successor, to_sorted_array, and
is_valid_bst methods all work without modification because they don't depend
on height balance — only on the BST ordering invariant.

### Composition languages (Rust, Go, Elixir, Lua, Perl, Swift)

**Rust**: `AVLNode<T: Ord>` struct adds a `height: i32` field to the BST node.
`avl_insert` and `avl_delete` are standalone functions. A `Balance` trait with
`rotate_left`, `rotate_right`, `rebalance` methods. The `SearchTree<T>` trait
from DT07 is implemented for `AVLNode<T>`.

**Go**: `AVLNode[T constraints.Ordered]` struct. Functions `AVLInsert`,
`AVLDelete`, `RotateLeft`, `RotateRight`, `Rebalance`. The `SearchTree[T]`
interface from DT07 is satisfied by `*AVLNode[T]`.

**Elixir**: A module `AVLTree` with functions operating on a
`%AVLNode{value, left, right, height}` map. Pattern matching on balance factors
makes the rotation cases very clean. All functions are pure (return new trees).

**Swift**: Either value-type (struct) for a fully immutable implementation
returning new trees, or reference-type (class) with in-place mutations. The
immutable variant is more correct for concurrent use.

---

## Test Strategy

### Property-based tests

1. **AVL invariant after every operation**: after any sequence of inserts and
   deletes, `is_valid_avl(tree)` must return `True`.

2. **Height bound**: for a tree with n nodes, `height(tree) ≤ 1.44 × log₂(n + 2)`.

3. **Sort equivalence**: `to_sorted_array(avl_tree)` equals `sorted(values)`
   after any sequence of inserts on distinct values.

4. **Insert-delete cancellation**: inserting then deleting a value leaves the
   tree equal (by structure and values) to the original.

5. **No degenerate case**: inserting `[1, 2, ..., n]` in sorted order still
   produces a tree of height ≈ log n (not n).

### Unit tests

- Right rotation on a LL-imbalanced tree: check new root and child arrangement
- Left rotation on a RR-imbalanced tree: check new root and child arrangement
- LR double rotation: manually insert [3, 1, 2] and verify structure
- RL double rotation: manually insert [1, 3, 2] and verify structure
- All four rotation cases: construct tree, insert trigger value, verify result
- Heights are correctly updated after rotation
- Delete causing rebalance: delete a node that creates an imbalance deeper in
  the tree, verify rebalancing propagates all the way to root
- `balance_factor` on each node of a known tree
- `is_valid_avl` on manually constructed invalid tree → False

### Coverage targets

≥ 95% line and branch coverage. All four rotation cases (LL, RR, LR, RL) must
be exercised by tests. Deletion-triggered rebalance must be covered separately
from insertion-triggered rebalance.

---

## Future Extensions

1. **Augmented AVL**: add `size` per node for order-statistics (kth_smallest,
   rank in O(log n)). Add `sum` per node for range sum queries.

2. **Rank-balanced trees**: a generalization where balance factor bounds are
   widened to ±2. Fewer rotations, slightly worse balance.

3. **Weight-balanced trees**: balance by subtree SIZE rather than HEIGHT. Useful
   when you need rank operations frequently and care about the exact split.

4. **Persistent AVL**: the purely functional (immutable) implementation enables
   path-copying persistence: every version of the tree is available at O(log n)
   extra cost per modification.

5. **Parallel AVL**: bulk insertions can be parallelized by splitting the input
   into chunks, building AVL trees in parallel, and merging. Merge of two AVL
   trees of sizes m, n is O(m log(n/m + 1)).
