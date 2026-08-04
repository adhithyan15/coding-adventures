# One fixture, three languages

Cross-language machine learning starts much smaller than a portable tensor
library. It starts with one question: can two programs read the same numbers
and independently reach the same answer?

Copying an expected answer into Go, Ruby, and Rust would prove almost nothing.
All three programs could repeat the same typo. NN34 instead gives each program
the original NN03 fixture. The fixture contains inputs, weights, a bias, an
activation, and an expected prediction. Every consumer must do the arithmetic
itself.

## The complete model fits on paper

The model has two inputs and one output neuron:

```text
x = [2, -1]
w = [0.5, -0.25]
b = 0.1
activation = identity
```

First multiply each input by its own weight:

```text
contribution 1 =  2 *  0.5  = 1.0
contribution 2 = -1 * -0.25 = 0.25
```

Then add the two contributions and the bias:

```text
preactivation = 1.0 + 0.25 + 0.1
              = 1.35
```

The identity activation returns its input unchanged:

```text
prediction = identity(1.35) = 1.35
```

This tiny trace is valuable because a learner can audit every number before
thinking about language runtimes, foreign-function interfaces, or accelerated
hardware.

## What changes between the lanes?

The mathematics does not change, but the mechanics do:

- **Go** represents the fixture with structs and uses the standard JSON
  decoder. It stands for a compiled language with garbage collection.
- **Ruby** walks hashes and arrays at runtime. It stands for a dynamic,
  interpreted language.
- **Rust** deserializes into closed structs and owns every value explicitly. It
  stands for a systems-native language.

Each implementation rejects unknown fields, duplicate fields where its JSON
boundary exposes them, surprising shapes, non-finite arithmetic, oversized
files, and unsupported models. “Thin” means the programs implement only this
one forward contract. It does not mean they accept ambiguous input.

## Why emit a receipt?

A sentence such as `it passed` hides too much. Each lane emits one JSON receipt
with the two contributions, bias, preactivation, prediction, maximum absolute
error, and pass result. The Python orchestrator parses that receipt and
recomputes its claims.

The evidence chain is:

1. load the same committed NN03 fixture;
2. recompute the forward pass natively;
3. emit one closed receipt without explanatory stdout;
4. compare the receipt with the fixture expectation and tolerance.

A zero process exit is necessary but not sufficient. Missing output, extra
output, a dishonest error value, a timeout, invalid UTF-8, or a mismatched
prediction fails the complete gate.

## Run all three consumers

From the repository root:

```bash
python code/scripts/validate_cross_language_fixture_consumers.py
```

The orchestrator uses fixed argument arrays and no shell. It resolves the
fixture once, passes that exact path as one argument, bounds each process, and
expects three independently earned receipts.

## Native today, Rust core later

All three NN34 lanes are marked `native`: each language executes its own two
multiplications and additions. That is the clearest baseline for learning and
correctness.

The next tranche can expose a stable Rust C ABI and add binding-backed lanes.
Those lanes must keep the same fixture and receipt contract. Calling Rust may
change performance, memory ownership, and deployment, but it must not change
what answer counts as correct.

## What comes next

This first contract supports only one identity-activated neuron. Complexity can
grow one visible step at a time:

1. add sigmoid, tanh, and ReLU forward fixtures;
2. add multiple output neurons and dense layers;
3. add loss and first-step gradient receipts;
4. compare native execution with Rust-core bindings;
5. benchmark only after every lane still passes the same fixture oracle.

That order keeps portability attached to understanding instead of turning it
into another layer of hidden magic.
