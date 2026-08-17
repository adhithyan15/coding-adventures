# Changelog

## Unreleased

- Add `verify_grant_signature/5`, which checks every public `D18G` binding and
  the originator's Ed25519 signature without a receiver private key. Elixir
  could previously reach those checks only through `open_channel_key_grant/6`,
  which requires a `ReceiverKeyPair` in order to unwrap -- so an originator,
  holding no receiver secrets, had no way to verify the grants it had just
  sealed. D18T plan validation requires exactly that, which is why Rust, Python,
  TypeScript, Go, and Ruby already expose the equivalent. Brings Elixir to
  parity and completes the six-language surface.
- Refactor `open_channel_key_grant/6` onto a shared private
  `verify_grant_bindings!/5` so the receiver-key-free and unwrapping entry
  points cannot drift on binding order or stable error codes. No behavior
  change: every pre-existing opening fixture still produces its declared code,
  now asserted against both entry points.

## 0.1.0 - 2026-08-14

- Add the immutable Elixir D18F message and creation-field structs.
- Add exact D18M v1 and canonical JSON codecs with stable failure codes.
- Add repository-native XChaCha20-Poly1305, Ed25519, and SHA-256 integration.
- Add injected source creation and a pure monotonic UUID-v7 generator.
- Consume every shared positive, negative, and oversize fixture.
- Add the Elixir-native D18Q `D18G` codec and exact repository-owned
  X25519/HKDF/XChaCha20-Poly1305/Ed25519 grant operations.
- Add redacted immutable key values with honest `not_enforceable` erasure
  reporting and fail-closed validation-order errors.
- Add explicit immutable receiver epoch state, deterministic prospective
  rotation, and complete shared D18Q fixture coverage.
