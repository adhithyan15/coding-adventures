# coding_adventures_tree_set (Dart)

Immutable ordered sets over a swappable balanced-tree backend (AVL default):
the Dart lane of the tree-set identity.

A `TreeSet` keeps values sorted with logarithmic operations, rank access,
range slices, and full set algebra — all returning new sets, so every
version stays usable.

## API

```dart
import 'package:coding_adventures_tree_set/coding_adventures_tree_set.dart';

final a = TreeSet<int>.fromList([3, 1, 2]);
a.toSortedArray();                 // [1, 2, 3]
a.contains(2);                     // true
a.rank(2);                         // 1 (0-based)
a.kthSmallest(2);                  // 2 (1-based)
a.range(1, 2, true);               // [1, 2]
a.minValue(); a.maxValue();        // first() / last() aliases too
a.predecessor(2); a.successor(2);
a.insert(4); a.remove(1);          // delete() aliases remove()
a.union(b); a.intersection(b);     // set algebra, sorted results
a.difference(b); a.symmetricDifference(b);
a.isSubset(b); a.isSuperset(b); a.isDisjoint(b); a.equals(b);
a.length; a.size(); a.isEmpty; a.isNotEmpty;

// Explicit backends (a future red-black lane plugs in here)
TreeSet<int>.withBackend(AvlTreeBackend<int>());
```

* Backends implement `OrderedTreeBackend`; only AVL ships today.
* `rank` is 0-based, `kthSmallest` 1-based, matching the reference lanes.
* Depends only on the in-lane `avl-tree` package. Pure computation:
  no clock, no I/O, no randomness.

## Tests

```bash
dart pub get
dart test
```

The suite mirrors the reference vectors (operations, ranges, algebra),
runs a seeded 500-pair model sweep against Dart's `Set` oracle covering
every algebra operation and predicate plus ranges, and covers backends,
aliases, empties, and immutability. Coverage gate: 95% branch+function.
