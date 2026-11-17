# coding-adventures-test-bootstrap

Zero-dependency Python test foundation providing deterministic PRNG (`SplitMix64`), property-based testing with shrinking, coverage-guided fuzz harness, and structured TFV/evidence summaries compatible with VER00/VER13 verified shipping policies.

## Features

- **SplitMix64 PRNG**: Exact 64-bit deterministic pseudorandom generator, matching reference vectors (`seed 0 -> 0xe220a8397b1dcdaf`), unbiased rejection sampling in `bounded(n)`, and FNV-1a 64-bit hashing.
- **Property-Based Testing**: `for_all` engine with rich generator combinators (`Gen.map`, `Gen.filter`, `Gen.flat_map`), integrated hierarchical shrinking (integers, bytes, text, lists, dicts), and reproducible counterexample reporting.
- **Fuzz Harness**: Deterministic mutation engine (`fuzz_bytes`, `fuzz_text`) supporting bit/byte flipping, insertions, deletions, replacements, and crossover with crash deduplication.
- **Structured Test Evidence & TFV**: In-memory TFV (TF01 §2 format) parser and serializer, along with deterministic JSON/TFV assurance report emission (VER00/VER13).
- **CWE-400 Hardening**: Strict upper bounds on input sizes (`MAX_INPUT_BYTES`), case counts (`MAX_CASES`), shrink iterations (`MAX_SHRINK_STEPS`), and fuzz iterations (`MAX_ITERATIONS`), with strict input validation on all entrypoints.
