# Changelog

All notable changes to `rsa-public-operation` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-19

### Added

- Added a caller-defined nonzero operational modulus-byte ceiling.
- Added RFC 8017 RSAVP1 over previously validated RSA public keys using the
  repository-owned arbitrary-precision integer substrate.
- Added exact modulus-width input and output handling, representative range
  checks, stable redacted errors, and independent multi-limb test vectors.

### Security

- The crate exposes no raw-key parser, private exponent, signing, decryption,
  key generation, hashing, padding interpretation, certificate trust, or
  transport authority.
- Arithmetic branches only on public verification data and is not offered for
  private RSA operations.
