# Changelog

## Unreleased

- Add the immutable TypeScript D18Q `D18G` grant model, exact binary codec,
  explicit-material and CSPRNG sealing, fail-closed opening order, and stable
  error taxonomy.
- Add receiver epoch installation, pure sorted rotation plans, managed secret
  containers, and honest `best_effort` secret-erasure reporting.
- Consume every canonical D18Q derivation intermediate, record, negative case,
  receiver-state transition, and prospective-revocation fixture.
- Make the package-native BUILD run this package after installing local crypto
  dependencies, and fix HKDF typed-array compatibility under current TypeScript.

## 0.1.0

- Add the immutable TypeScript D18F message model.
- Add exact D18M v1 binary and canonical lossless JSON codecs.
- Add fail-closed creation, epoch-aware verification, and stable errors using
  repository SHA-256, Ed25519, and XChaCha20-Poly1305 primitives.
- Consume every shared positive, negative, and compact oversize fixture.
- Add injected monotonic UUID-v7 and nanosecond clock sources.
