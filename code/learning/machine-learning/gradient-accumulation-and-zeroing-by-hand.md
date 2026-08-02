# Gradient Accumulation and Zeroing, by Hand

In the previous lesson, a dynamic computation graph recorded what ran and
backward walked that graph in reverse. That explains where a gradient comes
from. It does not yet explain where the gradient goes.

Most training libraries give every trainable parameter a second piece of
mutable state: a **gradient buffer**. If the parameter is `w`, frameworks often
show that buffer as `w.grad`.

The three rules to remember are:

```text
backward adds
optimizer step reads
zero_grad clears
```

The optimizer step usually does **not** clear the buffer for you.

## Why a Buffer Exists

A parameter can influence a loss through several routes. It can also be reused
for several micro-batches before the machine has enough memory to process the
whole batch at once. In both situations, the complete gradient is a sum of
contributions.

Addition is therefore the natural default:

```text
w.grad_after = w.grad_before + new_contribution
```

That default is powerful. It is also stateful. If you forget what is already in
the buffer, a perfectly correct backward pass can still produce a wrong
training update.

## Our Tiny Model

Use one parameter and no bias:

```text
prediction = w * x
```

For one sample with target `y`, use half-squared error:

```text
loss = 0.5 * (prediction - y)^2
```

The factor `0.5` cancels the `2` from differentiating a square. The gradient is:

```text
dL/dw = (prediction - y) * x
```

Start with:

```text
w = 1
w.grad = 0
```

Use two samples:

| Sample | `x` | `y` |
| --- | ---: | ---: |
| A | 2 | 1 |
| B | -1 | 1 |

## First Backward Call

For sample A:

```text
prediction = 1 * 2 = 2
residual = 2 - 1 = 1
loss = 0.5 * 1^2 = 0.5
dL/dw = 1 * 2 = 2
```

Backward does not replace the buffer. It adds:

```text
w.grad = 0 + 2 = 2
```

The parameter is still `1`. Backward calculated a gradient; it did not update
the parameter.

## Second Backward Call Without Zeroing

For sample B:

```text
prediction = 1 * -1 = -1
residual = -1 - 1 = -2
loss = 0.5 * (-2)^2 = 2
dL/dw = -2 * -1 = 2
```

The buffer already contains `2`, so the new contribution is added:

```text
w.grad = 2 + 2 = 4
```

Nothing mysterious happened. Two backward calls produced the timeline:

```text
0 -> 2 -> 4
```

## What Zeroing Changes

Repeat the same calls, but clear between them:

```text
backward(A): 0 + 2 = 2
zero_grad(): 2 -> 0
backward(B): 0 + 2 = 2
```

Now the final buffer is `2`, not `4`. Zeroing does not change `w`; it only says
that future backward calls belong to a new accumulation window.

## Sum Versus Mean

Suppose A and B are two micro-batches that together represent one batch. Their
summed gradient is `4`.

If the intended batch loss is a **sum**, apply `4` directly. With learning rate
`0.1`:

```text
w_new = 1 - 0.1 * 4 = 0.6
```

If the intended batch loss is a **mean**, divide by the number of micro-batches:

```text
mean_gradient = 4 / 2 = 2
w_new = 1 - 0.1 * 2 = 0.8
```

Both calculations are internally consistent. The mistake is accumulating a
sum while assuming the learning rate was tuned for a mean, or dividing twice
because each micro-batch loss was already scaled.

The fixture makes the divisor explicit. There is no hidden framework default.

## The Optimizer Does Not Empty the Buffer

After the mean update:

```text
w = 0.8
w.grad = 4
```

The optimizer read `4`, divided it by `2`, and moved the parameter. The buffer
is still `4` until an explicit clear:

```text
zero_grad()
w.grad = 0
```

That separation is useful. It lets tools inspect gradients after a step, lets
several optimizers read the same gradients, and lets a program deliberately
accumulate across calls. It also means the training loop owns the schedule.

## The Stale-Gradient Failure

Imagine that we forget the clear after moving `w` to `0.8`. The next sample is:

