# DT09 — Red-Black Tree

## Overview

A Red-Black tree is a self-balancing binary search tree where each node is
colored **red** or **black**, and five invariants on those colors guarantee
that the tree height is at most `2 × log₂(n + 1)` — ensuring O(log n) for
all operations in the worst case.

Red-Black trees are arguably the most widely used balanced BST in practice:
- Linux kernel's process scheduler and memory allocator (rbtree.h)
- Java's `TreeMap` and `TreeSet`
- C++ STL's `std::map` and `std::set`
- Python's `sortedcontainers` (SortedList is backed by a list-of-lists, but
  Java/C++ implementations commonly use RB trees)
- Nginx's timer wheel

### Why Red-Black Over AVL?

Both guarantee O(log n). The difference is in the constant factors:

- AVL is **more strictly balanced** (height ≤ 1.44 log n), so lookups are
  slightly faster on average.
- Red-Black requires **fewer rotations** per insert/delete (at most 3 total,
  vs. up to O(log n) for AVL deletes), so writes are faster.

For workloads with many writes (databases, OS schedulers), Red-Black wins.
For read-heavy workloads (lookup tables), AVL may be preferable.

---

## Layer Position

```
DT02: tree
DT03: binary-tree
DT07: binary-search-tree    ← parent: RB tree is a BST with color invariants
DT08: avl-tree              ← sibling: stricter balance, more rotations
DT09: red-black-tree        ← [YOU ARE HERE]
DT10: treap                 ← sibling: randomized balance
DT11: b-tree                ← cousin: Red-Black trees are isomorphic to 2-3-4 trees
```

**Interesting structural fact**: a Red-Black tree is isomorphic to a B-tree
(DT11) of order 4 (also called a 2-3-4 tree). Every "black node with its red
children" in an RB tree corresponds to a node in the B-tree. This is why
Red-Black trees show up in database indexes.

---

## Concepts

### The Five Red-Black Invariants

Every valid Red-Black tree satisfies ALL of the following:

```
1. COLORING:    Every node is either RED or BLACK.
2. ROOT:        The root is BLACK.
3. NULL LEAVES: Every null pointer (NIL leaf) is considered BLACK.
4. RED RULE:    Red nodes can only have BLACK children.
                (No two consecutive red nodes on any root-to-leaf path.)
5. BLACK HEIGHT: Every path from a given node down to any NIL leaf
                 passes through the same number of BLACK nodes.
                 This count is called the node's "black-height".
```

Rules 4 and 5 together bound the tree height. Here's the argument:

- By Rule 5, every root-to-leaf path has the same number of black nodes.
  Call this the tree's black-height `bh`.
- The SHORTEST possible root-to-leaf path is all black nodes: length = `bh`.
- By Rule 4, no two consecutive red nodes exist.
- The LONGEST possible root-to-leaf path alternates red-black: length = `2 × bh`.
- Therefore: `height ≤ 2 × bh ≤ 2 × log₂(n + 1)`.

This is the "looseness" compared to AVL. AVL bounds height to 1.44 log n.
Red-Black allows up to 2 log n. The upside: maintaining the RB invariants
requires fewer structural changes per insert/delete.

### Visualizing a Valid Red-Black Tree

Using `R` for red, `B` for black, `NIL` for null leaves (all black).

```
               7(B)
             /       \
           3(R)       18(R)
          /   \       /   \
        2(B)  4(B) 10(B)  22(B)
        /\    /\   /  \   /  \
      NIL NIL NIL NIL 8(R) 11(R) NIL NIL
                      /\   / \
                    NIL NIL NIL NIL
```

Let's verify all 5 rules:
- Rule 1: every node is R or B ✓
- Rule 2: root (7) is black ✓
- Rule 3: all null leaves are black ✓
- Rule 4: 3(R) has children 2(B) and 4(B) ✓; 18(R) has children 10(B) and 22(B) ✓
- Rule 5: every path from 7 to NIL has exactly 3 black nodes:
  - 7→3→2→NIL: 7(B), 3(R), 2(B), NIL(B) = 3 black ✓
  - 7→3→4→NIL: 7(B), 3(R), 4(B), NIL(B) = 3 black ✓
  - 7→18→10→8→NIL: 7(B), 18(R), 10(B), 8(R), NIL(B) = 3 black ✓
  - 7→18→22→NIL: 7(B), 18(R), 22(B), NIL(B) = 3 black ✓

---

### Insertion: The 5 Cases

