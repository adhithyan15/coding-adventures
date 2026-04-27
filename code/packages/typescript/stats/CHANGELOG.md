# Changelog

## 0.1.0 — 2026-04-04

### Added

- Initial implementation of ST01 statistics package.
- Descriptive statistics: mean, median, mode, variance, standard_deviation,
  min, max, range.
- Frequency analysis: frequency_count, frequency_distribution, chi_squared,
  chi_squared_text.
- Cryptanalysis helpers: index_of_coincidence, entropy.
- Constants: ENGLISH_FREQUENCIES (standard English letter frequencies).
- Tree-shakeable architecture: one file per function with barrel re-export.
- Comprehensive test suite covering all parity test vectors from ST01 spec.
