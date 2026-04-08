# DT10 — Treap

## Overview

A treap is a binary search tree where each node has two properties:
1. A **key** that satisfies the **BST property**: left subtree keys < node key < right subtree keys
2. A **priority** (a random number) that satisfies the **heap property**: every node's priority is greater than its children's priorities

The name is a portmanteau of **tree** and **heap**. It was invented by Raimund
Seidel and Cecilia Aragon in 1989.

### The Central Insight

If you assign each node a uniformly random priority, the resulting tree shape
is **identical** to a random BST — which has expected height O(log n).

Why? Because there is a **unique** treap for any set of (key, priority) pairs.
Given the keys and priorities, the structure is completely determined. The node
with the maximum priority must be the root (heap property), and the keys then
split into the left and right subtrees (BST property) — recursively.

This means you don't need any rotation logic to maintain balance. Random
priorities do the job probabilistically. Expected O(log n) operations with
very high probability.

### Treap vs AVL vs Red-Black

| Property              | AVL (DT08)      | Red-Black (DT09) | Treap (DT10)         |
|-----------------------|-----------------|------------------|----------------------|
| Balance type          | Deterministic   | Deterministic    | Randomized           |
| Height guarantee      | Worst-case O(log n) | Worst-case O(log n) | Expected O(log n) |
| Insert complexity     | O(log n) + rotations | O(log n) ≤ 3 rotations | O(log n) expected |
| Implementation        | Moderate        | Complex          | Simple               |
| Split/Merge support   | Hard            | Hard             | Natural (elegant!)   |
| Persistent variant    | Possible        | Possible         | Very natural         |
| Common use            | Read-heavy DBs  | Linux, Java, C++ | Competitive programming |

The treap's killer feature is **split** and **merge**: two fundamental
operations that make complex set manipulations trivially composable. These
are much harder to implement cleanly in AVL or Red-Black trees.

---

## Layer Position

```
DT02: tree
DT03: binary-tree
DT07: binary-search-tree    ← parent: treap is a BST with heap priorities
DT08: avl-tree              ← sibling: deterministic balance
DT09: red-black-tree        ← sibling: deterministic balance
DT10: treap                 ← [YOU ARE HERE]
DT11: b-tree                ← cousin
DT20: skip-list             ← distant cousin: also randomized O(log n)
```

The **skip list** (DT20) is spiritually related — also a randomized data
structure with expected O(log n) operations. Treaps are often preferred for
ordered-set operations; skip lists are often preferred for concurrent
applications.

---

## Concepts

### Visualizing a Treap

Each node is shown as `(key, priority)`. Higher priority = closer to root.

```
Insert keys [5, 3, 7, 1, 4] with random priorities:

Keys:      5  3  7  1  4
Priorities: 91 53 75 22 68   (random, e.g., from a PRNG)

The node with the highest priority (91) must be the root → (5, 91)

Keys < 5 go left: keys [3, 1, 4] with priorities [53, 22, 68]
Keys > 5 go right: keys [7] with priorities [75]

In the left subtree [1,3,4], highest priority is 68 → root is (4, 68)
  Keys < 4: [1, 3] with priorities [22, 53] → (3, 53) is root
    Keys < 3: [1] → (1, 22) is leaf
  Keys > 4: none

In the right subtree [7], it's a leaf → (7, 75)

Final treap:
             (5, 91)
            /        \
        (4, 68)      (7, 75)
        /
      (3, 53)
      /
   (1, 22)
```

Verify BST property: inorder traversal gives 1, 3, 4, 5, 7 ✓ (sorted)
Verify heap property: every parent priority > children's priorities ✓

### Uniqueness Theorem

**Given a set of distinct (key, priority) pairs, there is exactly one treap
containing them.**

Proof sketch:
- The node with the maximum priority must be the root (heap property says no
  other node can be above it)
- All keys less than the root's key form the left subtree; all greater form the right
- Recursively, each subtree also has a unique treap structure

This means: if two treap implementations insert the same (key, priority) pairs
in any order, they produce identical trees. The structure is deterministic given
the priorities — only the priorities are random.

---

### Split and Merge: The Core Operations

Instead of rotations, treap operations are built from two primitives:

**split(treap, key) → (left_treap, right_treap)**
Divides the treap into two treaps: one with all keys ≤ key, one with all keys > key.

