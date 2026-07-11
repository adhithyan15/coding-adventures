# BinarySearchTree (Swift)

An **immutable** binary search tree of `Comparable` elements, in pure Swift.

Swift port of the `binary-search-tree` package that already exists in Rust and
other languages in the coding-adventures monorepo.

## What it does

Every node keeps left-subtree values smaller and right-subtree values larger, so
search / insert / delete are O(h) and an in-order walk yields sorted output.
Each node caches its subtree size, which powers the O(h) order-statistic
queries.

| Member | Purpose |
|---|---|
| `insert(_:)` / `delete(_:)` | return a **new** tree; the original is untouched |
| `search(_:)` / `contains(_:)` | exact lookup |
| `minValue()` / `maxValue()` | smallest / largest element |
| `predecessor(_:)` / `successor(_:)` | nearest strictly-smaller / -larger element |
| `kthSmallest(_:)` | the k-th smallest element (1-based) |
| `rank(_:)` | number of elements strictly less than a value |
| `toSortedArray()` | elements in ascending order |
| `isValid()` / `height` / `count` / `isEmpty` | introspection |
| `BST.fromSorted(_:)` | O(n) balanced build from a sorted array |

## Immutability

`insert` and `delete` return a new tree; the receiver is unchanged. Internally
the tree is an `indirect enum` (a value type), so unchanged subtrees are shared
structurally between versions — the same behaviour as the reference's
clone-on-write, done cheaply.

## Usage

```swift
import BinarySearchTree

let tree = BST<Int>().insert(8).insert(3).insert(10).insert(1).insert(6)
tree.contains(6)          // true
tree.minValue()           // 1
tree.kthSmallest(3)       // 6
tree.rank(6)              // 2
tree.toSortedArray()      // [1, 3, 6, 8, 10]
let smaller = tree.delete(3)  // a new tree; `tree` still has 3
```

## Running the tests

```
swift test
```
