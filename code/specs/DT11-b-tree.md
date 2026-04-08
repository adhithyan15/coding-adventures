# DT11 — B-Tree

## Overview

A B-tree generalizes the binary search tree (DT07) to nodes with **M children**
instead of 2. The letter "B" likely stands for "Bayer" (its inventor Rudolf Bayer)
or "balanced" — the tree is always perfectly height-balanced: every leaf sits at
the same depth.

B-trees were designed in 1970 for one specific purpose: **minimizing disk reads**.
When your data lives on a spinning hard drive or SSD, reading a block of 4 KB or
8 KB costs the same time whether you read 1 byte or all 8 KB. So you might as well
pack many keys into each node and read hundreds of keys per disk access. A B-tree
with M=1000 on a billion records is only ~3 levels tall. You find any record with
at most 3 disk reads.

Real-world uses: SQLite (tables and indexes), filesystem metadata (NTFS, HFS+, ext4),
some NoSQL engines, and many embedded databases.

## Layer Position

```
DT00: graph
DT01: directed-graph
DT02: tree
DT03: binary-tree
DT07: binary-search-tree   ← B-tree generalizes this concept
DT08: avl-tree             (sibling: self-balancing BST, 2 children)
DT09: red-black-tree       (sibling: self-balancing BST, 2 children)
DT10: treap                (sibling: randomized BST, 2 children)
DT11: b-tree               ← [YOU ARE HERE] (M children, disk-oriented)
  └── DT12: b-plus-tree    (variant: all data at leaves, linked leaf list)

DT13: trie                 (different lineage: string-keyed tree)
DT14: radix-tree
```

**Depends on:** DT07 (BST concepts: sorted keys, range queries, inorder traversal).
**Used by:** DT12 (B+ tree extends this design), database storage engines,
filesystem directory trees.

## Concepts

### Why Binary Trees Are Bad for Disk Storage

A binary search tree on 1 billion records has height ~log₂(1,000,000,000) ≈ 30.
Each level visit requires reading one node. If each node is on a different disk page,
that is 30 disk reads per lookup. A modern SSD can do ~100,000 reads per second,
so 30 reads takes ~300 microseconds. A billion such lookups would take 83 hours.

The problem is not the algorithm — O(log n) is optimal for comparison-based search.
The problem is the **page size mismatch**. Each disk read fetches 4–16 KB, but a
BST node holds only one key. You paid for 4000 bytes and used 8.

**The fix:** put more keys in each node. If each node holds 1000 keys, the tree is
only 3 levels tall (1000³ = 1 billion). Now a lookup costs 3 disk reads, not 30.
That is the entire motivation for B-trees.

```
Binary tree on 1 billion records:
  Level 0: 1 node          ← root read #1
  Level 1: 2 nodes         ← read #2
  Level 2: 4 nodes         ← read #3
  ...
  Level 30: 1B nodes       ← read #30  ← 30 disk reads per lookup!

B-tree (order 1000) on 1 billion records:
  Level 0: 1 node (holds ~1000 keys)  ← root read #1
  Level 1: ~1000 nodes                ← read #2
  Level 2: ~1M nodes                  ← read #3  ← done!
```

### Order and Minimum Degree

There are two equivalent ways to parameterize a B-tree. This spec uses **minimum
degree t** (Cormen/CLRS convention), which is cleaner for implementation.

```
Given minimum degree t (t ≥ 2):
  - Every non-root node has at least t-1 keys
  - Every non-root node has at most 2t-1 keys
  - Every internal node with k keys has k+1 children
  - All leaves are at the same depth
  - The root has at least 1 key (unless the tree is empty)

A B-tree of minimum degree t=2 is called a 2-3-4 tree
(non-root nodes have 1, 2, or 3 keys — equivalently 2, 3, or 4 children).

The "order" M you sometimes see in textbooks is M = 2t (the max number of children).
```

### Node Structure

