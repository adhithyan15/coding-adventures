# dsp-complex (C)

**Interleaved Complex Tensor & Scalar Operations** — pure ISO C17. A faithful
port of the [`dsp-complex`](../../rust/dsp-complex) crate for the DSP layer.

## What it does

Stores complex signals and spectra in interleaved `[re_0, im_0, re_1, im_1, ...]`
single-precision `float` buffers with zero `<math.h>` or `-lm` dependency.

- **Checked constructors**: `dsp_complex_tensor_zeros`, `dsp_complex_tensor_from_real`,
  `dsp_complex_tensor_from_real_imag`, `dsp_complex_tensor_from_interleaved`, `dsp_complex_tensor_clone`.
- **Safe indexing**: `dsp_complex_tensor_get`, `dsp_complex_tensor_set`.
- **Projections & arithmetic**: `real`, `imag`, `magnitude`, `phase`, `conjugate`,
  `scale`, `add`, `sub`, `mul`, `div`.
- **Zero-on-free cleanup**: `dsp_complex_tensor_free`, `dsp_complex_free_buffer`.

## Building

```sh
sh BUILD
```