**merge(left_treap, right_treap) → treap**
Combines two treaps into one, assuming every key in left_treap < every key in right_treap.
(The caller is responsible for ensuring this precondition.)

These two operations are more fundamental than rotations because they compose
cleanly. Insert becomes: split at the new key, create a new root, merge back.
Delete becomes: split out the target node, merge the two halves.

#### Split — Detailed Walk-through

```
split(node, key):
  # Returns (left, right) where left contains all keys ≤ key
  # and right contains all keys > key

  if node is None:
    return (None, None)

  if node.key <= key:
    # Current node goes to the LEFT side.
    # Recursively split the RIGHT subtree to separate > key nodes.
    (right_left, right_right) = split(node.right, key)
    new_node = TreapNode(node.key, node.priority, node.left, right_left)
    return (new_node, right_right)
  else:
    # Current node goes to the RIGHT side.
    # Recursively split the LEFT subtree to separate ≤ key nodes.
    (left_left, left_right) = split(node.left, key)
    new_node = TreapNode(node.key, node.priority, left_right, node.right)
    return (left_left, new_node)
```

Visual example: split the treap from above at key=4:

```
Starting treap:
             (5, 91)
            /        \
        (4, 68)      (7, 75)
        /
      (3, 53)
      /
   (1, 22)

split(root=(5,91), key=4):
  5 > 4 → 5 goes RIGHT. Recurse on left child (4,68).

  split(node=(4,68), key=4):
    4 <= 4 → 4 goes LEFT. Recurse on right child (None).

    split(None, 4): return (None, None)

    new_node = (4, 68) with left=(3,53)->(1,22), right=None
    return ( (4,68) subtree, None )

  Back at (5,91): left_left=(4,68) subtree, left_right=None
  new_node = (5,91) with left=None, right=(7,75)
  return ( (4,68) subtree, (5,91) subtree )

Result:
  LEFT:                  RIGHT:
    (4, 68)                (5, 91)
    /                          \
  (3, 53)                    (7, 75)
  /
(1, 22)

Keys ≤ 4: {1, 3, 4} ✓       Keys > 4: {5, 7} ✓
```

#### Merge — Detailed Walk-through

```
merge(left, right):
  # Precondition: all keys in left < all keys in right
  # Returns a single treap preserving BST and heap properties

  if left is None: return right
  if right is None: return left

  if left.priority > right.priority:
    # left's root has higher priority → it stays as root
    # left's right subtree needs to merge with all of right
    new_right = merge(left.right, right)
    return TreapNode(left.key, left.priority, left.left, new_right)
  else:
    # right's root has higher priority → it becomes root
    # right's left subtree needs to merge with all of left
    new_left = merge(left, right.left)
    return TreapNode(right.key, right.priority, new_left, right.right)
```

The heap property guides the choice: whichever root has higher priority
stays on top. Then we recursively merge the "inner" subtrees.

Visual: merge the two treaps from the split:

```
merge( (4,68)->(3,53)->(1,22), (5,91)->(7,75) ):

  left.priority=68 < right.priority=91 → right's root (5,91) stays on top
  merge left subtree with right.left (None):
    → merge( (4,68) subtree, None ) = (4,68) subtree

  Result: (5,91) with left=(4,68) subtree, right=(7,75)
  (Reconstructs the original treap) ✓
```

---

### Insert via Split+Merge

```
insert(node, key, priority):
  (left, right) = split(node, key)
  new_node = TreapNode(key, priority, None, None)
  return merge(merge(left, new_node), right)
```

That's it. Three operations: split, create a singleton treap, merge twice.

The new node gets merged in at a position determined by its priority. If the
priority is the largest in the tree, the new node bubbles up to the root.
If it's the smallest, it sinks to a leaf.

Visual: insert (2, 80) into our treap:

