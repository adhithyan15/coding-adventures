# java/avl-tree

A generic, self-balancing **AVL tree** in Java — a binary search tree that
maintains O(log n) height by enforcing that every node's left and right
subtrees differ in height by at most 1.

## What is an AVL tree?

An AVL tree (Adelson-Velsky and Landis, 1962) is a self-balancing BST. Unlike
a plain BST — which degrades to O(n) on sorted input — an AVL tree guarantees
O(log n) for every operation by rebalancing after every insert and delete.

Each node is augmented with:
- **height**: distance to the deepest leaf below it
- **size**: count of nodes in its subtree

The size augmentation enables O(log n) *order statistics*: `kthSmallest` and
`rank`.

## Usage

```java
import com.codingadventures.avltree.AVLTree;

AVLTree<Integer> tree = new AVLTree<>();
tree.insert(10);
tree.insert(5);
tree.insert(20);
tree.insert(3);
tree.insert(7);

tree.contains(5);            // true
tree.min();                  // 3
tree.max();                  // 20

// Predecessor / successor
tree.predecessor(10);        // 7
tree.successor(10);          // 20

// Order statistics (1-based)
tree.kthSmallest(1);         // 3
tree.kthSmallest(3);         // 10

// 0-based rank (count of strictly smaller elements)
tree.rank(10);               // 2

// In-order traversal
tree.toSortedList();         // [3, 5, 7, 10, 20]

// Introspection
tree.height();               // ≤ 1.44 · log2(n)
tree.size();                 // 5
tree.isValid();              // true (checks all AVL invariants)

tree.delete(10);
tree.size();                 // 4
```

## Algorithm details

### Rebalancing: four rotation cases

After every insert or delete, the algorithm walks back up the tree and
rebalances any node whose balance factor (BF = height(left) − height(right))
has reached ±2:

| Case | Condition | Fix |
|---|---|---|
| Left-Left | BF > 1, left child is left-heavy | rotate right at node |
| Left-Right | BF > 1, left child is right-heavy | rotate left at left child, then rotate right at node |
| Right-Right | BF < −1, right child is right-heavy | rotate left at node |
| Right-Left | BF < −1, right child is left-heavy | rotate right at right child, then rotate left at node |

### Delete: in-order successor replacement

For a node with two children, the value is replaced with its in-order
successor (minimum of the right subtree), and that successor is deleted.
Rebalancing propagates up to the root.

## API

| Method | Description |
|---|---|
| `AVLTree()` | Create empty tree |
| `void insert(T value)` | Insert; no-op if already present |
| `void delete(T value)` | Remove; throws `NoSuchElementException` if absent |
| `boolean contains(T value)` | O(log n) membership test |
| `T min()` | Smallest value; throws if empty |
| `T max()` | Largest value; throws if empty |
| `T predecessor(T value)` | Largest value < given; null if none |
| `T successor(T value)` | Smallest value > given; null if none |
| `T kthSmallest(int k)` | k-th smallest (1-based); null if out of range |
| `int rank(T value)` | 0-based count of elements < value |
| `List<T> toSortedList()` | All elements in ascending order |
| `int height()` | Tree height (-1 for empty) |
| `int size()` | Number of elements |
| `boolean isEmpty()` | True when size == 0 |
| `int balanceFactor()` | BF of root node |
| `boolean isValid()` | Validates BST + AVL + height + size invariants |
| `boolean isValidBST()` | Validates BST ordering only |

## Complexity

| Operation | Time |
|---|---|
| insert | O(log n) |
| delete | O(log n) |
| contains | O(log n) |
| min / max | O(log n) |
| predecessor / successor | O(log n) |
| kthSmallest | O(log n) |
| rank | O(log n) |
| toSortedList | O(n) |
| height | O(1) (cached) |

## Running tests

```
gradle test
```

36 tests covering: construction, insertion (ascending, descending, random,
duplicate), contains, deletion (leaf, one-child, two-child), min/max,
predecessor/successor, kthSmallest, rank, toSortedList, height,
balanceFactor, isValid, and a 1000-operation stress test against a
reference `java.util.TreeSet`.
