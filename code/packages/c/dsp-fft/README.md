# dsp-fft (C)

**Iterative 1-D FFT / IFFT / RFFT / IRFFT** — pure ISO C17. A faithful port of
the [`dsp-fft`](../../rust/dsp-fft) crate, built on [`c/dsp-complex`](../dsp-complex).

## What it does

Computes 1-D Discrete Fourier Transforms on single-precision `float` buffers
with zero `<math.h>` / `-lm` dependency and zero recursion:

- **Radix-2 Cooley-Tukey** (`dsp_fft_scalar`, `dsp_ifft_scalar`) for `N = 2^k`.
- **Bluestein's algorithm** (`dsp_bluestein_scalar`) for arbitrary `N >= 1`.
- **Real-input half-spectrum** (`dsp_rfft_scalar`, `dsp_irfft_scalar`, `dsp_rfft`, `dsp_irfft`).
- **High-level dispatch** (`dsp_fft`, `dsp_ifft`) returning `dsp_complex_tensor_t`.

## Building

```sh
sh BUILD
```
