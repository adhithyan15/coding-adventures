# Training a Convolution Kernel by Hand

The previous lesson slid one fixed kernel across a signal. Training changes the
kernel so its feature map moves toward desired targets.

The surprising part is not the derivative itself. It is that one kernel weight
was reused at several positions, so its gradient must collect evidence from
every position where it appeared.

## Start with the NN05 Forward Pass

Reuse the same signal and asymmetric kernel:

```text
signal  = [2,  1, 3, 0, 4, 2]
kernel  = [1, -1, 2]
targets = [6, -2, 10, 0]
```

Valid stride-one cross-correlation produces:

```text
outputs = [7, -2, 11, 0]
errors  = [1,  0,  1, 0]
```

Only `y[0]` and `y[2]` are wrong. With mean squared error over four outputs:

```text
loss = (1^2 + 0^2 + 1^2 + 0^2) / 4 = 0.5
```

## Differentiate the Loss First

For each output:

```text
dLoss/dOutput[i] = 2 * error[i] / 4
```

| Output | Error | `dLoss/dOutput` |
| --- | ---: | ---: |
| `y[0]` | 1 | 0.5 |
| `y[1]` | 0 | 0 |
| `y[2]` | 1 | 0.5 |
| `y[3]` | 0 | 0 |

An output that already matches its target sends zero gradient backward.

## Follow One Output into the Kernel

The first output used window `[2, 1, 3]`:

```text
y[0] = 2*k[0] + 1*k[1] + 3*k[2]
```

Its output gradient is `0.5`, so its contributions are:

```text
toward k[0]: 0.5 * 2 = 1
toward k[1]: 0.5 * 1 = 0.5
toward k[2]: 0.5 * 3 = 1.5
```

The third output, `y[2]`, used window `[3, 0, 4]` and sends:

```text
[0.5 * 3, 0.5 * 0, 0.5 * 4] = [1.5, 0, 2]
```

## Add Contributions for Shared Weights

Every column belongs to one shared parameter. Add down each column:

| Kernel gradient | From `y[0]` | From `y[1]` | From `y[2]` | From `y[3]` | Sum |
| --- | ---: | ---: | ---: | ---: | ---: |
| `dL/dk[0]` | 1 | 0 | 1.5 | 0 | 2.5 |
| `dL/dk[1]` | 0.5 | 0 | 0 | 0 | 0.5 |
| `dL/dk[2]` | 1.5 | 0 | 2 | 0 | 3.5 |

Therefore:

```text
dLoss/dKernel = [2.5, 0.5, 3.5]
```

This reduction is the heart of convolutional backpropagation. The kernel has
only three parameters, but each parameter can influence several outputs.

## Check the Gradient Independently

Before trusting the backward pass, perturb one weight at a time. For `k[j]`
and `epsilon = 0.000001`:

```text
numeric[j] = (loss(k[j] + epsilon) - loss(k[j] - epsilon))
             / (2 * epsilon)
```

Binary64 arithmetic produces approximately:

```text
[2.4999999999, 0.5000000001, 3.5000000003]
```

That matches `[2.5, 0.5, 3.5]` within `1e-9`. The numerical calculation uses
only forward loss evaluations, so it can catch mistakes in the analytical
gradient.

## Take One Learning Step

Use learning rate `0.02` and update all weights simultaneously:

```text
k[0] =  1 - 0.02 * 2.5 =  0.95
k[1] = -1 - 0.02 * 0.5 = -1.01
k[2] =  2 - 0.02 * 3.5 =  1.93
```

The new kernel produces:

```text
next outputs = [6.68, -2.08, 10.57, -0.18]
next loss    = 0.206525
```

One step lowered loss from `0.5` to `0.206525`. This does not prove every
learning rate will work; it proves this gradient points downhill locally.

## Inspect It Interactively

Open the **Spatial** view in the
[ML Learning Visualizer](../../programs/typescript/ml-learning-visualizer/README.md).
Selecting a feature-map output now synchronizes two paths:

1. the forward window and multiply-accumulate that produced `y[i]`;
2. the backward contributions that `y[i]` sends to every shared kernel weight.

The contribution table adds across all positions, compares the analytical sum
with a finite difference, and previews the next kernel and loss. Apply one step
to watch the detector and its next gradient change together.

## Portable and Fast Implementations

NN06 stores this entire trace in
`code/specs/fixtures/convolution-training-v1`. Any implementation can validate
against it with:

```text
python code/scripts/validate_convolution_training_labs.py
```

A language-native implementation needs only nested loops and arrays. A
performance-oriented binding can pass contiguous signal, kernel, and
output-gradient buffers to a Rust forward/backward core through a stable C ABI.
The existing Rust `dsp-conv` package can support an adapted forward path, but a
neural backward kernel still needs to be defined. Both paths should reproduce
the NN06 fixture before larger tensors, channels, or accelerators are added.

The next spatial tranche adds the axes hidden by this example: image height and
width, input and output channels, pooling, and normalization.
