# Changelog

## 0.1.0 - 2026-10-21

### Added

- Added a non-copyable composition that binds exact target-server policy to
  one reviewed-root RSA SHA-256 issuer link for the same decoded child object.
- Added payload-free mismatch errors, redacted debug output, and focused tests
  for same-object acceptance and byte-equal distinct-object rejection.
- Documented that anchor constraints, path construction, revocation, TLS,
  HTTP, OAuth transport, and network authority remain outside this package.
