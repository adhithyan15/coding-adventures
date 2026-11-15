# Changelog

All notable changes to `coding-adventures-dsp-complex` will be documented in this file.

## [0.1.0] - 2026-11-15

### Added
- Initial pure-Python `ComplexTensor` implementation.
- Checked constructors: `try_from_real`, `from_real`, `from_real_imag`, `from_interleaved`, and `from_complex_list`.
- Projections: `real()`, `imag()`, `magnitude()`, `phase()`, and `conjugate()`.
- Pre-flight CWE-400 resource checks (`MAX_SAMPLES = 16_777_216`).
- CWE-20 finite-float enforcement rejecting `NaN` and `Inf`.
- Warrant specification `dsp-complex.warrant` and required capabilities manifest.
