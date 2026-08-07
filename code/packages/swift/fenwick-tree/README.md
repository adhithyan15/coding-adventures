# FenwickTree (Swift)

A Binary Indexed Tree (Fenwick tree) over `Double`, implemented in pure Swift.

Swift port of the `fenwick-tree` package that already exists in Rust, Python,
and other languages in the coding-adventures monorepo; mirrors the reference
behaviour exactly (1-based indices, `prefixSum(0) == 0`).

## What it does

A Fenwick tree answers, in **O(log n)**:

| Operation | Meaning |
|---|---|
| `update(i, delta:)` | add `delta` to the value at 1-based index `i` |
| `prefixSum(i)` | sum of the first `i` values (`prefixSum(0) == 0`) |
| `rangeSum(l, r)` | sum over the inclusive range `l...r` |
| `pointQuery(i)` | the single value at index `i` |
| `findKth(target)` | smallest index whose prefix sum ≥ `target` (order statistic) |

Plus `count`, `isEmpty`, and `bitArray` for introspection. Operations that can
fail (out-of-range index, invalid range, bad `findKth` target) **throw**
`FenwickError` — the idiomatic Swift equivalent of the reference's `Result`.

## Usage

```swift
import FenwickTree

var tree = FenwickTree(values: [3, 2, 1, 7, 4])
try tree.prefixSum(4)      // 13
try tree.rangeSum(2, 4)    // 10
try tree.update(3, delta: 5)
try tree.pointQuery(3)     // 6
try tree.findKth(10)       // 4
```

## How it works

Each 1-based slot `i` stores the sum of the `lowbit(i)` values ending at `i`,
where `lowbit(i) = i & -i` isolates the lowest set bit. Walking up
(`i += lowbit(i)`) reaches every slot an update must touch; walking down
(`i -= lowbit(i)`) accumulates a prefix — each in log-many steps.

## Running the tests

```
swift test
```
