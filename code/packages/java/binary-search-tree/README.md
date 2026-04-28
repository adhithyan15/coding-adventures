# binary-search-tree — Java

A mutable Binary Search Tree (BST) with order-statistics support: O(log n)
insert, delete, search, predecessor, successor, k-th smallest, and rank. Each
node stores a `size` field enabling order-statistics operations without any
extra traversal.

## Usage

```java
import com.codingadventures.bst.BinarySearchTree;

BinarySearchTree<Integer> t = new BinarySearchTree<>();
for (int v : List.of(5, 1, 8, 3, 7)) t.insert(v);

t.toSortedList();       // [1, 3, 5, 7, 8]
t.minValue();           // Optional[1]
t.maxValue();           // Optional[8]
t.predecessor(5);       // Optional[3]
t.successor(5);         // Optional[7]
t.kthSmallest(4);       // Optional[7]
t.rank(4);              // 2  (1 and 3 are less than 4)

t.delete(5);
t.contains(5);          // false
t.size();               // 4

// Build a balanced BST from a sorted list in O(n)
BinarySearchTree<Integer> b = BinarySearchTree.fromSortedList(List.of(1,2,3,4,5,6,7));
b.height();             // 2  (floor(log₂ 7))
b.isValid();            // true
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
every right descendant is strictly greater. This lets search halve the problem
size at each node — just like binary search on a sorted array.

**Deletion of a node with two children**: replace the node's value with its
in-order successor (the minimum of its right subtree), then delete the
successor from the right subtree. This preserves the BST property.

**Size augmentation**: each node stores `size = 1 + size(left) + size(right)`.
This enables O(log n) k-th smallest and rank queries:

```
kthSmallest(k):
  leftSize = size(node.left)
  if k == leftSize + 1  → found!
  if k <= leftSize      → recurse left with same k
  else                  → recurse right with k -= (leftSize + 1)
```

## Running Tests

```bash
gradle test
```

41 tests covering insert, delete (leaf/one-child/two-child), search, contains,
min/max, predecessor/successor, order statistics, validation, string keys,
and a 1000-element stress test.

## Part of the Coding Adventures series

Java counterpart to the Python, Rust, Go, TypeScript, and Kotlin implementations.
