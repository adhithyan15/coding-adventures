# coding_adventures_avl_tree (Dart)

AVL self-balancing binary search tree: the Dart lane of
[DT08](../../../specs/DT08-avl-tree.md).

An AVL tree keeps every node's subtree heights within one, so search,
insert, and delete all cost O(log n). Updates here are immutable —
methods return new trees — so every intermediate version stays usable.

## API

```dart
import 'package:coding_adventures_avl_tree/coding_adventures_avl_tree.dart';

var tree = AvlTree<int>.empty().insert(3).insert(1).insert(2);
tree.toSortedArray();              // [1, 2, 3]
tree.contains(2);                  // true
tree = tree.delete(1);             // old tree still holds [1, 2, 3]

tree.search(2)?.value;             // node or null
tree.minValue(); tree.maxValue();  // extrema or null
tree.predecessor(2); tree.successor(2);
tree.kthSmallest(1);               // 1-based, null when out of range
tree.rank(2);                      // count of smaller values
tree.height(); tree.size();
tree.isValidBst(); tree.isValidAvl();
tree.balanceFactor(tree.root()!);  // in [-1, 1] when valid
```

* Updates are immutable with path copying: only nodes on the update path
  are cloned, so each op costs O(log n) garbage instead of O(n), and old
  versions stay valid forever.
* Read-only free functions (`avlSearch`, `avlHeight`, `avlSize`,
  `balanceFactorOf`, `isValidAvlRoot`) mirror the reference lanes. Unlike
  the reference, this lane exposes no mutating free functions: Dart has no
  borrow checker, so ownership-taking node surgery could corrupt a live
  tree. All mutation flows through the `AvlTree` methods.
* `rank` is 0-based (count of strictly smaller values); `kthSmallest` is
  1-based; both match the reference lanes exactly.
* Zero dependencies. Pure computation: no clock, no I/O, no randomness.

## Tests

```bash
dart pub get
dart test
```

The suite mirrors the reference tests (all four rotation shapes, order
statistics, deletions, neighbors, validity), runs a seeded 1,500-operation
model-based sweep against a sorted-list oracle with per-step agreement,
and covers immutability, duplicates, string keys, and free functions.
Coverage gate: 95% branch+function.
