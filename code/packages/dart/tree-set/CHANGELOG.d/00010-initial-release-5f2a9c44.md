## 0.1.0

- Initial Dart lane of the tree-set identity: immutable `TreeSet` over a
  swappable `OrderedTreeBackend` (AVL adapter first consumer of the new
  Dart avl-tree lane) with rank, ranges, and full set algebra.
- 9 tests: reference vectors, a seeded 500-pair `Set`-oracle sweep over
  every operation and predicate, backends, and immutability; 95%
  branch+function coverage gate.
