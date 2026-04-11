# DT12 — B+ Tree

## Overview

A B+ tree is a refinement of the B-tree (DT11) with two structural changes that
make it dramatically better for database workloads:

1. **All data lives at the leaves.** Internal nodes store only separator keys for
   routing — they hold no values. This makes internal nodes smaller, so more of
   them fit in memory and the tree is shallower.

2. **Leaf nodes form a sorted linked list.** Every leaf has a pointer to the next
   leaf. Once you find the starting leaf for a range query, you walk the linked list
   without backtracking up the tree.

These two changes turn the B-tree into the standard data structure for database
indexes. PostgreSQL, MySQL InnoDB, SQLite (for indexed columns), Oracle, and most
other relational databases use B+ trees for their indexes.

## Layer Position

```
DT11: b-tree             ← direct parent (B+ tree is a refinement)
DT12: b-plus-tree        ← [YOU ARE HERE]
  ├── used by: relational database indexes (PostgreSQL, MySQL, SQLite)
  ├── used by: filesystem indexes (NTFS $INDEX, HFS+ catalog)
  └── used by: key-value stores (LevelDB SSTable index blocks)

DT13: trie               (sibling: string-keyed, different lineage)
DT14: radix-tree
```

**Depends on:** DT11 (B-tree: node splitting, minimum degree, balancing).
**Key difference from DT11:** internal nodes are routing-only; leaves are linked.

## Concepts

### The Structural Difference

The clearest way to understand a B+ tree is to contrast it visually with a B-tree
holding the same data.

```
B-TREE (DT11) — keys and values everywhere:

               [20: "Carol", 40: "Eve"]
              /            |           \
  [5:"Alice",10:"Bob"]  [25:"Dave"]  [45:"Frank",55:"Grace"]

Every node holds values. Finding key 20 terminates at the root.
Range query for [5..25] requires traversal with backtracking.

─────────────────────────────────────────────────────────────

B+ TREE (DT12) — data only at leaves, internal nodes route only:

                    [20,  40]              ← internal: separator keys, no values
                   /    |    \
          [10, 20]   [25, 40]   [45, 55]  ← leaves: (key, value) pairs
            │           │            │
            └───────────┴────────────┘    ← leaf linked list (next pointers)

  leaf 1: [(5,"Alice"),(10,"Bob"),(20,"Carol")]  → leaf 2
  leaf 2: [(25,"Dave"),(40,"Eve")]               → leaf 3
  leaf 3: [(45,"Frank"),(55,"Grace")]            → None

Key observations:
  1. Key 20 appears in BOTH the internal node AND the leaf (as a separator copy).
     In a B-tree, it only appears once (in the internal node).
  2. To find "Carol", you follow root→leaf 1 and read the leaf.
  3. Range query [10..40]: go to leaf 1, collect (10,"Bob"),(20,"Carol"),
     follow next-pointer to leaf 2, collect (25,"Dave"),(40,"Eve"). Done.
     No backtracking to the tree.
```

### Why This Is Better for Databases

**Denser internal nodes → shallower tree:**

In a B-tree, internal nodes hold (key, value) pairs. If each value is 100 bytes
and each key is 8 bytes, a 4 KB page holds ~37 entries. In a B+ tree, internal
nodes hold only keys, so the same 4 KB page holds ~500 entries. With 500-way
branching vs 37-way branching, the B+ tree is significantly shallower.

```
B-tree   with 37-way branching:  log_37(1B)   ≈ 5.4  → 6 disk reads
B+ tree  with 500-way branching: log_500(1B)  ≈ 3.2  → 4 disk reads

That 2 disk read difference matters at millions of queries per second.
```

**Leaf linked list → free range scans:**

```
Query: "SELECT * FROM users WHERE age BETWEEN 25 AND 35"

B-tree: find leftmost key in range (O(log n)), collect all matching keys while
        continuing inorder traversal — requires climbing back up the tree.

B+ tree: find leftmost leaf (O(log n)), then just follow next-pointers until
         you leave the range. No upward traversal. Pure sequential I/O.
```

