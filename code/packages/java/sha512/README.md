# Java SHA-512 (`com.codingadventures.sha512`)

Pure, from-scratch Java implementation of the SHA-512 cryptographic hash algorithm (FIPS 180-4) with zero third-party dependencies and zero host-cryptography substitution (`java.security.MessageDigest` is never imported in production code).

## Features
- **FIPS 180-4 Standard Compliance**: Full 80-round 64-bit Merkle–Damgård compression with exact round constants and initial state.
- **One-Shot & Streaming**:
  - `Sha512.sha512(byte[] data)` returns 64-byte `byte[]`.
  - `Sha512.sha512Hex(byte[] data)` returns 128-character lowercase hexadecimal string.
  - `Sha512.Hasher` (`Sha512.create()`) provides streaming incremental hashing with `update`, `digest`, `hexDigest`, and `copy`.
- **128-Bit Bit-Length Preservation**: Supports streaming counters beyond $2^{61}$ bytes without wraparound and rejects inputs $\ge 2^{125}$ bytes before state mutation.
- **Standing Rule #20**: Zero inline error literals in operational logic; all error diagnostics are centralized in `Sha512Messages`.
- **Zero Magic Numbers**: All wire constants, shift offsets, and masks are centralized in `Sha512Constants`.

## 6-Class SHA-512 Resilience Showcase

| Class | Vulnerability Vector | Defense Mechanism | Test Case |
|---|---|---|---|
| 1 | 64-bit Counter Wraparound | Independent 128-bit length counter (`byteCountHigh`, `byteCountLow`) | Verifies carry from low to high word beyond $2^{61}$ bytes |
| 2 | FIPS Domain Bit Overflow | Rejection before state mutation when length $\ge 2^{125}$ bytes | Verifies `Sha512Exception` and state preservation on overflow attempt |
| 3 | Padding Boundary Splits | Exact block handling across 111, 112, 127, 128, 129 byte boundaries | Verifies multi-block and single-block padding conformance |
| 4 | Buffer Aliasing & Continuation | Deep-copied buffer in `update` and non-destructive snapshot in `digest()` | Modifies caller array post-update and continues updates post-digest |
| 5 | Null Input Fail-Closed | Synchronous null check rejecting null inputs before state modification | Verifies `NullPointerException` on null array input |
| 6 | Payload-Blind Diagnostics | Extracted static error messages without reflection of caller payload | Validates Standing Rule #20 conformance via `Sha512Messages` |
