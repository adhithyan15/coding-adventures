# Changelog

All notable changes to `@coding-adventures/pbt` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-08

### Added
- Standalone extraction of the TypeScript Level 2 Property-Based Testing engine (VER05).
- Zero runtime dependencies (`"dependencies": {}`).
- SplitMix64 64-bit deterministic PRNG with uniform bounded sampling and FNV-1a 64-bit hash.
- Monadic generator algebra (`Gen<T>`) supporting `map`, `filter`, `flatMap`, `list`, `oneOf`, `tuple`, `constant`, `integer`, `bytes`, `bool`, `u8`, `u64`.
- Integrated shrinkers: `shrinkInt`, `shrinkList`, and `shrinkBytes`.
- Deterministic replay verification via `simulateReplay`.
- Privacy-preserving failure reporting for CI environments with FNV-1a counterexample hashing.
- Full conformance to all 25 canonical test vectors in `code/fixtures/pbt-v1/cases.json`.
