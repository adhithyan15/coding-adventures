# DT06 — Fenwick Tree (Binary Indexed Tree)

## Overview

The Fenwick tree, invented by Peter Fenwick in 1994, solves ONE problem with
extraordinary elegance: **prefix sums with point updates**, in O(log n) time
and O(n) space.

A **prefix sum** is the sum of elements from index 1 to i (1-indexed). Given
an array `[3, 2, 1, 7, 4]`, the prefix sum at index 3 is `3 + 2 + 1 = 6`.

The Fenwick tree answers:
- "What is the sum of elements from index 1 to 5?" — O(log n)
- "Add 3 to element at index 2" — O(log n)
- "What is the sum of elements from index 3 to 7?" — two prefix sums, O(log n)

It does NOT generalize to arbitrary combine functions as cleanly as the segment
tree. It's specifically optimized for the prefix-sum use case where you need
both updates and queries on a running total.

### Why Does This Exist if We Already Have Segment Trees?

The Fenwick tree is:
- **Simpler to implement** — about 10 lines of code in its core
- **Half the memory** — O(n) vs O(4n) for a segment tree
- **Faster in practice** — fewer cache misses, better constant factors

For prefix-sum-specific problems (e.g., counting inversions, order statistics,
coordinate compression), a Fenwick tree is the tool of choice.

### The Magic of Lowest Set Bit

The entire data structure hinges on one bit trick:

```
lowbit(i) = i & (-i)
```

This extracts the **lowest set bit** of i. Let's see why it works:

```
In two's complement arithmetic, -i is the bitwise NOT of i, plus 1.
This flips all bits up to and including the lowest set bit, and
leaves the lowest set bit itself intact.

Example: i = 12 = 1100 in binary
  -i  = ...10100  (flip bits of 1100, add 1: 0011 + 1 = 0100, so 12 → ...10100)
  Wait, let's be precise with 8 bits:
  i  = 00001100  (12)
  ~i = 11110011  (bitwise NOT)
  -i = 11110100  (add 1: 11110011 + 1 = 11110100)
  i & (-i) = 00001100 & 11110100 = 00000100 = 4

  The lowest set bit of 12 (binary 1100) is the "4" bit (bit position 2).
  lowbit(12) = 4. ✓

More examples:
  i=1  (0001): lowbit = 1
  i=2  (0010): lowbit = 2
  i=3  (0011): lowbit = 1
  i=4  (0100): lowbit = 4
  i=5  (0101): lowbit = 1
  i=6  (0110): lowbit = 2
  i=7  (0111): lowbit = 1
  i=8  (1000): lowbit = 8
```

Each index i in the Fenwick tree stores the aggregate of the `lowbit(i)` elements
ending at i. Index i is "responsible for" the range `(i - lowbit(i), i]`.

## Layer Position

```
DT02: tree
DT03: binary-tree
DT04: heap
DT05: segment-tree         ← more general, more complex sibling
DT06: fenwick-tree         ← [YOU ARE HERE]

DT07+ : search trees (different branch)
```

**Depends on:** Nothing! Pure bit arithmetic on a flat array.
**Related to:** DT05 (Segment Tree) — a more powerful but more complex alternative.
**Used by:** Counting inversions, order statistics, coordinate-compressed range
sums, competitive programming.

## Concepts

### What Each Cell Stores

The Fenwick tree is a 1-indexed array `bit[]`. Cell `bit[i]` stores the sum of
`lowbit(i)` consecutive elements of the original array, ending at position i.

```
Index (1-based): 1    2    3    4    5    6    7    8
Binary:          001  010  011  100  101  110  111  1000
lowbit:          1    2    1    4    1    2    1    8
Range covered:   [1]  [1,2] [3] [1,4] [5] [5,6] [7] [1,8]
```

Let's be precise: `bit[i]` = sum of `arr[i - lowbit(i) + 1 .. i]`.

```
bit[1] covers arr[1..1]    (length 1)  → arr[1]
bit[2] covers arr[1..2]    (length 2)  → arr[1] + arr[2]
bit[3] covers arr[3..3]    (length 1)  → arr[3]
bit[4] covers arr[1..4]    (length 4)  → arr[1]+arr[2]+arr[3]+arr[4]
bit[5] covers arr[5..5]    (length 1)  → arr[5]
bit[6] covers arr[5..6]    (length 2)  → arr[5] + arr[6]
bit[7] covers arr[7..7]    (length 1)  → arr[7]
bit[8] covers arr[1..8]    (length 8)  → all of arr[1..8]
```

