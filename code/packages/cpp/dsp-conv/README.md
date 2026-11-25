# dsp-conv (C++)

**1-D, FFT/Overlap-Add, and 2-D Convolution & Image Filters** — header-only, self-contained pure ISO C++17 (zero `<cmath>` / `-lm`) in namespace `ca::dsp_conv`.

Faithful C++17 implementation matching the Rust [`dsp-conv`](../../rust/dsp-conv) and TypeScript [`dsp-conv`](../../typescript/dsp-conv) contracts.

## Features

- **1-D convolution**: `conv1d`, `convolve` (`Full`/`Same`/`Valid`), `correlate`.
- **Fast O(N log N) convolution**: `convolve_fft` (`fft_convolve`), iterative `overlap_add`.
- **2-D & separable convolution**: `conv2d`, `sep_conv2d`, `convolve_2d` (`convolve2d`).
- **Image filter kernels**: `gaussian_blur_kernel`, `box_blur_kernel`, `sobel_x_kernel`, `sobel_y_kernel`, `laplacian_kernel`, `sharpen_kernel`.
- **VER15 & Zero-CVE hardened**: pre-flight bounds before scans, checked `std::size_t` arithmetic, double intermediates with `|x| <= FLT_MAX` narrowing guards, and strictly iterative execution.

## Building

```sh
sh BUILD
```

On Windows:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\run.ps1
```
