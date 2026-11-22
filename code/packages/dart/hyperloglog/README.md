# coding_adventures_hyperloglog

Pure Dart implementation of the DT21 persistent HyperLogLog cardinality estimation sketch with Standing Rule #20 diagnostics.

## Features

- **Persistent & Immutable**: Every addition and merge returns a new instance, leaving the receiver unchanged.
- **Deterministic Hashing**: Combines DT17 FNV-1a 64-bit with MurmurHash3 64-bit `fmix64` finalization.
- **Standing Rule #20 Compliant**: Closed sealed exception hierarchy (`HyperLogLogException`), centralized diagnostic messages, and zero payload reflection.

## Security Showcase (5 Vulnerability Classes Mitigated)

1. **ReDoS / Pattern Exploits**: Zero regular expressions are employed; raw byte streaming and integer bit-shifts eliminate regex backtracking attacks.
2. **State & Memory Mutation**: All internal registers are stored in unmodifiable lists (`List.unmodifiable`), preventing prototype or state tampering across threads or callers.
3. **Type Confusion**: Input arguments are strictly typed with sealed error handling; unsupported objects throw sanitized payload-blind `HyperLogLogException.invalidInput()`.
4. **Resource Exhaustion (Algorithmic Complexity)**: Sketch precision is strictly bounded to `4 <= p <= 16`, capping register allocations to at most 65,536 elements (`O(1)` memory bound).
5. **Information Leakage**: Exception messages are static constants under Standing Rule #20; no input data, secret tokens, or user payloads are reflected in error messages or string representations.
