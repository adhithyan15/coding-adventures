# dsp-conv (C)

**1-D, FFT/Overlap-Add, and 2-D Convolution & Image Filters** — self-contained pure ISO C17 (zero `<math.h>` / `-lm`).

Faithful C17 implementation matching the Rust [`dsp-conv`](../../rust/dsp-conv) and TypeScript [`dsp-conv`](../../typescript/dsp-conv) contracts.

## Features

- **1-D convolution**: `conv_conv1d` (same-size with `Zero`/`Replicate`/`Reflect`/`Wrap` boundary modes), `conv_convolve` (`Full`/`Same`/`Valid`), `conv_correlate` (cross-correlation).
- **Fast O(N log N) convolution**: `conv_convolve_fft` and iterative `conv_overlap_add` using an internal self-contained double-precision radix-2 FFT.
- **2-D & separable convolution**: `conv_conv2d` (same-size), `conv_sep_conv2d` (row-then-column), `conv_convolve_2d` (`Full`/`Same`/`Valid`).
- **Image filter kernels**: `conv_gaussian_blur_kernel`, `conv_box_blur_kernel`, `conv_sobel_x_kernel`, `conv_sobel_y_kernel`, `conv_laplacian_kernel`, `conv_sharpen_kernel`.
- **VER15 & Zero-CVE hardened**: pre-flight bounds before element scans, checked `size_t` arithmetic, double intermediates with `|x| <= FLT_MAX` narrowing guards, and idempotent volatile zero-on-free (`conv_free_buf`).

## Building

```sh
sh BUILD
```

On Windows:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\run.ps1
```
