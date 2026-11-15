# Changelog

## 0.1.0 - 2026-11-15

### Added
- Pure Python stdlib `SplitMix64` PRNG with reference vectors and unbiased `bounded(n)` rejection sampling.
- FNV-1a 64-bit hashing for deterministic test IDs and seed deriving.
- Property-based testing framework (`for_all`, `Gen` combinators, shrinking for primitives, bytes, text, and collections).
- Deterministic fuzz harness (`fuzz_bytes`, `fuzz_text`) with mutation algebra and crash fingerprinting.
- TFV v1 parser and serializer with depth and integer size limits.
- Structured assurance evidence emitter compatible with VER00/VER13.
- Full CWE-400 bounds enforcement (`MAX_CASES`, `MAX_INPUT_BYTES`, `MAX_SHRINK_STEPS`, `MAX_ITERATIONS`).