```
                   ┌─────────────────────────────────────────┐
  internal node:   │  C0 │ K1 │ C1 │ K2 │ C2 │ K3 │ C3 │ V3 │
                   └─────────────────────────────────────────┘
                        ↓       ↓       ↓       ↓
                    subtree  subtree  subtree  subtree
                    with     between  between  with
                    keys     K1..K2   K2..K3   keys
                    < K1                       > K3

  leaf node:       │ K1 │ V1 │ K2 │ V2 │ K3 │ V3 │
                   (no children pointers)
```

Each internal node with k keys has k+1 children pointers. The children are subtrees
containing keys that fall in the intervals defined by the node's keys:

```
For a node with keys [20, 40, 60]:

  C0            C1           C2           C3
keys < 20   20 ≤ k < 40   40 ≤ k < 60   keys ≥ 60
```

In a B-tree (as opposed to a B+ tree), internal nodes also store (key, value) pairs.
When a key matches an internal node key, you found the record — no need to descend to a leaf.

### A Concrete Example: B-Tree with t=2

With t=2 (minimum degree 2), non-root nodes hold 1–3 keys and 2–4 children.

```
Insert keys 1..10 into an empty B-tree (t=2):

After inserting 1, 2, 3:
     [1, 2, 3]       ← root is a leaf; still has room (max 3 keys)

Insert 4: root is full (3 keys). Split root!
     [2]              ← median key 2 promoted to new root
    /   \
  [1]   [3, 4]       ← left child has keys < 2; right has keys > 2

Insert 5, 6:
     [2]
    /   \
  [1]   [3, 4, 5, 6] ← right child full (4 keys = 2t)

Wait — with t=2, max keys = 2t-1 = 3. So 4 keys triggers a split.
After insert 5 the right child [3, 4, 5] is full.

Insert 6: right child [3, 4, 5] overflows to [3, 4, 5, 6].
Split: median 4 promoted. Root becomes [2, 4].
     [2,  4]
    /   |   \
  [1] [3]  [5, 6]

Insert 7, 8, 9, 10 (showing final result):
          [4]
         /   \
      [2]     [6,  8]
     /   \   /  |   \
   [1]  [3] [5] [7] [9, 10]
```

### Search

Search is exactly like BST search, but at each node you binary-search among multiple
keys to decide which child to follow.

```
search(node, target):
  i = 0
  while i < len(node.keys) and target > node.keys[i]:
      i += 1
  if i < len(node.keys) and target == node.keys[i]:
      return node.values[i]   ← found in internal node
  if node.is_leaf:
      return None             ← not in tree
  return search(node.children[i], target)
```

```
Search for key 7 in the tree above:
  Root [4]: 7 > 4, follow child[1] which is [6, 8]
  Node [6, 8]: 7 > 6 and 7 < 8, follow child[1]
  Leaf [7]: 7 == 7. Found!
  Total: 3 node visits = 3 disk reads.
```

### Insertion

B-tree insertion maintains the invariant by **splitting full nodes on the way down**.
This "proactive splitting" approach avoids backtracking.

**Step 1:** If the root is full, split it first (the tree grows taller by 1).

**Step 2:** Walk down toward the leaf where the key belongs, splitting any full
child before descending into it.

**Step 3:** Insert into the leaf (which is guaranteed not full because we split it
if it was full on the way down).

```
Split a full child:

Before:               After splitting child C1 at median key M:

  parent: [A, B]          parent: [A, M, B]
  /   |   \              /   |   |    \
 C0  C1   C2           C0  left right  C2
     |
  [K1, M, K3]     left = [K1]     right = [K3]
  (2t-1 keys)     (t-1 keys)      (t-1 keys)

The median M is promoted to the parent.
Left half stays as the child; right half becomes a new sibling.
```

**ASCII split example with t=2:**

