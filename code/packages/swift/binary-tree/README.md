# BinaryTree (Swift)

This package implements the DT03 binary-tree contract for Swift. A binary tree
has an optional root, and every node has independently named left and right
children. The package supplies the four standard traversals, identity-based
child and depth queries, shape predicates, height and size, and a positional
level-order representation.

## Contract

- `BinaryTreeNode` is an immutable identity-bearing class. Equal payloads do
  not make two nodes interchangeable.
- `BinaryTreeSlot.missing` is different from `.value(nil)` when the payload is
  optional. This keeps a missing position distinct from a present nullable
  payload.
- Supplied roots must be proper trees. Reusing one node at two positions is
  rejected, as is a cycle if one arrives through foreign or unsafe code.
- Level-order arrays are positional: children of position `i` occupy
  `2*i + 1` and `2*i + 2`. A present node below a missing parent is rejected,
  while trailing missing positions are removed canonically.
- Every traversal and shape algorithm is iterative. Node, slot, positional
  output, and construction work is bounded by 1,048,575 positions.
- Payload ownership is shallow. The tree retains caller values according to
  normal Swift value/reference semantics and never deep-copies them.
- The empty tree has height `-1` and is full, complete, and perfect.

The package intentionally has no DT02 runtime dependency and no ASCII renderer.
It is a finite, deterministic, in-memory library with an empty capability
profile.

## Example

```swift
import BinaryTree

let tree = try BinaryTree.fromLevelOrder([
  .value("A"),
  .value("B"),
  .value("C"),
  .missing,
  .value("D"),
])

tree.inorder()             // ["B", "D", "A", "C"]
tree.levelOrder()          // ["A", "B", "C", "D"]
try tree.toLevelOrder()    // preserves the interior missing position
```

Identity queries take nodes rather than payloads:

```swift
let leaf = BinaryTreeNode(value: 2)
let rooted = try BinaryTree(
  root: BinaryTreeNode(value: 1, left: leaf)
)

rooted.contains(leaf)      // true
try rooted.depth(of: leaf) // 1
```

## Validation

From this directory:

```sh
sh BUILD
```

The build front runs the package tests with coverage and compiles a release
build. On macOS it selects Swift through `xcrun`; Linux uses `swift` directly,
and `BUILD_windows` performs the equivalent fail-closed Swift validation.
