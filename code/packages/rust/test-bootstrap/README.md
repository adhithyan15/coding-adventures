# test-bootstrap

The bottom rung of the repository's testing ladder: a tiny, dependency-free
test harness for Rust, specified by
[TF01](../../../specs/TF01-bootstrap-harness.md).

## Where it fits

The verified-shipping program ([VER00](../../../specs/VER00-verified-shipping-program.md))
checks every testing tool with the layer below it. Generated tests run on a
harness, the property engine is pinned by its corpus, and so on. This crate is
the layer with nothing below it, so it is kept small enough to verify by
reading, and pinned by data:

```text
  L3  property, fuzz, microbench, proofs ...
  L2  runtime support (shrinking, mutators)
  L1  Warrant: specs compiled into plain tests ─┐
  L0  test-bootstrap  ◄─────────────────────────┘  this crate
        ▲
        └── checked by code/specs/fixtures/test-bootstrap-v1 (85 cases),
            whose answers an independent Python reference re-derives
```

It is the first step of the [VER26](../../../specs/VER26-ct-compare-pilot.md)
`ct-compare` pilot.

## What it does

| Module    | Contents |
|-----------|----------|
| `tfv`     | A strict parser for TFV v1, the line format corpora travel in, and TF01 deep equality |
| `verdict` | The pure core: `deep_equal` and `error_is`, returning `Pass` or `Fail(failure_id, message)` |
| `prng`    | SplitMix64, FNV-1a 64, an unbiased bounded draw, the run shuffle, per-test sub-seeds |
| `run`     | `Suite`, seeded shuffled runs, panic isolation, TAP v14 output, `--seed`, `--sabotage` |
| `pbt`     | Level 2 property-based testing engine (VER05): generators, integrated shrinking, closed PBT vocabulary |
| `fuzz`    | Level 2 coverage-guided fuzz engine (VER06): mutators, 65,536-bit bitmap, power schedule, crash deduplication |
| `bench`   | Level 5 microbenchmark harness (VER07): warmup/calibration, robust statistics, deterministic cost counters, budgets |

## Usage

A test binary is a plain `main` with Cargo's libtest turned off:

```toml
[[test]]
name = "conformance"
path = "tests/conformance.rs"
harness = false
```

```rust
use coding_adventures_test_bootstrap::{run, Suite, Value};

fn main() {
    let mut suite = Suite::new();
    suite.add("math/two-plus-two", |ctx| {
        ctx.deep_equal(&Value::from_u64(2 + 2), &Value::from_u64(4))
    });
    std::process::exit(run::main(&suite));
}
```

```text
$ cargo test --test conformance -- --seed 7
TAP version 14
# tf-lane: rust
# tf-seed: 7
1..1
ok 1 - math/two-plus-two
```

- Tests run in an order shuffled by `--seed` (default 0). The same seed always
  gives the same order, in every language that implements TF01.
- `ctx.seed()` is the test's own seed, which depends only on the run seed and
  the test's id.
- A panic fails that test and the run continues. Its message is not copied
  into TAP, because it may quote inputs.
