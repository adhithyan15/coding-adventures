# Changelog — bloom-filter

All notable changes to this package will be documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning: [Semantic Versioning](https://semver.org/).

---

## [0.1.0] — 2026-04-08

### Added

- `BloomFilter` class implementing a space-efficient probabilistic set membership
  filter (DT22).
- Auto-sizing constructor: computes optimal bit count `m` and hash count `k`
  from `expected_items` and `false_positive_rate` using the standard formulas.
- `BloomFilter.from_params(bit_count, hash_count)` class method for explicit
  parameter control (bypasses auto-sizing).
- `add(element)` — sets k bits in the bit array using double hashing.
- `contains(element)` — checks k bits; returns False only if element is
  definitely absent (zero false negatives).
- `__contains__` dunder enabling `element in bf` syntax.
- Properties: `bit_count`, `hash_count`, `bits_set`, `fill_ratio`,
  `estimated_false_positive_rate`.
- `is_over_capacity()` — True when more elements than expected have been added.
- `size_bytes()` — memory usage of the bit array.
- Static utilities: `optimal_m(n, p)`, `optimal_k(m, n)`,
  `capacity_for_memory(memory_bytes, p)`.
- `__repr__` showing m, k, bits_set, fill_ratio, and estimated FPR.
- Double hashing using `fnv1a_32` (h1) and `djb2` (h2) from
  `coding-adventures-hash-functions` (DT17).
- Compact bit array stored as `bytearray`, 8 bits per byte.
- Full type annotations with `from __future__ import annotations`.
- Literate programming inline documentation: algorithm diagrams, math,
  derivations, real-world use cases.
- 95%+ test coverage enforced via pytest-cov.
- Tests covering: no false negatives, FPR within 2× target, all properties,
  from_params factory, optimal parameter formulas, over-capacity detection,
  determinism, edge cases (empty string, None, Unicode, duplicates), and
  __repr__/__contains__.