Sequential I/O on modern hardware is 10–100x faster than random I/O, because:
- Spinning disks: sequential reads avoid seek time.
- SSDs: sequential reads allow read-ahead prefetching.
- OS: sequential page faults trigger prefetch.

**Full table scan is trivially efficient:**

```python
# Full scan of a B+ tree indexed table:
leaf = find_leftmost_leaf(tree)
while leaf is not None:
    for key, value in leaf.entries:
        process(key, value)
    leaf = leaf.next   # O(1) hop to next leaf
```

This is essentially reading a sorted array. A B-tree full scan requires an
inorder traversal that jumps around the tree randomly.

### Insertion

Insertion in a B+ tree is similar to B-tree insertion with one important
difference: **when a leaf splits, the separator key is COPIED into the parent,
not moved.** The separator key remains in the leaf.

```
Insert key 30 into this B+ tree (t=2, max 3 keys per leaf):

Before:
           [20,  40]
          /    |    \
  [5,10,20]  [25,30] →  [45,55]
  (leaf 1)   (leaf 2)   (leaf 3)

Actually leaf 2 only has 2 keys; insert 30:
  leaf 2 becomes [25,30,40]? No — 40 belongs in leaf 3.
  Insert 30 into leaf 2: [25,30]→[25,30] with 30 added: still only 2 entries.

Let me redo with a cleaner example. Insert 35 into leaf 2 [25,40]:
  leaf 2 becomes [25,35,40] — full (3 keys = 2t-1 = 3).

Insert 38 into leaf 2 [25,35,40] — it's full, must split:

  Split leaf 2 at median:
    left:  [25,35]    (keys < median)
    right: [38,40]    (keys ≥ median, including 38 being inserted)
    separator: 38     ← COPIED into parent (38 stays in right leaf too!)

After split:
           [20,  38,  40]     ← 38 inserted into parent (was [20,40])
          /    |    |    \
  [5,10,20] [25,35] [38,40] [45,55]
       ↓        ↓       ↓       ↓     ← linked list maintained

Compare to B-tree split: if key 38 were in an internal node after split,
it would NOT appear in the leaf. In B+ tree, separator stays in leaf.
```

**Separator key promotion in B+ tree:**
```
After leaf split, the SMALLEST key of the NEW (right) leaf is promoted.
The right leaf still contains that key.

B-tree:    [1,2,|3|,4,5]  → parent gets 3; left=[1,2]; right=[4,5]
B+ tree:   [1,2,|3|,4,5]  → parent gets 3; left=[1,2]; right=[3,4,5]
                                              promoted key 3 stays in right leaf ↑
```

### Deletion

Deletion always removes from a leaf. If the leaf underflows (fewer than t-1 keys),
borrow from a sibling or merge.

When merging leaves, the separator key in the parent is **deleted** (not pushed down
to a leaf as in a B-tree), because the separator is just a routing copy.

```
Delete key 25 from a leaf [20,25,30]:
  Result: [20,30] — still has ≥ t-1 keys. Done.

Delete key 20 from a leaf [20] (only 1 key, t=2, minimum 1 key):
  Underflow! t-1 = 1, current = 0.

  Option A — borrow from right sibling [25,30]:
    Take 25 from right sibling.
    Leaf becomes [25]. Right sibling becomes [30].
    Update separator in parent (was 25, now 30 — the new smallest in right).

  Option B — merge with right sibling [25]:
    Merge [20]+[25] → [25] (we deleted 20, so just [25]).
    Delete the separator from the parent.
    If parent underflows, propagate the fix upward.
```

### The Clustered Index (InnoDB's Approach)

MySQL InnoDB takes B+ trees one step further: the **primary key index IS the table**.
The B+ tree leaf nodes don't contain pointers to rows — they contain the actual row
data. This is called a **clustered index** (the data is physically clustered by key).

```
Secondary index:
  B+ tree leaf: (index_key → primary_key)
  To fetch the row: look up primary_key in the clustered index (second tree walk)

Primary (clustered) index (InnoDB):
  B+ tree leaf: (primary_key → entire row data)
  To fetch the row: one tree walk, leaf contains the row

Trade-off:
  Clustered index: fast primary key lookups, fast range scans on PK
  Secondary indexes: require two tree walks (index lookup → PK → row)
  But: row data is stored once, not duplicated per secondary index
```

