# java/tree-set

A generic **sorted set** in Java with O(log n) insert/delete/contains and
a rich API including order statistics, range queries, and set algebra
(union, intersection, difference, symmetric difference).

## What is a TreeSet?

A TreeSet is a sorted set — like a `java.util.HashSet` but with elements always
kept in ascending order. All core operations run in O(log n) thanks to a
red-black tree backing store (`java.util.TreeMap`).

This `TreeSet<T>` extends Java's standard sorted set concept with:
- **Order statistics**: `rank`, `kthSmallest`
- **Range queries**: `range(low, high)`
- **Set algebra**: `union`, `intersection`, `difference`, `symmetricDifference`
- **Predicates**: `isSubset`, `isSuperset`, `isDisjoint`

## Usage

```java
import com.codingadventures.treeset.TreeSet;

TreeSet<Integer> s = new TreeSet<>(List.of(5, 3, 8, 1, 9, 2));

s.contains(3);          // true
s.min();                // 1
s.max();                // 9
s.predecessor(5);       // 3
s.successor(5);         // 8
s.rank(5);              // 3  (0-based: three elements are smaller)
s.kthSmallest(1);       // 1  (1-based)
s.byRank(0);            // 1  (0-based)

// Range query — all elements with 3 ≤ x ≤ 7
s.range(3, 7);          // [3, 5]

// Set algebra
TreeSet<Integer> t = new TreeSet<>(List.of(4, 5, 6, 7));
s.union(t);             // [1, 2, 3, 4, 5, 6, 7, 8, 9]
s.intersection(t);      // [5]
s.difference(t);        // [1, 2, 3, 8, 9]
s.symmetricDifference(t); // [1, 2, 3, 4, 6, 7, 8, 9]

// Predicates
TreeSet<Integer> sub = new TreeSet<>(List.of(3, 5));
sub.isSubset(s);        // true
s.isSuperset(sub);      // true
sub.isDisjoint(t);      // false (5 is shared)

// Iteration (always ascending)
for (int v : s) System.out.println(v);
```

## API

| Method | Description |
|---|---|
| `TreeSet()` | Create empty set |
| `TreeSet(Iterable<T>)` | Create from values |
| `TreeSet(TreeSet<T>)` | Independent copy |
| `TreeSet<T> add(T)` | Insert (chainable); throws for null |
| `boolean remove(T)` | Remove; returns true if was present |
| `boolean delete(T)` / `discard(T)` | Aliases for remove |
| `boolean contains(T)` / `has(T)` | O(log n) membership |
| `int size()` / `boolean isEmpty()` | Cardinality |
| `T min()` / `max()` / `first()` / `last()` | Extreme elements (null if empty) |
| `T predecessor(T)` | Largest element < value |
| `T successor(T)` | Smallest element > value |
| `int rank(T)` | 0-based count of elements < value |
| `T byRank(int)` | Element at 0-based position |
| `T kthSmallest(int k)` | Element at 1-based position |
| `List<T> range(T low, T high)` | All elements in [low, high] |
| `List<T> range(T low, T high, boolean inclusive)` | Inclusive or exclusive bounds |
| `List<T> toList()` / `toSortedArray()` | All elements sorted |
| `TreeSet<T> union(TreeSet<T>)` | Elements in either set |
| `TreeSet<T> intersection(TreeSet<T>)` | Elements in both sets |
| `TreeSet<T> difference(TreeSet<T>)` | Elements only in this set |
| `TreeSet<T> symmetricDifference(TreeSet<T>)` | Elements in exactly one set |
| `boolean isSubset(TreeSet<T>)` | All elements of this in other |
| `boolean isSuperset(TreeSet<T>)` | All elements of other in this |
| `boolean isDisjoint(TreeSet<T>)` | No shared elements |
| `boolean equals(TreeSet<T>)` | Element-wise equality |

## Running tests

```
gradle test
```

40 tests covering: construction, add/remove, min/max, predecessor/successor,
rank/byRank/kthSmallest, range (inclusive and exclusive), toList, all set
algebra operations, all predicates, equals/hashCode, iteration order, and a
1000-operation stress test against a reference `java.util.TreeSet`.
