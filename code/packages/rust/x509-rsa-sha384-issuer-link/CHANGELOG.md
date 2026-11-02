# Changelog

All notable changes to `x509-rsa-sha384-issuer-link` will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-10-30

### Added

- Added a stronger issuer-link entry point that consumes exact certificate-
  wide critical-extension/issuing-CA evidence and certificate-bound RSA
  algorithm/key evidence before Name or signature work.
- Added a non-copyable binder that retains that authenticated link together
  with complete intermediate policy for the exact same decoded issuer object.

### Security

- Rejects byte-equal evidence from another decoded certificate object before
  invoking caller policy or RSA verification. Algorithm-policy data does not
  route the fixed SHA-384 verifier.
- Complete intermediate policy remains evidence only: path length,
  NameConstraints evaluation, later links, trust, revocation, TLS, HTTP, OAuth,
  audit, trace, provider, and network effects remain separate.

## [0.1.0] - 2026-10-29

### Added

- Added one directed child-to-candidate-issuer Name-policy composition.
- Added issuing-CA role, caller-owned RSA key policy, and exact RSA PKCS #1
  v1.5 SHA-384 certificate-signature verification for that same pair.
- Added non-copyable evidence retaining the exact child, issuer, and validated
  issuer key with stable payload-free failures.

### Security

- Complete path construction and validation, full issuer policy, trust,
  revocation, TLS, HTTP, audit, trace, and OAuth transport remain separate.
