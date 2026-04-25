# java/treap

A purely functional Treap implementation in Java 21 (DT10).

## What it is

A treap is a randomized binary search tree where each node carries two values:
- **key**: determines BST ordering (left < node < right)
- **priority**: a random double; determines heap ordering (parent > children)

Random priorities guarantee O(log n) expected height — no rotations needed.

## Design

**Purely functional** — insert and delete return NEW treap objects using
split+merge as the core primitives.

**Split+merge approach**:
- `split(key)` → (left ≤ key, right > key) — O(log n)
- `merge(left, right)` → treap — O(log n)
- `insert` = split + singleton + merge twice
- `delete` = two splits + merge (discard the target)

## API

```java
Treap t = Treap.withSeed(42L);
t = t.insert(5).insert(3).insert(7);

t.contains(3);         // true
t.min();               // Optional.of(3)
t.max();               // Optional.of(7)
t.predecessor(5);      // Optional.of(3)
t.successor(5);        // Optional.of(7)
t.kthSmallest(2);      // 5
t.toSortedList();      // [3, 5, 7]
t.isValidTreap();      // true

Treap.SplitResult parts = t.split(4);
// parts.left()  → treap with {3}
// parts.right() → treap with {5, 7}

Treap t2 = t.delete(5);
t2.contains(5);        // false
t2.isValidTreap();     // true
```

## Where it fits

```
DT07: binary-search-tree    ← parent
DT08: avl-tree              ← sibling (deterministic)
DT09: red-black-tree        ← sibling (deterministic)
DT10: treap                 ← this package (randomized)
DT20: skip-list             ← distant cousin (also randomized)
```

## Running tests

```bash
gradle test
```
