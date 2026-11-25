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

It compares indexed access with native slice iteration over 4,096 values and
emits JSON including invocation, the Rust compiler version embedded at build
time, debug/release profile, OS, and architecture. The installed executable
does not need `rustc` on `PATH` to report its build compiler.

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
