# Fenwick tree

An in-memory Dart implementation of the shared [DT06 contract](../../../specs/DT06-fenwick-tree.md).
It stores signed integers and answers prefix, inclusive range, point and
nonnegative-frequency order-statistic queries. Indices are one-based, with
`prefixSum(0)` representing the empty prefix.

```dart
import 'package:coding_adventures_fenwick_tree/coding_adventures_fenwick_tree.dart';

final sums = FenwickTree.fromList([0, 2, 0, 3]);
print(sums.rangeSum(2, 4)); // 5
sums.update(4, -2);
print(sums.findKth(3)); // 4
```

`FenwickTree(size)` creates zero-valued points. `FenwickTree.fromList(values)`
owns a copy of its input. Every stored state must satisfy
`sum(abs(points)) <= 9007199254740991`, and each input value and update delta
must individually be within the same signed bound. Construction and updates
reject values outside that domain. Validation completes before an update
changes points, interval cells or negative-frequency bookkeeping.

`FenwickException.code` exposes the shared identifiers `negative_size`,
`out_of_domain`, `index_out_of_range`, `invalid_range`, `empty`,
`nonpositive_target` and `exceeds_total`. `rangeSum` validates reversal before
endpoints. `update` validates the index before the delta and resulting L1
budget. `findKth` validates empty, nonpositive and above-total targets in that
order, then rejects a tree containing negative points with the package-specific
identifier `negative_frequency`. Negative deltas remain valid and can restore
a tree's nonnegative-frequency precondition.

Construction uses linear propagation and O(n) memory. Prefix and range queries,
updates and `findKth` use O(log n) interval steps. Point queries use O(1).
The runtime library uses Dart integers and memory allocation with no file,
network, clock or process access. Native Dart is the validated execution target.

## Validation

Run `sh BUILD` from this directory. The Windows front uses the same commands.
Both fronts format-check, analyze, execute the native test suite and run
`dart run tool/mutation_check.dart`.

The tests consume all 19 shared fixture cases against the actual implementation.
Their independent oracle uses flat-array loops. The bounded exhaustive suite
covers all 1,093 arrays of lengths 0 through 6 over `{-1, 0, 1}`, constructs a fresh
tree for each valid index and each delta in `{-1, 0, 1}`, and checks all prefix,
point and inclusive range queries. Counts are asserted in the test:
18,045 updates, 19,138 state audits and 596,092 query results. The separate
nonnegative domain uses all 1,093 arrays of lengths 0 through 6 over `{0, 1, 2}`
and all targets `1..total+1`, totaling 7,108 target cases, including refusals.
These domains exhaust one-step updates from each starting array. Focused
sequences cover lengths 7, 8 and 9, portable numeric endpoints, native minimum
integer rejection, error precedence, input ownership and rejection atomicity.

The mutation runner makes temporary copies of the production source, compiles
them and demands test assertion failures for nine concrete defects: omitted
constructor/update propagation, prefix/range endpoint mistakes, wrong kth
equality, missing L1 and delta bounds, mutation before rejection and stale
negative-frequency bookkeeping. Dependency/bootstrap or compilation failures
do not count as detected mutants.

These are finite executable checks. The package makes no formal proof claim,
and the runtime capability declaration describes the library's direct effects.
