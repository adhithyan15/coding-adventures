# Changelog

All notable changes to `tls13-inner-plaintext` are documented here.

## 0.1.0 - 2026-10-25

### Added

- Allocation-free decoding of authenticated TLS 1.3 inner plaintext.
- Caller-policy-bounded encoding with zero padding and mutation-safe preflight.
- Stable payload-free failures for malformed syntax and rejected policy.
