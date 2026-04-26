# fenwick-tree — Kotlin

A Fenwick Tree (Binary Indexed Tree) for O(log n) prefix sums and point
updates. Invented by Peter Fenwick in 1994. The entire algorithm relies on
one beautiful bit trick: `lowbit(i) = i and -i`.

## Usage

```kotlin
import com.codingadventures.fenwicktree.FenwickTree

// From existing array [3, 2, -1, 6, 5] (0-indexed → 1-based in tree)
val t = FenwickTree(longArrayOf(3, 2, -1, 6, 5))

t.prefixSum(3)      // 4   (3 + 2 + -1)
t.rangeSum(2, 4)    // 7   (2 + -1 + 6)
t.prefixSum(5)      // 15  (sum of all)

// Point update: add 10 to position 3
t.update(3, 10)
t.rangeSum(2, 4)    // 17  (2 + 9 + 6)

// Or start empty
val t2 = FenwickTree(100)
for (i in 1..100) t2.update(i, i.toLong())
t2.prefixSum(100)   // 5050
```

## How it works

The BIT array is 1-indexed. Each cell `bit[i]` stores the sum of `lowbit(i)`
consecutive elements ending at position `i`:

```
Index:   1    2    3    4    5    6    7    8
Binary: 001  010  011  100  101  110  111  1000
lowbit:   1    2    1    4    1    2    1    8
Range: [1] [1,2] [3] [1,4] [5] [5,6] [7] [1,8]
```

**Prefix sum** walks downward stripping the lowest set bit:
`prefixSum(7) = bit[7] + bit[6] + bit[4]` (3 steps ≤ log₂ 8).

**Update** walks upward adding the lowest set bit:
`update(3) touches bit[3], bit[4], bit[8]` (3 steps ≤ log₂ 8).

## Running Tests

```bash
gradle test
```

28 tests covering construction, updates, prefix sums, range sums, bounds
validation, negative deltas, and a 1000-element smoke test.

## Part of the Coding Adventures series

Kotlin counterpart to the Python, Rust, Go, TypeScript, and Java implementations.