### Comparison: B-tree vs B+ tree

```
Property                   │ B-tree (DT11)      │ B+ tree (DT12)
───────────────────────────┼────────────────────┼──────────────────────
Where values are stored    │ internal + leaves  │ leaves only
Internal node size         │ larger (key+val)   │ smaller (key only)
Branching factor           │ lower              │ higher → shallower
Point lookup               │ can terminate early│ always reaches leaf
Range scan                 │ inorder traversal  │ follow linked list
Full scan                  │ inorder traversal  │ walk linked list
Separator key after split  │ moved to parent    │ copied to parent
                           │ (not in leaf)      │ (stays in right leaf)
Delete key from internal   │ replace w/ pred    │ only delete from leaf
node                       │ or successor       │ (internal keys are routing)
Best for                   │ general purpose    │ database indexes
```

## Representation

### Internal Node

```python
@dataclass
class BPlusInternalNode:
    """
    Stores only separator keys for routing.
    A node with k keys has k+1 children.
    keys[i] is the smallest key in children[i+1].
    """
    keys:     list[Any]              # separator keys (no values!)
    children: list["BPlusNode"]      # BPlusInternalNode or BPlusLeafNode
```

### Leaf Node

```python
@dataclass
class BPlusLeafNode:
    """
    Stores actual (key, value) pairs.
    Forms a doubly-linked (or singly-linked) sorted list.
    """
    keys:   list[Any]           # sorted keys
    values: list[Any]           # values[i] corresponds to keys[i]
    next:   "BPlusLeafNode | None"  # pointer to next leaf
    # Optional: prev pointer for reverse scans
```

### Tree

```python
@dataclass
class BPlusTree:
    root:        BPlusInternalNode | BPlusLeafNode  # root may be a leaf if height=0
    t:           int               # minimum degree (t ≥ 2)
    size:        int               # total number of key-value pairs
    first_leaf:  BPlusLeafNode     # leftmost leaf — start of linked list
```

### Space

```
Internal nodes: O(n/t) nodes × O(t) keys = O(n)
Leaf nodes:     O(n/t) nodes × O(t) pairs = O(n)
Total: O(n)
```

## Algorithms (Pure Functions)

