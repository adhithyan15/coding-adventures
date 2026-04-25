# java/red-black-tree

A purely functional Red-Black Tree implementation in Java 21 (DT09).

## What it is

A Red-Black tree is a self-balancing binary search tree where each node
carries a color bit (RED or BLACK). Five invariants on those colors guarantee
the tree height is at most `2 × log₂(n + 1)`, giving O(log n) worst-case for
all operations.

Red-Black trees are the most widely used balanced BST in production:
- Java's `TreeMap` and `TreeSet`
- C++ STL `std::map` and `std::set`
- Linux kernel process scheduler (`rbtree.h`)

## Design

**Purely functional** — insert and delete return NEW tree objects. The
original tree is never mutated. Based on:
- Okasaki's 4-case balance function for insertion (1999)
- Sedgewick's Left-Leaning Red-Black (LLRB) algorithm for deletion

## API

```java
RBTree t = RBTree.empty();
t = t.insert(10).insert(5).insert(15);

t.contains(5);       // true
t.min();             // Optional.of(5)
t.max();             // Optional.of(15)
t.predecessor(10);   // Optional.of(5)
t.successor(10);     // Optional.of(15)
t.kthSmallest(2);    // 10
t.toSortedList();    // [5, 10, 15]
t.blackHeight();     // 2
t.isValidRB();       // true

RBTree t2 = t.delete(10);
t2.contains(10);     // false
t2.isValidRB();      // true
```

## Where it fits

```
DT03: binary-tree
DT07: binary-search-tree    ← parent
DT08: avl-tree              ← sibling (stricter balance)
DT09: red-black-tree        ← this package
DT10: treap                 ← sibling (randomized balance)
```

## Running tests

```bash
gradle test
```
