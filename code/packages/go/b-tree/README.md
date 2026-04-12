# go/b-tree — Generic B-Tree (DT11)

A generic, self-balancing B-tree written in Go using generics.

## What it does

A **B-tree** is a balanced multi-way search tree designed for systems that
read and write large blocks of data.  Unlike a binary search tree (which has
at most 2 children per node), a B-tree node holds up to `2t-1` keys and `2t`
children.  The result is a short, fat tree that minimises the number of
levels you must traverse — critical for disk I/O where each level is a
separate read.

```
         [30]
        /    \
   [10,20]  [40,50]
   / | \    / | \
 [5][15][25][35][45][55]
```

All leaves are at the same depth — this is the core invariant that guarantees
O(log n) worst-case for all operations.

## Where it fits in the stack

This package (DT11) sits in the **data structures** layer of the
coding-adventures monorepo.  It has **no external dependencies** — it is a
pure, self-contained implementation.

The companion package [`go/b-plus-tree`](../b-plus-tree) (DT12) builds on the
same ideas but moves all values to the leaf level and adds a linked list across
leaves for O(n) full scans.

## API

```go
import btree "github.com/adhithyan15/coding-adventures/code/packages/go/b-tree"

// Create a B-tree with minimum degree 3, keyed by int, valued by string.
t := btree.New[int, string](3, func(a, b int) bool { return a < b })

t.Insert(10, "ten")
t.Insert(20, "twenty")
t.Insert(5,  "five")

v, ok := t.Search(10)     // "ten", true
t.Contains(99)            // false

t.Delete(10)

pairs := t.Inorder()      // sorted key-value pairs
pairs = t.RangeQuery(5, 20) // pairs with 5 ≤ key ≤ 20

min, _ := t.MinKey()      // 5
max, _ := t.MaxKey()      // 20

t.Len()                   // 2
t.Height()                // 0 (just a root leaf after deletes)
t.IsValid()               // true
```

## Parameters

| Parameter | Meaning | Minimum |
|-----------|---------|---------|
| `t` | Minimum degree | 2 |

With `t=2` each node has 1–3 keys (a 2-3-4 tree).
With `t=100` each node has 99–199 keys (database-scale).

## Running tests

```bash
go test ./... -v -cover
```

Expected coverage: ≥ 95%.
