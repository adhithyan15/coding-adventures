# Changelog

## 0.1.0 - 2026-10-29

- Add fixed-memory dual SHA-256/SHA-384 transcript state before TLS 1.3
  cipher-suite selection.
- Bind selected transcript state and snapshots to one policy-authorized cipher
  suite and enforce a caller-selected cumulative byte ceiling.
- Add one-shot RFC 8446 HelloRetryRequest `message_hash` rewriting, stable
  payload-free errors, capability metadata, and adversarial tests.
