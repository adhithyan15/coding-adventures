# binary-tree — Kotlin

A generic binary tree with level-order construction, structural predicates
(full, complete, perfect), four traversals (in/pre/post/level-order), array
projection, and ASCII rendering.

## Usage

```kotlin
import com.codingadventures.binarytree.BinaryTree

// Build from level-order (BFS) list; null = absent node
//          1
//         / \
//        2   3
//       / \   \
//      4   5   6
val t = BinaryTree.fromLevelOrder(listOf(1, 2, 3, 4, 5, null, 6))

t.inorder()        // [4, 2, 5, 1, 3, 6]
t.preorder()       // [1, 2, 4, 5, 3, 6]
t.postorder()      // [4, 5, 2, 6, 3, 1]
t.levelOrder()     // [1, 2, 3, 4, 5, 6]

t.height()         // 2
t.size             // 6
t.isFull()         // false  (node 3 has only right child)
t.isComplete()     // false  (null before node 6)
t.isPerfect()      // false

// Perfect tree:
val p = BinaryTree.fromLevelOrder(listOf(1, 2, 3, 4, 5, 6, 7))
p.isPerfect()      // true  (height 2, 2^3 - 1 = 7 nodes)

println(t.toAscii())
// `-- 1
//     |-- 2
//     |   |-- 4
//     |   `-- 5
//     `-- 3
//         `-- 6
```

## How it works

**Level-order construction** maps index `i` to left child `2i+1` and right
child `2i+2` — the standard heap-array index formula applied to trees.

**Shape predicates**:
- `isFull`: recursive — every non-leaf must have exactly 2 children.
- `isComplete`: null-sentinel BFS using `LinkedList` (Kotlin's `ArrayDeque`
  doesn't permit null items). Once a null slot is seen, any subsequent
  non-null node means incompleteness.
- `isPerfect`: size must equal `2^(h+1) - 1`.

## Running Tests

```bash
gradle test
```

42 tests covering construction, find, shape predicates, all four traversals,
toArray, toAscii, height/size, string values, and manual construction.

## Part of the Coding Adventures series

Kotlin counterpart to the Python, Rust, Go, TypeScript, and Java implementations.
