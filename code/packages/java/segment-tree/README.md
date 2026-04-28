# java/segment-tree

A generic segment tree for range queries and point updates in O(log n) (DT05).

## What it is

A segment tree is a binary tree where every node stores an aggregate
(sum, min, max, GCD, …) over a contiguous sub-range of an array.

```
Array:    [2,  1,  5,  3,  4]
Indices:   0   1   2   3   4

Segment tree (sum):
               [0,4] = 15
              /          \
       [0,2] = 8        [3,4] = 7
       /      \          /     \
  [0,1]=3  [2,2]=5  [3,3]=3  [4,4]=4
  /    \
[0,0]=2 [1,1]=1
```

Both operations take O(log n):
- **range query**: aggregate of `array[ql..qr]`
- **point update**: `array[i] = new_value`, re-computes ancestors

## Where it fits

```
DT03: binary-tree          ← structural parent
DT04: heap                 ← sibling (also array-backed)
DT05: segment-tree         ← [YOU ARE HERE]
  └── DT06: fenwick-tree   ← simpler alternative (sums only)
```

## API

```java
// Range sum:
SegmentTree<Integer> st = SegmentTree.sumTree(new int[]{2, 1, 5, 3, 4});
st.query(1, 3);   // → 9   (1 + 5 + 3)
st.update(2, 7);  // arr[2] is now 7
st.query(1, 3);   // → 11  (1 + 7 + 3)

// Range minimum:
SegmentTree<Integer> rm = SegmentTree.minTree(new int[]{5, 3, 7, 1, 9});
rm.query(0, 3);   // → 1

// Custom combine (range product):
Integer[] arr = {2, 3, 4, 5};
SegmentTree<Integer> prod = new SegmentTree<>(arr, (a, b) -> a * b, 1);
prod.query(0, 3); // → 120

// Reconstruct array:
st.toList();      // [2, 1, 7, 3, 4]  (after update above)
```

## Factory methods

| Method | combine | identity |
|--------|---------|----------|
| `sumTree(int[])` | `a + b` | 0 |
| `minTree(int[])` | `min(a, b)` | Integer.MAX_VALUE |
| `maxTree(int[])` | `max(a, b)` | Integer.MIN_VALUE |
| `gcdTree(int[])` | `gcd(a, b)` | 0 |

## Running tests

```bash
gradle test
```