```python
# ─── Search (point lookup) ──────────────────────────────────────────────────

def search(tree: BPlusTree, key: Any) -> Any | None:
    """
    Return value for key, or None.
    Always descends to a leaf — never terminates at internal nodes.
    Time: O(t · log_t n).
    """
    leaf = _find_leaf(tree.root, key)
    # Binary search within the leaf
    for k, v in zip(leaf.keys, leaf.values):
        if k == key:
            return v
    return None

def _find_leaf(node, key) -> BPlusLeafNode:
    """Follow separator keys down to the correct leaf."""
    if isinstance(node, BPlusLeafNode):
        return node
    # Find the child index to follow
    i = 0
    while i < len(node.keys) and key >= node.keys[i]:
        i += 1
    return _find_leaf(node.children[i], key)

# ─── Range scan (the killer feature) ───────────────────────────────────────

def range_scan(tree: BPlusTree, low: Any, high: Any) -> list[tuple[Any, Any]]:
    """
    Return all (key, value) pairs where low ≤ key ≤ high.
    Uses the leaf linked list — no upward traversal.
    Time: O(t · log_t n + k) where k = number of results.
    This is MUCH faster in practice than a B-tree range scan
    because it performs sequential I/O on the leaf level.
    """
    results = []
    leaf = _find_leaf(tree.root, low)

    while leaf is not None:
        for k, v in zip(leaf.keys, leaf.values):
            if k > high:
                return results
            if k >= low:
                results.append((k, v))
        leaf = leaf.next   # follow the linked list — O(1)

    return results

# ─── Full scan ──────────────────────────────────────────────────────────────

def full_scan(tree: BPlusTree) -> list[tuple[Any, Any]]:
    """
    Return all (key, value) pairs in sorted order.
    Just walks the leaf linked list from left to right.
    Time: O(n). Space: O(n).
    """
    results = []
    leaf = tree.first_leaf
    while leaf is not None:
        for k, v in zip(leaf.keys, leaf.values):
            results.append((k, v))
        leaf = leaf.next
    return results

# ─── Insert ─────────────────────────────────────────────────────────────────

def insert(tree: BPlusTree, key: Any, value: Any) -> BPlusTree:
    """
    Insert (key, value) into the B+ tree.
    Splits leaf if full; propagates split up as needed.
    When a leaf splits, the SMALLEST KEY OF THE RIGHT LEAF is copied to parent.
    Time: O(t · log_t n).
    """
    # Implementation mirrors B-tree insert with the crucial difference:
    # after leaf split, right leaf KEEPS the promoted key.
    # See _split_leaf() below.
    ...

def _split_leaf(leaf: BPlusLeafNode, t: int) -> tuple[BPlusLeafNode, Any, BPlusLeafNode]:
    """
    Split a full leaf into two leaves.
    Returns (left_leaf, separator_key, right_leaf).
    separator_key is the SMALLEST key in right_leaf (and also stays in right_leaf).

    B-tree split pushes median OUT of both children.
    B+ tree leaf split KEEPS the separator in the right leaf.
    """
    mid = len(leaf.keys) // 2
    separator = leaf.keys[mid]     # separator = smallest key of right half

    right = BPlusLeafNode(
        keys=leaf.keys[mid:],      # separator stays here ← key difference from B-tree
        values=leaf.values[mid:],
        next=leaf.next,
    )
    left = BPlusLeafNode(
        keys=leaf.keys[:mid],
        values=leaf.values[:mid],
        next=right,                # update linked list pointer
    )
    return left, separator, right

def _split_internal(node, i: int, t: int):
    """
    Split an internal node's i-th child.
    Same as B-tree: median is promoted and REMOVED from both halves.
    (Internal nodes don't store data, so this is safe.)
    """
    # Identical to BTree._split_child() from DT11
    ...

# ─── Delete ─────────────────────────────────────────────────────────────────

def delete(tree: BPlusTree, key: Any) -> BPlusTree:
    """
    Delete key from the B+ tree.
    Always deletes from a leaf.
    Separator keys in internal nodes may become "stale" after deletion
    (they were just routing hints; the tree still works correctly).
    However, merging a leaf requires removing the corresponding separator from the parent.
    Time: O(t · log_t n).
    """
    ...
```

## Public API

```python
from typing import Any, Generic, TypeVar, Iterator

K = TypeVar("K")
V = TypeVar("V")

class BPlusTree(Generic[K, V]):
    """
    A B+ tree — the data structure underlying most database indexes.

    Key differences from BTree (DT11):
      - Internal nodes store only separator keys (no values).
      - Leaf nodes form a sorted linked list via next-pointers.
      - Range scans follow the linked list: O(log n + k) without backtracking.
      - Full scans walk the leaf list: O(n) sequential I/O.

    Parameterized by minimum degree t (t ≥ 2).
    Choose t ≈ page_size / (2 * key_size) for internal nodes.
    """

    def __init__(self, t: int = 2) -> None: ...

    # ─── Core operations ─────────────────────────────────────────────
    def insert(self, key: K, value: V) -> None:
        """Insert or update. O(t · log_t n)."""
        ...

    def delete(self, key: K) -> None:
        """Remove key. No-op if not present. O(t · log_t n)."""
        ...

    def search(self, key: K) -> V | None:
        """Point lookup. Always reaches a leaf. O(t · log_t n)."""
        ...

    def __contains__(self, key: K) -> bool: ...
    def __getitem__(self, key: K) -> V: ...     # raises KeyError
    def __setitem__(self, key: K, value: V) -> None: ...
    def __delitem__(self, key: K) -> None: ...  # raises KeyError

    # ─── Range operations (the killer feature) ───────────────────────
    def range_scan(self, low: K, high: K) -> list[tuple[K, V]]:
        """
        Return all (key, value) where low ≤ key ≤ high, sorted by key.
        Uses the leaf linked list — no tree backtracking.
        O(t · log_t n + k) where k = number of results.
        """
        ...

    def full_scan(self) -> list[tuple[K, V]]:
        """
        Return all (key, value) pairs in sorted order.
        Walks the leaf linked list from first to last leaf.
        O(n) with sequential memory access pattern.
        """
        ...

    def __iter__(self) -> Iterator[K]:
        """Iterate all keys in sorted order."""
        ...

    def items(self) -> Iterator[tuple[K, V]]:
        """Iterate all (key, value) pairs in sorted order."""
        ...

    # ─── Queries ─────────────────────────────────────────────────────
    def min_key(self) -> K:
        """First leaf's first key. O(1) with first_leaf pointer."""
        ...

    def max_key(self) -> K:
        """O(log_t n) — follow rightmost path to rightmost leaf."""
        ...

    # ─── Metadata ────────────────────────────────────────────────────
    def __len__(self) -> int: ...
    def __bool__(self) -> bool: ...
    def height(self) -> int: ...

    def is_valid(self) -> bool:
        """
        Verify all B+ tree invariants:
          - All leaves at same depth.
          - Internal nodes: key counts in [1, 2t-1].
          - Non-root nodes: key counts ≥ t-1.
          - Leaf nodes: sorted, linked list in sorted order.
          - Separator keys in internal nodes match smallest keys in right children.
        O(n). For testing only.
        """
        ...
```

