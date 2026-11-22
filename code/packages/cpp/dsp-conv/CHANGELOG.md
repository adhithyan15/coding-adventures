# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-21

### Added

- Header-only, self-contained pure ISO C++17 (zero `<cmath>` / `-lm`) implementation of `dsp-conv` in `ca::dsp_conv`.
- 1-D same-size convolution (`conv1d`), `Full`/`Same`/`Valid` convolution (`convolve`), cross-correlation (`correlate`), O(N log N) FFT convolution (`convolve_fft`), and iterative overlap-add (`overlap_add`).
- 2-D same-size (`conv2d`), separable (`sep_conv2d`), and `Full`/`Same`/`Valid` (`convolve_2d`) convolution plus Gaussian, box, Sobel X/Y, Laplacian, and sharpen kernels.
- VER15 & Zero-CVE hardening with unit, Family-1/4 invariant, hostile-input, and deterministic SplitMix64 PBT suites.