```
Original:                     split at key=2:
     (5, 91)                  LEFT: (1,22)    RIGHT: (3,53)->(4,68)->(5,91)->(7,75)
    /        \
(4, 68)      (7, 75)          merge(LEFT, (2,80)):
    /                         Priority 80 > 22 → (2,80) on top, (1,22) below
  (3, 53)                     → (2,80) with left=(1,22), right=None
  /
(1, 22)

merge( (2,80) subtree, RIGHT ):
  left.priority=80 < right's priority... what IS right's root? (3,53) is the root.
  Wait — let me reconsider. After split at key=2:
  LEFT = (1,22)   RIGHT = treap containing {3,4,5,7} with root (5,91)

  merge( merge((1,22), (2,80)), RIGHT ):
    merge((1,22), (2,80)):
      left.priority=22 < right.priority=80 → (2,80) on top
      merge((1,22), (2,80).left=None) → (1,22)
      return (2,80) with left=(1,22), right=None

  merge( (2,80) subtree, (5,91) subtree ):
    left.priority=80 < right.priority=91 → (5,91) on top
    merge( (2,80) subtree, (4,68) ) where (4,68) is (5,91)'s left child:
      left.priority=80 > right.priority=68 → (2,80) on top
      merge( (2,80).right=None, (4,68) ) → (4,68) subtree
      return (2,80) with left=(1,22), right=(4,68) subtree
    return (5,91) with left=(2,80) subtree, right=(7,75)

Final treap:
            (5, 91)
           /        \
       (2, 80)      (7, 75)
       /     \
   (1, 22)  (4, 68)
            /
          (3, 53)

Inorder: 1,2,3,4,5,7 ✓  Heap property: 91>80>68>53 ✓  91>75 ✓
```

### Delete via Split+Merge

```
delete(node, key):
  (left, right) = split(node, key - ε)   # left has keys < key
  (target, right) = split(right, key)    # target is just the key node (or None)
  # Discard target, merge the two halves
  return merge(left, right)
```

