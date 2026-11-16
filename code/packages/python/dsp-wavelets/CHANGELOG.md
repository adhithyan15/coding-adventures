# Changelog

## 0.1.0 - 2026-11-15

### Added
- `Wavelet` class supporting Haar, Daubechies, Symlets, Coiflets, Biorthogonal, Morlet, and Ricker / Mexican Hat.
- Boundary extension modes (`Zero`, `Replicate`, `Reflect`, `Symmetric`, `Periodic`).
- `dwt` and `idwt` single-level 1-D discrete wavelet transform and synthesis.
- `wavedec` and `waverec` multi-level decomposition and reconstruction.
- `dwt_1d` and `idwt_1d` flattened Mallat pyramid transforms.
- `swt` and `iswt` stationary wavelet transform (undecimated DWT) and inverse.
- `cwt` continuous wavelet transform for Morlet and Ricker wavelets.
- `dwt_2d` and `idwt_2d` 2-D separable quad-tree image wavelet transforms.
- VER15 security hardening against CWE-400, CWE-20, CWE-674, CWE-1333, and pure zero-capability sandbox contract.
