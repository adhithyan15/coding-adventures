# Backward and Optimizer Lowering, by Hand

Forward lowering turned a graph into an ordered program. Training needs two
more programs:

1. a **backward program** that turns a loss gradient into parameter and input
   gradients; and
2. an **optimizer program** that turns a parameter gradient into new parameter
   state.

Keeping those programs separate matters. Backward answers, "What gradient did
the model produce?" The optimizer answers, "What should we do with it?" An SGD
step, Adam step, gradient clip, skipped update, or accumulated micro-batch can
all consume the same backward result.

This lesson uses one trainable scalar:

```text
prediction = w * x
residual   = prediction - target
loss       = 0.5 * residual^2
```

There is nowhere for the arithmetic to hide, but the same saved-value,
reverse-order, reduction, and update rules appear in large networks.

## 1. One row on paper

Use:

```text
w = 0.5
x = 2
target = 0
learning rate = 0.1
```

The forward pass saves:

```text
prediction = 0.5 * 2 = 1
residual   = 1 - 0 = 1
loss       = 0.5 * 1^2 = 0.5
```

Backward starts with the statement "change in loss per change in loss is 1":

```text
d_loss = 1
```

The half-squared-error derivative is the residual:

```text
d_residual = residual * d_loss = 1 * 1 = 1
```

Subtraction passes that gradient to the prediction:

```text
d_prediction = d_residual = 1
```

The multiplication `w * x` has two local derivatives:

```text
d_w = x * d_prediction = 2 * 1 = 2
d_x = w * d_prediction = 0.5 * 1 = 0.5
```

SGD reads `d_w` and moves against it:

```text
applied_gradient = d_w / divisor = 2 / 1 = 2
parameter_delta  = -0.1 * 2 = -0.2
w_next           = 0.5 + (-0.2) = 0.3
```

The optimizer step does **not** clear the gradient buffer. After the step,
`grad_w` is still `2` until an explicit zero operation runs.

## 2. Why backward needs saved values

The local rule for `w * x` needs the values of both `w` and `x`. The loss rule
needs the residual. Recomputing those values is sometimes possible, but it is
not automatically free or safe. Randomness, mutation, control flow, and
expensive operations can make recomputation different or costly.

The tiny program therefore makes its saved-value contract explicit:

```text
saved: x, w, prediction, residual
```

A compiler may later choose to keep, move, compress, or recompute a value, but
that is a separate optimization. The mathematical backward program first says
which value it needs.

## 3. Lower the backward graph

The NN30 backward instruction stream is:

| ID | Operation | Reads | Writes |
|---|---|---|---|
| `b0` | `SEED_LOSS_GRAD` | nothing | `d_loss` |
| `b1` | `HALF_SQUARED_ERROR_GRAD` | `residual`, `d_loss` | `d_residual` |
| `b2` | `PROPAGATE_GRAD` | `d_residual` | `d_prediction` |
| `b3` | `PARAMETER_LOCAL_GRAD` | `x`, `d_prediction` | `local_d_w` |
| `b4` | `ACCUMULATE_GRAD` | `grad_w`, `local_d_w` | `grad_w` |
| `b5` | `INPUT_GRAD` | `w`, `d_prediction` | `d_x` |

`b3` computes one row's contribution. `b4` owns the stateful addition into the
parameter gradient buffer. Keeping those separate exposes why shared
parameters and micro-batches add contributions instead of overwriting them.

## 4. Lower the optimizer separately

The optimizer instruction stream is:

| ID | Operation | Reads | Writes |
|---|---|---|---|
| `o0` | `READ_GRAD_BUFFER` | `grad_w` | `total_d_w` |
| `o1` | `DIVIDE_GRAD` | `total_d_w`, scenario divisor | `applied_d_w` |
| `o2` | `SGD_UPDATE` | `w`, `applied_d_w`, learning rate | `w_next` |
| `o3` | `KEEP_GRAD_BUFFER` | `grad_w` | `grad_w_after_step` |

The divisor is runtime policy, not an inferred compiler constant. A two-row
mean uses `2`; a summed gradient or one-row update uses `1`. A portable runtime
must receive that choice explicitly.

## 5. The same program over two rows

Now use:

```text
w = 1
learning rate = 0.1

row 0: x =  2, target = 1
row 1: x = -1, target = 1
divisor = 2
```

Forward values:

| Row | Prediction | Residual | Loss |
|---:|---:|---:|---:|
| 0 | `2` | `1` | `0.5` |
| 1 | `-1` | `-2` | `2` |

Backward values:

| Row | `d_prediction` | Local `d_w = x * d_prediction` | `d_x = w * d_prediction` |
|---:|---:|---:|---:|
| 0 | `1` | `2` | `1` |
| 1 | `-2` | `2` | `-2` |

The stable row-order reduction is:

```text
grad_w = 0 + 2 + 2 = 4
```

The optimizer applies the mean:

```text
applied_gradient = 4 / 2 = 2
w_next = 1 - 0.1 * 2 = 0.8
```

