# Changelog

All notable changes to `x509-rsa-sha256-issuer-link` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-19

### Added

- Added one directed child-to-candidate-issuer Name-policy composition.
- Added candidate issuing-CA role and RSA public-key policy validation.
- Added exact RSA PKCS #1 v1.5 SHA-256 child-signature verification.
- Added non-copyable evidence bound to the exact child, candidate issuer, and
  validated issuer key, plus stable payload-free failure categories.
- Added a consuming composition from unique issuer-candidate selection evidence
  that authenticates the exact bound child and candidate without repeating Name
  policy.

### Security

- The composition keeps name equality, RSA strength, decoding work, and public
  operation limits caller-owned and explicit.
- Passing one link does not establish a path, trust anchor, validity,
  revocation, TLS, HTTP, or network authority.
- Selection-bound authentication prevents cross-certificate substitution
  between candidate selection and RSA link verification.
