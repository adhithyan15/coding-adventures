# Changelog

All notable changes to `x509-ocsp-revocation-policy` will be documented in this
file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-25

### Added

- Added exact CertID-bound OCSP status and freshness composition.
- Added internal derivation of revocation observations from the retained exact
  `SingleResponse` and the shared fail-closed revocation-policy gate.
- Added non-copyable evidence retaining the exact CertID binding and explicit
  observation time, plus stable payload-free failure categories.
- Added focused good, revoked, unknown, missing-next-update, inclusive-boundary,
  inverted, future, stale, and unsupported-extension coverage.

### Security

- All `singleExtensions` fail closed until their semantics are implemented.
- Response-signature verification, responder authorization, outer extension
  policy, issuer-link and path authentication, trust, TLS, HTTP, OAuth, clock,
  cache, and network authority remain separate boundaries.
