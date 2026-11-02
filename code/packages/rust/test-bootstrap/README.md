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
        └── checked by code/specs/fixtures/test-bootstrap-v1 (72 cases),
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
- Exit status: 0 when every test passed, 1 otherwise, 2 for a bad command line.

## Sabotage mode

`--sabotage always-pass` makes `ctx.deep_equal` and `ctx.error_is` always
return `Pass`. The self-corpus contains cases that must *fail*, so a sabotaged
run must print `not ok` lines. The BUILD checks that with `grep`, from outside
the harness. A harness that had lost the ability to report failure would pass
every normal run, but it fails this check.

## Its own tests

- `src/**`: unit tests on libtest, for the parts the corpus cannot reach
  (TAP layout, argument parsing, panics, bad ids).
- `tests/self_corpus.rs`: every record of `cases.tfv` run on this harness.
  The corpus is embedded with `include_str!`, so the crate reads no files and
  `required_capabilities.json` is empty.
- `code/scripts/bootstrap_harness_conformance.py` re-derives every expected
  answer in standard-library Python and checks that `cases.tfv` is exactly
  what `cases.json` generates.

To change the corpus, edit `cases.json`, then regenerate:

```bash
python3 code/scripts/bootstrap_harness_conformance.py --write
```
