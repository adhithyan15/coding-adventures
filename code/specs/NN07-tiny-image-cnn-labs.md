# NN07: Tiny Image CNN Labs

## Status

Draft specification for deterministic, language-neutral image convolution,
channel reduction, normalization, activation, and pooling traces.

## Purpose

NN05 and NN06 introduce one-dimensional shared kernels. NN07 turns the signal
into a tiny image and makes the dimensions that are usually hidden inside a
tensor library explicit:

1. each output filter owns one kernel for every input channel;
2. each channel produces a partial sum at the same spatial position;
3. the partial sums and one bias become a feature-map value;
4. each output channel is normalized over its spatial positions;
5. ReLU removes negative normalized values; and
6. max pooling keeps the strongest value and its location.

The fixture is deliberately small enough to calculate on paper: two `3 x 3`
input channels, two `2 x 2` filters, two `2 x 2` output maps, and one pooled
value per output channel.

## V1 Forward Pipeline

V1 uses cross-correlation, so kernels are not reversed. For output filter `f`
at row `r`, column `c`:

```text
channel_sum[f, d, r, c] =
  sum(input[d, r + kr, c + kc] * kernel[f, d, kr, kc])

convolution[f, r, c] = bias[f] + sum(channel_sum[f, d, r, c] for d)
```

Padding is valid and both spatial strides are one.

## Spatial Per-Channel Normalization

V1 normalizes the four values in each output channel independently. Population
variance is used, followed by an affine scale and shift:

```text
mean[f]     = sum(convolution[f]) / spatial_count
variance[f] = sum((value - mean[f])^2) / spatial_count
denom[f]    = sqrt(variance[f] + epsilon)

normalized[f, r, c] =
  gamma[f] * (convolution[f, r, c] - mean[f]) / denom[f] + beta[f]
```

This is an intentionally small normalization primitive, not a claim that all
frameworks use the same axes. Batch normalization, instance normalization, and
layer normalization differ mainly in which values share the statistics.

The teaching fixture uses `variance = 5` and `epsilon = 4`, giving a denominator
of exactly `3`. The unusually large epsilon is pedagogical: it keeps every
normalization result hand-calculable while still pinning the stabilizing term.
Stored floating-point results use the fixture's `absolute_tolerance` for
comparison.

## Activation and Pooling

ReLU is elementwise:

```text
activated = max(0, normalized)
```

V1 then applies non-overlapping `2 x 2` max pooling. The expected fixture stores
both each pooled value and its zero-based `[row, column]` winner. Ties are broken
by the first value in row-major order.

## Fixture Layout

The V1 corpus lives in:

```text
code/specs/fixtures/tiny-image-cnn-v1/
  schema.json
  labs/*.json
```

Validate and execute it with:

```text
python code/scripts/validate_tiny_image_cnn_labs.py
```

## Conformance Levels

- **Channel conformance:** reproduce each input channel's partial contribution.
- **Convolution conformance:** reduce channels, add bias, and reproduce both
  feature maps.
- **Normalization conformance:** reproduce means, population variances,
  denominators, and normalized maps.
- **Activation conformance:** reproduce the ReLU maps.
- **Pooling conformance:** reproduce max values and row-major winner positions.
- **Inspectable conformance:** expose the window, products, channel sums, bias,
  normalization arithmetic, and pool winner for a selected output.

## Native and Rust Direction

The existing Rust `dsp-conv::conv2d` primitive already handles row-major 2D
images, but it performs centered, same-size mathematical convolution one
channel at a time. NN07 instead pins valid, unflipped cross-correlation followed
by a reduction across input channels and a bias. A tensor-facing adapter can
flip and crop around `dsp-conv` for an initial parity experiment, but the
long-term Rust NN core should expose this convention directly with explicit
NCHW or NHWC layout, normalization, ReLU, and pooling over caller-owned
contiguous buffers.

The Rust `feature-normalization` package demonstrates deterministic population
mean and variance over table columns. Its reductions are useful reference code,
but its grouping axes and zero-variance behavior are not yet NN07's per-output-
channel affine normalization. The first stable C ABI should take dimensions,
strides, scalar parameters, and pointers explicitly rather than exposing Rust
containers.

Every native implementation and binding should consume this JSON fixture before
an accelerated path is considered conformant. Later lowering work can fuse the
pipeline, but a reference mode must retain intermediate channel contributions,
normalization statistics, and pool indices so learners and debuggers can still
open the fused operation.