More precisely: split so that left has keys strictly less than `key`, then
split the remainder so that `right` has keys strictly greater than `key`.
Discard the middle piece (which contains only `key`, or is empty if `key`
wasn't in the tree).

---

### Rotation-based Insert (Alternative)

For comparison, here's the traditional rotation-based approach:

```
insert_rotate(node, key, priority):
  if node is None:
    return TreapNode(key, priority, None, None)

  if key < node.key:
    new_left = insert_rotate(node.left, key, priority)
    node = TreapNode(node.key, node.priority, new_left, node.right)
    if node.left.priority > node.priority:
      node = rotate_right(node)   # restore heap property
  elif key > node.key:
    new_right = insert_rotate(node.right, key, priority)
    node = TreapNode(node.key, node.priority, node.left, new_right)
    if node.right.priority > node.priority:
      node = rotate_left(node)    # restore heap property
  # else: duplicate key, no action

  return node
```

The rotation approach is closer to how AVL and Red-Black trees work, but
split/merge is generally preferred for treaps because:
- It generalizes to bulk operations (merge two treaps, split at a range)
- It composes cleanly with persistent/immutable implementations
- The logic is simpler to reason about

---

### is_valid_treap

Verify that both BST and heap properties hold:

```
is_valid_treap(node, min_key=-∞, max_key=+∞, max_priority=+∞):
  if node is None:
    return True

  # BST property: key must be in (min_key, max_key)
  if not (min_key < node.key < max_key):
    return False

  # Heap property: priority must be < parent's priority
  if node.priority > max_priority:
    return False

  return (
    is_valid_treap(node.left,  min_key,   node.key, node.priority) and
    is_valid_treap(node.right, node.key, max_key,  node.priority)
  )
```

---

## Representation

```
TreapNode:
  key:      comparable
  priority: number (typically float or int, randomly assigned)
  left:     TreapNode | None
  right:    TreapNode | None

Treap:
  root: TreapNode | None
  rng:  random number generator (for assigning priorities at insert time)
```

The `rng` (random number generator) should be seeded with a fixed seed for
**deterministic** testing and with a random seed for **production** use.
Some implementations accept an explicit priority at insert time, leaving the
randomization to the caller. This is useful for:
- Reproducible testing: use a known sequence of priorities
- Adversarial resistance: use cryptographically random priorities to prevent
  an attacker from crafting inputs that degrade performance

For a **persistent** (immutable) treap, the `rng` is a pure function returning
`(priority, new_rng_state)` — it threads the RNG state through the call chain.

---

## Algorithms (Pure Functions)

### split

```
split(node, key) → (TreapNode | None, TreapNode | None):
  if node is None:
    return (None, None)
  if node.key <= key:
    (rl, rr) = split(node.right, key)
    return (TreapNode(node.key, node.priority, node.left, rl), rr)
  else:
    (ll, lr) = split(node.left, key)
    return (ll, TreapNode(node.key, node.priority, lr, node.right))
```

Time: O(h) = O(log n) expected.

### merge

```
merge(left, right) → TreapNode | None:
  if left is None: return right
  if right is None: return left
  if left.priority > right.priority:
    return TreapNode(left.key, left.priority, left.left, merge(left.right, right))
  else:
    return TreapNode(right.key, right.priority, merge(left, right.left), right.right)
```

Time: O(h_left + h_right) = O(log n) expected.

### insert

```
insert(treap, key, priority=None):
  if priority is None:
    priority = random_float()
  (left, right) = split(treap.root, key)
  new_node = TreapNode(key, priority, None, None)
  return Treap(merge(merge(left, new_node), right))
```

Time: O(log n) expected.

### delete

```
delete(treap, key):
  (left, rest) = split(treap.root, key - ε)   # strict: keys < key
  # Use a version of split that separates exactly at key:
  # split_strict(node, key) → (keys < key, keys >= key)
  (mid, right) = split_strict(rest, key)       # mid has only key (or is empty)
  return Treap(merge(left, right))

split_strict(node, key):
  # Variant: left has keys strictly < key, right has keys >= key
  if node is None:
    return (None, None)
  if node.key < key:
    (rl, rr) = split_strict(node.right, key)
    return (TreapNode(node.key, node.priority, node.left, rl), rr)
  else:
    (ll, lr) = split_strict(node.left, key)
    return (ll, TreapNode(node.key, node.priority, lr, node.right))
```

Time: O(log n) expected.

### search

Search is identical to BST search (DT07) — priorities are not consulted.

```
search(node, key):
  if node is None: return None
  if key == node.key: return node
  if key < node.key: return search(node.left, key)
  else: return search(node.right, key)
```

Time: O(log n) expected.

---

## Public API (Python-style pseudocode)

```python
class TreapNode:
    key: Any            # comparable
    priority: float     # random, for heap ordering
    left: Optional[TreapNode]
    right: Optional[TreapNode]

class Treap:
    root: Optional[TreapNode]

    @classmethod
    def empty(cls) -> Treap: ...

    # Core operations:
    def insert(self, key: Any, priority: Optional[float] = None) -> Treap:
        """Return new treap with key inserted. Priority is random if not provided."""

    def delete(self, key: Any) -> Treap:
        """Return new treap with key removed (unchanged if not found)."""

    def search(self, key: Any) -> Optional[TreapNode]:
        """Return node with given key, or None."""

    def contains(self, key: Any) -> bool: ...

    # The split/merge API (the real power of treaps):
    def split(self, key: Any) -> tuple[Treap, Treap]:
        """Return (left, right) where left has all keys ≤ key."""

    @staticmethod
    def merge(left: Treap, right: Treap) -> Treap:
        """Merge two treaps. All left keys must be < all right keys."""

    # BST operations (inherited from DT07 logic):
    def min_key(self) -> Any: ...
    def max_key(self) -> Any: ...
    def predecessor(self, key: Any) -> Optional[Any]: ...
    def successor(self, key: Any) -> Optional[Any]: ...
    def kth_smallest(self, k: int) -> Any: ...
    def to_sorted_array(self) -> list: ...

    # Validation:
    def is_valid_treap(self) -> bool:
        """Verify both BST and heap properties hold throughout the tree."""

    def height(self) -> int: ...
    def size(self) -> int: ...
```

---

## Composition Model

### Inheritance languages (Python, Ruby, TypeScript)

Treap extends BST (DT07):

```
BinaryTree (DT03)
  └── BST (DT07)
        ├── AVLTree (DT08)
        ├── RBTree (DT09)
        └── Treap (DT10)     ← adds priority field, overrides insert/delete,
                                adds split/merge
```

Treap overrides `insert` and `delete` with the split/merge implementation.
Inherits `search`, `min_key`, `max_key`, `predecessor`, `successor`,
`to_sorted_array`, `is_valid_bst` from BST.

Adds `split`, `merge`, `is_valid_treap` as new methods.

### Composition languages (Rust, Go, Elixir, Lua, Perl, Swift)

**Rust**: `TreapNode<K: Ord>` with `key: K`, `priority: f64`, optional children
wrapped in `Option<Box<TreapNode<K>>>`. Functions `treap_split`, `treap_merge`,
`treap_insert`, `treap_delete`. The `SearchTree<K>` trait from DT07 implemented.
The RNG can be injected via a `rand::Rng` trait object.

**Go**: `TreapNode[K constraints.Ordered]` struct. Functions `Split`, `Merge`,
`Insert`, `Delete`, `Search`. `SearchTree[K]` interface satisfied. Uses
`math/rand` for priority generation.

**Elixir**: A module with pure functions on `%TreapNode{key, priority, left, right}`.
Pattern matching makes `merge` elegant:

```elixir
def merge(nil, right), do: right
def merge(left, nil), do: left
def merge(%{priority: lp} = left, %{priority: rp} = right) when lp > rp do
  %{left | right: merge(left.right, right)}
end
def merge(left, right) do
  %{right | left: merge(left, right.left)}
end
```

**Swift**: Value type (struct) for a fully immutable treap — insert and delete
return new trees. Or reference type (class) for in-place mutation. The split/merge
design works especially well with value semantics.

**Lua**: Tables as nodes. Functions as closures capturing the RNG. Straightforward
recursive implementation.

**Perl**: Hash references as nodes. Recursive functions. `List::Util::shuffle`
or a custom LCG for priorities.

---

## Test Strategy

### Property-based tests

1. **Both invariants after every operation**: `is_valid_treap(tree)` returns True
   after any sequence of inserts and deletes.

2. **Sort equivalence**: `to_sorted_array(treap)` equals `sorted(keys)` after
   any sequence of inserts on distinct keys.

3. **Split + Merge identity**: `merge(split(treap, k)) == treap` (same set of
   keys and priorities, same structure — since structure is uniquely determined
   by key-priority pairs).

4. **Split correctness**: after `(left, right) = split(treap, k)`, every key
   in left ≤ k and every key in right > k.

5. **Height distribution**: over 1000 random insertions of n=1000 keys,
   the mean height should be within 3σ of `2 × log₂(1001) ≈ 20`. Heights
   much larger than 40 would indicate a bug in priority assignment.

6. **Determinism with fixed seed**: two treaps built with the same RNG seed
   and same insertion order produce identical structures.

### Unit tests

- Insert into empty treap → correct root with given (key, priority)
- Insert node with highest priority → becomes root
- Insert node with lowest priority → becomes leaf
- Delete leaf → parent updated correctly
- Delete root → new root has correct key and priority
- split at min_key → left has one element, right has rest
- split at max_key → left has all, right is empty
- split at non-existent key → split point is correct
- merge(empty, treap) → treap unchanged
- merge(treap, empty) → treap unchanged
- merge two non-overlapping treaps → correct combined treap
- `is_valid_treap` on tree where heap property is violated → False
- `is_valid_treap` on tree where BST property is violated → False
- `is_valid_treap` on a valid treap → True

### Coverage targets

≥ 95% line and branch coverage. Both branches of `merge` (left wins vs right
wins), both branches of `split` (go left vs go right), and all deletion
sub-cases must be exercised.

---

## Future Extensions

1. **Implicit treap** (also called "implicit key treap"): instead of storing
   explicit keys, use the subtree SIZE as an implicit key. This makes the treap
   behave like an array with O(log n) insert, delete, split, and merge at any
   position. Extremely powerful for competitive programming (sequence operations).

2. **Persistent treap**: the purely functional implementation already IS
   persistent — each operation creates O(log n) new nodes. All old versions
   of the treap are accessible. Used for version-controlled sequences.

3. **Lazy propagation**: add a "lazy tag" to each node (like a segment tree)
   that represents a pending operation to be pushed down. Enables range
   assignments (e.g., "set all elements in indices 3-7 to 0") in O(log n).
   Very common in competitive programming.

4. **Randomized mergeable heaps**: replace BST keys with just priorities. This
   gives a randomized priority queue with O(log n) merge — useful for Dijkstra
   with edge relaxation.

5. **Deterministic treap**: replace random priorities with hash(key, secret)
   where secret is a random 64-bit value chosen at construction time. Provides
   expected O(log n) without external randomness and prevents adversarial
   worst-case inputs (the attacker doesn't know the secret).
