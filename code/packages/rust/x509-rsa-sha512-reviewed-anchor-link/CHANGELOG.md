# Changelog

## 0.1.0 - 2026-10-29

### Added

- Added a non-copyable composition that binds the strongest repository-owned
  RSA SHA-512 issuer-policy evidence to reviewed-root selection evidence for
  the same exact decoded issuer certificate.
- Added payload-free mismatch errors and redacted debug output.
- Added an independent public D-TRUST BR Root CA 2 2023 interoperability
  vector covering SHA-512 self-signature verification, reviewed-snapshot
  selection, successful exact-object binding, and distinct-object rejection.

### Security

- The binding grants no target policy, anchor-constraint, generalized path,
  revocation, TLS, HTTP, OAuth transport, or network authority.
