# Changelog

## 0.2.0 — 2026-11-14

### Added
- `clamp_scalar` is public (RFC 7748 section 5, `decodeScalar25519`); it was a
  private helper.  `x25519` still clamps its scalar itself.
- Warrant pilot of fixed-length byte arrays (VER04j section 14.4):
  `code/specs/x25519.warrant` (RFC 7748 known answers, the clamp as a
  `Bytes<32> -> Bytes<32>` claim), `evidence.bindings`, the generated
  `tests/generated/warrant_evidence.rs` and its `tests/warrant_evidence.rs` shim.
  A pilot of the generator path, not an adoption: no report, no secret labels.
- Dev-dependencies `coding_adventures_test_bootstrap` and `coding_adventures_sha256`
  for the generated harness.

## 0.1.0 — 2026-04-12

### Added
- Initial implementation of X25519 (RFC 7748)
- Field arithmetic over GF(2^255 - 19) using radix-2^51 limbs
- Montgomery ladder with constant-time conditional swap
- Optimized squaring with doubled cross-terms
- Fermat inversion via addition chain
- `x25519()` — core scalar multiplication
- `x25519_base()` — multiplication by base point u=9
- `generate_keypair()` — public key derivation
- Full test suite with RFC 7748 test vectors
- Iterated test (1 and 1000 iterations)
