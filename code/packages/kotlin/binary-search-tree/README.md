# binary-search-tree — Kotlin

A mutable Binary Search Tree (BST) with order-statistics support: O(log n)
insert, delete, search, predecessor, successor, k-th smallest, and rank. Each
node caches a `size` field (subtree count) enabling order-statistics queries
without extra traversal.

## Usage

```kotlin
import com.codingadventures.bst.BinarySearchTree

val t = BinarySearchTree<Int>()
listOf(5, 1, 8, 3, 7).forEach { t.insert(it) }

t.toSortedList()          // [1, 3, 5, 7, 8]
t.minValue()              // 1
t.maxValue()              // 8
t.predecessor(5)          // 3
t.successor(5)            // 7
t.kthSmallest(4)          // 7
t.rank(4)                 // 2  (1 and 3 are less than 4)

t.delete(5)
t.contains(5)             // false
t.size                    // 4

// Build a balanced BST from a sorted list in O(n)
val b = BinarySearchTree.fromSortedList(listOf(1, 2, 3, 4, 5, 6, 7))
b.height()                // 2  (floor(log₂ 7))
b.isValid()               // true
```

## How it works

```
         5
        / \
       3   8
      / \   \
     1   4   9
```

The BST property: every left descendant is strictly less than its ancestor,
every right descendant strictly greater. Search halves the problem space at
every node — exactly like binary search on a sorted array.

**Size augmentation**: `node.size = 1 + size(left) + size(right)`, maintained
on every mutation. Enables O(log n) k-th smallest and rank queries:

```
kthSmallest(k):
  leftSize = node.left?.size ?: 0
  k == leftSize + 1  → found
  k <= leftSize      → recurse left with same k
  else               → recurse right with k -= (leftSize + 1)
```

## Running Tests

```bash
gradle test
```

41 tests covering insert, delete (leaf/one-child/two-child), search, contains,
min/max, predecessor/successor, order statistics, validation, string keys,
and a 1000-element stress test.

## Part of the Coding Adventures series

Kotlin counterpart to the Python, Rust, Go, TypeScript, and Java implementations.
