# Changelog

All notable changes to `x509-rsa-pkcs1-v1-5-sha256-signature` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-19

### Added

- Added exact `sha256WithRSAEncryption` and present-NULL parameter checks.
- Added composition of repository-owned SHA-256, bounded RSAVP1, and canonical
  PKCS #1 v1.5 SHA-256 encoded-message validation over preserved TBS DER.
- Added non-copyable borrowed success evidence bound to the exact certificate
  and validated issuer key, plus stable payload-free failure categories.

### Security

- The API accepts only separately policy-validated public RSA keys and exposes
  no private-key, signing, decryption, trust, path, TLS, or network authority.
- Issuer selection and linkage remain explicit caller-owned policy rather than
  being inferred from names or the supplied key.
