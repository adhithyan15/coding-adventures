# ca::microbenchmark

A reusable C++17 paired benchmark collector and JSON report analyzer, plus the
`array-microbenchmark` indexed-array example. It uses
`ca::rank_statistics` for exact median intervals and `ca::bounded_json` for
bounded report replay.

```cpp
#include <ca/microbenchmark.hpp>

ca::microbenchmark::Config config;
config.pairs = 40;
const ca::microbenchmark::Batch baseline = [](std::int64_t n) { return n * 10; };
const ca::microbenchmark::Batch candidate = [](std::int64_t n) { return n * 9; };
const auto report = ca::microbenchmark::collect(baseline, candidate, 10, config);
```

Each callback returns its complete batch checksum or throws. Collection verifies
one operation, calibrates a common batch size, and measures adjacent pairs in a
seed-shuffled balanced order. The default clock is `std::chrono::steady_clock`;
tests inject deterministic clocks. Checksums and elapsed ticks are signed
64-bit values, and the JSON parser accepts only exact signed-64-bit integer
tokens for integer fields.

The JSON replay interface uses `bounded-json` limits: at most 4 MiB, depth 128,
65,536 values, and 16 MiB of parser allocations. A valid 10,000-pair report
fits those limits; reports beyond them are rejected. Replay validates the
balanced schedule and recomputes analysis, but does not authenticate the
observations or descriptive metadata.
The optional workload metadata is a nonblank UTF-8 producer identifier, not an
allowlist or instruction to run that workload. The array executable continues
to emit `indexed-array-sum-4096`.

Install the dependency packages first, then this CMake package. Consumers use
`find_package(ca_microbenchmark CONFIG REQUIRED)` and link `ca::microbenchmark`.
The install config declares both transitive package dependencies. Source tests
use the local `testdata/microbenchmark-pairs-v1.json` copy and check it against
the canonical fixture in the repository.

Run `array-microbenchmark` for the shared 4,096-element indexed-versus-iterator
workload. It emits the standard workload, invocation, revision, compiler,
platform, and optional debug/release profile metadata. Timings are advisory and
do not establish physical cache behavior or a universal speed ranking.

The public `indexed_batch` and `iterator_batch` helpers accept caller-provided
arrays. Negative operation counts throw `std::invalid_argument`; a per-pass or
accumulated checksum that exceeds signed 64-bit range throws
`std::overflow_error`. Both traversal paths perform the same checked additions,
so the overflow checks are part of the measured callback cost.
