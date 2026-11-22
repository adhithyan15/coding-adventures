# Changelog

All notable changes to `coding_adventures_hash_set` will be documented in this file.

## [0.1.0] - 2026-11-21

### Added
- Pure Dart implementation of the DT19 persistent hash set profile.
- Persistent operations: `adding`, `removing`, `discarding`.
- Persistent set algebra retaining receiver table profile: `union`, `intersection`, `difference`, `symmetricDifference`.
- Set relational predicates: `isSubsetOf`, `isSupersetOf`, `isDisjointWith`, `equals`, `==`.
- Underlying deterministic physical storage order via `elements`.
- Sealed exception hierarchy (`HashSetException`) complying with Standing Rule #20.
- 5-Class Security Showcase and exhaustive unit, property, and mutation fuzz test suites.
