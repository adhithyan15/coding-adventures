# Changelog — kotlin/tree-set

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-25

### Added

- `TreeSet<T : Comparable<T>>` — sorted set backed by `java.util.TreeMap`
- `TreeSet(values: Iterable<T> = emptyList())` — constructor; duplicates collapsed
- `TreeSet.of(vararg values: T)` — factory companion function
- `add(T): TreeSet<T>` — O(log n) insert; chainable
- `remove(T): Boolean` / `delete(T)` / `discard(T)` — O(log n) remove
- `contains(T): Boolean` / `has(T)` — O(log n) membership
- `val size: Int` / `val isEmpty: Boolean` — O(1) cardinality
- `min(): T?` / `max()` / `first()` / `last()` — O(log n) extreme elements
- `predecessor(T): T?` — O(log n) via `TreeMap.lowerKey()`
- `successor(T): T?` — O(log n) via `TreeMap.higherKey()`
- `rank(T): Int` — O(n) via `headMap().size`; 0-based count of smaller elements
- `byRank(Int): T?` — O(rank) iteration; 0-based element at position
- `kthSmallest(Int): T?` — O(k) 1-based order statistic
- `range(T, T, inclusive: Boolean = true): List<T>` — O(log n + k) range query
- `toList(): List<T>` / `toSortedArray()` / `toArray()` — O(n) sorted snapshot
- `union(TreeSet<T>): TreeSet<T>` — O(n + m); returns new set
- `intersection(TreeSet<T>): TreeSet<T>` — O(min(n,m)·log(max(n,m))); new set
- `difference(TreeSet<T>): TreeSet<T>` — O(n·log m); returns new set
- `symmetricDifference(TreeSet<T>): TreeSet<T>` — O((n+m)·log(n+m)); new set
- `isSubset(TreeSet<T>): Boolean` — O(n·log m)
- `isSuperset(TreeSet<T>): Boolean` — O(m·log n)
- `isDisjoint(TreeSet<T>): Boolean` — O(min(n,m)·log(max(n,m)))
- `equals(Any?)` / `hashCode()` — delegates to `TreeMap.keys`
- `iterator()` — ascending order
- 38 unit tests covering all operations, set algebra, predicates, and a 1000-op
  stress test against `java.util.TreeSet`
