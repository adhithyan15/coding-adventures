# Backpropagation Through Time, by Hand

The recurrent cell in the previous lesson reused one set of parameters at
three time steps. Training reverses that unrolled chain. A gradient can travel
from a later state into an earlier state, and every execution contributes to
the gradients of the shared parameters.

This reverse walk is **backpropagation through time**, or BPTT. The complete
language-neutral oracle is
[`00-final-state-loss.json`](../../specs/fixtures/recurrent-bptt-v1/labs/00-final-state-loss.json).
The formulas and conformance rules are in
[`NN10-recurrent-bptt-labs.md`](../../specs/NN10-recurrent-bptt-labs.md).

## Reuse the three-step forward pass

Use the same scalar ReLU cell and numbers:

```text
a[t] = W_x * x[t] + W_h * h[t - 1] + b
h[t] = ReLU(a[t])

x                  = [1, 2, 0]
h[-1]              = 0
W_x, W_h, b        = 2, 0.5, -1
a[0], a[1], a[2]  = 1, 3.5, 0.75
h[0], h[1], h[2]  = 1, 3.5, 0.75
```

Only the final state is compared with target `y = 0`. Half-squared error keeps
the derivative small enough to see:

```text
L = 0.5 * (h[2] - y)^2
  = 0.5 * (0.75 - 0)^2
  = 0.28125

dL/dh[2] = h[2] - y = 0.75
```

Save each input, previous state, preactivation, and state during the forward
pass. The backward pass needs those values.

## Two ways a state can receive gradient

At a time step, the state gradient has two possible sources:

```text
dL/dh[t] = direct gradient from a loss at t
         + gradient carried backward from t + 1
```

This example has a loss only at the final step, so the direct gradient is
`0.75` at `t = 2` and zero earlier. Earlier steps still receive gradient
through the recurrent links.

Every preactivation is positive, so every ReLU derivative is `1`:

```text
dL/da[t] = dL/dh[t] * ReLU'(a[t])
```

## Reverse step 2

Start at the loss and walk from right to left:

```text
direct dL/dh[2] = 0.75
future gradient = 0
dL/dh[2]        = 0.75 + 0 = 0.75
dL/da[2]        = 0.75 * 1 = 0.75
```

This execution used `x[2] = 0` and `h[1] = 3.5`, so its local parameter
contributions are:

```text
dL/dW_x at t=2 = 0.75 * x[2] = 0
dL/dW_h at t=2 = 0.75 * h[1] = 2.625
dL/db   at t=2 = 0.75
```

The gradient crossing the recurrent edge into the previous state is:

```text
dL/dh[1] from the future = 0.75 * W_h = 0.375
```

## Reverse step 1

There is no direct loss at step 1, but `0.375` arrived from step 2:

```text
direct dL/dh[1] = 0
future gradient = 0.375
dL/dh[1]        = 0 + 0.375 = 0.375
dL/da[1]        = 0.375 * 1 = 0.375

dL/dW_x at t=1 = 0.375 * x[1] = 0.75
dL/dW_h at t=1 = 0.375 * h[0] = 0.375
dL/db   at t=1 = 0.375

dL/dh[0] from the future = 0.375 * 0.5 = 0.1875
```

## Reverse step 0

The same pattern reaches the first execution:

```text
direct dL/dh[0] = 0
future gradient = 0.1875
dL/dh[0]        = 0 + 0.1875 = 0.1875
dL/da[0]        = 0.1875 * 1 = 0.1875

dL/dW_x at t=0 = 0.1875 * x[0]  = 0.1875
dL/dW_h at t=0 = 0.1875 * h[-1] = 0
dL/db   at t=0 = 0.1875

dL/dh[-1] = 0.1875 * 0.5 = 0.09375
```

`dL/dh[-1]` matters when an initial state is trainable or came from an earlier
sequence chunk. Even when the initial state is fixed at zero, its gradient
should be an explicit result rather than silently discarded inside the loop.

## Add contributions into shared gradients

There is one `W_x`, one `W_h`, and one `b`—not a separate parameter set at each
time. Their gradients are sums:

| parameter | step 2 | step 1 | step 0 | total |
| --- | ---: | ---: | ---: | ---: |
| `W_x` | 0 | 0.75 | 0.1875 | **0.9375** |
| `W_h` | 2.625 | 0.375 | 0 | **3** |
| `b` | 0.75 | 0.375 | 0.1875 | **1.3125** |

This is gradient accumulation: use `+=` for each shared parameter contribution.
Replacing a gradient at every step would keep only one execution's evidence.

## Audit the backward pass independently

Central finite differences perturb one parameter without using the BPTT code:

```text
numerical gradient = (L(parameter + 0.000001)
                    - L(parameter - 0.000001)) / 0.000002
```

| parameter | BPTT | finite difference | absolute error |
| --- | ---: | ---: | ---: |
| `W_x` | 0.9375 | 0.937500000131 | 0.000000000131 |
| `W_h` | 3 | 3.000000000086 | 0.000000000086 |
| `b` | 1.3125 | 1.312500000045 | 0.000000000045 |

The largest disagreement is about `1.31e-10`. That is floating-point rounding,
not evidence that the two methods disagree.

## Take one update

With learning rate `0.1`, subtract each total gradient:

```text
W_x = 2   - 0.1 * 0.9375 = 1.90625
W_h = 0.5 - 0.1 * 3      = 0.2
b   = -1  - 0.1 * 1.3125 = -1.13125
```

The updated preactivations are `[0.775, 2.83625, -0.564]`, so the states are
`[0.775, 2.83625, 0]`. The final prediction now equals the target, and this
single-example loss falls from `0.28125` to `0`.

## Common implementation bugs

- Walking forward during the backward pass uses gradients before they exist.
- Assigning a shared gradient instead of adding contributions loses time steps.
- Skipping `ReLU'(a[t])` lets gradient cross an inactive cell incorrectly.
- Recomputing with updated parameters mixes two different model states.
- Hiding the initial-state gradient breaks chunked-sequence training.
- Forgetting to zero parameter-gradient buffers between optimizer steps makes
  separate batches accumulate accidentally.

Longer sequences add practical choices such as truncated BPTT, saved-state
buffers, and recomputation. Those choices change memory and compute costs, not
the local arithmetic shown here.

## Try the backward microscope

The
[`ml-learning-visualizer`](../../programs/typescript/ml-learning-visualizer/README.md)
places the forward states above a right-to-left backward lane. Select a reverse
step to see its direct and future gradients combine, then inspect one reduction
table for all shared-parameter contributions, the finite-difference audit, and
the proposed update.

## Cross-language checkpoint

An NN10 implementation is ready to scale when it reproduces the fixture's
reverse step order, all local contributions, shared-gradient totals,
initial-state gradient, finite-difference check, and updated forward pass.

A Rust core can implement V1 as a reverse loop over saved scalar states or as
a static unrolled graph. A future C ABI should keep forward-state storage,
gradient buffers, accumulation-versus-zeroing policy, and initial-state
gradient ownership explicit. Other languages can either implement the same
fixture directly or consume that core without making BPTT a Rust-only concept.
