# Native Go dense training: follow every parameter

Go now has its own reusable dense framework above its matrix foundation.
There is no Python process or Rust binding inside the training loop. The model
owns each layer's weights and biases, computes full-network derivatives, and
commits one complete SGD update only after its candidate loss is finite.

## A paper-sized model

Use the shared [paper fixture](../../specs/fixtures/neural-framework-v1/dense.json).
For input `[1, 2]`, the first linear layer produces `[2.5, 0.5]`:
`1*0.5 + 2*1 = 2.5` and `1*(-0.5) + 2*0.5 = 0.5`.
The second layer predicts `2.5*1 + 0.5*(-1) + 0.25 = 2.25`.
Against target 1, squared error is `1.25^2 = 1.5625`.
Backward starts with `2*1.25 = 2.5`, which reaches both layers.
At learning rate 0.01, every parameter updates and prediction becomes 1.77;
loss falls to 0.5929. Tests assert each weight/bias gradient, not just loss.

The runnable inspection trace is:

```text
checkpoint(0) -> Gradients(input,target) -> pre-update trace
              -> TrainStep(rate)        -> candidate parameters
              -> finite candidate loss  -> checkpoint(1)
                 any failure            -> checkpoint(0) unchanged
```

Call `Gradients` to inspect `Predictions`, `Loss`, `WeightGradients`, and
`BiasGradients`. These are owned snapshots. Changing a trace or exported
checkpoint does not change the model. Multiple output coordinates are averaged
together, so adding an output does not accidentally multiply the learning rate.

## Native training and resume

From `code/packages/go/neural-framework`, run `go run ./cmd/xor` to train seeded
XOR for exactly 3000 updates. The JSON receipt records achieved loss and all
parameters. Passing its checkpoint alone to `go run ./cmd/xor --resume` on stdin
advances one update; the validator repeats through step 3002 and checks loss
continuity and XOR predictions. See the [API README](../../packages/go/neural-framework/README.md).

`New` validates/copies topology before matrix construction. Stable sigmoid
avoids exponential overflow; ReLU's derivative at zero is zero. Failed updates,
nonfinite values, malformed batches and exhausted safe-integer counters fail
without partial mutation. The same [49 checkpoint documents](../../specs/fixtures/neural-framework-v1/checkpoints.json)
exercise Go's public loader, including valid controls and malformed shapes,
metadata, boolean counts and unknown fields. Go's default JSON zero-value/null
behavior is explicitly guarded rather than silently admitting missing data.

This native library does not yet count as required four-language interchange,
nor imply CNN, recurrent, transformer or accelerated training. The following
conformance tranche will exchange actual Go checkpoints with all existing
native lanes. Track that separately in the [framework roadmap](./FRAMEWORK-ROADMAP.md).
