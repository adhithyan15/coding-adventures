# coding_adventures_bloom_filter

Pure Dart implementation of the DT22 persistent Bloom filter with Standing Rule #20 diagnostics.

## Features

- **Persistent & Immutable**: Every insertion returns a new instance, leaving the receiver unchanged.
- **Deterministic Double Hashing**: Combines DT17 FNV-1a 32-bit and DJB2 with MurmurHash3 32-bit finalizer.
- **Standing Rule #20 Compliant**: Closed sealed exception hierarchy (`BloomFilterException`), centralized diagnostic messages, and zero payload reflection.

## Security Showcase (5 Vulnerability Classes Mitigated)

1. **ReDoS / Pattern Exploits**: Zero regular expressions are employed; raw byte streaming and integer bit-shifts eliminate regex backtracking attacks.
2. **State & Memory Mutation**: All internal bit arrays are stored in unmodifiable lists (`List.unmodifiable`), preventing state corruption across callers or concurrent reads.
3. **Type Confusion**: Input arguments are strictly typed with sealed error handling; unsupported objects throw sanitized payload-blind `BloomFilterException.invalidInput()`.
4. **Resource Exhaustion (Algorithmic Complexity)**: Bit counts are strictly bounded to at most `134,217,728` bits (16 MB), and hash counts are capped at `64`, preventing out-of-memory denial-of-service.
5. **Information Leakage**: Exception messages are static constants under Standing Rule #20; no input data, secret tokens, or user payloads are reflected in error messages or string representations.
