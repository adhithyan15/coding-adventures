# kotlin/tree-set

A generic **sorted set** in Kotlin with O(log n) insert/delete/contains and
a rich API including order statistics, range queries, and set algebra
(union, intersection, difference, symmetric difference).

## What is a TreeSet?

A TreeSet is a sorted set — like a `HashSet` but with elements always kept in
ascending order. All core operations run in O(log n) thanks to a red-black
tree backing store (`java.util.TreeMap`).

This `TreeSet<T>` extends the standard sorted set concept with:
- **Order statistics**: `rank`, `kthSmallest`
- **Range queries**: `range(low, high)`
- **Set algebra**: `union`, `intersection`, `difference`, `symmetricDifference`
- **Predicates**: `isSubset`, `isSuperset`, `isDisjoint`

## Usage

```kotlin
import com.codingadventures.treeset.TreeSet

val s = TreeSet(listOf(5, 3, 8, 1, 9, 2))

s.contains(3)          // true
s.min()                // 1
s.max()                // 9
s.predecessor(5)       // 3
s.successor(5)         // 8
s.rank(5)              // 3  (0-based: three elements are smaller)
s.kthSmallest(1)       // 1  (1-based)
s.byRank(0)            // 1  (0-based)

// Range query — all elements with 3 ≤ x ≤ 7
s.range(3, 7)          // [3, 5]

// Set algebra
val t = TreeSet(listOf(4, 5, 6, 7))
s.union(t)             // [1, 2, 3, 4, 5, 6, 7, 8, 9]
s.intersection(t)      // [5]
s.difference(t)        // [1, 2, 3, 8, 9]
s.symmetricDifference(t) // [1, 2, 3, 4, 6, 7, 8, 9]

// Predicates
val sub = TreeSet(listOf(3, 5))
sub.isSubset(s)        // true
s.isSuperset(sub)      // true

// Iteration (always ascending)
for (v in s) println(v)

// Factory
val t2 = TreeSet.of(10, 20, 30)
```

## API

| Member | Description |
|---|---|
| `TreeSet<T>(values: Iterable<T> = emptyList())` | Create from values |
| `TreeSet.of(vararg values: T)` | Factory from varargs |
| `fun add(value: T): TreeSet<T>` | Insert (chainable) |
| `fun remove(value: T): Boolean` | Remove; returns true if was present |
| `fun delete(T)` / `discard(T)` | Aliases for remove |
| `fun contains(T): Boolean` / `has(T)` | O(log n) membership |
| `val size: Int` / `val isEmpty: Boolean` | Cardinality |
| `fun min(): T?` / `max()` / `first()` / `last()` | Extreme elements (null if empty) |
| `fun predecessor(T): T?` | Largest element < value |
| `fun successor(T): T?` | Smallest element > value |
| `fun rank(T): Int` | 0-based count of elements < value |
| `fun byRank(Int): T?` | Element at 0-based position |
| `fun kthSmallest(Int): T?` | Element at 1-based position |
| `fun range(low: T, high: T, inclusive: Boolean = true): List<T>` | Range query |
| `fun toList(): List<T>` / `toSortedArray()` | All elements sorted |
| `fun union(TreeSet<T>): TreeSet<T>` | Elements in either set |
| `fun intersection(TreeSet<T>): TreeSet<T>` | Elements in both sets |
| `fun difference(TreeSet<T>): TreeSet<T>` | Elements only in this set |
| `fun symmetricDifference(TreeSet<T>): TreeSet<T>` | Elements in exactly one set |
| `fun isSubset(TreeSet<T>): Boolean` | All elements of this in other |
| `fun isSuperset(TreeSet<T>): Boolean` | All elements of other in this |
| `fun isDisjoint(TreeSet<T>): Boolean` | No shared elements |
| `fun equals(other: Any?): Boolean` | Element-wise equality |

## Running tests

```
gradle test
```

38 tests covering: construction, add/remove, min/max, predecessor/successor,
rank/byRank/kthSmallest, range (inclusive and exclusive), toList, all set
algebra operations, all predicates, equals/hashCode, iteration order, and a
1000-operation stress test against a reference `java.util.TreeSet`.
