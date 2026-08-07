# dsp-wavelets (C)

**Discrete Wavelet Transforms** — pure ISO C17. A faithful port of the Rust
[`dsp-wavelets`](../../rust/dsp-wavelets) crate.

## What it does

Implements the Discrete Wavelet Transform (DWT) and its inverse via the
**Mallat pyramid algorithm**, in one and two dimensions, for the orthogonal
wavelet families **Haar**, **Daubechies** (Db2/4/6/8), **Symlets** (Sym4), and
**Coiflets** (Coif1).

A wavelet transform decomposes a signal into scale-and-position-localised basis
functions — an adaptive time-frequency tiling. One forward step is two FIR
filter passes (a lowpass `h` and a highpass `g`) followed by downsample-by-2;
`levels` of DWT applies the pair recursively to the approximation, producing
the flattened layout `[cA_J | cD_J | cD_{J-1} | ... | cD_1]`.

The 2-D transform is separable (row-then-column) and yields the four sub-bands
`LL / HL / LH / HH` per level (the basis of JPEG 2000-style image coding).

## API

```c
#include "dsp_wavelets.h"

float signal[32] = { /* ... */ };
float *coeffs = NULL, *recon = NULL;
size_t clen = 0, rlen = 0;

if (wv_dwt_1d(signal, 32, wv_haar(), 3, WV_BOUND_PERIODIC, &coeffs, &clen) == WV_OK) {
    wv_idwt_1d(coeffs, clen, wv_haar(), 3, WV_BOUND_PERIODIC, 32, &recon, &rlen);
    /* recon reconstructs signal within ~1e-4 */
    wv_free(coeffs);
    wv_free(recon);
}
```

Transform routines allocate their result and return it via an out-pointer +
length; free it with `wv_free()`. `wv_slice_level` returns a *borrowed* pointer
into the caller's coefficient buffer. Every fallible function returns a
`wv_error_t` status code (`WV_OK == 0`). All arithmetic is single-precision
`float`, matching the crate's `f32` dtype.

## Building

```sh
sh BUILD          # POSIX: gcc and/or clang, via the shared iso-harness
```

Each compiler prints `N checks, 0 failed`. Verified clean under ASan + UBSan
and macOS `leaks` (the transforms are heavily allocation-driven).
