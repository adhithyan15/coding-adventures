# coding-adventures-b-plus-tree

Pure Dart implementation of the DT12 B+ tree. All values live in sorted leaf
nodes, while internal nodes contain routing separators. The leaves form a
linked list, so an inclusive range scan pays one tree lookup and then walks
adjacent leaves sequentially.

```dart
import 'package:coding_adventures_b_plus_tree/coding_adventures_b_plus_tree.dart';

void main() {
  final tree = BPlusTree<int, String>(3)
    ..insert(10, 'ten')
    ..insert(5, 'five')
    ..insert(20, 'twenty');

  print(tree.search(10)); // ten
  print(tree.rangeScan(5, 20));
  // [(5, five), (10, ten), (20, twenty)]
}
```

The first optional constructor argument is the minimum degree `t` (`t >= 2`).
Keys use their natural `Comparable` order by default; pass a comparator as the
second argument for custom key types. Values may be any Dart type, including
nullable types. Mutations rebuild balanced levels from the sorted in-memory
entries, which keeps the implementation deterministic and every non-root node
within the DT12 degree bounds.
