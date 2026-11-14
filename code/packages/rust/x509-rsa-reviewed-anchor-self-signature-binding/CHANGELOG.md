# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-13

### Added

- Added a non-copyable composition that binds data-driven repository-owned RSA
  PKCS #1 v1.5 SHA-256, SHA-384, or SHA-512 verification to reviewed-root
  selection only when the routed child, issuer key policy, and selected root
  retain the same exact decoded certificate object.
- Added public-root interoperability coverage for every supported routed
  verifier and byte-equal distinct-object rejection at both identity checks.
- Added stable payload-free failures, redacted result formatting, a pure
  capability manifest, and Unix and Windows package build fronts.

### Security

- The binding performs no new cryptography and grants no trust-anchor, role,
  time, constraint, path, revocation, TLS, HTTP, OAuth, credential, audit,
  trace, I/O, external-effect, or network authority.
