# NN34: Cross-language fixture consumers

## Status

Implemented.

## Purpose

NN34 proves that the language-neutral corpus is consumable outside Python and
the browser. Native Go, Ruby, and Rust programs read the same NN03
weighted-neuron fixture, recompute the forward pass, and emit a shared receipt.

## Contract

`code/specs/fixtures/cross-language-consumers-v1/` contains:

- `catalog.json`: the source fixture, hand calculation, receipt fields, and
  exact native command vectors;
- `schema.json`: the closed Draft 2020-12 interchange shape;
- `README.md` and `CHANGELOG.md`: execution and evolution notes.

The source model remains
`code/specs/fixtures/neural-learning-v1/labs/00-weighted-neuron.json`.
Consumers may not translate it into language-specific constants.

## Native lanes

| Lane | Language family | Source | Execution |
| --- | --- | --- | --- |
| `go-native` | compiled, garbage-collected | `code/programs/go/neural-fixture-consumer/main.go` | native binary64 |
| `ruby-native` | dynamic, interpreted | `code/programs/ruby/neural-fixture-consumer/main.rb` | native binary64 |
| `rust-native` | systems-native | `code/programs/rust/neural-fixture-consumer/src/main.rs` | native binary64 |

Each lane supports exactly the forward-only two-input identity neuron. It
rejects unsupported identities, layers, shapes, training steps, non-finite
arithmetic, and oversized files before emitting evidence.

## Receipt

Successful stdout is exactly one JSON object with these ordered contract keys:

```text
schema_version, lane_id, fixture_id, row, contributions, bias,
preactivation, prediction, maximum_absolute_error, passes
```

The orchestrator treats the receipt as an untrusted claim. It validates the
closed key set, identity, contribution trace, prediction, recomputed maximum
error, and tolerance result.

## Execution boundary

Run all three lanes from the repository root:

```bash
python code/scripts/validate_cross_language_fixture_consumers.py
```

The orchestrator validates the complete catalog before launching fixed command
arrays without a shell. It bounds stdout and stderr while each process runs,
applies a 60-second timeout, requires strict UTF-8, rejects successful stderr,
and propagates non-zero exits.

## Interactive trace

The Language Consumers workbench reads the same NN34 catalog and NN03 fixture.
It lets the learner select Go, Ruby, or Rust while the two products, bias
addition, identity activation, and tolerance comparison stay fixed. The
browser labels these as registered native lanes; only the CLI and CI claim
that the external runtimes executed.

## Rust-core direction

NN34 establishes native baselines. NN35 may add a stable Rust C ABI and mark a
lane `rust-core-binding`, but binding-backed receipts must remain comparable to
the native receipts without changing fixture semantics, shapes, precision, or
tolerance.
