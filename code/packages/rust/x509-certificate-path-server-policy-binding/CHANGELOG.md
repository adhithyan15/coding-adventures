# Changelog

All notable changes to `x509-certificate-path-server-policy-binding` are
documented in this file.

## [0.1.0] - 2026-10-23

### Added

- Added a non-copyable evidence composition that binds a constructed
  certificate path to complete server policy for its exact target object.
- Rejected byte-equal server-policy evidence produced from an independent
  certificate decode.
- Added payload-free error categories and redacted display output.
- Documented that the crate provides no issuer authentication, path
  validation, trust, constraints, revocation, TLS, HTTP, OAuth, or network
  authority.
