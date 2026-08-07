# NN09: Recurrent Unroll Labs

## Status

Draft specification for deterministic, language-neutral scalar recurrent
forward traces.

## Purpose

NN09 introduces sequence networks with one hidden-state value and three time
steps. It separates three ideas that are easy to blur together:

- the input changes at every step;
- the previous hidden state becomes an input to the next step; and
- the same parameters are reused at every step.

The V1 fixture uses ReLU so every value can be calculated by hand without a
scientific calculator.

## V1 Recurrence

For time step `t`:

```text
input_product[t]     = input_weight * input[t]
recurrent_product[t] = recurrent_weight * state[t - 1]
preactivation[t]     = input_product[t] + recurrent_product[t] + bias
state[t]             = max(0, preactivation[t])
```

`state[-1]` is the explicit initial state. V1 uses:

```text
input               = [1, 2, 0]
state[-1]           = 0
input_weight        = 2
recurrent_weight    = 0.5
bias                = -1
```

The three resulting states are `[1, 3.5, 0.75]`. The final input is zero, but
the final state remains positive because the preceding `3.5` travels through
the recurrent connection.

## Unrolling and Sharing

An implementation may execute one cell three times in a loop or build a
three-cell acyclic graph. The trace must make both views equivalent:

```text
state[-1] -> cell 0 -> state[0] -> cell 1 -> state[1] -> cell 2 -> state[2]
                x[0] ^                x[1] ^                x[2] ^
```

The three drawn cells are not three independently parameterized layers. Their
`input_weight`, `recurrent_weight`, and `bias` values refer to one shared
parameter set.

## Memory Ablation

V1 also pins a counterfactual run in which the recurrent contribution is zero
at every step while every other value stays fixed. Its states are `[1, 3, 0]`.
The state differences `[0, 0.5, 0.75]` isolate what the recurrent path adds.
This ablation is a teaching comparison, not a second model family.

## Fixture Layout

The V1 corpus lives in:

```text
code/specs/fixtures/recurrent-unroll-v1/
  schema.json
  labs/*.json
```

Validate and execute it with:

```text
python code/scripts/validate_recurrent_unroll_labs.py
```

## Conformance Levels

- **Step conformance:** reproduce both products, preactivation, and activated
  state for every time step.
- **State-chain conformance:** feed each resulting state into the next step and
  reproduce the final state.
- **Sharing conformance:** use the same three parameter values at all steps.
- **Ablation conformance:** reproduce the no-recurrence states and differences.
- **Inspectable conformance:** let a learner select one step and distinguish
  its new input, carried state, shared parameters, and activation.

## Native and Rust Direction

The existing Rust `neural-network` and `neural-graph-vm` packages model and run
acyclic neural graphs. A V1 consumer can either execute this scalar recurrence
with a small Rust loop or statically unroll three cells into a DAG. Static
unrolling must preserve parameter identity even if three graph edges carry the
same numeric value.

A future Rust sequence core should accept caller-owned input and state buffers,
explicit sequence and state sizes, parameter buffers, activation choice, and an
optional trace output. Its C ABI must make initial-state ownership and final-
state writeback explicit. Batched or fused kernels remain conformant only when
a reference/debug path can recover the per-step values pinned by NN09.
