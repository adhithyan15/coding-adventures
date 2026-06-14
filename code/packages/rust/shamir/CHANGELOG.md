# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of Shamir's Secret Sharing over GF(2^8) under
  the AES reduction polynomial `X^8 + X^4 + X^3 + X + 1`.
- `split(secret, k, n) -> Vec<Share>` — split a byte secret into `n`
  shares such that any `k` reconstruct it. CSPRNG-drawn polynomial
  coefficients per byte; coefficients held in `Zeroizing<Vec<u8>>` so
  they wipe on early-return error paths and on natural drop.
- `combine(shares) -> Vec<u8>` — reconstruct via Lagrange interpolation
  at `x = 0`. Validates: shares must have consistent y-length, distinct
  `x`, and `x != 0`. No knowledge of `k` required (Shamir is
  threshold-agnostic at reconstruct time).
- `Share { x, y }` — single-share value. `Drop` zeroizes `y`. `Debug`
  prints `<redacted>` for `y` so test logs / panic messages cannot
  leak share material.
- `Share::encode` / `Share::decode` — `x || y` byte serialisation, 1
  byte header.
- `ShamirError` typed enum: `InvalidThreshold`, `EmptySecret`,
  `BelowThreshold`, `InconsistentShares`, `InvalidShare`, `Csprng`.
- 28 unit tests covering: GF(2^8) field axioms (XOR addition,
  multiplication commutativity, multiplicative identity, division
  inverts multiplication, AES MixColumns published vectors), threshold
  validation (k=0, k>n, n>255, empty secret), round-trips for
  `(k, n) ∈ {(2,3), (3,5), (3,7), (4,4), (1,3), (100,200)}` and for
  every single-byte secret, share-x distinctness, tamper detection
  (silently producing garbage — Shamir is unauthenticated by design),
  encode/decode round-trip, decode rejection of malformed and zero-x
  shares, and `Debug` redaction.
