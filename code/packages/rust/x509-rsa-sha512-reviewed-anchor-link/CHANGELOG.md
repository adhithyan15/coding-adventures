# Changelog

## 0.3.0 - 2026-10-30

### Added

- Added an additive two-link composition that carries the existing
  authenticated SHA-512 intermediate path-length transition into the exact
  intermediate-to-reviewed-root binding.
- Added complete-input accessors, redacted debug output, retained-state
  coverage, and rejection of byte-equal separately decoded intermediates.

### Security

- The stronger result retains the authenticated adjacent link, complete
  intermediate policy, advanced path-length state, reviewed parent link, and
  exact root selection. It grants no path search, constraint, revocation, TLS,
  HTTP, OAuth, credential, provider, audit, trace, or network authority.

## 0.2.0 - 2026-10-30

### Added

- Added a non-copyable two-link composition that joins a complete-intermediate-
  policy-bound RSA SHA-512 target link to an intermediate-to-reviewed-root
  proof only through the same exact decoded intermediate object.
- Added byte-equal distinct-intermediate rejection, complete-input accessors,
  redacted debug output, and payload-free failure coverage.

### Security

- The result retains existing public verification evidence only. It grants no
  path-length, constraint, generalized path, revocation, TLS, HTTP, OAuth,
  credential, provider, audit, trace, or network authority.

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
