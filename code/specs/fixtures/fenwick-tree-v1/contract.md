# DT06 observable contract (v1)

A Fenwick tree stores an exact sequence of signed integer point values. The
portable domain is bounded by `sum(abs(current_values)) <= 9007199254740991`
(2^53-1); this keeps every intermediate sum exact even in implementations
whose numeric storage is IEEE-754 binary64. Construction and successful updates
must preserve that invariant; every constructor value and update delta must also
individually lie in `[-(2^53-1), 2^53-1]`. Check the update index before the
delta and resulting-state bounds. Inputs of non-integer type (including a
non-integral numeric value) are outside v1 adapter conformance; adapters may
reject them according to their native type system. Reject invalid or out-of-domain input before
mutation. The fixture expects integral values and deltas only.

* Construction accepts length zero and arbitrary signed values in-domain; a
  negative length is invalid.
* `prefix_sum(i)` accepts `0 <= i <= n`; index zero yields 0.
* `range_sum(l,r)` requires `l <= r` first, then `1 <= l <= r <= n`.
  Thus a reversed range is reported as invalid range even if an endpoint is
  also out of range.
* `point_query(i)` and `update(i,delta)` require `1 <= i <= n`.
* Updates add delta to exactly one point. Rejected updates preserve every point
  and every query result.
* `find_kth(k)` requires a non-empty tree, then `k > 0`, then `k <= total`.
  It returns the least `i` for which prefix sum is at least k. This operation
  has the precondition that all current point values are nonnegative; negative
  updates are allowed when they leave every point nonnegative. All-zero trees
  reject every positive k.
* Negative point values and signed prefix/range sums are valid for all
  operations other than `find_kth`.

For each implementation, compare results to an independent flat-array oracle;
do not derive expected results from another Fenwick implementation. Exhaust
all arrays of lengths 0 through 6 over {-1,0,1}, then all legal updates with
deltas {-1,0,1} that remain in the portable domain, checking every prefix,
range, and point query after each operation. For `find_kth`, use only
nonnegative arrays and every integer k from 1 through total+1. This finite
space is small enough to run in routine validation and catches propagation
errors at power-of-two boundaries. Include lengths 7, 8, and 9 in focused
fixtures to test parent propagation across and beyond powers of two.
