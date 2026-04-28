# kotlin/segment-tree

Idiomatic Kotlin port of the segment tree (DT05), purely generic.

## What it is

A segment tree supporting range queries and point updates in O(log n).

## Kotlin idioms

- Generic class `SegmentTree<T>` with lambda `(T, T) -> T` for combine
- Companion object factory functions: `sumTree`, `minTree`, `maxTree`, `gcdTree`
- `val` properties: `size`, `isEmpty`
- `kotlin.math.min` / `kotlin.math.max` for standard combines
- `require()` for precondition checks (throws `IllegalArgumentException`)

## API

```kotlin
// Range sum:
val st = SegmentTree.sumTree(intArrayOf(2, 1, 5, 3, 4))
st.query(1, 3)   // → 9  (1 + 5 + 3)
st.update(2, 7)  // arr[2] is now 7
st.query(1, 3)   // → 11 (1 + 7 + 3)

// Range minimum:
val rm = SegmentTree.minTree(intArrayOf(5, 3, 7, 1, 9))
rm.query(0, 3)   // → 1

// Custom combine (range product):
val arr = arrayOf(2, 3, 4, 5)
val prod = SegmentTree(arr, { a, b -> a * b }, 1)
prod.query(0, 3) // → 120

// Reconstruct array:
st.toList()      // [2, 1, 7, 3, 4]  (after update above)
```

## Running tests

```bash
gradle test
```
