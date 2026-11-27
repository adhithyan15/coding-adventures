# Microbenchmark (Swift)

Paired native timing, exact median-ratio analysis, JSON report replay, and an
indexed-array benchmark executable for the VER07 contract.

## Dependencies

- `RankStatistics` local Swift package product.

## Usage

```swift
import Microbenchmark

let values = ArrayWorkload.makeValues()
let config = Configuration(pairs: 40, confidencePercent: 95, minimumFactor: 1.05)
let report = try collect(
    baseline: { operations in try ArrayWorkload.indexedBatch(values, operations: operations) },
    candidate: { operations in try ArrayWorkload.iteratorBatch(values, operations: operations) },
    expectedChecksum: ArrayWorkload.checksum,
    config: config
)
let json = try encodeReport(report)
let replayed = try replayReport(json)
print(replayed.analysis.verdict)
```

The default collector uses a monotonic `DispatchTime` clock; tests and callers
can inject a deterministic clock. Reports retain every raw pair and replay
recomputes analysis rather than trusting a supplied verdict. Signed `Int64`
fields remain JSON integer tokens and are range-checked by Codable decoding.
The optional workload field is validated as nonblank, NUL-free producer
metadata; replay accepts other workload identifiers without executing them.
The library's `runCommandLine` helper accepts revision metadata through its
injected `environment` dictionary and defaults to no ambient variables. The
packaged executable reads only `GIT_REVISION` and `GITHUB_SHA`, in that order.
Its `os` report field is a compile-time operating-system family label
(`windows`, `macos`, `linux`, or `unknown`), not a host-version string.
The indexed-array command continues to emit `indexed-array-sum-4096`.
Replay rejects duplicate object keys (including escaped aliases), unknown
fields, and forged analysis. JSON input/output is capped at 4 MiB, nesting at
128 containers, and replay at 65,536 values; the collector supports up to
10,000 pairs.

Run the native indexed-array command with:

```bash
swift run -c release array-microbenchmark --pairs 40 --revision local
```

It compares indexed traversal against native array iteration over 4,096 values.
The command writes one JSON report to stdout and diagnostics to stderr; measured
durations must be positive, but no performance threshold gates correctness.

## Build and tests

```bash
bash BUILD
```

On Windows, `BUILD_windows` delegates to `tools/run-windows.cmd` so the Swift
availability check guards the full test and installed-consumer sequence. When
Swift is present, test and consumer failures are returned to the build tool.
