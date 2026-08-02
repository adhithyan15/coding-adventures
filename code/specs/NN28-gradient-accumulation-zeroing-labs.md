# NN28: Gradient Accumulation and Zeroing Labs

## Status

Version 1 is implemented by the language-neutral fixture under
[`fixtures/gradient-accumulation-v1`](./fixtures/gradient-accumulation-v1/README.md).

## Purpose

NN27 explains how one backward pass computes gradients. NN28 explains the
stateful part that frameworks often hide: a parameter owns a persistent
gradient buffer, each backward call adds to it, an optimizer reads it without
clearing it, and an explicit zero operation starts a fresh accumulation window.

The corpus stays deliberately scalar. Every value fits on paper, while the
event schedule is the same one used by tensor frameworks and distributed
micro-batch training.

## Canonical Arithmetic

For parameter `w`, sample input `x`, target `y`, and half-squared error:

```text
prediction = w * x
residual = prediction - y
loss = 0.5 * residual^2
local_gradient = residual * x
gradient_buffer_after = gradient_buffer_before + local_gradient
```

An SGD optimizer event with learning rate `eta` and explicit divisor `d` is:

```text
applied_gradient = gradient_buffer / d
w_after = w_before - eta * applied_gradient
gradient_buffer_after = gradient_buffer_before
```

Only `zero_grad` changes the gradient buffer to zero.

## Required Cases

The V1 roster and order are fixed:

1. `accumulate_two_calls` — two local gradients of `2` produce buffer states
   `0 -> 2 -> 4`.
2. `zero_between_calls` — an explicit reset produces `0 -> 2 -> 0 -> 2`.
3. `mean_then_zero` — two micro-batches use divisor `2`, update `w` from `1`
   to `0.8`, then clear the still-populated buffer.
4. `stale_next_batch` — omitting the reset adds a new `0.8` gradient to stale
   `4`, so the next optimizer incorrectly applies `4.8` and moves `w` to
   `0.32`.

Every backward event is independently checked with central finite differences
at epsilon `1e-5`. The canonical absolute tolerance is `1e-8`; fixtures cannot
loosen it.

## Validation and Safety

- Reject duplicate JSON keys and non-finite JSON constants.
- Reject unknown fields, duplicate sample identifiers, missing sample
  references, unsupported events, and non-positive or oversized divisors.
- Cap samples at four and events at twelve.
- Bound teaching inputs before arithmetic and reject non-finite or oversized
  derived predictions, losses, gradients, buffers, and updates.
- Compare the complete derived trace with the checked-in oracle and separately
  require the numerical-gradient error to remain below the canonical tolerance.

## Cross-Language and Rust-Core Boundary

Language bindings should own parameter identity, event scheduling, and the
choice of accumulation window. A Rust core may accelerate bounded tensor
kernels for buffer addition, scaling, and SGD, but must not silently choose a
divisor or clear a host-owned gradient buffer. A future stable C ABI should use
opaque parameter/buffer handles or caller-provided slices with explicit lengths;
it must never retain raw host-language object pointers.

Scalar consumers should reproduce this fixture first. Tensor consumers then
apply the same event semantics elementwise, using deterministic reduction order
where byte-for-byte parity is required.

## Versioning

V1 is append-only. Changing event semantics, the canonical case roster,
tolerance, or optimizer-zeroing rule requires a new fixture directory and
schema identifier.
