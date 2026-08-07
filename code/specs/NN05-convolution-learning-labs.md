# NN05: Convolution Learning Labs

## Status

Draft specification for deterministic, language-neutral spatial traces.

## Purpose

NN03 exposes neuron arithmetic and NN04 exposes optimization. NN05 begins the
spatial track with the smallest useful operation: slide one shared kernel over
a one-dimensional signal and reveal every multiply-accumulate.

The same fixture can drive a browser visualizer, a hand-worked lesson, a native
implementation, or an adapter over the Rust DSP core.

## V1 Operation

V1 uses the convention called **convolution** by neural-network libraries but
mathematically defined as cross-correlation:

```text
output[i] = sum(signal[i + j] * kernel[j] for j = 0..K-1)
```

The kernel is not reversed. V1 is deliberately limited to:

- one one-dimensional signal;
- one one-dimensional kernel;
- stride one;
- valid padding, so the kernel never extends beyond the signal;
- no bias or activation;
- binary64 arithmetic.

For signal length `N` and kernel length `K`, V1 produces `N - K + 1` values.
The kernel must be non-empty and no longer than the signal.

## Trace Contract

Every output position stores enough information to reproduce the computation:

- `start_index`: the zero-based signal index beneath kernel element zero;
- `window`: the `K` signal values under the kernel;
- `products`: element-wise `window[j] * kernel[j]` values;
- `accumulator`: the running sum, including the initial zero;
- `output`: the final accumulator value.

Thus `accumulator` always contains `K + 1` values and begins with zero. Its last
value must equal both the position's `output` and the corresponding value in
`expected.outputs`.

## Why an Asymmetric Kernel

The canonical fixture uses `[1, -1, 2]`. Reversing it changes the answer, so an
implementation cannot accidentally substitute mathematical convolution for
the neural-network convention and still pass.

## Fixture Layout

The V1 corpus lives in:

```text
code/specs/fixtures/convolution-learning-v1/
  schema.json
  labs/*.json
```

Validate and execute it with:

```text
python code/scripts/validate_convolution_learning_labs.py
```

## Conformance Levels

- **Output conformance:** reproduce every final output within tolerance.
- **Trace conformance:** reproduce every window, product, and accumulator.
- **Inspectable conformance:** let a learner select a position and see how the
  shared kernel produced it.
- **Accelerated conformance:** execute through a native or Rust-backed core
  while preserving the same trace oracle.

The existing Rust `dsp-conv` package implements same-size mathematical
convolution. A conforming NN05 adapter can reverse the neural kernel before the
call and select the valid interior, or implement the V1 cross-correlation loop
directly. Either path must be checked against this corpus. Later NN05 versions
can add bias, padding, stride, channels, trainable kernels, and gradients
without changing V1 behavior.
