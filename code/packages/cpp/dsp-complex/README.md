# dsp-complex (C++)

**Interleaved Complex Tensor & Scalar Operations** — header-only pure ISO C++17
(`namespace ca::dsp_complex`). A faithful port of the [`dsp-complex`](../../rust/dsp-complex) crate.

## What it does

Provides `ca::dsp_complex::Complex` and `ca::dsp_complex::ComplexTensor` (`ComplexArray`)
for interleaved `[re_0, im_0, ...]` single-precision `float` buffers with checked sizing,
finite-number validation, bounds-checked element indexing, projections, and element-wise
arithmetic.

## Building

```sh
sh BUILD
```
