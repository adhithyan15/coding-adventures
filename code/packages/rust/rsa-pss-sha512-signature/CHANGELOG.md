# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-18

### Added

- Added fixed-profile SHA-512, MGF1-SHA-512, 64-byte-salt RSASSA-PSS
  signature verification over repository-owned RSA and EMSA-PSS primitives.
- Added independent OpenSSL-verified success coverage plus public-operation,
  odd-width prefix, digest, scratch, limit, and redaction failure coverage.
- Added borrowed non-copyable evidence that binds the exact validated public
  key, signature bytes, and already-computed digest.
- Declared the package's empty external-effect capability set.
