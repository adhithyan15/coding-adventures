# kotlin/treap

Idiomatic Kotlin port of the Treap (DT10), purely functional.

## What it is

A randomized BST+heap hybrid. Keys follow BST order; random priorities
follow max-heap order. The two properties together uniquely determine the
tree's shape for any set of (key, priority) pairs.

## Kotlin idioms

- `data class Node` with `copy()` for functional updates
- `Pair<Node?, Node?>` for split results (destructuring: `val (l, r) = split(...)`)
- `val` computed properties: `min`, `max`, `size`, `height`, `isEmpty`
- `kotlin.random.Random` for priority generation
- Top-level `mergeTreaps()` function (companion can't have overloaded `merge`)

## API

```kotlin
var t = Treap.withSeed(42L)
t = t.insert(5).insert(3).insert(7)

t.contains(3)          // true
t.min                  // 3
t.max                  // 7
t.predecessor(5)       // 3
t.successor(5)         // 7
t.kthSmallest(2)       // 5
t.toSortedList()       // [3, 5, 7]
t.isValidTreap()       // true

val (left, right) = t.split(4)
// left  → treap with {3}
// right → treap with {5, 7}

val merged = mergeTreaps(left, right)
val t2 = t.delete(5)
t2.isValidTreap()      // true
```

## Running tests

```bash
gradle test
```
