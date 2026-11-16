# Changelog

## 0.1.0 - 2026-11-15

### Added
- `conv1d` same-size 1-D convolution with four boundary modes (`Zero`, `Replicate`, `Reflect`, `Wrap`).
- `convolve` with `Full`, `Same`, `Valid` modes and boundary extensions.
- `correlate` 1-D cross-correlation.
- `fft_convolve` FFT-based convolution via `dsp-fft`.
- `overlap_add` block-convolution algorithm.
- `conv2d` and `convolve2d` for same-size and discrete 2-D filtering on row-major images.
- `sep_conv2d` separable 2-D convolution fast path.
- Image filter design helpers (`gaussian_blur_kernel`, `sobel_x_kernel`, `sobel_y_kernel`, `box_blur_kernel`, `laplacian_kernel`, `sharpen_kernel`).
- VER15 hardening against CWE-400, CWE-20, CWE-674, CWE-1333, and pure zero-capability sandbox contract.
