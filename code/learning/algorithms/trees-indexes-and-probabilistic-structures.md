<!-- learning-concepts: binary-search-tree, binary-tree, avl-tree, red-black-tree, b-tree, b-plus-tree, trie, radix-tree, fenwick-tree, segment-tree, skip-list, tree-set, bitset, bloom-filter, hyperloglog -->

# Trees, Indexes, and Probabilistic Structures

These structures answer different versions of the same question: how much work
and memory should we spend now to make a future lookup cheap?

## Start With the Workload

Before choosing a structure, ask:

1. Are keys ordered, or do we only test membership?
2. Does the structure live in memory or on storage?
3. Are updates common?
4. Must every answer be exact?
5. Do we need point lookups, prefix lookups, ranges, or aggregates?

The name of a structure matters less than the workload it was designed for.

## Ordered Trees

A binary search tree keeps smaller keys on the left and larger keys on the
right. Its operations cost O(h), where h is the tree height. A lucky insertion
order produces a shallow tree; a sorted insertion order can produce a linked
list.

AVL and red-black trees prevent that collapse with rotations:

- an AVL tree maintains a stricter height balance and favors lookup speed
- a red-black tree permits more imbalance and often performs fewer update rotations

Both turn the important operations into O(log n) work.

B-trees and B+ trees solve a different physical problem. Their nodes contain
many keys so one node aligns with a disk page or cache-friendly block. A B+
tree keeps records in linked leaves, making range scans especially natural.
That is why database indexes usually look more like B+ trees than binary trees.

## Keys With Shared Prefixes

A trie follows one key component per edge. Keys such as `cat`, `car`, and
`care` share their first two edges. Lookup depends on key length rather than
the number of stored keys.

A radix tree compresses chains of single-child nodes into longer edge labels.
It retains prefix behavior while reducing pointer and node overhead.

## Range Aggregates

Fenwick and segment trees are not primarily search trees:

- a Fenwick tree stores partial sums in a compact array and supports prefix
  queries and point updates in O(log n)
- a segment tree stores aggregates for intervals and supports more general
  range queries, usually with more memory

The aggregate can be a sum, minimum, maximum, or any operation with the
composition properties the implementation requires.

## Trading Certainty for Space

Some workloads do not need an exact collection of every key.

A bitset maps integer positions directly to bits. It is exact and exceptionally
compact when the universe of possible values is bounded.

A Bloom filter hashes each item into several bit positions. A negative answer
is definitive; a positive answer may be a false positive. Bloom filters are
useful as a cheap guard before an expensive storage lookup.

HyperLogLog estimates the number of distinct values. It keeps information
about unusually long hash prefixes, then combines many small registers into a
cardinality estimate. It cannot enumerate the values it counted.

## A Practical Selection Guide

| Need | Good starting point |
| --- | --- |
| Ordered in-memory map | AVL or red-black tree |
| Storage index and range scans | B+ tree |
| Prefix completion | Trie or radix tree |
| Prefix sums with point updates | Fenwick tree |
| General range aggregates | Segment tree |
| Fast approximate membership | Bloom filter |
| Approximate distinct count | HyperLogLog |
| Dense integer membership | Bitset |

The repository implements these structures separately so their invariants are
visible. Compare their tests: the most revealing tests are usually the ones
that force a rebalance, cross a node boundary, or exercise an approximation's
allowed error.