```
Before insert 8 — node [5, 7] is full (3 keys = 2t-1):
          [4]
         /   \
      [2]     [6,  8]   ← already split... let me show an explicit example

Inserting key 3 into tree:
          [4]
         /   \
      [2]     [6, 8]
     /   \
   [1]   [3]       ← 3 inserted into leaf

Now insert 2.5 (hypothetically):
  Walk to [1, 3] leaf — full? No (only 2 keys, max is 3). Insert directly.
  [1, 2.5, 3]

Now insert 1.5:
  Walk toward [1, 2.5, 3] — it's full (3 keys = 2t-1 = 3).
  Split before descending!
  Median of [1, 2.5, 3] is 2.5. Promote 2.5 to parent [2].
  Parent becomes [2, 2.5].
  Left child: [1].   Right child: [3].
  Now descend into [1] and insert 1.5 → [1, 1.5].
```

### Deletion

Deletion has three main cases depending on where the key is found:

```
Case 1: Key k is in a LEAF node with ≥ t keys.
  → Delete directly. The leaf still has ≥ t-1 keys. Done.

     [2, 4, 6]   delete 4   →   [2, 6]   ✓

Case 2: Key k is in an INTERNAL node.
  Sub-case 2a: Left child of k has ≥ t keys.
    → Replace k with its inorder predecessor (the rightmost key in the left subtree).
      Then delete the predecessor from the left subtree.

  Sub-case 2b: Right child of k has ≥ t keys.
    → Replace k with its inorder successor (the leftmost key in the right subtree).
      Then delete the successor from the right subtree.

  Sub-case 2c: Both children have exactly t-1 keys.
    → Merge the two children with k into one node (size 2t-1).
      Delete k from the merged node.

Case 3: Key k is not in the current node (we're routing to a child).
  If the target child has only t-1 keys (minimum), we must "fill" it first:

  Fill strategy A — Rotate right (borrow from left sibling):
    If left sibling has ≥ t keys:
    → Move the separator key from parent down into the child.
      Move the rightmost key from left sibling up into the parent.

  Fill strategy B — Rotate left (borrow from right sibling):
    If right sibling has ≥ t keys:
    → Move the separator key from parent down into the child.
      Move the leftmost key from right sibling up into the parent.

  Fill strategy C — Merge:
    If both siblings have exactly t-1 keys:
    → Merge the child with one sibling and the separator key from the parent.
      The parent loses one key and one child pointer.
```

**Merge before/after:**

```
Before merge (t=2, merging child C1 with sibling C2):

  parent: [A, SEP, B]
          /   |    \
        C0   C1    C2
             |      |
           [K1]   [K2]    ← each has t-1=1 key (minimum)

After merge:

  parent: [A, B]           ← SEP removed from parent
          /   \
        C0    merged
               |
           [K1, SEP, K2]  ← merged node has 2t-1 = 3 keys

If parent now has fewer than t-1 keys, propagate the fix upward.
```

### Height and Performance

```
For n keys and minimum degree t:
  Minimum height:  h ≥ log_{2t}(n+1) - 1
  Maximum height:  h ≤ log_t((n+1)/2)

At t=1000, a B-tree on 1 billion keys:
  max height = log_1000(500,000,000) ≈ 3.0   → at most 3 levels!

Operation complexities:
  Search:   O(t · log_t n)  — at each of O(log_t n) nodes, binary search among O(t) keys
  Insert:   O(t · log_t n)
  Delete:   O(t · log_t n)
  Range:    O(t · log_t n + k)  where k is number of results

Disk reads (what actually matters):
  Each operation visits O(log_t n) nodes = O(log_t n) disk reads.
  With t=1000 and n=10^9: ≈ 3 disk reads. With t=2 (2-3-4 tree): ≈ 30 disk reads.
```

## Representation

### Node

```python
@dataclass
class BTreeNode:
    keys:     list[Any]          # sorted list of keys; length is 1..2t-1
    values:   list[Any]          # parallel to keys; values[i] belongs to keys[i]
    children: list["BTreeNode"]  # length is 0 (leaf) or len(keys)+1 (internal)
    is_leaf:  bool

    def is_full(self, t: int) -> bool:
        return len(self.keys) == 2 * t - 1
```

### Tree