```text
x = 1
y = 0
```

Its correct new gradient is:

```text
prediction = 0.8 * 1 = 0.8
residual = 0.8 - 0 = 0.8
dL/dw = 0.8 * 1 = 0.8
```

But the buffer still contains `4`:

```text
w.grad = 4 + 0.8 = 4.8
```

The next step uses `4.8`:

```text
w_wrong = 0.8 - 0.1 * 4.8 = 0.32
```

With a clean buffer, it would have used only `0.8`:

```text
w_clean = 0.8 - 0.1 * 0.8 = 0.72
```

Backward was correct in both programs. The difference came entirely from
buffer history.

## A Reliable Training-Loop Order

For one optimizer update per batch, a clear loop is:

```text
zero gradients
run forward
run backward
optimizer step
```

For `k` micro-batches:

```text
zero gradients
repeat k times:
    run forward
    run backward and add into the same buffers
scale once if a mean is intended
optimizer step
```

Some frameworks place `zero_grad()` after the optimizer step instead of before
the next forward pass. That is equivalent only if every path between those
points is controlled. Placing the clear at the start of the accumulation window
makes the invariant easier to see: the buffer begins at zero.

## Check One Gradient Independently

The analytical rule for sample A said `dL/dw = 2`. Distrust it once with a
central finite difference and `epsilon = 0.00001`:

```text
numerical_gradient =
    (loss(w + epsilon) - loss(w - epsilon)) / (2 * epsilon)
```

Fresh forward passes give approximately `2`. The NN28 validator performs this
check at every backward event, including after an optimizer has changed `w`.
It also rejects a fixture whose claimed trace is self-consistent but whose
numerical error exceeds `1e-8`.

Finite differences verify each local gradient. They do not decide whether your
buffer should contain a sum, a mean, or zero. That is a scheduling decision.

## Tensor and Multi-Language Meaning

Real parameters are tensors, so `w.grad` becomes a tensor with the same shape as
`w`. The scalar rule applies element by element:

```text
gradient_buffer[i] += new_gradient[i]
```

Every language consumer should agree on the event semantics before optimizing
the kernels. A browser, Python notebook, Go package, Swift app, or Rust CLI can
all replay the same JSON schedule and compare every state transition.

For performance, a Rust core can own bounded numeric kernels for buffer
addition, scaling, and SGD. The host language should still own parameter object
identity and the order of `backward`, `step`, and `zero` calls. A C ABI must use
explicit slice lengths or opaque handles; it must not retain raw pointers to
garbage-collected host objects. The core must never silently choose mean versus
sum or clear a buffer as a side effect.

## Use the Visualizer

Open the **Grad Buffers** workbench in the TypeScript
[ML Learning Visualizer](../../programs/typescript/ml-learning-visualizer/README.md).

Try these comparisons:

1. Open both backward events in **Two backward calls** and watch `w.grad` move
   from `0` to `2` to `4`.
2. Switch to **Zero between calls** and inspect the reset as its own state
   transition.
3. Open the optimizer event in **Mean, step, zero** and confirm that the buffer
   stays `4` while `w` becomes `0.8`.
4. Switch to **Forgotten zero** and compare the new local gradient `0.8` with
   the contaminated buffer `4.8`.

The language-neutral arithmetic lives in the
[NN28 fixture](../../specs/fixtures/gradient-accumulation-v1/README.md), and the
normative contract is [NN28](../../specs/NN28-gradient-accumulation-zeroing-labs.md).

## Exercises

1. Change sample B's target so its gradient is `-2`. What happens to the buffer
   after A and B?
2. Keep the summed gradient `4`, but use learning rate `0.05`. Why does that
   match the mean-gradient update at learning rate `0.1`?
3. Insert `zero_grad()` immediately after the first optimizer event in the stale
   schedule. Compute the corrected final parameter.
4. Accumulate four identical micro-batches. Write the sum and mean gradients,
   then state exactly where you would divide.

When you can predict every buffer state before clicking it, gradient
accumulation is no longer framework magic. It is ordinary addition plus an
explicit lifetime.
