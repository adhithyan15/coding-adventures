# Changelog

## Unreleased

- Add `VerifyGrantSignature`, which checks every public `D18G` binding and the
  originator's Ed25519 signature without a receiver private key. Go previously
  reached these checks only through `OpenChannelKeyGrant`, which needs a
  `ReceiverKeyPair` in order to unwrap — so an originator, holding no receiver
  secrets, had no way to verify the grants it had just sealed. D18T plan
  validation requires exactly that, which is why Rust, Python, and TypeScript
  already export the equivalent. Brings Go to parity with them.
- Refactor `OpenChannelKeyGrant` onto a shared `verifyGrantBindings` path so the
  receiver-key-free and unwrapping entry points cannot drift on binding order or
  stable error codes. No behavior change: every pre-existing opening fixture
  still produces its declared code, now asserted against both entry points.
- Add immutable-by-convention D18Q `D18G` v1 grants using repository X25519,
  HKDF-SHA256, XChaCha20-Poly1305, and Ed25519 primitives.
- Add managed keys, atomic monotonic receiver epoch state, deterministic
  prospective rotation, stable errors, and honest `best_effort` erasure.
- Consume every canonical positive, negative, state, and rotation fixture.

## 0.1.0 - 2026-08-14

- Add the immutable Go D18F message model with defensive byte ownership.
- Add exact D18M v1 and canonical JSON codecs with stable failure codes.
- Add repository-native XChaCha20-Poly1305, Ed25519, and SHA-256 integration.
- Add injected source creation and a monotonic UUID-v7 generator.
- Consume every shared positive, negative, and oversize fixture.