```python
@dataclass
class BTree:
    root: BTreeNode | None
    t:    int          # minimum degree; t ≥ 2
    size: int          # total number of keys stored
```

### Space Complexity

```
Each node: O(t) keys, O(t) values, O(t) children → O(t) per node.
Number of nodes: O(n / t).
Total space: O(n) — proportional to number of keys regardless of t.
```

## Algorithms (Pure Functions)

All algorithms are expressed as pure transformations. In practice, B-trees are
often mutated in place (for efficiency), but the logic is the same.

```python
# ─── Search ────────────────────────────────────────────────────────────────

def search(tree: BTree, key: Any) -> Any | None:
    """
    Return the value for key, or None if key not in tree.
    Descends the tree, binary-searching at each node.
    Time: O(t · log_t n).
    """
    if tree.root is None:
        return None
    return _search_node(tree.root, key)

def _search_node(node: BTreeNode, key: Any) -> Any | None:
    # Find the first index where keys[i] >= key
    i = 0
    while i < len(node.keys) and key > node.keys[i]:
        i += 1
    if i < len(node.keys) and key == node.keys[i]:
        return node.values[i]     # found right here
    if node.is_leaf:
        return None               # reached a leaf, not found
    return _search_node(node.children[i], key)

# ─── Insert ────────────────────────────────────────────────────────────────

def insert(tree: BTree, key: Any, value: Any) -> BTree:
    """
    Return a new BTree with (key, value) inserted.
    Splits full nodes top-down to ensure leaf always has room.
    Time: O(t · log_t n).
    """
    t = tree.t
    if tree.root is None:
        return BTree(
            root=BTreeNode(keys=[key], values=[value], children=[], is_leaf=True),
            t=t,
            size=1,
        )

    root = tree.root
    if root.is_full(t):
        # Root is full — split it. Tree height increases by 1.
        new_root = BTreeNode(keys=[], values=[], children=[root], is_leaf=False)
        new_root = _split_child(new_root, 0, t)
        new_root = _insert_non_full(new_root, key, value, t)
    else:
        new_root = _insert_non_full(root, key, value, t)

    # Determine if we actually added a new key or replaced an existing one
    old_size = tree.size
    new_size = old_size + (0 if _search_node(new_root, key) is not None and
                              _search_node(tree.root, key) is not None else 1)
    return BTree(root=new_root, t=t, size=new_size)

def _insert_non_full(node: BTreeNode, key: Any, value: Any, t: int) -> BTreeNode:
    """Insert into a node that is guaranteed not full."""
    i = len(node.keys) - 1

    if node.is_leaf:
        # Find correct position and insert
        keys   = list(node.keys)
        values = list(node.values)
        while i >= 0 and key < keys[i]:
            i -= 1
        if i >= 0 and key == keys[i]:
            values[i] = value   # update existing key
        else:
            keys.insert(i + 1, key)
            values.insert(i + 1, value)
        return BTreeNode(keys=keys, values=values, children=[], is_leaf=True)
    else:
        # Find which child to descend into
        while i >= 0 and key < node.keys[i]:
            i -= 1
        i += 1
        children = list(node.children)
        if children[i].is_full(t):
            node = _split_child(node, i, t)
            # After split, recheck which side the key belongs to
            if key > node.keys[i]:
                i += 1
        new_child = _insert_non_full(children[i], key, value, t)
        children[i] = new_child
        return BTreeNode(
            keys=list(node.keys),
            values=list(node.values),
            children=children,
            is_leaf=False,
        )

def _split_child(parent: BTreeNode, i: int, t: int) -> BTreeNode:
    """
    Split parent.children[i] (which must be full) at its median.
    Promotes the median key into parent at position i.
    Returns new parent node.

    Before:                    After:
      parent: [A]                parent: [A, median, B]  (if i=1)
      child:  [k0..median..k_{2t-2}]
                               left: [k0..k_{t-2}]   right: [k_t..k_{2t-2}]
    """
    child = parent.children[i]
    mid   = t - 1    # index of median in child.keys

    left = BTreeNode(
        keys=child.keys[:mid],
        values=child.values[:mid],
        children=child.children[:mid + 1] if not child.is_leaf else [],
        is_leaf=child.is_leaf,
    )
    right = BTreeNode(
        keys=child.keys[mid + 1:],
        values=child.values[mid + 1:],
        children=child.children[mid + 1:] if not child.is_leaf else [],
        is_leaf=child.is_leaf,
    )
    median_key = child.keys[mid]
    median_val = child.values[mid]

    new_keys     = list(parent.keys);     new_keys.insert(i, median_key)
    new_values   = list(parent.values);   new_values.insert(i, median_val)
    new_children = list(parent.children); new_children[i] = left
    new_children.insert(i + 1, right)

    return BTreeNode(
        keys=new_keys,
        values=new_values,
        children=new_children,
        is_leaf=parent.is_leaf,
    )

# ─── Delete ────────────────────────────────────────────────────────────────

def delete(tree: BTree, key: Any) -> BTree:
    """
    Return new BTree with key removed.
    Handles all three deletion cases.
    Time: O(t · log_t n).
    """
    if tree.root is None:
        return tree
    new_root = _delete_node(tree.root, key, tree.t)
    # If root becomes empty after merge, its only child is the new root
    if new_root and len(new_root.keys) == 0 and not new_root.is_leaf:
        new_root = new_root.children[0]
    return BTree(root=new_root, t=tree.t, size=max(0, tree.size - 1))

# ─── Range Query ───────────────────────────────────────────────────────────

def range_query(tree: BTree, low: Any, high: Any) -> list[tuple[Any, Any]]:
    """
    Return all (key, value) pairs where low ≤ key ≤ high, in sorted order.
    Time: O(t · log_t n + k) where k is the number of results.
    """
    results: list[tuple[Any, Any]] = []
    if tree.root is not None:
        _range_node(tree.root, low, high, results)
    return results

def _range_node(node: BTreeNode, low: Any, high: Any,
                results: list[tuple[Any, Any]]) -> None:
    """DFS collecting all keys in [low, high]."""
    i = 0
    while i < len(node.keys):
        # Visit left subtree of keys[i] if it might contain keys in range
        if not node.is_leaf and low < node.keys[i]:
            _range_node(node.children[i], low, high, results)
        if low <= node.keys[i] <= high:
            results.append((node.keys[i], node.values[i]))
        i += 1
    # Visit rightmost child
    if not node.is_leaf and low <= high:
        _range_node(node.children[i], low, high, results)

# ─── Min / Max ─────────────────────────────────────────────────────────────

def min_key(tree: BTree) -> Any:
    """Return the smallest key. O(log_t n). Raises if tree is empty."""
    if tree.root is None:
        raise ValueError("tree is empty")
    node = tree.root
    while not node.is_leaf:
        node = node.children[0]
    return node.keys[0]

def max_key(tree: BTree) -> Any:
    """Return the largest key. O(log_t n). Raises if tree is empty."""
    if tree.root is None:
        raise ValueError("tree is empty")
    node = tree.root
    while not node.is_leaf:
        node = node.children[-1]
    return node.keys[-1]
```

