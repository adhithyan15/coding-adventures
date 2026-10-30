# Changelog

## 0.1.0 - 2026-10-30

### Added

- Added a non-copyable composition that binds exact target-server policy to
  an authenticated RSA SHA-512 issuer link terminating at a reviewed root.
- Added an exact decoded-target identity gate, payload-free mismatch errors,
  and redacted debug output.

### Security

- The binding performs no new parsing or signature verification and grants no
  anchor-constraint, multi-hop path, revocation, TLS, HTTP, OAuth transport,
  or network authority.
