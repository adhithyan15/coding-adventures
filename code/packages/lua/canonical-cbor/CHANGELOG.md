# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-14

### Added
- RFC 8949 deterministic canonical CBOR codec implementing §4.2.3 Length-First Map Key Ordering (CBR01 profile).
- Pure Lua 5.4 implementation with zero runtime dependencies.
- Full 55/55 test fixture conformance from `code/specs/fixtures/canonical-cbor-v1/cases.json`.
- Centralized `messages.lua` containing all 14 closed payload-blind error diagnostics and `<redacted>` placeholder.
- Centralized `constants.lua` defining zero-magic-constant protocol parameters with read-only metatable protection.
- Immutable algebraic value model (`types.lua`) with metatable TOCTOU spoofing defenses and strict UTF-8 validation.
- Atomic `encode_into_checked` buffer appending with rollback on encoding errors.
- Spanned map entry decoding (`decode_map_spans`) with 0-based offset spans `[start, finish)`.
- 7-class Lua CVE Resilience Showcase defending against 64-bit integer wraparound (`0xFFFFFFFFFFFFFFFF == -1`), pre-allocation OOM bombs, recursion exhaustion, signature bypass, UTF-8 smuggling, trailing-byte smuggling, and secret reflection leakage.
- Deterministic PRNG round-trip and map-permutation property tests, plus 10,000-iteration byte-mutation fuzz loop.
- Offline `debug.sethook` line-coverage harness asserting >= 95% coverage across all source files.