- Failure IDs are a closed vocabulary of fifteen values (TF01's six, VER05's
  four, VER06's five). The runner checks
  `verdict::is_registered_failure_id` at serialization, including directly
  constructed `Verdict::Fail` values. An unknown ID emits only
  `Bail out! invalid failure id`, stops remaining tests and exits nonzero;
  neither the invalid ID nor its message is printed. A valid reported prefix
  keeps its counts, and the malformed verdict is not counted as a result.
- Exit status: 0 when every test passed, 1 otherwise, 2 for a bad command line.

## Sabotage mode

`--sabotage always-pass` makes `ctx.deep_equal` and `ctx.error_is` always
return `Pass`. The self-corpus contains cases that must *fail*, so a sabotaged
run must print `not ok` lines. The BUILD checks that with `grep`, from outside
the harness. A harness that had lost the ability to report failure would pass
every normal run, but it fails this check.

## Its own tests

Everything here runs on libtest (or a spawned process), never on the harness
being tested, so the harness is not the thing reporting on itself.

- `src/**`: unit tests for the parts the corpus cannot reach.
- `tests/self_corpus.rs`: every record of `cases.tfv` run on this harness.
  The corpus is embedded with `include_str!`, so the crate reads no files and
  `required_capabilities.json` is empty. `pbt_corpus.rs` and `fuzz_corpus.rs`
  do the same for `pbt-v1` and `fuzz-v1`.
- `tests/l0_contracts.rs` (+ `exit_fixture.rs`): "fails when it should". Every
  verdict path yields a registered non-pass verdict; panics of every shape are
  failures; exact TAP bytes; real-process exit statuses 0, 1 and 2;
  `/dev/full` and a closed pipe are not passes; the sabotaged self-corpus fails
  *exactly* the expected-fail cases.
  `src/bin/exit_fixture.rs` and `src/bin/self_corpus.rs` expose the same
  harness-free test programs as Cargo binary targets so integration tests use
  Cargo's `CARGO_BIN_EXE_*` paths instead of assuming test executables share a
  directory.
- `tests/tfv_exhaustive.rs`: 364,413 bounded-exhaustive TFV documents and a
  220,900-pair deep-equality universe, with hand-written oracles and the
  canonical-spelling property. With `TFV_DIFFERENTIAL_OUT=<path>` it also dumps
  every verdict, which `code/scripts/tfv_differential.py` replays through the
  independent Python reference (`code/scripts/tfv.py`).
- `tests/prng_contracts.rs`: published SplitMix64 and FNV-1a vectors, an
  independent `u128` reference, `bounded` against the spec sampler, sub-seed
  independence.
- `tests/pbt_properties.rs`, `tests/fuzz_properties.rs`,
  `tests/bench_statistics.rs`: verdict honesty, generator distributions,
  exhaustive shrinking, campaign determinism and dedup, coverage feedback,
  mutator algebra, hand-computed statistics and budgets.
- `code/scripts/bootstrap_harness_conformance.py` re-derives every expected
  answer in standard-library Python and checks that `cases.tfv` is exactly
  what `cases.json` generates.

## Known limits

These are specified behaviour or open items, written down so nobody has to
rediscover them (TF01 §8, VER05 §7, VER06 §9, VER07 §8):

- **No assertion counting.** A test body that returns `Pass` without checking
  anything passes. `Verdict` is `#[must_use]`, which catches the common slip of
  computing a check and discarding it.
- **A test that calls `process::exit(0)` leaves a truncated report with exit
  status 0.** The plan line `1..N` is printed first so a supervisor can compare
  it with the result lines; do not trust the exit status alone for suites whose
  tests you do not control. There are no timeouts at this layer.
- **Independent `Bytes` draws almost never collide** (never at fixed length 16).
  An "equal inputs" claim needs a correlated generator; see VER05 §7.2.
- **`AllocCounter` measures the real heap, but only in a binary that installs
  the allocator.** Add `coding_adventures_test_bootstrap::install_counting_allocator!();`
  to the bench or test binary. Without it the counter reports `NotLive` and the
  `max_alloc_bytes` budget is listed as unchecked, never passed. Tallies are
  per thread: allocations on threads the closure spawns are not counted.
- **Standard output is checked, not trusted.** A closed (Linux, macOS) or read-only
  fd 1 makes the runner exit 1 instead of silently printing nothing (Rust std
  hides `EBADF`). Still undetectable: a stdout that discards bytes (`/dev/null`),
  a closed fd 1 on Unix systems other than Linux, Android and macOS, a read-only Windows handle, and an fd 1
  closed after start. Table in TF01 §8.3 item 9; supervisors must still require
  the `1..N` plan and `N` result lines.
- **`FuzzReport::is_clean()`**, not `crashes.is_empty()`, is the "found nothing"
  test: timeouts and out-of-memory outcomes are counted separately.
- **PBT CI redaction** uses a 64-bit FNV digest over a low-entropy string, not
  the SHA-256 of VER05 §5.2, and is brute-forceable for small input domains.

To change the corpus, edit `cases.json`, then regenerate:

```bash
python3 code/scripts/bootstrap_harness_conformance.py --write
```
