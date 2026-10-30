# Changelog

## 0.2.0 - 2026-10-30

### Added

- Added a non-copyable two-link composition that joins policy-bound RSA
  SHA-384 target-to-intermediate and intermediate-to-reviewed-root proofs only
  through the same exact decoded intermediate object.
- Added byte-equal distinct-intermediate rejection, complete-input accessors,
  redacted debug output, and payload-free failure coverage.

### Security

- The result retains existing public verification evidence only. It grants no
  path-length, constraint, generalized path, revocation, TLS, HTTP, OAuth,
  credential, provider, audit, trace, or network authority.

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
