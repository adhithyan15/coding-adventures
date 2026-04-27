# kotlin/b-tree

A generic, self-balancing **B-tree** implementation in Kotlin with full
key-value mapping, proactive top-down splitting, CLRS-correct deletion, and
structural validation.

## What is a B-tree?

A B-tree generalises a binary search tree so that each node holds many keys
instead of just one. This is the property that makes B-trees ideal for
disk-based storage:

- A hard drive or SSD reads data in **blocks** (typically 4 KiB or more).
- If each tree node fills exactly one block, every lookup takes O(log_t n)
  *block reads* instead of O(log n) random pointer dereferences.
- With t=50, a height-4 tree holds roughly **50⁴ ≈ 6 million** keys in just
  4 I/O operations.
- SQLite, PostgreSQL, MySQL, and most filesystems use B-tree variants.

## Minimum degree `t`

Every B-tree is parameterised by an integer `t ≥ 2`:

| Property | Value |
|---|---|
| Min keys per non-root node | `t - 1` |
| Max keys per any node | `2t - 1` |
| Children per internal node | `keys + 1` |

With `t = 2` (the default), each node holds 1–3 keys and 2–4 children — the
famous **2-3-4 tree**.

## Usage

```kotlin
import com.codingadventures.btree.BTree

// 2-3-4 tree (t=2, the default)
val tree = BTree<Int, String>()

tree.insert(5, "five")
tree.insert(3, "three")
tree.insert(7, "seven")
tree.insert(1, "one")

// Search
tree.search(3)              // "three"
tree.search(99)             // null
tree.contains(5)            // true

// Min / max
tree.minKey()               // 1
tree.maxKey()               // 7

// Range query — all entries with 3 ≤ key ≤ 6
tree.rangeQuery(3, 6)       // [(3, "three"), (5, "five")]

// In-order iteration (ascending)
for ((key, value) in tree.inorder()) {
    println("$key → $value")
}

// Shape
tree.height                 // 0 (fits in one node)
tree.size                   // 4
tree.isValid()              // true

// Delete
tree.delete(3)
tree.contains(3)            // false
tree.size                   // 3

// Update (re-inserting an existing key updates its value)
tree.insert(5, "FIVE")
tree.search(5)              // "FIVE"
```

### Higher minimum degree

```kotlin
// t=10: each node holds 9–19 keys; tree stays very flat for large datasets
val wide = BTree<String, Int>(t = 10)
```

## Algorithm details

### Insertion — proactive top-down splitting

When descending to the insertion point, every full node encountered is
**split immediately** (before descending into it). By the time the leaf is
reached, every ancestor has room for one more key — no backtracking needed.

```
Root is full → create new root, split old root
               height increases by 1
Insert into non-full leaf
```

### Deletion — CLRS three-case algorithm

The invariant: every non-root node must have ≥ `t-1` keys after deletion.

**Pre-fill** any node with exactly `t-1` keys before descending into it:
- **3a**: borrow from a sibling with ≥ t keys (rotate through parent)
- **3b**: merge with a sibling + pull separator down from parent

Then the actual removal:
- **Case 1**: key in a leaf → remove directly
- **Case 2a**: key in internal node, left child fat → replace with in-order predecessor
- **Case 2b**: key in internal node, right child fat → replace with in-order successor
- **Case 2c**: key in internal node, both children thin → merge and recurse

## API

| Member | Description |
|---|---|
| `BTree(t: Int = 2)` | Create with minimum degree `t` (must be ≥ 2) |
| `fun insert(key: K, value: V)` | Insert or update |
| `fun delete(key: K)` | Remove; throws `NoSuchElementException` if absent |
| `fun search(key: K): V?` | Look up value; returns `null` if absent |
| `fun contains(key: K): Boolean` | Membership test |
| `fun minKey(): K` | Smallest key; throws if empty |
| `fun maxKey(): K` | Largest key; throws if empty |
| `fun rangeQuery(low: K, high: K): List<Pair<K, V>>` | All entries with `low ≤ key ≤ high` |
| `fun inorder(): List<Pair<K, V>>` | All entries in ascending key order |
| `val height: Int` | Distance from root to leaf (0 for leaf-only tree) |
| `val size: Int` | Number of key-value pairs |
| `val isEmpty: Boolean` | True when `size == 0` |
| `fun isValid(): Boolean` | Validates all 6 B-tree structural invariants |

## Complexity

| Operation | Time |
|---|---|
| insert | O(t · log_t n) |
| delete | O(t · log_t n) |
| search / contains | O(t · log_t n) |
| minKey / maxKey | O(log_t n) |
| rangeQuery(low, high) | O(t · log_t n + k) where k = result size |
| inorder | O(n) |
| height | O(log_t n) |

## Running tests

```
gradle test
```

45 tests covering: construction, insertion, search, deletion (all CLRS cases),
minKey/maxKey, rangeQuery, inorder traversal, height, isValid, and a 1000-key
stress test comparing against a reference `TreeMap`.
