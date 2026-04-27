# Changelog — coding_adventures_x3dh

## [0.1.0] — 2026-04-24

### Added

- `IdentityKeyPair` struct with independent X25519 (DH) and Ed25519 (signing) keys.
  Implements `Zeroize` and `Drop` so secret material is wiped when dropped.
  Exposes `x25519_secret()` accessor for use by the Sealed Sender layer.
- `PreKeyPair` struct for X25519 signed and one-time prekeys.
  Implements `Zeroize` and `Drop`. Exposes `secret()` accessor.
- `PreKeyBundle` struct carrying Bob's published public key material.
- `X3DHOutput` struct wrapping the shared key and ephemeral public key.
  Implements `Zeroize` and `Drop` (wipes `shared_key` on drop).
- `X3DHError` enum with `InvalidSignature` and `KdfError` variants.
- `generate_identity_keypair()` — generates two independent keys (X25519 + Ed25519)
  from separate random seeds with RFC 7748 clamping.
- `generate_prekey_pair()` — generates an X25519 prekey with RFC 7748 clamping.
- `sign_prekey()` — Ed25519 sign a prekey's public bytes with an identity key.
- `create_prekey_bundle()` — assemble a `PreKeyBundle` from Bob's keys.
- `x3dh_send()` — Alice's four-DH sender operation; all intermediates are
  held in `Zeroizing<>` and wiped before return.
- `x3dh_receive()` — Bob's mirrored four-DH receiver operation; same
  `Zeroizing<>` discipline applied.
- 15 unit tests covering: key generation, clamping, signature verification,
  protocol correctness (with and without OPK), replay resistance, and
  wrong-key isolation.
