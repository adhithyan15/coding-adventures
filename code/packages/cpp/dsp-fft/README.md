# dsp-fft (C++)

**Iterative 1-D FFT / IFFT / RFFT / IRFFT** — header-only pure ISO C++17
(`namespace ca::dsp_fft`). A faithful port of the [`dsp-fft`](../../rust/dsp-fft)
crate, built on [`cpp/dsp-complex`](../dsp-complex).

## What it does

Provides `fft_scalar`, `ifft_scalar`, `bluestein_scalar`, `rfft_scalar`,
`irfft_scalar`, and `ca::dsp_complex::ComplexTensor`-backed `fft`, `ifft`, `rfft`,
and `irfft` with backward `1/N` normalization.

## Building

```sh
sh BUILD
```
