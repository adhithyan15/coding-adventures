# Changelog

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
