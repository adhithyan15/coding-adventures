# Changelog — java/tree-set

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-25

### Added

- `TreeSet<T extends Comparable<T>>` — sorted set backed by `java.util.TreeMap`
- `TreeSet()` — empty constructor
- `TreeSet(Iterable<T>)` — constructor from values; duplicates are collapsed
- `TreeSet(TreeSet<T>)` — copy constructor (independent copy)
- `add(T)` — O(log n) insert; chainable; throws `IllegalArgumentException` for null
- `remove(T)` / `delete(T)` / `discard(T)` — O(log n) remove; returns boolean
- `contains(T)` / `has(T)` — O(log n) membership
- `size()` / `isEmpty()` — O(1) cardinality
- `min()` / `max()` / `first()` / `last()` — O(log n) extreme elements (null if empty)
- `predecessor(T)` — O(log n) via `TreeMap.lowerKey()`
- `successor(T)` — O(log n) via `TreeMap.higherKey()`
- `rank(T)` — O(n) via `headMap().size()`; 0-based count of smaller elements
- `byRank(int)` — O(rank) iteration; 0-based element at position
- `kthSmallest(int)` — O(k) 1-based order statistic
- `range(T low, T high)` — O(log n + k) via `TreeMap.subMap()`; inclusive bounds
- `range(T low, T high, boolean inclusive)` — inclusive or exclusive bounds
- `toList()` / `toSortedArray()` — O(n) sorted snapshot
- `union(TreeSet<T>)` — O(n + m); returns new set
- `intersection(TreeSet<T>)` — O(min(n,m)·log(max(n,m))); returns new set
- `difference(TreeSet<T>)` — O(n·log m); returns new set
- `symmetricDifference(TreeSet<T>)` — O((n+m)·log(n+m)); returns new set
- `isSubset(TreeSet<T>)` — O(n·log m)
- `isSuperset(TreeSet<T>)` — O(m·log n)
- `isDisjoint(TreeSet<T>)` — O(min(n,m)·log(max(n,m)))
- `equals(Object)` / `hashCode()` — delegates to `TreeMap.keySet()`
- `iterator()` — ascending order via `TreeMap.keySet().iterator()`
- 40 unit tests covering all operations, set algebra, predicates, and a 1000-op
  stress test against `java.util.TreeSet`
