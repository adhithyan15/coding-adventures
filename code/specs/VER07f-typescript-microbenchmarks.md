# VER07f — TypeScript paired microbenchmark lane

The TypeScript lane implements the paired experiment in
[VER07b](VER07b-polyglot-microbenchmark-comparisons.md), the rank-statistics
contract in [VER07c](VER07c-rank-statistics.md), and the collector and indexed
array workload in [VER07d](VER07d-reference-microbenchmarks.md). It provides
two publishable packages: `rank-statistics` for reusable exact median
statistics, and `microbenchmark` for pairing, analysis, collection, and the
`indexed-array-sum-4096` command.

## Exact integer profile

Integer fields, checksums, and clock ticks preserve VER07d's signed-64-bit
range. The lane requires Node.js 22.15 or newer, whose native `JSON.parse`
reviver exposes the original number token (`context.source`) and whose
`JSON.rawJSON` emits exact integer tokens. Parsing converts out-of-safe-range
integer tokens to `bigint`; serialization writes `bigint` values as raw JSON
number tokens. Values supplied through the API may be safe integer numbers or
signed-64-bit `bigint`s. Clock readings and batch counts use `bigint` internally.
Both durations are converted to binary64 only when forming their ratio, as
VER07d requires. Statistical samples remain finite binary64 values as specified
by VER07c.

## Repository dependency identity

`microbenchmark` is independently publishable and declares an exact semver
dependency on `@coding-adventures/rank-statistics`. The tracked npm lockfile
uses a sibling link for repository builds. The dependency inventory may count
this as local only when the lockfile is tracked and records an exact link from
the dependency's `node_modules` entry to a tracked in-repository package
manifest with the same package name and exact version. The lockfile root must
match the package manifest and declare the same dependency. The inventory
accepts this local-link classification only with numeric `lockfileVersion: 3`;
missing, string, boolean, and other numeric versions are not supported by this
rule. Ranges, npm aliases, untracked or mismatched targets, remote lock entries,
and links that resolve outside the repository are not local. This rule leaves the published
`package.json` semver metadata intact; it does not approve arbitrary scoped
package names or grow the external dependency allowlist.

## Validation

Both packages replay the portable fixture. The rank-statistics tests compare
the exact binomial cutoff with an independent bounded sign-assignment oracle,
including ties and confidence cutoffs. Generated property tests use fixed seeds
and bounded local generators. Collector tests inject a clock and check warmup,
calibration, balanced order, checksum validation, errors, and report replay.
The array command checks its known checksum without a timing threshold.
