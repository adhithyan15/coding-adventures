# Changelog

## 0.1.0 - 2026-10-30

### Added

- Added a non-copyable composition that binds exact target-server policy to
  an authenticated RSA SHA-384 issuer link terminating at a reviewed root.
- Added an exact decoded-target identity gate, payload-free mismatch errors,
  and redacted debug output.
- Added bounded selected-anchor DNS NameConstraints evaluation for the exact
  retained target Subject Alternative Name, with separate caller-owned ASN.1
  limits and non-copyable success evidence.

### Security

- The binding performs no new signature verification. The follow-up decodes
  only the selected anchor's optional constraint bytes and exact target SAN,
  exposes payload-free failures, and grants no IP/intermediate constraint,
  hostname, multi-hop path, revocation, TLS, HTTP, OAuth transport, or network
  authority.
