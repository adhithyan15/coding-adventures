# Changelog — CodingAdventures::Matrix (Perl)

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/).

---

## [0.02] — 2026-04-04

### Added (ML03 Matrix Extensions)

- **Reductions:** `sum()`, `sum_rows()`, `sum_cols()`, `mean()`, `mat_min()`, `mat_max()`, `argmin()`, `argmax()`
- **Element-wise math:** `mat_map($fn)`, `mat_sqrt()`, `mat_abs()`, `mat_pow($exp)`
- **Shape operations:** `flatten()`, `reshape($rows, $cols)`, `mat_row($i)`, `mat_col($j)`, `slice($r0, $r1, $c0, $c1)`
- **Equality and comparison:** `equals($B)`, `close($B, $tol)`
- **Factory methods:** `identity($n)`, `from_diagonal($values)`

### Changed

- `set($i, $j, $val)` now returns a **new** matrix instead of mutating in place
- `get($i, $j)` now validates bounds and dies on out-of-range indices

---

## [0.01] — 2026-03-29

### Added

- `zeros($rows, $cols)` — allocate an all-zero m×n matrix
- `from_2d($data)` — construct from a nested arrayref (deep copy)
- `from_1d($data)` — construct a 1×n row vector from a flat arrayref
- `from_scalar($val)` — construct a 1×1 matrix
- `rows()`, `cols()`, `data()` — dimension and raw-data accessors
- `get($i, $j)`, `set($i, $j, $val)` — element access (zero-based)
- `add($B)` — element-wise addition with dimension check
- `add_scalar($s)` — add a scalar to every element
- `subtract($B)` — element-wise subtraction with dimension check
- `scale($s)` — multiply every element by a scalar
- `transpose()` — flip rows and columns
- `dot($B)` — matrix multiplication with inner-dimension check
- `Makefile.PL` and `cpanfile` for CPAN-style packaging
- `t/00-load.t` and `t/01-basic.t` with property and identity-matrix checks
