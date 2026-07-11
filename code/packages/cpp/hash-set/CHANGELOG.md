# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `hash-set` crate (DT19): a set
  implemented as a thin wrapper over the sibling `hash-map` package
  (`ca::hash_set<T>` = `ca::hash_map<T, unit>`).
- Membership: `add`, `remove`, `contains`, `size`, `empty`, `to_vector`.
- Set algebra (each returns a fresh set): `union_with` (`union` is a keyword),
  `intersection`, `difference`, `symmetric_difference`.
- Relations: `is_subset`, `is_superset`, `is_disjoint`, `equals`.
- Elements may be `std::string` or any trivially-copyable type.
- Header-only cross-package build wiring (`# build-tool: deps=cpp/hash-map`).
