# Changelog

All notable changes to `coding-adventures-dsp-dct` will be documented in this file.

## [0.1.0] - 2026-11-15

### Added
- Pure Python DCT-I, DCT-II, DCT-III, and DCT-IV forward and inverse transforms.
- Fast $O(N \log N)$ DCT-II via FFT and DCT-III via IFFT (Makhoul reduction).
- DST-I, DST-II, DST-III, and DST-IV forward and inverse transforms.
- 2-D transforms: `dct_2d` and `idct_2d`.
- Normalization options: `DctNorm.NONE` and `DctNorm.ORTHO`.
- Pre-flight CWE-400 bounds checks (`MAX_DCT_LEN = 1_048_576`, `MAX_SAMPLES = 16_777_216`).
- CWE-20 finite-float enforcement rejecting `NaN` and `Inf`.
- Warrant specification `dsp-dct.warrant` and required capabilities manifest.
