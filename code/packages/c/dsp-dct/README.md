# dsp-dct (C)

**Discrete Cosine and Sine Transforms** — pure ISO C17 (zero `<math.h>`, zero `-lm`).

## What it does

Implements 1-D **DCT-I, DCT-II, DCT-III, DCT-IV** and **DST-I, DST-II, DST-III, DST-IV**,
plus separable 2-D **DCT/IDCT** and **DST/IDST** under both un-normalised (`DSP_DCT_NORM_NONE`)
and orthonormal (`DSP_DCT_NORM_ORTHO`) scaling. DCT-II and DCT-III run in `O(N log N)` via
Makhoul's reduction over a self-contained iterative radix-2 + Bluestein FFT in double precision.

## Usage

```c
#include "dsp_dct.h"

float signal[8] = {1.0f, 2.0f, 3.0f, 4.0f, 5.0f, 6.0f, 7.0f, 8.0f};
float *coeffs = NULL, *recon = NULL;
size_t clen = 0, rlen = 0;

if (dsp_dct_1d(signal, 8, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &coeffs, &clen) == DSP_DCT_OK) {
    dsp_idct_1d(coeffs, clen, DSP_DCT_III, DSP_DCT_NORM_ORTHO, &recon, &rlen);
    dsp_dct_free(coeffs);
    dsp_dct_free(recon);
}
```

Alias-safe in-place calls (`out == signal`) are supported via `dsp_dct_1d_into`,
`dsp_idct_1d_into`, `dsp_dst_1d_into`, `dsp_idst_1d_into`, `dsp_dct_2d_into`, and
`dsp_idct_2d_into`.

## Building

```sh
sh BUILD
```
