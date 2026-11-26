# VER07k — .NET paired microbenchmark ports

This spec adds native C# and F# implementations of the paired comparison and reusable rank-statistics contracts in [VER07b](VER07b-polyglot-microbenchmark-comparisons.md), [VER07c](VER07c-rank-statistics.md), and [VER07d](VER07d-reference-microbenchmarks.md). Each language publishes two independently consumable NuGet packages: rank statistics and microbenchmark collection/analysis. The F# public surface must use idiomatic F# types and functions; the C# and F# implementations remain separate packages and do not require one another at runtime.

## Rank-statistics packages

Each package provides a median over finite binary64 samples and the exact two-sided order-statistic confidence interval for the median. Inputs are non-empty and are not mutated. Even medians use nearest ties-to-even and remain finite for all finite inputs. Confidence is an integer percentage from 1 through 99. The largest rank `k` is chosen exactly from

```text
200 * sum(C(n,j), j=0..k-1) <= (100-confidence) * 2^n
```

with no floating-point probability cutoff; no qualifying rank returns no interval. Invalid input is reported through the language's normal error/result conventions. C# and F# may use .NET `BigInteger` for this exact calculation. Tests use an independent bounded sign-assignment oracle for small samples, frozen wide-cutoff vectors, deterministic generated properties, and the shared fixture.

## Microbenchmark packages

Each package exposes paired collection, analysis, and replay. Both variants first run one verified operation outside timing. Calibration doubles a common batch size until both durations meet the target or the configured maximum is exhausted. Measurements run adjacently in a seeded balanced schedule (baseline-first gets the extra pair when odd); one failure returns no partial report. Clock ticks must be nonnegative and nonregressing; measured durations must be positive and all checksum/tick arithmetic must fit signed 64-bit integers. The default configuration is 40 pairs, confidence 95%, factor 1.05, seed 0, target 100,000 ns, and maximum batch 65,536; pair count is 1 through 10,000.

Reports retain raw pairs and use only the shared JSON fields: `config` (`pairs`, `confidence_percent`, `minimum_factor`, `seed`, `target_ns`, `max_batch`), `batch_size`, `pairs` (`baseline_ns`, `candidate_ns`, `order`), and `analysis` (`ratio`, `interval`, `verdict`), plus standard CLI metadata `workload`, `invocation`, `revision`, `runtime`, `os`, `arch`, and optional `profile`. Standard metadata is either entirely absent or present as a complete set; `profile` may appear only with the complete set. Present metadata strings and invocation entries are nonempty. A present `workload` must be a string containing no NUL and at least one character outside ASCII whitespace (U+0020 or U+0009 through U+000D); Unicode whitespace such as U+2003 is allowed. Integer values are serialized and replayed losslessly as signed 64-bit JSON integers. Replay rejects duplicate or unknown fields, missing or invalid fields, unbalanced order counts, out-of-range integers, and derived analysis that differs from the raw pairs. It does not authenticate observations or metadata cryptographically.

For predictable replay costs, the collector and report parser accept at most 10,000 pairs, 4 MiB of UTF-8 JSON, 128 levels of nesting, and 65,536 JSON values. These bounds allow replay of any report produced by the collector.

Both analyzers convert each positive duration to binary64 before candidate/baseline division and apply the exact threshold rules: `faster` when the upper endpoint is strictly below `1/factor`, `slower` when the lower endpoint is strictly above `factor`, `equivalent` when the complete interval is within the closed band, otherwise `inconclusive`.

## Indexed-array programs and packaging

Each native CLI builds the 4,096-element array before timing, with value `(17*i + 3) % 251`, and compares indexed access against the language's native iteration. Each operation sums the entire array and must match the fixed checksum. Public batch helpers reject negative operation counts and return zero for zero operations. Metadata is JSON-escaped and includes the standard fields only. Revision selection ignores blank environment values, falling back from `GIT_REVISION` to `GITHUB_SHA` to `unknown`; a blank explicit `--revision` is an error. No speed threshold is a correctness condition.

Every library is packable as a separate NuGet package with a versioned package identity and XML/readme documentation. Both language lanes include package-local copies of the canonical fixture; repository tests compare each copy byte-for-byte to `code/specs/fixtures/microbenchmark-pairs-v1.json`. Installed-package consumer tests reference each NuGet artifact and exercise public APIs without repository source paths. C# and F# BUILD files run tests and package/consumer validation. No Warrant dependency is added.

Windows `BUILD_windows` files are interpreted as individual `cmd /C` commands. A PowerShell helper must therefore be launched explicitly with `powershell -NoProfile -File <script.ps1>`; the consumer scripts use Windows PowerShell 5-compatible syntax. Installed-consumer validation packs the two packages into a temporary local feed and restores through a temporary `NuGet.Config` that explicitly lists that feed and the public NuGet v3 source. Package-source mapping sends `CodingAdventures.*` packages only to the local feed, while other dependencies use nuget.org. This avoids machine-level NuGet configuration and source-argument parsing differences and proves the consumer uses the just-packed artifacts. Temporary recursive cleanup is limited to the script's canonical GUID-named child of the canonical temp directory and rejects a reparse-point target.

## Validation limits

Fake clocks cover scheduling, calibration, positive and zero durations, regression, exhaustion, checksum errors, overflow, and callback failures without timing thresholds. Generated tests use fixed seeds. Each native CLI is run with 40 pairs and its output is replayed through the package API. Measurements are advisory and describe only the measured workload and environment, not physical cache behavior or another processor.
