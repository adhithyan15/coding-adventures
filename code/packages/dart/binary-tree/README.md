# Binary Tree (Dart)

This package implements the bounded DT03 binary-tree contract for Dart. It is
an immutable pointer tree whose nodes are named by identity, so duplicate or
nullable payloads never make child and depth queries ambiguous.

## Contract

- iterative inorder, preorder, postorder, and level-order traversals
- size, edge height, and full/complete/perfect shape predicates
- tagged level-order slots that distinguish a missing node from a null value
- canonical sparse-array round trips with orphan rejection
- rejection of shared nodes and positions beyond 1,048,575 output slots
- stable error codes, shallow payload ownership, and no OS capabilities

## Example

```dart
import 'package:coding_adventures_binary_tree/binary_tree.dart';

void main() {
  final tree = BinaryTree<int>.fromLevelOrder(const [
    BinaryTreeSlot.value(1),
    BinaryTreeSlot.value(2),
    BinaryTreeSlot.value(3),
    BinaryTreeSlot.missing(),
    BinaryTreeSlot.value(4),
  ]);

  print(tree.inorder()); // [2, 4, 1, 3]
  print(tree.height); // 2
}
```

The package is deliberately independent of DT02. Binary-tree positions have
left/right identity and sparse projection rules that a generic child list does
not provide.
