# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `dsp-wavelets` crate: Discrete Wavelet
  Transforms via the Mallat pyramid algorithm.
- 1-D forward/inverse DWT (`wv_dwt_1d` / `wv_idwt_1d`), band navigation
  (`wv_split_levels`, `wv_slice_level`), and separable 2-D forward/inverse DWT
  (`wv_dwt_2d` / `wv_idwt_2d`) with the `LL/HL/LH/HH` sub-band layout.
- Orthogonal wavelet families Haar, Daubechies (Db2/4/6/8), Symlets (Sym4), and
  Coiflets (Coif1); the tabulated `dec_lo` coefficients plus the QMF highpass
  derivation (`wv_analysis_lowpass`, `wv_qmf_highpass`). Symmetric and Periodic
  boundary extension.
- Status-code API (`wv_error_t`, `WV_OK == 0`); results are malloc'd and freed
  with `wv_free`, mirroring the crate's `Result`/`Vec<f32>` surface. All
  arithmetic is single-precision `float`.
- Defensive caps (`WV_MAX_LEVELS`, `WV_MAX_SAMPLES`) and checked allocations
  guarding against size_t overflow, matching the crate's security-review
  bounds.
- 2032 checks mirroring the crate's unit tests (error paths, the hand-worked
  Haar reference vector, constant/dirac structural properties, 1-D/2-D
  perfect-reconstruction round-trips, split/slice, and the filter-bank
  invariants) run under every ISO C compiler via the shared `iso-harness`.
  Verified clean under ASan + UBSan and macOS `leaks`.
