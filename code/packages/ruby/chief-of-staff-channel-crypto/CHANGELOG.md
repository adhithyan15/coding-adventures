# Changelog

## Unreleased

- Add `verify_grant_signature`, which checks every public `D18G` binding and the
  originator's Ed25519 signature without a receiver private key. Ruby could
  previously reach those checks only through `open_channel_key_grant`, which
  requires a `ReceiverKeyPair` in order to unwrap -- so an originator, holding
  no receiver secrets, had no way to verify the grants it had just sealed. D18T
  plan validation requires exactly that, which is why Rust, Python, and
  TypeScript already export the equivalent. Brings Ruby to parity.
- Refactor `open_channel_key_grant` onto a shared `verify_grant_bindings` path
  so the receiver-key-free and unwrapping entry points cannot drift on binding
  order or stable error codes. No behavior change: every pre-existing opening
  fixture still produces its declared code, now asserted against both entry
  points. A raise from `Ed25519.verify` is folded into the same
  `invalid_signature` result the false return already produced, so the
  post-unwrap rescue no longer has to distinguish the two.

## 0.1.0 - 2026-08-14

- Add the frozen Ruby D18F message model with copied, frozen byte ownership.
- Add exact D18M v1 and canonical JSON codecs with stable failure codes.
- Add repository-native XChaCha20-Poly1305, Ed25519, and SHA-256 integration.
- Add injected source creation and a monotonic UUID-v7 generator.
- Consume every shared positive, negative, and oversize fixture.
- Add the Ruby-native D18Q `D18G` grant codec, exact X25519/HKDF/XChaCha20-
  Poly1305/Ed25519 sealing and fail-closed opening order.
- Add managed CMK, receiver, and signing-key values with redacted inspection
  and honest `best_effort` controlled erasure.
- Add atomic receiver epoch installation, deterministic prospective rotation,
  stable errors, and complete shared D18Q fixture coverage.
