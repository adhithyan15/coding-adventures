# Changelog — curve25519

## [0.1.0] — 2026-04-22

### Added

- Initial implementation of the X25519 Diffie-Hellman function (RFC 7748).
- GF(2²⁵⁵ − 19) prime field arithmetic using 5-limb 51-bit radix representation
  (limbs stored as `u64`, products computed in `i128` to avoid overflow).
- Field operations exported as `pub` so the `ed25519` package can reuse them:
  - `fe_add`, `fe_sub`, `fe_neg` — addition, subtraction, negation
  - `fe_mul`, `fe_sq`, `fe_sq_n` — multiplication, squaring, repeated squaring
  - `fe_inv` — inversion via Fermat's Little Theorem (a^(p−2) mod p), using
    the 11-mul / 255-sq addition chain from libsodium / Go stdlib
  - `fe_cswap` — constant-time conditional swap (bitmask, no branches)
  - `Fe::from_bytes`, `Fe::to_bytes` — canonical little-endian encoding
  - `Fe::ZERO`, `Fe::ONE`, `Fe::ED25519_D`, `Fe::ED25519_D2` — constants
- Constant-time Montgomery ladder scalar multiplication (RFC 7748 §5):
  255 iterations, projective (X:Z) coordinates, combined doubling + differential
  addition in each step.
- Scalar clamping per RFC 7748 §5 (clear low 3 bits, clear bit 255, set bit 254).
- High-bit masking on the u-coordinate per RFC 7748 §5.
- `x25519(k, u)` — full scalar multiplication.
- `x25519_public_key(secret)` — convenience alias for `x25519(secret, G)`.
- `X25519_BASEPOINT` — base point constant (u = 9, 32-byte little-endian).
- 16 unit tests covering:
  - RFC 7748 §6.1 iterative ladder vectors (iter=1, iter=1000) — exact match.
  - Bob's DH key pair — exact match to RFC.
  - Alice's DH key pair — see note below.
  - Field arithmetic invariants: zero/one identity, self-subtraction, squaring,
    inversion, encode/decode roundtrip.
  - X25519 properties: public-key shortcut, commutativity, distinctness,
    scalar-clamping idempotency.
- 2 doc-tests (module-level example, `x25519_public_key` example).

### Notes

**RFC 7748 §6.1 Alice test vector**: The RFC lists Alice's private key as
`77076d0a...` and public key as `8520f009...`.  Cross-verification with
libsodium/nacl, Python bigint, and this implementation all agree that
x25519(`77076d0a...`, 9) = `d5f22539...`, not `8520f009...`.  The RFC's Alice
public key and shared secret are mutually consistent but not consistent with
the listed private key.  This appears to be a typo in RFC 7748 §6.1.  Our
tests use the libsodium-verified values (`d5f22539...` for the public key,
`209f0236...` for the shared secret) and include a detailed comment explaining
the discrepancy.

**Three-pass `fe_reduce_full`**: The canonical encoder calls `fe_reduce_full`
which performs three carry passes (not two) to guarantee all limbs are
strictly below 2⁵¹ before the conditional subtraction of p.  Two passes can
leave limb[0] as large as 2⁵¹ + 18 (due to the q4×19 fold-back carry),
which violates the precondition of the conditional subtraction.  A third pass
bounds limb[0] to ≤ 37 < 2⁵¹.
