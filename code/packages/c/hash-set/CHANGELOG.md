# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `hash-set` crate (DT19): a byte-string set
  implemented as a thin wrapper over the sibling `hash-map` package
  (`HashSet<T>` = `HashMap<T, ()>`).
- Membership: `hashset_new` / `hashset_new_with` / `hashset_free`, `hashset_add`,
  `hashset_remove`, `hashset_contains`, `hashset_size`, `hashset_is_empty`,
  `hashset_for_each`.
- Set algebra (each returns a fresh set): `hashset_union`,
  `hashset_intersection`, `hashset_difference`, `hashset_symmetric_difference`.
- Relations: `hashset_is_subset`, `hashset_is_superset`, `hashset_is_disjoint`,
  `hashset_equals`.
- Cross-package build wiring (`# build-tool: deps=c/hash-map`); the tests compile
  the sibling `hash-map` source and set algebra is built on `hashmap_for_each`.
