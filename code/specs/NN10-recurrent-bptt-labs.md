# NN10: Recurrent Backpropagation Through Time Labs

## Status

Draft specification for deterministic, language-neutral scalar BPTT traces.

## Purpose

NN10 reverses NN09's three-step recurrent chain. It shows how a final-state
loss travels backward through time and how one shared parameter receives a
separate gradient contribution from each execution of the recurrent cell.

## Forward Contract

V1 reuses NN09's scalar ReLU recurrence:

```text
a[t] = W_x * x[t] + W_h * h[t - 1] + b
h[t] = ReLU(a[t])
```

with `x = [1, 2, 0]`, `h[-1] = 0`, `W_x = 2`, `W_h = 0.5`, and `b = -1`.
The states are `[1, 3.5, 0.75]`.

V1 measures only the final state against target `0`:

```text
loss = 0.5 * (h[2] - target)^2 = 0.28125
```

## Backward Contract

Walk from time `2` to time `0`. At each step:

```text
dL/dh[t] = direct_loss_gradient[t] + future_state_gradient[t]
dL/da[t] = dL/dh[t] * ReLU'(a[t])

local dW_x[t] = dL/da[t] * x[t]
local dW_h[t] = dL/da[t] * h[t - 1]
local db[t]    = dL/da[t]

future_state_gradient[t - 1] = dL/da[t] * W_h
```

All V1 preactivations are positive, so every ReLU derivative is one. The state
gradients are `0.75`, `0.375`, and `0.1875` in backward execution order.

## Shared-Parameter Reduction

The time-local contributions must be added because all three executions refer
to one parameter set:

```text
dW_x = 0 + 0.75 + 0.1875 = 0.9375
dW_h = 2.625 + 0.375 + 0 = 3
db   = 0.75 + 0.375 + 0.1875 = 1.3125
```

Overwriting the shared gradient at each time step is non-conformant.

## Independent Gradient Check and Update

V1 checks all three analytical totals with central finite differences using
epsilon `1e-6`. The maximum absolute difference is below `1e-9`.

One gradient-descent update with learning rate `0.1` produces:

```text
W_x = 1.90625
W_h = 0.2
b   = -1.13125
```

The updated states are `[0.775, 2.83625, 0]`, so the final loss is `0`.

## Fixture Layout

```text
code/specs/fixtures/recurrent-bptt-v1/
  schema.json
  labs/*.json
```

Validate it with:

```text
python code/scripts/validate_recurrent_bptt_labs.py
```

## Conformance Levels

- **Forward conformance:** reproduce all preactivations, states, prediction,
  and loss.
- **Backward conformance:** reproduce every backward-time chain-rule value.
- **Accumulation conformance:** reproduce every time-local parameter
  contribution and its shared total.
- **Audit conformance:** match central finite differences within tolerance.
- **Update conformance:** reproduce the proposed parameters, states, and loss.
- **Inspectable conformance:** let a learner select one backward step and trace
  its incoming gradient, local contributions, and gradient to the prior state.

## Native and Rust Direction

A Rust reference can statically unroll V1 or run a reverse loop over saved
forward values. A future sequence core needs explicit saved-state or
recomputation policy, caller-owned gradient buffers, accumulation semantics,
and a way to distinguish zeroing a gradient buffer from adding into it.

The stable C ABI should take forward inputs/states, upstream gradients,
parameter buffers, and output gradient buffers with documented shapes and
ownership. Fused BPTT kernels remain conformant only when a debug path can
recover the time-local contributions and reduction pinned by NN10.
