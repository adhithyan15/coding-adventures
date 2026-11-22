# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-21

### Added

- Self-contained pure ISO C17 (zero `<math.h>` / `-lm`) implementation of `dsp-conv`.
- 1-D same-size convolution (`conv_conv1d`), `Full`/`Same`/`Valid` convolution (`conv_convolve`), and cross-correlation (`conv_correlate`).
- O(N log N) FFT convolution (`conv_convolve_fft`) and iterative overlap-add (`conv_overlap_add`).
- 2-D same-size (`conv_conv2d`), separable (`conv_sep_conv2d`), and `Full`/`Same`/`Valid` (`conv_convolve_2d`) convolution plus Gaussian, box, Sobel X/Y, Laplacian, and sharpen kernel generators.
- VER15 & Zero-CVE hardening: pre-flight bounds before scans, checked `size_t` math, double intermediates with `|x| <= FLT_MAX` narrowing guards, volatile zero-on-free (`conv_free_buf`), unit, Family-1/4 invariant, hostile-input, and deterministic SplitMix64 PBT suites.
