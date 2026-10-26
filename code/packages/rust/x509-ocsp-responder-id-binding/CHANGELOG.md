# Changelog

All notable changes to `x509-ocsp-responder-id-binding` will be documented in
this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-24

### Added

- Added exact `byName` binding through one caller-owned Name-policy callback.
- Added fixed-length RFC 6960 `byKey` binding with repository-owned SHA-1 and
  constant-time complete-digest comparison.
- Added non-copyable evidence retaining the exact response, certificate, and
  identifier kind, plus stable payload-free mismatch categories.
- Added focused success, callback-direction, length, first-byte, and last-byte
  mismatch tests.

### Security

- The fixed SHA-1 operation is limited to RFC 6960 responder key identifiers;
  it does not enable SHA-1 certificate or OCSP response signatures.
- Responder authorization, response signature verification, certificate role,
  EKU, path, trust, freshness, revocation, TLS, HTTP, and network authority
  remain explicit separate boundaries.