## Public API

```python
from typing import Any, Generic, TypeVar

K = TypeVar("K")   # key type — must support < and ==
V = TypeVar("V")   # value type — arbitrary

class BTree(Generic[K, V]):
    """
    A B-tree of minimum degree t.
    Each non-root node holds between t-1 and 2t-1 keys.
    All leaves are at the same depth.
    Keys within each node are sorted.

    Choose t based on your disk page size:
        t = page_size / (2 * (key_size + value_size + pointer_size))
    Typical values: t=50..500 for database engines.
    """

    def __init__(self, t: int = 2) -> None:
        """
        t is the minimum degree (t ≥ 2).
        t=2 gives a 2-3-4 tree (classic textbook B-tree).
        """
        ...

    # ─── Core operations ───────────────────────────────────────────
    def insert(self, key: K, value: V) -> None:
        """Insert or update (key, value). O(t · log_t n)."""
        ...

    def delete(self, key: K) -> None:
        """Remove key from tree. No-op if key not present. O(t · log_t n)."""
        ...

    def search(self, key: K) -> V | None:
        """Return value for key, or None if not found. O(t · log_t n)."""
        ...

    def __contains__(self, key: K) -> bool:
        """key in tree → bool."""
        ...

    def __getitem__(self, key: K) -> V:
        """tree[key] → raises KeyError if not found."""
        ...

    def __setitem__(self, key: K, value: V) -> None:
        """tree[key] = value."""
        ...

    def __delitem__(self, key: K) -> None:
        """del tree[key] → raises KeyError if not found."""
        ...

    # ─── Queries ───────────────────────────────────────────────────
    def min_key(self) -> K:
        """Return smallest key. O(log_t n). Raises if empty."""
        ...

    def max_key(self) -> K:
        """Return largest key. O(log_t n). Raises if empty."""
        ...

    def range_query(self, low: K, high: K) -> list[tuple[K, V]]:
        """Return all (key, value) pairs with low ≤ key ≤ high. O(t·log_t n + k)."""
        ...

    def inorder(self) -> list[tuple[K, V]]:
        """Return all (key, value) pairs in sorted key order. O(n)."""
        ...

    # ─── Metadata ──────────────────────────────────────────────────
    def __len__(self) -> int:
        """Number of keys stored."""
        ...

    def __bool__(self) -> bool:
        """True if tree has at least one key."""
        ...

    def height(self) -> int:
        """Height of tree (0 if root is a leaf). O(log_t n)."""
        ...

    def is_valid(self) -> bool:
        """
        Verify all B-tree invariants. For testing only.
        Checks: key counts, sorted order, all leaves same depth.
        O(n).
        """
        ...
```

