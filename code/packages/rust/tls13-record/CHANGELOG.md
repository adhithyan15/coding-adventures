# Changelog

## 0.1.0 - 2026-10-25

- Add strict allocation-free TLS 1.3 plaintext and ciphertext outer-record
  decoding with exact and stream-prefix entry points.
- Add explicit plaintext content-type and fragment-length policy plus the RFC
  8446 plaintext and ciphertext absolute ceilings.
- Add preflighted caller-buffer encoding that leaves output untouched on
  failure.
- Add payload-free stable errors, borrowed validated record values, capability
  metadata, and adversarial framing tests.
