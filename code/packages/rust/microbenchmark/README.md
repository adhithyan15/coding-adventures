# Paired microbenchmarks

`coding_adventures_microbenchmark` implements the paired collection and
replayable analysis contract in VER07b, with the indexed-array experiment from
VER07d. It compares a baseline and candidate on adjacent batches of identical
logical work, verifies checksums outside the timer, calibrates a shared batch
size, and reports the raw observations alongside the exact rank interval and
verdict.

```rust
use coding_adventures_microbenchmark::{collect, Config, Error};

let mut baseline = |n| Ok::<i64, Error>(10 * n as i64);
let mut candidate = |n| Ok::<i64, Error>(10 * n as i64);
let mut tick = 0i64;
let mut now = || { tick += 1; tick }; // Tests inject a progressing fake clock.
let report = collect(
    Config { pairs: 1, target_ns: 1, max_batch: 1, ..Config::default() },
    10,
    &mut baseline,
    &mut candidate,
    &mut now,
)?;
assert_eq!(report.pairs.len(), 1);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Run the executable from the Rust workspace:

```text
cargo run --release -p coding_adventures_microbenchmark --bin array-microbenchmark -- --pairs 40 --revision <commit>
```

The optional `--rustc-version "<rustc --version output>"` argument adds a
caller-supplied compiler label to the report. If it is omitted, `runtime` is
`"unknown"`. The CLI validates that the label is a bounded, nonblank, single
line rustc version string without control characters or Unicode line separators,
but it does not query or authenticate the compiler.
The exact supplied value is retained in the invocation arguments. A CI job may
query its selected compiler separately and pass that output when it can prove
which compiler it used.

The public analyzer accepts at most 10,000 pairs. It checks confidence, factor,
and pair count before validating pair contents or allocating its ratio vector.

It compares indexed access with native slice iteration over 4,096 values and
emits JSON including invocation, the caller-supplied Rust compiler label (or
`"unknown"`), debug/release profile, OS, and architecture. Building a library
consumer does not run a package build script to discover compiler metadata.

Tests use the package-local copy of the shared VER07 fixture so they also run
from extracted crate sources. In the repository, the test compares that copy
byte-for-byte with `code/specs/fixtures/microbenchmark-pairs-v1.json`.
Measurements are advisory and apply only to that workload and environment; they
do not prove physical cache behavior or predict another processor. Never use a
timing threshold as a correctness test. Fake clocks cover deterministic
collection behavior.

Single-report replay validates any nonblank UTF-8 workload producer identifier;
the metadata does not select executable code. The core `Report` value remains
an analysis projection without producer metadata. The array CLI continues to
emit `indexed-array-sum-4096`.

The library depends on the small Rust `rank-statistics` package. It does not use
the existing test-bootstrap single-call benchmark API because that harness
measures one callable and bootstrap medians, while VER07b requires adjacent
paired variants and an exact order-statistic interval.