## Composition Model

### Inheritance (Python, Ruby, TypeScript)

B-tree builds on BST concepts but does NOT inherit from BSTNode — the node
structure is completely different. Instead, inherit from a shared abstract
`SearchTree` interface that defines the public API contract.

```python
# Python
from abc import ABC, abstractmethod

class SearchTree(ABC, Generic[K, V]):
    @abstractmethod
    def insert(self, key: K, value: V) -> None: ...
    @abstractmethod
    def search(self, key: K) -> V | None: ...
    @abstractmethod
    def delete(self, key: K) -> None: ...

class BTree(SearchTree[K, V]):
    def __init__(self, t: int = 2):
        self._root: BTreeNode | None = None
        self._t = t
        self._size = 0
    # ... implements SearchTree interface
```

```typescript
// TypeScript
interface SearchTree<K, V> {
  insert(key: K, value: V): void;
  search(key: K): V | undefined;
  delete(key: K): void;
}

class BTree<K, V> implements SearchTree<K, V> {
  constructor(private readonly t: number = 2) {}
  // ...
}
```

### Composition (Rust, Go, Elixir, Lua, Perl, Swift)

Rust: generic over key type with `Ord` bound.

```rust
pub struct BTree<K: Ord, V> {
    root: Option<Box<BTreeNode<K, V>>>,
    t:    usize,
    size: usize,
}

struct BTreeNode<K, V> {
    keys:     Vec<K>,
    values:   Vec<V>,
    children: Vec<Box<BTreeNode<K, V>>>,
    is_leaf:  bool,
}

impl<K: Ord + Clone, V: Clone> BTree<K, V> {
    pub fn new(t: usize) -> Self { ... }
    pub fn insert(&mut self, key: K, value: V) { ... }
    pub fn search(&self, key: &K) -> Option<&V> { ... }
    pub fn delete(&mut self, key: &K) { ... }
}
```

Go: uses a comparison function for generic key types.

```go
type BTree[K any, V any] struct {
    root *bTreeNode[K, V]
    t    int
    size int
    less func(a, b K) bool
}

type bTreeNode[K any, V any] struct {
    keys     []K
    values   []V
    children []*bTreeNode[K, V]
    isLeaf   bool
}
```

