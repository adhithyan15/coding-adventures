# NN06: Convolution Training Labs

## Status

Draft specification for deterministic, language-neutral 1D convolution
gradient traces.

## Purpose

NN05 shows how one shared kernel produces a feature map. NN06 makes that
kernel trainable and exposes the key consequence of weight sharing: every
output position can contribute to the gradient of every kernel weight.

The V1 fixture pins one forward pass, one analytical backward pass, an
independent finite-difference check, and one gradient-descent update.

## V1 Operation

V1 retains the NN05 forward convention:

```text
output[i] = sum(signal[i + j] * kernel[j] for j = 0..K-1)
```

The kernel is not reversed. Padding is valid, stride is one, and there is one
signal, one kernel, no bias, and no activation.

For `M = N - K + 1` outputs and targets, V1 uses mean squared error:

```text
error[i]          = output[i] - target[i]
loss              = sum(error[i]^2) / M
dLoss/dOutput[i]  = 2 * error[i] / M
```

## Shared-Kernel Gradient

Kernel weight `j` appears once in every output window. Its gradient is the sum
of all those paths:

```text
contribution[i][j] = dLoss/dOutput[i] * signal[i + j]
dLoss/dKernel[j]   = sum(contribution[i][j] for i = 0..M-1)
```

The trace stores every per-output contribution before the reduction. This is
the central learning goal of NN06: shared parameters accumulate gradients.

## Independent Gradient Check

For each kernel weight `k[j]`, V1 uses a central finite difference while all
other values remain fixed:

```text
numeric[j] = (loss(k[j] + epsilon) - loss(k[j] - epsilon)) / (2 * epsilon)
```

The numerical implementation must not call or reuse the analytical backward
pass.

## Update

One plain gradient-descent step updates every kernel weight simultaneously:

```text
next_kernel[j] = kernel[j] - learning_rate * dLoss/dKernel[j]
```

The expected next outputs and loss are evaluated after the complete kernel has
been updated.

## Fixture Layout

The V1 corpus lives in:

```text
code/specs/fixtures/convolution-training-v1/
  schema.json
  labs/*.json
```

Validate and execute it with:

```text
python code/scripts/validate_convolution_training_labs.py
```

## Conformance Levels

- **Forward conformance:** reproduce outputs, errors, and loss.
- **Backward conformance:** reproduce every contribution and reduced kernel
  gradient.
- **Gradient-check conformance:** independently match the numerical gradient.
- **Update conformance:** reproduce the next kernel, outputs, and loss.
- **Inspectable conformance:** let a learner follow one output's path into each
  shared weight, then see the cross-position reduction.

## Native and Rust Direction

The existing Rust `dsp-conv` package can accelerate the adapted forward pass,
but it does not currently define neural-network gradients. A small Rust NN
kernel should expose valid cross-correlation forward and backward operations
over caller-owned contiguous buffers. Thin language bindings can pass signal,
kernel, and output-gradient pointers through a stable C ABI, while native
implementations use the same JSON fixture as their oracle.

The backward kernel is deliberately just the formula above. A future tensor
runtime may lower it to matrix or convolution primitives, but the simple loop
must remain available as the reference path and finite-difference checks must
continue to validate accelerated implementations.