## Composition Model

### Inheritance (Python, Ruby, TypeScript)

B+ tree shares the same abstract `SearchTree` interface as B-tree (DT11), so
calling code can swap implementations transparently.

```python
# Python — B+ tree extends the same abstract base as B-tree
class BPlusTree(SearchTree[K, V]):
    def __init__(self, t: int = 2):
        self._root: BPlusInternalNode | BPlusLeafNode = BPlusLeafNode([], [], None)
        self._first_leaf = self._root
        self._t = t
        self._size = 0
```

```typescript
// TypeScript
class BPlusTree<K, V> implements SearchTree<K, V> {
  private root: BPlusInternalNode<K, V> | BPlusLeafNode<K, V>;
  private firstLeaf: BPlusLeafNode<K, V>;

  constructor(private readonly t: number = 2) {
    const leaf = new BPlusLeafNode<K, V>();
    this.root = leaf;
    this.firstLeaf = leaf;
  }
}
```

```ruby
# Ruby
class BPlusTree
  include Enumerable

  def initialize(t: 2)
    @leaf = BPlusLeafNode.new([], [], nil)
    @root = @leaf
    @first_leaf = @leaf
    @t = t
    @size = 0
  end

  def each(&block)
    leaf = @first_leaf
    while leaf
      leaf.keys.zip(leaf.values).each(&block)
      leaf = leaf.next_leaf
    end
  end
end
```

### Composition (Rust, Go, Elixir, Lua, Perl, Swift)

```rust
// Rust — two distinct node types via enum
enum BPlusNode<K, V> {
    Internal(BPlusInternal<K, V>),
    Leaf(BPlusLeaf<K, V>),
}

struct BPlusInternal<K, V> {
    keys:     Vec<K>,
    children: Vec<Box<BPlusNode<K, V>>>,
}

struct BPlusLeaf<K, V> {
    keys:   Vec<K>,
    values: Vec<V>,
    next:   Option<*mut BPlusLeaf<K, V>>,  // raw pointer for linked list
}

pub struct BPlusTree<K: Ord, V> {
    root:       Box<BPlusNode<K, V>>,
    first_leaf: *mut BPlusLeaf<K, V>,  // for full_scan O(n) start
    t:          usize,
    size:       usize,
}
```

Go: use interface for node polymorphism.

```go
type bplusNode[K any, V any] interface {
    isLeaf() bool
    findChild(key K, less func(K,K) bool) int
}

type bplusInternal[K any, V any] struct {
    keys     []K
    children []bplusNode[K, V]
}

type bplusLeaf[K any, V any] struct {
    keys   []K
    values []V
    next   *bplusLeaf[K, V]
}

type BPlusTree[K any, V any] struct {
    root      bplusNode[K, V]
    firstLeaf *bplusLeaf[K, V]
    t         int
    size      int
    less      func(a, b K) bool
}
```

