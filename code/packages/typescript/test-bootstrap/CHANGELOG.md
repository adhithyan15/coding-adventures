# Changelog

## 0.1.0 (2026-11-08)

- Initial release of `@coding-adventures/test-bootstrap`.
- Strict TFV v1 parser and validator (TF01 §2).
- Pure deep equality algorithm matching TF01 §3.1 (int/bigint, canonical f64 NaN, -0 !== +0, bytes, text, list, map).
- Deterministic SplitMix64 pseudo-random generator, FNV-1a 64 test-id hashing, Fisher-Yates array shuffling, and isolated sub-seeds (TF01 §5).
- TAP version 14 test reporter and failure-detecting sabotage mode (TF01 §6).
- Ergonomic Level 1 matchers (`describe`, `it`, `test`, `expect`, `beforeEach`, `afterEach`) over Node 22 native `node:test`.
- Zero external runtime dependencies.
