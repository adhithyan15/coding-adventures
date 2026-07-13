# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Header-only ISO C++17 port of the Rust `dsp-wavelets` crate in namespace
  `ca::dsp_wavelets`: Discrete Wavelet Transforms via the Mallat pyramid
  algorithm.
- 1-D forward/inverse DWT (`dwt_1d` / `idwt_1d`), band navigation
  (`split_levels`, `slice_level`), and separable 2-D forward/inverse DWT
  (`dwt_2d` / `idwt_2d`) with the `LL/HL/LH/HH` sub-band layout.
- Orthogonal wavelet families Haar, Daubechies (Db2/4/6/8), Symlets (Sym4), and
  Coiflets (Coif1) via `Wavelet` factories; the tabulated `dec_lo` coefficients
  plus the QMF highpass derivation (`analysis_lowpass`, `qmf_highpass`).
  Symmetric and Periodic boundary extension.
- `std::vector<float>` in/out (mirroring `Vec<f32>`); `slice_level` returns a
  borrowed `FloatView`. Where the Rust crate returns `Result`, this port throws
  a `WaveletError` carrying an `Error` code. Defensive `MAX_LEVELS` /
  `MAX_SAMPLES` caps.
- 941 checks mirroring the crate's unit tests (error paths, the hand-worked
  Haar reference vector, constant/dirac structural properties, 1-D/2-D
  perfect-reconstruction round-trips, split/slice, and the filter-bank
  invariants) run under every ISO C++ compiler via the shared `iso-harness`.
  Verified clean under ASan + UBSan.
