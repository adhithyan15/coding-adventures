# @coding-adventures/microbenchmark

Collects adjacent, balanced baseline/candidate timing pairs, preserves raw
observations, and analyzes the candidate-to-baseline median ratio with the
exact median interval from `@coding-adventures/rank-statistics`.

```ts
import { analyze, type Pair } from "@coding-adventures/microbenchmark";
const observations: Pair[] = [
  { baseline_ns: 100, candidate_ns: 80, order: "baseline-first" },
];
console.log(analyze(observations, 95, 1.05));
```

Run the indexed traversal comparison with `npx array-microbenchmark`. This
lane requires Node.js 22.15 or newer. It uses the native JSON reviver token source
and `JSON.rawJSON` to preserve signed-64-bit integer fields exactly; batch
counts, checksums, clock ticks, and raw pair durations use `bigint` in memory.
Only candidate and baseline durations are converted to binary64 for ratio
analysis.
The public `analyze` API accepts at most 10,000 pairs. It checks input shape
and scalar options before rejecting an oversized list, and does so before
mapping or validating pair members.

The portable pair-analysis fixture is included under `testdata/` so packaged
conformance tools can replay the same cases. The published manifest declares
the semver dependency on `@coding-adventures/rank-statistics`; the repository
lockfile resolves that version to the sibling source package for local builds.
A registry release must publish rank-statistics first.
