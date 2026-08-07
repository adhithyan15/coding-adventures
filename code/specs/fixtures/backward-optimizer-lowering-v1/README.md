# Backward and Optimizer Lowering Fixture V1

This directory is the deterministic, language-neutral NN30 corpus.

## Contents

- [`schema.json`](./schema.json) describes the closed JSON shape.
- [`labs/00-scalar-sgd-training.json`](./labs/00-scalar-sgd-training.json) pins
  one-row, two-row, and nonzero-buffer saved values, backward IR, optimizer IR,
  matrix training IR, gradients, and SGD updates.
- [`CHANGELOG.md`](./CHANGELOG.md) records versioned changes.

## Validation

From the repository root:

```text
python code/scripts/validate_backward_optimizer_lowering_labs.py
pytest code/scripts/tests/test_backward_optimizer_lowering_labs.py -q
```

The executable validator is stricter than schema-only validation. It derives
the canonical instruction streams, executes every path, checks finite
differences, enforces bounds and roster order, and compares the complete trace
with the checked-in oracle.

## Consumer Contract

Consumers must parse this JSON strictly, preserve canonical IDs and row order,
and reproduce the expected streams and traces. The current TypeScript app uses
the production forward graph compiler, then implements this fixture's explicit
training-lowering reference contract. Other languages should consume the JSON
rather than copying constants from prose.
