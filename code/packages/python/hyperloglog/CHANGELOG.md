# Changelog

All notable changes to `coding-adventures-hyperloglog` are documented here.

## [0.1.0] — 2026-04-08

### Added

- Initial implementation of `HyperLogLog` class (DT21).
- `HyperLogLog.__init__(precision)` — creates an empty sketch with 2^precision registers.
  - Valid precision range: 4–16. Default: 14 (Redis default, ±0.81% error, ~12 KB).
  - Raises `ValueError` for out-of-range precision.
- `HyperLogLog.add(element)` — hashes element via FNV-1a 64-bit, splits hash into
  register index (top b bits) and leading-zero count (bottom 64-b bits), updates
  register with maximum ρ seen. O(1).
- `HyperLogLog.count()` — estimates distinct cardinality via harmonic mean of 2^M[j]
  across all registers, with α bias correction, small-range LinearCounting correction,
  and large-range logarithmic correction. O(m).
- `HyperLogLog.merge(other)` — returns new sketch representing the union of both
  sketches via element-wise max of registers. Raises `ValueError` on precision mismatch. O(m).
- `HyperLogLog.__len__()` — delegates to `count()`.
- `HyperLogLog.__repr__()` — shows precision, register count, and error rate.
- `HyperLogLog.precision` property.
- `HyperLogLog.num_registers` property — equals 2^precision.
- `HyperLogLog.error_rate` property — 1.04 / sqrt(num_registers).
- `HyperLogLog.error_rate_for_precision(precision)` static method.
- `HyperLogLog.memory_bytes(precision)` static method — returns packed 6-bit memory usage.
- `HyperLogLog.optimal_precision(desired_error)` static method — smallest b that meets error target.
- Internal `_count_leading_zeros(value, bit_width)` pure function — unit-tested independently.
- Internal `_alpha(m)` bias-correction constant function — unit-tested independently.
- Extensive test suite with >95% coverage:
  - Basic accuracy at 1K, 10K, 100K distinct elements
  - Duplicate suppression
  - Merge: disjoint sets, overlapping sets, empty sketches, immutability check
  - Precision validation
  - Static utility method correctness
  - Internal helper correctness
  - Memory invariants
  - Small-range correction behaviour
- `py.typed` marker for PEP 561 type-stub support.
- `BUILD` and `BUILD_windows` scripts for the repo build tool.
- `README.md` with usage examples and algorithm overview.
