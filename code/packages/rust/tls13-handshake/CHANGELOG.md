# Changelog

## 0.1.0 - 2026-10-29

- Add allocation-free TLS 1.3 handshake-message decoding with exact and
  stream-prefix entry points.
- Add explicit, duplicate-free message-type policy and a caller-selected body
  ceiling within RFC 8446's 24-bit limit.
- Add preflighted caller-buffer encoding, borrowed exact transcript bytes,
  payload-free stable errors, capability metadata, and adversarial tests.
