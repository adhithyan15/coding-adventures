# coding-adventures-b-tree

Pure Dart implementation of the DT11 minimum-degree B-tree. A B-tree stores
sorted key-value pairs in wide, balanced nodes, making it useful for indexes
that minimize storage-page reads.

The package supports upsert, point lookup, deletion with borrow/merge repair,
inclusive range queries, ordered traversal, min/max queries, height reporting,
and structural invariant validation.

```dart
import 'package:coding_adventures_b_tree/coding_adventures_b_tree.dart';

void main() {
  final tree = BTree<int, String>(3)
    ..insert(10, 'ten')
    ..insert(5, 'five')
    ..insert(20, 'twenty');

  print(tree.search(10)); // ten
  print(tree.rangeQuery(5, 10)); // [(5, five), (10, ten)]
}
```

The optional constructor argument is the minimum degree `t` (`t >= 2`). Each
non-root node holds between `t - 1` and `2t - 1` keys. Keys use their natural
`Comparable` order by default; pass a comparator as the second constructor
argument for custom key types. Values may be any Dart type, including nullable
types.
