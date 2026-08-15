# Changelog

## Unreleased

- Add the immutable Python D18Q `D18G` v1 channel-key grant codec and sealed
  X25519/HKDF-SHA256/XChaCha20-Poly1305/Ed25519 workflow.
- Add managed channel, receiver, and signing keys; atomic monotonic receiver
  epoch state; and deterministic prospective rotation plans.
- Consume every canonical positive, negative, state, and rotation fixture and
  report Python secret erasure honestly as `not_enforceable`.

## 0.1.0

- Add the frozen, slotted Python D18F message model.
- Add exact D18M v1 binary and canonical lossless JSON codecs.
- Add fail-closed creation, epoch-aware verification, and stable errors using
  repository SHA-256, Ed25519, UUID, and XChaCha20-Poly1305 primitives.
- Consume every shared positive, negative, and compact oversize fixture.
- Add injected monotonic UUID-v7 and nanosecond clock sources.