When we insert a new node, we always color it **RED** initially. Red is less
disruptive to the black-height rule (Rule 5) — adding a red node doesn't
change any path's black count. However, it might violate Rule 4 (two
consecutive reds) or Rule 2 (root must be black).

We fix violations by a series of **recolorings** and **rotations**, working
our way up the tree. Let `N` = new node, `P` = parent, `G` = grandparent,
`U` = uncle (P's sibling).

```
Case 1: N is the root
  → Color N black. Done. (Fixes Rule 2.)

Case 2: P is black
  → No violation! N is red, P is black → Rule 4 is satisfied.
    No action needed.

Case 3: P is red, U is red  (uncle is red)
  → Recolor: P → black, U → black, G → red.
  → Recurse: G is now red — it might violate Rule 4 with G's parent.
    Treat G as the "new node" and re-apply cases from G upward.

Case 4: P is red, U is black, N is the "inner" child
  (N is right child of P, and P is left child of G)
  (or N is left child of P, and P is right child of G)
  → Rotate P toward G (this is the "outer" direction):
    - If P is left child: rotate P left (N becomes P's parent)
    - If P is right child: rotate P right
  → Now N and P have swapped roles. Apply Case 5 to the old P.

Case 5: P is red, U is black, N is the "outer" child
  (N is left child of P, and P is left child of G)
  (or N is right child of P, and P is right child of G)
  → Rotate G toward U's side (away from P):
    - If P is left child: rotate G right
    - If P is right child: rotate G left
  → Recolor: P → black, G → red.
  → Done. The subtree is now balanced.
```

The key insight: Cases 4 and 5 are geometric. Case 4 transforms an "inner"
child situation into an "outer" child situation with one rotation, then Case 5
finishes with one more rotation and a recolor.

#### Cases 3+5 Combined Diagram (the "outer child" path)

```
Case 3: uncle is red — just recolor
Before:                     After:
       G(B)                       G(R)  ← might propagate up
      / \                        / \
     P(R) U(R)                 P(B) U(B)
    /                          /
   N(R)                       N(R)

Case 5: uncle is black, N is outer child — rotate + recolor
Before (N is left of P, P is left of G):    After:
         G(B)                                   P(B)
        / \                                    / \
       P(R) U(B)                              N(R) G(R)
      / \                                         / \
     N(R) T3                                    T3  U(B)
    / \
   T1  T2

Right-rotate G, color P black, color G red. Done!
```

#### Cases 4→5 Combined Diagram (the "inner child" path)

```
Case 4: N is inner child (right child of left-child P)
Before:                     After Case 4 (left-rotate P):
       G(B)                          G(B)
      / \                           / \
     P(R) U(B)                     N(R) U(B)
    / \                            /
   T1  N(R)                       P(R)
      / \                        /
     T2  T3                    T1
                        (T2, T3 are now N's children)

Now N has become the "outer child" scenario → apply Case 5.
```

---

### Worked Insertion: [7, 3, 18, 10, 22, 8, 11, 26, 2, 6, 13]

This is the classic CLRS (Introduction to Algorithms) textbook example.

```
Step 1: Insert 7
  7 is root → color black.
  Tree:  7(B)

Step 2: Insert 3
  3 < 7, goes left. P=7(B) is black → Case 2, no fix needed.
  Tree:  7(B)
        /
       3(R)

Step 3: Insert 18
  18 > 7, goes right. P=7(B) is black → Case 2, no fix needed.
  Tree:  7(B)
        / \
       3(R) 18(R)

Step 4: Insert 10
  10 > 7, go right to 18. 10 < 18, go left. Place 10(R) as left child of 18.
  P=18(R) is red. U=3(R) is red → Case 3: recolor 18→B, 3→B, 7→R.
  7 is root → Case 1: recolor 7→B.
  Tree:  7(B)
        / \
       3(B) 18(B)
            /
          10(R)

Step 5: Insert 22
  22 > 7 → right; 22 > 18 → right. Place 22(R) as right child of 18.
  P=18(B) is black → Case 2, no fix needed.
  Tree:  7(B)
        / \
       3(B) 18(B)
            / \
          10(R) 22(R)

Step 6: Insert 8
  8 > 7 → right; 8 < 18 → left; 8 < 10 → left.
  Place 8(R) as left child of 10.
  P=10(R) is red. U=22(R) is red → Case 3: recolor 10→B, 22→B, 18→R.
  18's parent is 7(B) → Case 2, no fix needed.
  Tree:  7(B)
        / \
       3(B) 18(R)
            / \
          10(B) 22(B)
          /
         8(R)

Step 7: Insert 11
  11 > 7 → right; 11 < 18 → left; 11 > 10 → right.
  Place 11(R) as right child of 10.
  P=10(B) is black → Case 2, no fix needed.
  Tree:  7(B)
        / \
       3(B) 18(R)
            / \
          10(B) 22(B)
          / \
         8(R) 11(R)

Step 8: Insert 26
  26 > 7→18→22; place 26(R) as right child of 22.
  P=22(B) is black → Case 2, done.

Step 9: Insert 2
  2 < 7→3; place 2(R) as left child of 3.
  P=3(B) is black → Case 2, done.

Step 10: Insert 6
  6 < 7→3; 6 > 3; place 6(R) as right child of 3.
  P=3(B) is black → Case 2, done.

Step 11: Insert 13
  13 > 7→18→10→11; place 13(R) as right child of 11.
  P=11(R) is red. U=8(R) is red → Case 3: recolor 11→B, 8→B, 10→R.
  10's parent is 18(R) which is red → continue fixing up.
  P=18(R) is red. U=3(B) is black. 10 is left child of 18, 18 is right child of 7.
  10 is inner child (left of right-child P) → Case 4: right-rotate 18.
  Now 10 is above 18. 10 is now right child of 7, 18 is right child of 10.
  Now 10's position is "outer child" (right of right-child) relative to G=7 → Case 5:
  Left-rotate 7, recolor 10→B, 7→R.

Final tree:
          10(B)
         /     \
       7(R)    18(R)
      / \      / \
    3(B) 8(B) 11(B) 22(B)
    / \     \    \     \
   2(R) 6(R) (NIL) 13(R) 26(R)
```

---

### Deletion: The Double-Black Problem (Conceptual Overview)

Deletion in a Red-Black tree is notoriously complex — a full treatment would
fill 10-15 pages. Here is the conceptual framework.

When we delete a node, the BST deletion logic (from DT07) is applied first:
replace the node with its inorder successor if it has two children, reducing
to the case of deleting a node with at most one child.

The complication arises when we delete a **black** node with no (or one)
child. Removing a black node decreases the black-height of all paths through
it — violating Rule 5. We conceptually assign the "extra" blackness to the
replacement node (or to a NIL leaf), creating a **double-black** node. A
double-black counts as 2 for black-height purposes, keeping Rule 5 satisfied
temporarily while we fix the violation.

There are 6 cases to resolve double-black, based on the sibling's color and
the sibling's children's colors. Each case either:
- Removes the double-black by redistributing black from the sibling or parent
- Rotates to bring a red node into a position where it can absorb the extra black
- Propagates the double-black upward (reduces to a simpler case)

The important guarantees:
- At most O(log n) recolorings happen
- At most 3 rotations happen total (this is the major advantage over AVL)
- The algorithm terminates and restores all 5 invariants

For a full step-by-step treatment of all 6 deletion cases, see:
- CLRS "Introduction to Algorithms", Chapter 13.4
- Sedgewick's "Left-Leaning Red-Black Trees" (simplified variant)
- Okasaki's "Purely Functional Data Structures" (functional variant)

---

## Representation

```
Color: enum { RED, BLACK }

RBNode:
  value:  comparable
  color:  Color
  left:   RBNode | None   (None is implicitly BLACK per Rule 3)
  right:  RBNode | None

RBTree:
  root: RBNode | None     (None is an empty tree)
```

A common implementation technique: use a sentinel **NIL node** (a fixed black
node with no value) instead of actual `None` values. This simplifies deletion
because you can always ask for `node.color` without null checks.

```
NIL = RBNode(value=None, color=BLACK, left=NIL, right=NIL)  # self-referential sentinel
```

All leaf "pointers" point to this single NIL sentinel rather than to `None`.
This is the approach used in the Linux kernel's rbtree implementation.

For a purely functional (immutable) implementation, sentinel nodes are
inconvenient. Use `None` for empty trees and handle it explicitly.

---

## Algorithms (Pure Functions)

### insert

```
insert(tree, value):
  new_root = insert_recursive(tree.root, value)
  new_root = RBNode(new_root.value, BLACK, new_root.left, new_root.right)
  return RBTree(new_root)

insert_recursive(node, value):
  if node is None:
    return RBNode(value, RED, None, None)   # new nodes start red

  if value < node.value:
    new_node = RBNode(node.value, node.color,
                      insert_recursive(node.left, value), node.right)
  elif value > node.value:
    new_node = RBNode(node.value, node.color,
                      node.left, insert_recursive(node.right, value))
  else:
    return node  # duplicate

  return fix_insertion(new_node)

fix_insertion(node):
  # Check for red-red violations and fix them
  # This is a simplified version of the 5 cases above
  # Applied bottom-up via the recursive call stack
  ... (see the 5 cases in the Concepts section)
```

### delete

```
delete(tree, value):
  new_root = delete_recursive(tree.root, value)
  if new_root is None:
    return RBTree(None)
  new_root = RBNode(new_root.value, BLACK, new_root.left, new_root.right)
  return RBTree(new_root)

delete_recursive(node, value):
  ... (BST delete logic + double-black resolution, 6 cases)
```

### is_valid_rb

Verify all 5 Red-Black invariants:

```
is_valid_rb(tree):
  if tree.root is None:
    return True  # empty tree is valid

  # Rule 2: root must be black
  if tree.root.color != BLACK:
    return False

  # Rules 1, 3, 4, 5 checked recursively
  black_height = check_node(tree.root)
  return black_height != -1  # -1 signals a violation found

check_node(node):
  # Returns black-height of this subtree, or -1 if invariant violated

  if node is None:
    return 1  # NIL nodes are black and contribute 1 to black-height
              # (counting the NIL itself as a black node)

  # Rule 4: red node must have black children
  if node.color == RED:
    if (node.left is not None and node.left.color == RED):
      return -1  # two consecutive reds
    if (node.right is not None and node.right.color == RED):
      return -1

  left_bh  = check_node(node.left)
  right_bh = check_node(node.right)

  if left_bh == -1 or right_bh == -1:
    return -1  # propagate violation

  # Rule 5: black-heights must match
  if left_bh != right_bh:
    return -1

  # Black-height of this node = children's black-height + (1 if this node is black)
  return left_bh + (1 if node.color == BLACK else 0)
```

### black_height

```
black_height(node):
  if node is None:
    return 0
  left_bh = black_height(node.left)
  right_bh = black_height(node.right)
  # In a valid RB tree, left_bh == right_bh
  own = 1 if node.color == BLACK else 0
  return own + left_bh
```

---

## Public API (Python-style pseudocode)

```python
class Color(Enum):
    RED = "RED"
    BLACK = "BLACK"

class RBNode:
    value: Any
    color: Color
    left: Optional[RBNode]
    right: Optional[RBNode]

class RBTree:
    root: Optional[RBNode]

    # Inherited from BST (DT07) — all still work unchanged:
    def search(self, value: Any) -> Optional[RBNode]: ...
    def contains(self, value: Any) -> bool: ...
    def min_value(self) -> Any: ...
    def max_value(self) -> Any: ...
    def predecessor(self, value: Any) -> Optional[Any]: ...
    def successor(self, value: Any) -> Optional[Any]: ...
    def kth_smallest(self, k: int) -> Any: ...
    def to_sorted_array(self) -> list: ...

    # RB-specific operations:
    def insert(self, value: Any) -> RBTree:
        """Return new RB tree with value inserted and invariants restored."""

    def delete(self, value: Any) -> RBTree:
        """Return new RB tree with value removed and invariants restored."""

    def is_valid_rb(self) -> bool:
        """Verify all 5 Red-Black invariants hold."""

    def black_height(self) -> int:
        """Return the black-height of the root (0 for empty tree)."""
```

---

## AVL vs Red-Black Comparison

| Property                 | AVL (DT08)                        | Red-Black (DT09)                    |
|--------------------------|-----------------------------------|-------------------------------------|
| Balance guarantee        | Strict: height ≤ 1.44 × log₂ n   | Loose: height ≤ 2 × log₂(n+1)      |
| Balance mechanism        | Height-based balance factor       | Color-based invariants              |
| Extra storage per node   | `height` (integer)                | `color` (1 bit)                     |
| Search performance       | Slightly faster (shorter tree)    | Slightly slower                     |
| Insert rotations         | Up to O(log n)                    | At most 2                           |
| Delete rotations         | Up to O(log n)                    | At most 3                           |
| Implementation complexity | Simpler (4 rotation patterns)    | More complex (5 insert + 6 delete cases) |
| Ideal workload           | Read-heavy                        | Write-heavy                         |
| Real-world examples      | -                                 | Linux kernel, Java TreeMap, C++ std::map |

### When to Choose Which

Choose **AVL** when:
- Lookups far outnumber inserts/deletes (e.g., an in-memory index built once
  and queried millions of times)
- You need the absolute minimum height (tighter balance = faster search)

Choose **Red-Black** when:
- Inserts and deletes are frequent (e.g., a process scheduler, memory allocator)
- You need to minimize rotation overhead
- You're implementing a general-purpose ordered map/set

---

## Composition Model

### Inheritance languages (Python, Ruby, TypeScript)

Red-Black tree extends BST (DT07):

```
BinaryTree (DT03)
  └── BST (DT07)
        ├── AVLTree (DT08)
        └── RBTree (DT09)   ← overrides insert, delete; adds color logic
```

RBTree reimplements insert and delete with the 5/6 case logic. BST's search,
min, max, traversal methods are inherited unchanged.

### Composition languages (Rust, Go, Elixir, Lua, Perl, Swift)

**Rust**: `RBNode<T: Ord>` with a `Color` enum field. Functions `rb_insert`,
`rb_delete`, `rb_fix_insert`, `rb_fix_delete`. The `SearchTree<T>` trait
from DT07 is implemented for `RBTree<T>`. The sentinel NIL approach is
tricky in Rust with ownership; using `Option<Box<RBNode<T>>>` is cleaner.

**Go**: `RBNode[T constraints.Ordered]` with `Color` type. Functions
`RBInsert`, `RBDelete`, `RBFixInsert`. The `SearchTree[T]` interface satisfied.

**Elixir**: A module with functions operating on `%RBNode{value, color, left, right}`.
Pattern matching on `{color, uncle_color}` tuples makes the insertion cases
very elegant — see Chris Okasaki's famous 4-pattern Haskell implementation
which handles all insertion cases in one function.

**Swift**: Reference types (class) are more natural here since rotations
restructure pointers. A value-type (struct) implementation requires returning
new tree roots and reconstructing the path.

---

## Test Strategy

### Property-based tests

1. **All 5 invariants after every operation**: `is_valid_rb(tree)` returns True
   after any sequence of inserts and deletes.

2. **Black-height consistency**: `black_height(node)` returns the same value
   for the root after every operation (black-height only changes if the tree
   structure changes in a way that removes/adds black levels).

3. **Sort equivalence**: `to_sorted_array` returns sorted values after any
   sequence of operations.

4. **Insert-delete inverse**: inserting then deleting a value restores the
   original structure (note: the SHAPE may differ due to rotations, but the
   sorted order and invariants must hold).

5. **Maximum height bound**: after inserting n random values, `height ≤ 2 × ceil(log₂(n+1))`.

### Unit tests

- Insert into empty tree → root is black (Rule 2)
- Trigger Case 3 (red uncle): verify recoloring propagates correctly
- Trigger Case 4 (black uncle, inner child): verify single rotation happens
- Trigger Case 5 (black uncle, outer child): verify rotation + recolor
- Insert the classic 11-element sequence [7,3,18,10,22,8,11,26,2,6,13] and
  verify the final tree structure matches the expected result
- `is_valid_rb` on a tree with two consecutive reds → False
- `is_valid_rb` on a tree with mismatched black-heights → False
- `black_height` on known tree → correct value
- Delete a red leaf → tree unchanged in structure
- Delete a black leaf with red sibling → correct restructuring
- Delete root → new root is black

### Coverage targets

≥ 95% line and branch coverage. All 5 insertion cases and all 6 deletion
cases must be exercised. The fix-up propagation path (Case 3 cascading to root)
must be covered.

---

## Future Extensions

1. **Left-Leaning Red-Black Tree (LLRB)**: Robert Sedgewick's simplification
   that adds one constraint — red links only lean left. Reduces the number of
   cases for insert/delete significantly. See his 2008 paper.

2. **Persistent Red-Black Tree**: the path-copying technique from functional
   programming creates O(log n) new nodes per insert/delete, keeping all
   previous versions accessible. Used in Clojure's persistent sorted map.

3. **Concurrent Red-Black Tree**: fine-grained locking or lock-free variants
   for multi-threaded use. Much harder than concurrent AVL due to the cascading
   color changes in deletion.

4. **Order-statistics augmentation**: add `size` per node to support rank and
   kth-smallest queries in O(log n). The Linux kernel rbtree doesn't do this
   by default but it's a common extension.

5. **2-3-4 tree isomorphism**: implement a B-tree of order 4 (DT11) and show
   the bijection to Red-Black trees. Each black node with its red children maps
   to one B-tree node. Insightful for understanding why the cases work.
