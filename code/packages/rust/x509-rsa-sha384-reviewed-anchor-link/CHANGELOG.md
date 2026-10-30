# Changelog

## 0.1.0 - 2026-10-29

### Added

- Added a non-copyable composition binding the strongest repository-owned RSA
  SHA-384 issuer-policy proof to reviewed-root selection for the same exact
  decoded issuer object.
- Added a public Microsoft RSA Root Certificate Authority 2017 interoperability
  vector covering repository-owned parsing, policy, signature verification,
  and reviewed-snapshot selection.
- Added byte-equal distinct-object rejection and payload-redaction coverage.

### Security

- The result grants no anchor-constraint, target-policy, general path,
  revocation, TLS, HTTP, OAuth transport, credential, or network authority.
