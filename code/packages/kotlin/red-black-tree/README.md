# kotlin/red-black-tree

Idiomatic Kotlin port of the Red-Black Tree (DT09), purely functional.

## What it is

A Left-Leaning Red-Black Tree using Sedgewick's LLRB algorithm for both
insertion and deletion. All operations return new trees — originals are
never mutated.

## Kotlin idioms

- `data class Node` with `copy()` for functional updates
- `val` computed properties (`isRed`, `min`, `max`, `size`, `height`)
- Extension function `Node?.isRed()` for null-safe color checks
- `companion object` factory and helper functions
- `when` expressions in traversal logic

## API

```kotlin
var t = RBTree.empty()
t = t.insert(10).insert(5).insert(15)

t.contains(5)         // true
t.min                 // 5
t.max                 // 15
t.predecessor(10)     // 5
t.successor(10)       // 15
t.kthSmallest(2)      // 10
t.toSortedList()      // [5, 10, 15]
t.blackHeight         // 2
t.isValidRB()         // true

val t2 = t.delete(10)
t2.contains(10)       // false
t2.isValidRB()        // true
```

## Running tests

```bash
gradle test
```