The instruction identifiers did not change. Only the columns grew from one row
to two and the explicit divisor changed.

## 6. Matrix training lowering

The matrix plan keeps per-row values as columns:

```text
x column          -> [ 2, -1]
residual column   -> [ 1, -2]
d_prediction      -> [ 1, -2]
local d_w column  -> [ 2,  2]
d_x column        -> [ 1, -2]
```

Then it crosses from row-parallel work to shared state:

```text
REDUCE_SUM_GRAD         [2, 2] -> batch_d_w = 4
ACCUMULATE_GRAD_BUFFER  0 + 4  -> grad_w = 4
DIVIDE_GRAD             4 / 2  -> 2
SGD_UPDATE              1 - 0.1 * 2 -> 0.8
```

The reduction order is part of the contract. Floating-point addition is not
mathematically associative, so implementations that require reproducibility
must use the fixture's ascending row order rather than whatever order worker
threads happen to finish.

### Prove that the buffer really persists

A fresh buffer can hide an overwrite bug because both "replace with `2`" and
"add `2` to `0`" produce `2`. The third fixture scenario therefore enters
backward with `grad_w = 3`. Its new row contributes `2`:

```text
batch_d_w = 2
grad_w    = 3 + 2 = 5
w_next    = 0.5 - 0.1 * (5 / 1) = 0
```

The optimizer leaves the buffer at `5`. The finite-difference audit still
compares the current batch contribution `2` with the current batch loss slope
`2`; the carried `3` came from an earlier execution and is not part of that
loss function.

## 7. Check the gradient independently

For a batch loss `L(w)`, central finite differences estimate:

```text
dL/dw ~= (L(w + epsilon) - L(w - epsilon)) / (2 * epsilon)
```

With `epsilon = 0.00001`, the one-row estimate is approximately `2` and the
two-row summed-loss estimate is approximately `4`. The audit compares the new
batch contribution, not any gradient already in the persistent buffer. It does
not reuse the backward program, so agreement catches missing factors, wrong
signs, and accidental averaging.

Finite differences are an audit, not the runtime algorithm. They cost two
extra forward executions per checked parameter and become expensive in large
models.

## 8. Three kinds of state

Training becomes easier to reason about when state is named:

- **saved forward values** belong to one forward/backward execution;
- **gradient buffers** belong to parameters and may span several backward
  executions; and
- **optimizer state** belongs to the optimizer and may span many steps.

Plain SGD has no momentum tensor, but it still owns learning-rate and divisor
policy. Adam would add first- and second-moment buffers plus a step counter.
Those are optimizer inputs and outputs, not secret backward side effects.

## 9. Rust-core boundary

A future Rust bridge can translate NN30 matrix operations into MX01 tensor
operations:

- column multiplication becomes `Mul`;
- gradient addition becomes `Add`;
- stable batch reduction becomes `ReduceSum`; and
- scalar scaling becomes multiplication by explicit constants.

The host must still own semantic decisions:

- which tensor is parameter `w`;
- which saved value belongs to which execution;
- whether the reduction is a sum or mean;
- when an optimizer step runs;
- when a gradient buffer is cleared; and
- which optimizer state is retained.

An FFI should pass opaque handles or caller-provided typed buffers with explicit
lengths, shapes, dtypes, and ownership. Rust must not retain pointers to
garbage-collected host objects or silently clear host-owned gradient state.

## 10. A language-neutral implementation recipe

Every consumer can reproduce the lab without sharing TypeScript objects:

1. strictly parse and validate the NN30 JSON fixture;
2. execute the canonical production forward graph and save named values;
3. emit the six backward instructions in canonical order;
4. emit the four optimizer instructions separately;
5. lower row-parallel rules into matrix column operations;
6. reduce the current batch's contributions in ascending row order;
7. add that batch sum to the incoming parameter gradient buffer;
8. apply the explicit divisor and learning rate;
9. keep the gradient buffer populated after the optimizer step;
10. compare direct, scalar-IR, and matrix-plan results; and
11. audit the current batch contribution with central finite differences.

The fixture owns all values, identifiers, and expected traces. Ports should not
copy numbers from this prose.

## 11. What lowering does not decide yet

This tranche does not choose:

- CPU, GPU, WebGPU, or accelerator placement;
- mixed-precision accumulation;
- quantization;
- buffer reuse or residency;
- distributed reduction order; or
- momentum and adaptive optimizer layouts.

Those belong to later roadmap tranches because they add new observable choices.

## Exercises

1. Change the one-row target from `0` to `1`. Which backward values become zero?
2. Use divisor `1` for the two-row batch. What update does summed SGD apply?
3. Reverse the row records. Why should a deterministic runtime still reduce in
   canonical row order?
4. Insert gradient clipping between `o1` and `o2`. Which program owns it?
5. Sketch the additional optimizer state needed for momentum.

The useful mental model is: forward saves evidence, backward produces
gradients, and the optimizer consumes those gradients under explicit policy.
