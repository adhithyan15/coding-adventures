# Changelog

All notable changes to the C++ `matrix` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `matrix` crate
  (namespace `ca::matrix`), storing elements as
  `std::vector<std::vector<double>>` exactly like the Rust `Vec<Vec<f64>>`.
- Constructors (`Matrix`, `new_1d`, `new_scalar`, `zeros`, `identity`,
  `from_diagonal`); arithmetic (`add`, `subtract`, `add_scalar`, `scale`,
  `transpose`, `dot`); element access (`get`, immutable `set`); reductions
  (`sum`, `mean`, `min_val`, `max_val`, `sum_rows`, `sum_cols`, `argmin`,
  `argmax`); element-wise math (`map`, `sqrt`, `abs_val`, `pow_val`); shape ops
  (`flatten`, `reshape`, `row`, `col`, `slice`); and comparison (`equals`,
  `close`).
- Dimension mismatches throw `std::invalid_argument` and bad indices throw
  `std::out_of_range`, in place of the Rust `Result<Matrix, _>`.
- `sqrt` and a general `pow` computed without `<cmath>`; the integer exponent
  path is exact, the fractional path uses `exp(y·ln x)`.
- Memory-safety guards: the constructor rejects ragged (non-rectangular) input,
  and `reshape` guards its `size_t` dimension product against overflow before
  comparing element counts, so an overflowed product can never alias the true
  count and drive iterator arithmetic out of bounds.
- 86 checks against the Rust crate's own reference values, run under every
  available C++ compiler via the shared `iso-harness`.