Visual representation (each cell's coverage):

```
Index:  1   2   3   4   5   6   7   8
        ■               ■           ■      ← arr[1]
            ■■          ■           ■      ← arr[2] (bit[2] covers [1,2])
                ■                   ■      ← arr[3]
            ■■■■                    ■      ← arr[4] (bit[4] covers [1,4])
                        ■           ■      ← arr[5]
                        ■■          ■      ← arr[6] (bit[6] covers [5,6])
                                ■   ■      ← arr[7]
            ████████████████████████      ← bit[8] covers [1,8]

bit[4] = sum of all ■ in column 4 from the diagram above
       = arr[1] + arr[2] + arr[3] + arr[4]
```

### Prefix Sum Query: Walk Downward

To compute the prefix sum of arr[1..i], we sum a set of BIT cells. Which cells?
Those whose coverage exactly tiles the range [1..i] without overlap or gap.

The algorithm: start at i, add `bit[i]`, then move to `i - lowbit(i)`, repeat
until i = 0.

```
prefix_sum(i):
  total = 0
  while i > 0:
    total += bit[i]
    i -= lowbit(i)  # drop the lowest set bit
  return total
```

**Why does this work?** Each step strips the lowest set bit from i. The cells
visited cover non-overlapping, contiguous ranges that together span [1..i].

Let's trace `prefix_sum(7)` (sum of arr[1..7]):

```
i = 7 (binary 111):
  add bit[7] → covers arr[7..7]
  i = 7 - lowbit(7) = 7 - 1 = 6

i = 6 (binary 110):
  add bit[6] → covers arr[5..6]
  i = 6 - lowbit(6) = 6 - 2 = 4

i = 4 (binary 100):
  add bit[4] → covers arr[1..4]
  i = 4 - lowbit(4) = 4 - 4 = 0

i = 0: stop.

Total = bit[7] + bit[6] + bit[4]
      = arr[7] + (arr[5]+arr[6]) + (arr[1]+arr[2]+arr[3]+arr[4])
      = arr[1] + arr[2] + arr[3] + arr[4] + arr[5] + arr[6] + arr[7] ✓
```

The number of steps is at most the number of set bits in i (at most log n steps).

Let's visualize which cells are visited for several prefix queries:

```
prefix_sum(1):  visits bit[1]              → 1 step
prefix_sum(2):  visits bit[2]              → 1 step (bit[2] covers [1,2])
prefix_sum(3):  visits bit[3], bit[2]      → 2 steps
prefix_sum(4):  visits bit[4]              → 1 step (bit[4] covers [1,4])
prefix_sum(5):  visits bit[5], bit[4]      → 2 steps
prefix_sum(6):  visits bit[6], bit[4]      → 2 steps
prefix_sum(7):  visits bit[7], bit[6], bit[4] → 3 steps
prefix_sum(8):  visits bit[8]              → 1 step (bit[8] covers [1,8])
```

Power-of-2 queries are the fastest (1 step). The worst case is when i has many
set bits, e.g., i = 2^k - 1 (binary 111...1), which takes k = log n steps.

### Point Update: Walk Upward

To add `delta` to arr[i], we must update all BIT cells that cover position i.
Which cells cover i? Those at indices formed by adding lowbit(i) repeatedly.

```
update(i, delta):
  while i <= n:
    bit[i] += delta
    i += lowbit(i)  # climb to next responsible ancestor
```

**Why `i += lowbit(i)`?** Think of it as "who else covers this position?" If
`bit[i]` covers range `(i - lowbit(i), i]`, the next cell that also covers
position i is at `i + lowbit(i)` — because that cell's range extends left far
enough to include i.

Let's trace `update(3, delta)` (updating arr[3]):

```
i = 3 (binary 011):
  update bit[3] (covers [3,3] — includes index 3)
  i = 3 + lowbit(3) = 3 + 1 = 4

i = 4 (binary 100):
  update bit[4] (covers [1,4] — includes index 3)
  i = 4 + lowbit(4) = 4 + 4 = 8

i = 8 (binary 1000):
  update bit[8] (covers [1,8] — includes index 3)
  i = 8 + lowbit(8) = 8 + 8 = 16

i = 16 > n (assume n = 8): stop.

Cells updated: bit[3], bit[4], bit[8] — exactly the cells that cover position 3. ✓
```

### Worked Example: Full Trace

Array: `[3, 2, 1, 7, 4]` (1-indexed)

**Build the BIT (O(n log n) approach — update each element):**

Start with `bit = [0, 0, 0, 0, 0, 0]` (index 0 unused).

```
update(1, 3): bit[1]+=3, bit[2]+=3, bit[4]+=3
  bit = [_, 3, 3, 0, 3, 0]

update(2, 2): bit[2]+=2, bit[4]+=2
  bit = [_, 3, 5, 0, 5, 0]

update(3, 1): bit[3]+=1, bit[4]+=1
  bit = [_, 3, 5, 1, 6, 0]

update(4, 7): bit[4]+=7
  bit = [_, 3, 5, 1, 13, 0]

update(5, 4): bit[5]+=4
  bit = [_, 3, 5, 1, 13, 4]
```

Final BIT: `[_, 3, 5, 1, 13, 4]`

Verify:
- bit[1] = arr[1] = 3 ✓
- bit[2] = arr[1]+arr[2] = 3+2 = 5 ✓
- bit[3] = arr[3] = 1 ✓
- bit[4] = arr[1]+arr[2]+arr[3]+arr[4] = 3+2+1+7 = 13 ✓
- bit[5] = arr[5] = 4 ✓

**Query: prefix_sum(3)** (sum of arr[1..3]):

```
i=3: add bit[3] = 1. i = 3 - 1 = 2.
i=2: add bit[2] = 5. i = 2 - 2 = 0.
Stop. Total = 1 + 5 = 6. 
Verify: arr[1]+arr[2]+arr[3] = 3+2+1 = 6. ✓
```

**Range sum [2, 4]:** `prefix_sum(4) - prefix_sum(1)`

```
prefix_sum(4): i=4, add bit[4]=13, i=0. Total = 13.
prefix_sum(1): i=1, add bit[1]=3,  i=0. Total = 3.
Range sum = 13 - 3 = 10.
Verify: arr[2]+arr[3]+arr[4] = 2+1+7 = 10. ✓
```

**Point update: arr[3] += 5** (set arr[3] = 6):

```
i=3: bit[3] += 5 → bit[3] = 6. i = 3+1 = 4.
i=4: bit[4] += 5 → bit[4] = 18. i = 4+4 = 8 > n. Stop.

New BIT: [_, 3, 5, 6, 18, 4]

Verify prefix_sum(4): bit[4] = 18 = 3+2+6+7 = 18. ✓
```

### O(n) Build: Direct Construction

The O(n log n) build (call update n times) works, but we can build in O(n) by
constructing the BIT values directly:

```python
def build(arr: list) -> list:
    """Build BIT from array in O(n). arr is 0-indexed; returns 1-indexed BIT."""
    n = len(arr)
    bit = [0] * (n + 1)
    for i in range(1, n + 1):
        bit[i] += arr[i - 1]         # add this element
        parent = i + (i & -i)        # climb to next cell
        if parent <= n:
            bit[parent] += bit[i]    # propagate
    return bit
```

This works because each cell bit[i] needs to accumulate its range, and each cell
propagates its value to its "parent" in O(1). Total: O(n).

## Comparison with Segment Tree

| Feature | Fenwick Tree | Segment Tree |
|---|---|---|
| Space | O(n) | O(4n) |
| Build time | O(n) or O(n log n) | O(n) |
| Point update | O(log n) | O(log n) |
| Prefix query | O(log n) | O(log n) |
| Range query | O(log n) (two prefix sums) | O(log n) |
| Range update | Hard (needs two BITs) | Easy (lazy propagation) |
| Arbitrary combine | Difficult (must be invertible) | Easy (any associative fn) |
| Code complexity | Very simple (~10 lines) | Moderate (~30-50 lines) |
| Cache performance | Excellent | Good |
| Constant factor | Very small | Small |

**Use Fenwick when:**
- You only need prefix sums (or prefix aggregates with an invertible operation)
- You want minimal code and maximum speed
- You don't need range updates

**Use Segment Tree when:**
- You need range updates (lazy propagation)
- Your combine function isn't easily invertible (min, max, GCD)
- You need more complex queries (range min + point update)

### Why Are Invertible Operations Required for Fenwick Range Queries?

A Fenwick tree computes `range_sum(l, r) = prefix_sum(r) - prefix_sum(l-1)`.
The subtraction here is the INVERSE of addition. For this to work, the operation
must have an inverse.

- Sum: inverse is subtraction. Fenwick works. ✓
- Product: inverse is division. Fenwick works (if no zeros). ✓
- Min/Max: NO inverse. `min(prefix_min(r), something) = prefix_min(l-1)` cannot
  be solved for "something". Fenwick does NOT work for range min. ✗
- XOR: inverse is XOR itself. Fenwick works for prefix XOR. ✓

For min/max queries, use a segment tree.

## Representation

```
class FenwickTree:
    _bit: list[int]   # 1-indexed array; _bit[0] unused, _bit[1..n] are the tree
    _n: int           # length of the original array
```

The BIT array is the entire data structure. No auxiliary pointers, no node
objects, no extra storage.

## Algorithms (Pure Functions)

```python
def build(array: list[int]) -> list[int]:
    """
    Build a Fenwick tree from a 0-indexed array.
    Returns a 1-indexed list (index 0 is unused sentinel 0).
    O(n) time.
    """
    n = len(array)
    bit = [0] * (n + 1)
    for i in range(1, n + 1):
        bit[i] += array[i - 1]
        parent = i + (i & -i)
        if parent <= n:
            bit[parent] += bit[i]
    return bit

def update(bit: list[int], i: int, delta: int) -> list[int]:
    """
    Add delta to position i (1-indexed) in the original array.
    Returns updated BIT (mutates in place — copy first for functional style).
    O(log n) time.
    """
    new_bit = list(bit)
    n = len(bit) - 1
    while i <= n:
        new_bit[i] += delta
        i += i & (-i)   # i += lowbit(i)
    return new_bit

def prefix_sum(bit: list[int], i: int) -> int:
    """
    Sum of original array[1..i] (1-indexed).
    O(log n) time.
    """
    total = 0
    while i > 0:
        total += bit[i]
        i -= i & (-i)   # i -= lowbit(i)
    return total

def range_sum(bit: list[int], left: int, right: int) -> int:
    """
    Sum of original array[left..right] (1-indexed, inclusive).
    O(log n) time.
    """
    if left > right:
        return 0
    if left == 1:
        return prefix_sum(bit, right)
    return prefix_sum(bit, right) - prefix_sum(bit, left - 1)

def point_query(bit: list[int], i: int) -> int:
    """
    Value of original array[i] (1-indexed).
    range_sum(i, i) works but this is slightly more readable.
    O(log n) time.
    """
    return range_sum(bit, i, i)

# --- Useful extension: find k-th element ---

def find_kth(bit: list[int], k: int) -> int:
    """
    Find smallest index i such that prefix_sum(i) >= k.
    (Assumes all values are non-negative.)
    O(log n) time using binary lifting.
    """
    n = len(bit) - 1
    idx = 0
    log = n.bit_length()
    for shift in range(log, -1, -1):
        next_idx = idx + (1 << shift)
        if next_idx <= n and bit[next_idx] < k:
            idx = next_idx
            k -= bit[idx]
    return idx + 1
```

## Public API

```python
class FenwickTree:
    """
    Binary Indexed Tree (Fenwick Tree) for prefix sums with point updates.

    1-indexed externally: positions 1 through n.

    Example:
        ft = FenwickTree([3, 2, 1, 7, 4])
        ft.prefix_sum(3)      # → 6  (3+2+1)
        ft.range_sum(2, 4)    # → 10 (2+1+7)
        ft.update(3, 5)       # arr[3] += 5  → arr is now [3,2,6,7,4]
        ft.prefix_sum(3)      # → 11 (3+2+6)
        ft.find_kth(6)        # → 3 (prefix sums: [3,5,11,...]; first ≥6 is at idx 3)
    """

    def __init__(self, array: list[int]):
        """Build from 0-indexed array in O(n)."""
        ...

    # --- Core operations ---
    def update(self, i: int, delta: int) -> None:
        """Add delta to position i (1-indexed). O(log n)."""
        ...

    def prefix_sum(self, i: int) -> int:
        """Sum of positions 1..i (1-indexed, inclusive). O(log n)."""
        ...

    def range_sum(self, left: int, right: int) -> int:
        """Sum of positions left..right (1-indexed, inclusive). O(log n)."""
        ...

    def point_query(self, i: int) -> int:
        """Value at position i (1-indexed). O(log n)."""
        ...

    # --- Advanced ---
    def find_kth(self, k: int) -> int:
        """
        Smallest index i where prefix_sum(i) >= k.
        Requires all values non-negative.
        Used for order statistics. O(log n).
        """
        ...

    # --- Queries ---
    def __len__(self) -> int: ...   # n (length of original array)
```

## Composition Model

### Inheritance (Python, Ruby, TypeScript)

A `FenwickTree` is self-contained — it doesn't inherit from `SegmentTree` or any
tree class. The "binary tree" relationship is conceptual, not structural.
Optionally create a `FenwickTree2D` subclass for 2D prefix sums.

```python
class FenwickTree:
    """1D prefix sums."""
    def __init__(self, n: int): ...

class FenwickTree2D(FenwickTree):
    """2D prefix sums. Each row is its own FenwickTree."""
    def __init__(self, rows: int, cols: int): ...
    def update(self, r: int, c: int, delta: int): ...
    def prefix_sum(self, r: int, c: int) -> int: ...
```

### Composition (Rust, Go)

```rust
pub struct FenwickTree {
    bit: Vec<i64>,
    n: usize,
}

impl FenwickTree {
    pub fn new(n: usize) -> Self { ... }
    pub fn from_slice(arr: &[i64]) -> Self { ... }
    pub fn update(&mut self, i: usize, delta: i64) { ... }
    pub fn prefix_sum(&self, i: usize) -> i64 { ... }
    pub fn range_sum(&self, l: usize, r: usize) -> i64 { ... }
}
```

```go
type FenwickTree struct {
    bit []int
    n   int
}

func NewFenwickTree(n int) *FenwickTree { ... }
func NewFromSlice(arr []int) *FenwickTree { ... }
func (ft *FenwickTree) Update(i, delta int) { ... }
func (ft *FenwickTree) PrefixSum(i int) int { ... }
func (ft *FenwickTree) RangeSum(l, r int) int { ... }
```

### Module (Elixir, Lua, Perl)

```elixir
defmodule FenwickTree do
  # Tree is a tuple {n, map_of_bit_values}
  # Using a Map for 1-indexed array (could use a tuple for fixed-size)

  def new(n), do: {n, Map.new(1..n, fn i -> {i, 0} end)}

  def update({n, bit}, i, delta) do
    new_bit = update_loop(bit, i, delta, n)
    {n, new_bit}
  end

  defp update_loop(bit, i, delta, n) when i > n, do: bit
  defp update_loop(bit, i, delta, n) do
    bit
    |> Map.update!(i, &(&1 + delta))
    |> update_loop(i + (i &&& -i), delta, n)
  end

  def prefix_sum({_n, bit}, i), do: prefix_loop(bit, i, 0)

  defp prefix_loop(_bit, 0, acc), do: acc
  defp prefix_loop(bit, i, acc) do
    prefix_loop(bit, i - (i &&& -i), acc + Map.get(bit, i, 0))
  end

  def range_sum(ft, l, r) do
    prefix_sum(ft, r) - prefix_sum(ft, l - 1)
  end
end
```

### Swift

```swift
struct FenwickTree {
    private var bit: [Int]
    let n: Int

    init(_ array: [Int]) {
        n = array.count
        bit = [Int](repeating: 0, count: n + 1)
        for (i, v) in array.enumerated() {
            update(i + 1, delta: v)
        }
    }

    mutating func update(_ i: Int, delta: Int) {
        var i = i
        while i <= n { bit[i] += delta; i += i & -i }
    }

    func prefixSum(_ i: Int) -> Int {
        var i = i; var total = 0
        while i > 0 { total += bit[i]; i -= i & -i }
        return total
    }

    func rangeSum(_ l: Int, _ r: Int) -> Int {
        prefixSum(r) - prefixSum(l - 1)
    }
}
```

## Test Strategy

### Correctness Against Brute Force

```python
import random

def brute_prefix(arr, i):
    return sum(arr[:i])

def brute_range(arr, l, r):
    return sum(arr[l-1:r])  # l, r are 1-indexed

for _ in range(500):
    n = random.randint(1, 100)
    arr = [random.randint(-50, 50) for _ in range(n)]
    ft = FenwickTree(arr)

    # Test all prefix sums
    for i in range(1, n + 1):
        assert ft.prefix_sum(i) == brute_prefix(arr, i)

    # Test all range sums
    for l in range(1, n + 1):
        for r in range(l, n + 1):
            assert ft.range_sum(l, r) == brute_range(arr, l, r)
```

### Update Correctness

```python
arr = [3, 2, 1, 7, 4]
ft = FenwickTree(arr)

# Update: arr[3] += 5 (1-indexed)
arr[2] += 5  # 0-indexed equivalent
ft.update(3, 5)

for i in range(1, 6):
    assert ft.prefix_sum(i) == brute_prefix(arr, i)
```

### Lowbit Correctness

```python
def lowbit(i):
    return i & (-i)

# Verify for all i from 1 to 64
expected = {1:1, 2:2, 3:1, 4:4, 5:1, 6:2, 7:1, 8:8,
            16:16, 32:32, 64:64, 12:4, 6:2, 24:8}
for i, expected_lb in expected.items():
    assert lowbit(i) == expected_lb
```

### find_kth Correctness

```python
arr = [1, 2, 3, 4, 5]  # prefix sums: 1, 3, 6, 10, 15
ft = FenwickTree(arr)

assert ft.find_kth(1) == 1   # first prefix sum ≥ 1 is at index 1 (sum=1)
assert ft.find_kth(2) == 2   # first prefix sum ≥ 2 is at index 2 (sum=3)
assert ft.find_kth(3) == 2   # first prefix sum ≥ 3 is at index 2 (sum=3)
assert ft.find_kth(4) == 3   # first prefix sum ≥ 4 is at index 3 (sum=6)
assert ft.find_kth(10) == 4  # first ≥ 10 is at index 4 (sum=10)
assert ft.find_kth(11) == 5  # first ≥ 11 is at index 5 (sum=15)
```

### Stress Test: Interleaved Updates and Queries

```python
import random

n = 1000
arr = [random.randint(1, 100) for _ in range(n)]
ft = FenwickTree(arr)

for _ in range(10_000):
    op = random.choice(['query', 'update'])
    if op == 'query':
        l = random.randint(1, n)
        r = random.randint(l, n)
        expected = sum(arr[l-1:r])
        assert ft.range_sum(l, r) == expected
    else:
        i = random.randint(1, n)
        delta = random.randint(-50, 50)
        arr[i-1] += delta
        ft.update(i, delta)
```

### Edge Cases

- Single-element array
- All zeros
- Negative values
- Update that sets a value to 0
- prefix_sum(0) should return 0 (empty prefix)
- prefix_sum(n) should equal sum of all elements

### Coverage Targets

- 95%+ line coverage
- `update` tested with multiple propagation steps (update index 1, which propagates all the way to bit[n] if n is a power of 2)
- `prefix_sum` tested with i=1, i=n, powers of 2, non-powers of 2
- `find_kth` tested with k=1, k=total_sum, and values in between

## Future Extensions

- **2D Fenwick tree** — prefix sums over a 2D matrix. Each row is a Fenwick tree;
  updates and queries are O(log m * log n) for an m × n matrix. Used in 2D range
  sum queries and problems like "count points in a rectangle."
- **Range update + point query** — by storing difference array increments instead
  of raw values, we can apply range updates in O(log n) and answer point queries
  in O(log n). This is the dual of the basic Fenwick tree.
- **Range update + range query** — use two Fenwick trees simultaneously to support
  both range updates and range queries in O(log n). Derivation uses the algebraic
  structure of prefix sums.
- **Order statistics** — with non-negative integer values treated as frequencies,
  the Fenwick tree naturally represents a frequency array. `find_kth` then gives
  the k-th smallest element in O(log n). This is the basis for counting inversions
  in O(n log n): for each element, count how many previous elements are greater.
- **Coordinate compression** — when values are large but sparse, map them to small
  indices first. The Fenwick tree operates on the compressed indices, giving
  O(n log n) algorithms for problems like "number of elements smaller than x
  seen so far."
