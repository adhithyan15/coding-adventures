## 0.1.0

- Initial Dart lane of the avl-tree identity: immutable `AvlTree` with
  insert/delete/search/contains, min/max, predecessor/successor,
  1-based kthSmallest, 0-based rank, sorted arrays, BST/AVL validity,
  balance factors, height/size, plus read-only free functions.
- Path copying (O(log n) garbage per update), private node fields with
  read-only getters, and no mutating free functions, so the immutable
  API cannot be corrupted from outside the library.
- 17 tests: rotation shapes, order statistics, deletions, neighbors, a
  seeded 1,500-op model sweep, multi-version isolation, and read-only
  free functions; 95% branch+function coverage gate.
