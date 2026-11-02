# Changelog

## 0.1.0 - 2026-10-29

- Add exact allocation-free RFC 8446 compatibility ChangeCipherSpec body
  decoding and caller-buffer encoding.
- Add exhaustive rejection tests for every noncanonical octet, exact-length
  enforcement, failure atomicity, and payload-free stable failures.
- Add capability metadata, documentation, and explicit handshake-state and
  transcript boundaries.
