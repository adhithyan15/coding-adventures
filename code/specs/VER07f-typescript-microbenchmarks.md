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
uses a sibling link for repository builds. The inventory treats it as local
only for a direct runtime `dependencies` entry with an ordinary exact `x.y.z`
version and numeric `lockfileVersion: 3`. The lock root must match the package
manifest and declare that exact dependency. Its `node_modules` link must resolve
exactly to `../<one sibling directory>`, and both the link and target entry must
match the tracked sibling manifest's name and version. The lock and manifests
must be regular stage-0 index entries, paths must remain inside the repository,
and any physical or tracked `npm-shrinkwrap.json` disables the exception.
Missing, string, boolean, unsupported-version, conflicted, range, alias,
untracked, mismatched, remote, and escaping path evidence remains external.
This rule leaves the published `package.json` semver metadata intact, approves
no other name-based exemption, and does not grow the external dependency
allowlist.

## Validation

Both packages replay the portable fixture. The rank-statistics tests compare
the exact binomial cutoff with an independent bounded sign-assignment oracle,
including ties and confidence cutoffs. Generated property tests use fixed seeds
and bounded local generators. Collector tests inject a clock and check warmup,
calibration, balanced order, checksum validation, errors, and report replay.
The array command checks its known checksum without a timing threshold.
