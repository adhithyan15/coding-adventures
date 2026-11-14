# coding_adventures_binary_search_tree

A pure Dart implementation of a persistent binary search tree (DT07). Every
insert and delete returns a new tree, sharing untouched immutable subtrees with
its source. Cached subtree sizes support rank and one-based selection.

```dart
import 'package:coding_adventures_binary_search_tree/coding_adventures_binary_search_tree.dart';

void main() {
  final tree = BST<int>();
  final populated = tree.insert(8).insert(3).insert(10).insert(6);
  print(populated.toSortedArray()); // [3, 6, 8, 10]
  print(populated.kthSmallest(2)); // 6
  print(populated.rank(8)); // 2 values are less than 8
}
```

Pass a `Comparator<T>` for custom orderings, including descending order. It
must define a stable total order, and every field used by it must stay
unchanged while keys are in the tree. The tree shares key objects between
versions; it does not clone them. A comparator result of zero defines key
equality, so insertion is a no-op for an equivalent key. `fromSortedArray`
requires strictly increasing values under the comparator. `minValue`,
`maxValue`, and `kthSmallest` return `null` for an empty tree or invalid rank;
keys are non-null, so `search` and `contains` distinguish a missing key.

Insertion, deletion, balanced construction, validation, and traversal use
recursion and therefore consume O(height) call stack. A skewed tree can have
height O(n); this implementation does not rebalance it.

## Reference checks

The tests use a separate `Set<int>` plus comparator-sorted `List<int>` as the
oracle. They enumerate every sequence of up to five operations from the
four-key domain `{0, 1, 2, 3}`. Each operation is insert or delete, so the
operation alphabet has eight actions. The traversal checks every prefix, both
in ascending and descending comparator order:

| Check | Exact count |
|---|---:|
| Operation histories, including empty | 74,898 |
| Current and retained tree states checked | 402,578 |
| Public query results checked | 2,584,106 |
| Nodes independently checked for ordering bounds and cached size | 112,472 |
| Sorted constructor candidate arrays | 682 |
| Constructor arrays accepted / rejected | 32 / 650 |

Every current state is checked against the sorted reference for contents, size,
minimum, maximum, membership, search, rank, predecessor, successor, and all
selection ranks from `-1` through `size + 1`. The independent recursive audit
checks strict subtree bounds and cached node sizes. Versions along each history are retained and rechecked after later operations.
A mutation audit ran the targeted test against two isolated copies after
`dart pub get --offline`:

- `dart test --plain-name 'checks every operation history through length five'`
  failed after changing the one-based `kthSmallest` comparison from
  `index == leftSize + 1` to `index == leftSize` (`Expected: <0>, Actual:
  <null>`).
- The same command failed after changing delete's final return to always
  return `this` (`Expected: [], Actual: [0]`).

Separate samples exercise 64-key balanced and maximally skewed shapes. The
bounded exhaustive history domain does not prove behavior for arbitrary key
types, inconsistent comparators, unbounded histories, stack exhaustion, or
concurrent mutation of objects referenced by keys. The comparator contract
requires a stable total order and immutable ordering fields.

Run `dart test` from this directory.
