# Rank statistics for TypeScript

This package provides finite-sample median and exact, distribution-free confidence intervals for a population median. It is a pure numerical library: callers supply the observations, while benchmark consumers remain responsible for measurement collection and practical verdict thresholds.

## API

```ts
import { median, medianInterval } from "@coding-adventures/rank-statistics";

median([1.25, 0.75, 1.0]);
// 1

medianInterval([0.97, 1.01, 1.04, 0.99, 1.02], 70);
// { low: 0.97, high: 1.04 }
```

`median(values)` returns the middle observation or the correctly rounded arithmetic midpoint of the two middle observations. It does not modify its input. `medianInterval(values, confidencePercent)` returns a closed interval whose endpoints are observations, or `null` when the exact rank cutoff admits no interval.

Both functions accept a nonempty dense JavaScript array of finite numbers. They do not coerce values. Invalid input is reported with `TypeError` or `RangeError` according to the validation rules in [VER07c](../../../specs/VER07c-rank-statistics.md#typescript-package-profile). `medianInterval` validates the integer confidence percentage from 1 through 99 before inspecting the sample.

## Complexity and arithmetic

Validation snapshots the array once in O(n) space and time. Sorting uses the host JavaScript engine's `Array.sort`; ECMAScript does not promise a portable time bound. The interval cutoff uses exact `BigInt` arithmetic and checks at most n candidate ranks; integer width grows with n. The even-sample median forms the exact dyadic midpoint and rounds once to binary64 using ties-to-even.

The checked-in [`testdata/microbenchmark-pairs-v1.json`](testdata/microbenchmark-pairs-v1.json) is a package-local copy of the shared fixture for consumer conformance. This package does not read that fixture at runtime.

## Development

Run `npm ci`, `npm run build`, and `npm run test:coverage`. The test suite includes deterministic exact-oracle campaigns and the canonical fixture replay. Coverage thresholds apply to production source files only; the campaign counts and raw V8 denominators are reported by the test run.
