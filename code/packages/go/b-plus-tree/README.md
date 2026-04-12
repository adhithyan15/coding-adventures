# go/b-plus-tree — Generic B+ Tree (DT12)

A generic, self-balancing B+ tree written in Go using generics.

## What it does

A **B+ tree** is a refinement of the B-tree that moves all values to the leaf
level and connects leaves in a linked list.  This makes range scans O(log n + k)
instead of O(k log n) — crucial for database query engines.

```
Internal level:        [30]
                      /    \
Internal level:   [10,20]  [40,50]
                  / | \    / | \
Leaf level:    [5][15][25][35][45][55]
               ↓   ↓   ↓   ↓   ↓   ↓
               linked list: 5→15→25→35→45→55→nil
```

Every leaf holds actual key-value pairs.  Internal nodes hold only routing keys.
The leaf linked list makes `FullScan()` traverse all n entries in O(n) time —
no tree traversal needed after the first leaf.

## Where it fits in the stack

This package (DT12) is the B-tree family's second member in the monorepo.  It
sits in the data structures layer and has **no external dependencies**.

| Package | Tag | What it adds |
|---------|-----|--------------|
| `go/b-tree` | DT11 | B-tree, values at every node |
| `go/b-plus-tree` | DT12 | B+ tree, values only at leaves + leaf list |

## Key difference from B-tree

When splitting a **leaf** node, the separator key is **copied** to the parent
and also kept in the right leaf.  When splitting an **internal** node, the
median is **moved** (not copied) to the parent — same as a B-tree.

## API

```go
import bpt "github.com/adhithyan15/coding-adventures/code/packages/go/b-plus-tree"

// Create a B+ tree with minimum degree 3, keyed by int, valued by string.
t := bpt.New[int, string](3, func(a, b int) bool { return a < b })

t.Insert(10, "ten")
t.Insert(20, "twenty")
t.Insert(5,  "five")

v, ok := t.Search(10)          // "ten", true

// Range scan uses leaf linked list — very fast.
pairs := t.RangeScan(5, 15)    // [{5:five} {10:ten}]

// Full scan is O(n) — just walks the leaf list.
all := t.FullScan()            // [{5:five} {10:ten} {20:twenty}]

t.Delete(10)

min, _ := t.MinKey()           // 5  (O(1) — just reads firstLeaf.keys[0])
max, _ := t.MaxKey()           // 20 (O(log n))

t.Len()                        // 2
t.Height()                     // 0 or 1 depending on splits
t.IsValid()                    // true
```

## Running tests

```bash
go test ./... -v -cover
```

Expected coverage: ≥ 95%.