Elixir: immutable persistent tree using recursive tuples.

```elixir
defmodule BTree do
  # tree = {t, root_node}
  # node = {:leaf, keys, values} | {:internal, keys, values, children}
  def new(t \\ 2), do: {t, nil}
  def insert({t, root}, key, value), do: {t, insert_node(root, key, value, t)}
  def search({_t, root}, key), do: search_node(root, key)
end
```

## Test Strategy

### Invariant Verifier

Write a helper that checks all B-tree properties after every operation:

```python
def verify_btree(tree: BTree) -> None:
    """
    Assert all B-tree invariants hold.
    Call after every insert, delete, or bulk load in tests.
    """
    if tree.root is None:
        return
    # All leaves at same depth
    leaf_depths = _collect_leaf_depths(tree.root, 0)
    assert len(set(leaf_depths)) == 1, f"Leaves at different depths: {leaf_depths}"
    # Key counts in bounds
    _verify_node(tree.root, tree.t, is_root=True)
    # Keys sorted within each node
    _verify_sorted(tree.root)

def _verify_node(node, t, is_root):
    min_keys = 1 if is_root else t - 1
    max_keys = 2 * t - 1
    assert min_keys <= len(node.keys) <= max_keys
    if not node.is_leaf:
        assert len(node.children) == len(node.keys) + 1
        for child in node.children:
            _verify_node(child, t, is_root=False)
```

### Test Cases

```
1. Empty tree: search returns None, len is 0, height is 0.

2. Single insert: search finds the key, tree has height 0 (leaf root).

3. Sequential inserts (1..100): verify sorted output from inorder().
   verify_btree() must pass after every insert.

4. Random inserts: insert 1000 random integers; verify inorder() is sorted.

5. Root split: with t=2, insert 3 keys to fill root, then insert one more.
   Verify height becomes 1 (root now has 1 key, 2 children).

6. Delete from leaf: insert 5 keys, delete the middle one. Verify remaining 4.

7. Delete triggering merge: create a minimal tree where deletion causes a merge.
   Verify height may decrease.

8. Delete triggering rotation: delete from a leaf that borrows from a sibling.

9. Delete all keys: insert N keys then delete them all in random order.
   Verify tree is empty after each deletion is valid.

10. Range query: insert keys 1..100, range_query(30, 60) returns exactly 31 pairs.

11. min_key / max_key: after random inserts, verify against Python's min()/max().

12. Update: insert key=5 value="a", then insert key=5 value="b".
    Verify search returns "b" and len is still 1.

13. Large scale: insert 100,000 keys, verify all searchable, all deletable.

14. t=2, t=5, t=50: run the same test suite with all three values of t.
```

### Coverage Targets

- 95%+ line coverage
- All deletion sub-cases (Case 1, 2a, 2b, 2c, 3-rotate-left, 3-rotate-right, 3-merge)
- Split propagation all the way to root (tree grows taller)
- Merge propagation all the way to root (tree shrinks)

## Future Extensions

- **DT12 B+ tree** — variant where all data lives in leaf nodes and leaves are
  linked, enabling O(k) range scans without backtracking. Used by all major RDBMS.
- **Bulk loading** — when building a B-tree from a sorted input (e.g., importing a
  CSV), you can fill nodes to 100% capacity bottom-up instead of inserting one-by-one.
  This is 5–10x faster and produces a more compact tree.
- **Copy-on-write (persistent B-tree)** — instead of mutating nodes, create new
  nodes on the write path and share unchanged subtrees. Enables snapshot isolation
  in databases (MVCC). PostgreSQL's heap uses this concept.
- **B* tree** — variant that delays splits by first trying to redistribute keys to
  siblings. Nodes are 2/3 full on average instead of 1/2, reducing tree height.
- **Concurrent B-tree** — using lock-coupling (crabbing): hold parent lock while
  acquiring child lock, then release parent. Allows concurrent reads and writes.