## Test Strategy

### Invariant Verifier

```python
def verify_bplus(tree: BPlusTree) -> None:
    """Call after every operation in tests."""
    # 1. All leaves at same depth
    leaf_depths = _collect_leaf_depths(tree.root, 0)
    assert len(set(leaf_depths)) == 1

    # 2. Internal nodes: separator keys match left-edge of right child
    _verify_separators(tree.root)

    # 3. Leaf linked list is sorted and complete
    seen = []
    leaf = tree.first_leaf
    while leaf:
        assert leaf.keys == sorted(leaf.keys)
        seen.extend(leaf.keys)
        leaf = leaf.next
    assert seen == sorted(seen), "leaf list not globally sorted"
    assert len(seen) == tree.size

    # 4. Every key searchable from root
    for key in seen:
        assert tree.search(key) is not None
```

### Test Cases

```
1. Empty tree: search → None, full_scan → [], range_scan → [].

2. Single insert: found by search and full_scan. Leaf linked list has 1 leaf.

3. Sequential inserts (1..50): full_scan returns them in order.
   verify_bplus() after each insert.

4. Leaf split: with t=2, fill root-leaf to 3 keys, insert 1 more.
   Verify height=1, root is now internal, first_leaf pointer correct,
   leaf linked list has 2 leaves with next-pointer set.

5. Range scan: insert 1..100, range_scan(30, 60) returns exactly
   [(30,v), (31,v), ..., (60,v)] — 31 entries, all in order.

6. Range scan — left-open: range_scan(0, 5) where min key is 1.
   Should return keys 1..5, not crash on 0.

7. Range scan — right-open: range_scan(95, 200) where max key is 100.
   Should return keys 95..100, stop at last leaf.

8. Full scan: verify full_scan() == sorted(all_inserted_pairs).

9. Delete from leaf (easy case): delete key with siblings having spare keys.

10. Delete triggering leaf merge: create two adjacent minimal leaves,
    delete from one, verify merge, verify linked list pointer updated.

11. Delete all keys in random order: verify is_valid() at each step,
    and tree is empty at end.

12. Separator key behavior: after split, confirm the separator key
    appears in BOTH the internal node AND the right leaf
    (unlike B-tree where it only appears in the internal node).

13. Clustered index simulation: store rows as (primary_key, row_dict),
    do range_scan on age: verify results without full table scan.

14. Performance — range scan vs point lookups:
    Insert 100,000 keys. Measure range_scan(40000, 60001) time.
    Expect: O(log n) to find start + O(k) sequential traversal.

15. Linked list integrity after many inserts and deletes:
    verify that following first_leaf.next.next.next... visits
    every key exactly once.
```

### Coverage Targets

- 95%+ line coverage
- Leaf split: left boundary, right boundary, middle key insertion
- Internal node split (propagation to root)
- All delete cases: easy leaf, borrow-left, borrow-right, merge
- range_scan with empty result, single result, all keys
- full_scan on empty tree, single leaf, multiple leaves

## Future Extensions

- **DT15 Suffix tree** — each edge is a substring instead of a single character;
  enables O(n) construction and O(m) substring lookup. Conceptually related to
  radix tree (DT14), which compresses tries similarly.
- **Concurrent B+ tree (Blink tree)** — add a "high key" to each node indicating
  the maximum key the node can hold. During a concurrent split, a reader can detect
  it went to the wrong node and follow a "right link" to catch up. No lock needed
  during traversal. Used in PostgreSQL's nbtree implementation.
- **Bulk loading** — given a pre-sorted list of (key, value) pairs, build the B+
  tree bottom-up in O(n) time: pack leaf nodes to ~70% capacity, then build internal
  nodes level by level. Dramatically faster than n individual inserts.
- **Compressed B+ tree** — use prefix compression for keys within a leaf (e.g.,
  "apple", "application" share prefix "app"). Store prefix once, compress suffixes.
  Reduces leaf size, allows higher branching factor.
- **Write-ahead logging** — every modification writes a log entry before touching
  the tree. If the process crashes mid-split, replay the log on recovery. This is
  how PostgreSQL and MySQL guarantee ACID durability.
