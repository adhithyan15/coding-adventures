# Changelog

All notable changes to `x509-ocsp-rsa-pkcs1-v1-5-sha256-signature` will be
documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-24

### Added

- Added exact `sha256WithRSAEncryption` and present-NULL parameter checks for a
  structurally decoded `BasicOCSPResponse`.
- Added composition of repository-owned SHA-256, bounded RSAVP1, and canonical
  PKCS #1 v1.5 SHA-256 encoded-message validation over preserved
  `ResponseData` DER.
- Added non-copyable evidence binding the exact response and separately
  validated RSA key, stable payload-free errors, tampering and limit tests, and
  a trace-digested independent OpenSSL interoperability fixture.

### Security

- The API accepts only separately policy-validated public RSA keys and exposes
  no private-key, signing, decryption, trust, TLS, or network authority.
- Algorithm routing, responder selection and authorization, `ResponderID` and
  certificate/path binding, freshness, and revocation decisions remain
  explicit caller-owned boundaries.
