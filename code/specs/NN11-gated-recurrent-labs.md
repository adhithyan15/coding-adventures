# NN11: GRU and LSTM Gate Comparison Labs

## Status

Draft specification for deterministic, language-neutral scalar gated-cell
traces.

## Purpose

NN11 compares the two most common gated recurrent cells with one shared scalar
example. The fixture exposes every gate activation, candidate calculation,
memory contribution, and output so a learner can see what GRUs combine and
what LSTMs keep separate.

## Shared Input Contract

V1 receives:

```text
x = 1
h_previous = 0.8
c_previous = 0.8
```

`c_previous` is used only by the LSTM. Gate preactivations are included so
consumers must reproduce the sigmoid or tanh activation rather than treating
gate values as unexplained constants.

## Scalar GRU Contract

V1 uses the update-gate convention in which `z` selects the new candidate:

```text
r = sigmoid(a_reset)
z = sigmoid(a_update)
n = tanh(input_product + recurrent_weight * (r * h_previous) + bias)
h = (1 - z) * h_previous + z * n
```

The canonical gate values are `r = 0.5`, `z = 0.25`, and `n = 0.6`. The
previous-state contribution is `0.6`, the candidate write is `0.15`, and the
next hidden state is `0.75`.

Some libraries define `z` as the amount of old state to retain. A conforming
NN11 consumer must use the explicit V1 equation above and document any adapter
needed for a library with the opposite naming convention.

## Scalar LSTM Contract

V1 uses:

```text
f = sigmoid(a_forget)
i = sigmoid(a_input)
o = sigmoid(a_output)
g = tanh(a_candidate)
c = f * c_previous + i * g
h = o * tanh(c)
```

The canonical values are `f = 0.5`, `i = 0.25`, `o = 0.75`, and `g = 0.6`.
The next cell state is `0.55`; `tanh(c)` is `0.5005202111902353`; and the next
hidden state is `0.3753901583926764`.

## Counterfactual Contract

The fixture changes one gate at a time without changing the other inputs:

- GRU update `0` preserves the old hidden state; update `1` selects the
  candidate.
- GRU reset `0` removes the old hidden state from candidate construction.
- LSTM forget `0` removes the old cell-state contribution.
- LSTM input `0` prevents the candidate write.
- LSTM output `0` hides the cell without erasing it; output `1` reveals the
  complete squashed cell.

These are controlled counterfactuals, not trained trajectories.

## Fixture Layout

The V1 corpus lives at:

```text
code/specs/fixtures/gated-recurrent-v1/
  schema.json
  labs/00-gru-lstm-gates.json
```

All numbers are JSON numbers. Consumers compare derived values with
`absolute_tolerance` and reject duplicate keys, non-finite values, unknown
fields, unsupported operations, and missing counterfactuals.

## Conformance Levels

1. **Gate:** reproduce sigmoid/tanh gate values from preactivations.
2. **Cell:** reproduce the complete canonical GRU and LSTM traces.
3. **Counterfactual:** reproduce every one-gate intervention.
4. **Trace:** expose the same named intermediates to a learner or debugger.

## Cross-Language and Rust-Core Direction

V1 needs only scalar arithmetic, so every language should implement it
directly first. A Rust sequence core can later batch these equations over
vectors while preserving a trace mode containing gate preactivations,
activations, state contributions, cell state, and hidden state.

A future C ABI should use explicit input/output buffers and caller-owned state.
GRU hidden state and the LSTM `(hidden, cell)` pair must have different typed
descriptors; packing the LSTM cell into invisible runtime state would break
chunked inference and cross-language reproducibility.
