# Changelog

All notable changes to `x509-ocsp-rsa-algorithm-policy` will be documented in
this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-29

### Added

- Added exact caller-supplied OCSP signature OID and parameter profiles with
  inclusive RSA modulus and signature bit-length ranges.
- Added canonical strictly increasing public-exponent policy and rejection of
  unsupported or ambiguous algorithm profiles.
- Added non-copyable evidence bound to the exact response and validated RSA
  key, stable payload-free failures, and adversarial tests for every gate.

### Security

- The package is pure policy over public decoded data and invokes no
  cryptographic operation or external effect.
- Verifier routing, response authentication, responder authorization,
  freshness, revocation, path validation, and trust remain separate gates.
